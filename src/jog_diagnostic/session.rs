//! One motor writer owns startup, decisions, and neutral cleanup.

use std::io::Write;

use anyhow::{bail, ensure};
use tracing::warn;

use super::ControlArgs;
use super::capture::{LogRecord, RawFrame, Summary, emit};
use super::decode::{Decoder, FRESHNESS_US};
use super::motor::{Controller, neutral_report};

pub enum Event {
    Sample(RawFrame),
    Stop,
    Deadline,
    InputTimeout,
    ReaderError(anyhow::Error),
}

pub struct Session<'a, F: FnMut(&[u8]) -> anyhow::Result<usize>> {
    writer: F,
    log: &'a mut dyn Write,
    decoder: Decoder,
    controller: Controller,
    summary: Summary,
    finished: bool,
}

impl<'a, F: FnMut(&[u8]) -> anyhow::Result<usize>> Session<'a, F> {
    pub fn new(control: ControlArgs, writer: F, log: &'a mut dyn Write) -> anyhow::Result<Self> {
        let mut session = Self {
            writer,
            log,
            decoder: Decoder::default(),
            controller: Controller::new(control),
            summary: Summary::default(),
            finished: false,
        };
        // The guard exists before even the startup neutral can fail.
        session.write_report(neutral_report(), "startup_neutral")?;
        Ok(session)
    }

    pub fn handle(&mut self, event: Event, now_us: u64) -> anyhow::Result<bool> {
        let result = self.process(event, now_us);
        if let Err(error) = result {
            if let Err(cleanup) = self.finish("error") {
                return Err(error.context(format!("neutral cleanup also failed: {cleanup:#}")));
            }
            return Err(error);
        }
        result
    }

    fn process(&mut self, event: Event, now_us: u64) -> anyhow::Result<bool> {
        match event {
            Event::Sample(frame) => {
                ensure!(
                    now_us >= frame.host_us && now_us - frame.host_us <= FRESHNESS_US,
                    "queued input is older than 50ms or from the future"
                );
                emit(
                    self.log,
                    &LogRecord::Raw {
                        frame: frame.clone(),
                    },
                )?;
                let motion = self.decoder.decode(&frame)?;
                self.summary.observe(&motion);
                let decision = self.controller.decide(&motion);
                emit(self.log, &LogRecord::Motion { motion, decision })?;
                self.write_report(decision.report, "motion")?;
                Ok(true)
            }
            Event::Stop => {
                self.finish("ctrl_c")?;
                Ok(false)
            }
            Event::Deadline => {
                self.finish("duration")?;
                Ok(false)
            }
            Event::InputTimeout => bail!("no fresh input within 50ms"),
            Event::ReaderError(error) => Err(error),
        }
    }

    pub fn finish(&mut self, reason: &str) -> anyhow::Result<()> {
        if !self.finished {
            self.write_report(neutral_report(), "final_neutral")?;
            emit(self.log, &LogRecord::end(reason, self.summary))?;
            self.log.flush()?;
            self.finished = true;
        }
        Ok(())
    }

    fn write_report(&mut self, report: [u8; 11], reason: &str) -> anyhow::Result<()> {
        let result = (self.writer)(&report);
        let written = result.as_ref().ok().copied();
        let error = match &result {
            Ok(length) if *length != report.len() => {
                Some(format!("short motor write: {length}/11"))
            }
            Ok(_) => None,
            Err(error) => Some(format!("{error:#}")),
        };
        let logging = emit(
            self.log,
            &LogRecord::Motor {
                reason: reason.to_owned(),
                report,
                written,
                error,
            },
        );
        let hardware = result.and_then(|length| {
            ensure!(length == report.len(), "short motor write: {length}/11");
            Ok(())
        });
        match (hardware, logging) {
            (Ok(()), logging) => logging,
            (Err(error), Ok(())) => Err(error),
            (Err(error), Err(logging)) => {
                Err(error.context(format!("motor log also failed: {logging:#}")))
            }
        }
    }
}

impl<F: FnMut(&[u8]) -> anyhow::Result<usize>> Drop for Session<'_, F> {
    fn drop(&mut self) {
        if !self.finished
            && let Err(error) = self.write_report(neutral_report(), "drop_neutral")
        {
            warn!(%error, "DIAGNOSTIC_NEUTRAL_FAILED");
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
