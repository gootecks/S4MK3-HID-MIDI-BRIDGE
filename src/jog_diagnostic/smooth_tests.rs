use super::decode::tests::{frame, with_fader};
use super::motor::neutral_report;
use super::*;

pub(super) fn control() -> ControlArgs {
    ControlArgs {
        mode: Mode::FaderSmooth,
        side: Side::Left,
        gain: 1,
        max_force: 30000,
        assist_force: Some(0),
    }
}

pub(super) fn observe(
    decoder: &mut Decoder,
    controller: &mut Controller,
    index: u32,
    count: u32,
    raw: u16,
) -> anyhow::Result<()> {
    controller.decide(&decoder.decode(&with_fader(
        frame(u64::from(index) * 2000, index + 1, [count, 0]),
        raw,
    ))?)?;
    Ok(())
}

#[test]
fn encoder_step_gaps_do_not_create_the_d_decay_ripple() -> anyhow::Result<()> {
    // Given steady motion quantized to four counts every four milliseconds.
    for (mode, expected_ripple) in [(Mode::FaderDamped, 7500), (Mode::FaderSmooth, 0)] {
        let mut settings = control();
        settings.mode = mode;
        let mut controller = Controller::new(settings);
        let mut decoder = Decoder::default();
        let mut settled = Vec::new();
        // When real reports and output clock slots alternate movement and rest.
        for index in 0..=200 {
            observe(
                &mut decoder,
                &mut controller,
                index,
                1000 + index / 2 * 4,
                if index == 0 { 2048 } else { 4095 },
            )?;
            let force = controller
                .output_at(u64::from(index) * 2000)
                .context("clock")?
                .torque[0];
            if index >= 180 {
                settled.push(force.unsigned_abs());
            }
        }
        // Then D remains the negative control while E settles without per-step fade.
        let min = settled.iter().min().context("minimum")?;
        let max = settled.iter().max().context("maximum")?;
        assert_eq!(max - min, expected_ripple);
        assert_eq!(*max, 30000);
    }
    Ok(())
}

#[test]
fn fractional_smoothing_preserves_small_caps_and_bounds_every_output() -> anyhow::Result<()> {
    // Given the supported ceiling boundaries, including a one-unit ceiling.
    for cap in [1, 30000, 60000] {
        let mut settings = control();
        settings.max_force = cap;
        let mut controller = Controller::new(settings);
        let mut decoder = Decoder::default();
        let mut last = 0;
        // When sustained motion is supplied long enough for the filter to settle.
        for index in 0..=200 {
            observe(
                &mut decoder,
                &mut controller,
                index,
                1000 + index * 2,
                if index == 0 { 2048 } else { 4095 },
            )?;
            let decision = controller
                .output_at(u64::from(index) * 2000)
                .context("clock")?;
            last = decision.torque[0].unsigned_abs();
            assert!(last <= u32::from(cap));
            assert_eq!(decision.torque[1], 0);
        }
        // Then a small cap is not silently rounded to a permanent no-op.
        assert_eq!(last, u32::from(cap));
    }
    Ok(())
}

#[test]
fn bounded_rest_and_duplicate_input_do_not_extend_smoothed_force() -> anyhow::Result<()> {
    let mut controller = Controller::new(control());
    let mut decoder = Decoder::default();
    // Given qualified, nonzero smoothed output.
    for index in 0..=50 {
        observe(
            &mut decoder,
            &mut controller,
            index,
            1000 + index * 2,
            if index == 0 { 2048 } else { 4095 },
        )?;
        controller
            .output_at(u64::from(index) * 2000)
            .context("clock")?;
    }
    // When a duplicate and then a rest sample arrive without actual movement.
    controller.decide(&decoder.decode(&with_fader(frame(102_000, 51, [1100, 0]), 4095))?)?;
    observe(&mut decoder, &mut controller, 52, 1100, 4095)?;
    assert!(
        controller
            .output_at(107_999)
            .context("before cutoff")?
            .torque[0]
            < 0
    );
    // Then the independent eight-millisecond cutoff bypasses the smoothing tail.
    assert_eq!(
        controller.output_at(108_000).context("cutoff")?.report,
        neutral_report()
    );
    observe(&mut decoder, &mut controller, 55, 1102, 4095)?;
    assert_eq!(
        controller
            .output_at(110_000)
            .context("requalification")?
            .torque,
        [0, 0]
    );
    Ok(())
}

#[test]
fn center_lower_half_and_reversal_bypass_magnitude_filtering() -> anyhow::Result<()> {
    for (raw, count) in [(2048, 1082), (0, 1082), (4095, 1078)] {
        let mut controller = Controller::new(control());
        let mut decoder = Decoder::default();
        // Given genuinely nonzero output before each safety transition.
        for index in 0..=40 {
            observe(
                &mut decoder,
                &mut controller,
                index,
                1000 + index * 2,
                if index == 0 { 2048 } else { 4095 },
            )?;
            controller
                .output_at(u64::from(index) * 2000)
                .context("clock")?;
        }
        assert!(controller.output_at(80000).context("active")?.torque[0] < 0);
        // When the actual decoder observes center, lower half, or reversed motion.
        let result =
            controller.decide(&decoder.decode(&with_fader(frame(82000, 42, [count, 0]), raw))?)?;
        // Then the observation itself is zero, not a filtered decline.
        assert_eq!(result.report, neutral_report());
        assert_eq!(
            controller.output_at(82000).context("immediate")?.report,
            neutral_report()
        );
    }
    Ok(())
}

#[test]
fn fader_ceiling_decrease_clamps_without_waiting_for_the_filter() -> anyhow::Result<()> {
    let mut controller = Controller::new(control());
    let mut decoder = Decoder::default();
    // Given a high stored magnitude.
    for index in 0..=40 {
        observe(
            &mut decoder,
            &mut controller,
            index,
            1000 + index * 2,
            if index == 0 { 2048 } else { 4095 },
        )?;
        controller
            .output_at(u64::from(index) * 2000)
            .context("clock")?;
    }
    // When a much smaller cap is selected on a fresh moving sample.
    observe(&mut decoder, &mut controller, 41, 1082, 2300)?;
    let limited = controller.output_at(82000).context("limited")?;
    let cap = limited.fader.context("selection")?.selected_force;
    assert!(limited.torque[0].unsigned_abs() <= u32::from(cap));
    // Then raising the fader alone cannot exceed the last movement's ceiling.
    observe(&mut decoder, &mut controller, 42, 1082, 4095)?;
    assert!(
        controller.output_at(84000).context("raised")?.torque[0].unsigned_abs() <= u32::from(cap)
    );
    Ok(())
}
