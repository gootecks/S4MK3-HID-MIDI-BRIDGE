//! Qualified movement magnitude for braking and held-touch assistance.

use std::collections::VecDeque;

use super::continuity::TAIL_US;
use super::decode::{Motion, Status};
use super::model::{Decision, FaderZone, report_for};

pub const WINDOW_US: u64 = 20_000;
pub const DEADBAND_COUNTS: u64 = 4;
pub const FULL_SCALE_COUNTS: u64 = 20;
pub const QUALIFICATION_US: u64 = 6_000;
pub const QUALIFICATION_COUNTS: u32 = 6;

pub struct Damping {
    side: usize,
    recent: VecDeque<(u64, u32)>,
    direction: i32,
    since_us: u64,
    counts: u32,
    qualified: bool,
    moved_at_us: u64,
    last_force: u16,
    latest: Option<Decision>,
}

impl Damping {
    #[must_use]
    pub const fn new(side: usize) -> Self {
        Self {
            side,
            recent: VecDeque::new(),
            direction: 0,
            since_us: 0,
            counts: 0,
            qualified: false,
            moved_at_us: 0,
            last_force: 0,
            latest: None,
        }
    }

    pub fn observe(&mut self, motion: &Motion, decision: Decision) -> Decision {
        if self
            .latest
            .and_then(|previous| previous.fader)
            .and_then(|level| level.zone)
            != decision.fader.and_then(|level| level.zone)
        {
            self.clear_motion();
        }
        self.latest = Some(decision);
        if decision.fader.is_some_and(|level| {
            level.armed
                && level.selected_force > 0
                && match level.zone {
                    Some(FaderZone::Brake) => true,
                    Some(FaderZone::Assist) => motion.touch[self.side],
                    Some(FaderZone::Neutral) | None => false,
                }
        }) {
            match motion.status[self.side] {
                Status::Baseline => self.clear_motion(),
                Status::Moving => {
                    let direction = motion.delta[self.side].signum();
                    if direction != self.direction
                        || motion.host_us.saturating_sub(self.moved_at_us) >= TAIL_US
                    {
                        self.clear_motion();
                        self.direction = direction;
                        self.since_us = motion.host_us;
                    }
                    self.recent
                        .retain(|(at, _)| motion.host_us - at < WINDOW_US);
                    self.recent
                        .push_back((motion.host_us, motion.delta[self.side].unsigned_abs()));
                    self.counts = self
                        .counts
                        .saturating_add(motion.delta[self.side].unsigned_abs());
                    self.qualified = self.counts >= QUALIFICATION_COUNTS
                        && motion.host_us - self.since_us >= QUALIFICATION_US;
                    self.moved_at_us = motion.host_us;
                    self.last_force = decision.fader.map_or(0, |level| level.selected_force);
                }
                Status::Rest | Status::Duplicate => {}
            }
        } else {
            self.clear_motion();
        }
        self.at(motion.host_us).unwrap_or(decision)
    }

    #[must_use]
    pub fn at(&self, now_us: u64) -> Option<Decision> {
        let tail = TAIL_US.saturating_sub(now_us.saturating_sub(self.moved_at_us));
        self.render(now_us, tail)
    }

    #[must_use]
    pub fn steady_at(&self, now_us: u64) -> Option<Decision> {
        let tail = if self.direction_at(now_us) == 0 {
            0
        } else {
            TAIL_US
        };
        self.render(now_us, tail)
    }

    #[must_use]
    pub const fn direction_at(&self, now_us: u64) -> i32 {
        if self.qualified && now_us.saturating_sub(self.moved_at_us) < TAIL_US {
            self.direction
        } else {
            0
        }
    }

    fn render(&self, now_us: u64, tail: u64) -> Option<Decision> {
        self.latest.map(|mut decision| {
            let counts: u64 = self
                .recent
                .iter()
                .filter(|(at, _)| now_us.saturating_sub(*at) < WINDOW_US)
                .map(|(_, count)| u64::from(*count))
                .sum();
            let speed = counts
                .min(FULL_SCALE_COUNTS)
                .saturating_sub(DEADBAND_COUNTS);
            let ceiling = decision
                .fader
                .map_or(0, |level| level.selected_force.min(self.last_force));
            let force = if self.qualified {
                u64::from(ceiling) * speed * tail
                    / ((FULL_SCALE_COUNTS - DEADBAND_COUNTS) * TAIL_US)
            } else {
                0
            };
            // The scaled magnitude is bounded by the selected u16 ceiling.
            let bytes = force.to_le_bytes();
            let sign = match decision.fader.and_then(|level| level.zone) {
                Some(FaderZone::Assist) => 1,
                Some(FaderZone::Brake) => -1,
                Some(FaderZone::Neutral) | None => 0,
            };
            decision.torque = [0; 2];
            decision.torque[self.side] =
                sign * self.direction * i32::from(u16::from_le_bytes([bytes[0], bytes[1]]));
            decision.report = report_for(decision.torque);
            decision
        })
    }

    fn clear_motion(&mut self) {
        self.recent.clear();
        self.direction = 0;
        self.counts = 0;
        self.qualified = false;
    }
}
