use super::*;
use crate::jog_diagnostic::decode::tests::frame;
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
        })
        .decide(&motion);
        // Then direction, bounded magnitude, and the other wheel's neutral agree.
        assert_eq!(decision.torque, torque);
        assert_eq!(decision.report, report);
    }
    Ok(())
}

#[test]
fn assist_is_neutral_at_rest_and_on_first_or_duplicate_samples() -> anyhow::Result<()> {
    // Given assistance and an initially stationary wheel.
    let controller = Controller::new(ControlArgs {
        mode: Mode::Assist,
        side: Side::Left,
        gain: 1,
        max_force: 100,
    });
    let mut decoder = Decoder::default();
    for input in [
        frame(0, 1, [0, 0]),
        frame(2000, 2, [0, 0]),
        frame(4000, 2, [0, 0]),
    ] {
        // When first/rest/duplicate input is processed.
        let decision = controller.decide(&decoder.decode(&input)?);
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
    })
    .decide(&motion);
    // Then units and the little-endian encoding preserve the actual bound.
    assert_eq!(decision.torque, [1000, 0]);
    assert_eq!(&decision.report[2..6], &[32, 1, 232, 3]);
    Ok(())
}
