//! Kontrol S8 control catalog: every MIDI address the bridge sends to Djay.
//!
//! The mapping generator and the translator both read these tables, so an
//! address is written once. `docs/s8/mapping.md` describes each assignment.

use crate::full::catalog::ControlNote;

/// Zero-based MIDI channels, as the plist stores them.
pub const MIXER_CHANNEL: u8 = 4;
pub const DECK_SELECT_CHANNEL: u8 = 5;
pub const SHIFT_CHANNEL_OFFSET: u8 = 8;

pub const DECKS: [u8; 4] = [1, 2, 3, 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Note,
    /// 7-bit position.
    Absolute,
    /// Signed steps around 64 (`rotary-64`).
    Relative,
    /// 7-bit position that Djay turns into motion (`rotary-absolute`).
    RotaryAbsolute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Base,
    Shift,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Sent on the channel of a deck; `{deck}` and `{bank}` are filled in.
    Deck,
    Mixer,
    DeckSelect,
}

/// The pad layers, named after the buttons that select them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadMode {
    Hotcue,
    Loop,
    Freeze,
    Remix,
}

impl PadMode {
    pub const ALL: [Self; 4] = [Self::Hotcue, Self::Loop, Self::Freeze, Self::Remix];
}

/// One MIDI address and the Djay action bound to it.
///
/// A control with a Shift row uses it while Shift is held; a control without
/// one keeps its base row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Binding {
    /// Decoder name; deck-scoped inputs omit the `left_` / `right_` prefix.
    pub input: &'static str,
    pub layer: Layer,
    pub scope: Scope,
    pub number: u8,
    pub kind: Kind,
    pub target: &'static str,
    pub pickup: bool,
    pub flipped: bool,
    pub sensitivity: Option<f64>,
    pub acceleration: Option<u8>,
}

impl Binding {
    const fn new(input: &'static str, number: u8, kind: Kind, target: &'static str) -> Self {
        Self {
            input,
            layer: Layer::Base,
            scope: Scope::Deck,
            number,
            kind,
            target,
            pickup: false,
            flipped: false,
            sensitivity: None,
            acceleration: None,
        }
    }

    const fn shift(self) -> Self {
        Self {
            layer: Layer::Shift,
            ..self
        }
    }

    const fn mixer(self) -> Self {
        Self {
            scope: Scope::Mixer,
            ..self
        }
    }

    const fn with_pickup(self) -> Self {
        Self {
            pickup: true,
            ..self
        }
    }

    const fn with_flip(self) -> Self {
        Self {
            flipped: true,
            ..self
        }
    }

    const fn tuned(self, sensitivity: f64, acceleration: u8) -> Self {
        Self {
            sensitivity: Some(sensitivity),
            acceleration: Some(acceleration),
            ..self
        }
    }

    /// Zero-based MIDI channel. `deck` is 1-4 and only matters in deck scope.
    pub const fn channel(&self, deck: u8) -> u8 {
        match (self.scope, self.layer) {
            (Scope::Mixer, _) => MIXER_CHANNEL,
            (Scope::DeckSelect, _) => DECK_SELECT_CHANNEL,
            (Scope::Deck, Layer::Base) => deck.saturating_sub(1),
            (Scope::Deck, Layer::Shift) => {
                deck.saturating_sub(1).saturating_add(SHIFT_CHANNEL_OFFSET)
            }
        }
    }

    /// The Djay key path for `deck`. Decks A and C share sampler bank 1,
    /// decks B and D bank 2.
    pub fn key_path(&self, deck: u8) -> String {
        let bank = if deck % 2 == 1 { "1" } else { "2" };
        self.target
            .replace(DECK_SLOT, &deck.to_string())
            .replace(BANK_SLOT, bank)
    }
}

const DECK_SLOT: &str = "{deck}";
const BANK_SLOT: &str = "{bank}";

const fn note(input: &'static str, number: u8, target: &'static str) -> Binding {
    Binding::new(input, number, Kind::Note, target)
}

const fn cc(input: &'static str, number: u8, target: &'static str) -> Binding {
    Binding::new(input, number, Kind::Absolute, target)
}

const fn relative(input: &'static str, number: u8, target: &'static str) -> Binding {
    Binding::new(input, number, Kind::Relative, target)
}

const fn strip(input: &'static str, number: u8, target: &'static str) -> Binding {
    Binding::new(input, number, Kind::RotaryAbsolute, target)
}

/// One deck side; the side's selected deck supplies the channel.
pub const DECK: &[Binding] = &[
    note("play", 0, "turntable{deck}.playPause"),
    note(
        "cue",
        1,
        "turntable{deck}.cuePositionOrJumpConsideringPlayState1",
    ),
    note("cue", 1, "turntable{deck}.gotoZero").shift(),
    note("sync", 2, "turntable{deck}.bpmSync"),
    note("flux", 4, "turntable{deck}.deckSlipToggle"),
    note("flux", 4, "turntable{deck}.reverseHold").shift(),
    note("loop_encoder_press", 6, "turntable{deck}.autoLoopOnOff"),
    note("loop_encoder_press", 6, "turntable{deck}.reloop").shift(),
    note("touchstrip_touch", 7, "turntable{deck}.scratchingMode"),
    note("touchstrip_touch", 7, "turntable{deck}.jogSeekMode").shift(),
    note("back", 9, "musicLibrary.libraryBack"),
    note("back", 9, "musicLibrary.toggleLibrarySource").shift(),
    note("browse_encoder_press", 12, "musicLibrary.load{deck}"),
    note(
        "browse_encoder_press",
        12,
        "musicLibrary.toggleLibraryVisible",
    )
    .shift(),
    note("edit", 13, "musicLibrary.togglePreview"),
    note("capture", 22, "turntable{deck}.recordSample"),
    note("freeze", 27, "turntable{deck}.padModeSlicer"),
    note("freeze", 27, "turntable{deck}.slicerLoopModeToggle").shift(),
    note("on_1", 56, "turntable{deck}.unmixerFourTrackChannel1Muted"),
    note("on_2", 57, "turntable{deck}.unmixerFourTrackChannel2Muted"),
    note("on_3", 58, "turntable{deck}.unmixerFourTrackChannel3Muted"),
    note("on_4", 59, "turntable{deck}.unmixerFourTrackChannel4Muted"),
    note("on_1", 56, "turntable{deck}.unmixerFourTrackChannel1Solo").shift(),
    note("on_2", 57, "turntable{deck}.unmixerFourTrackChannel2Solo").shift(),
    note("on_3", 58, "turntable{deck}.unmixerFourTrackChannel3Solo").shift(),
    note("on_4", 59, "turntable{deck}.unmixerFourTrackChannel4Solo").shift(),
    relative("loop_encoder", 1, "turntable{deck}.autoLoopDurationRotary"),
    relative("loop_encoder", 1, "turntable{deck}.skipRotary").shift(),
    relative("browse_encoder", 3, "musicLibrary.libraryRotary").with_flip(),
    relative("browse_encoder", 3, "musicLibrary.sectionRotary")
        .with_flip()
        .shift(),
    strip("touchstrip", 5, "turntable{deck}.scratchingMove")
        .with_flip()
        .tuned(18.0, 11),
    strip("touchstrip", 5, "turntable{deck}.jogSeekMove")
        .tuned(7.4, 0)
        .shift(),
    cc(
        "fader_1",
        48,
        "turntable{deck}.unmixerFourTrackChannel1Volume",
    )
    .with_pickup(),
    cc(
        "fader_2",
        49,
        "turntable{deck}.unmixerFourTrackChannel2Volume",
    )
    .with_pickup(),
    cc(
        "fader_3",
        50,
        "turntable{deck}.unmixerFourTrackChannel3Volume",
    )
    .with_pickup(),
    cc(
        "fader_4",
        51,
        "turntable{deck}.unmixerFourTrackChannel4Volume",
    )
    .with_pickup(),
];

/// The left FX unit; the deck it targets supplies the channel. Dial 1 is
/// dry/wet for all three effect slots.
pub const FX_LEFT: &[Binding] = &[
    note("fx_1", 32, "turntable{deck}.fxActive"),
    note("fx_2", 33, "turntable{deck}.fx1Enabled"),
    note("fx_3", 34, "turntable{deck}.fx2Enabled"),
    note("fx_4", 35, "turntable{deck}.fx3Enabled"),
    note("fx_2", 33, "turntable{deck}.fx1SelectNext").shift(),
    note("fx_3", 34, "turntable{deck}.fx2SelectNext").shift(),
    note("fx_4", 35, "turntable{deck}.fx3SelectNext").shift(),
    cc("fx_dial_2", 32, "turntable{deck}.fx1ParameterValue").with_pickup(),
    cc("fx_dial_3", 33, "turntable{deck}.fx2ParameterValue").with_pickup(),
    cc("fx_dial_4", 34, "turntable{deck}.fx3ParameterValue").with_pickup(),
    cc("fx_dial_1", 35, "turntable{deck}.fx1WetDryValue").with_pickup(),
    cc("fx_dial_1", 36, "turntable{deck}.fx2WetDryValue").with_pickup(),
    cc("fx_dial_1", 37, "turntable{deck}.fx3WetDryValue").with_pickup(),
];

/// The right FX unit, on its own numbers so both units can target one deck.
pub const FX_RIGHT: &[Binding] = &[
    note("fx_1", 36, "turntable{deck}.fxActive"),
    note("fx_2", 37, "turntable{deck}.fx1Enabled"),
    note("fx_3", 38, "turntable{deck}.fx2Enabled"),
    note("fx_4", 39, "turntable{deck}.fx3Enabled"),
    note("fx_2", 37, "turntable{deck}.fx1SelectNext").shift(),
    note("fx_3", 38, "turntable{deck}.fx2SelectNext").shift(),
    note("fx_4", 39, "turntable{deck}.fx3SelectNext").shift(),
    cc("fx_dial_2", 40, "turntable{deck}.fx1ParameterValue").with_pickup(),
    cc("fx_dial_3", 41, "turntable{deck}.fx2ParameterValue").with_pickup(),
    cc("fx_dial_4", 42, "turntable{deck}.fx3ParameterValue").with_pickup(),
    cc("fx_dial_1", 43, "turntable{deck}.fx1WetDryValue").with_pickup(),
    cc("fx_dial_1", 44, "turntable{deck}.fx2WetDryValue").with_pickup(),
    cc("fx_dial_1", 45, "turntable{deck}.fx3WetDryValue").with_pickup(),
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

/// Mixer controls, addressed by their full decoder names.
pub const MIXER: &[Binding] = &[
    cc("mixer_gain_a", 0, "turntable1.gain").mixer(),
    cc("mixer_gain_b", 1, "turntable2.gain").mixer(),
    cc("mixer_gain_c", 2, "turntable3.gain").mixer(),
    cc("mixer_gain_d", 3, "turntable4.gain").mixer(),
    cc("mixer_eq_hi_a", 4, "turntable1.highEQ").mixer(),
    cc("mixer_eq_hi_b", 5, "turntable2.highEQ").mixer(),
    cc("mixer_eq_hi_c", 6, "turntable3.highEQ").mixer(),
    cc("mixer_eq_hi_d", 7, "turntable4.highEQ").mixer(),
    cc("mixer_eq_mid_a", 8, "turntable1.midEQ").mixer(),
    cc("mixer_eq_mid_b", 9, "turntable2.midEQ").mixer(),
    cc("mixer_eq_mid_c", 10, "turntable3.midEQ").mixer(),
    cc("mixer_eq_mid_d", 11, "turntable4.midEQ").mixer(),
    cc("mixer_eq_low_a", 12, "turntable1.lowEQ").mixer(),
    cc("mixer_eq_low_b", 13, "turntable2.lowEQ").mixer(),
    cc("mixer_eq_low_c", 14, "turntable3.lowEQ").mixer(),
    cc("mixer_eq_low_d", 15, "turntable4.lowEQ").mixer(),
    cc("mixer_fader_a", 16, "mixer.lineVolume1").mixer(),
    cc("mixer_fader_b", 17, "mixer.lineVolume2").mixer(),
    cc("mixer_fader_c", 18, "mixer.lineVolume3").mixer(),
    cc("mixer_fader_d", 19, "mixer.lineVolume4").mixer(),
    cc("crossfader", 20, "mixer.crossfade").mixer(),
    cc("mixer_filter_a", 21, "turntable1.filter").mixer(),
    cc("mixer_filter_b", 22, "turntable2.filter").mixer(),
    cc("mixer_filter_c", 23, "turntable3.filter").mixer(),
    cc("mixer_filter_d", 24, "turntable4.filter").mixer(),
    relative("tempo_encoder", 26, "turntableSelected.speed")
        .tuned(3.5, 0)
        .mixer(),
    note("mixer_cue_a", 0, "mixer.monitorActive1").mixer(),
    note("mixer_cue_b", 1, "mixer.monitorActive2").mixer(),
    note("mixer_cue_c", 2, "mixer.monitorActive3").mixer(),
    note("mixer_cue_d", 3, "mixer.monitorActive4").mixer(),
    note("mixer_mic_1", 4, "microphone.active").mixer(),
    note("mixer_tempo", 26, "turntableSelected.resetSpeed").mixer(),
];

/// Crossfader assign switch per mixer channel A-D: left, middle, right.
pub const CROSSFADER_ASSIGN: [[Binding; 3]; 4] = [
    [
        note("xfader_assign_a", 80, "mixer.crossfadeAssignment1Left").mixer(),
        note("xfader_assign_a", 81, "mixer.crossfadeAssignment1Through").mixer(),
        note("xfader_assign_a", 82, "mixer.crossfadeAssignment1Right").mixer(),
    ],
    [
        note("xfader_assign_b", 83, "mixer.crossfadeAssignment2Left").mixer(),
        note("xfader_assign_b", 84, "mixer.crossfadeAssignment2Through").mixer(),
        note("xfader_assign_b", 85, "mixer.crossfadeAssignment2Right").mixer(),
    ],
    [
        note("xfader_assign_c", 86, "mixer.crossfadeAssignment3Left").mixer(),
        note("xfader_assign_c", 87, "mixer.crossfadeAssignment3Through").mixer(),
        note("xfader_assign_c", 88, "mixer.crossfadeAssignment3Right").mixer(),
    ],
    [
        note("xfader_assign_d", 89, "mixer.crossfadeAssignment4Left").mixer(),
        note("xfader_assign_d", 90, "mixer.crossfadeAssignment4Through").mixer(),
        note("xfader_assign_d", 91, "mixer.crossfadeAssignment4Right").mixer(),
    ],
];

const fn deck_select(number: u8, target: &'static str) -> Binding {
    Binding {
        scope: Scope::DeckSelect,
        ..note("deck", number, target)
    }
}

/// Sent by a DECK button for the deck its side switches to, decks 1-4.
pub const DECK_SELECT: [Binding; 4] = [
    deck_select(0, "application.turntable1Selected"),
    deck_select(1, "application.turntable2Selected"),
    deck_select(2, "application.turntable3Selected"),
    deck_select(3, "application.turntable4Selected"),
];

/// A binding on one deck; `deck` is 0 outside deck scope.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
    pub binding: Binding,
    pub deck: u8,
}

impl Entry {
    pub const fn channel(&self) -> u8 {
        self.binding.channel(self.deck)
    }

    pub fn key_path(&self) -> String {
        self.binding.key_path(self.deck)
    }
}

/// Every address of the catalog, on every deck and layer.
pub fn entries() -> impl Iterator<Item = Entry> {
    let deck_scoped = DECKS.into_iter().flat_map(|deck| {
        [DECK, FX_LEFT, FX_RIGHT]
            .into_iter()
            .chain(PadMode::ALL.map(pads))
            .flatten()
            .map(move |binding| Entry {
                binding: *binding,
                deck,
            })
    });
    let fixed = MIXER
        .iter()
        .chain(CROSSFADER_ASSIGN.iter().flatten())
        .chain(&DECK_SELECT)
        .map(|binding| Entry {
            binding: *binding,
            deck: 0,
        });
    deck_scoped.chain(fixed)
}

/// Controls the bridge keeps to itself.
pub const LOCAL_CONTROLS: &[ControlNote] = &[
    ControlNote {
        name: "shift",
        reason: "selects the Shift layer of its side",
    },
    ControlNote {
        name: "deck",
        reason: "switches its side between decks A/C or B/D, then sends the deck-select note",
    },
    ControlNote {
        name: "hotcue, loop, freeze, remix",
        reason: "select the pad layer of their side",
    },
    ControlNote {
        name: "mixer_fx_X_1, mixer_fx_X_2",
        reason: "select mixer channel X as the target of the left or right FX unit",
    },
    ControlNote {
        name: "mixer_filter_on_X",
        reason: "gates the FILTER knob of mixer channel X",
    },
];

/// Decoded controls with no Djay assignment in this layout.
pub const UNASSIGNED_CONTROLS: &[ControlNote] = &[
    ControlNote {
        name: "mixer_snap, mixer_quantize",
        reason: "left out of this prototype",
    },
    ControlNote {
        name: "mixer_mic_2",
        reason: "Djay has one microphone action",
    },
    ControlNote {
        name: "deck_assign_X",
        reason: "Djay decks are fixed to their mixer channels",
    },
    ControlNote {
        name: "fx_select",
        reason: "reserved for an on-screen effect picker",
    },
    ControlNote {
        name: "screen_left_2, screen_left_3, screen_right_1..4, screen_encoder_1..4",
        reason: "reserved for the app's screen pages",
    },
    ControlNote {
        name: "screen_left_1, screen_left_4",
        reason: "Native Instruments functions",
    },
    ControlNote {
        name: "*_touch except touchstrip_touch",
        reason: "touch caps are not sent to Djay",
    },
    ControlNote {
        name: "*_latch",
        reason: "toggle states the S8 keeps itself",
    },
];
