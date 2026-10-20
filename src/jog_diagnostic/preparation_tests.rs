use super::*;
use crate::jog_diagnostic::capture::RawFrame;
use crate::jog_diagnostic::decode::tests::{frame, with_fader};
use crate::jog_diagnostic::motor::neutral_report;
use crate::jog_diagnostic::session::{Event, Session};

#[path = "prepared_output_tests.rs"]
mod active_window;
use crossbeam_channel::Sender;
use std::cell::Cell;

fn fader_control(mode: Mode) -> ControlArgs {
    ControlArgs {
        mode,
        side: Side::Left,
        gain: 1,
        max_force: 30_000,
        assist_force: Some(8_000),
    }
}

fn rows(log: &[u8]) -> anyhow::Result<Vec<serde_json::Value>> {
    std::str::from_utf8(log)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .map_err(Into::into)
}

pub(super) fn assert_paced_expiry(
    log: &[u8],
    writes: &[Vec<u8>],
    now_us: u64,
) -> anyhow::Result<()> {
    assert!(writes.iter().all(|report| report == &neutral_report()));
    let logged = rows(log)?;
    let ready: Vec<_> = logged.iter().filter(|row| row["kind"] == "ready").collect();
    assert_eq!(ready.len(), 1);
    let ready_us = ready[0]["host_us"]
        .as_u64()
        .context("ready host time missing")?;
    assert!(ready_us > 30_000_000);
    assert!(now_us >= ready_us + 1_000_000);
    assert!(now_us < ready_us + 1_050_000);
    assert!(
        logged
            .iter()
            .any(|row| row["kind"] == "end" && row["reason"] == "duration")
    );
    Ok(())
}

pub(super) fn paced_preparation_writer<'a>(
    clock: &'a Cell<u64>,
    active: &'a Cell<bool>,
    ticks: &'a Cell<u16>,
    last_write_us: &'a Cell<u64>,
    frames_tx: Sender<RawFrame>,
    writes: &'a mut Vec<Vec<u8>>,
) -> impl FnMut(&[u8]) -> anyhow::Result<usize> + 'a {
    move |report| {
        last_write_us.set(clock.get());
        writes.push(report.to_vec());
        if active.get() {
            let at = clock.get() + 50_000;
            clock.set(at);
            ticks.set(ticks.get() + 1);
            frames_tx
                .try_send(with_fader(
                    frame(at, 602 + u32::from(ticks.get()), [600, 0]),
                    2048,
                ))
                .map_err(|error| anyhow::anyhow!("{error}"))?;
        }
        Ok(report.len())
    }
}

pub(super) fn bounded_test_clock<'a>(
    clock: &'a Cell<u64>,
    calls: &'a Cell<u16>,
) -> impl Fn() -> anyhow::Result<u64> + Copy + 'a {
    move || {
        calls.set(calls.get() + 1);
        anyhow::ensure!(calls.get() < 2000, "paced loop made no progress");
        Ok(clock.get())
    }
}

pub(super) fn prime_paced_preparation<F>(session: &mut Session<'_, F>) -> anyhow::Result<()>
where
    F: FnMut(&[u8]) -> anyhow::Result<usize>,
{
    session.prepare(1)?;
    for index in 0_u32..=600 {
        let at = u64::from(index) * 50_000;
        let raw = if index > 0 && index % 18 == 0 {
            0
        } else {
            2048
        };
        session.handle(
            Event::Sample(with_fader(frame(at, index + 1, [index, 0]), raw)),
            at,
        )?;
    }
    assert_eq!(session.remaining_duration_us(30_000_000, 1), None);
    Ok(())
}

#[test]
fn preparation_requires_one_continuous_neutral_second() -> anyhow::Result<()> {
    let mut writes = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            fader_control(Mode::FaderTension),
            |report| {
                writes.push(report.to_vec());
                Ok(report.len())
            },
            &mut log,
        )?;
        session.prepare(5)?;

        // A transient center reading arms the real controller, but motion
        // outside the band must remain neutral and reset the hold countdown.
        session.handle(Event::Sample(with_fader(frame(0, 1, [0, 0]), 2048)), 0)?;
        session.handle(
            Event::Sample(with_fader(frame(20_000, 2, [20, 0]), 0)),
            20_000,
        )?;
        assert_eq!(session.remaining_duration_us(20_000, 5), None);

        session.handle(
            Event::Sample(with_fader(frame(40_000, 3, [40, 0]), 2016)),
            40_000,
        )?;
        assert_eq!(session.remaining_duration_us(1_039_999, 5), None);
        assert_eq!(session.remaining_duration_us(1_040_000, 5), None);

        session.handle(
            Event::Sample(with_fader(frame(60_000, 4, [60, 0]), 4095)),
            60_000,
        )?;
        assert_eq!(session.remaining_duration_us(1_040_000, 5), None);
        session.handle(
            Event::Sample(with_fader(frame(80_000, 5, [80, 0]), 2080)),
            80_000,
        )?;
        assert_eq!(session.remaining_duration_us(1_079_999, 5), None);
        assert_eq!(session.remaining_duration_us(1_080_000, 5), None);

        // Until ready, both the actual report and the proposed decision stay
        // zero even though the controller's armed state is now sticky.
        session.handle(
            Event::Sample(with_fader(frame(100_000, 6, [100, 0]), 0)),
            100_000,
        )?;
        assert_eq!(session.remaining_duration_us(1_080_000, 5), None);

        // Restart at a fresh center reading; readiness is relative to that
        // stable hold, with a full requested duration starting at READY.
        for index in 0_u32..=20 {
            let at = 120_000 + u64::from(index) * 50_000;
            session.handle(
                Event::Sample(with_fader(frame(at, index + 7, [100, 0]), 2048)),
                at,
            )?;
        }
        assert_eq!(session.remaining_duration_us(2_120_001, 5), Some(3_999_999));
    }
    assert!(writes.iter().all(|report| report == &neutral_report()));
    let logged = rows(&log)?;
    assert!(
        logged
            .iter()
            .filter(|row| row["kind"] == "motion")
            .all(|row| row["decision"]["torque"] == serde_json::json!([0, 0])
                && row["decision"]["report"] == serde_json::json!(neutral_report()))
    );
    let preparation: Vec<_> = logged
        .iter()
        .filter(|row| row["kind"] == "preparation")
        .collect();
    assert_eq!(preparation.len(), 1);
    assert_eq!(preparation[0]["seconds"], 5);
    assert_eq!(preparation[0]["neutral_hold_us"], 1_000_000);
    let ready: Vec<_> = logged.iter().filter(|row| row["kind"] == "ready").collect();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0]["host_us"], 1_120_000);
    Ok(())
}

#[test]
fn unprepared_session_duration_remains_launch_relative() -> anyhow::Result<()> {
    let mut log = Vec::new();
    let session = Session::new(
        fader_control(Mode::FaderTension),
        |report| Ok(report.len()),
        &mut log,
    )?;
    assert_eq!(session.remaining_duration_us(2_000_000, 5), Some(3_000_000));
    Ok(())
}

#[test]
fn watchdog_stop_and_reader_fault_remain_active_during_preparation() -> anyhow::Result<()> {
    for terminal in [
        Event::InputTimeout,
        Event::Stop,
        Event::ReaderError(anyhow::anyhow!("reader failed during preparation")),
    ] {
        let mut writes = Vec::new();
        let mut log = Vec::new();
        let mut session = Session::new(
            fader_control(Mode::FaderTension),
            |report| {
                writes.push(report.to_vec());
                Ok(report.len())
            },
            &mut log,
        )?;
        session.prepare(1)?;
        session.handle(Event::Sample(with_fader(frame(0, 1, [0, 0]), 2048)), 0)?;
        let result = session.handle(terminal, 1);
        assert!(!matches!(result, Ok(true)));
        drop(session);
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}

#[test]
fn prepared_duration_expires_relative_to_ready_time() -> anyhow::Result<()> {
    let mut log = Vec::new();
    let mut session = Session::new(
        fader_control(Mode::FaderContinuity),
        |report| Ok(report.len()),
        &mut log,
    )?;
    prime_paced_preparation(&mut session)?;
    for index in 1_u32..=20 {
        let at = 30_000_000 + u64::from(index) * 50_000;
        session.handle(
            Event::Sample(with_fader(frame(at, index + 602, [600, 0]), 2048)),
            at,
        )?;
    }
    assert_eq!(
        session.remaining_duration_us(30_750_000, 1),
        Some(1_000_000)
    );
    assert_eq!(session.remaining_duration_us(31_749_999, 1), Some(1));
    assert_eq!(session.remaining_duration_us(31_750_000, 1), Some(0));
    Ok(())
}
