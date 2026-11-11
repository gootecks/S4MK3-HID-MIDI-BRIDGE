//! Kontrol S8 feedback addresses: where Djay reports the state the LEDs show.
//!
//! A control's embedded output reuses that control's own address, on the base
//! layer of its deck. Root outputs cover state no control carries and live on
//! one channel of their own. The mapping generator and the feedback decoder
//! both read this file.

use super::{Binding, Kind, Layer, PadMode, Scope, pads};

/// Zero-based channel of the root outputs (channel 7 in Djay's list).
pub const FEEDBACK_CHANNEL: u8 = 6;

pub const NOTE_TYPE: u8 = 1;
pub const CONTROLLER_TYPE: u8 = 3;

/// Cue colour tokens ride in the velocity of these notes; the output
/// dictionary stays empty so Djay sends the token rather than a 0-127 scale.
pub const CUE_COLOUR_NOTES: std::ops::RangeInclusive<u8> = 40..=47;

const DECK_TOKEN: &str = "{deck}";

pub const PAD_INPUTS: [&str; 8] = [
    "pad_1", "pad_2", "pad_3", "pad_4", "pad_5", "pad_6", "pad_7", "pad_8",
];

impl Kind {
    pub const fn message_type(self) -> u8 {
        match self {
            Self::Note => NOTE_TYPE,
            Self::Absolute | Self::Relative | Self::RotaryAbsolute => CONTROLLER_TYPE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootFamily {
    Playing,
    Loaded,
    Progress,
}

/// One root output per deck: the key path, kind and first data byte; deck `n`
/// (1-4) uses `base + n - 1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RootOutput {
    pub family: RootFamily,
    pub target: &'static str,
    pub kind: Kind,
    pub base: u8,
}

impl RootOutput {
    /// `(zero-based channel, message type, data byte)` for `deck` 1-4.
    pub const fn address(&self, deck: u8) -> (u8, u8, u8) {
        (
            FEEDBACK_CHANNEL,
            self.kind.message_type(),
            self.base + deck.saturating_sub(1),
        )
    }

    pub fn key_path(&self, deck: u8) -> String {
        let (head, tail) = self
            .target
            .split_once(DECK_TOKEN)
            .unwrap_or((self.target, ""));
        format!("{head}{deck}{tail}")
    }
}

pub const ROOT_OUTPUTS: [RootOutput; 3] = [
    RootOutput {
        family: RootFamily::Playing,
        target: "turntable{deck}.playing",
        kind: Kind::Absolute,
        base: 4,
    },
    RootOutput {
        family: RootFamily::Loaded,
        target: "turntable{deck}.song.loadingSuccess",
        kind: Kind::Note,
        base: 0,
    },
    RootOutput {
        family: RootFamily::Progress,
        target: "turntable{deck}.display.songProgress",
        kind: Kind::Absolute,
        base: 8,
    },
];

/// The note numbers on which Djay returns feedback for a deck's own controls.
/// Shared with the S4 MK3 mapping, which verified them.
pub const fn embedded_number(number: u8) -> bool {
    matches!(number, 0..=4 | 6 | 8 | 15..=19 | 32..=63)
}

/// Whether Djay reports the state of `binding` on its own address.
pub const fn has_embedded_output(binding: &Binding) -> bool {
    matches!(binding.layer, Layer::Base)
        && matches!(binding.scope, Scope::Deck)
        && matches!(binding.kind, Kind::Note)
        && embedded_number(binding.number)
}

/// The note on which `input` of `table` receives feedback, if it has any.
pub fn feedback_number(table: &'static [Binding], input: &str) -> Option<u8> {
    table
        .iter()
        .find(|binding| binding.input == input && has_embedded_output(binding))
        .map(|binding| binding.number)
}

/// The feedback note of pad `index` (0-7) in `mode`.
pub fn pad_feedback(mode: PadMode, index: usize) -> Option<u8> {
    feedback_number(pads(mode), PAD_INPUTS.get(index)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s8::catalog::{DECK, FX_LEFT, FX_RIGHT};

    #[test]
    fn deck_controls_report_on_their_own_notes() {
        assert_eq!(feedback_number(DECK, "play"), Some(0));
        assert_eq!(feedback_number(DECK, "cue"), Some(1));
        assert_eq!(feedback_number(DECK, "sync"), Some(2));
        assert_eq!(feedback_number(DECK, "flux"), Some(4));
        assert_eq!(feedback_number(DECK, "loop_encoder_press"), Some(6));
        assert_eq!(feedback_number(DECK, "on_4"), Some(59));
        assert_eq!(feedback_number(DECK, "back"), None);
        assert_eq!(feedback_number(DECK, "capture"), None);
        assert_eq!(feedback_number(DECK, "touchstrip_touch"), None);
    }

    #[test]
    fn fx_units_and_pads_report_where_they_are_bound() {
        assert_eq!(feedback_number(FX_LEFT, "fx_1"), Some(32));
        assert_eq!(feedback_number(FX_LEFT, "fx_4"), Some(35));
        assert_eq!(feedback_number(FX_RIGHT, "fx_1"), Some(36));
        assert_eq!(feedback_number(FX_RIGHT, "fx_4"), Some(39));
        assert_eq!(pad_feedback(PadMode::Hotcue, 0), Some(40));
        assert_eq!(pad_feedback(PadMode::Hotcue, 7), Some(47));
        assert_eq!(pad_feedback(PadMode::Remix, 0), Some(48));
        assert_eq!(pad_feedback(PadMode::Loop, 0), None);
        assert_eq!(pad_feedback(PadMode::Freeze, 0), None);
        assert_eq!(pad_feedback(PadMode::Hotcue, 8), None);
    }

    #[test]
    fn root_outputs_sit_on_the_feedback_channel_without_overlap() {
        let mut seen = std::collections::BTreeSet::new();
        for output in ROOT_OUTPUTS {
            for deck in super::super::DECKS {
                assert!(seen.insert(output.address(deck)));
                assert_eq!(output.address(deck).0, FEEDBACK_CHANNEL);
            }
        }
    }
}
