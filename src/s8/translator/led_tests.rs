use super::Translator;
use super::tests::{button, run, slider, tap, touch};
use crate::s8::feedback::FeedbackState;
use crate::s8::led_state::{Local, present};
use crate::s8::leds::{self, MAX_VALUE, Side, Strip};

fn left_cue_held(translator: &Translator) -> bool {
    Local::from_translator(translator, Side::Left).cue_held
}

#[test]
fn plain_cue_counts_while_held_and_not_after_release() {
    let mut translator = Translator::new();
    assert!(!left_cue_held(&translator));

    run(&mut translator, &[button("left_cue", true)]);
    assert!(left_cue_held(&translator));
    assert!(!translator.cue_held(1));

    run(&mut translator, &[button("left_cue", false)]);
    assert!(!left_cue_held(&translator));
}

#[test]
fn a_cue_hold_does_not_follow_the_side_to_another_deck() {
    let mut translator = Translator::new();
    run(&mut translator, &[button("left_cue", true)]);
    tap(&mut translator, "left_deck");
    assert_eq!(translator.deck(0), 3);
    assert!(!left_cue_held(&translator));

    tap(&mut translator, "left_deck");
    assert_eq!(translator.deck(0), 1);
    assert!(left_cue_held(&translator));

    run(&mut translator, &[button("left_cue", false)]);
    assert!(!left_cue_held(&translator));
}

#[test]
fn shift_cue_goes_to_zero_and_does_not_count_as_a_cue_hold() {
    let mut translator = Translator::new();
    run(&mut translator, &[button("left_shift", true)]);
    let out = run(&mut translator, &[button("left_cue", true)]);
    assert_eq!(out, [[0x98, 1, 127]]);
    assert!(!left_cue_held(&translator));

    run(&mut translator, &[button("left_cue", false)]);
    assert!(!left_cue_held(&translator));
}

#[test]
fn strip_touch_and_release_reach_the_local_state_and_the_orange_strip() {
    let mut translator = Translator::new();
    let state = FeedbackState::default();
    let local = |translator: &Translator| Local::from_translator(translator, Side::Left);

    run(&mut translator, &[touch("left_touchstrip_touch", true)]);
    assert_eq!(local(&translator).strip_touch, None);

    run(&mut translator, &[slider("left_touchstrip", 0.5)]);
    let touched = local(&translator);
    assert!(
        touched.strip_touch.is_some(),
        "a moving touch has a position"
    );
    let position = touched.strip_touch.unwrap_or_default();
    assert_eq!(
        present(Side::Left, &state, &touched).strip.orange,
        leds::dot(position, MAX_VALUE)
    );
    assert_eq!(
        Local::from_translator(&translator, Side::Right).strip_touch,
        None
    );

    run(&mut translator, &[touch("left_touchstrip_touch", false)]);
    let released = local(&translator);
    assert_eq!(released.strip_touch, None);
    assert_eq!(
        present(Side::Left, &state, &released).strip,
        Strip::default()
    );
}
