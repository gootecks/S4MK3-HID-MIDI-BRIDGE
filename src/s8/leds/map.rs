//! Deck payload byte offsets (report ID excluded) and value encoding. Unconfirmed on hardware: `PAD_ORDER`, `DIM`, `STRIP_LEFT_IS_LOW_OFFSET`, `SYNC`/`SYNC_SECOND`, `DISPLAY`.

pub const ON: u8 = 0x7f;
pub const DIM: u8 = 0x14;
pub const OFF: u8 = 0;

pub const PAD_ORDER: [usize; 3] = [0, 1, 2];
pub const PADS: usize = 0;
pub const PAD_BYTES: usize = 3;

pub const FX_SELECT: usize = 24;
pub const FX: usize = 25;
pub const DISPLAY: [usize; 8] = [29, 30, 31, 32, 33, 34, 35, 36];
pub const BACK: usize = 37;
pub const CAPTURE: usize = 38;
pub const EDIT: usize = 39;
pub const SLOTS: usize = 40;
pub const HOTCUE: usize = 44;
pub const LOOP_MODE: usize = 46;
pub const FREEZE: usize = 48;
pub const REMIX: usize = 50;
pub const FLUX: usize = 52;
pub const DECK: usize = 53;
pub const SHIFT: usize = 55;
#[cfg(test)]
pub const SYNC_SECOND: usize = 56;
pub const SYNC: usize = 57;
pub const CUE: usize = 58;
pub const PLAY: usize = 59;
pub const RING_WHITE: usize = 60;
pub const RING_BLUE: usize = 64;
pub const STRIP_BLUE: usize = 68;
pub const STRIP_ORANGE: usize = 93;

pub const STRIP_LEFT_IS_LOW_OFFSET: bool = true;
