use super::decode::tests::{frame, with_fader};
use super::motor::neutral_report;
use super::session::{Event, Session};
use super::smooth_tests::control;
use anyhow::Context as _;

#[test]
fn preparation_gates_qualified_motion_until_stable_neutral_ready() -> anyhow::Result<()> {
    // Given a transient center crossing that arms the controller but not READY.
    let mut log = Vec::new();
    let mut writes = Vec::new();
    {
        let mut session = Session::new(
            control(),
            |r| {
                writes.push(r.to_vec());
                Ok(r.len())
            },
            &mut log,
        )?;
        session.prepare_manual()?;
        session.handle(Event::Sample(with_fader(frame(0, 1, [1000, 0]), 2048)), 0)?;
        // When sustained qualified movement occurs while preparation is pending.
        for index in 1..=50 {
            let at = u64::from(index) * 2000;
            session.handle(
                Event::Sample(with_fader(
                    frame(at, index + 1, [1000 + 2 * index, 0]),
                    4095,
                )),
                at,
            )?;
            session.handle(Event::OutputTick, at)?;
        }
        for index in 0..=20 {
            let at = 102_000 + u64::from(index) * 50000;
            session.handle(
                Event::Sample(with_fader(frame(at, 52 + index, [1100, 0]), 2048)),
                at,
            )?;
            session.handle(Event::OutputTick, at)?;
        }
        for index in 1..=12 {
            let at = 1_102_000 + u64::from(index) * 2000;
            session.handle(
                Event::Sample(with_fader(
                    frame(at, 72 + index, [1100 + 2 * index, 0]),
                    4095,
                )),
                at,
            )?;
            session.handle(Event::OutputTick, at)?;
        }
        session.handle(Event::Stop, 1_128_000)?;
    }
    // Then every actual write before READY was zero, but later force was real.
    let records = std::str::from_utf8(&log)?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    let ready = records
        .iter()
        .position(|r| r["kind"] == "ready")
        .context("READY")?;
    assert_eq!(records[ready]["host_us"], 1_102_000);
    assert!(
        records[..ready]
            .iter()
            .filter(|r| r["kind"] == "motor")
            .all(|r| r["report"] == serde_json::json!(neutral_report()))
    );
    assert!(writes.iter().any(|r| r[4..6] != [0, 0]));
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    assert!(
        records
            .iter()
            .any(|r| r["kind"] == "preparation" && r["seconds"].is_null())
    );
    Ok(())
}
