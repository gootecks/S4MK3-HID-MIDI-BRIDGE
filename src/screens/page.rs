//! Live-data quadrants and a reactive physical STEMS legend.

pub use super::info::Info;
use super::{Frame, HEIGHT, WIDTH, rgb};

const BACKGROUND: u16 = rgb(10, 12, 18);
const PANEL: u16 = rgb(24, 30, 42);
const BORDER: u16 = rgb(96, 108, 128);
const PRIMARY: u16 = rgb(244, 248, 255);
const SECONDARY: u16 = rgb(164, 178, 202);
const PRESSED: u16 = rgb(252, 176, 36);
const SCALE: usize = 2;
const MARGIN: usize = 8;
const GAP: usize = 4;
const PAD_WIDTH: usize = (WIDTH - 2 * MARGIN - 3 * GAP) / 4;
const PADS_Y: usize = 40;
const PAD_HEIGHT: usize = (HEIGHT - MARGIN - PADS_Y - GAP) / 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadState {
    pub active: Option<bool>,
    pub held: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Empty,
    Info(Info),
    Stems { shifted: bool, pads: [PadState; 8] },
}

impl Page {
    #[must_use]
    pub const fn selecting(&self) -> bool {
        match self {
            Self::Info(info) => info.selecting,
            Self::Empty | Self::Stems { .. } => false,
        }
    }
}

pub fn render(page: &Page, frame: &mut Frame) {
    render_phase(page, frame, true);
}

pub fn render_phase(page: &Page, frame: &mut Frame, bright: bool) {
    frame.clear(BACKGROUND);
    let (shifted, pads) = match page {
        Page::Empty => return,
        Page::Info(info) => return super::info::render(info, frame, bright),
        Page::Stems { shifted, pads } => (shifted, pads),
    };
    for (column, name) in ["DRUM", "BASS", "HARM", "VOX"].into_iter().enumerate() {
        let x = MARGIN + column * (PAD_WIDTH + GAP);
        frame.centered_text(x + PAD_WIDTH / 2, 12, name, SCALE, PRIMARY);
    }
    for (index, pad) in pads.iter().enumerate() {
        let x = MARGIN + (index % 4) * (PAD_WIDTH + GAP);
        let y = PADS_Y + (index / 4) * (PAD_HEIGHT + GAP);
        let (fill, ink, status) = match pad.active {
            Some(true) => (PRIMARY, BACKGROUND, "ON"),
            Some(false) => (PANEL, PRIMARY, "OFF"),
            None => (PANEL, SECONDARY, "?"),
        };
        let border = if pad.held { 3 } else { 1 };
        frame.rect(
            x,
            y,
            PAD_WIDTH,
            PAD_HEIGHT,
            if pad.held { PRESSED } else { BORDER },
        );
        frame.rect(
            x + border,
            y + border,
            PAD_WIDTH - 2 * border,
            PAD_HEIGHT - 2 * border,
            BACKGROUND,
        );
        frame.rect(
            x + border + 1,
            y + border + 1,
            PAD_WIDTH - 2 * (border + 1),
            PAD_HEIGHT - 2 * (border + 1),
            fill,
        );
        let action = if index >= 4 {
            "SOLO"
        } else if *shifted {
            "ONLY"
        } else {
            "MUTE"
        };
        frame.centered_text(x + PAD_WIDTH / 2, y + 16, action, SCALE, ink);
        frame.centered_text(x + PAD_WIDTH / 2, y + 56, status, SCALE, ink);
    }
}

pub fn stopped(frame: &mut Frame) {
    frame.clear(BACKGROUND);
    for (y, text, color) in [
        (68, "BRIDGE", PRIMARY),
        (100, "STOPPED", PRIMARY),
        (148, "START TO", SECONDARY),
        (172, "RESUME", SECONDARY),
    ] {
        frame.centered_text(WIDTH / 2, y, text, SCALE, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(active: Option<bool>) -> Page {
        Page::Stems {
            shifted: false,
            pads: [PadState {
                active,
                held: false,
            }; 8],
        }
    }

    #[test]
    fn confirmed_activation_inverts_each_tile_without_overwriting_gutters() {
        // Given every pad confirmed active.
        let mut frame = Frame::default();
        // When the full-screen STEMS legend is rendered.
        render(&page(Some(true)), &mut frame);
        // Then all eight tile interiors are inverted within their physical slots.
        for index in 0..8 {
            let x = MARGIN + (index % 4) * (PAD_WIDTH + GAP);
            let y = PADS_Y + (index / 4) * (PAD_HEIGHT + GAP);
            assert_eq!(frame.pixels[(y + 8) * WIDTH + x + 8], PRIMARY);
        }
        for col in 0..3 {
            let x = MARGIN + col * (PAD_WIDTH + GAP) + PAD_WIDTH;
            for row in 0..HEIGHT {
                assert!(
                    frame.pixels[row * WIDTH + x..row * WIDTH + x + GAP]
                        .iter()
                        .all(|&pixel| pixel == BACKGROUND)
                );
            }
        }
    }

    #[test]
    fn unknown_state_is_visibly_different_from_confirmed_off() {
        // Given no feedback versus a real off snapshot.
        let mut unknown = Frame::default();
        let mut off = Frame::default();
        // When both are rendered.
        render(&page(None), &mut unknown);
        render(&page(Some(false)), &mut off);
        // Then the status pixels differ without requiring a color distinction.
        assert_ne!(unknown.pixels, off.pixels);
        assert_eq!(unknown.pixels[(PADS_Y + 8) * WIDTH + MARGIN + 8], PANEL);
        assert_eq!(off.pixels[(PADS_Y + 8) * WIDTH + MARGIN + 8], PANEL);
        let status_pixels: Vec<_> = (PADS_Y + 56..PADS_Y + 72)
            .flat_map(|y| (MARGIN..MARGIN + PAD_WIDTH).map(move |x| y * WIDTH + x))
            .collect();
        assert_ne!(
            status_pixels
                .iter()
                .map(|&i| unknown.pixels[i] == SECONDARY)
                .collect::<Vec<_>>(),
            status_pixels
                .iter()
                .map(|&i| off.pixels[i] == PRIMARY)
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn held_outline_does_not_change_confirmed_fill_or_other_tiles() {
        // Given an active pad, with and without its physical press.
        let mut held = [PadState {
            active: Some(true),
            held: false,
        }; 8];
        held[0].held = true;
        let mut normal_frame = Frame::default();
        let mut held_frame = Frame::default();
        // When the physical press is rendered.
        render(&page(Some(true)), &mut normal_frame);
        render(
            &Page::Stems {
                shifted: false,
                pads: held,
            },
            &mut held_frame,
        );
        // Then only that tile changes and its confirmed interior stays active.
        assert_ne!(normal_frame.pixels, held_frame.pixels);
        assert_eq!(
            held_frame.pixels[(PADS_Y + 8) * WIDTH + MARGIN + 8],
            PRIMARY
        );
        for (index, (&normal, &pressed)) in normal_frame
            .pixels
            .iter()
            .zip(&held_frame.pixels)
            .enumerate()
        {
            if !(MARGIN..MARGIN + PAD_WIDTH).contains(&(index % WIDTH))
                || !(PADS_Y..PADS_Y + PAD_HEIGHT).contains(&(index / WIDTH))
            {
                assert_eq!(normal, pressed);
            }
        }
    }

    #[test]
    fn quiet_page_erases_all_previous_pad_content() {
        // Given a reused STEMS frame.
        let mut frame = Frame::default();
        render(&page(Some(true)), &mut frame);
        // When a quiet page replaces it.
        render(&Page::Empty, &mut frame);
        // Then no old state or legend survives.
        assert!(frame.pixels.iter().all(|&pixel| pixel == BACKGROUND));
    }

    #[test]
    fn stopped_replaces_every_previous_role_pixel() {
        // Given an active frame and a fresh frame.
        let mut reused = Frame::default();
        render(&page(Some(true)), &mut reused);
        let mut fresh = Frame::default();
        // When both receive the stopped page.
        stopped(&mut reused);
        stopped(&mut fresh);
        // Then stopped output is independent of prior roles and visibly populated.
        assert_eq!(reused.pixels, fresh.pixels);
        assert!(reused.pixels.contains(&PRIMARY));
        assert!(!reused.pixels.contains(&PRESSED));
    }
}
