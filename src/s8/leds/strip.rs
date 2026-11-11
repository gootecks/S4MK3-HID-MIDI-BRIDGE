//! Touchstrip LED bank helpers; index 0 is the physical left end.

use super::{MAX_VALUE, STRIP_LEDS};

const MAX_INPUT: u16 = 127;

fn scaled(value: u16, span: u16) -> usize {
    let rounded = (value.min(MAX_INPUT) * span + MAX_INPUT / 2) / MAX_INPUT;
    usize::from(rounded)
}

pub fn dot(position: u8, intensity: u8) -> [u8; STRIP_LEDS] {
    let mut leds = [0; STRIP_LEDS];
    let span = u16::try_from(STRIP_LEDS - 1).unwrap_or(24);
    let index = scaled(u16::from(position), span).min(STRIP_LEDS - 1);
    leds[index] = intensity.min(MAX_VALUE);
    leds
}

pub fn bar(fraction: u8, intensity: u8) -> [u8; STRIP_LEDS] {
    let mut leds = [0; STRIP_LEDS];
    let span = u16::try_from(STRIP_LEDS).unwrap_or(25);
    let mut count = scaled(u16::from(fraction), span);
    if fraction > 0 {
        count = count.max(1);
    }
    for led in leds.iter_mut().take(count) {
        *led = intensity.min(MAX_VALUE);
    }
    leds
}
