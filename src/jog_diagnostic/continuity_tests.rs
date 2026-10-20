use super::decode::tests::{frame, with_fader};
use super::motor::neutral_report;
use super::session::{Event, Session};
use super::*;

fn control() -> ControlArgs {
    ControlArgs {
        mode: Mode::FaderContinuity,
        side: Side::Left,
        gain: 1,
        max_force: 60000,
        assist_force: Some(8000),
    }
}

#[test]
fn a_failed_or_short_tick_write_preserves_error_and_neutralizes() -> anyhow::Result<()> {
    // Given a transport that accepts samples but fails on actual paced force.
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
                            return Ok(5);
                        }
                        anyhow::bail!("paced write failed");
                    }
                    Ok(report.len())
                },
                &mut log,
            )?;
            session.handle(Event::Sample(with_fader(frame(0, 1, [0, 0]), 2048)), 0)?;
            session.handle(
                Event::Sample(with_fader(frame(2000, 2, [10, 0]), 4095)),
                2000,
            )?;
            // When the first nonzero tick reaches that real session writer.
            let error = session
                .handle(Event::OutputTick, 2000)
                .err()
                .context("must fail")?;
            assert!(error.to_string().contains(if short {
                "short motor write"
            } else {
                "paced write failed"
            }));
        }
        // Then the failure survives and the last attempted report is neutral.
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}

#[test]
fn stale_tick_terminates_even_while_the_input_thread_has_not_completed() -> anyhow::Result<()> {
    // Given a prior accepted full-force tick, with no new input.
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
            Event::Sample(with_fader(frame(2000, 2, [10, 0]), 4095)),
            2000,
        )?;
        session.handle(Event::OutputTick, 2000)?;
        // When a tick is received after the independent input freshness bound.
        let error = session
            .handle(Event::OutputTick, 52001)
            .err()
            .context("stale input")?;
        assert!(error.to_string().contains("no fresh input"));
    }
    // Then the actual final write is neutral without waiting for Reader completion.
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    Ok(())
}

#[test]
fn a_fader_increase_cannot_amplify_force_without_new_motion() -> anyhow::Result<()> {
    // Given movement at half braking force followed by a higher fader setting.
    let mut decoder = Decoder::default();
    let mut controller = Controller::new(control());
    controller.decide(&decoder.decode(&with_fader(frame(0, 1, [0, 0]), 2048))?)?;
    controller.decide(&decoder.decode(&with_fader(frame(2000, 2, [10, 0]), 3087))?)?;
    // When only the fader rises while the counter remains unchanged.
    let decision =
        controller.decide(&decoder.decode(&with_fader(frame(4000, 3, [10, 0]), 4095))?)?;
    // Then the old 29985-unit force decays, rather than the new 60000 cap.
    assert_eq!(decision.torque, [-22488, 0]);
    Ok(())
}

#[test]
fn rest_and_duplicate_do_not_refresh_the_bounded_force_tail() -> anyhow::Result<()> {
    // Given real center-armed motion followed by fresh rest and duplicate samples.
    let mut decoder = Decoder::default();
    let mut controller = Controller::new(control());
    let mut forces = Vec::new();
    for (time, timer, count, raw) in [
        (0, 1, 0, 2048),
        (2000, 2, 10, 4095),
        (4000, 3, 10, 4095),
        (6000, 3, 10, 4095),
        (10000, 4, 10, 4095),
    ] {
        // When both statuses pass through the real decoder and controller.
        forces.push(
            controller
                .decide(&decoder.decode(&with_fader(frame(time, timer, [count, 0]), raw))?)?
                .torque,
        );
    }
    // Then force fades from the last real movement, not from new samples.
    assert_eq!(
        forces,
        [[0, 0], [-60000, 0], [-45000, 0], [-30000, 0], [0, 0]]
    );
    assert_eq!(
        controller.output_at(20000).context("output")?.report,
        neutral_report()
    );
    Ok(())
}

#[test]
fn assistance_release_clears_history_and_retouch_alone_stays_zero() -> anyhow::Result<()> {
    // Given held assistance, release, retouch without motion, then fresh movement.
    let mut decoder = Decoder::default();
    let mut controller = Controller::new(control());
    let mut forces = Vec::new();
    for (time, count, raw, touched) in [
        (0, 0, 2048, true),
        (2000, 10, 0, true),
        (4000, 10, 0, false),
        (6000, 10, 0, true),
        (8000, 0, 0, true),
    ] {
        let mut input = with_fader(frame(time, u32::try_from(time + 1)?, [count, 0]), raw);
        if !touched {
            input.buttons[17] = 0;
        }
        // When the sequence is decoded with real touch evidence.
        forces.push(controller.decide(&decoder.decode(&input)?)?.torque);
    }
    // Then release and retouch are zero; only fresh reversed movement restores force.
    assert_eq!(forces, [[0, 0], [8000, 0], [0, 0], [0, 0], [-8000, 0]]);
    Ok(())
}

#[test]
fn zone_changes_and_center_clear_previous_direction() -> anyhow::Result<()> {
    // Given braking followed by assistance/center without further movement.
    let mut decoder = Decoder::default();
    let mut controller = Controller::new(control());
    let mut forces = Vec::new();
    for (time, count, raw) in [
        (0, 0, 2048),
        (1000, 10, 4095),
        (2000, 10, 0),
        (3000, 20, 0),
        (4000, 20, 2048),
        (5000, 20, 4095),
    ] {
        // When the fader changes zones.
        forces.push(
            controller
                .decide(&decoder.decode(&with_fader(
                    frame(time, u32::try_from(time + 1)?, [count, 0]),
                    raw,
                ))?)?
                .torque[0],
        );
    }
    // Then the prior zone never supplies direction to a newly selected one.
    assert_eq!(forces, [0, -60000, 0, 8000, 0, 0]);
    Ok(())
}

#[test]
fn session_ticks_emit_decayed_force_and_expire_without_new_samples() -> anyhow::Result<()> {
    // Given a paced session whose sample path must not write nonzero force.
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
            Event::Sample(with_fader(frame(2000, 2, [10, 0]), 4095)),
            2000,
        )?;
        // When the production tick path runs on exact synthetic times.
        for now in [2000, 4000, 6000, 8000, 10000] {
            session.handle(Event::OutputTick, now)?;
        }
        session.finish("test")?;
    }
    // Then only ticks apply force, with no extra nonzero sample write.
    let nonzero = writes
        .iter()
        .filter(|r| r[4] != 0 || r[5] != 0)
        .map(|r| u16::from_le_bytes([r[4], r[5]]))
        .collect::<Vec<_>>();
    assert_eq!(nonzero, [60000, 45000, 30000, 15000]);
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    Ok(())
}

#[test]
fn terminal_and_invalid_events_neutralize_after_a_real_paced_write() -> anyhow::Result<()> {
    // Given actual full-force paced output before each terminal/fault event.
    for event in [
        Event::Stop,
        Event::Deadline,
        Event::InputTimeout,
        Event::ReaderError(anyhow::anyhow!("reader lost")),
        Event::Sample(frame(4000, 3, [20, 0])),
        Event::Sample(with_fader(frame(4000, 3, [20, 0]), 4096)),
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
                Event::Sample(with_fader(frame(2000, 2, [10, 0]), 4095)),
                2000,
            )?;
            session.handle(Event::OutputTick, 2000)?;
            // When production handles the terminal/fault condition.
            assert!(!matches!(session.handle(event, 4000), Ok(true)));
        }
        // Then full-force bytes precede an actual final neutral write.
        assert!(writes.iter().any(|r| r[4..6] == [96, 234]));
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}
