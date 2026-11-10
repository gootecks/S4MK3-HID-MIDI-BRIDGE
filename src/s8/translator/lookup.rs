//! Name parsing and catalog row lookup. Nothing here allocates.

use crate::s8::catalog::{Binding, Kind, Layer, PadMode};

pub(super) const LEFT: usize = 0;
pub(super) const RIGHT: usize = 1;

/// Splits `left_play` into the left side and `play`.
pub(super) fn side(name: &str) -> Option<(usize, &str)> {
    name.strip_prefix("left_")
        .map(|rest| (LEFT, rest))
        .or_else(|| name.strip_prefix("right_").map(|rest| (RIGHT, rest)))
}

pub(super) fn pad_mode(button: &str) -> Option<PadMode> {
    match button {
        "hotcue" => Some(PadMode::Hotcue),
        "loop" => Some(PadMode::Loop),
        "freeze" => Some(PadMode::Freeze),
        "remix" => Some(PadMode::Remix),
        _ => None,
    }
}

/// The mixer channel index 0-3 of a single letter `a`-`d`.
pub(super) fn channel(letter: &str) -> Option<usize> {
    match letter {
        "a" => Some(0),
        "b" => Some(1),
        "c" => Some(2),
        "d" => Some(3),
        _ => None,
    }
}

/// `mixer_fx_a_2` is channel A assigned to FX unit 2: `(unit index, channel)`.
pub(super) fn fx_assign(name: &str) -> Option<(usize, usize)> {
    let rest = name.strip_prefix("mixer_fx_")?;
    let (letter, unit) = rest.split_once('_')?;
    let unit = match unit {
        "1" => LEFT,
        "2" => RIGHT,
        _ => return None,
    };
    Some((unit, channel(letter)?))
}

pub(super) fn filter_on(name: &str) -> Option<usize> {
    channel(name.strip_prefix("mixer_filter_on_")?)
}

pub(super) fn filter_knob(name: &str) -> Option<usize> {
    channel(name.strip_prefix("mixer_filter_")?)
}

pub(super) fn assign_position(name: &str) -> Option<(usize, AssignBit)> {
    let rest = name.strip_prefix("xfader_assign_")?;
    let (bit, letter) = rest.split_once('_')?;
    let bit = match bit {
        "left" => AssignBit::Left,
        "middle" => AssignBit::Middle,
        _ => return None,
    };
    Some((channel(letter)?, bit))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AssignBit {
    Left,
    Middle,
}

/// The row of `input` and `kind`: the Shift row while Shift is held when the
/// control has one, otherwise its base row.
pub(super) fn find(
    table: &'static [Binding],
    input: &str,
    kind: Kind,
    shifted: bool,
) -> Option<&'static Binding> {
    let mut base = None;
    for row in table
        .iter()
        .filter(|row| row.kind == kind && row.input == input)
    {
        match row.layer {
            Layer::Shift if shifted => return Some(row),
            Layer::Base => base = Some(row),
            Layer::Shift => {}
        }
    }
    base
}

/// A 7-bit controller value from a normalised slider position.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn seven_bit(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 127.0).round() as u8
}
