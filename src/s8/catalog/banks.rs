//! Default FX and pad bindings.

use super::{Binding, PadMode, cc, note};

/// The left FX unit; the deck it targets supplies the channel. Controls 1-3
/// are the three effect slots; dial 4 is dry/wet for all of them.
pub const FX_LEFT: &[Binding] = &[
    note("fx_1", 32, "turntable{deck}.fx1Enabled"),
    note("fx_2", 33, "turntable{deck}.fx2Enabled"),
    note("fx_3", 34, "turntable{deck}.fx3Enabled"),
    note("fx_4", 35, "turntable{deck}.fxActive"),
    note("fx_1", 32, "turntable{deck}.fx1SelectNext").shift(),
    note("fx_2", 33, "turntable{deck}.fx2SelectNext").shift(),
    note("fx_3", 34, "turntable{deck}.fx3SelectNext").shift(),
    cc("fx_dial_1", 32, "turntable{deck}.fx1ParameterValue").with_pickup(),
    cc("fx_dial_2", 33, "turntable{deck}.fx2ParameterValue").with_pickup(),
    cc("fx_dial_3", 34, "turntable{deck}.fx3ParameterValue").with_pickup(),
    cc("fx_dial_4", 35, "turntable{deck}.fx1WetDryValue").with_pickup(),
    cc("fx_dial_4", 35, "turntable{deck}.fx2WetDryValue").with_pickup(),
    cc("fx_dial_4", 35, "turntable{deck}.fx3WetDryValue").with_pickup(),
];

/// The right FX unit, on its own numbers so both units can target one deck.
pub const FX_RIGHT: &[Binding] = &[
    note("fx_1", 36, "turntable{deck}.fx1Enabled"),
    note("fx_2", 37, "turntable{deck}.fx2Enabled"),
    note("fx_3", 38, "turntable{deck}.fx3Enabled"),
    note("fx_4", 39, "turntable{deck}.fxActive"),
    note("fx_1", 36, "turntable{deck}.fx1SelectNext").shift(),
    note("fx_2", 37, "turntable{deck}.fx2SelectNext").shift(),
    note("fx_3", 38, "turntable{deck}.fx3SelectNext").shift(),
    cc("fx_dial_1", 40, "turntable{deck}.fx1ParameterValue").with_pickup(),
    cc("fx_dial_2", 41, "turntable{deck}.fx2ParameterValue").with_pickup(),
    cc("fx_dial_3", 42, "turntable{deck}.fx3ParameterValue").with_pickup(),
    cc("fx_dial_4", 43, "turntable{deck}.fx1WetDryValue").with_pickup(),
    cc("fx_dial_4", 43, "turntable{deck}.fx2WetDryValue").with_pickup(),
    cc("fx_dial_4", 43, "turntable{deck}.fx3WetDryValue").with_pickup(),
];

const HOTCUE_PADS: &[Binding] = &[
    note("pad_1", 40, "turntable{deck}.cueOrJumpIfAlreadySet1"),
    note("pad_2", 41, "turntable{deck}.cueOrJumpIfAlreadySet2"),
    note("pad_3", 42, "turntable{deck}.cueOrJumpIfAlreadySet3"),
    note("pad_4", 43, "turntable{deck}.cueOrJumpIfAlreadySet4"),
    note("pad_5", 44, "turntable{deck}.cueOrJumpIfAlreadySet5"),
    note("pad_6", 45, "turntable{deck}.cueOrJumpIfAlreadySet6"),
    note("pad_7", 46, "turntable{deck}.cueOrJumpIfAlreadySet7"),
    note("pad_8", 47, "turntable{deck}.cueOrJumpIfAlreadySet8"),
    note("pad_1", 40, "turntable{deck}.clearCuePoint1").shift(),
    note("pad_2", 41, "turntable{deck}.clearCuePoint2").shift(),
    note("pad_3", 42, "turntable{deck}.clearCuePoint3").shift(),
    note("pad_4", 43, "turntable{deck}.clearCuePoint4").shift(),
    note("pad_5", 44, "turntable{deck}.clearCuePoint5").shift(),
    note("pad_6", 45, "turntable{deck}.clearCuePoint6").shift(),
    note("pad_7", 46, "turntable{deck}.clearCuePoint7").shift(),
    note("pad_8", 47, "turntable{deck}.clearCuePoint8").shift(),
];

/// Bounce loops, held: 1/32 beat on pad 1 to 4 beats on pad 8.
const LOOP_PADS: &[Binding] = &[
    note("pad_1", 64, "turntable{deck}.bounceLoop003125BeatInterval"),
    note("pad_2", 65, "turntable{deck}.bounceLoop00625BeatInterval"),
    note("pad_3", 66, "turntable{deck}.bounceLoop0125BeatInterval"),
    note("pad_4", 67, "turntable{deck}.bounceLoop025BeatInterval"),
    note("pad_5", 68, "turntable{deck}.bounceLoop05BeatInterval"),
    note("pad_6", 69, "turntable{deck}.bounceLoop1BeatInterval"),
    note("pad_7", 70, "turntable{deck}.bounceLoop2BeatInterval"),
    note("pad_8", 71, "turntable{deck}.bounceLoop4BeatInterval"),
];

const SLICER_PADS: &[Binding] = &[
    note("pad_1", 72, "turntable{deck}.slicer8Slice1"),
    note("pad_2", 73, "turntable{deck}.slicer8Slice2"),
    note("pad_3", 74, "turntable{deck}.slicer8Slice3"),
    note("pad_4", 75, "turntable{deck}.slicer8Slice4"),
    note("pad_5", 76, "turntable{deck}.slicer8Slice5"),
    note("pad_6", 77, "turntable{deck}.slicer8Slice6"),
    note("pad_7", 78, "turntable{deck}.slicer8Slice7"),
    note("pad_8", 79, "turntable{deck}.slicer8Slice8"),
];

const SAMPLER_PADS: &[Binding] = &[
    note(
        "pad_1",
        48,
        "sampler.turntable{bank}.player1.playingConsideringHoldSetting",
    ),
    note(
        "pad_2",
        49,
        "sampler.turntable{bank}.player2.playingConsideringHoldSetting",
    ),
    note(
        "pad_3",
        50,
        "sampler.turntable{bank}.player3.playingConsideringHoldSetting",
    ),
    note(
        "pad_4",
        51,
        "sampler.turntable{bank}.player4.playingConsideringHoldSetting",
    ),
    note(
        "pad_5",
        52,
        "sampler.turntable{bank}.player5.playingConsideringHoldSetting",
    ),
    note(
        "pad_6",
        53,
        "sampler.turntable{bank}.player6.playingConsideringHoldSetting",
    ),
    note(
        "pad_7",
        54,
        "sampler.turntable{bank}.player7.playingConsideringHoldSetting",
    ),
    note(
        "pad_8",
        55,
        "sampler.turntable{bank}.player8.playingConsideringHoldSetting",
    ),
    note("pad_1", 48, "sampler.turntable{bank}.player1.paused").shift(),
    note("pad_2", 49, "sampler.turntable{bank}.player2.paused").shift(),
    note("pad_3", 50, "sampler.turntable{bank}.player3.paused").shift(),
    note("pad_4", 51, "sampler.turntable{bank}.player4.paused").shift(),
    note("pad_5", 52, "sampler.turntable{bank}.player5.paused").shift(),
    note("pad_6", 53, "sampler.turntable{bank}.player6.paused").shift(),
    note("pad_7", 54, "sampler.turntable{bank}.player7.paused").shift(),
    note("pad_8", 55, "sampler.turntable{bank}.player8.paused").shift(),
];

pub const fn pads(mode: PadMode) -> &'static [Binding] {
    match mode {
        PadMode::Hotcue => HOTCUE_PADS,
        PadMode::Loop => LOOP_PADS,
        PadMode::Freeze => SLICER_PADS,
        PadMode::Remix => SAMPLER_PADS,
    }
}
