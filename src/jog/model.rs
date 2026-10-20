//! Wire decision and motor-report representation shared with the diagnostic.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Decision {
    pub torque: [i32; 2],
    pub report: [u8; 11],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fader: Option<FaderState>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct FaderState {
    pub raw: u16,
    pub armed: bool,
    pub selected_force: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<FaderZone>,
}

impl FaderState {
    /// Resolve the diagnostic bipolar fader into its selected force.
    #[must_use]
    pub fn bipolar(raw: u16, armed: bool, assist_force: u16, max_force: u16) -> Self {
        let (zone, force) = if raw < 2016 {
            (
                FaderZone::Assist,
                u32::from(2016 - raw) * u32::from(assist_force) / 2016,
            )
        } else if raw > 2080 {
            (
                FaderZone::Brake,
                u32::from(raw - 2080) * u32::from(max_force) / 2015,
            )
        } else {
            (FaderZone::Neutral, 0)
        };
        // Both fader scalings are bounded by their u16 ceilings.
        let bytes = force.to_le_bytes();
        Self {
            raw,
            armed,
            selected_force: u16::from_le_bytes([bytes[0], bytes[1]]),
            zone: Some(zone),
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FaderZone {
    Assist,
    Neutral,
    Brake,
}

#[must_use]
pub fn report_for(torque: [i32; 2]) -> [u8; 11] {
    let mut report = neutral_report();
    for (side, value) in torque.iter().enumerate() {
        let offset = 1 + side * 5;
        if *value < 0 {
            report[offset + 1] = 0xe0;
            report[offset + 2] = 0xfe;
        }
        let bytes = value.unsigned_abs().to_le_bytes();
        report[offset + 3..offset + 5].copy_from_slice(&bytes[..2]);
    }
    report
}

#[must_use]
pub const fn neutral_report() -> [u8; 11] {
    [0x31, 1, 0x20, 1, 0, 0, 1, 0x20, 1, 0, 0]
}
