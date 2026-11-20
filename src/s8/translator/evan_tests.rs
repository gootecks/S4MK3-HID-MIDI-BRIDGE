use super::tests::{button, run, slider, tap};
use super::{Layout, Translator};

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
