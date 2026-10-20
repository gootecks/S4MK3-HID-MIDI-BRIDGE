//! Input-edge priority, a 2ms output clock, and unconditional neutral cleanup.

use anyhow::{Context as _, bail};
use crossbeam_channel::{Receiver, TryRecvError, select_biased};
use s4mk3_hid_midi_bridge::tension::{Engine, neutral_report};
use std::time::Duration;
use tracing::{info, warn};

use super::input::{Frame, INTERVAL_US, Reader};

pub enum Action<'a> {
    Initialize,
    Motor([u8; 11]),
    Frame(&'a Frame),
    Feedback(u64),
}

pub struct Output<F: FnMut(Action<'_>) -> anyhow::Result<()>> {
    service: F,
    finished: bool,
}

impl<F: FnMut(Action<'_>) -> anyhow::Result<()>> Output<F> {
    pub fn new(service: F) -> anyhow::Result<Self> {
        let mut output = Self {
            service,
            finished: false,
        };
        (output.service)(Action::Motor(neutral_report()))?;
        Ok(output)
    }

    pub fn initialize(&mut self) -> anyhow::Result<()> {
        (self.service)(Action::Initialize)
    }

    pub fn finish(&mut self) -> anyhow::Result<()> {
        if !self.finished {
            (self.service)(Action::Motor(neutral_report()))?;
            self.finished = true;
            info!("TENSION_FINAL_NEUTRAL: native write accepted");
        }
        Ok(())
    }

    fn sample(&mut self, engine: &mut Engine, frame: &Frame, now_us: u64) -> anyhow::Result<()> {
        if let Some(report) =
            engine.observe(frame.report(0), frame.report(1), frame.host_us, now_us)?
        {
            (self.service)(Action::Motor(report))?;
        }
        (self.service)(Action::Frame(frame))
    }
}

impl<F: FnMut(Action<'_>) -> anyhow::Result<()>> Drop for Output<F> {
    fn drop(&mut self) {
        if !self.finished
            && let Err(error) = self.finish()
        {
            warn!(%error, "TENSION_NEUTRAL_FAILED");
        }
    }
}

pub fn run<C, F>(
    engine: &mut Engine,
    reader: &mut Reader,
    stop: &Receiver<()>,
    duration_us: Option<u64>,
    clock: C,
    output: &mut Output<F>,
) -> anyhow::Result<()>
where
    C: Fn() -> anyhow::Result<u64>,
    F: FnMut(Action<'_>) -> anyhow::Result<()>,
{
    let mut next_tick_us = INTERVAL_US;
    loop {
        if terminal(reader, stop, duration_us, clock()?)? {
            return Ok(());
        }
        let now_us = clock()?;
        if now_us >= next_tick_us {
            // A queued touch release cannot sit behind an overdue force write.
            for _ in 0..32 {
                if terminal(reader, stop, duration_us, clock()?)? {
                    return Ok(());
                }
                match reader.frames.try_recv() {
                    Ok(frame) => output.sample(engine, &frame, clock()?)?,
                    Err(TryRecvError::Disconnected) => bail!("snapshot channel closed"),
                    Err(TryRecvError::Empty) => break,
                }
            }
            if terminal(reader, stop, duration_us, clock()?)? {
                return Ok(());
            }
            let tick_us = clock()?;
            (output.service)(Action::Motor(engine.tick(tick_us)?))?;
            (output.service)(Action::Feedback(tick_us))?;
            // No bursts to catch up slots consumed by MIDI, logging or native I/O.
            next_tick_us = tick_us.saturating_add(INTERVAL_US);
            continue;
        }
        let wait_us = next_tick_us
            .min(engine.input_deadline_us())
            .min(duration_us.unwrap_or(u64::MAX))
            .saturating_sub(now_us);
        if wait_us == 0 {
            bail!("no fresh tension input within 50ms");
        }
        select_biased! {
            recv(stop) -> _ => return Ok(()),
            recv(reader.done) -> result => {
                reader.completed = true;
                result.context("snapshot completion channel closed")??;
                bail!("snapshot reader stopped unexpectedly");
            },
            recv(reader.frames) -> frame => {
                let frame = frame.context("snapshot channel closed")?;
                let now_us = clock()?;
                if terminal(reader, stop, duration_us, now_us)? {
                    return Ok(());
                }
                output.sample(engine, &frame, now_us)?;
            },
            default(Duration::from_micros(wait_us)) => {}
        }
    }
}

fn terminal(
    reader: &mut Reader,
    stop: &Receiver<()>,
    duration_us: Option<u64>,
    now_us: u64,
) -> anyhow::Result<bool> {
    match stop.try_recv() {
        Ok(()) | Err(TryRecvError::Disconnected) => return Ok(true),
        Err(TryRecvError::Empty) => {}
    }
    match reader.done.try_recv() {
        Ok(result) => {
            reader.completed = true;
            result?;
            bail!("snapshot reader stopped unexpectedly");
        }
        Err(TryRecvError::Disconnected) => bail!("snapshot completion channel closed"),
        Err(TryRecvError::Empty) => {}
    }
    Ok(duration_us.is_some_and(|deadline| now_us >= deadline))
}

#[cfg(test)]
mod tests;
