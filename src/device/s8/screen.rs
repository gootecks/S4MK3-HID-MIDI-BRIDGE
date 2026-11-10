//! Kontrol S8 screen frames for bulk OUT `0x04` on interface 6.
//!
//! A frame is a 16-byte header, run-length commands counted in pixel pairs,
//! and an 8-byte footer. Pixels are RGB565, big-endian. Full-panel `literal`
//! and `repeat` frames are checked on hardware
//! (`docs/s8/screen-frame-validation.md`). `skip` and any `Rect` other than
//! `Rect::FULL` are not: the position of `x` and `y` in the header is taken
//! from the Maschine MK3 display family and no S8 capture shows it.

pub mod decode;
pub mod pattern;
pub mod stats;

pub const INTERFACE: u8 = 6;
pub const ENDPOINT: u8 = 0x04;
pub const WIDTH: usize = 480;
pub const HEIGHT: usize = 272;
pub const PIXEL_BYTES: usize = WIDTH * HEIGHT * 2;
pub const FRAME_BYTES: usize = 16 + 4 + PIXEL_BYTES + 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Left,
    Right,
}

impl Panel {
    #[must_use]
    pub const fn index(self) -> u8 {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const FULL: Self = Self {
        x: 0,
        y: 0,
        width: 480,
        height: 272,
    };

    /// Pixel pairs the commands of a frame for this area must cover.
    #[must_use]
    pub fn pairs(self) -> usize {
        usize::from(self.width) * usize::from(self.height) / 2
    }
}

/// Starts a frame in `frame`, replacing its contents.
pub fn begin(frame: &mut Vec<u8>, panel: Panel, rect: Rect) {
    frame.clear();
    frame.extend_from_slice(&[0x84, 0x00, panel.index(), 0x60, 0, 0, 0, 0]);
    for field in [rect.x, rect.y, rect.width, rect.height] {
        frame.extend_from_slice(&field.to_be_bytes());
    }
}

/// Sends `pairs` as they are: two big-endian RGB565 pixels each.
pub fn literal(frame: &mut Vec<u8>, pairs: &[[u8; 4]]) {
    for run in pairs.chunks(usize::from(u16::MAX)) {
        command(frame, 0x00, u16::try_from(run.len()).unwrap_or(u16::MAX));
        frame.extend_from_slice(run.as_flattened());
    }
}

/// Fills the next `pairs` pixel pairs with `pair`.
pub fn repeat(frame: &mut Vec<u8>, pair: [u8; 4], pairs: u16) {
    command(frame, 0x01, pairs);
    frame.extend_from_slice(&pair);
}

/// Leaves the next `pairs` pixel pairs as the panel shows them.
pub fn skip(frame: &mut Vec<u8>, pairs: u16) {
    command(frame, 0x02, pairs);
}

pub fn end(frame: &mut Vec<u8>, panel: Panel) {
    frame.extend_from_slice(&[0x03, 0x00, 0x00, 0x00, 0x40, 0x00, panel.index(), 0x00]);
}

/// Full uncompressed frame of `FRAME_BYTES` bytes from big-endian RGB565.
pub fn full(frame: &mut Vec<u8>, panel: Panel, pixels: &[u8; PIXEL_BYTES]) {
    begin(frame, panel, Rect::FULL);
    literal(frame, pixels.as_chunks::<4>().0);
    end(frame, panel);
}

/// Full-panel frame of one repeated pixel pair, 32 bytes.
pub fn solid(frame: &mut Vec<u8>, panel: Panel, pair: [u8; 4]) {
    begin(frame, panel, Rect::FULL);
    repeat(frame, pair, 0xff00);
    end(frame, panel);
}

fn command(frame: &mut Vec<u8>, opcode: u8, pairs: u16) {
    let [high, low] = pairs.to_be_bytes();
    frame.extend_from_slice(&[opcode, 0x00, high, low]);
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER_LEFT: [u8; 16] = [
        0x84, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xe0, 0x01,
        0x10,
    ];

    fn recorded(screen: u8, pair: [u8; 4]) -> Vec<u8> {
        let mut bytes = HEADER_LEFT.to_vec();
        bytes[2] = screen;
        bytes.extend_from_slice(&[0x01, 0x00, 0xff, 0x00]);
        bytes.extend_from_slice(&pair);
        bytes.extend_from_slice(&[0x03, 0x00, 0x00, 0x00, 0x40, 0x00, screen, 0x00]);
        bytes
    }

    #[test]
    fn solid_frames_match_the_frames_shown_in_phase_2() {
        let mut frame = Vec::new();
        for (panel, screen) in [(Panel::Left, 0), (Panel::Right, 1)] {
            for pair in [[0; 4], [0xf8, 0x00, 0xf8, 0x00]] {
                solid(&mut frame, panel, pair);
                assert_eq!(frame, recorded(screen, pair));
            }
        }
    }

    #[test]
    fn full_frame_is_one_literal_command_over_the_whole_panel() {
        let mut pixels = vec![0_u8; PIXEL_BYTES];
        pixels[0] = 0xf8;
        pixels[PIXEL_BYTES - 1] = 0x1f;
        let mut frame = vec![0xaa; 3];
        full(&mut frame, Panel::Right, &pixels.as_chunks().0[0]);

        assert_eq!(frame.len(), FRAME_BYTES);
        assert_eq!(Rect::FULL.pairs(), 0xff00);
        assert_eq!(frame[..16], recorded(1, [0; 4])[..16]);
        assert_eq!(frame[16..20], [0x00, 0x00, 0xff, 0x00]);
        assert_eq!(frame[20..FRAME_BYTES - 8], pixels[..]);
        assert_eq!(
            frame[FRAME_BYTES - 8..],
            [0x03, 0x00, 0x00, 0x00, 0x40, 0x00, 0x01, 0x00]
        );
    }

    #[test]
    fn partial_frame_carries_its_area_and_mixed_runs() {
        let rect = Rect {
            x: 160,
            y: 96,
            width: 160,
            height: 80,
        };
        let mut frame = Vec::new();
        begin(&mut frame, Panel::Left, rect);
        skip(&mut frame, 0x1000);
        repeat(&mut frame, [0x07, 0xe0, 0x07, 0xe0], 0x0800);
        literal(&mut frame, &[[0x00, 0x1f, 0xff, 0xff]; 2]);
        end(&mut frame, Panel::Left);

        assert_eq!(rect.pairs(), 0x1900);
        assert_eq!(
            frame[..16],
            [
                0x84, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0xa0, 0x00, 0x60, 0x00, 0xa0,
                0x00, 0x50
            ]
        );
        assert_eq!(
            frame[16..],
            [
                0x02, 0x00, 0x10, 0x00, 0x01, 0x00, 0x08, 0x00, 0x07, 0xe0, 0x07, 0xe0, 0x00, 0x00,
                0x00, 0x02, 0x00, 0x1f, 0xff, 0xff, 0x00, 0x1f, 0xff, 0xff, 0x03, 0x00, 0x00, 0x00,
                0x40, 0x00, 0x00, 0x00
            ]
        );
    }
}
