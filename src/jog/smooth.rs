//! Clock-driven magnitude smoothing, with independent immediate zero gates.

use super::damped::Damping;
use super::decode::Motion;
use super::model::{Decision, FaderZone, report_for};

pub const TIME_CONSTANT_US: u64 = 20_000;
const SCALE: u128 = 65_536;

pub struct Smoothing {
    side: usize,
    damping: Damping,
    magnitude_q16: u128,
    updated_at_us: Option<u64>,
}

impl Smoothing {
    #[must_use]
    pub const fn new(side: usize) -> Self {
        Self {
            side,
            damping: Damping::new(side),
            magnitude_q16: 0,
            updated_at_us: None,
        }
    }

    pub fn observe(&mut self, motion: &Motion, decision: Decision) -> Decision {
        self.observe_with_assist_gate(motion, decision, motion.touch[self.side])
    }

    pub(crate) fn observe_with_assist_gate(
        &mut self,
        motion: &Motion,
        decision: Decision,
        assist_gate: bool,
    ) -> Decision {
        let mut result = self
            .damping
            .observe_with_assist_gate(motion, decision, assist_gate);
        let direction = self.damping.direction_at(motion.host_us);
        if direction == 0 {
            self.magnitude_q16 = 0;
        } else {
            let cap = result.fader.map_or(0, |level| level.selected_force);
            self.magnitude_q16 = self.magnitude_q16.min(u128::from(cap) * SCALE);
        }
        self.render(&mut result, direction);
        result
    }

    pub fn at(&mut self, now_us: u64) -> Option<Decision> {
        let mut result = self.damping.steady_at(now_us)?;
        let direction = self.damping.direction_at(now_us);
        let elapsed = self.updated_at_us.map_or(0, |at| now_us.saturating_sub(at));
        self.updated_at_us = Some(now_us);
        if direction == 0 {
            self.magnitude_q16 = 0;
        } else {
            let dt = u128::from(elapsed);
            let tau = u128::from(TIME_CONSTANT_US);
            let target = u128::from(result.torque[self.side].unsigned_abs()) * SCALE;
            self.magnitude_q16 = (self.magnitude_q16 * tau + target * dt) / (tau + dt);
            let cap = result.fader.map_or(0, |level| level.selected_force);
            self.magnitude_q16 = self.magnitude_q16.min(u128::from(cap) * SCALE);
        }
        self.render(&mut result, direction);
        Some(result)
    }

    fn render(&self, decision: &mut Decision, direction: i32) {
        // The fixed-point value is bounded by a u16 ceiling. Keep fractions
        // internally; rounding only here lets a ceiling of 1 still produce 1.
        let bytes = ((self.magnitude_q16 + SCALE / 2) / SCALE).to_le_bytes();
        let magnitude = i32::from(u16::from_le_bytes([bytes[0], bytes[1]]));
        let sign = match decision.fader.and_then(|level| level.zone) {
            Some(FaderZone::Assist) => 1,
            Some(FaderZone::Brake) => -1,
            Some(FaderZone::Neutral) | None => 0,
        };
        decision.torque = [0; 2];
        decision.torque[self.side] = sign * direction * magnitude;
        decision.report = report_for(decision.torque);
    }
}
