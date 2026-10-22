//! Measured, pre-rendered sans-serif text with RGB565 coverage compositing.

use super::{Frame, HEIGHT, WIDTH};

mod atlas;

const CHARACTERS: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789±%/?.-";

struct Glyph {
    advance: usize,
    left: i16,
    top: i16,
    width: usize,
    // A-P encodes 0-15 coverage; masks already have their final pixel size.
    mask: &'static [u8],
}

fn glyph(character: char, scale: usize) -> Option<&'static Glyph> {
    let index = CHARACTERS.chars().position(|item| item == character)?;
    atlas::SIZES
        .get(scale.checked_sub(1)?)
        .map(|size| &size[index])
}

impl Frame {
    /// Advance width of embedded text at a fixed 8-48 pixel font size (scale 1-6).
    #[must_use]
    pub fn text_width(text: &str, scale: usize) -> usize {
        text.chars()
            .map(|character| glyph(character, scale).map_or(0, |item| item.advance))
            .sum()
    }

    pub fn centered_text(
        &mut self,
        center_x: usize,
        y: usize,
        text: &str,
        scale: usize,
        color: u16,
    ) {
        self.text(
            center_x.saturating_sub(Self::text_width(text, scale) / 2),
            y,
            text,
            scale,
            color,
        );
    }

    /// Composite native-size glyphs; y is the common capital-letter top.
    pub fn text(&mut self, mut x: usize, y: usize, text: &str, scale: usize, color: u16) {
        for character in text.chars() {
            let Some(glyph) = glyph(character, scale) else {
                continue;
            };
            for (row, coverage) in glyph.mask.chunks(glyph.width.max(1)).enumerate() {
                let Some(target_y) = (y + row).checked_add_signed(isize::from(glyph.top)) else {
                    continue;
                };
                if target_y >= HEIGHT {
                    continue;
                }
                for (column, alpha) in coverage.iter().enumerate() {
                    let Some(target_x) = (x + column).checked_add_signed(isize::from(glyph.left))
                    else {
                        continue;
                    };
                    if target_x < WIDTH && *alpha != b'A' {
                        let pixel = &mut self.pixels[target_y * WIDTH + target_x];
                        *pixel = blend(*pixel, color, u16::from(*alpha - b'A'));
                    }
                }
            }
            x += glyph.advance;
        }
    }
}

fn blend(background: u16, foreground: u16, coverage: u16) -> u16 {
    let channel = |shift: u32, mask: u16| {
        (((background >> shift) & mask) * (15 - coverage)
            + ((foreground >> shift) & mask) * coverage
            + 7)
            / 15
    };
    (channel(11, 31) << 11) | (channel(5, 63) << 5) | channel(0, 31)
}

#[cfg(test)]
mod tests;
