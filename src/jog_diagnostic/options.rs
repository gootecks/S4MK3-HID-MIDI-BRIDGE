//! Boundary validation and defaults for diagnostic control modes.

use anyhow::ensure;

use super::{ControlArgs, ControlOptions, Mode, Side};

impl ControlOptions {
    pub(super) fn resolve(self) -> anyhow::Result<ControlArgs> {
        ensure!(
            matches!(self.mode, Mode::FaderPlay) || !matches!(self.side, Side::Both),
            "--side both requires fader-play"
        );
        ensure!(
            matches!(
                self.mode,
                Mode::FaderTension
                    | Mode::FaderContinuity
                    | Mode::FaderDamped
                    | Mode::FaderSmooth
                    | Mode::FaderAssist
                    | Mode::FaderPlay
            ) || self.assist_force.is_none(),
            "--assist-force requires a bipolar fader mode"
        );
        ensure!(
            !matches!(self.mode, Mode::FaderDamped | Mode::FaderSmooth)
                || self.assist_force.is_none_or(|force| force == 0),
            "braking diagnostics --assist-force must be 0"
        );
        if self.mode.uses_fader() {
            ensure!(
                matches!(self.mode, Mode::FaderPlay) || matches!(self.side, Side::Left),
                "legacy fader modes require --side left and the physical deck1 fader"
            );
            ensure!(
                self.gain.is_none(),
                "fader modes use direct force; omit --gain"
            );
        }
        let max_force = match self.mode {
            Mode::FaderAssist => {
                ensure!(
                    self.max_force.is_none(),
                    "fader-assist disables braking; omit --max-force"
                );
                0
            }
            Mode::FaderBrake => {
                let force = self.max_force.unwrap_or(5000);
                ensure!(force <= 25000, "fader-brake --max-force must be <=25000");
                force
            }
            Mode::FaderTension
            | Mode::FaderContinuity
            | Mode::FaderDamped
            | Mode::FaderSmooth
            | Mode::FaderPlay => self.max_force.unwrap_or(30000),
            Mode::Baseline | Mode::Brake | Mode::Assist => {
                let max_force = self.max_force.unwrap_or(100);
                ensure!(max_force <= 1000, "fixed-mode --max-force must be <=1000");
                max_force
            }
        };
        Ok(ControlArgs {
            mode: self.mode,
            side: self.side,
            gain: self.gain.unwrap_or(1),
            max_force,
            assist_force: match self.mode {
                Mode::FaderTension | Mode::FaderContinuity => {
                    Some(self.assist_force.unwrap_or(8000))
                }
                Mode::FaderDamped | Mode::FaderSmooth => Some(0),
                Mode::FaderAssist | Mode::FaderPlay => Some(self.assist_force.unwrap_or(2000)),
                Mode::Baseline | Mode::Brake | Mode::Assist | Mode::FaderBrake => None,
            },
        })
    }
}
