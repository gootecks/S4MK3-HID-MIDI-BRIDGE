use super::decode::tests::{frame, with_fader};
use super::motor::{FaderZone, neutral_report};
use super::session::{Event, Session};
use super::*;

fn control() -> ControlArgs {
    ControlArgs {
        mode: Mode::FaderTension,
        side: Side::Left,
        gain: 1,
        max_force: 60000,
        assist_force: Some(8000),
    }
}

#[test]
fn center_arms_both_zones_and_release_removes_assistance() -> anyhow::Result<()> {
    // Given off-center startup, both directions, and touch/release changes.
    let mut decoder = Decoder::default();
    let mut controller = Controller::new(control());
    let mut torques = Vec::new();
    for (index, (raw, count, touched)) in [
        (4095, 1000, true),
        (0, 1010, true),
        (2016, 1020, true),
        (0, 1030, true),
        (0, 1040, false),
        (0, 1050, true),
        (0, 1040, true),
        (2080, 1030, true),
        (2081, 1040, true),
        (4095, 1050, false),
        (4095, 1040, false),
        (4095, 1040, false),
        (4095, 1040, false),
    ]
    .into_iter()
    .enumerate()
    {
        let index = u32::try_from(index)?;
        let mut input = with_fader(
            frame(u64::from(index) * 2000, index.min(11) + 1, [count, 2000]),
            raw,
        );
        if !touched {
            input.buttons[17] = 0;
        }
        // When the actual decoder/controller processes the complete sequence.
        let decision = controller.decide(&decoder.decode(&input)?)?;
        torques.push(decision.torque);
        if index == 9 {
            assert_eq!(decision.report, [49, 1, 224, 254, 96, 234, 1, 32, 1, 0, 0]);
            assert_eq!(
                decision.fader.context("selection")?.zone,
                Some(FaderZone::Brake)
            );
        }
    }
    // Then center is zero, assistance follows only held motion, and braking opposes.
    assert_eq!(
        torques,
        vec![
            [0, 0],
            [0, 0],
            [0, 0],
            [8000, 0],
            [0, 0],
            [8000, 0],
            [-8000, 0],
            [0, 0],
            [-29, 0],
            [-60000, 0],
            [60000, 0],
            [0, 0],
            [0, 0],
        ]
    );
    Ok(())
}

#[test]
fn neutral_edges_and_disabled_assistance_preserve_zero() -> anyhow::Result<()> {
    // Given a center-armed controller with assistance deliberately disabled.
    let mut settings = control();
    settings.assist_force = Some(0);
    let mut controller = Controller::new(settings);
    let mut decoder = Decoder::default();
    decoder.decode(&with_fader(frame(0, 1, [0, 0]), 2048))?;
    controller.decide(&decoder.decode(&with_fader(frame(2000, 2, [10, 0]), 2048))?)?;
    for (index, raw) in [0, 2015, 2016, 2080].into_iter().enumerate() {
        let index = u32::try_from(index)?;
        // When each lower/neutral boundary is visited while moving.
        let decision = controller.decide(&decoder.decode(&with_fader(
            frame(u64::from(index + 2) * 2000, index + 3, [20 + index * 10, 0]),
            raw,
        ))?)?;
        // Then disabling assistance never changes the zero-command invariant.
        assert_eq!(decision.report, neutral_report());
    }
    Ok(())
}

#[test]
fn active_tension_faults_and_terminal_events_attempt_neutral() -> anyhow::Result<()> {
    // Given genuinely encoded full-force braking before each fault.
    for event in [
        Event::Stop,
        Event::Deadline,
        Event::InputTimeout,
        Event::ReaderError(anyhow::anyhow!("input lost")),
        Event::Sample(frame(4000, 3, [30, 0])),
        Event::Sample(with_fader(frame(4000, 3, [30, 0]), 4096)),
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
            session.handle(Event::Sample(with_fader(frame(0, 1, [0, 0]), 2048)), 0)?;
            session.handle(
                Event::Sample(with_fader(frame(2000, 2, [20, 0]), 4095)),
                2000,
            )?;
            // When the real live session receives terminal or invalid input.
            assert!(!matches!(session.handle(event, 4000), Ok(true)));
        }
        // Then actual nonzero bytes preceded actual final-neutral bytes.
        assert!(
            writes
                .iter()
                .any(|r| r == &[49, 1, 224, 254, 96, 234, 1, 32, 1, 0, 0])
        );
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}

#[test]
fn failed_or_short_tension_write_preserves_error_and_neutral_attempt() -> anyhow::Result<()> {
    // Given a transport failing only on a new-mode nonzero motor report.
    for short in [false, true] {
        let mut writes = Vec::new();
        let mut log = Vec::new();
        {
            let mut session = Session::new(
                control(),
                |report| {
                    writes.push(report.to_vec());
                    if report[4] != 0 || report[5] != 0 {
                        if short {
                            Ok(5)
                        } else {
                            anyhow::bail!("tension transport failed")
                        }
                    } else {
                        Ok(report.len())
                    }
                },
                &mut log,
            )?;
            session.handle(Event::Sample(with_fader(frame(0, 1, [0, 0]), 2048)), 0)?;
            // When full-force output goes through the actual session writer.
            let error = session
                .handle(
                    Event::Sample(with_fader(frame(2000, 2, [20, 0]), 4095)),
                    2000,
                )
                .err()
                .context("write must fail")?;
            // Then the failure propagates and cleanup is attempted.
            assert!(error.to_string().contains(if short {
                "short motor write"
            } else {
                "tension transport failed"
            }));
        }
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}
