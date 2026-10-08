//! Versioned raw evidence and derived decisions share one replayable format.

use std::io::Write;

use serde::{Deserialize, Serialize};

use super::ControlArgs;
use super::decode::Motion;
use super::motor::{Decision, neutral_report};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawFrame {
    pub host_us: u64,
    pub buttons: Vec<u8>,
    pub jog: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analog: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct Summary {
    pub samples: u64,
    pub net_counts: [i64; 2],
    pub released_counts: [i64; 2],
}

impl Summary {
    pub fn observe(&mut self, motion: &Motion) {
        self.samples += 1;
        for side in 0..2 {
            self.net_counts[side] += i64::from(motion.delta[side]);
            if !motion.touch[side] {
                self.released_counts[side] += i64::from(motion.delta[side]);
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LogRecord {
    Session {
        version: u8,
        source: String,
        control: Option<ControlArgs>,
    },
    Raw {
        frame: RawFrame,
    },
    CaptureEnd {
        samples: u64,
    },
    Motion {
        motion: Motion,
        decision: Decision,
    },
    Motor {
        reason: String,
        report: [u8; 11],
        written: Option<usize>,
        error: Option<String>,
    },
    End {
        reason: String,
        summary: Summary,
        neutral_report: [u8; 11],
    },
}

impl LogRecord {
    pub fn session(source: &str, control: Option<ControlArgs>) -> Self {
        Self::Session {
            version: 1,
            source: source.to_owned(),
            control,
        }
    }

    pub fn end(reason: &str, summary: Summary) -> Self {
        Self::End {
            reason: reason.to_owned(),
            summary,
            neutral_report: neutral_report(),
        }
    }
}

pub fn emit(output: &mut (impl Write + ?Sized), record: &LogRecord) -> anyhow::Result<()> {
    serde_json::to_writer(&mut *output, record)?;
    output.write_all(b"\n")?;
    Ok(())
}
