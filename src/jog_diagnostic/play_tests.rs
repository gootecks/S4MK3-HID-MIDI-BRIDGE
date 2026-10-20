use super::assist_tests::{input, step};
use super::decode::tests::with_fader;
use super::motor::neutral_report;
use super::session::{Event, Session};
use super::*;
use std::cell::Cell;

fn control() -> ControlArgs {
    ControlArgs {
        mode: Mode::FaderPlay,
        side: Side::Left,
        gain: 1,
        max_force: 30000,
        assist_force: Some(2000),
    }
}

#[test]
fn fixed_free_play_halves_match_the_proven_one_sided_laws() -> anyhow::Result<()> {
    // Given the same raw gesture for combined and one-sided operation.
    for (raw, mode, sign) in [(0, Mode::FaderAssist, 1), (4095, Mode::FaderSmooth, -1)] {
        for forward in [true, false] {
            let mut reference = control();
            reference.mode = mode;
            reference.max_force = if raw == 0 { 0 } else { 30000 };
            reference.assist_force = Some(if raw == 0 { 2000 } else { 0 });
            let mut existing = Controller::new(reference);
            let mut combined = Controller::new(control());
            let mut a = Decoder::default();
            let mut b = Decoder::default();
            let mut last = 0;
            // When each real decoder/controller receives identical frames and ticks.
            for index in 0..=100 {
                let count = if forward {
                    1000 + index * 2
                } else {
                    1000 - index * 2
                };
                let frame = with_fader(
                    input(index, count, true),
                    if index == 0 { 2048 } else { raw },
                );
                step(&mut a, &mut existing, &frame)?;
                step(&mut b, &mut combined, &frame)?;
                let time = u64::from(index) * 2000;
                let expected = existing.output_at(time).context("existing")?;
                let actual = combined.output_at(time).context("combined")?;
                assert_eq!(actual.torque, expected.torque);
                assert_eq!(actual.report, expected.report);
                last = actual.torque[0];
            }
            // Then combining selection hasn't weakened or changed either law.
            assert_eq!(last.signum(), if forward { sign } else { -sign });
        }
    }
    Ok(())
}

#[test]
fn skipping_center_clears_both_force_histories_before_requalification() -> anyhow::Result<()> {
    for (from, to, sign) in [(0, 4095, -1), (4095, 0, 1)] {
        for moving in [false, true] {
            let mut controller = Controller::new(control());
            let mut decoder = Decoder::default();
            // Given nonzero output in the old zone.
            for index in 0..=40 {
                step(
                    &mut decoder,
                    &mut controller,
                    &with_fader(
                        input(index, 1000 + index * 2, true),
                        if index == 0 { 2048 } else { from },
                    ),
                )?;
                controller
                    .output_at(u64::from(index) * 2000)
                    .context("clock")?;
            }
            assert_ne!(
                controller.output_at(80000).context("active")?.torque,
                [0, 0]
            );
            // When the next report jumps directly to the opposite half.
            let count = if moving { 1082 } else { 1080 };
            let changed = step(
                &mut decoder,
                &mut controller,
                &with_fader(input(41, count, true), to),
            )?;
            // Then the transition itself and first tick are neutral.
            assert_eq!(changed.report, neutral_report());
            assert_eq!(
                controller.output_at(82000).context("transition")?.torque,
                [0, 0]
            );
            for index in 42..=45 {
                step(
                    &mut decoder,
                    &mut controller,
                    &with_fader(input(index, count + (index - 41) * 2, true), to),
                )?;
                controller
                    .output_at(u64::from(index) * 2000)
                    .context("clock")?;
            }
            assert_eq!(
                controller.output_at(90000).context("qualified")?.torque[0].signum(),
                sign
            );
        }
    }
    Ok(())
}

#[test]
fn release_disables_assistance_but_preserves_moving_braking() -> anyhow::Result<()> {
    for raw in [0, 4095] {
        let mut controller = Controller::new(control());
        let mut decoder = Decoder::default();
        for index in 0..=40 {
            step(
                &mut decoder,
                &mut controller,
                &with_fader(
                    input(index, 1000 + index * 2, true),
                    if index == 0 { 2048 } else { raw },
                ),
            )?;
            controller
                .output_at(u64::from(index) * 2000)
                .context("clock")?;
        }
        // Given active output, when touch releases during fresh positive motion.
        let result = step(
            &mut decoder,
            &mut controller,
            &with_fader(input(41, 1082, false), raw),
        )?;
        // Then only assistance is touch-gated; braking still opposes the movement.
        if raw == 0 {
            assert_eq!(result.torque, [0, 0]);
        } else {
            assert!(result.torque[0] < 0);
        }
        assert_eq!(result.torque[1], 0);
    }
    Ok(())
}

#[test]
fn combined_zone_switch_writes_neutral_before_next_output_slot() -> anyhow::Result<()> {
    for (from, to) in [(0, 4095), (4095, 0), (0, 2048), (4095, 2048)] {
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
            session.handle(Event::Sample(input(0, 1000, true)), 0)?;
            for index in 1..=12 {
                let at = u64::from(index) * 2000;
                session.handle(
                    Event::Sample(with_fader(input(index, 1000 + index * 2, true), from)),
                    at,
                )?;
                session.handle(Event::OutputTick, at)?;
            }
            let before = calls.get();
            // Given real force, when selection changes without waiting for a tick.
            session.handle(Event::Sample(with_fader(input(13, 1026, true), to)), 26000)?;
            // Then the immediate actual write is zero, and cleanup still writes.
            assert_eq!(calls.get(), before + 1);
            session.handle(Event::Stop, 27000)?;
        }
        assert!(writes.iter().any(|r| r[4..6] != [0, 0]));
        assert!(writes.iter().rev().take(2).all(|r| r == &neutral_report()));
    }
    Ok(())
}

#[test]
fn offline_engine_matches_validated_fader_play_for_raw_fixtures() -> anyhow::Result<()> {
    use s4mk3_hid_midi_bridge::tension::{Config, Engine};

    // Given a fixed saved setting equivalent to each full-scale fader half,
    // when both paths decode the same numbered snapshots and clock ticks,
    // then they must emit identical native reports in both directions.
    for (value, raw, sign) in [(-100, 0, 1), (100, 4095, -1)] {
        for forward in [true, false] {
            let config: Config = serde_json::from_str(&format!(r#"{{"tension":{value}}}"#))?;
            let mut engine = Engine::new(config);
            let mut reference = control();
            reference.assist_force = Some(7000);
            let mut controller = Controller::new(reference);
            let mut decoder = Decoder::default();
            let mut last = 0;
            for index in 0..=100 {
                let count = if forward {
                    1000 + index * 2
                } else {
                    1000 - index * 2
                };
                let mut frame = input(index, count, true);
                frame = with_fader(frame, if index == 0 { 2048 } else { raw });
                let motion = decoder.decode(&frame)?;
                controller.decide(&motion)?;
                engine.observe(&frame.buttons, &frame.jog, frame.host_us, frame.host_us)?;
                let expected = controller
                    .output_at(frame.host_us)
                    .context("diagnostic tick")?;
                let actual = engine.tick(frame.host_us)?;
                assert_eq!(actual, expected.report, "setting {value}, index {index}");
                last = expected.torque[0];
            }
            assert_eq!(last.signum(), if forward { sign } else { -sign });
        }
    }
    Ok(())
}
