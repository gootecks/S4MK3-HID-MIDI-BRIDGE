//! Opt-in 2 ms output cadence for the jog diagnostic variant.

use std::time::{Duration, Instant};

use anyhow::{Context as _, ensure};
use crossbeam_channel::{Receiver, select_biased};

use super::continuity::OUTPUT_INTERVAL_US;
use super::reader::Reader;
use super::session::{Event, Session};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Scheduler {
    next_tick_us: u64,
}

impl Scheduler {
    const fn new(now_us: u64) -> Self {
        Self {
            next_tick_us: now_us.saturating_add(OUTPUT_INTERVAL_US),
        }
    }

    const fn due(&mut self, now_us: u64) -> bool {
        if now_us < self.next_tick_us {
            return false;
        }
        let missed = (now_us - self.next_tick_us) / OUTPUT_INTERVAL_US + 1;
        self.next_tick_us = self
            .next_tick_us
            .saturating_add(missed.saturating_mul(OUTPUT_INTERVAL_US));
        true
    }

    const fn remaining_us(self, now_us: u64) -> u64 {
        self.next_tick_us.saturating_sub(now_us)
    }
}

pub(super) fn run<F: FnMut(&[u8]) -> anyhow::Result<usize>>(
    session: &mut Session<'_, F>,
    reader: &mut Reader,
    timing: (&Receiver<()>, Instant, Option<u64>),
) -> anyhow::Result<()> {
    run_clocked(
        session,
        reader,
        (timing.0, || elapsed_us(timing.1), timing.2),
    )
}

fn run_clocked<F, C>(
    session: &mut Session<'_, F>,
    reader: &mut Reader,
    timing: (&Receiver<()>, C, Option<u64>),
) -> anyhow::Result<()>
where
    F: FnMut(&[u8]) -> anyhow::Result<usize>,
    C: Fn() -> anyhow::Result<u64> + Copy,
{
    ensure!(
        session.is_paced(),
        "paced loop requires fader-continuity mode"
    );
    let mut scheduler = Scheduler::new(0);
    loop {
        if terminal(session, reader, timing)? {
            return Ok(());
        }
        let now_us = timing.1()?;
        if scheduler.due(now_us) {
            // Drain at most the bounded queue capacity so an already-queued
            // release/center update cannot be hidden by older motion.
            for _ in 0..32 {
                if terminal(session, reader, timing)? {
                    return Ok(());
                }
                match reader.frames.try_recv() {
                    Ok(frame) => {
                        session.handle(Event::Sample(frame), timing.1()?)?;
                    }
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        session.handle(
                            Event::ReaderError(anyhow::anyhow!("snapshot channel closed")),
                            timing.1()?,
                        )?;
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => break,
                }
            }
            if terminal(session, reader, timing)? {
                return Ok(());
            }
            session.handle(Event::OutputTick, timing.1()?)?;
            continue;
        }

        let input_deadline_us = session.input_deadline_us();
        let input_wait_us = input_deadline_us.saturating_sub(now_us);
        if input_wait_us == 0 {
            session.handle(Event::InputTimeout, now_us)?;
            return Ok(());
        }
        let tick_wait_us = scheduler.remaining_us(now_us);
        let duration_wait = timing
            .2
            .and_then(|seconds| session.remaining_duration_us(now_us, seconds))
            .map_or(Duration::MAX, Duration::from_micros);
        let timeout = Duration::from_micros(input_wait_us.min(tick_wait_us)).min(duration_wait);
        select_biased! {
            recv(timing.0) -> _ => {
                session.handle(Event::Stop, timing.1()?)?;
                return Ok(());
            },
            recv(reader.done) -> result => {
                reader.completed = true;
                let error = result.context("reader completion channel closed")
                    .and_then(|result| result)
                    .err()
                    .unwrap_or_else(|| anyhow::anyhow!("reader stopped unexpectedly"));
                session.handle(Event::ReaderError(error), timing.1()?)?;
                return Ok(());
            }
            recv(reader.frames) -> frame => {
                let event = match frame {
                    Ok(frame) => Event::Sample(frame),
                    Err(error) => Event::ReaderError(error.into()),
                };
                let now_us = timing.1()?;
                if timing.2.is_some_and(|seconds| session.remaining_duration_us(now_us, seconds) == Some(0)) {
                    session.handle(Event::Deadline, now_us)?;
                    return Ok(());
                }
                if !session.handle(event, now_us)? {
                    return Ok(());
                }
            },
            default(timeout) => {
                let now_us = timing.1()?;
                if timing.2.is_some_and(|seconds| session.remaining_duration_us(now_us, seconds) == Some(0)) {
                    session.handle(Event::Deadline, now_us)?;
                    return Ok(());
                }
                if now_us >= session.input_deadline_us() {
                    session.handle(Event::InputTimeout, now_us)?;
                    return Ok(());
                }
                // The next loop emits a tick if its scheduled slot has arrived.
            },
        }
    }
}

fn terminal<F, C>(
    session: &mut Session<'_, F>,
    reader: &mut Reader,
    timing: (&Receiver<()>, C, Option<u64>),
) -> anyhow::Result<bool>
where
    F: FnMut(&[u8]) -> anyhow::Result<usize>,
    C: Fn() -> anyhow::Result<u64> + Copy,
{
    match timing.0.try_recv() {
        Ok(()) | Err(crossbeam_channel::TryRecvError::Disconnected) => {
            session.handle(Event::Stop, timing.1()?)?;
            return Ok(true);
        }
        Err(crossbeam_channel::TryRecvError::Empty) => {}
    }
    match reader.done.try_recv() {
        Ok(result) => {
            reader.completed = true;
            let error = result
                .err()
                .unwrap_or_else(|| anyhow::anyhow!("reader stopped unexpectedly"));
            session.handle(Event::ReaderError(error), timing.1()?)?;
            return Ok(true);
        }
        Err(crossbeam_channel::TryRecvError::Disconnected) => {
            session.handle(
                Event::ReaderError(anyhow::anyhow!("reader completion channel closed")),
                timing.1()?,
            )?;
            return Ok(true);
        }
        Err(crossbeam_channel::TryRecvError::Empty) => {}
    }
    let now_us = timing.1()?;
    if timing
        .2
        .is_some_and(|seconds| session.remaining_duration_us(now_us, seconds) == Some(0))
    {
        session.handle(Event::Deadline, now_us)?;
        return Ok(true);
    }
    Ok(false)
}

fn elapsed_us(started: Instant) -> anyhow::Result<u64> {
    Ok(u64::try_from(started.elapsed().as_micros())?)
}

#[cfg(test)]
#[path = "paced_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "smooth_paced_tests.rs"]
mod smooth_tests;

#[cfg(test)]
#[path = "assist_paced_tests.rs"]
mod assist_tests;

#[cfg(test)]
#[path = "wheels_paced_tests.rs"]
mod wheels_tests;
