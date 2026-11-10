use super::Translator;
use super::tests::{button, report, run, slider, touch};

#[test]
fn crossfader_assign_sends_one_pulse_per_changed_channel_per_report() {
    let mut translator = Translator::new();

    let resting = report(
        &mut translator,
        true,
        &[
            button("xfader_assign_middle_a", true),
            button("xfader_assign_middle_b", true),
            button("xfader_assign_middle_c", true),
            button("xfader_assign_middle_d", true),
        ],
    );
    assert_eq!(
        resting,
        [
            [0x94, 81, 127],
            [0x84, 81, 0],
            [0x94, 84, 127],
            [0x84, 84, 0],
            [0x94, 87, 127],
            [0x84, 87, 0],
            [0x94, 90, 127],
            [0x84, 90, 0],
        ]
    );

    let to_left = run(
        &mut translator,
        &[
            button("xfader_assign_left_a", true),
            button("xfader_assign_middle_a", false),
        ],
    );
    assert_eq!(to_left, [[0x94, 80, 127], [0x84, 80, 0]]);

    let across = run(
        &mut translator,
        &[
            button("xfader_assign_left_a", false),
            button("xfader_assign_middle_a", true),
            button("xfader_assign_middle_a", false),
        ],
    );
    assert_eq!(across, [[0x94, 82, 127], [0x84, 82, 0]]);

    let unchanged = run(
        &mut translator,
        &[
            button("xfader_assign_middle_a", true),
            button("xfader_assign_middle_a", false),
        ],
    );
    assert!(unchanged.is_empty());
    assert_eq!(run(&mut translator, &[slider("crossfader", 0.0)]).len(), 1);
    assert!(run(&mut translator, &[]).is_empty());
}

#[test]
fn crossfader_assign_changes_in_one_report_each_send_their_own_pulse() {
    let mut translator = Translator::new();
    report(
        &mut translator,
        true,
        &[button("xfader_assign_middle_a", true)],
    );

    let out = run(
        &mut translator,
        &[
            button("xfader_assign_middle_a", false),
            button("xfader_assign_left_c", true),
        ],
    );

    assert_eq!(
        out,
        [
            [0x94, 82, 127],
            [0x84, 82, 0],
            [0x94, 86, 127],
            [0x84, 86, 0]
        ]
    );
}

#[test]
fn filter_on_is_a_bridge_owned_toggle_that_centres_the_knob_while_off() {
    let mut translator = Translator::new();
    assert_eq!(
        run(&mut translator, &[slider("mixer_filter_a", 0.25)]),
        [[0xb4, 21, 32]]
    );

    assert_eq!(
        run(&mut translator, &[button("mixer_filter_on_a", true)]),
        [[0xb4, 21, 64]]
    );
    assert!(run(&mut translator, &[button("mixer_filter_on_a", false)]).is_empty());
    assert!(run(&mut translator, &[slider("mixer_filter_a", 0.75)]).is_empty());
    assert!(run(&mut translator, &[button("mixer_filter_on_a_latch", true)]).is_empty());

    assert_eq!(
        run(&mut translator, &[button("mixer_filter_on_a", true)]),
        [[0xb4, 21, 95]]
    );
    assert!(run(&mut translator, &[button("mixer_filter_on_a", false)]).is_empty());
    assert_eq!(
        run(&mut translator, &[slider("mixer_filter_a", 1.0)]),
        [[0xb4, 21, 127]]
    );
    assert_eq!(
        run(&mut translator, &[slider("mixer_filter_b", 1.0)]),
        [[0xb4, 22, 127]]
    );
}

#[test]
fn strip_touch_and_motion_belong_to_the_deck_and_layer_at_touch_time() {
    let mut translator = Translator::new();
    assert!(run(&mut translator, &[slider("left_touchstrip", 0.5)]).is_empty());

    assert_eq!(
        run(&mut translator, &[touch("left_touchstrip_touch", true)]),
        [[0x90, 7, 127]]
    );
    assert_eq!(
        run(&mut translator, &[button("left_deck", true)]),
        [[0x95, 2, 127]]
    );
    run(&mut translator, &[button("left_shift", true)]);
    assert_eq!(
        run(&mut translator, &[slider("left_touchstrip", 0.5)]),
        [[0xb0, 5, 63]]
    );
    assert_eq!(
        run(&mut translator, &[touch("left_touchstrip_touch", false)]),
        [[0x80, 7, 0]]
    );
    assert!(run(&mut translator, &[slider("left_touchstrip", 0.0)]).is_empty());

    assert_eq!(
        run(&mut translator, &[touch("left_touchstrip_touch", true)]),
        [[0x9a, 7, 127]]
    );
    run(&mut translator, &[button("left_shift", false)]);
    assert_eq!(
        run(&mut translator, &[slider("left_touchstrip", 1.0)]),
        [[0xba, 5, 127]]
    );
    assert_eq!(
        run(&mut translator, &[touch("left_touchstrip_touch", false)]),
        [[0x8a, 7, 0]]
    );
}

#[test]
fn strip_values_follow_the_raw_position_and_repeat_only_on_change() {
    let mut translator = Translator::new();
    run(&mut translator, &[touch("right_touchstrip_touch", true)]);

    let steps = [
        (1.0 / 1024.0, Some(0)),
        (8.0 / 1024.0, None),
        (9.0 / 1024.0, Some(1)),
        (1.0, Some(127)),
        (0.0, None),
    ];
    for (value, expected) in steps {
        let out = run(&mut translator, &[slider("right_touchstrip", value)]);
        let expected: Vec<[u8; 3]> = expected.map(|cc| [0xb1, 5, cc]).into_iter().collect();
        assert_eq!(out, expected, "value {value}");
    }
}
