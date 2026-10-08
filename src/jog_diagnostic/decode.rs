//! Decode the full 32-bit counters without assuming a hardware timer frequency.

use anyhow::ensure;
use serde::{Deserialize, Serialize};

use super::capture::RawFrame;

pub const FRESHNESS_US: u64 = 50_000;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Baseline,
    Moving,
    Rest,
    Duplicate,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Motion {
    pub host_us: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck1_fader: Option<u16>,
    pub touch: [bool; 2],
    pub counters: [u32; 2],
    pub timers: [u32; 2],
    pub delta: [i32; 2],
    pub elapsed_us: [u64; 2],
    pub status: [Status; 2],
}

#[derive(Clone, Copy)]
struct Observation {
    host_us: u64,
    timer: u32,
    counter: u32,
}

#[derive(Default)]
pub struct Decoder {
    previous: [Option<Observation>; 2],
    last_host_us: Option<u64>,
}

impl Decoder {
    pub fn decode(&mut self, frame: &RawFrame) -> anyhow::Result<Motion> {
        ensure!(
            frame.buttons.len() == 23 && frame.buttons[0] == 1,
            "expected numbered 23-byte buttons report"
        );
        ensure!(
            frame.jog.len() == 60 && frame.jog[0] == 3,
            "expected numbered 60-byte jog report"
        );
        if let Some(last) = self.last_host_us {
            ensure!(
                frame.host_us > last,
                "host timestamps must strictly increase"
            );
        }
        let deck1_fader = if let Some(analog) = &frame.analog {
            ensure!(
                analog.len() == 79 && analog[0] == 2,
                "expected numbered 79-byte analog report"
            );
            let value = u16::from_le_bytes([analog[3], analog[4]]);
            ensure!(value <= 4095, "deck1 fader exceeds 12-bit range 4095");
            Some(value)
        } else {
            None
        };
        let mut motion = Motion {
            host_us: frame.host_us,
            deck1_fader,
            touch: [frame.buttons[17] & 0x10 != 0, frame.buttons[17] & 0x20 != 0],
            counters: [word(&frame.jog, 12), word(&frame.jog, 40)],
            timers: [word(&frame.jog, 8), word(&frame.jog, 36)],
            delta: [0; 2],
            elapsed_us: [0; 2],
            status: [Status::Baseline; 2],
        };
        for side in 0..2 {
            let current = Observation {
                host_us: frame.host_us,
                timer: motion.timers[side],
                counter: motion.counters[side],
            };
            if let Some(old) = self.previous[side] {
                let elapsed = current.host_us - old.host_us;
                ensure!(
                    elapsed <= FRESHNESS_US,
                    "wheel {side} input gap exceeds 50ms"
                );
                if current.timer == old.timer {
                    ensure!(
                        current.counter == old.counter,
                        "counter changed without sensor time"
                    );
                    motion.status[side] = Status::Duplicate;
                    continue; // A duplicate cannot refresh the freshness deadline.
                }
                ensure!(
                    current.timer > old.timer
                        || (old.timer > u32::MAX - 1_000_000 && current.timer < 1_000_000),
                    "wheel {side} sensor time reversed"
                );
                let delta =
                    i32::from_le_bytes(current.counter.wrapping_sub(old.counter).to_le_bytes());
                ensure!(
                    delta.unsigned_abs() <= 512,
                    "wheel {side} count jump exceeds 512"
                );
                motion.delta[side] = delta;
                motion.elapsed_us[side] = elapsed;
                motion.status[side] = if delta == 0 {
                    Status::Rest
                } else {
                    Status::Moving
                };
            }
            self.previous[side] = Some(current);
        }
        self.last_host_us = Some(frame.host_us);
        Ok(motion)
    }
}

fn word(report: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        report[offset],
        report[offset + 1],
        report[offset + 2],
        report[offset + 3],
    ])
}

#[cfg(test)]
#[path = "decode_tests.rs"]
pub(super) mod tests;
