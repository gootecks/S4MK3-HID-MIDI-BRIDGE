//! Latest-state mailbox and screen ownership stay off HID/MIDI/motor service.

use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

use anyhow::Context as _;

use super::{Display, Frame, UsbScreens, page};

mod service;
use service::run;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    On,
    Off,
    Retry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Active,
    Inactive,
    Failed,
    Pending,
}

impl State {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Failed => "failed",
            Self::Pending => "pending",
        }
    }
}

#[derive(Clone, Copy)]
struct Request {
    id: u64,
    generation: u64,
    action: Action,
}

#[derive(Default)]
struct Mailbox {
    pending: Option<[page::Page; 2]>,
    latest: Option<Request>,
    release_required: bool,
    initialized: bool,
    stop: bool,
}

type Shared = Arc<(Mutex<Mailbox>, Condvar)>;

#[derive(Clone)]
pub struct Control {
    shared: Shared,
}

impl Control {
    pub fn request(&self, id: u64, action: Action) {
        match self.shared.0.lock() {
            Ok(mut mailbox) => {
                if mailbox.stop {
                    return;
                }
                let generation = mailbox
                    .latest
                    .map_or(1, |old| old.generation.wrapping_add(1));
                mailbox.latest = Some(Request {
                    id,
                    generation,
                    action,
                });
                if matches!(action, Action::Off | Action::Retry) {
                    mailbox.release_required = true;
                }
                self.shared.1.notify_one();
            }
            Err(error) => tracing::warn!(%error, "could not request S4 screen control"),
        }
    }
}

pub struct Worker {
    shared: Shared,
    thread: Option<JoinHandle<()>>,
    last: Option<[page::Page; 2]>,
}

impl Worker {
    /// Start asynchronous screen service; USB ownership remains on its thread.
    ///
    /// # Errors
    /// Returns an error if the worker thread cannot be created.
    pub fn start() -> anyhow::Result<Self> {
        Self::start_with(|| {
            let mut usb = UsbScreens::open()?;
            Ok(move |screen, frame: &Frame| usb.write(screen, frame))
        })
    }

    /// Start a controllable screen service without claiming USB before HID input.
    ///
    /// # Errors
    /// Returns an error if the worker thread cannot be created.
    pub fn start_managed(enabled: bool) -> anyhow::Result<Self> {
        let worker = Self::start_with_config(
            || {
                let mut usb = UsbScreens::open()?;
                Ok(move |screen, frame: &Frame| usb.write(screen, frame))
            },
            enabled,
            Some(|id, state: State| println!("S4_SCREEN_STATUS {id} {}", state.label())),
        )?;
        println!(
            "S4_SCREEN_STATUS 0 {}",
            if enabled { "pending" } else { "inactive" }
        );
        Ok(worker)
    }

    fn start_with<F, W>(open: F) -> anyhow::Result<Self>
    where
        F: FnOnce() -> anyhow::Result<W> + Send + 'static,
        W: FnMut(Display, &Frame) -> anyhow::Result<()> + Send + 'static,
    {
        let mut open = Some(open);
        Self::start_with_config(
            move || open.take().context("screen worker already opened")?(),
            true,
            None::<fn(u64, State)>,
        )
    }

    fn start_with_config<F, W, S>(open: F, enabled: bool, status: Option<S>) -> anyhow::Result<Self>
    where
        F: FnMut() -> anyhow::Result<W> + Send + 'static,
        W: FnMut(Display, &Frame) -> anyhow::Result<()> + Send + 'static,
        S: Fn(u64, State) + Send + 'static,
    {
        let mut mailbox = Mailbox::default();
        if status.is_some() {
            mailbox.latest = Some(Request {
                id: 0,
                generation: 0,
                action: if enabled { Action::On } else { Action::Off },
            });
        }
        let shared = Arc::new((Mutex::new(mailbox), Condvar::new()));
        let task_shared = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name(String::from("s4-screens"))
            .spawn(move || {
                if let Err(error) = run(&task_shared, open, enabled, status.as_ref()) {
                    tracing::warn!(%error, "SCREENS_DISABLED: MIDI, LEDs and tension remain active");
                }
                match task_shared.0.lock() {
                    Ok(mut mailbox) => {
                        mailbox.stop = true;
                        task_shared.1.notify_all();
                    }
                    Err(error) => tracing::warn!(%error, "screen mailbox unavailable at shutdown"),
                }
            })
            .context("start S4 screen worker")?;
        Ok(Self {
            shared,
            thread: Some(thread),
            last: None,
        })
    }

    #[must_use]
    pub fn control(&self) -> Control {
        Control {
            shared: Arc::clone(&self.shared),
        }
    }

    pub fn publish(&mut self, pages: [page::Page; 2]) {
        if self.last == Some(pages) {
            return;
        }
        self.last = Some(pages);
        match self.shared.0.lock() {
            Ok(mut mailbox) => {
                if !mailbox.stop {
                    mailbox.pending = Some(pages);
                    mailbox.initialized = true;
                    self.shared.1.notify_one();
                }
            }
            Err(error) => tracing::warn!(%error, "could not publish S4 screen state"),
        }
    }

    pub fn refresh(&mut self, pages: [page::Page; 2]) {
        if self
            .shared
            .0
            .lock()
            .is_ok_and(|mailbox| mailbox.initialized)
        {
            self.publish(pages);
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        match self.shared.0.lock() {
            Ok(mut mailbox) => {
                mailbox.stop = true;
                mailbox.pending = None;
                self.shared.1.notify_one();
            }
            Err(error) => tracing::warn!(%error, "could not stop S4 screen worker"),
        }
        // Full bridge shutdown has already neutralized motors. Live commands never join here.
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::warn!("S4 screen worker panicked");
        }
    }
}

#[cfg(test)]
mod tests;
