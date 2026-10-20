use super::Mode;
use super::decode::tests::{frame, with_fader};
use super::motor::neutral_report;
use super::session::{Event, Session};
use super::smooth_tests::control;
use anyhow::Context as _;
use std::cell::Cell;

pub(super) fn prime<F: FnMut(&[u8]) -> anyhow::Result<usize>>(
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

fn rows(log: &[u8]) -> anyhow::Result<Vec<serde_json::Value>> {
    std::str::from_utf8(log)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .map_err(Into::into)
}

#[test]
fn only_e_skips_neutral_sample_duplicates_but_keeps_clock_and_cleanup() -> anyhow::Result<()> {
    // Given unchanged D as a negative control for duplicate zero writes.
    for mode in [Mode::FaderDamped, Mode::FaderSmooth] {
        let mut settings = control();
        settings.mode = mode;
        let mut log = Vec::new();
        let mut writes = Vec::new();
        {
            let mut session = Session::new(
                settings,
                |r| {
                    writes.push(r.to_vec());
                    Ok(r.len())
                },
                &mut log,
            )?;
            // When quiet fresh reports arrive between output slots.
            for index in 0..=10 {
                let at = u64::from(index) * 2000;
                session.handle(
                    Event::Sample(with_fader(frame(at, index + 1, [1000, 0]), 2048)),
                    at,
                )?;
                if index > 0 {
                    session.handle(Event::OutputTick, at)?;
                }
            }
            session.handle(Event::Stop, 22000)?;
        }
        let motor: Vec<_> = rows(&log)?
            .into_iter()
            .filter(|r| r["kind"] == "motor")
            .collect();
        // Then periodic and final reports remain, and only E removes sample writes.
        assert_eq!(
            motor.iter().filter(|r| r["reason"] == "motion").count(),
            if matches!(mode, Mode::FaderSmooth) {
                0
            } else {
                11
            }
        );
        assert_eq!(motor.iter().filter(|r| r["reason"] == "tick").count(), 10);
        assert_eq!(
            writes.len(),
            if matches!(mode, Mode::FaderSmooth) {
                12
            } else {
                23
            }
        );
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}

#[test]
fn center_and_reversal_neutral_write_precedes_next_slot_without_duplicates() -> anyhow::Result<()> {
    for (raw, count) in [(2048, 1024), (4095, 1022)] {
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
            // Given actual nonzero smoothed motor bytes before the transition.
            prime(&mut session)?;
            let before = calls.get();
            // When center or an opposite count arrives before the next clock slot.
            session.handle(
                Event::Sample(with_fader(frame(26000, 14, [count, 0]), raw)),
                26000,
            )?;
            assert_eq!(calls.get(), before + 1);
            session.handle(Event::OutputTick, 27000)?;
            assert_eq!(calls.get(), before + 2);
            session.handle(
                Event::Sample(with_fader(frame(28000, 15, [count, 0]), raw)),
                28000,
            )?;
            assert_eq!(calls.get(), before + 2);
            session.handle(Event::Stop, 29000)?;
            // Then immediate zero and periodic/final zeros all remain observable.
            assert_eq!(calls.get(), before + 3);
        }
        assert!(writes.iter().any(|r| r[4..6] != [0, 0]));
        assert!(writes.iter().rev().take(3).all(|r| r == &neutral_report()));
    }
    Ok(())
}

#[test]
fn cleanup_neutral_is_never_deduplicated_for_stop_fault_or_watchdog() -> anyhow::Result<()> {
    for (event, at) in [
        (Event::Stop, 26000),
        (Event::Deadline, 26000),
        (Event::InputTimeout, 26000),
        (Event::ReaderError(anyhow::anyhow!("lost reader")), 26000),
        (Event::Sample(frame(26000, 14, [1026, 0])), 26000),
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
            // Given active output, when any termination path runs.
            prime(&mut session)?;
            assert!(!matches!(session.handle(event, at), Ok(true)));
        }
        // Then a real final-neutral write follows the actual force.
        assert!(writes.iter().any(|r| r[4..6] != [0, 0]));
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
        assert_eq!(
            rows(&log)?
                .iter()
                .filter(|r| r["reason"] == "final_neutral")
                .count(),
            1
        );
    }
    Ok(())
}

#[test]
fn failed_or_short_force_write_is_not_hidden_by_the_zero_cache() -> anyhow::Result<()> {
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
                        anyhow::bail!("smooth transport failed")
                    }
                },
                &mut log,
            )?;
            // Given a failure only on real force, when the session emits force.
            let error = prime(&mut session).err().context("nonzero must fail")?;
            assert!(error.to_string().contains(if short {
                "short motor write"
            } else {
                "smooth transport failed"
            }));
        }
        // Then actual I/O failure propagates and neutral is still attempted.
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}
