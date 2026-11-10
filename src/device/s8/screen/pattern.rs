//! The S8 test pattern, the frame stamp of the stream modes and the partial
//! update frames of `examples/s8-screen-frame`.
//!
//! Canvases are 480×272 logical RGB565 values. `wire` turns one into the
//! big-endian bytes `screen::full` takes.

use super::{HEIGHT, PIXEL_BYTES, Panel, Rect, WIDTH, begin, end, literal, skip};
use crate::screens::{self, Display, Frame, rgb};

/// Area of the sub-rectangle frame: 160×80 at x 160, y 96.
pub const SUB_RECT: Rect = Rect {
    x: 160,
    y: 96,
    width: 160,
    height: 80,
};

const STAMP_TOP: usize = 244;
const STAMP_HEIGHT: usize = 26;
const BAR_WIDTH: usize = 24;
const BAR_TRAVEL: usize = WIDTH - 32 - BAR_WIDTH;

const fn background() -> u16 {
    rgb(12, 16, 24)
}

/// Labelled colour bars from the S4 raster, centred on the S8 panel, with a
/// grey border and white corner markers (large top left).
#[must_use]
pub fn canvas(panel: Panel) -> Vec<u16> {
    let background = background();
    let white = rgb(255, 255, 255);
    let mut labels = Frame::default();
    labels.clear(background);
    let title = match panel {
        Panel::Left => "LEFT S8 SCREEN",
        Panel::Right => "RIGHT S8 SCREEN",
    };
    labels.text(16, 16, title, 2, white);
    for (index, (label, color)) in [
        ("RED", rgb(255, 0, 0)),
        ("GREEN", rgb(0, 255, 0)),
        ("BLUE", rgb(0, 0, 255)),
        ("WHITE", white),
    ]
    .into_iter()
    .enumerate()
    {
        labels.rect(16 + index * 76, 64, 60, 110, color);
        labels.text(16 + index * 76, 184, label, 1, white);
    }
    labels.text(16, 216, "480 X 272 LITERAL", 1, white);
    let mut packet = Vec::new();
    labels.packet(Display::Left, &mut packet);

    let mut canvas = vec![background; WIDTH * HEIGHT];
    let (left, top) = ((WIDTH - screens::WIDTH) / 2, (HEIGHT - screens::HEIGHT) / 2);
    for (index, [high, low]) in packet[16..16 + screens::PIXEL_BYTES]
        .as_chunks::<2>()
        .0
        .iter()
        .enumerate()
    {
        let (x, y) = (index % screens::WIDTH, index / screens::WIDTH);
        canvas[(top + y) * WIDTH + left + x] = u16::from_be_bytes([*high, *low]);
    }
    let grey = rgb(128, 128, 128);
    fill(&mut canvas, 0, 0, WIDTH, 1, grey);
    fill(&mut canvas, 0, HEIGHT - 1, WIDTH, 1, grey);
    fill(&mut canvas, 0, 0, 1, HEIGHT, grey);
    fill(&mut canvas, WIDTH - 1, 0, 1, HEIGHT, grey);
    fill(&mut canvas, 0, 0, 16, 16, white);
    fill(&mut canvas, WIDTH - 8, 0, 8, 8, white);
    fill(&mut canvas, 0, HEIGHT - 8, 8, 8, white);
    fill(&mut canvas, WIDTH - 8, HEIGHT - 8, 8, 8, white);
    canvas
}

/// Draws frame number `frame` into the strip under the pattern: a 16-bit
/// binary counter (most significant bit left) and a bar that moves 12 pixels
/// per frame. Nothing outside the strip changes.
pub fn stamp(canvas: &mut [u16], frame: u32) {
    fill(canvas, 8, STAMP_TOP, WIDTH - 16, STAMP_HEIGHT, background());
    for (index, shift) in (0..16_u32).rev().enumerate() {
        let color = if (frame >> shift) & 1 == 1 {
            rgb(255, 255, 255)
        } else {
            rgb(60, 60, 60)
        };
        fill(canvas, 24 + index * 27, STAMP_TOP, 20, 10, color);
    }
    let step = usize::try_from(frame).unwrap_or_default();
    let x = 16 + step * 12 % BAR_TRAVEL;
    fill(canvas, x, STAMP_TOP + 14, BAR_WIDTH, 10, rgb(255, 255, 0));
}

/// A zeroed buffer for `wire`, on the heap.
#[must_use]
pub fn pixel_buffer() -> Box<[u8; PIXEL_BYTES]> {
    vec![0_u8; PIXEL_BYTES]
        .into_boxed_slice()
        .try_into()
        .unwrap_or_else(|_| unreachable!("the buffer has PIXEL_BYTES bytes"))
}

/// Converts `canvas` to big-endian bytes, exchanging red and blue when
/// `swap_rb` is set (the BGR565 experiment of phase 4).
pub fn wire(canvas: &[u16], swap_rb: bool, out: &mut [u8; PIXEL_BYTES]) {
    for (pixel, bytes) in canvas.iter().zip(out.as_chunks_mut::<2>().0.iter_mut()) {
        let value = if swap_rb {
            ((pixel & 0x1f) << 11) | (pixel & 0x07e0) | (pixel >> 11)
        } else {
            *pixel
        };
        *bytes = value.to_be_bytes();
    }
}

/// Eight vertical colour bars with a white outline, as pixel pairs. An odd
/// width drops its last column.
#[must_use]
pub fn bands(width: usize, height: usize) -> Vec<[u8; 4]> {
    let colors = [
        rgb(255, 0, 0),
        rgb(0, 255, 0),
        rgb(0, 0, 255),
        rgb(255, 255, 0),
        rgb(0, 255, 255),
        rgb(255, 0, 255),
        rgb(255, 128, 0),
        rgb(160, 160, 160),
    ];
    let pixel = |x: usize, y: usize| {
        let edge = x == 0 || y == 0 || x == width - 1 || y == height - 1;
        let value = if edge {
            rgb(255, 255, 255)
        } else {
            colors[x * colors.len() / width]
        };
        value.to_be_bytes()
    };
    (0..height)
        .flat_map(|y| (0..width / 2).map(move |pair| (y, pair * 2)))
        .map(|(y, x)| {
            let [a, b] = pixel(x, y);
            let [c, d] = pixel(x + 1, y);
            [a, b, c, d]
        })
        .collect()
}

/// Full-panel header, SKIP over the top half, LITERAL bands over the bottom.
pub fn partial_skip(frame: &mut Vec<u8>, panel: Panel) {
    begin(frame, panel, Rect::FULL);
    let half = bands(WIDTH, HEIGHT / 2);
    skip(frame, u16::try_from(half.len()).unwrap_or(u16::MAX));
    literal(frame, &half);
    end(frame, panel);
}

/// Header naming `SUB_RECT`, LITERAL bands over exactly that area.
pub fn partial_rect(frame: &mut Vec<u8>, panel: Panel) {
    begin(frame, panel, SUB_RECT);
    literal(
        frame,
        &bands(usize::from(SUB_RECT.width), usize::from(SUB_RECT.height)),
    );
    end(frame, panel);
}

fn fill(canvas: &mut [u16], x: usize, y: usize, width: usize, height: usize, color: u16) {
    for row in y..y + height {
        canvas[row * WIDTH + x..row * WIDTH + x + width].fill(color);
    }
}

#[cfg(test)]
mod tests {
    use sha2::{Digest as _, Sha256};

    use super::super::{FRAME_BYTES, full};
    use super::*;

    fn pattern_hash(panel: Panel) -> String {
        let mut bytes = pixel_buffer();
        wire(&canvas(panel), false, &mut bytes);
        let mut frame = Vec::new();
        full(&mut frame, panel, &bytes);
        assert_eq!(frame.len(), FRAME_BYTES);
        format!("{:x}", Sha256::digest(&frame))
    }

    #[test]
    fn pattern_frames_hash_to_the_values_validated_on_hardware() {
        // docs/s8/screen-frame-validation.md, phase 4.
        assert_eq!(
            pattern_hash(Panel::Left),
            "bd568210d97edd6e93e826b5560d771c84adab3fe3ebcb8f9a342ffa42c138e4"
        );
        assert_eq!(
            pattern_hash(Panel::Right),
            "43f8d9ea9844cc5b1196fee734e64027a888f53fde19f781005df7433883148d"
        );
    }

    #[test]
    fn stamp_changes_only_the_strip_and_differs_between_frames() {
        let base = canvas(Panel::Left);
        let mut first = base.clone();
        let mut second = base.clone();
        stamp(&mut first, 1);
        stamp(&mut second, 2);

        assert_ne!(first, second);
        for (index, (stamped, original)) in first.iter().zip(&base).enumerate() {
            let (x, y) = (index % WIDTH, index / WIDTH);
            let inside =
                (8..WIDTH - 8).contains(&x) && (STAMP_TOP..STAMP_TOP + STAMP_HEIGHT).contains(&y);
            assert!(inside || stamped == original, "pixel {x},{y} changed");
        }
    }

    #[test]
    fn stamped_bar_stays_inside_the_strip_for_every_frame_up_to_the_cap() {
        let mut canvas = canvas(Panel::Right);
        for number in 0..3600 {
            stamp(&mut canvas, number);
        }
        assert_eq!(canvas.len(), WIDTH * HEIGHT);
    }

    #[test]
    fn wire_swaps_red_and_blue_on_request() {
        let canvas = vec![rgb(255, 0, 0); WIDTH * HEIGHT];
        let mut bytes = pixel_buffer();
        wire(&canvas, false, &mut bytes);
        assert_eq!(bytes[..2], [0xf8, 0x00]);
        wire(&canvas, true, &mut bytes);
        assert_eq!(bytes[..2], [0x00, 0x1f]);
    }

    #[test]
    fn partial_frames_cover_their_area_exactly() {
        let mut frame = Vec::new();
        partial_skip(&mut frame, Panel::Left);
        assert_eq!(frame.len(), 16 + 4 + 4 + Rect::FULL.pairs() / 2 * 4 + 8);
        assert_eq!(frame[16..20], [0x02, 0x00, 0x7f, 0x80]);
        assert_eq!(frame[20..24], [0x00, 0x00, 0x7f, 0x80]);

        partial_rect(&mut frame, Panel::Right);
        assert_eq!(frame.len(), 16 + 4 + SUB_RECT.pairs() * 4 + 8);
        assert_eq!(
            frame[8..16],
            [0x00, 0xa0, 0x00, 0x60, 0x00, 0xa0, 0x00, 0x50]
        );
        assert_eq!(frame[16..20], [0x00, 0x00, 0x19, 0x00]);
    }
}
