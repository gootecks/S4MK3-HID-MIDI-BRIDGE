use super::*;

fn movement(
    engine: &mut Engine,
    index: u32,
    touch: [bool; 2],
    qualified: [bool; 2],
) -> anyhow::Result<[u8; 11]> {
    let at = u64::from(index) * 2000;
    let counts = [1_000_000 - index * 2, 2_000_000 - index * 2];
    let (buttons, jog) = sample(at, index + 1, counts, touch);
    engine.observe(&buttons, &jog, at, at)?;
    assert!(engine.set_backspin(qualified).is_none());
    engine.tick(at)
}

fn assisted(setting: i32) -> anyhow::Result<Engine> {
    let mut engine = Engine::new(selected(setting)?);
    for index in 0..=200 {
        movement(&mut engine, index, [true; 2], [index >= 8, false])?;
    }
    Ok(engine)
}

#[test]
fn selected_strength_scales_only_the_assist_and_keeps_the_release_deadline() -> anyhow::Result<()> {
    for strength in [0, 25, 50, 100] {
        // Given normal -25 tension and a separately selected backspin strength.
        let mut engine = Engine::with_backspin_strength(selected(-25)?, strength)?;
        for index in 0..=200 {
            movement(&mut engine, index, [true; 2], [index >= 8, false])?;
        }
        let held = engine.tick(400_000)?;
        let expected_held = if strength == 0 {
            1750
        } else {
            70 * u16::from(strength)
        };
        assert_eq!(magnitude(held, 0), expected_held);
        // When the left hand releases and real movement continues.
        for index in 201..=800 {
            let report = movement(&mut engine, index, [false, true], [true, false])?;
            let elapsed = u64::from(index - 201) * 2000;
            let expected =
                7000 * u64::from(strength) * 1_000_000_u64.saturating_sub(elapsed) / 100_000_000;
            // Then only the override scales; zero acts off and the fixed deadline survives.
            assert_eq!(u64::from(magnitude(report, 0)), expected);
            assert_eq!(magnitude(report, 1), 1750);
        }
        assert_eq!(engine.selection.selected_force, 1750);
    }
    Ok(())
}

#[test]
fn excessive_strength_cannot_construct_a_motor_engine() -> anyhow::Result<()> {
    // Given a value beyond the supported percentage range.
    let config = selected(-25)?;
    // When an external caller attempts to construct an engine.
    let result = Engine::with_backspin_strength(config, 101);
    // Then it fails before any force state or device access exists.
    assert!(result.is_err());
    Ok(())
}

#[test]
fn released_assist_fades_on_output_clock_without_refresh_from_motion() -> anyhow::Result<()> {
    // Given one qualified wheel at the existing maximum, and normal -25 on the other.
    let mut engine = assisted(-25)?;
    assert_eq!(magnitude(engine.tick(400_000)?, 0), 7000);
    // When physical touch releases and fresh backward movement continues past one second.
    for index in 201..=800 {
        let report = movement(&mut engine, index, [false, true], [true, false])?;
        let elapsed = u64::from(index - 201) * 2000;
        let expected = 7000 * 1_000_000_u64.saturating_sub(elapsed) / 1_000_000;
        // Then real touch stays false, output follows the fade, and no later movement rearms it.
        assert!(!engine.motion.context("decoded physical motion")?.touch[0]);
        assert_eq!(u64::from(magnitude(report, 0)), expected);
        assert_eq!(magnitude(report, 1), 1750);
        assert!(magnitude(report, 0) <= MAX_ASSIST_FORCE);
    }
    Ok(())
}

#[test]
fn release_deadline_is_enforced_without_a_new_snapshot() -> anyhow::Result<()> {
    // Given a released event with a still-fresh moving snapshot just before the deadline.
    let mut engine = assisted(-25)?;
    for index in 201..=700 {
        movement(&mut engine, index, [false, true], [true, false])?;
    }
    let mut normal = Engine::new(selected(-25)?);
    for index in 0..=700 {
        movement(&mut normal, index, [index <= 200, true], [false; 2])?;
    }
    // When the output clock reaches one second from physical release, without new input.
    let report = engine.tick(1_402_000)?;
    // Then the assisted wheel is neutral even though its motion window remains qualified.
    assert_eq!(magnitude(report, 0), 0);
    assert_eq!(report[6..], normal.tick(1_402_000)?[6..]);
    Ok(())
}

#[test]
fn retouch_cancels_before_requalification_and_restores_normal_assistance() -> anyhow::Result<()> {
    // Given released assistance on the left and ordinary assistance on the right.
    let mut engine = assisted(-25)?;
    for index in 201..=210 {
        assert!(
            magnitude(
                movement(&mut engine, index, [false, true], [true, false])?,
                0
            ) > 1750
        );
    }
    let at = 422_000;
    let (buttons, jog) = sample(at, 212, [999_578, 1_999_578], [true; 2]);
    // When the user retouches while the old qualification flag remains true.
    let immediate = engine
        .observe(&buttons, &jog, at, at)?
        .context("retouch zero")?;
    assert!(engine.set_backspin([true, false]).is_none());
    // Then the override clears immediately; later held movement only gets the saved -25.
    assert_eq!(magnitude(immediate, 0), 0);
    assert_eq!(magnitude(immediate, 1), 1750);
    for index in 212..=410 {
        let report = movement(&mut engine, index, [true; 2], [true, false])?;
        assert!(magnitude(report, 0) <= 1750);
    }
    assert_eq!(magnitude(engine.tick(820_000)?, 0), 1750);
    Ok(())
}

#[test]
fn cancelled_event_zeroes_only_its_override_before_the_next_tick() -> anyhow::Result<()> {
    // Given active left-only assistance after release.
    let mut engine = assisted(-25)?;
    movement(&mut engine, 201, [false, true], [true, false])?;
    // When the full-profile translator cancels that event.
    let immediate = engine
        .set_backspin([false; 2])
        .context("event cancellation")?;
    // Then only that wheel is zeroed, and stale true flags cannot restart an untouched wheel.
    assert_eq!(magnitude(immediate, 0), 0);
    assert_eq!(magnitude(immediate, 1), 1750);
    for index in 202..=240 {
        assert_eq!(
            magnitude(
                movement(&mut engine, index, [false, true], [true, false])?,
                0
            ),
            0
        );
    }
    Ok(())
}

#[test]
fn rest_and_reversal_cancel_even_if_software_qualification_is_delayed() -> anyhow::Result<()> {
    for reverse in [false, true] {
        // Given a released assisted wheel with a stale qualified software event.
        let mut engine = assisted(-25)?;
        movement(&mut engine, 201, [false, true], [true, false])?;
        // When counts stop for the existing 8 ms inactivity window or reverse immediately.
        let mut last = neutral_report();
        for index in 202..=205 {
            let at = u64::from(index) * 2000;
            let left = if reverse {
                999_598 + (index - 201) * 2
            } else {
                999_598
            };
            let (buttons, jog) =
                sample(at, index + 1, [left, 2_000_000 - index * 2], [false, true]);
            engine.observe(&buttons, &jog, at, at)?;
            assert!(engine.set_backspin([true, false]).is_none());
            last = engine.tick(at)?;
            if reverse {
                assert_eq!(magnitude(last, 0), 0);
            }
        }
        // Then force clears without waiting for MIDI ownership or the one-second deadline.
        assert_eq!(magnitude(last, 0), 0);
        assert_eq!(magnitude(last, 1), 1750);
    }
    Ok(())
}

#[test]
fn freshness_reset_cannot_resume_a_previous_released_pulse() -> anyhow::Result<()> {
    // Given a released assist pulse and a subsequent freshness suspension.
    let mut engine = assisted(-25)?;
    movement(&mut engine, 201, [false, true], [true, false])?;
    engine.clear_motion();
    // When fresh baseline/movement returns with old true qualification but no physical touch.
    for index in 300..=350 {
        let report = movement(&mut engine, index, [false; 2], [true, false])?;
        // Then old assistance stays cancelled; ordinary freshness checks still apply.
        assert_eq!(report, neutral_report());
    }
    assert!(engine.tick(750_001).is_err());
    Ok(())
}

#[test]
fn normal_braking_selection_returns_after_the_pulse_without_affecting_other_wheel()
-> anyhow::Result<()> {
    // Given saved +50 braking, overridden only on the left during its detected event.
    let mut engine = assisted(50)?;
    assert_eq!(magnitude(engine.tick(400_000)?, 0), 7000);
    // When the released pulse runs to its fixed deadline.
    let mut final_report = neutral_report();
    for index in 201..=701 {
        final_report = movement(&mut engine, index, [false; 2], [true, false])?;
        assert_eq!(magnitude(final_report, 1), 15000);
    }
    // Then the original braking selection, direction and strength return.
    assert_eq!(magnitude(final_report, 0), 15000);
    assert_ne!(final_report[3], 0xfe);
    assert_eq!(engine.selection.selected_force, 15000);
    Ok(())
}

#[test]
fn either_or_both_wheels_keep_separate_release_deadlines() -> anyhow::Result<()> {
    for enabled in [[true, false], [false, true], [true; 2]] {
        // Given neutral normal tension and independently qualified wheel selections.
        let mut engine = Engine::new(selected(0)?);
        for index in 0..=200 {
            let qualified = if index >= 8 { enabled } else { [false; 2] };
            movement(&mut engine, index, [true; 2], qualified)?;
        }
        // When the left hand releases 100 ms before the right.
        for index in 201..=780 {
            let touch = [false, index < 251];
            let report = movement(&mut engine, index, touch, enabled)?;
            // Then neither wheel borrows the other's permission or release deadline.
            for (side, release_index) in [(0, 201), (1, 251)] {
                let elapsed = u64::from(index.saturating_sub(release_index)) * 2000;
                let expected = if enabled[side] {
                    7000 * 1_000_000_u64.saturating_sub(elapsed) / 1_000_000
                } else {
                    0
                };
                assert_eq!(u64::from(magnitude(report, side)), expected);
            }
        }
    }
    Ok(())
}
