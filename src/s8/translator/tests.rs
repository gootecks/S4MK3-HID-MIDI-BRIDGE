use encdr::{DeviceId, Event};

use super::Translator;

pub(super) fn button(name: &'static str, pressed: bool) -> Event {
    Event::Button {
        device: DeviceId::default(),
        name,
        pressed,
    }
}

pub(super) fn touch(name: &'static str, touched: bool) -> Event {
    Event::Touch {
        device: DeviceId::default(),
        name,
        touched,
    }
}

pub(super) fn encoder(name: &'static str, delta: i32) -> Event {
    Event::Encoder {
        device: DeviceId::default(),
        name,
        delta,
    }
}

pub(super) fn slider(name: &'static str, value: f32) -> Event {
    Event::Slider {
        device: DeviceId::default(),
        name,
        value,
    }
}

pub(super) fn report(translator: &mut Translator, resting: bool, events: &[Event]) -> Vec<[u8; 3]> {
    let mut out = Vec::new();
    for event in events {
        translator.handle(event, resting, &mut out);
    }
    translator.end_report(&mut out);
    out
}

pub(super) fn run(translator: &mut Translator, events: &[Event]) -> Vec<[u8; 3]> {
    report(translator, false, events)
}

pub(super) fn tap(translator: &mut Translator, name: &'static str) -> Vec<[u8; 3]> {
    let mut out = run(translator, &[button(name, true)]);
    out.extend(run(translator, &[button(name, false)]));
    out
}

#[test]
fn a_button_sends_note_on_then_note_off_on_its_deck_channel() {
    let mut translator = Translator::new();

    assert_eq!(
        run(&mut translator, &[button("left_play", true)]),
        [[0x90, 0, 127]]
    );
    assert_eq!(
        run(&mut translator, &[button("left_play", false)]),
        [[0x80, 0, 0]]
    );
    assert_eq!(
        tap(&mut translator, "right_play"),
        [[0x91, 0, 127], [0x81, 0, 0]]
    );
}

#[test]
fn shift_selects_the_shift_row_of_its_own_side_only() {
    let mut translator = Translator::new();
    run(&mut translator, &[button("left_shift", true)]);

    assert_eq!(
        tap(&mut translator, "left_cue"),
        [[0x98, 1, 127], [0x88, 1, 0]]
    );
    assert_eq!(
        tap(&mut translator, "right_cue"),
        [[0x91, 1, 127], [0x81, 1, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_sync"),
        [[0x90, 2, 127], [0x80, 2, 0]]
    );
    assert_eq!(
        tap(&mut translator, "mixer_cue_b"),
        [[0x94, 1, 127], [0x84, 1, 0]]
    );
}

#[test]
fn release_goes_to_the_address_pressed_when_shift_changes_in_between() {
    let mut translator = Translator::new();

    run(&mut translator, &[button("left_cue", true)]);
    run(&mut translator, &[button("left_shift", true)]);
    assert_eq!(
        run(&mut translator, &[button("left_cue", false)]),
        [[0x80, 1, 0]]
    );

    run(&mut translator, &[button("left_cue", true)]);
    run(&mut translator, &[button("left_shift", false)]);
    assert_eq!(
        run(&mut translator, &[button("left_cue", false)]),
        [[0x88, 1, 0]]
    );
}

#[test]
fn deck_button_toggles_deck_sends_deck_select_and_moves_later_controls() {
    let mut translator = Translator::new();
    run(&mut translator, &[button("left_play", true)]);

    assert_eq!(
        run(&mut translator, &[button("left_deck", true)]),
        [[0x95, 2, 127]]
    );
    assert_eq!(
        run(&mut translator, &[button("left_deck", false)]),
        [[0x85, 2, 0]]
    );
    assert_eq!(
        run(&mut translator, &[button("left_play", false)]),
        [[0x80, 0, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_play"),
        [[0x92, 0, 127], [0x82, 0, 0]]
    );
    assert_eq!(
        tap(&mut translator, "right_play"),
        [[0x91, 0, 127], [0x81, 0, 0]]
    );

    assert_eq!(
        tap(&mut translator, "right_deck"),
        [[0x95, 3, 127], [0x85, 3, 0]]
    );
    assert_eq!(
        tap(&mut translator, "right_play"),
        [[0x93, 0, 127], [0x83, 0, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_deck"),
        [[0x95, 0, 127], [0x85, 0, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_play"),
        [[0x90, 0, 127], [0x80, 0, 0]]
    );
}

#[test]
fn release_goes_to_the_deck_pressed_when_the_deck_toggles_in_between() {
    let mut translator = Translator::new();
    run(&mut translator, &[button("left_cue", true)]);
    run(&mut translator, &[button("left_deck", true)]);

    assert_eq!(
        run(&mut translator, &[button("left_cue", false)]),
        [[0x80, 1, 0]]
    );
}

#[test]
fn each_pad_mode_hits_its_catalog_rows() {
    let mut translator = Translator::new();
    assert_eq!(
        tap(&mut translator, "left_pad_1"),
        [[0x90, 40, 127], [0x80, 40, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_pad_8"),
        [[0x90, 47, 127], [0x80, 47, 0]]
    );

    assert!(tap(&mut translator, "left_loop").is_empty());
    assert_eq!(
        tap(&mut translator, "left_pad_1"),
        [[0x90, 64, 127], [0x80, 64, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_pad_8"),
        [[0x90, 71, 127], [0x80, 71, 0]]
    );

    assert_eq!(
        tap(&mut translator, "left_freeze"),
        [[0x90, 27, 127], [0x80, 27, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_pad_1"),
        [[0x90, 72, 127], [0x80, 72, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_pad_8"),
        [[0x90, 79, 127], [0x80, 79, 0]]
    );

    assert!(tap(&mut translator, "left_remix").is_empty());
    assert_eq!(
        tap(&mut translator, "left_pad_1"),
        [[0x90, 48, 127], [0x80, 48, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_pad_8"),
        [[0x90, 55, 127], [0x80, 55, 0]]
    );

    assert!(tap(&mut translator, "left_hotcue").is_empty());
    assert_eq!(
        tap(&mut translator, "left_pad_2"),
        [[0x90, 41, 127], [0x80, 41, 0]]
    );
}

#[test]
fn pad_mode_is_kept_per_side_and_sampler_pads_follow_the_deck() {
    let mut translator = Translator::new();
    tap(&mut translator, "left_loop");

    assert_eq!(
        tap(&mut translator, "right_pad_1"),
        [[0x91, 40, 127], [0x81, 40, 0]]
    );
    tap(&mut translator, "right_remix");
    tap(&mut translator, "right_deck");
    assert_eq!(
        tap(&mut translator, "right_pad_1"),
        [[0x93, 48, 127], [0x83, 48, 0]]
    );
}

#[test]
fn shift_picks_the_shift_row_only_where_the_pad_layer_has_one() {
    let mut translator = Translator::new();
    run(&mut translator, &[button("left_shift", true)]);
    assert_eq!(
        tap(&mut translator, "left_pad_1"),
        [[0x98, 40, 127], [0x88, 40, 0]]
    );

    assert_eq!(
        tap(&mut translator, "left_freeze"),
        [[0x98, 27, 127], [0x88, 27, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_pad_1"),
        [[0x90, 72, 127], [0x80, 72, 0]]
    );

    tap(&mut translator, "left_remix");
    assert_eq!(
        tap(&mut translator, "left_pad_1"),
        [[0x98, 48, 127], [0x88, 48, 0]]
    );

    tap(&mut translator, "left_loop");
    assert_eq!(
        tap(&mut translator, "left_pad_1"),
        [[0x90, 64, 127], [0x80, 64, 0]]
    );
}

#[test]
fn release_goes_to_the_pad_pressed_when_the_pad_mode_changes_in_between() {
    let mut translator = Translator::new();
    run(&mut translator, &[button("left_pad_1", true)]);
    tap(&mut translator, "left_loop");

    assert_eq!(
        run(&mut translator, &[button("left_pad_1", false)]),
        [[0x80, 40, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_pad_1"),
        [[0x90, 64, 127], [0x80, 64, 0]]
    );
}

#[test]
fn fx_assign_retargets_its_unit_and_notes_are_released_where_pressed() {
    let mut translator = Translator::new();
    assert_eq!(
        tap(&mut translator, "left_fx_1"),
        [[0x90, 32, 127], [0x80, 32, 0]]
    );
    assert_eq!(
        tap(&mut translator, "right_fx_2"),
        [[0x91, 37, 127], [0x81, 37, 0]]
    );

    run(&mut translator, &[button("left_fx_1", true)]);
    assert!(tap(&mut translator, "mixer_fx_c_1").is_empty());
    assert_eq!(
        run(&mut translator, &[button("left_fx_1", false)]),
        [[0x80, 32, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_fx_1"),
        [[0x92, 32, 127], [0x82, 32, 0]]
    );

    assert!(tap(&mut translator, "mixer_fx_d_2").is_empty());
    assert_eq!(
        tap(&mut translator, "right_fx_2"),
        [[0x93, 37, 127], [0x83, 37, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_fx_3"),
        [[0x92, 34, 127], [0x82, 34, 0]]
    );

    run(&mut translator, &[button("left_shift", true)]);
    assert_eq!(
        tap(&mut translator, "left_fx_2"),
        [[0x9a, 33, 127], [0x8a, 33, 0]]
    );
}

#[test]
fn the_fx_unit_target_is_independent_of_the_deck_of_its_side() {
    let mut translator = Translator::new();
    tap(&mut translator, "left_deck");

    assert_eq!(
        tap(&mut translator, "left_fx_1"),
        [[0x90, 32, 127], [0x80, 32, 0]]
    );
    tap(&mut translator, "mixer_fx_b_1");
    assert_eq!(
        tap(&mut translator, "left_fx_1"),
        [[0x91, 32, 127], [0x81, 32, 0]]
    );
    assert_eq!(
        tap(&mut translator, "left_play"),
        [[0x92, 0, 127], [0x82, 0, 0]]
    );
    assert_eq!(
        run(&mut translator, &[slider("left_fx_dial_2", 1.0)]),
        [[0xb1, 32, 127]]
    );
}

#[test]
fn relative_encoders_send_clamped_steps_around_64_and_skip_zero() {
    let mut translator = Translator::new();
    let cases = [
        (1, 65),
        (-1, 63),
        (200, 127),
        (-200, 1),
        (63, 127),
        (-63, 1),
    ];
    for (delta, value) in cases {
        assert_eq!(
            run(&mut translator, &[encoder("left_loop_encoder", delta)]),
            [[0xb0, 1, value]],
            "delta {delta}"
        );
    }
    assert!(run(&mut translator, &[encoder("left_loop_encoder", 0)]).is_empty());

    run(&mut translator, &[button("left_shift", true)]);
    assert_eq!(
        run(&mut translator, &[encoder("left_loop_encoder", 2)]),
        [[0xb8, 1, 66]]
    );
    assert_eq!(
        run(&mut translator, &[encoder("tempo_encoder", 2)]),
        [[0xb4, 26, 66]]
    );
    assert_eq!(
        run(&mut translator, &[encoder("right_browse_encoder", -3)]),
        [[0xb1, 3, 61]]
    );
    assert_eq!(
        tap(&mut translator, "left_loop_encoder_press"),
        [[0x98, 6, 127], [0x88, 6, 0]]
    );
}

#[test]
fn sliders_send_only_when_the_seven_bit_value_changes() {
    let mut translator = Translator::new();

    assert_eq!(
        run(&mut translator, &[slider("left_fader_1", 1.0)]),
        [[0xb0, 48, 127]]
    );
    assert!(run(&mut translator, &[slider("left_fader_1", 1.0)]).is_empty());
    assert_eq!(
        run(&mut translator, &[slider("left_fader_1", 0.5)]),
        [[0xb0, 48, 64]]
    );
    assert!(run(&mut translator, &[slider("left_fader_1", 0.503)]).is_empty());
    assert_eq!(
        run(&mut translator, &[slider("mixer_fader_a", 0.0)]),
        [[0xb4, 16, 0]]
    );
    assert_eq!(
        run(&mut translator, &[slider("crossfader", 1.0)]),
        [[0xb4, 20, 127]]
    );
}

#[test]
fn resting_reports_sync_mixer_sliders_and_only_seed_the_rest() {
    let mut translator = Translator::new();

    assert_eq!(
        report(&mut translator, true, &[slider("mixer_gain_b", 1.0)]),
        [[0xb4, 1, 127]]
    );
    assert!(report(&mut translator, true, &[slider("left_fader_1", 0.5)]).is_empty());
    assert!(report(&mut translator, true, &[slider("right_fx_dial_1", 0.5)]).is_empty());
    assert!(run(&mut translator, &[slider("left_fader_1", 0.5)]).is_empty());
    assert_eq!(
        run(&mut translator, &[slider("left_fader_1", 1.0)]),
        [[0xb0, 48, 127]]
    );

    assert!(report(&mut translator, true, &[button("left_play", true)]).is_empty());
    assert!(run(&mut translator, &[button("left_play", false)]).is_empty());
    assert!(report(&mut translator, true, &[encoder("left_loop_encoder", 3)]).is_empty());
}

#[test]
fn a_shift_held_at_rest_still_selects_the_shift_layer() {
    let mut translator = Translator::new();
    report(&mut translator, true, &[button("left_shift", true)]);

    assert_eq!(
        tap(&mut translator, "left_cue"),
        [[0x98, 1, 127], [0x88, 1, 0]]
    );
}

#[test]
fn a_release_without_a_press_and_a_repeated_press_send_nothing_extra() {
    let mut translator = Translator::new();
    assert!(run(&mut translator, &[button("left_play", false)]).is_empty());

    assert_eq!(
        run(&mut translator, &[button("left_play", true)]),
        [[0x90, 0, 127]]
    );
    assert!(run(&mut translator, &[button("left_play", true)]).is_empty());
    assert_eq!(
        run(&mut translator, &[button("left_play", false)]),
        [[0x80, 0, 0]]
    );
    assert!(run(&mut translator, &[button("left_play", false)]).is_empty());
}

#[test]
fn local_unassigned_and_unknown_controls_send_nothing() {
    let mut translator = Translator::new();
    let names = [
        "left_screen_left_1",
        "left_screen_right_3",
        "left_fx_select",
        "deck_assign_a",
        "mixer_snap",
        "mixer_quantize",
        "mixer_mic_2",
        "mixer_cue_a_latch",
        "mixer_filter_on_a_latch",
        "nonsense",
    ];
    for name in names {
        assert!(tap(&mut translator, name).is_empty(), "{name}");
    }
    assert!(tap(&mut translator, "left_shift").is_empty());
    assert!(run(&mut translator, &[touch("left_fader_touch_1", true)]).is_empty());
    assert!(run(&mut translator, &[touch("left_loop_encoder_touch", true)]).is_empty());
    assert!(run(&mut translator, &[slider("left_unknown", 1.0)]).is_empty());
    assert!(run(&mut translator, &[encoder("left_screen_encoder_1", 4)]).is_empty());
}
