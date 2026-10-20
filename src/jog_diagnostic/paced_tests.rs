use super::*;
use crate::jog_diagnostic::capture::RawFrame;
use crate::jog_diagnostic::continuity::OUTPUT_INTERVAL_US;
use crate::jog_diagnostic::motor::neutral_report;
use crate::jog_diagnostic::{ControlArgs, Mode, Side};
use crossbeam_channel::bounded;
use std::time::{Duration, Instant};

#[test]
fn production_loop_outputs_on_clock_slots_without_more_input() -> anyhow::Result<()> {
    use crate::jog_diagnostic::decode::tests::{frame, with_fader};
    use std::cell::Cell;

    // Given actual armed assistance, a silent reader and a pre-subscribed Stop.
    let (mut reader, _frames_tx, _done_tx) = Reader::channel_only();
    let (stop_tx, stop) = bounded(1);
    let clock = Cell::new(2000_u64);
    let clock_calls = Cell::new(0_u16);
    let running = Cell::new(false);
    let mut applied = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            control(),
            |report| {
                if running.get() {
                    let at = clock.get();
                    applied.push((at, u16::from_le_bytes([report[4], report[5]])));
                    if at == 10000 {
                        running.set(false);
                        stop_tx.send(())?;
                    } else {
                        clock.set(at + OUTPUT_INTERVAL_US);
                    }
                }
                Ok(report.len())
            },
            &mut log,
        )?;
        session.handle(Event::Sample(with_fader(frame(0, 1, [0, 0]), 2048)), 0)?;
        session.handle(Event::Sample(with_fader(frame(2000, 2, [10, 0]), 0)), 2000)?;
        running.set(true);
        // When the production loop runs against an explicitly advanced clock.
        run_clocked(
            &mut session,
            &mut reader,
            (
                &stop,
                || {
                    clock_calls.set(clock_calls.get() + 1);
                    ensure!(clock_calls.get() < 100, "paced loop made no progress");
                    Ok(clock.get())
                },
                None,
            ),
        )?;
    }
    // Then output continues between samples and fades to zero at the fixed bound.
    assert_eq!(
        applied,
        [
            (2000, 8000),
            (4000, 6000),
            (6000, 4000),
            (8000, 2000),
            (10000, 0)
        ]
    );
    Ok(())
}

fn control() -> ControlArgs {
    ControlArgs {
        mode: Mode::FaderContinuity,
        side: Side::Left,
        gain: 1,
        max_force: 30_000,
        assist_force: Some(8_000),
    }
}

#[test]
fn queued_release_is_applied_before_an_overdue_force_tick() -> anyhow::Result<()> {
    use crate::jog_diagnostic::decode::tests::{frame, with_fader};
    use std::cell::Cell;

    // Given assistance and a queued release before an overdue output slot.
    let (mut reader, frames_tx, _done_tx) = Reader::channel_only();
    let mut released = with_fader(frame(4000, 3, [10, 0]), 0);
    released.buttons[17] = 0;
    frames_tx.send(released)?;
    let (stop_tx, stop) = bounded(1);
    let running = Cell::new(false);
    let mut applied = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            control(),
            |report| {
                if running.replace(false) {
                    applied.push(report.to_vec());
                    stop_tx.send(())?;
                }
                Ok(report.len())
            },
            &mut log,
        )?;
        session.handle(Event::Sample(with_fader(frame(0, 1, [0, 0]), 2048)), 0)?;
        session.handle(Event::Sample(with_fader(frame(2000, 2, [10, 0]), 0)), 2000)?;
        running.set(true);
        // When the actual production selection loop sees both available.
        run_clocked(&mut session, &mut reader, (&stop, || Ok(4000), None))?;
    }
    // Then release, not another force tick, supplies the first actual output.
    assert_eq!(reader.frames.len(), 0);
    assert_eq!(applied, [neutral_report().to_vec()]);
    Ok(())
}

#[test]
fn scheduler_advances_on_fixed_slots_without_catch_up_bursts() {
    let mut scheduler = Scheduler::new(0);
    assert_eq!(scheduler.next_tick_us, OUTPUT_INTERVAL_US);
    assert!(!scheduler.due(OUTPUT_INTERVAL_US - 1));
    assert!(scheduler.due(OUTPUT_INTERVAL_US));
    assert_eq!(scheduler.next_tick_us, 2 * OUTPUT_INTERVAL_US);
    assert!(!scheduler.due(2 * OUTPUT_INTERVAL_US - 1));

    // A delayed wake emits one tick and skips all missed slots.
    assert!(scheduler.due(5 * OUTPUT_INTERVAL_US + OUTPUT_INTERVAL_US / 4));
    assert_eq!(scheduler.next_tick_us, 6 * OUTPUT_INTERVAL_US);
    assert!(!scheduler.due(5 * OUTPUT_INTERVAL_US + OUTPUT_INTERVAL_US / 4));
    assert!(scheduler.due(6 * OUTPUT_INTERVAL_US));
    assert_eq!(scheduler.next_tick_us, 7 * OUTPUT_INTERVAL_US);
}

fn test_frame() -> RawFrame {
    RawFrame {
        host_us: 0,
        buttons: vec![0; 16],
        jog: vec![0; 5],
        analog: None,
    }
}

#[test]
fn stop_precedes_queued_input_in_the_production_loop() -> anyhow::Result<()> {
    let (mut reader, frames_tx, _done_tx) = Reader::channel_only();
    frames_tx.send(test_frame())?;
    let (stop_tx, stop) = bounded(1);
    stop_tx.send(())?;
    let started = Instant::now()
        .checked_sub(Duration::from_micros(2 * OUTPUT_INTERVAL_US))
        .context("time origin")?;
    let mut writes = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            control(),
            |report| {
                writes.push(report.to_vec());
                Ok(report.len())
            },
            &mut log,
        )?;
        run(&mut session, &mut reader, (&stop, started, None))?;
    }
    assert_eq!(reader.frames.len(), 1);
    assert_eq!(writes.len(), 2, "overdue output tick must not beat Stop");
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    Ok(())
}

#[test]
fn expired_duration_precedes_queued_input_in_the_production_loop() -> anyhow::Result<()> {
    let (mut reader, frames_tx, _done_tx) = Reader::channel_only();
    frames_tx.send(test_frame())?;
    let (_stop_tx, stop) = bounded(1);
    let mut writes = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            control(),
            |report| {
                writes.push(report.to_vec());
                Ok(report.len())
            },
            &mut log,
        )?;
        run(&mut session, &mut reader, (&stop, Instant::now(), Some(0)))?;
    }
    assert_eq!(reader.frames.len(), 1);
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    Ok(())
}

#[test]
fn reader_error_precedes_queued_input_in_the_production_loop() -> anyhow::Result<()> {
    let (mut reader, frames_tx, done_tx) = Reader::channel_only();
    frames_tx.send(test_frame())?;
    done_tx.send(Err(anyhow::anyhow!("injected reader failure")))?;
    let (_stop_tx, stop) = bounded(1);
    let started = Instant::now()
        .checked_sub(Duration::from_micros(2 * OUTPUT_INTERVAL_US))
        .context("time origin")?;
    let mut writes = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            control(),
            |report| {
                writes.push(report.to_vec());
                Ok(report.len())
            },
            &mut log,
        )?;
        let error = run(&mut session, &mut reader, (&stop, started, None))
            .err()
            .context("reader error must terminate the loop")?;
        assert!(error.to_string().contains("injected reader failure"));
    }
    assert_eq!(reader.frames.len(), 1);
    assert_eq!(
        writes.len(),
        2,
        "overdue output tick must not beat reader error"
    );
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    Ok(())
}

#[test]
fn explicit_clock_preserves_preparation_and_expires_after_ready() -> anyhow::Result<()> {
    use std::cell::Cell;

    let (mut reader, frames_tx, _done_tx) = Reader::channel_only();
    let (_stop_tx, stop) = bounded(1);
    let clock = Cell::new(30_000_000_u64);
    let active = Cell::new(false);
    let ticks = Cell::new(0_u16);
    let calls = Cell::new(0_u16);
    let last_write_us = Cell::new(0_u64);
    let mut log = Vec::new();
    let mut writes = Vec::new();
    let sink = crate::jog_diagnostic::preparation_tests::paced_preparation_writer(
        &clock,
        &active,
        &ticks,
        &last_write_us,
        frames_tx,
        &mut writes,
    );
    let mut session = Session::new(control(), sink, &mut log)?;
    crate::jog_diagnostic::preparation_tests::prime_paced_preparation(&mut session)?;
    active.set(true);
    run_clocked(
        &mut session,
        &mut reader,
        (
            &stop,
            crate::jog_diagnostic::preparation_tests::bounded_test_clock(&clock, &calls),
            Some(1),
        ),
    )?;
    drop(session);
    crate::jog_diagnostic::preparation_tests::assert_paced_expiry(
        &log,
        &writes,
        last_write_us.get(),
    )?;
    Ok(())
}
