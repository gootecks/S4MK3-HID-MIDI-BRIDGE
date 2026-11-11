//! Pure mapping from Djay feedback and local controller state to the LEDs of one deck side.

use super::catalog::{self, DECK, FX_LEFT, FX_RIGHT, PadMode};
use super::feedback::FeedbackState;
use super::led_palette::{cue_colour, dimmed};
use super::leds::{self, Dual, Level, MAX_VALUE, Rgb, Side, SideLeds, Strip};
use super::translator::Translator;

#[cfg(test)]
mod feedback_tests;

const CUE_NOTE: u8 = 1;
const SYNC_NOTE: u8 = 2;
const FLUX_NOTE: u8 = 4;
const LOOP_NOTE: u8 = 6;
const FX_INPUTS: [&str; 4] = ["fx_1", "fx_2", "fx_3", "fx_4"];
const SLOT_INPUTS: [&str; 4] = ["on_1", "on_2", "on_3", "on_4"];
/// Decks A and B (0, 1) light the blue LED of a `Dual`, decks C and D the white one.
const FIRST_WHITE_DECK: usize = 2;
const RESTING_DIVISOR: u8 = 6;
const LOOP_COLOUR: Rgb = Rgb::new(0, MAX_VALUE, 0);
const FREEZE_COLOUR: Rgb = Rgb::new(0, 80, MAX_VALUE);
const REMIX_COLOUR: Rgb = Rgb::new(MAX_VALUE, 48, 0);
const HOTCUE_COLOUR: Rgb = Rgb::new(MAX_VALUE, MAX_VALUE, MAX_VALUE);

/// Active and inactive pad colour of a mode that Djay does not colour itself.
const fn mode_colours(mode: PadMode) -> (Rgb, Rgb) {
    let active = match mode {
        PadMode::Hotcue => HOTCUE_COLOUR,
        PadMode::Loop => LOOP_COLOUR,
        PadMode::Freeze => FREEZE_COLOUR,
        PadMode::Remix => REMIX_COLOUR,
    };
    match mode {
        PadMode::Hotcue => (active, Rgb::OFF),
        _ => (active, dimmed(active, RESTING_DIVISOR)),
    }
}

/// The translator state one side's LEDs depend on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Local {
    /// Selected deck, 0-3.
    pub deck: usize,
    pub shifted: bool,
    pub pad_mode: PadMode,
    /// Deck the side's FX unit targets, 0-3.
    pub fx_target: usize,
    pub strip_touch: Option<u8>,
    pub cue_held: bool,
}

impl Local {
    pub fn from_translator(translator: &Translator, side: Side) -> Self {
        let index = side.index();
        Self {
            deck: usize::from(translator.deck(index).saturating_sub(1)),
            shifted: translator.shift_held(index),
            pad_mode: translator.pad_mode(index),
            fx_target: usize::from(translator.fx_target(index).saturating_sub(1)),
            strip_touch: translator.strip_position(index),
            cue_held: translator.cue_held(index),
        }
    }
}

const fn level(on: bool) -> Level {
    if on { Level::On } else { Level::Dim }
}

const fn dual(deck: usize, level: Level) -> Dual {
    if deck >= FIRST_WHITE_DECK {
        Dual {
            white: level,
            blue: Level::Off,
        }
    } else {
        Dual {
            white: Level::Off,
            blue: level,
        }
    }
}

fn pad(mode: PadMode, index: usize, deck: usize, feedback: &FeedbackState) -> Rgb {
    let (active, inactive) = mode_colours(mode);
    let Some(number) = catalog::pad_feedback(mode, index) else {
        return inactive;
    };
    match mode {
        PadMode::Hotcue => feedback
            .note(deck, number)
            .and_then(cue_colour)
            .unwrap_or(Rgb::OFF),
        PadMode::Remix if feedback.note_on(deck, number) => active,
        _ => inactive,
    }
}

fn fx_level(side: Side, index: usize, local: &Local, feedback: &FeedbackState) -> Level {
    let table = match side {
        Side::Left => FX_LEFT,
        Side::Right => FX_RIGHT,
    };
    let number = FX_INPUTS
        .get(index)
        .and_then(|input| catalog::feedback_number(table, input));
    number.map_or(Level::Off, |number| {
        level(feedback.note_on(local.fx_target, number))
    })
}

fn slot_level(index: usize, deck: usize, feedback: &FeedbackState) -> Level {
    SLOT_INPUTS
        .get(index)
        .and_then(|input| catalog::feedback_number(DECK, input))
        .map_or(Level::Off, |number| level(feedback.note_on(deck, number)))
}

/// Both banks light the same physical row, so the bar skips the LED under the finger dot.
fn strip(deck: usize, local: &Local, feedback: &FeedbackState) -> Strip {
    let mut strip = Strip::default();
    if let Some(progress) = feedback.progress(deck) {
        strip.blue = leds::bar(progress, MAX_VALUE);
    }
    if let Some(position) = local.strip_touch {
        strip.orange = leds::dot(position, MAX_VALUE);
        for (blue, orange) in strip.blue.iter_mut().zip(strip.orange) {
            if orange != 0 {
                *blue = 0;
            }
        }
    }
    strip
}

/// The LEDs of `side` for the deck it currently controls.
pub fn present(side: Side, feedback: &FeedbackState, local: &Local) -> SideLeds {
    let deck = local.deck;
    let mode_button = |mode: PadMode| {
        dual(
            deck,
            if local.pad_mode == mode {
                Level::On
            } else {
                Level::Dim
            },
        )
    };
    let on = |number: u8| feedback.note_on(deck, number);
    let ring = dual(deck, if on(LOOP_NOTE) { Level::On } else { Level::Dim });
    SideLeds {
        pads: std::array::from_fn(|index| pad(local.pad_mode, index, deck, feedback)),
        fx_select: Level::Dim,
        fx: std::array::from_fn(|index| fx_level(side, index, local, feedback)),
        display: [Level::Off; 8],
        back: Level::Dim,
        capture: Level::Dim,
        edit: Level::Dim,
        slots: std::array::from_fn(|index| slot_level(index, deck, feedback)),
        hotcue: mode_button(PadMode::Hotcue),
        loop_mode: mode_button(PadMode::Loop),
        freeze: mode_button(PadMode::Freeze),
        remix: mode_button(PadMode::Remix),
        flux: if on(FLUX_NOTE) { Level::On } else { Level::Off },
        deck: dual(deck, Level::On),
        shift: level(local.shifted),
        sync: level(on(SYNC_NOTE)),
        cue: if on(CUE_NOTE) || local.cue_held {
            Level::On
        } else {
            Level::Off
        },
        play: if feedback.is_playing(deck) {
            Level::On
        } else {
            Level::Off
        },
        loop_ring: [ring; 4],
        strip: strip(deck, local, feedback),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::full::feedback::Message;
    use crate::s8::catalog::{ROOT_OUTPUTS, RootFamily};

    fn local(deck: usize) -> Local {
        Local {
            deck,
            shifted: false,
            pad_mode: PadMode::Hotcue,
            fx_target: deck,
            strip_touch: None,
            cue_held: false,
        }
    }

    fn feed(state: &mut FeedbackState, status: u8, number: u8, value: u8) {
        state.apply(
            Message {
                status,
                number,
                value,
            },
            Duration::ZERO,
        );
    }

    fn root_number(family: RootFamily, deck: u8) -> Result<u8, String> {
        ROOT_OUTPUTS
            .iter()
            .find(|root| root.family == family)
            .map(|root| root.address(deck).2)
            .ok_or_else(|| String::from("root output"))
    }

    const BLUE: Dual = Dual {
        white: Level::Off,
        blue: Level::On,
    };
    const WHITE: Dual = Dual {
        white: Level::On,
        blue: Level::Off,
    };

    #[test]
    fn deck_shows_the_selected_deck_colour_and_switching_swaps_blue_for_white() {
        let state = FeedbackState::default();
        assert_eq!(present(Side::Left, &state, &local(0)).deck, BLUE);
        assert_eq!(present(Side::Left, &state, &local(1)).deck, BLUE);
        assert_eq!(present(Side::Left, &state, &local(2)).deck, WHITE);
        assert_eq!(present(Side::Right, &state, &local(3)).deck, WHITE);
    }

    #[test]
    fn pad_mode_buttons_light_the_active_mode_and_dim_the_others_in_the_deck_colour() {
        let state = FeedbackState::default();
        for (mode, deck) in [(PadMode::Loop, 0), (PadMode::Remix, 2)] {
            let leds = present(
                Side::Left,
                &state,
                &Local {
                    pad_mode: mode,
                    ..local(deck)
                },
            );
            let expected =
                |this: PadMode| dual(deck, if this == mode { Level::On } else { Level::Dim });
            assert_eq!(leds.hotcue, expected(PadMode::Hotcue));
            assert_eq!(leds.loop_mode, expected(PadMode::Loop));
            assert_eq!(leds.freeze, expected(PadMode::Freeze));
            assert_eq!(leds.remix, expected(PadMode::Remix));
        }
        let leds = present(
            Side::Left,
            &state,
            &Local {
                pad_mode: PadMode::Freeze,
                ..local(2)
            },
        );
        assert_eq!(leds.freeze, WHITE);
        assert_eq!(leds.hotcue.white, Level::Dim);
        assert_eq!(leds.hotcue.blue, Level::Off);
    }

    #[test]
    fn shift_is_on_while_held_and_survives_a_deck_change() {
        let state = FeedbackState::default();
        assert_eq!(present(Side::Left, &state, &local(0)).shift, Level::Dim);
        let held = Local {
            shifted: true,
            ..local(0)
        };
        assert_eq!(present(Side::Left, &state, &held).shift, Level::On);
        let switched = Local { deck: 2, ..held };
        let leds = present(Side::Left, &state, &switched);
        assert_eq!(leds.shift, Level::On);
        assert_eq!(leds.deck, WHITE);
    }

    #[test]
    fn play_follows_the_selected_decks_root_output() -> Result<(), String> {
        let mut state = FeedbackState::default();
        let number = root_number(RootFamily::Playing, 3)?;
        feed(&mut state, 0xb6, number, 127);
        assert_eq!(present(Side::Left, &state, &local(2)).play, Level::On);
        assert_eq!(present(Side::Left, &state, &local(0)).play, Level::Off);
        Ok(())
    }

    #[test]
    fn cue_follows_djay_or_the_local_hold() {
        let mut state = FeedbackState::default();
        assert_eq!(present(Side::Left, &state, &local(0)).cue, Level::Off);
        let held = Local {
            cue_held: true,
            ..local(0)
        };
        assert_eq!(present(Side::Left, &state, &held).cue, Level::On);
        feed(&mut state, 0x91, CUE_NOTE, 127);
        assert_eq!(present(Side::Right, &state, &local(1)).cue, Level::On);
        assert_eq!(present(Side::Right, &state, &local(0)).cue, Level::Off);
    }

    #[test]
    fn sync_is_on_from_djay_else_dim_and_flux_on_else_off() {
        let mut state = FeedbackState::default();
        let leds = present(Side::Left, &state, &local(0));
        assert_eq!((leds.sync, leds.flux), (Level::Dim, Level::Off));
        feed(&mut state, 0x90, SYNC_NOTE, 127);
        feed(&mut state, 0x90, FLUX_NOTE, 127);
        let leds = present(Side::Left, &state, &local(0));
        assert_eq!((leds.sync, leds.flux), (Level::On, Level::On));
        let other = present(Side::Left, &state, &local(1));
        assert_eq!((other.sync, other.flux), (Level::Dim, Level::Off));
    }

    #[test]
    fn loop_ring_is_on_in_the_deck_colour_while_a_loop_is_active_else_dim() {
        let mut state = FeedbackState::default();
        let dim = present(Side::Left, &state, &local(2)).loop_ring;
        assert!(dim.iter().all(|segment| *segment == dual(2, Level::Dim)));
        feed(&mut state, 0x92, LOOP_NOTE, 127);
        let on = present(Side::Left, &state, &local(2)).loop_ring;
        assert!(on.iter().all(|segment| *segment == WHITE));
    }

    #[test]
    fn hotcue_pads_show_the_cue_colour_and_are_off_when_empty() {
        let mut state = FeedbackState::default();
        feed(&mut state, 0x90, 40, 1);
        feed(&mut state, 0x90, 43, 3);
        feed(&mut state, 0x90, 42, 0);
        let leds = present(Side::Left, &state, &local(0));
        assert_eq!(Some(leds.pads[0]), cue_colour(1));
        assert_eq!(Some(leds.pads[3]), cue_colour(3));
        assert_eq!(leds.pads[1], Rgb::OFF);
        assert_eq!(leds.pads[2], Rgb::OFF);
        let other = present(Side::Left, &state, &local(1));
        assert!(other.pads.iter().all(|pad| *pad == Rgb::OFF));
    }

    #[test]
    fn remix_pads_use_the_active_or_inactive_colour_of_the_sampler_players() {
        let mut state = FeedbackState::default();
        feed(&mut state, 0x90, 50, 127);
        let local = Local {
            pad_mode: PadMode::Remix,
            ..local(0)
        };
        let leds = present(Side::Left, &state, &local);
        let (active, inactive) = mode_colours(PadMode::Remix);
        assert_eq!(leds.pads[2], active);
        assert_eq!(leds.pads[0], inactive);
        assert_ne!(active, inactive);
    }

    #[test]
    fn loop_and_freeze_pads_show_their_inactive_colour() {
        let state = FeedbackState::default();
        for mode in [PadMode::Loop, PadMode::Freeze] {
            let leds = present(
                Side::Left,
                &state,
                &Local {
                    pad_mode: mode,
                    ..local(0)
                },
            );
            assert!(leds.pads.iter().all(|pad| *pad == mode_colours(mode).1));
        }
    }

    #[test]
    fn fx_buttons_follow_the_unit_of_the_side_on_the_fx_target_deck() {
        let mut state = FeedbackState::default();
        feed(&mut state, 0x91, 32, 127);
        feed(&mut state, 0x91, 39, 127);
        let local = Local {
            fx_target: 1,
            ..local(0)
        };
        let left = present(Side::Left, &state, &local);
        assert_eq!(left.fx, [Level::On, Level::Dim, Level::Dim, Level::Dim]);
        let right = present(Side::Right, &state, &local);
        assert_eq!(right.fx, [Level::Dim, Level::Dim, Level::Dim, Level::On]);
        let elsewhere = present(
            Side::Left,
            &state,
            &Local {
                fx_target: 0,
                ..local
            },
        );
        assert_eq!(elsewhere.fx, [Level::Dim; 4]);
    }

    #[test]
    fn slots_follow_the_stem_mute_feedback_of_the_selected_deck() {
        let mut state = FeedbackState::default();
        feed(&mut state, 0x92, 57, 127);
        let leds = present(Side::Left, &state, &local(2));
        assert_eq!(leds.slots, [Level::Dim, Level::On, Level::Dim, Level::Dim]);
        let other = present(Side::Left, &state, &local(0));
        assert_eq!(other.slots, [Level::Dim; 4]);
    }

    #[test]
    fn bound_buttons_without_feedback_are_dim_and_display_buttons_are_off() {
        let leds = present(Side::Left, &FeedbackState::default(), &local(0));
        assert_eq!(leds.fx_select, Level::Dim);
        assert_eq!(leds.back, Level::Dim);
        assert_eq!(leds.capture, Level::Dim);
        assert_eq!(leds.edit, Level::Dim);
        assert_eq!(leds.display, [Level::Off; 8]);
    }

    #[test]
    fn strip_shows_the_touch_dot_in_orange_and_progress_in_blue() -> Result<(), String> {
        let mut state = FeedbackState::default();
        let dark = present(Side::Left, &state, &local(0)).strip;
        assert_eq!(dark, Strip::default());
        feed(&mut state, 0xb6, root_number(RootFamily::Progress, 1)?, 127);
        let touched = Local {
            strip_touch: Some(0),
            ..local(0)
        };
        let strip = present(Side::Left, &state, &touched).strip;
        assert_eq!(strip.orange, leds::dot(0, MAX_VALUE));
        let mut bar = leds::bar(127, MAX_VALUE);
        bar[0] = 0;
        assert_eq!(strip.blue, bar);
        let other_deck = present(Side::Left, &state, &local(1)).strip;
        assert_eq!(other_deck, Strip::default());
        Ok(())
    }

    #[test]
    fn the_progress_bar_skips_the_led_under_the_finger_and_returns_on_release() -> Result<(), String>
    {
        let mut state = FeedbackState::default();
        feed(&mut state, 0xb6, root_number(RootFamily::Progress, 1)?, 127);
        let bar = leds::bar(127, MAX_VALUE);
        for position in [0, 64, 127] {
            let held = Local {
                strip_touch: Some(position),
                ..local(0)
            };
            let strip = present(Side::Left, &state, &held).strip;
            let dot = leds::dot(position, MAX_VALUE);
            for index in 0..strip.blue.len() {
                assert_eq!(
                    (strip.blue[index] != 0, strip.orange[index] != 0),
                    (bar[index] != 0 && dot[index] == 0, dot[index] != 0),
                    "position {position}, led {index}"
                );
            }
        }
        let released = present(Side::Left, &state, &local(0)).strip;
        assert_eq!((released.blue, released.orange), (bar, [0; 25]));
        Ok(())
    }

    #[test]
    fn local_snapshot_starts_on_decks_a_and_b() {
        let translator = Translator::new();
        let left = Local::from_translator(&translator, Side::Left);
        let right = Local::from_translator(&translator, Side::Right);
        assert_eq!((left.deck, right.deck), (0, 1));
        assert!(!left.shifted && !left.cue_held && left.strip_touch.is_none());
        assert_eq!(left.pad_mode, PadMode::Hotcue);
        assert_eq!((left.fx_target, right.fx_target), (0, 1));
    }
}
