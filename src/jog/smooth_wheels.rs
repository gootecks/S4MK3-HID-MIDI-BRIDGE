//! Independent wheel histories combined into one native motor report.

use super::decode::Motion;
use super::model::{Decision, neutral_report, report_for};
use super::smooth::Smoothing;

pub struct SmoothWheels {
    selected: [bool; 2],
    wheels: [Smoothing; 2],
}

impl SmoothWheels {
    #[must_use]
    pub const fn new(selected: [bool; 2]) -> Self {
        Self {
            selected,
            wheels: [Smoothing::new(0), Smoothing::new(1)],
        }
    }

    pub fn observe(&mut self, motion: &Motion, decision: Decision) -> Decision {
        let mut combined = decision;
        combined.torque = [0; 2];
        for (side, wheel) in self.wheels.iter_mut().enumerate() {
            if self.selected[side] {
                combined.torque[side] = wheel.observe(motion, decision).torque[side];
            }
        }
        combined.report = report_for(combined.torque);
        combined
    }

    pub fn at(&mut self, now_us: u64) -> Option<Decision> {
        let mut combined = None;
        for (side, wheel) in self.wheels.iter_mut().enumerate() {
            if let Some(decision) = wheel.at(now_us) {
                let output = combined.get_or_insert(Decision {
                    torque: [0; 2],
                    report: neutral_report(),
                    fader: decision.fader,
                });
                output.torque[side] = decision.torque[side];
            }
        }
        combined.map(|mut decision| {
            decision.report = report_for(decision.torque);
            decision
        })
    }
}
