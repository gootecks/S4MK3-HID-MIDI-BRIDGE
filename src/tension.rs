//! Offline shared wheel-tension selection and state, with no device I/O.

use std::path::Path;

use anyhow::{Context as _, ensure};
use serde::{Deserialize, Deserializer, de::Error as _};

use crate::jog::{
    Decision, FaderState,
    decode::{Decoder, FRESHNESS_US, Motion},
    model::report_for,
    smooth_wheels::SmoothWheels,
};

mod backspin;

pub use crate::jog::neutral_report;

const MAX_ASSIST_FORCE: u16 = 7000;

#[derive(Clone, Copy, Debug, Default)]
pub struct Config {
    tension: i32,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SavedConfig {
    tension: i32,
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let saved = SavedConfig::deserialize(deserializer)?;
        if !(-100..=100).contains(&saved.tension) {
            return Err(D::Error::custom("tension must be between -100 and 100"));
        }
        Ok(Self {
            tension: saved.tension,
        })
    }
}

impl Config {
    /// Read a saved selection, or use neutral when no file is supplied.
    ///
    /// # Errors
    /// Rejects unreadable JSON, unknown fields, non-integer tension, and values
    /// outside -100..=100.
    pub fn load(path: Option<&Path>) -> anyhow::Result<Self> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        let data = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
        serde_json::from_slice(&data).with_context(|| format!("parse {}", path.display()))
    }

    #[must_use]
    pub const fn value(&self) -> i32 {
        self.tension
    }
}

pub struct Engine {
    decoder: Decoder,
    wheels: SmoothWheels,
    selection: FaderState,
    last_input_us: u64,
    last_torque: [i32; 2],
    motion: Option<Motion>,
    backspin: [backspin::Assist; 2],
}

#[cfg(test)]
#[path = "tension_tests.rs"]
mod tests;

impl Engine {
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self::with_strength(config, 100)
    }

    /// Select optional backspin assistance independently of normal tension.
    ///
    /// # Errors
    /// Rejects percentages above the existing maximum assistance.
    pub fn with_backspin_strength(config: Config, strength: u8) -> anyhow::Result<Self> {
        ensure!(
            strength <= 100,
            "backspin strength must be between 0 and 100"
        );
        Ok(Self::with_strength(config, strength))
    }

    fn with_strength(config: Config, strength: u8) -> Self {
        let value = config.value();
        let selection = match value.cmp(&0) {
            std::cmp::Ordering::Less => {
                // Config bounds this scaled value to 7000.
                let bytes =
                    (value.unsigned_abs() * u32::from(MAX_ASSIST_FORCE) / 100).to_le_bytes();
                let cap = u16::from_le_bytes([bytes[0], bytes[1]]);
                FaderState::bipolar(0, true, cap, 0)
            }
            std::cmp::Ordering::Greater => {
                // Config bounds this scaled value to 30000.
                let bytes = (value.unsigned_abs() * 30000 / 100).to_le_bytes();
                let cap = u16::from_le_bytes([bytes[0], bytes[1]]);
                FaderState::bipolar(4095, true, 0, cap)
            }
            std::cmp::Ordering::Equal => FaderState::bipolar(2048, true, 0, 0),
        };
        Self {
            decoder: Decoder::default(),
            wheels: SmoothWheels::new([true, true]),
            selection,
            last_input_us: 0,
            last_torque: [0; 2],
            motion: None,
            backspin: [
                backspin::Assist::new(0, strength),
                backspin::Assist::new(1, strength),
            ],
        }
    }

    /// Discard all movement and smoothing history, preserving the selection.
    /// The caller must neutralize hardware first. This does not refresh the
    /// input deadline; a new snapshot must establish a baseline before ticking.
    pub fn clear_motion(&mut self) {
        self.decoder = Decoder::default();
        self.wheels = SmoothWheels::new([true, true]);
        self.last_torque = [0; 2];
        self.motion = None;
        for assist in &mut self.backspin {
            assist.clear();
        }
    }

    /// Supply current full-profile gesture qualification without changing touch.
    /// Returns an immediate partial neutral when an override is cancelled.
    #[must_use]
    pub fn set_backspin(&mut self, qualified: [bool; 2]) -> Option<[u8; 11]> {
        let motion = self.motion?;
        let mut changed = false;
        for (side, qualified) in qualified.into_iter().enumerate() {
            if self.backspin[side].qualify(qualified, &motion) && self.last_torque[side] != 0 {
                self.last_torque[side] = 0;
                changed = true;
            }
        }
        changed.then(|| report_for(self.last_torque))
    }

    /// Observe one numbered buttons/jog snapshot and return only an immediate
    /// zero-gate report. The caller owns all report writes and cleanup.
    ///
    /// # Errors
    /// Rejects stale, future, malformed, or discontinuous input.
    pub fn observe(
        &mut self,
        buttons: &[u8],
        jog: &[u8],
        host_us: u64,
        now_us: u64,
    ) -> anyhow::Result<Option<[u8; 11]>> {
        ensure!(
            now_us >= host_us && now_us - host_us <= FRESHNESS_US,
            "queued input is older than 50ms or from the future"
        );
        let motion = self.decoder.decode_snapshot(buttons, jog, None, host_us)?;
        self.motion = Some(motion);
        self.last_input_us = host_us;
        let selection = self.selection;
        let mut decision = self.wheels.observe(
            &motion,
            Decision {
                torque: [0; 2],
                report: neutral_report(),
                fader: Some(selection),
            },
        );
        for (side, assist) in self.backspin.iter_mut().enumerate() {
            if let Some(torque) = assist.observe(&motion, now_us) {
                decision.torque[side] = torque;
            }
        }
        let immediate_zero =
            (0..2).any(|side| decision.torque[side] == 0 && self.last_torque[side] != 0);
        if immediate_zero {
            self.last_torque = decision.torque;
            Ok(Some(report_for(decision.torque)))
        } else {
            Ok(None)
        }
    }

    /// Produce the next best-effort paced output from qualified fresh movement.
    ///
    /// # Errors
    /// Rejects ticks beyond the 50 ms input freshness deadline.
    pub fn tick(&mut self, now_us: u64) -> anyhow::Result<[u8; 11]> {
        ensure!(
            now_us <= self.input_deadline_us(),
            "no fresh input within 50ms"
        );
        let mut torque = self
            .wheels
            .at(now_us)
            .map_or([0; 2], |decision| decision.torque);
        for (side, assist) in self.backspin.iter_mut().enumerate() {
            if let Some(override_torque) = assist.at(now_us, self.motion.as_ref()) {
                torque[side] = override_torque;
            }
        }
        self.last_torque = torque;
        Ok(report_for(torque))
    }

    #[must_use]
    pub const fn input_deadline_us(&self) -> u64 {
        self.last_input_us.saturating_add(FRESHNESS_US)
    }
}
