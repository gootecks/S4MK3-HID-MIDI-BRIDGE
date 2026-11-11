//! Kontrol S8 deck LED output reports (0x80 left, 0x81 right) rendered from a plain model.

mod map;
mod strip;
#[cfg(test)]
mod tests;

pub use strip::{bar, dot};

pub const DECK_PAYLOAD_LEN: usize = 118;
pub const DECK_REPORT_LEN: usize = DECK_PAYLOAD_LEN + 1;
pub const STRIP_LEDS: usize = 25;
pub const MAX_VALUE: u8 = 0x7f;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub const ALL: [Self; 2] = [Self::Left, Self::Right];

    #[must_use]
    pub const fn report_id(self) -> u8 {
        match self {
            Self::Left => 0x80,
            Self::Right => 0x81,
        }
    }

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Level {
    #[default]
    Off,
    Dim,
    On,
}

impl Level {
    const fn byte(self) -> u8 {
        match self {
            Self::Off => map::OFF,
            Self::Dim => map::DIM,
            Self::On => map::ON,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const OFF: Self = Self { r: 0, g: 0, b: 0 };

    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dual {
    pub white: Level,
    pub blue: Level,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Strip {
    pub blue: [u8; STRIP_LEDS],
    pub orange: [u8; STRIP_LEDS],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SideLeds {
    pub pads: [Rgb; 8],
    pub fx_select: Level,
    pub fx: [Level; 4],
    pub display: [Level; 8],
    pub back: Level,
    pub capture: Level,
    pub edit: Level,
    pub slots: [Level; 4],
    pub hotcue: Dual,
    pub loop_mode: Dual,
    pub freeze: Dual,
    pub remix: Dual,
    pub flux: Level,
    pub deck: Dual,
    pub shift: Level,
    pub sync: Level,
    pub cue: Level,
    pub play: Level,
    pub loop_ring: [Dual; 4],
    pub strip: Strip,
}

struct Report([u8; DECK_REPORT_LEN]);

impl Report {
    const fn set(&mut self, offset: usize, value: u8) {
        self.0[offset + 1] = value;
    }

    const fn level(&mut self, offset: usize, level: Level) {
        self.set(offset, level.byte());
    }

    const fn dual(&mut self, offset: usize, dual: Dual) {
        self.level(offset, dual.white);
        self.level(offset + 1, dual.blue);
    }

    fn bank(&mut self, base: usize, values: &[u8; STRIP_LEDS]) {
        for (index, value) in values.iter().enumerate() {
            let offset = if map::STRIP_LEFT_IS_LOW_OFFSET {
                base + index
            } else {
                base + STRIP_LEDS - 1 - index
            };
            self.set(offset, (*value).min(MAX_VALUE));
        }
    }
}

#[must_use]
pub fn render(side: Side, leds: &SideLeds) -> [u8; DECK_REPORT_LEN] {
    let mut report = Report(clear(side));
    for (index, pad) in leds.pads.iter().enumerate() {
        let components = [pad.r, pad.g, pad.b];
        let base = map::PADS + index * map::PAD_BYTES;
        for (slot, source) in map::PAD_ORDER.iter().enumerate() {
            report.set(base + slot, components[*source].min(MAX_VALUE));
        }
    }
    report.level(map::FX_SELECT, leds.fx_select);
    for (index, level) in leds.fx.iter().enumerate() {
        report.level(map::FX + index, *level);
    }
    for (offset, level) in map::DISPLAY.iter().zip(leds.display.iter()) {
        report.level(*offset, *level);
    }
    report.level(map::BACK, leds.back);
    report.level(map::CAPTURE, leds.capture);
    report.level(map::EDIT, leds.edit);
    for (index, level) in leds.slots.iter().enumerate() {
        report.level(map::SLOTS + index, *level);
    }
    report.dual(map::HOTCUE, leds.hotcue);
    report.dual(map::LOOP_MODE, leds.loop_mode);
    report.dual(map::FREEZE, leds.freeze);
    report.dual(map::REMIX, leds.remix);
    report.level(map::FLUX, leds.flux);
    report.dual(map::DECK, leds.deck);
    report.level(map::SHIFT, leds.shift);
    report.level(map::SYNC_GREEN, leds.sync);
    report.level(map::CUE, leds.cue);
    report.level(map::PLAY, leds.play);
    for (index, segment) in leds.loop_ring.iter().enumerate() {
        report.level(map::RING_WHITE + index, segment.white);
        report.level(map::RING_BLUE + index, segment.blue);
    }
    report.bank(map::STRIP_BLUE, &leds.strip.blue);
    report.bank(map::STRIP_ORANGE, &leds.strip.orange);
    report.0
}

#[must_use]
pub const fn clear(side: Side) -> [u8; DECK_REPORT_LEN] {
    let mut report = [0; DECK_REPORT_LEN];
    report[0] = side.report_id();
    report
}
