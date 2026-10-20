//! Offline input-time decisions and simulated fixed-clock output proposals.

use std::fs::File;
use std::io::{BufRead as _, BufReader, BufWriter};
use std::path::Path;

use anyhow::{Context as _, ensure};

use super::capture::{LogRecord, Summary, emit};
use super::continuity::OUTPUT_INTERVAL_US;
use super::decode::{Decoder, FRESHNESS_US};
use super::motor::Controller;
use super::{ControlArgs, Mode};

pub fn run(path: &Path, control: ControlArgs) -> anyhow::Result<()> {
    let input = BufReader::new(File::open(path).context("open raw capture")?);
    let mut output = BufWriter::new(std::io::stdout().lock());
    let mut decoder = Decoder::default();
    let mut controller = Controller::new(control);
    let mut summary = Summary::default();
    let mut next_tick = control.mode.is_paced().then_some(OUTPUT_INTERVAL_US);
    let mut last_input = 0_u64;
    emit(&mut output, &LogRecord::session("replay", Some(control)))?;
    if matches!(
        control.mode,
        Mode::FaderDamped | Mode::FaderSmooth | Mode::FaderAssist | Mode::FaderPlay
    ) {
        emit(&mut output, &super::damped::config())?;
    }
    if matches!(
        control.mode,
        Mode::FaderSmooth | Mode::FaderAssist | Mode::FaderPlay
    ) {
        emit(&mut output, &super::smooth::config())?;
    }
    for (index, line) in input.lines().enumerate() {
        let record: LogRecord = serde_json::from_str(&line?)
            .with_context(|| format!("parse capture line {}", index + 1))?;
        if index == 0 {
            ensure!(
                matches!(record, LogRecord::Session { version: 1, .. }),
                "capture must start with a version 1 session"
            );
        }
        if let LogRecord::Raw { frame } = record {
            if control.mode.is_paced() {
                while next_tick.is_some_and(|time| time < frame.host_us) {
                    let time = next_tick.context("missing output time")?;
                    ensure!(
                        time - last_input <= FRESHNESS_US,
                        "no fresh input within 50ms"
                    );
                    if let Some(decision) = controller.output_at(time) {
                        emit(
                            &mut output,
                            &LogRecord::Tick {
                                host_us: time,
                                decision,
                            },
                        )?;
                    }
                    next_tick = Some(time + OUTPUT_INTERVAL_US);
                }
            }
            let motion = decoder.decode(&frame)?;
            summary.observe(&motion);
            let decision = controller.decide(&motion)?;
            emit(&mut output, &LogRecord::Motion { motion, decision })?;
            if control.mode.is_paced() {
                last_input = frame.host_us;
                let time = *next_tick.get_or_insert(frame.host_us);
                if time == frame.host_us {
                    let decision = controller
                        .output_at(time)
                        .context("missing clock decision")?;
                    emit(
                        &mut output,
                        &LogRecord::Tick {
                            host_us: time,
                            decision,
                        },
                    )?;
                    next_tick = Some(time + OUTPUT_INTERVAL_US);
                }
            }
        }
    }
    ensure!(summary.samples > 0, "capture contains no raw samples");
    emit(&mut output, &LogRecord::end("replay_eof", summary))
}
