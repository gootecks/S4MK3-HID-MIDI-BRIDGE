use std::time::Duration;

use encdr::{DeviceId, Event};

use super::*;
use crate::full::feedback::Message;
use crate::s8::catalog::{
    Binding, CUE_COLOUR_NOTES, DECKS, NOTE_TYPE, ROOT_OUTPUTS, RootFamily, feedback_number,
    has_embedded_output, pad_feedback, pads,
};

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;
const CONTROLLER: u8 = 0xb0;

fn local(deck: usize, pad_mode: PadMode) -> Local {
    Local {
        deck,
        shifted: false,
        pad_mode,
        fx_target: deck,
        strip_touch: None,
        cue_held: false,
    }
}

fn apply(state: &mut FeedbackState, status: u8, number: u8, value: u8) {
    state.apply(
        Message {
            status,
            number,
            value,
        },
        Duration::ZERO,
    );
}

fn root_apply(
    state: &mut FeedbackState,
    family: RootFamily,
    deck: u8,
    value: u8,
) -> Result<(), String> {
    let root = ROOT_OUTPUTS
        .iter()
        .find(|root| root.family == family)
        .ok_or("root output")?;
    let (channel, kind, number) = root.address(deck);
    let status = channel
        | if kind == NOTE_TYPE {
            NOTE_ON
        } else {
            CONTROLLER
        };
    apply(state, status, number, value);
    Ok(())
}

fn note_number(table: &'static [Binding], input: &str) -> Result<u8, String> {
    feedback_number(table, input).ok_or_else(|| format!("no feedback number for {input}"))
}

/// What the LED named by a catalog input shows, or `None` when no LED follows it.
fn shows(input: &str, leds: &SideLeds, local: &Local) -> Option<bool> {
    let slot = |prefix: &str| {
        input
            .strip_prefix(prefix)?
            .parse::<usize>()
            .ok()?
            .checked_sub(1)
    };
    match input {
        "cue" => return Some(leds.cue == Level::On),
        "sync" => return Some(leds.sync == Level::On),
        "flux" => return Some(leds.flux == Level::On),
        "loop_encoder_press" => {
            return Some(leds.loop_ring == [dual(local.deck, Level::On); 4]);
        }
        _ => {}
    }
    if let Some(level) = slot("on_").and_then(|index| leds.slots.get(index)) {
        return Some(*level == Level::On);
    }
    if let Some(level) = slot("fx_").and_then(|index| leds.fx.get(index)) {
        return Some(*level == Level::On);
    }
    let pad = slot("pad_").and_then(|index| leds.pads.get(index))?;
    match local.pad_mode {
        PadMode::Hotcue => Some(Some(*pad) == cue_colour(1)),
        PadMode::Remix => Some(*pad == mode_colours(PadMode::Remix).0),
        PadMode::Loop | PadMode::Freeze => None,
    }
}

type Group = (&'static [Binding], &'static [Side], PadMode);

fn groups() -> Vec<Group> {
    let mut groups: Vec<Group> = vec![
        (DECK, &Side::ALL, PadMode::Hotcue),
        (FX_LEFT, &[Side::Left], PadMode::Hotcue),
        (FX_RIGHT, &[Side::Right], PadMode::Hotcue),
    ];
    groups.extend(PadMode::ALL.map(|mode| (pads(mode), &Side::ALL as &[Side], mode)));
    groups
}

#[test]
fn every_embedded_output_of_the_catalog_lights_the_led_its_row_names() {
    let (mut checked, mut lit) = (0, 0);
    for (table, sides, mode) in groups() {
        for binding in table.iter().filter(|binding| has_embedded_output(binding)) {
            for deck in DECKS {
                let here = local(usize::from(deck - 1), mode);
                let value = if CUE_COLOUR_NOTES.contains(&binding.number) {
                    1
                } else {
                    127
                };
                let mut state = FeedbackState::default();
                apply(
                    &mut state,
                    NOTE_ON | binding.channel(deck),
                    binding.number,
                    value,
                );
                for &side in sides {
                    let label = format!("{} deck {deck} {side:?}", binding.target);
                    let before = present(side, &FeedbackState::default(), &here);
                    let after = present(side, &state, &here);
                    checked += 1;
                    if let Some(was) = shows(binding.input, &before, &here) {
                        assert!(!was, "{label}: lit before any feedback");
                        assert_eq!(shows(binding.input, &after, &here), Some(true), "{label}");
                        lit += 1;
                    } else {
                        assert_eq!(after, before, "{label}: no LED follows this output");
                    }
                }
                if let [side] = sides {
                    let other = if *side == Side::Left {
                        Side::Right
                    } else {
                        Side::Left
                    };
                    let untouched = present(other, &FeedbackState::default(), &here);
                    assert_eq!(
                        present(other, &state, &here),
                        untouched,
                        "{}",
                        binding.target
                    );
                }
            }
        }
    }
    assert!(checked > 0 && lit > 0);
}

#[test]
fn every_root_output_of_the_catalog_reaches_its_led_on_its_own_deck_only() -> Result<(), String> {
    for root in ROOT_OUTPUTS {
        for deck in DECKS {
            let mut state = FeedbackState::default();
            root_apply(&mut state, root.family, deck, 127)?;
            for side in Side::ALL {
                for shown in 0..4 {
                    let here = local(shown, PadMode::Hotcue);
                    let before = present(side, &FeedbackState::default(), &here);
                    let after = present(side, &state, &here);
                    let label = format!("{} deck {deck} shown {shown}", root.target);
                    if shown != usize::from(deck - 1) {
                        assert_eq!(after, before, "{label}");
                        continue;
                    }
                    match root.family {
                        RootFamily::Playing => {
                            assert_eq!(
                                (before.play, after.play),
                                (Level::Off, Level::On),
                                "{label}"
                            );
                        }
                        RootFamily::Progress => {
                            assert_eq!(after.strip.blue, leds::bar(127, MAX_VALUE), "{label}");
                            assert_eq!(before.strip.blue, Strip::default().blue, "{label}");
                        }
                        RootFamily::Loaded => assert_eq!(after, before, "{label}"),
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn sampler_pads_of_decks_b_and_d_read_the_mirrored_bank() -> Result<(), String> {
    let number = pad_feedback(PadMode::Remix, 2).ok_or("sampler pad 3")?;
    let (active, inactive) = mode_colours(PadMode::Remix);
    for source in 0..4_u8 {
        let mut state = FeedbackState::default();
        apply(&mut state, NOTE_ON | source, number, 127);
        for deck in 0..4_u8 {
            let leds = present(
                Side::Left,
                &state,
                &local(usize::from(deck), PadMode::Remix),
            );
            let mirrored = deck == source || deck == source ^ 2;
            let want = if mirrored { active } else { inactive };
            assert_eq!(leds.pads[2], want, "fed deck {source}, shown deck {deck}");
            assert!(
                leds.pads
                    .iter()
                    .enumerate()
                    .all(|(index, pad)| { index == 2 || *pad == inactive })
            );
        }
        apply(&mut state, NOTE_OFF | source, number, 0);
        for deck in 0..4 {
            let leds = present(Side::Left, &state, &local(deck, PadMode::Remix));
            assert_eq!(leds.pads[2], inactive);
        }
    }
    Ok(())
}

fn feed_deck_a(state: &mut FeedbackState) -> Result<(), String> {
    apply(state, NOTE_ON, note_number(DECK, "cue")?, 127);
    apply(state, NOTE_ON, note_number(DECK, "flux")?, 127);
    apply(
        state,
        NOTE_ON,
        note_number(DECK, "loop_encoder_press")?,
        127,
    );
    apply(state, NOTE_ON, note_number(DECK, "on_1")?, 127);
    apply(
        state,
        NOTE_ON,
        pad_feedback(PadMode::Hotcue, 0).ok_or("hotcue pad")?,
        1,
    );
    root_apply(state, RootFamily::Playing, 1, 127)?;
    root_apply(state, RootFamily::Progress, 1, 127)
}

fn feed_deck_c(state: &mut FeedbackState) -> Result<(), String> {
    apply(state, NOTE_ON | 2, note_number(DECK, "sync")?, 127);
    apply(state, NOTE_ON | 2, note_number(DECK, "on_2")?, 127);
    Ok(())
}

fn press_deck(translator: &mut Translator) {
    let mut out = Vec::new();
    for pressed in [true, false] {
        let event = Event::Button {
            device: DeviceId::default(),
            name: "left_deck",
            pressed,
        };
        translator.handle(&event, false, &mut out);
    }
}

#[test]
fn switching_the_deck_of_a_side_swaps_every_deck_scoped_led_at_once() -> Result<(), String> {
    let (mut only_a, mut only_c, mut both) = Default::default();
    feed_deck_a(&mut only_a)?;
    feed_deck_c(&mut only_c)?;
    feed_deck_a(&mut both)?;
    feed_deck_c(&mut both)?;
    let mut translator = Translator::new();
    let shown = |translator: &Translator| {
        present(
            Side::Left,
            &both,
            &Local::from_translator(translator, Side::Left),
        )
    };

    let on_a = shown(&translator);
    assert_eq!(
        on_a,
        present(Side::Left, &only_a, &local(0, PadMode::Hotcue))
    );
    press_deck(&mut translator);
    let on_c = shown(&translator);
    assert_eq!(
        on_c,
        present(Side::Left, &only_c, &local(2, PadMode::Hotcue))
    );

    assert_ne!(on_a.pads, on_c.pads);
    assert_ne!(on_a.slots, on_c.slots);
    assert_ne!(on_a.flux, on_c.flux);
    assert_ne!(on_a.deck, on_c.deck);
    assert_ne!(on_a.sync, on_c.sync);
    assert_ne!(on_a.cue, on_c.cue);
    assert_ne!(on_a.play, on_c.play);
    assert_ne!(on_a.loop_ring, on_c.loop_ring);
    assert_ne!(on_a.strip, on_c.strip);

    press_deck(&mut translator);
    assert_eq!(shown(&translator), on_a);
    Ok(())
}
