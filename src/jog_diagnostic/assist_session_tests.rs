use super::assist_tests::{control, input};
use super::motor::neutral_report;
use super::session::{Event, Session};
use anyhow::Context as _;
use std::cell::Cell;

pub(super) fn prime<F: FnMut(&[u8]) -> anyhow::Result<usize>>(
    session: &mut Session<'_, F>,
) -> anyhow::Result<()> {
    session.handle(Event::Sample(input(0, 1000, true)), 0)?;
    for index in 1..=12 {
        let at = u64::from(index) * 2000;
        session.handle(Event::Sample(input(index, 1000 + index * 2, true)), at)?;
        session.handle(Event::OutputTick, at)?;
    }
    Ok(())
}

#[test]
fn touch_release_writes_immediate_neutral_and_deduplicates_only_samples() -> anyhow::Result<()> {
    let mut log = Vec::new();
    let mut writes = Vec::new();
    let calls = Cell::new(0);
    {
        let mut session = Session::new(
            control(),
            |r| {
                calls.set(calls.get() + 1);
                writes.push(r.to_vec());
                Ok(r.len())
            },
            &mut log,
        )?;
        prime(&mut session)?;
        let before = calls.get();
        // Given real assist bytes, when touch releases before an output slot.
        session.handle(Event::Sample(input(13, 1026, false)), 26000)?;
        assert_eq!(calls.get(), before + 1);
        session.handle(Event::Sample(input(14, 1026, false)), 28000)?;
        assert_eq!(calls.get(), before + 1);
        session.handle(Event::OutputTick, 28000)?;
        assert_eq!(calls.get(), before + 2);
        session.handle(Event::Stop, 29000)?;
        // Then immediate, periodic and unconditional final zeros are all written.
        assert_eq!(calls.get(), before + 3);
    }
    assert!(
        writes
            .iter()
            .any(|r| r[2..4] == [0x20, 1] && r[4..6] != [0, 0])
    );
    assert!(writes.iter().all(|r| r[9..11] == [0, 0]));
    assert!(writes.iter().rev().take(3).all(|r| r == &neutral_report()));
    Ok(())
}

#[test]
fn assist_stop_watchdog_and_reader_fault_always_attempt_final_neutral() -> anyhow::Result<()> {
    for (event, at) in [
        (Event::Stop, 26000),
        (Event::Deadline, 26000),
        (Event::InputTimeout, 26000),
        (Event::ReaderError(anyhow::anyhow!("lost input")), 26000),
        (
            Event::Sample(super::decode::tests::frame(26000, 14, [1026, 0])),
            26000,
        ),
        (Event::OutputTick, 80000),
    ] {
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
            prime(&mut session)?;
            // Given actual force, when a stop or failure terminates the session.
            assert!(!matches!(session.handle(event, at), Ok(true)));
        }
        // Then the final hardware write is neutral, not merely a logged proposal.
        assert!(writes.iter().any(|r| r[4..6] != [0, 0]));
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}

#[test]
fn assist_transport_failure_is_propagated_and_neutral_is_attempted() -> anyhow::Result<()> {
    for short in [false, true] {
        let mut log = Vec::new();
        let mut writes = Vec::new();
        {
            let mut session = Session::new(
                control(),
                |r| {
                    writes.push(r.to_vec());
                    if r[4..6] == [0, 0] {
                        Ok(r.len())
                    } else if short {
                        Ok(5)
                    } else {
                        anyhow::bail!("assist transport failed")
                    }
                },
                &mut log,
            )?;
            // Given a failing real-force write, when assist first engages.
            let error = prime(&mut session).err().context("force must fail")?;
            assert!(error.to_string().contains(if short {
                "short motor write"
            } else {
                "assist transport failed"
            }));
        }
        // Then the error is not hidden by deduplication and cleanup writes zero.
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}
