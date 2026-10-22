//! Latest-state mailbox keeps screen transfers off HID/MIDI/motor service.

use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::Context as _;

use super::{Display, Frame, UsbScreens, page};

#[derive(Default)]
struct Mailbox {
    pending: Option<[page::Page; 2]>,
    stop: bool,
}

type Shared = Arc<(Mutex<Mailbox>, Condvar)>;

pub struct Worker {
    shared: Shared,
    thread: Option<JoinHandle<()>>,
    last: Option<[page::Page; 2]>,
}

impl Worker {
    /// Start asynchronous screen service. USB failures remain on this worker.
    ///
    /// # Errors
    /// Returns an error if the worker thread cannot be created.
    pub fn start() -> anyhow::Result<Self> {
        Self::start_with(|| {
            let mut usb = UsbScreens::open()?;
            Ok(move |screen, frame: &Frame| usb.write(screen, frame))
        })
    }

    fn start_with<F, W>(open: F) -> anyhow::Result<Self>
    where
        F: FnOnce() -> anyhow::Result<W> + Send + 'static,
        W: FnMut(Display, &Frame) -> anyhow::Result<()>,
    {
        let shared = Arc::new((Mutex::new(Mailbox::default()), Condvar::new()));
        let task_shared = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name(String::from("s4-screens"))
            .spawn(move || {
                if let Err(error) = run(&task_shared, open) {
                    tracing::warn!(%error, "SCREENS_DISABLED: MIDI, LEDs and tension remain active");
                }
                match task_shared.0.lock() {
                    Ok(mut mailbox) => mailbox.stop = true,
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

    pub fn publish(&mut self, pages: [page::Page; 2]) {
        if self.last == Some(pages) {
            return;
        }
        self.last = Some(pages);
        match self.shared.0.lock() {
            Ok(mut mailbox) => {
                if !mailbox.stop {
                    // Replace, never queue obsolete deck/layer states.
                    mailbox.pending = Some(pages);
                    self.shared.1.notify_one();
                }
            }
            Err(error) => tracing::warn!(%error, "could not publish S4 screen state"),
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
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::warn!("S4 screen worker panicked");
        }
    }
}

fn run<F, W>(shared: &Shared, open: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<W>,
    W: FnMut(Display, &Frame) -> anyhow::Result<()>,
{
    let mut write = open()?;
    let mut frame = Frame::default();
    let mut current = None;
    let mut previous = None;
    let mut bright = true;
    let mut next_flash: Option<Instant> = None;
    loop {
        let pages = {
            let mut mailbox = shared
                .0
                .lock()
                .map_err(|error| anyhow::anyhow!("{error}"))?;
            while mailbox.pending.is_none() && !mailbox.stop {
                mailbox = if let Some(deadline) = next_flash {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        break;
                    }
                    shared
                        .1
                        .wait_timeout(mailbox, remaining)
                        .map_err(|error| anyhow::anyhow!("{error}"))?
                        .0
                } else {
                    shared
                        .1
                        .wait(mailbox)
                        .map_err(|error| anyhow::anyhow!("{error}"))?
                };
            }
            if mailbox.stop {
                break;
            }
            current = mailbox.pending.take().or(current);
            drop(mailbox);
            current.context("screen wake without page state")?
        };
        let now = Instant::now();
        if pages.iter().any(page::Page::selecting) {
            if next_flash.is_none() {
                bright = true;
                next_flash = Some(now + Duration::from_millis(500));
            } else if next_flash.is_some_and(|deadline| now >= deadline) {
                bright = !bright;
                next_flash = Some(now + Duration::from_millis(500));
            }
        } else {
            bright = true;
            next_flash = None;
        }
        let rendered = pages.map(|page| (page, !page.selecting() || bright));
        for (side, screen) in [Display::Left, Display::Right].into_iter().enumerate() {
            // Do not send queued active pages after Stop.
            if shared
                .0
                .lock()
                .map_err(|error| anyhow::anyhow!("{error}"))?
                .stop
            {
                break;
            }
            if previous
                .as_ref()
                .is_none_or(|old: &[(page::Page, bool); 2]| old[side] != rendered[side])
            {
                page::render_phase(&pages[side], &mut frame, rendered[side].1);
                write(screen, &frame)?;
            }
        }
        previous = Some(rendered);
    }
    page::stopped(&mut frame);
    write(Display::Left, &frame)?;
    write(Display::Right, &frame)?;
    Ok(())
}

#[cfg(test)]
mod tests;
