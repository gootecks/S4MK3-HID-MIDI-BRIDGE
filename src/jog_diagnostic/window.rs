//! Trial-duration origin, separate from motor arming and input freshness.

pub const NEUTRAL_HOLD_US: u64 = 1_000_000;

pub enum TrialWindow {
    Immediate,
    Waiting { neutral_since_us: Option<u64> },
    Running { started_us: u64 },
}

impl TrialWindow {
    pub fn observe(&mut self, neutral: bool, now_us: u64) -> bool {
        match self {
            Self::Waiting { neutral_since_us } => {
                if !neutral {
                    *neutral_since_us = None;
                    return false;
                }
                let since = *neutral_since_us.get_or_insert(now_us);
                if now_us.saturating_sub(since) < NEUTRAL_HOLD_US {
                    return false;
                }
                *self = Self::Running { started_us: now_us };
                true
            }
            Self::Immediate | Self::Running { .. } => false,
        }
    }

    pub const fn output_enabled(&self) -> bool {
        match self {
            Self::Waiting { .. } => false,
            Self::Immediate | Self::Running { .. } => true,
        }
    }

    pub const fn remaining_us(&self, now_us: u64, seconds: u64) -> Option<u64> {
        let elapsed = match self {
            Self::Immediate => now_us,
            Self::Waiting { .. } => return None,
            Self::Running { started_us } => now_us.saturating_sub(*started_us),
        };
        Some(seconds.saturating_mul(1_000_000).saturating_sub(elapsed))
    }
}
