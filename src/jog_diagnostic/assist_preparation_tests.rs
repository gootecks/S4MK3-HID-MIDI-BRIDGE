use super::assist_tests::{control, input};
use super::decode::tests::with_fader;
use super::motor::neutral_report;
use super::session::{Event, Session};
use anyhow::Context as _;

#[test]
fn assist_is_gated_until_continuous_neutral_ready_and_then_requalifies() -> anyhow::Result<()> {
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
        // Given transient center arming, then internally qualifying held movement.
        for index in 0..=40 {
            let at = u64::from(index) * 2000;
            session.handle(Event::Sample(input(index, 1000 + index * 2, true)), at)?;
            if index > 0 {
                session.handle(Event::OutputTick, at)?;
            }
        }
        // When one second of continuous neutral is completed without a deadline.
        for index in 41..=541 {
            let at = u64::from(index) * 2000;
            session.handle(
                Event::Sample(with_fader(input(index, 1080, true), 2048)),
                at,
            )?;
            session.handle(Event::OutputTick, at)?;
        }
        for index in 542..=553 {
            let at = u64::from(index) * 2000;
            session.handle(
                Event::Sample(input(index, 1080 + (index - 541) * 2, true)),
                at,
            )?;
            session.handle(Event::OutputTick, at)?;
        }
        session.handle(Event::Stop, 1_108_000)?;
    }
    // Then output before READY is zero, and only freshly qualified movement assists.
    let rows = std::str::from_utf8(&log)?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    let ready = rows
        .iter()
        .position(|r| r["kind"] == "ready")
        .context("READY")?;
    assert_eq!(rows[ready]["host_us"], 1_082_000);
    assert!(
        rows[..ready]
            .iter()
            .filter(|r| r["kind"] == "motor")
            .all(|r| r["report"] == serde_json::json!(neutral_report()))
    );
    assert!(
        rows.iter()
            .any(|r| r["kind"] == "preparation" && r["seconds"].is_null())
    );
    assert!(writes.iter().any(|r| r[4..6] != [0, 0]));
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    Ok(())
}
