//! Pad colours: Djay's cue colour tokens as RGB for the S8 pads.

use std::sync::LazyLock;

use super::leds::{MAX_VALUE, Rgb};
use crate::full::feedback::PAD_WHITE_TOKEN;
use crate::full::leds::{Color, LedConfig};

/// The S4 colour a token resolves to, shared with the S4 path's defaults.
static TOKEN_COLOURS: LazyLock<([Color; 8], Color)> = LazyLock::new(|| {
    let config = LedConfig::default();
    (config.djay_pad_colors, config.djay_white)
});

/// RGB of `color` at full pad brightness (0..=`MAX_VALUE`).
const fn rgb(color: Color) -> Rgb {
    let m = MAX_VALUE;
    match color {
        Color::Red => Rgb::new(m, 0, 0),
        Color::Carrot => Rgb::new(m, 24, 0),
        Color::Orange => Rgb::new(m, 48, 0),
        Color::Honey => Rgb::new(m, 80, 0),
        Color::Yellow => Rgb::new(m, 112, 0),
        Color::Lime => Rgb::new(80, m, 0),
        Color::Green => Rgb::new(0, m, 0),
        Color::Aqua => Rgb::new(0, m, 48),
        Color::Cyan => Rgb::new(0, m, 100),
        Color::Sky => Rgb::new(0, 80, m),
        Color::Blue => Rgb::new(0, 0, m),
        Color::Purple => Rgb::new(48, 0, m),
        Color::Fuchsia => Rgb::new(100, 0, m),
        Color::Magenta => Rgb::new(m, 0, 100),
        Color::Azalea => Rgb::new(m, 0, 48),
        Color::Salmon => Rgb::new(m, 40, 40),
        Color::White => Rgb::new(m, m, m),
    }
}

/// The colour of a Djay cue token (1-8 palette, 9 white); `None` for 0 or unknown.
pub fn cue_colour(token: u8) -> Option<Rgb> {
    let (palette, white) = &*TOKEN_COLOURS;
    match token {
        1..=8 => palette.get(usize::from(token - 1)).copied().map(rgb),
        PAD_WHITE_TOKEN => Some(rgb(*white)),
        _ => None,
    }
}

/// Every component of `colour` divided by `divisor`, for resting pads.
pub const fn dimmed(colour: Rgb, divisor: u8) -> Rgb {
    Rgb::new(colour.r / divisor, colour.g / divisor, colour.b / divisor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_resolve_through_the_s4_djay_defaults() {
        assert_eq!(cue_colour(1), Some(rgb(Color::Red)));
        assert_eq!(cue_colour(3), Some(rgb(Color::Blue)));
        assert_eq!(cue_colour(PAD_WHITE_TOKEN), Some(rgb(Color::White)));
    }

    #[test]
    fn zero_and_unknown_tokens_have_no_colour() {
        assert_eq!(cue_colour(0), None);
        assert_eq!(cue_colour(10), None);
        assert_eq!(cue_colour(127), None);
    }

    #[test]
    fn every_colour_stays_within_the_report_range() {
        for token in 1..=PAD_WHITE_TOKEN {
            let Some(colour) = cue_colour(token) else {
                continue;
            };
            assert!(
                [colour.r, colour.g, colour.b]
                    .iter()
                    .all(|c| *c <= MAX_VALUE)
            );
            assert_ne!(colour, Rgb::OFF);
        }
    }

    #[test]
    fn dimming_divides_each_component() {
        assert_eq!(dimmed(Rgb::new(120, 60, 7), 6), Rgb::new(20, 10, 1));
    }
}
