use super::*;
use crate::jog_diagnostic::decode::tests::{frame, with_fader};
use crate::jog_diagnostic::{Side, decode::Decoder};

#[test]
fn modes_reverse_or_follow_real_motion_and_isolate_side() -> anyhow::Result<()> {
    // Given forward left and backward right movement from actual raw reports.
    let mut decoder = Decoder::default();
    decoder.decode(&frame(0, 1, [0, 1000]))?;
    let motion = decoder.decode(&frame(2000, 2, [200, 800]))?;
    for (mode, side, torque, report) in [
        (
            Mode::Brake,
            Side::Left,
            [-100, 0],
            [49, 1, 224, 254, 100, 0, 1, 32, 1, 0, 0],
        ),
        (
            Mode::Assist,
            Side::Right,
            [0, -100],
            [49, 1, 32, 1, 0, 0, 1, 224, 254, 100, 0],
        ),
        (Mode::Baseline, Side::Left, [0, 0], neutral_report()),
    ] {
        // When the selected diagnostic controller computes the report.
        let decision = Controller::new(ControlArgs {
            mode,
            side,
            gain: 2,
            max_force: 100,
            assist_force: None,
        })
        .decide(&motion)?;
        // Then direction, bounded magnitude, and the other wheel's neutral agree.
        assert_eq!(decision.torque, torque);
        assert_eq!(decision.report, report);
    }
    Ok(())
}

#[test]
fn assist_is_neutral_at_rest_and_on_first_or_duplicate_samples() -> anyhow::Result<()> {
    // Given assistance and an initially stationary wheel.
    let mut controller = Controller::new(ControlArgs {
        mode: Mode::Assist,
        side: Side::Left,
        gain: 1,
        max_force: 100,
        assist_force: None,
    });
    let mut decoder = Decoder::default();
    for input in [
        frame(0, 1, [0, 0]),
        frame(2000, 2, [0, 0]),
        frame(4000, 2, [0, 0]),
    ] {
        // When first/rest/duplicate input is processed.
        let decision = controller.decide(&decoder.decode(&input)?)?;
        // Then the controller never commands spontaneous rotation.
        assert_eq!(decision.report, neutral_report());
    }
    Ok(())
}

#[test]
fn magnitude_uses_host_elapsed_time_and_full_little_endian_cap() -> anyhow::Result<()> {
    // Given rapid forward motion and a cap requiring both magnitude bytes.
    let mut decoder = Decoder::default();
    decoder.decode(&frame(0, 10, [0, 0]))?;
    let motion = decoder.decode(&frame(1000, 11, [512, 0]))?;
    // When assistance reaches its cap.
    let decision = Controller::new(ControlArgs {
        mode: Mode::Assist,
        side: Side::Left,
        gain: 100,
        max_force: 1000,
        assist_force: None,
    })
    .decide(&motion)?;
    // Then units and the little-endian encoding preserve the actual bound.
    assert_eq!(decision.torque, [1000, 0]);
    assert_eq!(&decision.report[2..6], &[32, 1, 232, 3]);
    Ok(())
}

#[test]
fn fader_brake_waits_for_minimum_before_arming() -> anyhow::Result<()> {
    // Given a high startup fader and movement before the operator lowers it.
    let mut controller = Controller::new(ControlArgs {
        mode: Mode::FaderBrake,
        side: Side::Left,
        gain: 1,
        max_force: 5000,
        assist_force: None,
    });
    let mut decoder = Decoder::default();
    let mut trace = Vec::new();
    for (index, raw) in [4095, 4095, 32, 33, 0, 4095].into_iter().enumerate() {
        let index = u32::try_from(index)?;
        let motion = decoder.decode(&with_fader(
            frame(
                u64::from(index) * 2000,
                index + 1,
                [1000 + index * 10, 2000],
            ),
            raw,
        ))?;
        // When the actual controller processes the entire fader sequence.
        let decision = controller.decide(&motion)?;
        trace.push((decision.torque, decision.fader.map(|level| level.armed)));
    }
    // Then high startup is neutral, the minimum arms, and returning low is zero.
    assert_eq!(
        trace,
        vec![
            ([0, 0], Some(false)),
            ([0, 0], Some(false)),
            ([0, 0], Some(true)),
            ([-40, 0], Some(true)),
            ([0, 0], Some(true)),
            ([-5000, 0], Some(true)),
        ]
    );
    Ok(())
}

#[test]
fn fader_level_controls_force_independently_of_speed_and_reverses_only_to_brake()
-> anyhow::Result<()> {
    // Given an armed controller and the same fader level at different speeds.
    let mut controller = Controller::new(ControlArgs {
        mode: Mode::FaderBrake,
        side: Side::Left,
        gain: 1,
        max_force: 5000,
        assist_force: None,
    });
    let mut decoder = Decoder::default();
    let mut torques = Vec::new();
    for (index, (counter, raw)) in [
        (1000, 0),
        (1001, 2048),
        (1101, 2048),
        (1201, 4095),
        (1101, 4095),
        (1101, 4095),
        (1101, 4095),
    ]
    .into_iter()
    .enumerate()
    {
        let index = u32::try_from(index)?;
        let motion = decoder.decode(&with_fader(
            frame(u64::from(index) * 2000, index.min(5) + 1, [counter, 2000]),
            raw,
        ))?;
        // When the real decoder/controller sees movement, reversal, rest and duplicate.
        torques.push(controller.decide(&motion)?.torque);
    }
    // Then midpoint force stays constant, full scale reverses sign, and rest is zero.
    assert_eq!(
        torques,
        vec![
            [0, 0],
            [-2500, 0],
            [-2500, 0],
            [-5000, 0],
            [5000, 0],
            [0, 0],
            [0, 0]
        ]
    );
    Ok(())
}

#[test]
fn fader_mode_rejects_missing_analog_instead_of_inventing_a_level() -> anyhow::Result<()> {
    // Given historical valid jog evidence without an analog report.
    let motion = Decoder::default().decode(&frame(0, 1, [0, 0]))?;
    let mut controller = Controller::new(ControlArgs {
        mode: Mode::FaderBrake,
        side: Side::Left,
        gain: 1,
        max_force: 5000,
        assist_force: None,
    });
    // When the operator requests fader-controlled braking.
    // Then absent analog evidence cannot arm or produce a decision.
    assert!(controller.decide(&motion).is_err());
    Ok(())
}
