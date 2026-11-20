//! Kontrol S8 events to the MIDI messages of the S8 catalog.

mod lookup;

#[cfg(test)]
mod evan_tests;
#[cfg(test)]
mod led_tests;
#[cfg(test)]
mod mixer_tests;
#[cfg(test)]
mod replay_tests;
#[cfg(test)]
mod tests;

use encdr::Event;
use lookup::{AssignBit, find};

use super::catalog::{
    Binding, CROSSFADER_ASSIGN, DECK_SELECT, DECKS, FX_SLOT_INPUTS, Kind, Layer, Layout, MIXER,
    PadMode, Scope, UNBOUND, feedback_number,
};
use super::feedback::FeedbackState;
use super::strip::{self, Owner, Strip};

const FILTER_KNOBS: [&str; 4] = [
    "mixer_filter_a",
    "mixer_filter_b",
    "mixer_filter_c",
    "mixer_filter_d",
];
const FILTER_CENTRE: u8 = 64;
const STRIP_TOUCH: &str = "touchstrip_touch";
const STRIP: &str = "touchstrip";

#[derive(Clone, Copy, Debug)]
struct Held {
    name: &'static str,
    channel: u8,
    number: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Position {
    Left,
    Middle,
    Right,
}

/// The two bits the S8 sets for a crossfader assign switch; the right position
/// sets neither.
#[derive(Clone, Copy, Debug, Default)]
struct Assign {
    left: bool,
    middle: bool,
}

impl Assign {
    const fn position(self) -> Position {
        if self.left {
            Position::Left
        } else if self.middle {
            Position::Middle
        } else {
            Position::Right
        }
    }
}

const fn layer(shifted: bool) -> Layer {
    if shifted { Layer::Shift } else { Layer::Base }
}

const fn note_on(channel: u8, number: u8) -> [u8; 3] {
    [0x90 | channel, number, 127]
}

const fn note_off(channel: u8, number: u8) -> [u8; 3] {
    [0x80 | channel, number, 0]
}

const fn controller(channel: u8, number: u8, value: u8) -> [u8; 3] {
    [0xb0 | channel, number, value]
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub flux: bool,
    pub edit: bool,
}

/// Turns S8 events into MIDI. Each side tracks its own deck, Shift and pad
/// layer; a note is released where it was pressed.
pub struct Translator {
    layout: Layout,
    modifiers: [Modifiers; 2],
    decks: [u8; 2],
    shift: [bool; 2],
    pad_mode: [PadMode; 2],
    fx_target: [u8; 2],
    fx_all_held: [bool; 2],
    held: Vec<Held>,
    sent: [[Option<u8>; 128]; 16],
    filter_on: [bool; 4],
    filter_value: [Option<u8>; 4],
    assign: [Assign; 4],
    assign_sent: [Option<Position>; 4],
    assign_changed: bool,
    strips: [Strip; 2],
    strip_position: [Option<u8>; 2],
}

impl Default for Translator {
    fn default() -> Self {
        Self::new()
    }
}

impl Translator {
    pub fn new() -> Self {
        Self::with_layout(Layout::Default)
    }

    pub fn with_layout(layout: Layout) -> Self {
        Self {
            layout,
            modifiers: [Modifiers::default(); 2],
            decks: [DECKS[0], DECKS[1]],
            shift: [false; 2],
            pad_mode: [PadMode::Hotcue; 2],
            fx_target: [DECKS[0], DECKS[1]],
            fx_all_held: [false; 2],
            held: Vec::with_capacity(16),
            sent: [[None; 128]; 16],
            filter_on: [true; 4],
            filter_value: [None; 4],
            assign: [Assign::default(); 4],
            assign_sent: [None; 4],
            assign_changed: false,
            strips: [Strip::default(), Strip::default()],
            strip_position: [None; 2],
        }
    }

    pub const fn layout(&self) -> Layout {
        self.layout
    }

    pub const fn modifiers(&self, side: usize) -> Modifiers {
        self.modifiers[side]
    }

    fn modifier_held(&self) -> bool {
        self.modifiers.iter().any(|side| side.flux || side.edit)
    }

    fn deck_row(
        &self,
        deck: u8,
        input: &str,
        kind: Kind,
        shifted: bool,
    ) -> Option<&'static Binding> {
        lookup::find_rows(self.layout.deck_rows(deck), input, kind, shifted)
    }

    /// Translates one event. `resting` marks the first report of each report ID.
    pub fn handle(
        &mut self,
        event: &Event,
        resting: bool,
        feedback: Option<&FeedbackState>,
        out: &mut Vec<[u8; 3]>,
    ) {
        match *event {
            Event::Button { name, pressed, .. } => {
                self.button(name, pressed, resting, feedback, out);
            }
            Event::Touch { name, touched, .. } => self.touch(name, touched, resting, out),
            Event::Encoder { name, delta, .. } => self.encoder(name, delta, resting, out),
            Event::Slider { name, value, .. } => self.slider(name, value, resting, out),
            _ => {}
        }
    }

    /// The deck (1-4) a side currently controls.
    pub const fn deck(&self, side: usize) -> u8 {
        self.decks[side]
    }

    pub const fn shift_held(&self, side: usize) -> bool {
        self.shift[side]
    }

    pub const fn pad_mode(&self, side: usize) -> PadMode {
        self.pad_mode[side]
    }

    /// The deck (1-4) the side's FX unit currently targets.
    pub const fn fx_target(&self, side: usize) -> u8 {
        self.fx_target[side]
    }

    /// The 7-bit position of a finger on the side's strip, while touched and moving.
    pub const fn strip_position(&self, side: usize) -> Option<u8> {
        self.strip_position[side]
    }

    /// True while the side's unshifted CUE note is held on the deck it shows.
    pub fn cue_held(&self, side: usize) -> bool {
        let Some(row) = self.deck_row(self.decks[side], "cue", Kind::Note, false) else {
            return false;
        };
        let channel = row.channel(self.decks[side]);
        self.held
            .iter()
            .any(|held| held.channel == channel && held.number == row.number)
    }

    /// Settles what a whole report decided. Call it after the report's events.
    pub fn end_report(&mut self, out: &mut Vec<[u8; 3]>) {
        if !self.assign_changed {
            return;
        }
        self.assign_changed = false;
        for (channel, switch) in self.assign.iter().enumerate() {
            let position = switch.position();
            if self.assign_sent[channel] == Some(position) {
                continue;
            }
            self.assign_sent[channel] = Some(position);
            let index = match position {
                Position::Left => 0,
                Position::Middle => 1,
                Position::Right => 2,
            };
            let row = CROSSFADER_ASSIGN[channel][index];
            let address = row.channel(0);
            out.push(note_on(address, row.number));
            out.push(note_off(address, row.number));
        }
    }

    fn button(
        &mut self,
        name: &'static str,
        pressed: bool,
        resting: bool,
        feedback: Option<&FeedbackState>,
        out: &mut Vec<[u8; 3]>,
    ) {
        if let Some((channel, bit)) = lookup::assign_position(name) {
            let switch = &mut self.assign[channel];
            match bit {
                AssignBit::Left => switch.left = pressed,
                AssignBit::Middle => switch.middle = pressed,
            }
            self.assign_changed = true;
            return;
        }
        if let Some((side, "shift")) = lookup::side(name) {
            self.shift[side] = pressed;
            return;
        }
        if self.layout == Layout::Evan
            && let Some((side, "fx_4")) = lookup::side(name)
        {
            let was_held = std::mem::replace(&mut self.fx_all_held[side], pressed);
            if pressed && !was_held && !resting {
                self.toggle_all_fx(side, feedback, out);
            }
            return;
        }
        if self.layout == Layout::Evan
            && let Some((side, control @ ("flux" | "edit"))) = lookup::side(name)
        {
            let modifiers = &mut self.modifiers[side];
            if control == "edit" {
                modifiers.edit = pressed;
            } else {
                modifiers.flux = pressed;
            }
            return;
        }
        if !pressed {
            self.release(name, out);
        } else if !resting && !self.held.iter().any(|held| held.name == name) {
            self.press(name, out);
        }
    }

    fn toggle_all_fx(&self, side: usize, feedback: Option<&FeedbackState>, out: &mut Vec<[u8; 3]>) {
        // Evan's embedded outputs use the same channel and notes as the slot buttons.
        let channel = self.fx_target[side] - 1;
        let table = self.layout.fx(side);
        let slots = FX_SLOT_INPUTS.map(|input| {
            feedback_number(table, input).map(|number| {
                let on = feedback.is_some_and(|state| state.note_on(usize::from(channel), number));
                (number, on)
            })
        });
        let any_on = slots.iter().flatten().any(|&(_, on)| on);
        for (number, on) in slots.into_iter().flatten() {
            if on == any_on {
                out.push(note_on(channel, number));
                out.push(note_off(channel, number));
            }
        }
    }

    fn press(&mut self, name: &'static str, out: &mut Vec<[u8; 3]>) {
        let row = if let Some((side, suffix)) = lookup::side(name) {
            if suffix == "deck" {
                self.press_deck(name, side, out);
                return;
            }
            // Returning from Evan's Slicer uses the Hotcues selector's address
            // but FREEZE still owns the note until the physical button is released.
            let suffix = if self.layout == Layout::Evan
                && suffix == "freeze"
                && self.shift[side]
                && self.pad_mode[side] == PadMode::Freeze
            {
                "hotcue"
            } else {
                suffix
            };
            if let Some(mode) = lookup::pad_mode(self.layout, suffix, self.shift[side]) {
                self.pad_mode[side] = mode;
            }
            self.side_row(side, suffix, Kind::Note)
        } else if let (Layout::Default, Some((unit, channel))) =
            (self.layout, lookup::fx_assign(name))
        {
            self.fx_target[unit] = DECKS[channel];
            return;
        } else if let Some(channel) = lookup::filter_on(name) {
            self.toggle_filter(channel, out);
            return;
        } else {
            self.row(name, Kind::Note)
        };
        if let Some((row, deck)) = row {
            self.hold(name, row.channel(deck), row.number, out);
        }
    }

    fn press_deck(&mut self, name: &'static str, side: usize, out: &mut Vec<[u8; 3]>) {
        let deck = self.decks[side];
        let next = if deck > 2 { deck - 2 } else { deck + 2 };
        self.decks[side] = next;
        let row = DECK_SELECT[usize::from(next - 1)];
        self.hold(name, row.channel(0), row.number, out);
    }

    fn hold(&mut self, name: &'static str, channel: u8, number: u8, out: &mut Vec<[u8; 3]>) {
        out.push(note_on(channel, number));
        self.held.push(Held {
            name,
            channel,
            number,
        });
    }

    fn release(&mut self, name: &str, out: &mut Vec<[u8; 3]>) {
        if let Some(index) = self.held.iter().position(|held| held.name == name) {
            let held = self.held.swap_remove(index);
            out.push(note_off(held.channel, held.number));
        }
    }

    fn toggle_filter(&mut self, channel: usize, out: &mut Vec<[u8; 3]>) {
        self.filter_on[channel] = !self.filter_on[channel];
        let value = if self.filter_on[channel] {
            self.filter_value[channel]
        } else {
            Some(FILTER_CENTRE)
        };
        let row = find(MIXER, FILTER_KNOBS[channel], Kind::Absolute, false);
        if let (Some(row), Some(value)) = (row, value) {
            self.send_absolute(row, 0, value, false, out);
        }
    }

    fn touch(&mut self, name: &str, touched: bool, resting: bool, out: &mut Vec<[u8; 3]>) {
        let Some((side, STRIP_TOUCH)) = lookup::side(name) else {
            return;
        };
        if !touched {
            self.strip_position[side] = None;
            if let Some(owner) = self.strips[side].release() {
                let shifted = owner.layer == Layer::Shift;
                if let Some(row) = self.deck_row(owner.deck, STRIP_TOUCH, Kind::Note, shifted) {
                    out.push(note_off(row.channel(owner.deck), row.number));
                }
            }
        } else if !resting {
            self.strip_position[side] = None;
            let shifted = self.shift[side];
            self.strips[side].touch(Owner {
                deck: self.decks[side],
                layer: layer(shifted),
            });
            if let Some(row) = self.deck_row(self.decks[side], STRIP_TOUCH, Kind::Note, shifted) {
                out.push(note_on(row.channel(self.decks[side]), row.number));
            }
        }
    }

    fn encoder(&self, name: &str, delta: i32, resting: bool, out: &mut Vec<[u8; 3]>) {
        if resting || delta == 0 {
            return;
        }
        if let Some((row, deck)) = self.row(name, Kind::Relative) {
            let step = delta.clamp(-63, 63);
            let value = u8::try_from(64 + step).unwrap_or(FILTER_CENTRE);
            out.push(controller(row.channel(deck), row.number, value));
        }
    }

    fn slider(&mut self, name: &str, value: f32, resting: bool, out: &mut Vec<[u8; 3]>) {
        if let Some((side, STRIP)) = lookup::side(name) {
            self.strip_motion(side, value, out);
            return;
        }
        let Some((row, deck)) = self.row(name, Kind::Absolute) else {
            return;
        };
        let value = lookup::seven_bit(value);
        if let Some(channel) = lookup::filter_knob(name) {
            self.filter_value[channel] = Some(value);
            if !self.filter_on[channel] {
                return;
            }
        }
        self.send_absolute(row, deck, value, resting, out);
    }

    fn strip_motion(&mut self, side: usize, position: f32, out: &mut Vec<[u8; 3]>) {
        let Some((owner, value)) = self.strips[side].motion(strip::position(position)) else {
            return;
        };
        self.strip_position[side] = Some(value);
        let shifted = owner.layer == Layer::Shift;
        if let Some(row) = self.deck_row(owner.deck, STRIP, Kind::RotaryAbsolute, shifted) {
            out.push(controller(row.channel(owner.deck), row.number, value));
        }
    }

    /// Sends a 7-bit value once per change. Deck-scoped values at rest only
    /// seed the cache: the stem faders and FX dials have no position to sync.
    fn send_absolute(
        &mut self,
        row: &Binding,
        deck: u8,
        value: u8,
        resting: bool,
        out: &mut Vec<[u8; 3]>,
    ) {
        let channel = row.channel(deck);
        let Some(slot) = self
            .sent
            .get_mut(usize::from(channel))
            .and_then(|numbers| numbers.get_mut(usize::from(row.number)))
        else {
            return;
        };
        if *slot == Some(value) {
            return;
        }
        *slot = Some(value);
        if !(resting && row.scope == Scope::Deck) {
            out.push(controller(channel, row.number, value));
        }
    }

    /// The catalog row of a control and the deck it addresses; 0 outside deck scope.
    fn row(&self, name: &str, kind: Kind) -> Option<(&'static Binding, u8)> {
        if let Some((side, suffix)) = lookup::side(name) {
            return self.side_row(side, suffix, kind);
        }
        lookup::find_rows(self.layout.mixer_rows(), name, kind, false)
            .or_else(|| find(UNBOUND, name, kind, false))
            .map(|row| (row, 0))
    }

    fn side_row(&self, side: usize, suffix: &str, kind: Kind) -> Option<(&'static Binding, u8)> {
        let shifted = self.shift[side];
        let deck = self.decks[side];
        if !shifted
            && self.modifier_held()
            && let Some(row) = find(self.layout.pitch_rows(), suffix, kind, true)
        {
            return Some((row, deck));
        }
        if let Some(row) = find(
            self.layout.pad_rows(self.pad_mode[side]),
            suffix,
            kind,
            shifted,
        ) {
            return Some((row, deck));
        }
        if let Some(row) = find(self.layout.fx(side), suffix, kind, shifted) {
            return Some((row, self.fx_target[side]));
        }
        let shifted = if self.layout == Layout::Evan && suffix.starts_with("on_") {
            self.modifier_held()
        } else {
            shifted
        };
        self.deck_row(deck, suffix, kind, shifted)
            .map(|row| (row, deck))
    }
}
