use super::assist_tests::step;
use super::capture::RawFrame;
use super::decode::tests::{frame, with_fader};
use super::motor::neutral_report;
use super::session::{Event, Session};
use super::*;
use std::cell::Cell;

pub(super) fn control(side: Side) -> ControlArgs {
    ControlArgs {
        mode: Mode::FaderPlay,
        side,
        gain: 1,
        max_force: 30000,
        assist_force: Some(7000),
    }
}

pub(super) fn input(index: u32, counters: [u32; 2], selection: (u16, u8)) -> RawFrame {
    let mut raw = with_fader(
        frame(u64::from(index) * 2000, index + 1, counters),
        selection.0,
    );
    raw.buttons[17] = selection.1;
    raw
}

#[test]
fn right_selection_uses_right_motion_touch_and_packet_block_only() -> anyhow::Result<()> {
    for (raw, zone_sign) in [(0, 1), (4095, -1)] {
        for direction in [-1, 1] {
            let mut c = Controller::new(control(Side::Right));
            let mut d = Decoder::default();
            // Given opposite left/right movement and touch only on the right.
            for index in 0..=80 {
                let sample = input(
                    index,
                    [
                        1000 + index * 2,
                        if direction > 0 {
                            2000 + index * 2
                        } else {
                            2000 - index * 2
                        },
                    ],
                    (if index == 0 { 2048 } else { raw }, 0x20),
                );
                // When real reports and output clocks drive the selected wheel.
                step(&mut d, &mut c, &sample)?;
                let out = c.output_at(u64::from(index) * 2000).context("clock")?;
                assert_eq!(out.torque[0], 0);
                assert_eq!(&out.report[1..6], &neutral_report()[1..6]);
            }
            // Then the right native block carries the right sign, not left movement.
            let out = c.output_at(160_000).context("active")?;
            assert_eq!(out.torque[1].signum(), zone_sign * direction);
            assert_ne!(&out.report[9..11], &[0, 0]);
        }
    }
    Ok(())
}

#[test]
fn both_wheels_have_independent_touch_reversal_and_inactivity_state() -> anyhow::Result<()> {
    let mut c = Controller::new(control(Side::Both));
    let mut d = Decoder::default();
    // Given simultaneous assistance in opposite directions.
    for index in 0..=40 {
        step(
            &mut d,
            &mut c,
            &input(
                index,
                [1000 + index * 2, 2000 - index * 2],
                (if index == 0 { 2048 } else { 0 }, 0x30),
            ),
        )?;
        c.output_at(u64::from(index) * 2000).context("clock")?;
    }
    let active = c.output_at(80000).context("active")?;
    assert!(active.torque[0] > 0 && active.torque[1] < 0);
    // When only the right touch releases during continued movement.
    let released = step(&mut d, &mut c, &input(41, [1082, 1918], (0, 0x10)))?;
    assert!(released.torque[0] > 0);
    assert_eq!(released.torque[1], 0);
    c.output_at(82000).context("clock")?;
    // Then stationary right retouch cannot borrow left history.
    step(&mut d, &mut c, &input(42, [1084, 1918], (0, 0x30)))?;
    assert_eq!(c.output_at(84000).context("retouch")?.torque[1], 0);
    for index in 43..=50 {
        step(
            &mut d,
            &mut c,
            &input(
                index,
                [1084 + (index - 42) * 2, 1918 - (index - 42) * 2],
                (0, 0x30),
            ),
        )?;
        c.output_at(u64::from(index) * 2000).context("clock")?;
    }
    let reversed = step(&mut d, &mut c, &input(51, [1098, 1900], (0, 0x30)))?;
    assert_eq!(reversed.torque[0], 0);
    assert!(reversed.torque[1] < 0);
    c.output_at(102_000).context("clock")?;
    for index in 52..=56 {
        step(
            &mut d,
            &mut c,
            &input(index, [1098 - (index - 51) * 2, 1900], (0, 0x30)),
        )?;
        c.output_at(u64::from(index) * 2000).context("clock")?;
    }
    let rested = c.output_at(112_000).context("rest")?;
    assert!(rested.torque[0] < 0);
    assert_eq!(rested.torque[1], 0);
    Ok(())
}

#[test]
fn shared_zone_changes_reset_both_independent_histories() -> anyhow::Result<()> {
    let mut c = Controller::new(control(Side::Both));
    let mut d = Decoder::default();
    for index in 0..=40 {
        step(
            &mut d,
            &mut c,
            &input(
                index,
                [1000 + index * 2, 2000 - index * 2],
                (if index == 0 { 2048 } else { 0 }, 0x30),
            ),
        )?;
        c.output_at(u64::from(index) * 2000).context("clock")?;
    }
    // Given both active, when the common fader jumps straight to brake.
    let changed = step(&mut d, &mut c, &input(41, [1082, 1918], (4095, 0x30)))?;
    // Then neither old force survives; each side must qualify fresh motion.
    assert_eq!(changed.report, neutral_report());
    assert_eq!(c.output_at(82000).context("change")?.torque, [0, 0]);
    for index in 42..=46 {
        step(
            &mut d,
            &mut c,
            &input(index, [1082 + (index - 41) * 2, 1918], (4095, 0x30)),
        )?;
        c.output_at(u64::from(index) * 2000).context("clock")?;
    }
    let output = c.output_at(92000).context("new brake")?;
    assert!(output.torque[0] < 0);
    assert_eq!(output.torque[1], 0);
    Ok(())
}

#[test]
fn one_wheel_release_writes_partial_neutral_before_the_next_clock() -> anyhow::Result<()> {
    for (held, stopped) in [(0x10, 1), (0x20, 0)] {
        let mut writes = Vec::new();
        let mut log = Vec::new();
        let calls = Cell::new(0);
        {
            let mut s = Session::new(
                control(Side::Both),
                |r| {
                    calls.set(calls.get() + 1);
                    writes.push(r.to_vec());
                    Ok(r.len())
                },
                &mut log,
            )?;
            for index in 0..=12 {
                let at = u64::from(index) * 2000;
                s.handle(
                    Event::Sample(input(
                        index,
                        [1000 + index * 2, 2000 - index * 2],
                        (if index == 0 { 2048 } else { 0 }, 0x30),
                    )),
                    at,
                )?;
                if index > 0 {
                    s.handle(Event::OutputTick, at)?;
                }
            }
            // Given two nonzero physical motor blocks, when either one releases.
            let before = calls.get();
            s.handle(Event::Sample(input(13, [1026, 1974], (0, held))), 26000)?;
            // Then a partial neutral write must precede the next periodic output.
            assert_eq!(calls.get(), before + 1);
            s.handle(Event::Stop, 27000)?;
        }
        let partial = &writes[writes.len() - 2];
        let zero = 1 + stopped * 5;
        let peer = 1 + (1 - stopped) * 5;
        assert_eq!(&partial[zero..zero + 5], &neutral_report()[zero..zero + 5]);
        assert_ne!(&partial[peer + 3..peer + 5], &[0, 0]);
        assert_eq!(
            &partial[peer..peer + 5],
            &writes[writes.len() - 3][peer..peer + 5]
        );
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    }
    Ok(())
}
