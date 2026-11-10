//! `S8_CONTROL` lines for the companion app (`--app-events`).
//!
//! The display buttons and screen encoders have no MIDI: the translator
//! ignores them and the app acts on them instead. Buttons 1 and 4 belong to
//! Native Instruments and are never reported. Line format:
//! `docs/s8/display-buttons.md`.

use encdr::Event;
use s4mk3_hid_midi_bridge::device::s8::ScreenEncoderTurn;

/// Line for a display-button event, or `None` for every other event, for
/// buttons 1 and 4, and for the resting state in the first report.
#[must_use]
pub fn display_button(event: &Event, resting: bool) -> Option<String> {
    let Event::Button { name, pressed, .. } = *event else {
        return None;
    };
    if resting {
        return None;
    }
    let (side, number) = button_name(name)?;
    Some(format!(
        "S8_CONTROL display_button {side} {number} {}",
        if pressed { "down" } else { "up" }
    ))
}

/// Line for a screen-encoder turn; the delta keeps its sign.
#[must_use]
pub fn screen_encoder(turn: &ScreenEncoderTurn) -> Option<String> {
    let rest = turn.name.strip_suffix(|c: char| c.is_ascii_digit())?;
    let number = &turn.name[rest.len()..];
    let side = side_of(rest.strip_suffix("_screen_encoder_")?)?;
    matches!(number, "1" | "2" | "3" | "4")
        .then(|| format!("S8_CONTROL screen_encoder {side} {number} {}", turn.delta))
}

/// `left_screen_left_2` is button 2 and `right_screen_right_3` is button 7.
fn button_name(name: &str) -> Option<(&'static str, u8)> {
    let (side, rest) = name.split_once("_screen_")?;
    let side = side_of(side)?;
    let (half, index) = rest.split_once('_')?;
    let offset = match half {
        "left" => 0,
        "right" => 4,
        _ => return None,
    };
    let number = offset
        + match index {
            "1" => 1,
            "2" => 2,
            "3" => 3,
            "4" => 4,
            _ => return None,
        };
    (!matches!(number, 1 | 4)).then_some((side, number))
}

fn side_of(side: &str) -> Option<&'static str> {
    match side {
        "left" => Some("left"),
        "right" => Some("right"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encdr::DeviceId;

    fn button(name: &'static str, pressed: bool) -> Event {
        Event::Button {
            device: DeviceId::default(),
            name,
            pressed,
        }
    }

    fn turn(name: &'static str, delta: i32) -> ScreenEncoderTurn {
        ScreenEncoderTurn {
            name,
            delta,
            position: 0,
        }
    }

    #[test]
    fn every_usable_button_on_both_sides_maps_to_its_number() {
        // Given the six usable buttons per side.
        let names = [
            ("left", 2, "left_screen_left_2"),
            ("left", 3, "left_screen_left_3"),
            ("left", 5, "left_screen_right_1"),
            ("left", 6, "left_screen_right_2"),
            ("left", 7, "left_screen_right_3"),
            ("left", 8, "left_screen_right_4"),
            ("right", 2, "right_screen_left_2"),
            ("right", 3, "right_screen_left_3"),
            ("right", 5, "right_screen_right_1"),
            ("right", 6, "right_screen_right_2"),
            ("right", 7, "right_screen_right_3"),
            ("right", 8, "right_screen_right_4"),
        ];
        for (side, number, name) in names {
            // When pressed and released outside the resting report.
            let down = display_button(&button(name, true), false);
            let up = display_button(&button(name, false), false);
            // Then each reports its side, number and direction.
            assert_eq!(
                down.as_deref(),
                Some(format!("S8_CONTROL display_button {side} {number} down").as_str())
            );
            assert_eq!(
                up.as_deref(),
                Some(format!("S8_CONTROL display_button {side} {number} up").as_str())
            );
        }
    }

    #[test]
    fn native_instruments_buttons_and_other_controls_give_nothing() {
        for name in [
            "left_screen_left_1",
            "left_screen_left_4",
            "right_screen_left_1",
            "right_screen_left_4",
            "left_screen_left_5",
            "left_screen_middle_2",
            "left_screen_encoder_1",
            "deck_assign_a",
            "left_play",
            "screen_left_2",
            "center_screen_left_2",
        ] {
            assert_eq!(display_button(&button(name, true), false), None, "{name}");
        }
        let touch = Event::Touch {
            device: DeviceId::default(),
            name: "left_screen_left_2",
            touched: true,
        };
        assert_eq!(display_button(&touch, false), None);
    }

    #[test]
    fn the_resting_report_gives_nothing() {
        assert_eq!(
            display_button(&button("left_screen_left_2", true), true),
            None
        );
    }

    #[test]
    fn encoder_turns_name_side_and_number_and_keep_their_sign() {
        assert_eq!(
            screen_encoder(&turn("left_screen_encoder_1", 5)).as_deref(),
            Some("S8_CONTROL screen_encoder left 1 5")
        );
        assert_eq!(
            screen_encoder(&turn("right_screen_encoder_4", -20)).as_deref(),
            Some("S8_CONTROL screen_encoder right 4 -20")
        );
        assert_eq!(screen_encoder(&turn("left_screen_encoder_5", 1)), None);
        assert_eq!(screen_encoder(&turn("mixer_gain_a", 1)), None);
    }

    #[test]
    fn deltas_across_the_wrap_keep_their_sign() {
        use s4mk3_hid_midi_bridge::device::s8::ScreenEncoders;

        // Given encoders resting at 998 and 2.
        let mut encoders = ScreenEncoders::default();
        let mut report = |a: u16, b: u16| {
            let mut report = vec![2];
            for position in [a, b, 0, 0, 0, 0, 0, 0] {
                report.extend(position.to_le_bytes());
            }
            report.resize(109, 0);
            encoders.update(&report).collect::<Vec<_>>()
        };
        assert!(report(998, 2).is_empty());

        // When one passes 999 to 0 clockwise and the other 0 to 999 anticlockwise.
        let lines: Vec<_> = report(3, 997).iter().filter_map(screen_encoder).collect();

        // Then the clockwise delta is positive and the other negative.
        assert_eq!(
            lines,
            [
                "S8_CONTROL screen_encoder left 1 5",
                "S8_CONTROL screen_encoder left 2 -5"
            ]
        );
    }
}
