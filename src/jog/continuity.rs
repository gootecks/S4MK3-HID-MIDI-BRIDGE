//! Bounded motion memory, not a reconstruction of Traktor's velocity filter.

use super::decode::{Motion, Status};
use super::model::{Decision, FaderZone, report_for};

pub const TAIL_US: u64 = 8_000;
pub const OUTPUT_INTERVAL_US: u64 = 2_000;

pub struct Continuity {
    direction: i32,
    moved_at: u64,
    last_force: u16,
    latest: Option<Decision>,
}

impl Default for Continuity {
    fn default() -> Self {
        Self::new()
    }
}

impl Continuity {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            direction: 0,
            moved_at: 0,
            last_force: 0,
            latest: None,
        }
    }

    pub fn observe(&mut self, motion: &Motion, decision: Decision) -> Decision {
        let zone = decision.fader.and_then(|level| level.zone);
        let previous_zone = self.latest.and_then(|d| d.fader).and_then(|f| f.zone);
        let enabled = decision.fader.is_some_and(|level| {
            level.armed
                && level.selected_force > 0
                && (zone != Some(FaderZone::Assist) || motion.touch[0])
        });
        if zone != previous_zone || !enabled {
            self.direction = 0;
        }
        match motion.status[0] {
            Status::Moving => {
                self.direction = decision.torque[0].signum();
                self.moved_at = motion.host_us;
                let bytes = decision.torque[0].unsigned_abs().to_le_bytes();
                self.last_force = u16::from_le_bytes([bytes[0], bytes[1]]);
            }
            Status::Baseline => self.direction = 0,
            Status::Rest | Status::Duplicate => {}
        }
        self.latest = Some(decision);
        // A decision just observed always supplies the selection for this time.
        self.at(motion.host_us).unwrap_or(decision)
    }

    #[must_use]
    pub fn at(&self, now_us: u64) -> Option<Decision> {
        self.latest.map(|mut decision| {
            let remaining = TAIL_US.saturating_sub(now_us.saturating_sub(self.moved_at));
            let force = decision
                .fader
                .map_or(0, |level| level.selected_force.min(self.last_force));
            // remaining <= TAIL_US, so the result fits the selected u16 ceiling.
            let scaled = u64::from(force) * remaining / TAIL_US;
            let bytes = scaled.to_le_bytes();
            decision.torque = [
                self.direction * i32::from(u16::from_le_bytes([bytes[0], bytes[1]])),
                0,
            ];
            decision.report = report_for(decision.torque);
            decision
        })
    }
}
