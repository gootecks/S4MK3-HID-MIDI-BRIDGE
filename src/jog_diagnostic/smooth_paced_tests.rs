use super::*;
use crate::jog_diagnostic::decode::tests::{frame, with_fader};
use crate::jog_diagnostic::motor::neutral_report;
use crate::jog_diagnostic::smooth_session_tests::prime;
use crate::jog_diagnostic::smooth_tests::control;
use crossbeam_channel::bounded;
use std::cell::Cell;

#[test]
fn e_production_clock_continues_between_samples_and_enforces_rest_cutoff() -> anyhow::Result<()> {
    // Given real force, an explicit clock, and subscribed Stop before running.
    let (mut reader, _frames_tx, _done_tx) = Reader::channel_only();
    let (stop_tx, stop) = bounded(1);
    let clock = Cell::new(26000_u64);
    let calls = Cell::new(0);
    let active = Cell::new(false);
    let mut applied = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            control(),
            |r| {
                if active.get() {
                    let at = clock.get();
                    applied.push((at, u16::from_le_bytes([r[4], r[5]])));
                    if at == 32000 {
                        active.set(false);
                        stop_tx.send(())?;
                    } else {
                        clock.set(at + 2000);
                    }
                }
                Ok(r.len())
            },
            &mut log,
        )?;
        prime(&mut session)?;
        active.set(true);
        // When the production loop runs without any further input.
        run_clocked(
            &mut session,
            &mut reader,
            (
                &stop,
                || {
                    calls.set(calls.get() + 1);
                    ensure!(calls.get() < 100, "clock made no progress");
                    Ok(clock.get())
                },
                None,
            ),
        )?;
    }
    // Then periodic outputs persist but cannot carry force beyond 8ms.
    assert_eq!(
        applied.iter().map(|(at, _)| *at).collect::<Vec<_>>(),
        [26000, 28000, 30000, 32000]
    );
    assert!(applied[..3].iter().all(|(_, force)| *force > 0));
    assert_eq!(applied[3].1, 0);
    Ok(())
}

#[test]
fn queued_center_writes_zero_before_the_overdue_e_output_slot() -> anyhow::Result<()> {
    // Given real force and a center update queued before a due slot.
    let (mut reader, frames_tx, _done_tx) = Reader::channel_only();
    frames_tx.send(with_fader(frame(26000, 14, [1024, 0]), 2048))?;
    let (stop_tx, stop) = bounded(1);
    let active = Cell::new(false);
    let mut applied = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            control(),
            |r| {
                if active.replace(false) {
                    applied.push(r.to_vec());
                    stop_tx.send(())?;
                }
                Ok(r.len())
            },
            &mut log,
        )?;
        prime(&mut session)?;
        active.set(true);
        // When both the queued sample and scheduled tick are ready.
        run_clocked(&mut session, &mut reader, (&stop, || Ok(26000), None))?;
    }
    // Then the immediate sample write, not a clock slot, supplies neutral.
    assert_eq!(applied, [neutral_report().to_vec()]);
    let records = std::str::from_utf8(&log)?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(records.iter().filter(|r| r["kind"] == "tick").count(), 12);
    assert_eq!(
        records
            .iter()
            .filter(|r| r["kind"] == "motor" && r["reason"] == "motion")
            .count(),
        1
    );
    Ok(())
}
