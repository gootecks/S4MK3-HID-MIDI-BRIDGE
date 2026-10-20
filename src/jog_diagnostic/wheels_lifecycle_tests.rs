use super::Side;
use super::motor::neutral_report;
use super::session::{Event, Session};
use super::wheels_tests::{control, input};
use anyhow::Context as _;

fn prime<F: FnMut(&[u8]) -> anyhow::Result<usize>>(s: &mut Session<'_, F>) -> anyhow::Result<()> {
    for index in 0..=12 {
        let at = u64::from(index) * 2000;
        s.handle(
            Event::Sample(input(
                index,
                [1000 + index * 2, 2000 - index * 2],
                (if index == 0 { 2048 } else { 0 }, 0x30),
            )),
            at,
        )?;
        if index > 0 {
            s.handle(Event::OutputTick, at)?;
        }
    }
    Ok(())
}

#[test]
fn both_wheel_stop_and_fault_cleanup_clear_both_native_blocks() -> anyhow::Result<()> {
    for (event, at) in [
        (Event::Stop, 26000),
        (Event::InputTimeout, 26000),
        (Event::ReaderError(anyhow::anyhow!("lost input")), 26000),
        (Event::OutputTick, 80000),
    ] {
        let mut writes = Vec::new();
        let mut log = Vec::new();
        {
            let mut session = Session::new(
                control(Side::Both),
                |r| {
                    writes.push(r.to_vec());
                    Ok(r.len())
                },
                &mut log,
            )?;
            prime(&mut session)?;
            // Given actual nonzero requests on both wheels, when a stop/fault occurs.
            assert!(!matches!(session.handle(event, at), Ok(true)));
        }
        // Then a real full-neutral report follows the two-wheel force.
        assert!(
            writes
                .iter()
                .any(|r| r[4..6] != [0, 0] && r[9..11] != [0, 0])
        );
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}

#[test]
fn right_block_transport_failure_attempts_full_neutral() -> anyhow::Result<()> {
    let mut writes = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            control(Side::Both),
            |r| {
                writes.push(r.to_vec());
                if r[9..11] != [0, 0] {
                    anyhow::bail!("right block failed");
                }
                Ok(r.len())
            },
            &mut log,
        )?;
        // Given a failing right-force transport, when both wheels engage.
        let error = prime(&mut session).err().context("right force must fail")?;
        assert!(error.to_string().contains("right block failed"));
    }
    // Then error propagation retains the full two-wheel cleanup attempt.
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    Ok(())
}

#[test]
fn preparation_blocks_both_wheels_until_stable_neutral_ready() -> anyhow::Result<()> {
    let mut writes = Vec::new();
    let mut log = Vec::new();
    {
        let mut s = Session::new(
            control(Side::Both),
            |r| {
                writes.push(r.to_vec());
                Ok(r.len())
            },
            &mut log,
        )?;
        s.prepare_manual()?;
        s.initialize_rings()?;
        // Given qualifying motion on both sides before readiness.
        for index in 0..=40 {
            let at = u64::from(index) * 2000;
            s.handle(
                Event::Sample(input(
                    index,
                    [1000 + index * 2, 2000 - index * 2],
                    (if index == 0 { 2048 } else { 0 }, 0x30),
                )),
                at,
            )?;
            if index > 0 {
                s.handle(Event::OutputTick, at)?;
            }
        }
        // When CENTER is held and both wheels subsequently qualify fresh motion.
        for index in 41..=541 {
            let at = u64::from(index) * 2000;
            s.handle(Event::Sample(input(index, [1080, 1920], (2048, 0x30))), at)?;
            s.handle(Event::OutputTick, at)?;
        }
        for index in 542..=553 {
            let at = u64::from(index) * 2000;
            s.handle(
                Event::Sample(input(
                    index,
                    [1080 + (index - 541) * 2, 1920 - (index - 541) * 2],
                    (0, 0x30),
                )),
                at,
            )?;
            s.handle(Event::OutputTick, at)?;
        }
        s.handle(Event::Stop, 1_108_000)?;
    }
    // Then actual force is zero before READY and real on both wheels afterwards.
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
        writes
            .iter()
            .any(|r| r.len() == 11 && r[4..6] != [0, 0] && r[9..11] != [0, 0])
    );
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    Ok(())
}
