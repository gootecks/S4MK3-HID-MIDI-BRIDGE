//! Four-quadrant live-data page, independent of MIDI and USB.

use super::{Frame, HEIGHT, WIDTH, rgb};

const BACKGROUND: u16 = rgb(10, 12, 18);
const PRIMARY: u16 = rgb(244, 248, 255);
const SECONDARY: u16 = rgb(164, 178, 202);
const AMBER: u16 = rgb(252, 176, 36);
const GREEN: u16 = rgb(0, 220, 80);
const BLACK: u16 = 0;
const DIVIDER: u16 = rgb(60, 70, 90);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Info {
    pub jump: Option<&'static str>,
    pub loop_active: Option<bool>,
    pub loop_size: Option<&'static str>,
    pub key: Option<&'static str>,
    pub tempo_range: Option<u8>,
    pub selecting: bool,
}

pub(super) fn render(info: &Info, frame: &mut Frame, bright: bool) {
    let range = match info.tempo_range {
        Some(4) => "±4%",
        Some(6) => "±6%",
        Some(8) => "±8%",
        Some(10) => "±10%",
        Some(16) => "±16%",
        Some(25) => "±25%",
        Some(50) => "±50%",
        Some(75) => "±75%",
        Some(_) | None => "?",
    };
    let loop_value = match info.loop_active {
        Some(true) => info.loop_size.unwrap_or("?"),
        Some(false) => "OFF",
        None => "?",
    };
    for (index, (label, value)) in [
        ("JUMP", info.jump.unwrap_or("?")),
        ("LOOP", loop_value),
        ("KEY", info.key.unwrap_or("?")),
        ("RANGE", range),
    ]
    .into_iter()
    .enumerate()
    {
        let x = (index % 2) * (WIDTH / 2);
        let y = (index / 2) * (HEIGHT / 2);
        let selecting = index == 0 && info.selecting;
        let fill = if index == 1 && info.loop_active == Some(true) {
            GREEN
        } else if selecting && bright {
            AMBER
        } else {
            BACKGROUND
        };
        let ink = if fill == BACKGROUND { PRIMARY } else { BLACK };
        frame.rect(x, y, WIDTH / 2, HEIGHT / 2, fill);
        frame.text(
            x + 12,
            y + 12,
            label,
            2,
            if fill == BACKGROUND { SECONDARY } else { BLACK },
        );
        let scale = [6, 5, 4]
            .into_iter()
            .find(|&scale| Frame::text_width(value, scale) <= 136)
            .unwrap_or(4);
        frame.centered_text(
            x + WIDTH / 4,
            y + 44,
            value,
            scale,
            if index == 1 && info.loop_active == Some(false) {
                SECONDARY
            } else {
                ink
            },
        );
        if selecting {
            frame.centered_text(
                x + WIDTH / 4,
                y + 100,
                "SELECT",
                2,
                if bright { BLACK } else { AMBER },
            );
        }
    }
    frame.rect(WIDTH / 2, 0, 1, HEIGHT, DIVIDER);
    frame.rect(0, HEIGHT / 2, WIDTH, 1, DIVIDER);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(info: Info, bright: bool) -> Frame {
        let mut frame = Frame::default();
        render(&info, &mut frame, bright);
        frame
    }

    #[test]
    fn active_loop_has_green_fill_and_black_ink_only_in_its_quadrant() {
        // Given a confirmed active loop.
        let info = Info {
            loop_active: Some(true),
            loop_size: Some("8"),
            ..Info::default()
        };
        // When drawn at native resolution.
        let actual = frame(info, true);
        // Then green fills its quadrant and its label/value have black pixels.
        assert_eq!(actual.pixels[8 * WIDTH + 168], GREEN);
        assert!(actual.pixels[12 * WIDTH + 172..28 * WIDTH].contains(&BLACK));
        for (index, &pixel) in actual.pixels.iter().enumerate() {
            if pixel == GREEN {
                assert!(index % WIDTH > 160 && index / WIDTH < 120);
            }
        }
    }

    #[test]
    fn inactive_loop_never_shows_a_cached_size() {
        // Given an inactive loop with different old sizes.
        let info = Info {
            loop_active: Some(false),
            loop_size: Some("8"),
            ..Info::default()
        };
        // When rendered with stale size changed or absent.
        let actual = frame(info, true);
        // Then all OFF rasters are identical, while unknown and active differ.
        assert_eq!(
            actual.pixels,
            frame(
                Info {
                    loop_size: None,
                    ..info
                },
                true
            )
            .pixels
        );
        assert_ne!(actual.pixels, frame(Info::default(), true).pixels);
        assert!(!actual.pixels.contains(&GREEN));
    }

    #[test]
    fn selection_phases_preserve_number_geometry_and_other_quadrants() {
        // Given jump-size selection with known data.
        let info = Info {
            jump: Some("16"),
            selecting: true,
            ..Info::default()
        };
        // When both flash phases render.
        let bright = frame(info, true);
        let dim = frame(info, false);
        // Then the same number pixels remain visible; only JUMP changes.
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = y * WIDTH + x;
                if x >= 160 || y >= 120 {
                    assert_eq!(bright.pixels[index], dim.pixels[index]);
                } else if (44..92).contains(&y) {
                    assert_eq!(
                        bright.pixels[index] != AMBER,
                        dim.pixels[index] != BACKGROUND
                    );
                }
            }
        }
        assert_ne!(bright.pixels, dim.pixels);
    }

    #[test]
    fn long_values_and_plus_minus_stay_inside_their_quadrants() {
        // Given the longest fractional lengths, key and range.
        let info = Info {
            jump: Some("1/32"),
            loop_active: Some(true),
            loop_size: Some("1/16"),
            key: Some("12B"),
            tempo_range: Some(75),
            ..Info::default()
        };
        // When rendering into a native frame.
        let actual = frame(info, true);
        // Then margins remain clear around every value, and ± is real ink.
        for (x, y) in [(0, 0), (160, 0), (0, 120), (160, 120)] {
            let fill = if x == 160 && y == 0 {
                GREEN
            } else {
                BACKGROUND
            };
            for row in y + 44..y + 92 {
                assert_eq!(actual.pixels[row * WIDTH + x + 4], fill);
                assert_eq!(actual.pixels[row * WIDTH + x + 156], fill);
            }
        }
        assert!(
            (164..196).any(|y| actual.pixels[y * WIDTH + 176..y * WIDTH + 208].contains(&PRIMARY))
        );
    }
}
