use super::*;
use crate::jog_diagnostic::decode::tests::frame;
use crate::jog_diagnostic::{Mode, Side};
use anyhow::Context as _;

fn control() -> ControlArgs {
    ControlArgs {
        mode: Mode::Assist,
        side: Side::Left,
        gain: 1,
        max_force: 100,
    }
}

#[test]
fn stop_deadline_and_reader_failures_end_with_actual_neutral_bytes() -> anyhow::Result<()> {
    // Given a real session that has begun emitting nonzero assistance.
    for event in [
        Event::Stop,
        Event::Deadline,
        Event::InputTimeout,
        Event::ReaderError(anyhow::anyhow!("reader failed")),
    ] {
        let mut writes = Vec::new();
        let mut log = Vec::new();
        {
            let mut session = Session::new(
                control(),
                |report| {
                    writes.push(report.to_vec());
                    Ok(report.len())
                },
                &mut log,
            )?;
            session.handle(Event::Sample(frame(0, 1, [0, 0])), 0)?;
            session.handle(Event::Sample(frame(2000, 2, [200, 0])), 2000)?;
            // When a terminal event reaches the same state machine used live.
            let stopped = session.handle(event, 3000);
            // Then processing terminates (success or a propagated input failure).
            assert!(!matches!(stopped, Ok(true)));
        }
        assert_eq!(writes.first(), Some(&neutral_report().to_vec()));
        assert!(writes.iter().any(|report| report[4] == 100));
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}

#[test]
fn failed_or_short_nonzero_write_attempts_neutral_and_preserves_error() -> anyhow::Result<()> {
    // Given a transport which fails only on nonzero torque.
    for short in [false, true] {
        let mut writes = Vec::new();
        let mut log = Vec::new();
        {
            let mut session = Session::new(
                control(),
                |report| {
                    writes.push(report.to_vec());
                    if report[4] != 0 {
                        if short {
                            Ok(5)
                        } else {
                            bail!("injected write failure")
                        }
                    } else {
                        Ok(report.len())
                    }
                },
                &mut log,
            )?;
            session.handle(Event::Sample(frame(0, 1, [0, 0])), 0)?;
            // When the real writer sees a movement report.
            let error = session
                .handle(Event::Sample(frame(2000, 2, [200, 0])), 2000)
                .err()
                .context("transport fault must fail")?;
            // Then the original error survives, and neutral is the final write.
            assert!(error.to_string().contains(if short {
                "short motor write"
            } else {
                "injected write failure"
            }));
        }
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}

#[test]
fn stale_queue_and_malformed_input_neutralize_before_returning() -> anyhow::Result<()> {
    // Given an armed session and a stale or malformed raw frame.
    for (mut next, now) in [
        (frame(2000, 2, [200, 0]), 52_001),
        (frame(2000, 2, [200, 0]), 2000),
    ] {
        if now == 2000 {
            next.jog.truncate(4);
        }
        let mut writes = Vec::new();
        let mut log = Vec::new();
        {
            let mut session = Session::new(
                control(),
                |report| {
                    writes.push(report.to_vec());
                    Ok(report.len())
                },
                &mut log,
            )?;
            session.handle(Event::Sample(frame(0, 1, [0, 0])), 0)?;
            // When the unacceptable frame is processed, then it fails closed.
            assert!(session.handle(Event::Sample(next), now).is_err());
        }
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
        assert!(writes.iter().all(|report| report[4] == 0));
    }
    Ok(())
}

#[test]
fn original_motor_failure_survives_simultaneous_log_failure() -> anyhow::Result<()> {
    struct FailedLog;
    impl Write for FailedLog {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected log failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    // Given independent transport and logging failures at startup.
    let mut log = FailedLog;
    // When the actual session attempts its startup neutral.
    let error = Session::new(control(), |_| bail!("original motor fault"), &mut log)
        .err()
        .context("startup must fail")?;
    // Then the original motor failure remains diagnosable.
    assert!(format!("{error:#}").contains("original motor fault"));
    Ok(())
}
