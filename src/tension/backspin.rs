//! One bounded assist pulse per real touch gesture; no synthetic touch data.

use crate::jog::{
    Decision, FaderState,
    continuity::TAIL_US,
    decode::{Motion, Status},
    smooth::Smoothing,
};

use super::{MAX_ASSIST_FORCE, neutral_report};

const RELEASE_US: u64 = 1_000_000;

#[derive(Clone, Copy, Default)]
enum Phase {
    #[default]
    Idle,
    Held,
    Released(u64),
    Spent,
}

pub(super) struct Assist {
    side: usize,
    strength: u8,
    phase: Phase,
    qualified: bool,
    moved_at_us: Option<u64>,
    smoothing: Smoothing,
}

impl Assist {
    pub(super) const fn new(side: usize, strength: u8) -> Self {
        Self {
            side,
            strength,
            phase: Phase::Idle,
            qualified: false,
            moved_at_us: None,
            smoothing: Smoothing::new(side),
        }
    }

    pub(super) fn clear(&mut self) {
        self.cancel("input_reset");
        self.qualified = false;
    }

    pub(super) fn qualify(&mut self, qualified: bool, motion: &Motion) -> bool {
        let qualified = qualified && self.strength > 0;
        let active = self.active();
        match self.phase {
            Phase::Idle => {
                if qualified && !self.qualified && motion.touch[self.side] {
                    self.phase = Phase::Held;
                    tracing::info!(
                        side = self.side,
                        strength = self.strength,
                        "BACKSPIN_ASSIST_ARMED"
                    );
                    if motion.delta[self.side] < 0 {
                        self.moved_at_us = Some(motion.host_us);
                    }
                }
            }
            Phase::Held | Phase::Released(_) => {
                if !qualified {
                    self.cancel("event_end");
                }
            }
            Phase::Spent => {
                if !qualified && !motion.touch[self.side] {
                    self.phase = Phase::Idle;
                }
            }
        }
        self.qualified = qualified;
        active && !self.active()
    }

    pub(super) fn observe(&mut self, motion: &Motion, now_us: u64) -> Option<i32> {
        let mut released = false;
        match self.phase {
            Phase::Held => {
                if !motion.touch[self.side] {
                    self.phase = Phase::Released(motion.host_us);
                    released = true;
                }
            }
            Phase::Released(_) => {
                if motion.touch[self.side] {
                    self.cancel("retouch");
                }
            }
            Phase::Spent => {
                if !self.qualified && !motion.touch[self.side] {
                    self.phase = Phase::Idle;
                }
            }
            Phase::Idle => {}
        }
        if self.active() {
            match motion.status[self.side] {
                Status::Baseline => self.cancel("baseline"),
                Status::Moving => {
                    if motion.delta[self.side] < 0 {
                        self.moved_at_us = Some(motion.host_us);
                    } else {
                        self.cancel("reversal");
                    }
                }
                Status::Rest | Status::Duplicate => {}
            }
        }
        self.expire(now_us, Some(motion));
        let decision = self.smoothing.observe_with_assist_gate(
            motion,
            Decision {
                torque: [0; 2],
                report: neutral_report(),
                fader: Some(FaderState::bipolar(0, true, MAX_ASSIST_FORCE, 0)),
            },
            self.active(),
        );
        let torque = self.scaled(decision.torque[self.side], now_us);
        if released {
            tracing::info!(
                side = self.side,
                strength = self.strength,
                released_us = motion.host_us,
                deadline_us = motion.host_us.saturating_add(RELEASE_US),
                requested_torque = ?torque,
                status = ?motion.status[self.side],
                delta = motion.delta[self.side],
                "BACKSPIN_ASSIST_RELEASED"
            );
        }
        torque
    }

    pub(super) fn at(&mut self, now_us: u64, motion: Option<&Motion>) -> Option<i32> {
        self.expire(now_us, motion);
        let torque = self.smoothing.at(now_us)?.torque[self.side];
        self.scaled(torque, now_us)
    }

    const fn active(&self) -> bool {
        match self.phase {
            Phase::Held | Phase::Released(_) => true,
            Phase::Idle | Phase::Spent => false,
        }
    }

    fn expire(&mut self, now_us: u64, motion: Option<&Motion>) {
        let deadline = match self.phase {
            Phase::Released(at) => now_us.saturating_sub(at) >= RELEASE_US,
            Phase::Idle | Phase::Held | Phase::Spent => false,
        };
        let stopped = self.active()
            && self
                .moved_at_us
                .is_some_and(|at| now_us.saturating_sub(at) >= TAIL_US);
        if deadline {
            self.cancel("deadline");
        } else if stopped {
            tracing::info!(
                side = self.side,
                now_us,
                last_moved_us = ?self.moved_at_us,
                sample_host_us = ?motion.map(|sample| sample.host_us),
                sample_age_us = ?motion.map(|sample| now_us.saturating_sub(sample.host_us)),
                status = ?motion.map(|sample| sample.status[self.side]),
                delta = ?motion.map(|sample| sample.delta[self.side]),
                touch = ?motion.map(|sample| sample.touch[self.side]),
                "BACKSPIN_ASSIST_MOTION_STOP"
            );
            self.cancel("motion_stop");
        }
    }

    fn cancel(&mut self, reason: &'static str) {
        if self.active() {
            tracing::info!(side = self.side, reason, "BACKSPIN_ASSIST_CANCELLED");
        }
        self.phase = Phase::Spent;
        self.moved_at_us = None;
        self.smoothing = Smoothing::new(self.side);
    }

    fn scaled(&self, torque: i32, now_us: u64) -> Option<i32> {
        let remaining = match self.phase {
            Phase::Held => RELEASE_US,
            Phase::Released(at) => RELEASE_US.saturating_sub(now_us.saturating_sub(at)),
            Phase::Idle | Phase::Spent => return None,
        };
        let magnitude = u64::from(torque.unsigned_abs()) * remaining * u64::from(self.strength)
            / (RELEASE_US * 100);
        // The existing assist law bounds magnitude to MAX_ASSIST_FORCE (7000).
        let bytes = magnitude.to_le_bytes();
        Some(torque.signum() * i32::from(u16::from_le_bytes([bytes[0], bytes[1]])))
    }
}
