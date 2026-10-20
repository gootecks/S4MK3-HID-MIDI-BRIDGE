use super::capture::RawFrame;
use super::decode::tests::{frame, with_fader};
use super::motor::{Decision, neutral_report};
use super::*;

pub(super) fn control() -> ControlArgs {
    ControlArgs {
        mode: Mode::FaderAssist,
        side: Side::Left,
        gain: 1,
        max_force: 0,
        assist_force: Some(2000),
    }
}

pub(super) fn input(index: u32, count: u32, held: bool) -> RawFrame {
    let mut result = with_fader(
        frame(u64::from(index) * 2000, index + 1, [count, 0]),
        if index == 0 { 2048 } else { 0 },
    );
    result.buttons[17] = if held { 0x10 } else { 0 };
    result
}

pub(super) fn step(
    decoder: &mut Decoder,
    controller: &mut Controller,
    frame: &RawFrame,
) -> anyhow::Result<Decision> {
    controller.decide(&decoder.decode(frame)?)
}

pub(super) fn prime(decoder: &mut Decoder, controller: &mut Controller) -> anyhow::Result<()> {
    for index in 0..=40 {
        step(decoder, controller, &input(index, 1000 + index * 2, true))?;
        controller
            .output_at(u64::from(index) * 2000)
            .context("clock")?;
    }
    Ok(())
}

#[test]
fn assistance_follows_both_directions_and_respects_fractional_caps() -> anyhow::Result<()> {
    // Given each direction and the supported ceiling boundaries.
    for (forward, sign) in [(true, 1), (false, -1)] {
        for cap in [0, 1, 2000, 8000] {
            let mut settings = control();
            settings.assist_force = Some(cap);
            let mut controller = Controller::new(settings);
            let mut decoder = Decoder::default();
            let mut last = 0;
            // When genuine held movement continues over multiple encoder steps.
            for index in 0..=200 {
                let count = if forward {
                    1000 + index / 2 * 4
                } else {
                    1000 - index / 2 * 4
                };
                step(&mut decoder, &mut controller, &input(index, count, true))?;
                let result = controller
                    .output_at(u64::from(index) * 2000)
                    .context("clock")?;
                assert!(result.torque[0].unsigned_abs() <= u32::from(cap));
                assert_eq!(result.torque[1], 0);
                last = result.torque[0];
            }
            // Then it assists, rather than brakes, and reaches even a one-unit cap.
            assert_eq!(last, sign * i32::from(cap));
        }
    }
    Ok(())
}

#[test]
fn assistance_cannot_arm_until_center_is_observed() -> anyhow::Result<()> {
    let mut controller = Controller::new(control());
    let mut decoder = Decoder::default();
    // Given touched movement starting in the assistance half without a center visit.
    for index in 0..=20 {
        let result = step(
            &mut decoder,
            &mut controller,
            &with_fader(input(index, 1000 + index * 2, true), 0),
        )?;
        assert!(!result.fader.context("fader")?.armed);
        assert_eq!(
            controller
                .output_at(u64::from(index) * 2000)
                .context("clock")?
                .torque,
            [0, 0]
        );
    }
    // When center is observed and then fresh movement qualifies.
    step(
        &mut decoder,
        &mut controller,
        &with_fader(input(21, 1040, true), 2048),
    )?;
    for index in 22..=30 {
        step(
            &mut decoder,
            &mut controller,
            &input(index, 1040 + (index - 21) * 2, true),
        )?;
        controller
            .output_at(u64::from(index) * 2000)
            .context("clock")?;
    }
    // Then assistance can activate, but never before center.
    assert!(controller.output_at(60000).context("armed")?.torque[0] > 0);
    Ok(())
}

#[test]
fn release_clears_force_and_stationary_retouch_cannot_revive_it() -> anyhow::Result<()> {
    let mut controller = Controller::new(control());
    let mut decoder = Decoder::default();
    prime(&mut decoder, &mut controller)?;
    assert!(controller.output_at(80000).context("active")?.torque[0] > 0);
    // Given real force, when touch is released even on a moving sample.
    assert_eq!(
        step(&mut decoder, &mut controller, &input(41, 1082, false))?.report,
        neutral_report()
    );
    // Then stationary retouch has no retained direction, magnitude or candidate counts.
    step(&mut decoder, &mut controller, &input(42, 1082, true))?;
    assert_eq!(
        controller.output_at(84000).context("retouch")?.torque,
        [0, 0]
    );
    step(&mut decoder, &mut controller, &input(43, 1084, true))?;
    assert_eq!(
        controller
            .output_at(86000)
            .context("fresh qualification")?
            .torque,
        [0, 0]
    );
    for index in 44..=47 {
        step(
            &mut decoder,
            &mut controller,
            &input(index, 1084 + (index - 43) * 2, true),
        )?;
        controller
            .output_at(u64::from(index) * 2000)
            .context("clock")?;
    }
    assert!(
        controller
            .output_at(94000)
            .context("fresh assistance")?
            .torque[0]
            > 0
    );
    Ok(())
}

#[test]
fn untouched_movement_and_stationary_touch_do_not_prime_assistance() -> anyhow::Result<()> {
    let mut controller = Controller::new(control());
    let mut decoder = Decoder::default();
    // Given sufficient movement to qualify, but without touch.
    for index in 0..=40 {
        step(
            &mut decoder,
            &mut controller,
            &input(index, 1000 + index * 2, false),
        )?;
        assert_eq!(
            controller
                .output_at(u64::from(index) * 2000)
                .context("clock")?
                .torque,
            [0, 0]
        );
    }
    // When touch arrives stationary, then old movement cannot revive force.
    step(&mut decoder, &mut controller, &input(41, 1080, true))?;
    assert_eq!(
        controller.output_at(82000).context("stationary")?.torque,
        [0, 0]
    );
    step(&mut decoder, &mut controller, &input(42, 1082, true))?;
    assert_eq!(
        controller.output_at(84000).context("unqualified")?.torque,
        [0, 0]
    );
    Ok(())
}

#[test]
fn held_rest_and_duplicates_cut_force_without_filter_tail() -> anyhow::Result<()> {
    let mut controller = Controller::new(control());
    let mut decoder = Decoder::default();
    prime(&mut decoder, &mut controller)?;
    // Given real force, when rest and duplicated snapshots contain no new motion.
    step(&mut decoder, &mut controller, &input(41, 1080, true))?;
    let mut duplicate = input(42, 1080, true);
    duplicate.jog[8..12].copy_from_slice(&42_u32.to_le_bytes());
    duplicate.jog[36..40].copy_from_slice(&42_u32.to_le_bytes());
    step(&mut decoder, &mut controller, &duplicate)?;
    // Then zero occurs at the independent 8ms cutoff, not a smoothing decay.
    assert!(controller.output_at(87999).context("before cutoff")?.torque[0] > 0);
    assert_eq!(
        controller.output_at(88000).context("cutoff")?.report,
        neutral_report()
    );
    step(&mut decoder, &mut controller, &input(45, 1082, true))?;
    assert_eq!(
        controller.output_at(90000).context("restart")?.torque,
        [0, 0]
    );
    Ok(())
}

#[test]
fn center_upper_half_and_reversal_zero_on_observation() -> anyhow::Result<()> {
    for (raw, count) in [(2048, 1082), (4095, 1082), (0, 1078)] {
        let mut controller = Controller::new(control());
        let mut decoder = Decoder::default();
        prime(&mut decoder, &mut controller)?;
        // Given actual force, when a zero gate is observed before the next tick.
        let decision = step(
            &mut decoder,
            &mut controller,
            &with_fader(input(41, count, true), raw),
        )?;
        // Then magnitude filtering cannot postpone neutralization.
        assert_eq!(decision.report, neutral_report());
        assert_eq!(
            controller.output_at(82000).context("immediate")?.report,
            neutral_report()
        );
    }
    Ok(())
}

#[test]
fn selected_assist_decrease_clamps_and_raising_alone_cannot_amplify_target() -> anyhow::Result<()> {
    let mut controller = Controller::new(control());
    let mut decoder = Decoder::default();
    prime(&mut decoder, &mut controller)?;
    // Given a stored magnitude, when the fader moves nearer center on real motion.
    step(
        &mut decoder,
        &mut controller,
        &with_fader(input(41, 1082, true), 1900),
    )?;
    let low = controller.output_at(82000).context("limited")?;
    let cap = low.fader.context("fader")?.selected_force;
    assert!(low.torque[0].unsigned_abs() <= u32::from(cap));
    // Then selecting the larger ceiling without motion cannot exceed the movement cap.
    step(&mut decoder, &mut controller, &input(42, 1082, true))?;
    assert!(
        controller.output_at(84000).context("raised")?.torque[0].unsigned_abs() <= u32::from(cap)
    );
    Ok(())
}
