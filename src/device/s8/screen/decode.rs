//! Applies S8 wire frames to a framebuffer per panel, so a picture can be
//! drawn from the bytes a tool sends instead of from the canvas they were
//! built from.
//!
//! The decoder reads what `begin`, `literal`, `repeat`, `skip` and `end`
//! write: a 16-byte header with the panel and the rectangle, commands in
//! pixel pairs filling the rectangle row by row, and an 8-byte footer. `skip`
//! and rectangles other than `Rect::FULL` follow the encoder's hypotheses;
//! the S8 has not shown them yet.

use std::fmt;

use super::{HEIGHT, Panel, Rect, WIDTH};

const HEADER: usize = 16;
const FOOTER: usize = 8;

/// A frame the decoder refuses, with the byte offset of the problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeError {
    pub offset: usize,
    pub reason: &'static str,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "malformed frame at byte {}: {}",
            self.offset, self.reason
        )
    }
}

impl std::error::Error for DecodeError {}

const fn fail<T>(offset: usize, reason: &'static str) -> Result<T, DecodeError> {
    Err(DecodeError { offset, reason })
}

/// One command of a frame.
enum Run<'a> {
    Literal(&'a [u8]),
    Repeat([u8; 4], usize),
    Skip(usize),
}

/// Writes pixels into a rectangle of a panel buffer, row by row.
struct Painter<'a> {
    buffer: &'a mut [u16],
    rect: Rect,
    cursor: usize,
}

impl Painter<'_> {
    fn put(&mut self, pixel: u16) {
        let width = usize::from(self.rect.width);
        let x = usize::from(self.rect.x) + self.cursor % width;
        let y = usize::from(self.rect.y) + self.cursor / width;
        self.buffer[y * WIDTH + x] = pixel;
        self.cursor += 1;
    }
}

/// Both panels as RGB565 values, all black at the start.
pub struct Panels {
    buffers: [Vec<u16>; 2],
}

impl Default for Panels {
    fn default() -> Self {
        Self {
            buffers: [vec![0; WIDTH * HEIGHT], vec![0; WIDTH * HEIGHT]],
        }
    }
}

impl Panels {
    /// Applies one frame to the panel its header names and returns that
    /// panel. A malformed frame changes nothing.
    ///
    /// # Errors
    /// The header, a command, the pair count or the footer is wrong.
    pub fn apply(&mut self, frame: &[u8]) -> Result<Panel, DecodeError> {
        let (panel, rect) = walk(frame, |_| {})?;
        let mut painter = Painter {
            buffer: &mut self.buffers[usize::from(panel.index())],
            rect,
            cursor: 0,
        };
        walk(frame, |run| match run {
            Run::Literal(bytes) => {
                for pixel in bytes.as_chunks::<2>().0 {
                    painter.put(u16::from_be_bytes(*pixel));
                }
            }
            Run::Repeat(pair, pairs) => {
                for _ in 0..pairs {
                    painter.put(u16::from_be_bytes([pair[0], pair[1]]));
                    painter.put(u16::from_be_bytes([pair[2], pair[3]]));
                }
            }
            Run::Skip(pairs) => painter.cursor += pairs * 2,
        })?;
        Ok(panel)
    }

    /// RGB565 values of `panel`, row-major from the top left.
    #[must_use]
    pub fn pixels(&self, panel: Panel) -> &[u16] {
        &self.buffers[usize::from(panel.index())]
    }

    /// `panel` as 8-bit RGB, 3 bytes per pixel, row-major.
    #[must_use]
    pub fn rgb888(&self, panel: Panel) -> Vec<u8> {
        let mut out = Vec::with_capacity(WIDTH * HEIGHT * 3);
        for &pixel in self.pixels(panel) {
            out.extend_from_slice(&[
                expand(pixel >> 11, 3, 2),
                expand((pixel >> 5) & 0x3f, 2, 4),
                expand(pixel & 0x1f, 3, 2),
            ]);
        }
        out
    }
}

/// Widens a 5- or 6-bit channel to 8 bits by repeating its top bits.
fn expand(value: u16, shift: u32, back: u32) -> u8 {
    u8::try_from(((value << shift) | (value >> back)) & 0xff).unwrap_or(u8::MAX)
}

/// Checks `frame` completely and hands each command to `sink`.
fn walk<'a>(frame: &'a [u8], mut sink: impl FnMut(Run<'a>)) -> Result<(Panel, Rect), DecodeError> {
    if frame.len() < HEADER + FOOTER {
        return fail(frame.len(), "shorter than header and footer");
    }
    if frame[..2] != [0x84, 0x00] || frame[3] != 0x60 || frame[4..8] != [0; 4] {
        return fail(0, "header is not 84 00 <panel> 60 00 00 00 00");
    }
    let panel = match frame[2] {
        0 => Panel::Left,
        1 => Panel::Right,
        _ => return fail(2, "panel byte is not 0 or 1"),
    };
    let field = |at: usize| u16::from_be_bytes([frame[at], frame[at + 1]]);
    let rect = Rect {
        x: field(8),
        y: field(10),
        width: field(12),
        height: field(14),
    };
    if rect.width == 0
        || rect.height == 0
        || usize::from(rect.x) + usize::from(rect.width) > WIDTH
        || usize::from(rect.y) + usize::from(rect.height) > HEIGHT
        || (usize::from(rect.width) * usize::from(rect.height)) % 2 != 0
    {
        return fail(8, "rectangle empty, odd or outside the panel");
    }
    let footer = frame.len() - FOOTER;
    if frame[footer..] != [0x03, 0x00, 0x00, 0x00, 0x40, 0x00, panel.index(), 0x00] {
        return fail(footer, "footer is not 03 00 00 00 40 00 <panel> 00");
    }

    let mut at = HEADER;
    let mut pairs = 0;
    while at < footer {
        if footer - at < 4 || frame[at + 1] != 0 {
            return fail(at, "truncated or malformed command");
        }
        let count = usize::from(u16::from_be_bytes([frame[at + 2], frame[at + 3]]));
        if count == 0 {
            return fail(at, "command covers no pixels");
        }
        let body = at + 4;
        let used = match frame[at] {
            0x00 => count * 4,
            0x01 => 4,
            0x02 => 0,
            _ => return fail(at, "opcode is not LITERAL, REPEAT or SKIP"),
        };
        if body + used > footer {
            return fail(at, "command data runs into the footer");
        }
        let data = &frame[body..body + used];
        sink(match frame[at] {
            0x00 => Run::Literal(data),
            0x01 => Run::Repeat([data[0], data[1], data[2], data[3]], count),
            _ => Run::Skip(count),
        });
        pairs += count;
        if pairs > rect.pairs() {
            return fail(at, "commands cover more pixels than the rectangle");
        }
        at = body + used;
    }
    if pairs != rect.pairs() {
        return fail(footer, "commands cover fewer pixels than the rectangle");
    }
    Ok((panel, rect))
}

#[cfg(test)]
mod tests {
    use super::super::pattern::{
        SUB_RECT, bands, canvas, partial_rect, partial_skip, pixel_buffer, stamp, wire,
    };
    use super::super::{begin, end, full, literal, repeat, skip, solid};
    use super::*;

    const RED: [u8; 4] = [0xf8, 0x00, 0xf8, 0x00];

    fn full_frame(panel: Panel, canvas: &[u16]) -> Vec<u8> {
        let mut bytes = pixel_buffer();
        wire(canvas, false, &mut bytes);
        let mut frame = Vec::new();
        full(&mut frame, panel, &bytes);
        frame
    }

    #[test]
    fn full_pattern_frame_decodes_to_the_canvas_it_was_built_from() {
        for panel in [Panel::Left, Panel::Right] {
            let mut source = canvas(panel);
            stamp(&mut source, 1234);
            let mut panels = Panels::default();
            assert_eq!(panels.apply(&full_frame(panel, &source)), Ok(panel));
            assert_eq!(panels.pixels(panel), &source[..]);
        }
    }

    #[test]
    fn frame_for_the_right_panel_leaves_the_left_buffer_alone() {
        let mut panels = Panels::default();
        let source = canvas(Panel::Right);
        assert!(panels.apply(&full_frame(Panel::Right, &source)).is_ok());
        assert!(panels.pixels(Panel::Left).iter().all(|&pixel| pixel == 0));
        assert_eq!(panels.pixels(Panel::Right), &source[..]);
    }

    #[test]
    fn repeat_fills_the_panel_and_skip_keeps_what_was_there() {
        let mut panels = Panels::default();
        let mut frame = Vec::new();
        solid(&mut frame, Panel::Left, RED);
        assert!(panels.apply(&frame).is_ok());
        assert!(
            panels
                .pixels(Panel::Left)
                .iter()
                .all(|&pixel| pixel == 0xf800)
        );

        partial_skip(&mut frame, Panel::Left);
        assert!(panels.apply(&frame).is_ok());
        let (top, bottom) = panels.pixels(Panel::Left).split_at(WIDTH * HEIGHT / 2);
        assert!(top.iter().all(|&pixel| pixel == 0xf800));
        let expected: Vec<u16> = bands(WIDTH, HEIGHT / 2)
            .iter()
            .flat_map(|pair| {
                [
                    u16::from_be_bytes([pair[0], pair[1]]),
                    u16::from_be_bytes([pair[2], pair[3]]),
                ]
            })
            .collect();
        assert_eq!(bottom, &expected[..]);
    }

    #[test]
    fn sub_rectangle_frame_changes_only_its_area() {
        let mut panels = Panels::default();
        let mut frame = Vec::new();
        solid(&mut frame, Panel::Right, RED);
        assert!(panels.apply(&frame).is_ok());
        partial_rect(&mut frame, Panel::Right);
        assert!(panels.apply(&frame).is_ok());

        let (x0, y0) = (usize::from(SUB_RECT.x), usize::from(SUB_RECT.y));
        let (width, height) = (usize::from(SUB_RECT.width), usize::from(SUB_RECT.height));
        let pixels = panels.pixels(Panel::Right);
        let mut changed = 0;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let inside = (x0..x0 + width).contains(&x) && (y0..y0 + height).contains(&y);
                let edge = x == x0 || y == y0 || x == x0 + width - 1 || y == y0 + height - 1;
                let pixel = pixels[y * WIDTH + x];
                match (inside, edge) {
                    (false, _) => assert_eq!(pixel, 0xf800, "({x},{y})"),
                    (true, true) => assert_eq!(pixel, 0xffff, "({x},{y})"),
                    (true, false) => changed += usize::from(pixel != 0xf800),
                }
            }
        }
        assert!(
            changed > 0,
            "the rectangle's bars never left the background colour"
        );
        assert!(panels.pixels(Panel::Left).iter().all(|&pixel| pixel == 0));
    }

    #[test]
    fn rgb888_widens_channels_to_the_full_range() {
        let mut panels = Panels::default();
        let mut frame = Vec::new();
        solid(&mut frame, Panel::Left, [0xf8, 0x00, 0x07, 0xe0]);
        assert!(panels.apply(&frame).is_ok());
        let rgb = panels.rgb888(Panel::Left);
        assert_eq!(rgb.len(), WIDTH * HEIGHT * 3);
        assert_eq!(rgb[..6], [255, 0, 0, 0, 255, 0]);
        solid(&mut frame, Panel::Left, [0xff; 4]);
        assert!(panels.apply(&frame).is_ok());
        assert_eq!(panels.rgb888(Panel::Left)[..3], [255, 255, 255]);
    }

    #[test]
    fn malformed_frames_are_rejected_and_change_nothing() {
        let mut good = Vec::new();
        solid(&mut good, Panel::Left, RED);
        let mut short = Vec::new();
        begin(&mut short, Panel::Left, Rect::FULL);
        repeat(&mut short, RED, 0xfe00);
        end(&mut short, Panel::Left);
        let mut long = Vec::new();
        begin(&mut long, Panel::Left, Rect::FULL);
        literal(&mut long, &vec![[0; 4]; 0xff01]);
        end(&mut long, Panel::Left);
        let mut outside = Vec::new();
        begin(&mut outside, Panel::Left, Rect { x: 400, ..SUB_RECT });
        skip(&mut outside, 0x1900);
        end(&mut outside, Panel::Left);

        let mut bad_footer = good.clone();
        let last = bad_footer.len() - 1;
        bad_footer[last] = 1;
        let mut bad_opcode = good.clone();
        bad_opcode[16] = 0x07;
        let mut bad_panel = good.clone();
        bad_panel[2] = 2;
        let mut cut = good.clone();
        cut.truncate(good.len() - 10);
        let mut cut_literal = Vec::new();
        begin(&mut cut_literal, Panel::Left, Rect::FULL);
        literal(&mut cut_literal, &[[1; 4]; 4]);
        cut_literal.truncate(cut_literal.len() - 2);
        end(&mut cut_literal, Panel::Left);

        let mut panels = Panels::default();
        for (name, frame) in [
            ("empty", Vec::new()),
            ("footer", bad_footer),
            ("opcode", bad_opcode),
            ("panel", bad_panel),
            ("cut", cut),
            ("short", short),
            ("long", long),
            ("outside", outside),
            ("cut literal", cut_literal),
        ] {
            assert!(panels.apply(&frame).is_err(), "{name}");
        }
        assert!(panels.pixels(Panel::Left).iter().all(|&pixel| pixel == 0));
        assert!(panels.apply(&good).is_ok());
    }
}
