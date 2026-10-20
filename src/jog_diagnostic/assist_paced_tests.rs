use super::*;
use crate::jog_diagnostic::assist_session_tests::prime;
use crate::jog_diagnostic::assist_tests::{control, input};
use crate::jog_diagnostic::motor::neutral_report;
use crossbeam_channel::bounded;
use std::cell::Cell;

#[test]
fn queued_touch_release_is_processed_before_a_due_assist_tick() -> anyhow::Result<()> {
    let (mut reader, frames_tx, _done_tx) = Reader::channel_only();
    frames_tx.send(input(13, 1026, false))?;
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
        // Given actual assist output and a release queued before the due tick.
        prime(&mut session)?;
        active.set(true);
        // When the real production loop selects among the ready events.
        run_clocked(&mut session, &mut reader, (&stop, || Ok(26000), None))?;
    }
    // Then the immediate sample write is neutral; no extra force tick slips through.
    assert_eq!(applied, [neutral_report().to_vec()]);
    let rows = std::str::from_utf8(&log)?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(rows.iter().filter(|r| r["kind"] == "tick").count(), 12);
    assert!(
        rows.iter()
            .any(|r| r["kind"] == "motor" && r["reason"] == "motion")
    );
    Ok(())
}

#[test]
fn assist_output_clock_continues_without_samples_but_rest_cuts_force() -> anyhow::Result<()> {
    let (mut reader, _frames_tx, _done_tx) = Reader::channel_only();
    let (stop_tx, stop) = bounded(1);
    let at = Cell::new(26000_u64);
    let calls = Cell::new(0);
    let active = Cell::new(false);
    let mut applied = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            control(),
            |r| {
                if active.get() {
                    applied.push((at.get(), u16::from_le_bytes([r[4], r[5]])));
                    if at.get() == 32000 {
                        active.set(false);
                        stop_tx.send(())?;
                    } else {
                        at.set(at.get() + 2000);
                    }
                }
                Ok(r.len())
            },
            &mut log,
        )?;
        // Given actual force and a Stop subscription installed before triggering.
        prime(&mut session)?;
        active.set(true);
        // When production output slots run without fresh motion.
        run_clocked(
            &mut session,
            &mut reader,
            (
                &stop,
                || {
                    calls.set(calls.get() + 1);
                    ensure!(calls.get() < 100, "clock stalled");
                    Ok(at.get())
                },
                None,
            ),
        )?;
    }
    // Then clock writes continue, but the 8ms cutoff overrides smoothing.
    assert_eq!(
        applied.iter().map(|(time, _)| *time).collect::<Vec<_>>(),
        [26000, 28000, 30000, 32000]
    );
    assert!(applied[..3].iter().all(|(_, force)| *force > 0));
    assert_eq!(applied[3].1, 0);
    Ok(())
}
