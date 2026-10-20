use super::decode::tests::{frame, with_fader};
use super::motor::Decision;
use super::*;

pub(super) fn control() -> ControlArgs {
    ControlArgs {
        mode: Mode::FaderDamped,
        side: Side::Left,
        gain: 1,
        max_force: 30000,
        assist_force: Some(0),
    }
}

pub(super) fn sample(
    decoder: &mut Decoder,
    controller: &mut Controller,
    index: u32,
    count: u32,
    raw: u16,
) -> anyhow::Result<Decision> {
    controller.decide(&decoder.decode(&with_fader(
        frame(u64::from(index) * 2000, index + 1, [count, 1000]),
        raw,
    ))?)
}

#[test]
fn alternating_two_count_jitter_is_zero_instead_of_full_braking() -> anyhow::Result<()> {
    // Given the observed jitter and the unchanged A mode as negative control.
    for mode in [Mode::FaderDamped, Mode::FaderTension] {
        let mut args = control();
        args.mode = mode;
        let mut controller = Controller::new(args);
        let mut decoder = Decoder::default();
        sample(&mut decoder, &mut controller, 0, 1000, 2048)?;
        // When fresh real reports alternate tiny directions.
        for index in 1..=40 {
            let count = if index % 2 == 0 { 1000 } else { 1002 };
            let result = sample(&mut decoder, &mut controller, index, count, 4095)?;
            // Then only the new mode suppresses this full-force pulse pattern.
            assert_eq!(
                result.torque[0].unsigned_abs(),
                if matches!(mode, Mode::FaderDamped) {
                    0
                } else {
                    30000
                }
            );
            assert_eq!(result.torque[1], 0);
        }
    }
    Ok(())
}

#[test]
fn sustained_motion_qualifies_then_tapers_to_the_selected_cap() -> anyhow::Result<()> {
    // Given each supported boundary ceiling and fresh two-count motion.
    for cap in [1, 30000, 60000] {
        let mut args = control();
        args.max_force = cap;
        let mut controller = Controller::new(args);
        let mut decoder = Decoder::default();
        sample(&mut decoder, &mut controller, 0, 1000, 2048)?;
        let mut values = Vec::new();
        // When speed evidence accumulates across six milliseconds.
        for index in 1..=12 {
            values.push(
                sample(&mut decoder, &mut controller, index, 1000 + 2 * index, 4095)?.torque[0],
            );
        }
        // Then qualification is silent, low speed is scaled, and the cap holds.
        assert_eq!(&values[..3], &[0, 0, 0]);
        assert_eq!(values[3], -i32::from(cap / 4));
        assert_eq!(values[9], -i32::from(cap));
        assert!(
            values
                .iter()
                .all(|value| (-i32::from(cap)..=0).contains(value))
        );
    }
    Ok(())
}

#[test]
fn observed_reversal_neutralizes_before_qualifying_the_new_direction() -> anyhow::Result<()> {
    // Given active clockwise braking through the actual decoder/controller.
    let mut controller = Controller::new(control());
    let mut decoder = Decoder::default();
    sample(&mut decoder, &mut controller, 0, 1000, 2048)?;
    for index in 1..=12 {
        sample(&mut decoder, &mut controller, index, 1000 + index * 2, 4095)?;
    }
    // When the wheel reverses, its first three new reports cannot reverse force.
    let mut result = Vec::new();
    for index in 13..=24 {
        result.push(
            sample(
                &mut decoder,
                &mut controller,
                index,
                1024 - (index - 12) * 2,
                4095,
            )?
            .torque[0],
        );
    }
    // Then old-direction braking is immediately gone and new direction ramps.
    assert_eq!(&result[..3], &[0, 0, 0]);
    assert_eq!(result[3], 7500);
    assert_eq!(result[9], 30000);
    Ok(())
}

#[test]
fn rest_duplicates_and_fader_raise_cannot_refresh_motion_or_increase_its_force()
-> anyhow::Result<()> {
    // Given qualified low-fader braking.
    let mut controller = Controller::new(control());
    let mut decoder = Decoder::default();
    sample(&mut decoder, &mut controller, 0, 1000, 2048)?;
    for index in 1..=12 {
        sample(&mut decoder, &mut controller, index, 1000 + index * 2, 3000)?;
    }
    let before = controller.output_at(24000).context("output")?.torque[0].unsigned_abs();
    // When the fader rises with a rest report followed by a duplicate.
    sample(&mut decoder, &mut controller, 13, 1024, 4095)?;
    let mut duplicate = with_fader(frame(28000, 14, [1024, 1000]), 4095);
    duplicate.buttons[17] = 0;
    controller.decide(&decoder.decode(&duplicate)?)?;
    let tail = controller.output_at(28000).context("tail")?.torque[0].unsigned_abs();
    // Then it only decays, expires at 8ms and cannot revive from rest.
    assert!(tail > 0 && tail < before);
    assert_eq!(
        controller.output_at(32000).context("expired")?.torque,
        [0, 0]
    );
    assert_eq!(
        sample(&mut decoder, &mut controller, 17, 1026, 4095)?.torque,
        [0, 0]
    );
    Ok(())
}

#[test]
fn center_and_disabled_lower_half_clear_history_and_never_assist() -> anyhow::Result<()> {
    // Given each neutral/lower selection after genuine qualified braking.
    for raw in [0, 2015, 2016, 2048, 2080] {
        let mut controller = Controller::new(control());
        let mut decoder = Decoder::default();
        sample(&mut decoder, &mut controller, 0, 1000, 2048)?;
        for index in 1..=12 {
            sample(&mut decoder, &mut controller, index, 1000 + index * 2, 4095)?;
        }
        // When a lower/neutral sample arrives, then both paths clear immediately.
        assert_eq!(
            sample(&mut decoder, &mut controller, 13, 1026, raw)?.torque,
            [0, 0]
        );
        assert_eq!(
            controller.output_at(26000).context("cleared")?.torque,
            [0, 0]
        );
    }
    Ok(())
}
