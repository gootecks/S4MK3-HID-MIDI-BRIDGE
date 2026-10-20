//! Motion-only brake/assist proposals, never target-speed playback control.

use anyhow::Context as _;
pub use s4mk3_hid_midi_bridge::jog::{Decision, FaderState, FaderZone, neutral_report, report_for};

use super::continuity::Continuity;
use super::damped::Damping;
use super::decode::{Motion, Status};
use super::smooth_wheels::SmoothWheels;
use super::{ControlArgs, Mode};

pub struct Controller {
    control: ControlArgs,
    fader_armed: bool,
    continuity: Continuity,
    damping: Damping,
    smoothing: SmoothWheels,
}

impl Controller {
    pub const fn new(control: ControlArgs) -> Self {
        Self {
            control,
            fader_armed: false,
            continuity: Continuity::new(),
            damping: Damping::new(0),
            smoothing: SmoothWheels::new(control.side.mask()),
        }
    }

    pub fn decide(&mut self, motion: &Motion) -> anyhow::Result<Decision> {
        let decision = self.decide_direct(motion)?;
        Ok(match self.control.mode {
            Mode::FaderSmooth | Mode::FaderAssist | Mode::FaderPlay => {
                self.smoothing.observe(motion, decision)
            }
            Mode::FaderDamped => self.damping.observe(motion, decision),
            Mode::FaderContinuity => self.continuity.observe(motion, decision),
            Mode::Baseline | Mode::Brake | Mode::Assist | Mode::FaderBrake | Mode::FaderTension => {
                decision
            }
        })
    }

    pub fn output_at(&mut self, now_us: u64) -> Option<Decision> {
        match self.control.mode {
            Mode::FaderSmooth | Mode::FaderAssist | Mode::FaderPlay => self.smoothing.at(now_us),
            Mode::FaderDamped => self.damping.at(now_us),
            Mode::Baseline
            | Mode::Brake
            | Mode::Assist
            | Mode::FaderBrake
            | Mode::FaderTension
            | Mode::FaderContinuity => self.continuity.at(now_us),
        }
    }

    fn select_fader(&mut self, motion: &Motion) -> anyhow::Result<Option<FaderState>> {
        Ok(match self.control.mode {
            Mode::FaderBrake => {
                let raw = motion
                    .deck1_fader
                    .context("fader-brake requires analog report 2; capture with --analog")?;
                self.fader_armed |= raw <= 32;
                let selected_force = if raw <= 32 {
                    0
                } else {
                    u16::try_from(u32::from(raw) * u32::from(self.control.max_force) / 4095)?
                };
                Some(FaderState {
                    raw,
                    armed: self.fader_armed,
                    selected_force,
                    zone: None,
                })
            }
            Mode::FaderTension
            | Mode::FaderContinuity
            | Mode::FaderDamped
            | Mode::FaderSmooth
            | Mode::FaderAssist
            | Mode::FaderPlay => {
                let raw = motion
                    .deck1_fader
                    .context("fader-tension requires analog report 2; capture with --analog")?;
                self.fader_armed |= (2016..=2080).contains(&raw);
                Some(FaderState::bipolar(
                    raw,
                    self.fader_armed,
                    self.control
                        .assist_force
                        .context("missing assistance ceiling")?,
                    self.control.max_force,
                ))
            }
            Mode::Baseline | Mode::Brake | Mode::Assist => None,
        })
    }

    fn decide_direct(&mut self, motion: &Motion) -> anyhow::Result<Decision> {
        let mut torque = [0_i32; 2];
        let fader = self.select_fader(motion)?;
        let sign = match self.control.mode {
            Mode::Baseline => 0,
            Mode::Brake | Mode::FaderBrake => -1,
            Mode::Assist => 1,
            Mode::FaderTension
            | Mode::FaderContinuity
            | Mode::FaderDamped
            | Mode::FaderSmooth
            | Mode::FaderAssist
            | Mode::FaderPlay => match fader.context("missing tension selection")?.zone {
                Some(FaderZone::Assist) => 1,
                Some(FaderZone::Neutral) => 0,
                Some(FaderZone::Brake) => -1,
                None => anyhow::bail!("missing tension zone"),
            },
        };
        for (side, selected) in self.control.side.mask().into_iter().enumerate() {
            if !selected {
                continue;
            }
            match motion.status[side] {
                Status::Moving => {
                    let magnitude = if let Some(level) = fader {
                        let held = level.zone != Some(FaderZone::Assist) || motion.touch[side];
                        if level.armed && held {
                            u64::from(level.selected_force)
                        } else {
                            0
                        }
                    } else {
                        (u64::from(motion.delta[side].unsigned_abs())
                            * u64::from(self.control.gain)
                            * 1000
                            / motion.elapsed_us[side])
                            .min(u64::from(self.control.max_force))
                    };
                    // Magnitude is <= a u16 cap, so its low two bytes are lossless.
                    let bytes = magnitude.to_le_bytes();
                    torque[side] = sign
                        * motion.delta[side].signum()
                        * i32::from(u16::from_le_bytes([bytes[0], bytes[1]]));
                }
                Status::Baseline | Status::Rest | Status::Duplicate => {}
            }
        }
        Ok(Decision {
            torque,
            report: report_for(torque),
            fader,
        })
    }
}

#[cfg(test)]
#[path = "motor_tests.rs"]
mod tests;
