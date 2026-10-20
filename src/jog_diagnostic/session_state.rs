//! Operator-visible preparation and time-window state for a guarded session.

use tracing::info;

use super::super::capture::{LogRecord, emit};
use super::super::motor::{Decision, FaderState, FaderZone, neutral_report};
use super::super::window::{NEUTRAL_HOLD_US, TrialWindow};
use super::Session;

impl<F: FnMut(&[u8]) -> anyhow::Result<usize>> Session<'_, F> {
    pub fn prepare(&mut self, seconds: u64) -> anyhow::Result<()> {
        self.prepare_window(Some(seconds))
    }

    pub fn prepare_manual(&mut self) -> anyhow::Result<()> {
        self.prepare_window(None)
    }

    fn prepare_window(&mut self, seconds: Option<u64>) -> anyhow::Result<()> {
        self.window = TrialWindow::Waiting {
            neutral_since_us: None,
        };
        emit(
            self.log,
            &LogRecord::Preparation {
                seconds,
                neutral_hold_us: NEUTRAL_HOLD_US,
            },
        )?;
        info!(
            ?seconds,
            neutral_hold_us = NEUTRAL_HOLD_US,
            "DIAGNOSTIC_FIND_NEUTRAL: outputs neutral; hold channel 1 (second from left) at neutral until READY"
        );
        Ok(())
    }

    pub const fn remaining_duration_us(&self, now_us: u64, seconds: u64) -> Option<u64> {
        self.window.remaining_us(now_us, seconds)
    }

    pub(super) fn update_fader(&mut self, level: FaderState, now_us: u64) -> anyhow::Result<()> {
        let percent = u32::from(level.raw) * 100 / 4095;
        if self.last_fader.is_none_or(|previous| {
            previous.armed != level.armed
                || previous.zone != level.zone
                || (previous.selected_force == 0) != (level.selected_force == 0)
                || u32::from(previous.raw) * 100 / 4095 != percent
        }) {
            info!(
                percent,
                raw = level.raw,
                armed = level.armed,
                selected_force = level.selected_force,
                zone = ?level.zone,
                preparing = !self.window.output_enabled(),
                "DIAGNOSTIC_FADER: zero-force position arms; selection applies only to motion"
            );
        }
        self.last_fader = Some(level);
        if self.window.observe(
            level.armed && level.zone == Some(FaderZone::Neutral),
            now_us,
        ) {
            emit(self.log, &LogRecord::Ready { host_us: now_us })?;
            info!(
                host_us = now_us,
                "DIAGNOSTIC_TRIAL_READY: neutral held; trial active; Ctrl-C neutralizes"
            );
        }
        Ok(())
    }

    pub(super) const fn gate_decision(&self, mut decision: Decision) -> Decision {
        if !self.window.output_enabled() {
            decision.torque = [0, 0];
            decision.report = neutral_report();
        }
        decision
    }
}
