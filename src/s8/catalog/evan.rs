//! Evan's Djay actions, translated from Controller Editor controls to bridge addresses.

use super::{Binding, DECK, FX_LEFT, FX_RIGHT, Layout, MIXER, PadMode, cc, note, pads, strip};

const A: u8 = 0b0001;
const B: u8 = 0b0010;
const C: u8 = 0b0100;
const D: u8 = 0b1000;

impl Layout {
    pub fn deck_rows(self, deck: u8) -> impl Iterator<Item = &'static Binding> {
        let extra = match self {
            Self::Default => &[][..],
            Self::Evan => DECK_OVERRIDES,
        };
        DECK.iter()
            .filter(move |row| self == Self::Default || !is_replaced(row.input))
            .chain(extra.iter().filter(move |row| row.applies_to(deck)))
    }

    pub const fn fx(self, side: usize) -> &'static [Binding] {
        match (self, side) {
            (Self::Default, 0) => FX_LEFT,
            (Self::Default, _) => FX_RIGHT,
            (Self::Evan, 0) => LEFT_FX,
            (Self::Evan, _) => RIGHT_FX,
        }
    }

    pub fn mixer_rows(self) -> impl Iterator<Item = &'static Binding> {
        let extra = match self {
            Self::Default => &[][..],
            Self::Evan => TEMPO_NUDGE,
        };
        MIXER.iter().chain(extra)
    }

    pub const fn pad_rows(self, mode: PadMode) -> &'static [Binding] {
        match (self, mode) {
            (Self::Default, _) | (Self::Evan, PadMode::Freeze) => pads(mode),
            (Self::Evan, PadMode::Hotcue) => HOTCUES,
            (Self::Evan, PadMode::Loop | PadMode::Remix) => &[],
        }
    }

    pub const fn pitch_rows(self) -> &'static [Binding] {
        match self {
            Self::Default => &[],
            Self::Evan => PITCH_PLAY,
        }
    }
}

fn is_replaced(input: &str) -> bool {
    matches!(
        input,
        "flux"
            | "edit"
            | "back"
            | "browse_encoder_press"
            | "browse_encoder"
            | "capture"
            | "cue"
            | "loop_encoder_press"
            | "touchstrip"
            | "hotcue"
            | "loop"
            | "freeze"
            | "remix"
            | "on_1"
            | "on_2"
            | "on_3"
            | "on_4"
    )
}

const DECK_OVERRIDES: &[Binding] = &[
    note(
        "cue",
        1,
        "turntable{deck}.cuePositionOrJumpConsideringPlayState1",
    ),
    note("loop_encoder_press", 6, "turntable{deck}.autoLoopOnOff"),
    note("back", 9, "musicLibrary.toggleLibrarySource").on_decks(A | B),
    note("back", 9, "musicLibrary.libraryBack").on_decks(C | D),
    note("browse_encoder_press", 12, "musicLibrary.load{deck}"),
    note(
        "browse_encoder_press",
        12,
        "musicLibrary.toggleLibraryVisible",
    ),
    note("browse_encoder_press", 12, "musicLibrary.libraryBack")
        .shift()
        .on_decks(A | B),
    super::relative("browse_encoder", 3, "musicLibrary.libraryRotary").with_flip(),
    note("capture", 22, "turntable2.downBeat").on_decks(B),
    note("hotcue", 25, "turntable{deck}.bounceLoop1BeatInterval"),
    note("loop", 26, "turntable{deck}.bounceLoop05BeatInterval"),
    note("freeze", 27, "turntable{deck}.bounceLoop025BeatInterval"),
    note("remix", 28, "turntable{deck}.bounceLoop0125BeatInterval"),
    note("hotcue", 25, "turntable{deck}.padModeHotCue").shift(),
    note("freeze", 27, "turntable{deck}.padModeSlicer").shift(),
    note("on_1", 56, "turntable{deck}.pitchPlus").without_feedback(),
    note("on_2", 57, "turntable{deck}.pitchMinus").without_feedback(),
    note("on_3", 58, "turntable{deck}.resetPitch")
        .on_decks(A | C | D)
        .without_feedback(),
    note("on_4", 59, "turntable{deck}.resetSpeed")
        .on_decks(A | C | D)
        .without_feedback(),
    note("on_3", 58, "musicLibrary.deckMove2To3")
        .on_decks(B)
        .without_feedback(),
    note("on_4", 59, "musicLibrary.deckMove2To4")
        .on_decks(B)
        .without_feedback(),
    note("on_1", 56, "turntable{deck}.bounceLoop05BeatInterval")
        .shift()
        .on_decks(A | B),
    note("on_2", 57, "turntable{deck}.bounceLoop025BeatInterval")
        .shift()
        .on_decks(A | B),
    note("on_3", 58, "turntable{deck}.bounceLoop0125BeatInterval")
        .shift()
        .on_decks(A | B),
    note("on_4", 59, "turntable{deck}.bounceLoop00625BeatInterval")
        .shift()
        .on_decks(A | B),
    note("on_1", 56, "turntable3.autoLoop05BeatInterval")
        .shift()
        .on_decks(C),
    note("on_2", 57, "turntable3.autoLoop025BeatInterval")
        .shift()
        .on_decks(C),
    note("on_3", 58, "turntable3.autoLoop0125BeatInterval")
        .shift()
        .on_decks(C),
    note("on_4", 59, "turntable3.autoLoop00625BeatInterval")
        .shift()
        .on_decks(C),
    strip("touchstrip", 5, "turntable{deck}.scratchingMove")
        .with_flip()
        .tuned(20.0, 9)
        .on_decks(A),
    strip("touchstrip", 5, "turntable{deck}.scratchingMove")
        .with_flip()
        .tuned(18.0, 10)
        .on_decks(B),
    strip("touchstrip", 5, "turntable{deck}.scratchingMove")
        .with_flip()
        .tuned(16.0, 14)
        .on_decks(C),
    strip("touchstrip", 5, "turntable{deck}.scratchingMove")
        .with_flip()
        .tuned(19.0, 11)
        .on_decks(D),
    strip("touchstrip", 5, "turntable{deck}.jogSeekMove")
        .tuned(7.7, 0)
        .shift()
        .on_decks(A),
    strip("touchstrip", 5, "turntable{deck}.jogSeekMove")
        .tuned(7.1, 0)
        .shift()
        .on_decks(B),
    strip("touchstrip", 5, "turntable{deck}.jogSeekMove")
        .tuned(7.4, 0)
        .shift()
        .on_decks(C | D),
];

macro_rules! hotcue {
    ($pad:literal, $number:literal, $slot:literal) => {
        [
            note(
                $pad,
                $number,
                concat!("turntable{deck}.cueOrJumpIfAlreadySetCuePoint", $slot),
            ),
            note(
                $pad,
                $number,
                concat!("turntable{deck}.pitchPlayCuePointCuePoint", $slot),
            )
            .without_feedback(),
            note(
                $pad,
                $number,
                concat!("turntable{deck}.clearCuePointCuePoint", $slot),
            )
            .shift(),
        ]
    };
}

const HOTCUES: &[Binding] = &[
    hotcue!("pad_1", 40, "1")[0],
    hotcue!("pad_1", 40, "1")[1],
    hotcue!("pad_1", 40, "1")[2],
    hotcue!("pad_2", 41, "2")[0],
    hotcue!("pad_2", 41, "2")[1],
    hotcue!("pad_2", 41, "2")[2],
    hotcue!("pad_3", 42, "3")[0],
    hotcue!("pad_3", 42, "3")[1],
    hotcue!("pad_3", 42, "3")[2],
    hotcue!("pad_4", 43, "4")[0],
    hotcue!("pad_4", 43, "4")[1],
    hotcue!("pad_4", 43, "4")[2],
    hotcue!("pad_5", 44, "5")[0],
    hotcue!("pad_5", 44, "5")[1],
    hotcue!("pad_5", 44, "5")[2],
    hotcue!("pad_6", 45, "6")[0],
    hotcue!("pad_6", 45, "6")[1],
    hotcue!("pad_6", 45, "6")[2],
    hotcue!("pad_7", 46, "7")[0],
    hotcue!("pad_7", 46, "7")[1],
    hotcue!("pad_7", 46, "7")[2],
    hotcue!("pad_8", 47, "8")[0],
    hotcue!("pad_8", 47, "8")[1],
    hotcue!("pad_8", 47, "8")[2],
];

const PITCH_PLAY: &[Binding] = &[
    note("pad_1", 64, "turntable{deck}.pitchPlayAction1").shift(),
    note("pad_2", 65, "turntable{deck}.pitchPlayAction2").shift(),
    note("pad_3", 66, "turntable{deck}.pitchPlayAction3").shift(),
    note("pad_4", 67, "turntable{deck}.pitchPlayAction4").shift(),
    note("pad_5", 68, "turntable{deck}.pitchPlayAction5").shift(),
    note("pad_6", 69, "turntable{deck}.pitchPlayAction6").shift(),
    note("pad_7", 70, "turntable{deck}.pitchPlayAction7").shift(),
    note("pad_8", 71, "turntable{deck}.pitchPlayAction8").shift(),
];

// Each unit sends on its primary deck's channel. Djay applies the same address to both
// decks on that side, but only the primary deck returns FX feedback.
macro_rules! fx {
    ($mask:ident, $other:literal, $btn:literal, $dial:literal) => {
        &[
            note("fx_1", $btn, "turntable{deck}.fx1Enabled").on_decks($mask),
            note("fx_2", $btn + 1, "turntable{deck}.fx2Enabled").on_decks($mask),
            note("fx_3", $btn + 2, "turntable{deck}.fx3Enabled").on_decks($mask),
            note("fx_1", $btn, concat!("turntable", $other, ".fx1Enabled"))
                .on_decks($mask)
                .without_feedback(),
            note(
                "fx_2",
                $btn + 1,
                concat!("turntable", $other, ".fx2Enabled"),
            )
            .on_decks($mask)
            .without_feedback(),
            note(
                "fx_3",
                $btn + 2,
                concat!("turntable", $other, ".fx3Enabled"),
            )
            .on_decks($mask)
            .without_feedback(),
            cc("fx_dial_1", $dial, "turntable{deck}.fx1ParameterValue").on_decks($mask),
            cc("fx_dial_2", $dial + 1, "turntable{deck}.fx2ParameterValue").on_decks($mask),
            cc("fx_dial_3", $dial + 2, "turntable{deck}.fx3ParameterValue").on_decks($mask),
            cc(
                "fx_dial_1",
                $dial,
                concat!("turntable", $other, ".fx1ParameterValue"),
            )
            .on_decks($mask),
            cc(
                "fx_dial_2",
                $dial + 1,
                concat!("turntable", $other, ".fx2ParameterValue"),
            )
            .on_decks($mask),
            cc(
                "fx_dial_3",
                $dial + 2,
                concat!("turntable", $other, ".fx3ParameterValue"),
            )
            .on_decks($mask),
            cc("fx_dial_4", $dial + 3, "turntable{deck}.fx1WetDryValue").on_decks($mask),
            cc("fx_dial_4", $dial + 3, "turntable{deck}.fx2WetDryValue").on_decks($mask),
            cc("fx_dial_4", $dial + 3, "turntable{deck}.fx3WetDryValue").on_decks($mask),
            cc(
                "fx_dial_4",
                $dial + 3,
                concat!("turntable", $other, ".fx1WetDryValue"),
            )
            .on_decks($mask),
            cc(
                "fx_dial_4",
                $dial + 3,
                concat!("turntable", $other, ".fx2WetDryValue"),
            )
            .on_decks($mask),
            cc(
                "fx_dial_4",
                $dial + 3,
                concat!("turntable", $other, ".fx3WetDryValue"),
            )
            .on_decks($mask),
        ]
    };
}

const LEFT_FX: &[Binding] = fx!(A, "3", 32, 32);
const RIGHT_FX: &[Binding] = fx!(B, "4", 36, 40);

const TEMPO_NUDGE: &[Binding] = &[
    note("mixer_fx_a_1", 92, "turntable1.speedMinus").mixer(),
    note("mixer_fx_a_2", 93, "turntable1.speedPlus").mixer(),
    note("mixer_fx_b_1", 94, "turntable2.speedMinus").mixer(),
    note("mixer_fx_b_2", 95, "turntable2.speedPlus").mixer(),
    note("mixer_fx_c_1", 96, "turntable3.speedMinus").mixer(),
    note("mixer_fx_c_2", 97, "turntable3.speedPlus").mixer(),
    note("mixer_fx_d_1", 98, "turntable4.speedMinus").mixer(),
    note("mixer_fx_d_2", 99, "turntable4.speedPlus").mixer(),
];
