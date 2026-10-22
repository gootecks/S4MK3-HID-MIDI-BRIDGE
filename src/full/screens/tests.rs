use super::*;
use crate::full::{Control, Deck, feedback::Decoder};
use std::time::Duration;

fn feed(feedback: &mut FeedbackState, bytes: &[u8]) {
    Decoder::default().feed(bytes, |message| {
        feedback.apply(message, Duration::ZERO);
    });
}

fn pads(
    translator: &Translator,
    feedback: &FeedbackState,
    side: usize,
) -> anyhow::Result<[PadState; 8]> {
    match pages(translator, feedback)[side] {
        Page::Stems { pads, .. } => Ok(pads),
        Page::Empty | Page::Info(_) => anyhow::bail!("expected STEMS"),
    }
}

#[test]
fn construction_never_opens_screen_output_before_hid_initialization() {
    // Given either screen setting at bridge construction time.
    for enabled in [false, true] {
        // When the output lifecycle is constructed, before any initialized input.
        let output = Output::new(enabled);
        // Then no worker can yet claim USB or send a frame.
        assert!(output.worker.is_none());
    }
}

#[test]
fn disabled_screen_output_stays_off_when_input_arrives() {
    // Given an ordinary full-profile bridge without the screen opt-in.
    let mut output = Output::new(false);
    // When initialized input publishes its current local state.
    output.publish([Page::Empty; 2]);
    // Then it still does not acquire a display interface.
    assert!(output.worker.is_none());
}

#[test]
fn feedback_before_initialized_input_never_starts_usb() {
    // Given opted-in screen output before the first initialized input.
    let mut output = Output::new(true);
    // When Djay feedback refreshes the current page.
    output.refresh([Page::Empty; 2]);
    // Then screen startup still waits for initialized HID input.
    assert!(output.worker.is_none());
}

#[test]
fn feedback_alone_updates_the_selected_deck_without_changing_the_other_side() -> anyhow::Result<()>
{
    for (side, deck) in [(0, Deck::A), (1, Deck::B), (0, Deck::C), (1, Deck::D)] {
        // Given a selected deck with no physical pad event or software snapshot.
        let mut translator = Translator {
            pads: [PadMode::Stems; 2],
            ..Translator::default()
        };
        translator.decks[side] = deck;
        let mut feedback = FeedbackState::default();
        let before = pages(&translator, &feedback);
        // When real note feedback arrives, including running status and realtime.
        feed(
            &mut feedback,
            &[0x90 | deck.channel(), 56, 127, 60, 0xf8, 0],
        );
        // Then only that deck's page changes, using confirmed mute/solo values.
        let after = pages(&translator, &feedback);
        assert_ne!(after[side], before[side]);
        assert_eq!(after[side ^ 1], before[side ^ 1]);
        let visible = pads(&translator, &feedback, side)?;
        assert_eq!(visible[0].active, Some(true));
        assert_eq!(visible[4].active, Some(false));
        assert!(!visible[0].held);
        assert_eq!(visible[1].active, None);
    }
    Ok(())
}

#[test]
fn a_physical_press_does_not_invent_confirmed_activation() -> anyhow::Result<()> {
    // Given STEMS with no software feedback.
    let mut translator = Translator {
        pads: [PadMode::Stems; 2],
        ..Translator::default()
    };
    let feedback = FeedbackState::default();
    let mut sent = Vec::new();
    // When the real translator receives a physical mute press.
    translator.translate(&[Control::Button("left_pad_1", true)], None, &mut sent)?;
    // Then the press is visible but the mute state remains unknown.
    assert_eq!(sent, [[0x90, 56, 127]]);
    assert_eq!(
        pads(&translator, &feedback, 0)?[0],
        PadState {
            active: None,
            held: true
        }
    );
    Ok(())
}

#[test]
fn a_held_press_is_not_relabelled_as_a_new_deck_or_shift_action() -> anyhow::Result<()> {
    for change in ["left_deck_switch_c", "left_shift"] {
        // Given a mute press owned by deck A's normal channel.
        let mut translator = Translator {
            pads: [PadMode::Stems; 2],
            ..Translator::default()
        };
        translator.translate(
            &[Control::Button("left_pad_1", true)],
            None,
            &mut Vec::new(),
        )?;
        // When the visible deck or action layer changes while it is held.
        translator.translate(&[Control::Button(change, true)], None, &mut Vec::new())?;
        // Then the new visible action is not marked held; the old note keeps ownership.
        assert!(!pads(&translator, &FeedbackState::default(), 0)?[0].held);
        assert_eq!(
            translator.held["left_pad_1"],
            [Address {
                channel: 0,
                number: 56
            }]
        );
    }
    Ok(())
}

#[test]
fn pad_release_removes_the_outline_without_clearing_software_state() -> anyhow::Result<()> {
    // Given a held solo pad with confirmed active feedback.
    let mut translator = Translator {
        pads: [PadMode::Stems; 2],
        ..Translator::default()
    };
    let mut feedback = FeedbackState::default();
    feed(&mut feedback, &[0x90, 60, 127]);
    translator.translate(
        &[Control::Button("left_pad_5", true)],
        None,
        &mut Vec::new(),
    )?;
    // When the physical pad releases.
    let mut sent = Vec::new();
    translator.translate(&[Control::Button("left_pad_5", false)], None, &mut sent)?;
    // Then the outline clears, but the returned toggle state stays on.
    assert_eq!(sent, [[0x80, 60, 0]]);
    assert_eq!(
        pads(&translator, &feedback, 0)?[4],
        PadState {
            active: Some(true),
            held: false
        }
    );
    Ok(())
}

#[test]
fn exclusive_solo_requires_one_confirmed_solo_and_all_others_confirmed_off() -> anyhow::Result<()> {
    for (values, expected) in [
        ([Some(0), Some(127), Some(0), Some(0)], Some(true)),
        ([Some(127), Some(127), Some(0), Some(0)], Some(false)),
        ([Some(0), Some(127), None, Some(0)], None),
    ] {
        // Given Shift+STEMS and Djay's normal-channel solo states.
        let translator = Translator {
            pads: [PadMode::Stems; 2],
            shifted: [true; 2],
            ..Translator::default()
        };
        let mut feedback = FeedbackState::default();
        for (note, value) in (60_u8..64).zip(values) {
            if let Some(value) = value {
                feed(&mut feedback, &[0x90, note, value]);
            }
        }
        // When the shifted page derives the upper exclusive-solo row.
        let visible = pads(&translator, &feedback, 0)?;
        // Then ONLY never claims exclusivity from just one positive solo packet.
        assert_eq!(visible[1].active, expected);
        assert_eq!(visible[5].active, Some(true));
    }
    Ok(())
}

#[test]
fn hotcue_and_samples_share_info_without_leaking_stem_feedback() {
    for mode in [PadMode::Hotcue, PadMode::Samples] {
        // Given ordinary pad modes with active stem feedback and modifiers.
        let translator = Translator {
            pads: [mode; 2],
            shifted: [true; 2],
            ..Translator::default()
        };
        let mut feedback = FeedbackState::default();
        feed(&mut feedback, &[0x90, 56, 127]);
        // When pages are selected.
        let visible = pages(&translator, &feedback);
        // Then both modes use the same unknown info fields, not a pad legend.
        assert_eq!(visible, [Page::Info(Info::default()); 2]);
    }
}

mod info;
