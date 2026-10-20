//! A bounded snapshot producer, independent of the motor writer's deadline.

use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail, ensure};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError, TrySendError, bounded};
use hidapi::HidDevice;
use tracing::warn;

use super::capture::RawFrame;
use super::continuity::OUTPUT_INTERVAL_US;

pub struct Reader {
    pub frames: Receiver<RawFrame>,
    pub done: Receiver<anyhow::Result<()>>,
    cancel: Sender<()>,
    handle: Option<JoinHandle<()>>,
    pub completed: bool,
}

impl Reader {
    #[cfg(test)]
    pub fn channel_only() -> (Self, Sender<RawFrame>, Sender<anyhow::Result<()>>) {
        let (frames_tx, frames) = bounded(32);
        let (done_tx, done) = bounded(1);
        let (cancel, _) = bounded(1);
        (
            Self {
                frames,
                done,
                cancel,
                handle: None,
                completed: false,
            },
            frames_tx,
            done_tx,
        )
    }

    pub fn start(hid: HidDevice, started: Instant, include_analog: bool) -> anyhow::Result<Self> {
        let (frame_tx, frames) = bounded(32);
        let (done_tx, done) = bounded(1);
        let (cancel, cancel_rx) = bounded(1);
        let handle = std::thread::Builder::new()
            .name("jog-diagnostic-input".to_owned())
            .spawn(move || {
                let result = (|| {
                    let mut buffers = [[0_u8; 128]; 3];
                    let mut analog_buffer = [0_u8; 79];
                    let mut next_poll_us = 0;
                    let clock = || -> anyhow::Result<u64> {
                        Ok(u64::try_from(started.elapsed().as_micros())?)
                    };
                    let wait = |remaining_us| match cancel_rx
                        .recv_timeout(Duration::from_micros(remaining_us))
                    {
                        Ok(()) | Err(RecvTimeoutError::Disconnected) => true,
                        Err(RecvTimeoutError::Timeout) => false,
                    };
                    while poll_slot(&mut next_poll_us, &cancel_rx, (clock, wait))? {
                        let analog = if include_analog {
                            analog_buffer[0] = 2;
                            let length = hid
                                .get_input_report(&mut analog_buffer)
                                .context("request S4 analog report 2")?;
                            ensure!(
                                length == 79 && analog_buffer[0] == 2,
                                "unexpected S4 analog snapshot framing"
                            );
                            Some(analog_buffer.to_vec())
                        } else {
                            None
                        };
                        let lengths = crate::input::read_batch(&hid, true, &mut buffers)?;
                        let frame = RawFrame {
                            host_us: u64::try_from(started.elapsed().as_micros())?,
                            buttons: buffers[0][..lengths[0]].to_vec(),
                            jog: buffers[1][..lengths[1]].to_vec(),
                            analog,
                        };
                        if !publish_frame(frame, (&frame_tx, &cancel_rx))? {
                            return Ok(());
                        }
                    }
                    Ok(())
                })();
                // Completion means native handle teardown has also finished.
                drop(hid);
                if let Err(error) = done_tx.try_send(result) {
                    warn!(%error, "DIAGNOSTIC_READER_RESULT_UNDELIVERED");
                }
            })
            .context("start snapshot reader")?;
        Ok(Self {
            frames,
            done,
            cancel,
            handle: Some(handle),
            completed: false,
        })
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        match self.cancel.try_send(()) {
            Ok(()) | Err(crossbeam_channel::TrySendError::Disconnected(())) => {}
            Err(error) => bail!("cancel snapshot reader: {error}"),
        }
        if !self.completed {
            let result = self
                .done
                .recv_timeout(Duration::from_millis(50))
                .context("reader did not stop within 50ms; exit diagnostic process")?;
            self.completed = true;
            result?;
        }
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("snapshot reader panicked"))?;
        }
        Ok(())
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        // Never wait on a possibly stalled native call during unwinding.
        if self.handle.is_some() {
            match self.cancel.try_send(()) {
                Ok(()) | Err(crossbeam_channel::TrySendError::Disconnected(())) => {}
                Err(error) => warn!(%error, "DIAGNOSTIC_READER_CANCEL_FAILED"),
            }
        }
    }
}

fn poll_slot<C, W>(
    next_poll_us: &mut u64,
    cancel: &Receiver<()>,
    mut timing: (C, W),
) -> anyhow::Result<bool>
where
    C: FnMut() -> anyhow::Result<u64>,
    W: FnMut(u64) -> bool,
{
    loop {
        match cancel.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => return Ok(false),
            Err(TryRecvError::Empty) => {}
        }
        let now_us = timing.0()?;
        if now_us >= *next_poll_us {
            // Schedule from this actual start, not missed deadlines. An
            // available interrupt cannot create a catch-up polling burst.
            *next_poll_us = now_us.saturating_add(OUTPUT_INTERVAL_US);
            return Ok(true);
        }
        if timing.1(*next_poll_us - now_us) {
            return Ok(false);
        }
    }
}

fn publish_frame(
    frame: RawFrame,
    channels: (&Sender<RawFrame>, &Receiver<()>),
) -> anyhow::Result<bool> {
    match channels.1.try_recv() {
        Ok(()) | Err(TryRecvError::Disconnected) => return Ok(false),
        Err(TryRecvError::Empty) => {}
    }
    match channels.0.try_send(frame) {
        Ok(()) => Ok(true),
        Err(TrySendError::Full(frame)) => bail!(
            "snapshot queue overflow: capacity={:?}, rejected_snapshot_us={}",
            channels.0.capacity(),
            frame.host_us
        ),
        Err(TrySendError::Disconnected(frame)) => bail!(
            "snapshot consumer disconnected: rejected_snapshot_us={}",
            frame.host_us
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jog_diagnostic::decode::tests::frame;

    #[test]
    fn cancellation_after_snapshot_prevents_full_queue_publication() -> anyhow::Result<()> {
        // Given a full snapshot queue and cancellation while the native read ran.
        let (frames_tx, _frames_rx) = bounded(1);
        frames_tx.send(frame(0, 1, [0, 0]))?;
        let (cancel_tx, cancel_rx) = bounded(1);
        cancel_tx.send(())?;
        // When the completed snapshot is offered to the real publication path.
        let continued = publish_frame(frame(2000, 2, [0, 0]), (&frames_tx, &cancel_rx))?;
        // Then cancellation wins instead of reporting overflow on normal Stop.
        assert!(!continued);
        Ok(())
    }
}

#[cfg(test)]
#[path = "reader_pacing_tests.rs"]
mod pacing_tests;
