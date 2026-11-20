//! Writes changed deck and mixer CUE LED reports and clears them at exit.

use std::time::Duration;

use anyhow::{Result, ensure};
use tracing::warn;

use super::leds::{self, DECK_REPORT_LEN, Side, SideLeds};

/// Minimum time between two writes of one side's report, as on the S4 path.
const MIN_INTERVAL: Duration = Duration::from_millis(10);

/// Where reports go; the HID device in the bridge, a recorder in tests.
pub trait ReportSink {
    /// Writes one report (ID first) and returns the number of bytes accepted.
    fn write_report(&self, report: &[u8]) -> Result<usize>;

    /// Sends a feature report (ID first).
    fn write_feature(&self, report: &[u8]) -> Result<()>;
}

impl ReportSink for hidapi::HidDevice {
    fn write_report(&self, report: &[u8]) -> Result<usize> {
        Ok(self.write(report)?)
    }

    fn write_feature(&self, report: &[u8]) -> Result<()> {
        Ok(self.send_feature_report(report)?)
    }
}

pub struct LedOutput<'a, S: ReportSink> {
    sink: &'a S,
    last: [Option<[u8; DECK_REPORT_LEN]>; 2],
    sent_at: [Option<Duration>; 2],
    last_cue_mask: Option<u8>,
    finished: bool,
}

impl<'a, S: ReportSink> LedOutput<'a, S> {
    pub const fn new(sink: &'a S) -> Self {
        Self {
            sink,
            last: [None; 2],
            sent_at: [None; 2],
            last_cue_mask: None,
            finished: false,
        }
    }

    fn write(&self, report: &[u8; DECK_REPORT_LEN]) -> Result<()> {
        let written = self.sink.write_report(report)?;
        ensure!(
            written == report.len(),
            "short LED report write: {written} of {} bytes for report 0x{:02x}",
            report.len(),
            report[0]
        );
        Ok(())
    }

    fn write_cue_mask(&self, mask: u8) -> Result<()> {
        let mut report = [0; 33];
        report[0] = 0xf4;
        report[1] = 0x26;
        report[2] = mask;
        self.sink.write_feature(&report)
    }

    /// Writes changed deck reports when pacing permits and the mixer CUE
    /// report when Djay supplies a new mask. Returns the number written.
    pub fn tick(
        &mut self,
        leds: &[SideLeds; 2],
        mixer_cue_mask: Option<u8>,
        now: Duration,
    ) -> Result<usize> {
        let mut written = 0;
        for side in Side::ALL {
            let index = side.index();
            let report = leds::render(side, &leds[index]);
            if self.last[index] == Some(report) {
                continue;
            }
            let paced =
                self.sent_at[index].is_some_and(|sent| now.saturating_sub(sent) < MIN_INTERVAL);
            if paced {
                continue;
            }
            self.write(&report)?;
            self.last[index] = Some(report);
            self.sent_at[index] = Some(now);
            written += 1;
        }
        if let Some(mask) = mixer_cue_mask
            && self.last_cue_mask != Some(mask)
        {
            self.write_cue_mask(mask)?;
            self.last_cue_mask = Some(mask);
            written += 1;
        }
        Ok(written)
    }

    /// Clears both decks and, if written, the mixer CUE latch. Returns the first error.
    pub fn finish(&mut self) -> Result<()> {
        self.finished = true;
        let mut first = Ok(());
        for side in Side::ALL {
            let result = self.write(&leds::clear(side));
            self.last[side.index()] = None;
            if first.is_ok() {
                first = result;
            }
        }
        if self.last_cue_mask.take().is_some() {
            let result = self.write_cue_mask(0);
            if first.is_ok() {
                first = result;
            }
        }
        first
    }
}

impl<S: ReportSink> Drop for LedOutput<'_, S> {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if let Err(error) = self.finish() {
            warn!(%error, "S8_LEDS_CLEAR_FAILED");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::s8::leds::{Level, Rgb};

    #[derive(Default)]
    struct Recorder {
        reports: RefCell<Vec<Vec<u8>>>,
        features: RefCell<Vec<Vec<u8>>>,
        short_by: Cell<usize>,
    }

    impl ReportSink for Recorder {
        fn write_report(&self, report: &[u8]) -> Result<usize> {
            self.reports.borrow_mut().push(report.to_vec());
            Ok(report.len() - self.short_by.get())
        }

        fn write_feature(&self, report: &[u8]) -> Result<()> {
            self.features.borrow_mut().push(report.to_vec());
            Ok(())
        }
    }

    const fn at(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn lit() -> [SideLeds; 2] {
        let mut leds = [SideLeds::default(); 2];
        leds[0].play = Level::On;
        leds[1].pads[0] = Rgb::new(1, 2, 3);
        leds
    }

    fn ids(recorder: &Recorder) -> Vec<u8> {
        recorder.reports.borrow().iter().map(|r| r[0]).collect()
    }

    #[test]
    fn the_first_tick_writes_both_sides() -> Result<()> {
        let recorder = Recorder::default();
        let mut output = LedOutput::new(&recorder);
        assert_eq!(output.tick(&lit(), None, at(0))?, 2);
        assert_eq!(ids(&recorder), [0x80, 0x81]);
        assert!(
            recorder
                .reports
                .borrow()
                .iter()
                .all(|r| r.len() == DECK_REPORT_LEN)
        );
        output.finish()?;
        Ok(())
    }

    #[test]
    fn unchanged_reports_are_not_written_again() -> Result<()> {
        let recorder = Recorder::default();
        let mut output = LedOutput::new(&recorder);
        output.tick(&lit(), None, at(0))?;
        assert_eq!(output.tick(&lit(), None, at(50))?, 0);
        assert_eq!(output.tick(&lit(), None, at(500))?, 0);
        assert_eq!(recorder.reports.borrow().len(), 2);
        output.finish()?;
        Ok(())
    }

    #[test]
    fn only_the_changed_side_is_written() -> Result<()> {
        let recorder = Recorder::default();
        let mut output = LedOutput::new(&recorder);
        output.tick(&lit(), None, at(0))?;
        let mut next = lit();
        next[1].shift = Level::On;
        assert_eq!(output.tick(&next, None, at(20))?, 1);
        assert_eq!(ids(&recorder), [0x80, 0x81, 0x81]);
        output.finish()?;
        Ok(())
    }

    #[test]
    fn a_changed_report_waits_for_the_pacing_interval_and_is_not_lost() -> Result<()> {
        let recorder = Recorder::default();
        let mut output = LedOutput::new(&recorder);
        output.tick(&lit(), None, at(0))?;
        let mut next = lit();
        next[0].play = Level::Off;
        assert_eq!(output.tick(&next, None, at(9))?, 0);
        assert_eq!(output.tick(&next, None, at(10))?, 1);
        assert_eq!(ids(&recorder), [0x80, 0x81, 0x80]);
        output.finish()?;
        Ok(())
    }

    #[test]
    fn finish_writes_zeros_for_both_sides_once() -> Result<()> {
        let recorder = Recorder::default();
        let mut output = LedOutput::new(&recorder);
        output.tick(&lit(), None, at(0))?;
        output.finish()?;
        drop(output);
        assert!(recorder.features.borrow().is_empty());
        let reports = recorder.reports.borrow();
        assert_eq!(reports.len(), 4);
        for (report, side) in reports[2..].iter().zip(Side::ALL) {
            assert_eq!(report.as_slice(), leds::clear(side).as_slice());
        }
        Ok(())
    }

    #[test]
    fn dropping_without_finish_clears_both_sides() -> Result<()> {
        let recorder = Recorder::default();
        {
            let mut output = LedOutput::new(&recorder);
            output.tick(&lit(), None, at(0))?;
        }
        let reports = recorder.reports.borrow();
        assert_eq!(reports.len(), 4);
        assert_eq!(reports[2].as_slice(), leds::clear(Side::Left).as_slice());
        assert_eq!(reports[3].as_slice(), leds::clear(Side::Right).as_slice());
        Ok(())
    }

    #[test]
    fn a_short_write_is_an_error() -> Result<()> {
        let recorder = Recorder::default();
        recorder.short_by.set(1);
        let mut output = LedOutput::new(&recorder);
        let Err(error) = output.tick(&lit(), None, at(0)) else {
            anyhow::bail!("short write was accepted");
        };
        assert!(error.to_string().contains("short LED report write"));
        recorder.short_by.set(0);
        output.finish()?;
        Ok(())
    }

    #[test]
    fn mixer_cue_report_is_written_only_on_first_feedback_change_and_exit() -> Result<()> {
        let recorder = Recorder::default();
        let mut output = LedOutput::new(&recorder);
        assert_eq!(output.tick(&lit(), None, at(0))?, 2);
        assert!(recorder.features.borrow().is_empty());
        assert_eq!(output.tick(&lit(), Some(0), at(1))?, 1);
        assert_eq!(output.tick(&lit(), Some(0), at(2))?, 0);
        assert_eq!(output.tick(&lit(), Some(0x0d), at(3))?, 1);
        assert_eq!(output.tick(&lit(), Some(0x0d), at(4))?, 0);
        output.finish()?;
        drop(output);
        let features = recorder.features.borrow();
        assert_eq!(features.len(), 3);
        for (report, mask) in features.iter().zip([0, 0x0d, 0]) {
            let mut expected = [0; 33];
            expected[..3].copy_from_slice(&[0xf4, 0x26, mask]);
            assert_eq!(report.as_slice(), expected);
        }
        Ok(())
    }

    #[test]
    fn finish_attempts_both_sides_and_reports_a_short_write() {
        let recorder = Recorder::default();
        let mut output = LedOutput::new(&recorder);
        recorder.short_by.set(1);
        assert!(output.finish().is_err());
        assert_eq!(recorder.reports.borrow().len(), 2);
        drop(output);
        assert_eq!(
            recorder.reports.borrow().len(),
            2,
            "no second clear on drop"
        );
    }
}
