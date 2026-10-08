//! Motion-only brake/assist proposals, never target-speed playback control.

use serde::{Deserialize, Serialize};

use super::decode::{Motion, Status};
use super::{ControlArgs, Mode};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Decision {
    pub torque: [i32; 2],
    pub report: [u8; 11],
}

pub struct Controller {
    control: ControlArgs,
}

impl Controller {
    pub const fn new(control: ControlArgs) -> Self {
        Self { control }
    }

    pub fn decide(&self, motion: &Motion) -> Decision {
        let mut torque = [0_i32; 2];
        let side = self.control.side.index();
        let sign = match self.control.mode {
            Mode::Baseline => 0,
            Mode::Brake => -1,
            Mode::Assist => 1,
        };
        match motion.status[side] {
            Status::Moving => {
                let magnitude = (u64::from(motion.delta[side].unsigned_abs())
                    * u64::from(self.control.gain)
                    * 1000
                    / motion.elapsed_us[side])
                    .min(u64::from(self.control.max_force));
                // Magnitude is <= a u16 cap, so its low two bytes are lossless.
                let bytes = magnitude.to_le_bytes();
                torque[side] = sign
                    * motion.delta[side].signum()
                    * i32::from(u16::from_le_bytes([bytes[0], bytes[1]]));
            }
            Status::Baseline | Status::Rest | Status::Duplicate => {}
        }
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
        Decision { torque, report }
    }
}

pub const fn neutral_report() -> [u8; 11] {
    [0x31, 1, 0x20, 1, 0, 0, 1, 0x20, 1, 0, 0]
}

#[cfg(test)]
#[path = "motor_tests.rs"]
mod tests;
