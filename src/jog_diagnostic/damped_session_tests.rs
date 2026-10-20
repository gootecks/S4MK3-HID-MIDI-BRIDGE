use super::damped_tests::control;
use super::decode::tests::{frame, with_fader};
use super::motor::neutral_report;
use super::session::{Event, Session};
use anyhow::Context as _;

fn prime<F: FnMut(&[u8]) -> anyhow::Result<usize>>(
    session: &mut Session<'_, F>,
) -> anyhow::Result<()> {
    session.handle(Event::Sample(with_fader(frame(0, 1, [1000, 0]), 2048)), 0)?;
    for index in 1..=12 {
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
    Ok(())
}

#[test]
fn active_damping_neutralizes_on_stop_fault_timeout_and_invalid_input() -> anyhow::Result<()> {
    // Given real active damped reports before each terminal path.
    for event in [
        Event::Stop,
        Event::Deadline,
        Event::InputTimeout,
        Event::ReaderError(anyhow::anyhow!("lost reader")),
        Event::Sample(frame(26000, 14, [1026, 0])),
    ] {
        let mut log = Vec::new();
        let mut writes = Vec::new();
        {
            let mut session = Session::new(
                control(),
                |report| {
                    writes.push(report.to_vec());
                    Ok(report.len())
                },
                &mut log,
            )?;
            prime(&mut session)?;
            // When the terminal path runs, then the session stops.
            assert!(!matches!(session.handle(event, 26000), Ok(true)));
        }
        assert!(writes.iter().any(|r| r[4..6] != [0, 0]));
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}

#[test]
fn short_or_failed_damped_write_attempts_neutral_and_propagates_error() -> anyhow::Result<()> {
    for short in [false, true] {
        // Given a transport that fails only for actual nonzero damped bytes.
        let mut log = Vec::new();
        let mut writes = Vec::new();
        {
            let mut session = Session::new(
                control(),
                |report| {
                    writes.push(report.to_vec());
                    if report[4..6] == [0, 0] {
                        Ok(report.len())
                    } else if short {
                        Ok(5)
                    } else {
                        anyhow::bail!("failed damping write")
                    }
                },
                &mut log,
            )?;
            // When the writer receives qualified force, then error is propagated.
            let error = prime(&mut session).err().context("write must fail")?;
            assert!(error.to_string().contains(if short {
                "short motor write"
            } else {
                "failed damping write"
            }));
        }
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}

#[test]
fn manual_preparation_has_no_countdown_and_emits_real_force_only_after_ready() -> anyhow::Result<()>
{
    // Given manual preparation and active movement outside center for >30s.
    let mut log = Vec::new();
    let mut writes = Vec::new();
    {
        let mut session = Session::new(
            control(),
            |report| {
                writes.push(report.to_vec());
                Ok(report.len())
            },
            &mut log,
        )?;
        session.prepare_manual()?;
        for index in 0..=620 {
            let at = u64::from(index) * 50000;
            session.handle(
                Event::Sample(with_fader(frame(at, index + 1, [1000 + index, 0]), 4095)),
                at,
            )?;
            session.handle(Event::OutputTick, at)?;
        }
        assert_eq!(session.remaining_duration_us(31_000_000, 30), None);
        // When fresh center is held, qualification and movement can then proceed.
        for index in 1..=21 {
            let at = 31_000_000 + u64::from(index) * 50000;
            session.handle(
                Event::Sample(with_fader(frame(at, 621 + index, [1620, 0]), 2048)),
                at,
            )?;
        }
        for index in 1..=12 {
            let at = 32_050_000 + u64::from(index) * 2000;
            session.handle(
                Event::Sample(with_fader(
                    frame(at, 642 + index, [1620 + 2 * index, 0]),
                    4095,
                )),
                at,
            )?;
            session.handle(Event::OutputTick, at)?;
        }
        session.handle(Event::Stop, 32_076_000)?;
    }
    // Then waiting output stayed zero, READY was logged once, Stop cleaned up.
    let values = std::str::from_utf8(&log)?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    let ready = values
        .iter()
        .find(|v| v["kind"] == "ready")
        .context("READY")?;
    let ready_us = ready["host_us"].as_u64().context("ready time")?;
    assert!(ready_us > 31_000_000);
    assert!(values.iter().filter(|v| v["kind"] == "tick" && v["host_us"].as_u64().is_some_and(|at| at < ready_us)).all(|v| v["decision"]["torque"] == serde_json::json!([0, 0])));
    assert!(
        values
            .iter()
            .any(|v| v["kind"] == "preparation" && v["seconds"].is_null())
    );
    assert!(writes.iter().any(|r| r[4..6] != [0, 0]));
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    Ok(())
}
