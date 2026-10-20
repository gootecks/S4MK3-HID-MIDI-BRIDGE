use super::*;
use crate::jog_diagnostic::Side;
use crate::jog_diagnostic::motor::neutral_report;
use crate::jog_diagnostic::wheels_tests::{control, input};
use crossbeam_channel::bounded;
use std::cell::Cell;

#[test]
fn queued_single_wheel_release_precedes_overdue_both_wheel_tick() -> anyhow::Result<()> {
    let (mut reader, tx, _done) = Reader::channel_only();
    tx.send(input(13, [1026, 1974], (0, 0x10)))?;
    let (stop_tx, stop) = bounded(1);
    let active = Cell::new(false);
    let mut applied = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            control(Side::Both),
            |r| {
                if active.replace(false) {
                    applied.push(r.to_vec());
                    stop_tx.send(())?;
                }
                Ok(r.len())
            },
            &mut log,
        )?;
        for index in 0..=12 {
            let at = u64::from(index) * 2000;
            session.handle(
                Event::Sample(input(
                    index,
                    [1000 + index * 2, 2000 - index * 2],
                    (if index == 0 { 2048 } else { 0 }, 0x30),
                )),
                at,
            )?;
            if index > 0 {
                session.handle(Event::OutputTick, at)?;
            }
        }
        // Given active opposite forces, when right release and output tick are ready.
        active.set(true);
        run_clocked(&mut session, &mut reader, (&stop, || Ok(26000), None))?;
    }
    // Then right is neutralized without a tick or stopping the left force.
    assert_eq!(applied.len(), 1);
    assert_ne!(&applied[0][4..6], &[0, 0]);
    assert_eq!(&applied[0][6..11], &neutral_report()[6..11]);
    let rows = std::str::from_utf8(&log)?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(rows.iter().filter(|r| r["kind"] == "tick").count(), 12);
    Ok(())
}
