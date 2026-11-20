use std::time::Duration;

use super::tests::{button, run, slider, tap};
use super::{Layout, PadMode, Translator};
use crate::full::feedback::Message;
use crate::s8::feedback::FeedbackState;
use crate::s8::led_state::{Local, present};
use crate::s8::leds::{Level, Side};

#[test]
fn evan_pad_modes_do_not_take_over_the_ni_buttons() {
    let mut t = Translator::with_layout(Layout::Evan);
    assert_eq!(tap(&mut t, "left_pad_1"), [[0x90, 40, 127], [0x80, 40, 0]]);
    for number in [1, 4] {
        let name = if number == 1 {
            "left_display_1"
        } else {
            "left_display_4"
        };
        assert!(tap(&mut t, name).is_empty());
    }
    assert_eq!(tap(&mut t, "left_freeze"), [[0x90, 27, 127], [0x80, 27, 0]]);
    assert_eq!(tap(&mut t, "left_pad_1"), [[0x90, 40, 127], [0x80, 40, 0]]);
    run(&mut t, &[button("left_shift", true)]);
    assert_eq!(tap(&mut t, "left_freeze"), [[0x98, 27, 127], [0x88, 27, 0]]);
    run(&mut t, &[button("left_shift", false)]);
    assert_eq!(tap(&mut t, "left_pad_1"), [[0x90, 72, 127], [0x80, 72, 0]]);
    run(&mut t, &[button("left_shift", true)]);
    assert_eq!(tap(&mut t, "left_hotcue"), [[0x98, 25, 127], [0x88, 25, 0]]);
    assert_eq!(tap(&mut t, "left_pad_1"), [[0x98, 40, 127], [0x88, 40, 0]]);
}

#[test]
fn evan_shift_freeze_toggles_each_side_and_preserves_note_owners() {
    let mut t = Translator::with_layout(Layout::Evan);
    run(&mut t, &[button("left_shift", true)]);
    assert_eq!(tap(&mut t, "left_freeze"), [[0x98, 27, 127], [0x88, 27, 0]]);
    assert_eq!(
        (t.pad_mode(0), t.pad_mode(1)),
        (PadMode::Freeze, PadMode::Hotcue)
    );

    run(
        &mut t,
        &[button("right_shift", true), button("left_shift", false)],
    );
    assert_eq!(
        tap(&mut t, "right_freeze"),
        [[0x99, 27, 127], [0x89, 27, 0]]
    );
    assert_eq!(
        run(&mut t, &[button("left_pad_1", true)]),
        [[0x90, 72, 127]]
    );

    run(&mut t, &[button("left_shift", true)]);
    assert_eq!(
        run(&mut t, &[button("left_freeze", true)]),
        [[0x98, 25, 127]]
    );
    assert_eq!(
        (t.pad_mode(0), t.pad_mode(1)),
        (PadMode::Hotcue, PadMode::Freeze)
    );
    assert!(run(&mut t, &[button("left_freeze", true)]).is_empty());
    run(&mut t, &[button("left_shift", false)]);
    tap(&mut t, "left_deck");
    assert_eq!(
        run(&mut t, &[button("left_freeze", false)]),
        [[0x88, 25, 0]]
    );
    assert_eq!(run(&mut t, &[button("left_pad_1", false)]), [[0x80, 72, 0]]);
    assert_eq!(tap(&mut t, "left_pad_1"), [[0x92, 40, 127], [0x82, 40, 0]]);
    assert_eq!(
        tap(&mut t, "right_freeze"),
        [[0x99, 25, 127], [0x89, 25, 0]]
    );
    assert_eq!(t.pad_mode(1), PadMode::Hotcue);
}

#[test]
fn both_layouts_light_deck_cue_while_held_without_djay_feedback() {
    let state = FeedbackState::default();
    let cue_level =
        |t: &Translator| present(Side::Left, &state, &Local::from_translator(t, Side::Left)).cue;
    for layout in [Layout::Default, Layout::Evan] {
        let mut t = Translator::with_layout(layout);
        assert_eq!(cue_level(&t), Level::Off);
        assert_eq!(run(&mut t, &[button("left_cue", true)]), [[0x90, 1, 127]]);
        assert_eq!(cue_level(&t), Level::On);
        assert_eq!(run(&mut t, &[button("left_cue", false)]), [[0x80, 1, 0]]);
        assert_eq!(cue_level(&t), Level::Off);
    }
}

#[test]
fn evan_modifier_is_shared_and_releases_held_notes_on_the_original_deck() {
    let mut t = Translator::with_layout(Layout::Evan);
    assert!(run(&mut t, &[button("left_flux", true)]).is_empty());
    assert!(
        run(
            &mut t,
            &[button("right_edit", true), button("left_flux", false)]
        )
        .is_empty()
    );
    assert_eq!(tap(&mut t, "left_on_1"), [[0x98, 56, 127], [0x88, 56, 0]]);
    assert_eq!(
        run(&mut t, &[button("left_pad_1", true)]),
        [[0x98, 64, 127]]
    );
    assert!(run(&mut t, &[button("right_edit", false)]).is_empty());
    tap(&mut t, "left_deck");
    assert_eq!(run(&mut t, &[button("left_pad_1", false)]), [[0x88, 64, 0]]);
    assert_eq!(tap(&mut t, "left_pad_1"), [[0x92, 40, 127], [0x82, 40, 0]]);
}

#[test]
fn evan_tempo_notes_do_not_reassign_fx_when_changing_decks() {
    let mut t = Translator::with_layout(Layout::Evan);
    assert_eq!(
        tap(&mut t, "mixer_fx_c_1"),
        [[0x94, 96, 127], [0x84, 96, 0]]
    );
    tap(&mut t, "left_deck");
    assert_eq!(tap(&mut t, "left_fx_1"), [[0x90, 32, 127], [0x80, 32, 0]]);
    assert_eq!(
        run(&mut t, &[slider("left_fx_dial_1", 1.0)]),
        [[0xb0, 32, 127]]
    );
}

fn fx_feedback(state: &mut FeedbackState, status: u8, number: u8, value: u8) {
    state.apply(
        Message {
            status,
            number,
            value,
        },
        Duration::ZERO,
    );
}

fn tap_fx4(t: &mut Translator, state: &FeedbackState, name: &'static str) -> Vec<[u8; 3]> {
    let mut out = Vec::new();
    t.handle(&button(name, true), false, Some(state), &mut out);
    t.handle(&button(name, false), false, Some(state), &mut out);
    out
}

#[test]
fn evan_fx4_turns_only_enabled_slots_off_on_its_own_side() {
    let mut t = Translator::with_layout(Layout::Evan);
    let mut feedback = FeedbackState::default();
    fx_feedback(&mut feedback, 0x90, 32, 127);
    fx_feedback(&mut feedback, 0x91, 38, 127);
    fx_feedback(&mut feedback, 0x92, 33, 127);
    tap(&mut t, "left_deck");
    run(&mut t, &[button("left_shift", true)]);
    assert_eq!(
        tap_fx4(&mut t, &feedback, "left_fx_4"),
        [[0x90, 32, 127], [0x80, 32, 0]]
    );
    assert_eq!(
        tap_fx4(&mut t, &feedback, "right_fx_4"),
        [[0x91, 38, 127], [0x81, 38, 0]]
    );
    let leds = |side| present(side, &feedback, &Local::from_translator(&t, side)).fx;
    assert_eq!(
        leds(Side::Left),
        [Level::On, Level::Dim, Level::Dim, Level::On]
    );
    assert_eq!(
        leds(Side::Right),
        [Level::Dim, Level::Dim, Level::On, Level::On]
    );
}

#[test]
fn evan_fx4_flips_all_three_slots_when_feedback_says_all_off_or_all_on() {
    let mut t = Translator::with_layout(Layout::Evan);
    let mut feedback = FeedbackState::default();
    let all_three = [
        [0x90, 32, 127],
        [0x80, 32, 0],
        [0x90, 33, 127],
        [0x80, 33, 0],
        [0x90, 34, 127],
        [0x80, 34, 0],
    ];
    for number in 32..=34 {
        fx_feedback(&mut feedback, 0x90, number, 127);
    }
    assert_eq!(tap_fx4(&mut t, &feedback, "left_fx_4"), all_three);

    for number in 32..=34 {
        fx_feedback(&mut feedback, 0x80, number, 64);
    }
    let local = Local::from_translator(&t, Side::Left);
    assert_eq!(present(Side::Left, &feedback, &local).fx, [Level::Dim; 4]);
    assert_eq!(tap_fx4(&mut t, &feedback, "left_fx_4"), all_three);
}

#[test]
fn evan_fx4_without_feedback_taps_all_slots_once_per_press() {
    let mut t = Translator::with_layout(Layout::Evan);
    let mut out = Vec::new();
    let press = button("right_fx_4", true);
    let release = button("right_fx_4", false);
    t.handle(&press, true, None, &mut out);
    t.handle(&release, false, None, &mut out);
    assert!(out.is_empty());

    t.handle(&press, false, None, &mut out);
    assert_eq!(
        out,
        [
            [0x91, 36, 127],
            [0x81, 36, 0],
            [0x91, 37, 127],
            [0x81, 37, 0],
            [0x91, 38, 127],
            [0x81, 38, 0],
        ]
    );
    out.clear();
    t.handle(&press, false, None, &mut out);
    t.handle(&release, false, None, &mut out);
    assert!(out.is_empty());
}
