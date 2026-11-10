//! Screen worker: owns the USB side of the S8 screens on its own thread.
//!
//! The reader hands over frames with `Handle::submit`; the worker sends them
//! through a `Transport`. Each panel has one latest-wins slot, so a slow
//! transfer never blocks the reader or the HID/MIDI loop, and a newer frame
//! replaces an unsent older one. Specification:
//! `docs/s8/screen-transport.md`.

use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::Context as _;

use super::{FRAME_BYTES, PIXEL_BYTES, Panel, full, solid};

/// Frames per second one panel may receive at most.
///
/// Phase 5 (#43) measured about 16.6 ms per full frame, so this is the most
/// two panels can take together (`docs/s8/screen-stream-validation.md`).
pub const MAX_FRAMES_PER_SECOND: u64 = 30;

/// Shortest time between the starts of two transfers to the same panel.
pub const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / MAX_FRAMES_PER_SECOND);

/// Pixel pair of the frames sent to each panel on stop. Provisional: #27
/// decides what the panels show when the bridge exits.
pub const EXIT_PIXELS: [u8; 4] = [0; 4];

const PANELS: [Panel; 2] = [Panel::Left, Panel::Right];

/// The S8 side of the screens, claimed lazily and released on stop.
pub trait Transport: Send + 'static {
    /// Claims the screen interface. Called once, when the first frame is due.
    ///
    /// # Errors
    /// Fails when the device is missing or another process holds the interface.
    fn claim(&mut self) -> anyhow::Result<()>;

    /// Sends one complete frame as a single transfer.
    ///
    /// # Errors
    /// Fails on a rejected, timed-out or short transfer. The worker never retries.
    fn send(&mut self, frame: &[u8]) -> anyhow::Result<()>;

    /// Releases the interface; harmless when nothing is claimed.
    fn release(&mut self);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// The screen path is enabled and nothing is claimed.
    Pending,
    /// The device accepted a frame.
    Active,
    /// Released cleanly.
    Inactive,
    /// A claim or transfer failed, or the stream broke the protocol. The
    /// reason is one line.
    Failed(String),
}

impl Status {
    #[must_use]
    pub fn failed(reason: &str) -> Self {
        Self::Failed(reason.replace(|c: char| c.is_control(), " "))
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => f.write_str("pending"),
            Self::Active => f.write_str("active"),
            Self::Inactive => f.write_str("inactive"),
            Self::Failed(reason) => write!(f, "failed {reason}"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub interval: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval: FRAME_INTERVAL,
        }
    }
}

#[derive(Clone)]
enum Stop {
    Clean,
    Abort(String),
}

#[derive(Default)]
struct Slots {
    pending: [Option<Vec<u8>>; 2],
    spare: [Option<Vec<u8>>; 2],
    stop: Option<Stop>,
}

#[derive(Default)]
struct Shared {
    slots: Mutex<Slots>,
    wake: Condvar,
}

impl Shared {
    fn slots(&self) -> MutexGuard<'_, Slots> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Reader-side access to the worker.
#[derive(Clone)]
pub struct Handle {
    shared: Arc<Shared>,
}

impl Handle {
    /// Replaces any unsent frame of `panel`. Ignored once the worker stopped.
    pub fn submit(&self, panel: Panel, pixels: &[u8; PIXEL_BYTES]) {
        let index = usize::from(panel.index());
        let mut slots = self.shared.slots();
        if slots.stop.is_some() {
            return;
        }
        let mut buffer = slots.pending[index]
            .take()
            .or_else(|| slots.spare[index].take())
            .unwrap_or_else(|| Vec::with_capacity(PIXEL_BYTES));
        buffer.clear();
        buffer.extend_from_slice(pixels);
        slots.pending[index] = Some(buffer);
        drop(slots);
        self.shared.wake.notify_one();
    }

    /// Stops the screen path: exit frames, release, then `Failed(reason)`.
    pub fn abort(&self, reason: &str) {
        let mut slots = self.shared.slots();
        if slots.stop.is_none() {
            slots.stop = Some(Stop::Abort(reason.to_owned()));
        }
        drop(slots);
        self.shared.wake.notify_all();
    }
}

pub struct Worker {
    handle: Handle,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    /// Reports `Pending` on the calling thread, then starts the worker thread.
    /// Nothing is claimed until the first frame arrives.
    ///
    /// # Errors
    /// Returns an error if the thread cannot be created.
    pub fn start<T, R>(mut transport: T, config: Config, report: R) -> anyhow::Result<Self>
    where
        T: Transport,
        R: Fn(&Status) + Send + 'static,
    {
        report(&Status::Pending);
        let shared = Arc::new(Shared::default());
        let task = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name(String::from("s8-screens"))
            .spawn(move || run(&task, &mut transport, config.interval, &report))
            .context("start S8 screen worker")?;
        Ok(Self {
            handle: Handle { shared },
            thread: Some(thread),
        })
    }

    #[must_use]
    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        {
            let mut slots = self.handle.shared.slots();
            if slots.stop.is_none() {
                slots.stop = Some(Stop::Clean);
            }
            drop(slots);
            self.handle.shared.wake.notify_all();
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::warn!("S8 screen worker panicked");
        }
    }
}

#[derive(Default)]
struct Session {
    claimed: bool,
    active: bool,
    sent: [bool; 2],
    frame: Vec<u8>,
}

enum Wait {
    Frame(usize, Vec<u8>),
    Stop(Stop),
}

fn run<T: Transport>(
    shared: &Shared,
    transport: &mut T,
    interval: Duration,
    report: &dyn Fn(&Status),
) {
    let mut session = Session {
        frame: Vec::with_capacity(FRAME_BYTES),
        ..Session::default()
    };
    let outcome = serve(shared, transport, interval, report, &mut session)
        .and_then(|stop| park(transport, &mut session).map(|()| stop));
    transport.release();
    let status = match outcome {
        Ok(Stop::Clean) => Status::Inactive,
        Ok(Stop::Abort(reason)) => Status::failed(&reason),
        Err(error) => Status::failed(&format!("{error:#}")),
    };
    let mut slots = shared.slots();
    if slots.stop.is_none() {
        slots.stop = Some(Stop::Clean);
    }
    drop(slots);
    report(&status);
}

fn serve<T: Transport>(
    shared: &Shared,
    transport: &mut T,
    interval: Duration,
    report: &dyn Fn(&Status),
    session: &mut Session,
) -> anyhow::Result<Stop> {
    let mut next = [Instant::now(); 2];
    let mut last = 1;
    loop {
        let (index, pixels) = match wait_for_frame(shared, &next, &mut last) {
            Wait::Frame(index, pixels) => (index, pixels),
            Wait::Stop(stop) => return Ok(stop),
        };
        next[index] = Instant::now() + interval;
        let sent = send_frame(transport, session, PANELS[index], &pixels);
        shared.slots().spare[index] = Some(pixels);
        sent?;
        session.sent[index] = true;
        if !session.active {
            session.active = true;
            report(&Status::Active);
        }
    }
}

fn send_frame<T: Transport>(
    transport: &mut T,
    session: &mut Session,
    panel: Panel,
    pixels: &[u8],
) -> anyhow::Result<()> {
    if !session.claimed {
        transport.claim()?;
        session.claimed = true;
    }
    let pixels: &[u8; PIXEL_BYTES] = pixels.try_into().map_err(|_| {
        anyhow::anyhow!("frame holds {} bytes, expected {PIXEL_BYTES}", pixels.len())
    })?;
    full(&mut session.frame, panel, pixels);
    transport.send(&session.frame)
}

/// Blocks until a panel has a frame and its pacing interval has passed, or a
/// stop is requested. Panels alternate when both are ready.
fn wait_for_frame(shared: &Shared, next: &[Instant; 2], last: &mut usize) -> Wait {
    let mut slots = shared.slots();
    loop {
        if let Some(stop) = &slots.stop {
            return Wait::Stop(stop.clone());
        }
        let now = Instant::now();
        let mut wake: Option<Instant> = None;
        for index in [*last ^ 1, *last] {
            if slots.pending[index].is_none() {
                continue;
            }
            if next[index] <= now {
                if let Some(pixels) = slots.pending[index].take() {
                    *last = index;
                    return Wait::Frame(index, pixels);
                }
            } else {
                wake = Some(wake.map_or(next[index], |at| at.min(next[index])));
            }
        }
        slots = match wake {
            None => shared
                .wake
                .wait(slots)
                .unwrap_or_else(PoisonError::into_inner),
            Some(at) => {
                shared
                    .wake
                    .wait_timeout(slots, at.saturating_duration_since(now))
                    .unwrap_or_else(PoisonError::into_inner)
                    .0
            }
        };
    }
}

/// Black frames for every panel that received a frame, so the panels do not
/// keep the last app frame once the bridge stops.
fn park<T: Transport>(transport: &mut T, session: &mut Session) -> anyhow::Result<()> {
    for (index, panel) in PANELS.into_iter().enumerate() {
        if session.sent[index] {
            solid(&mut session.frame, panel, EXIT_PIXELS);
            transport
                .send(&session.frame)
                .context("send S8 exit frame")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
