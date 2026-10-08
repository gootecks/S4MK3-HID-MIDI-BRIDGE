//! Offline and explicitly opted-in native wheel-tension diagnostics.

mod capture;
mod decode;
mod live;
mod motor;
mod ownership;
mod reader;
mod session;
#[cfg(test)]
mod tests;

use std::fs::File;
use std::io::{BufRead as _, BufReader, BufWriter};
use std::path::PathBuf;

use anyhow::{Context as _, ensure};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use capture::{LogRecord, Summary, emit};
use decode::Decoder;
use motor::Controller;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Prepare and record supervised S4 MK3 wheel-tension experiments"
)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Capture buttons/jog snapshots without sending any output reports.
    Capture {
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=120))]
        seconds: u64,
        /// Include fresh mixer/fader snapshots (numbered report 2).
        #[arg(long)]
        analog: bool,
    },
    /// Decode a capture and calculate torque offline; never opens HID.
    Replay {
        #[arg(long)]
        input: PathBuf,
        #[command(flatten)]
        control: ControlArgs,
    },
    /// Run a supervised trial; motor output requires explicit opt-in.
    #[command(visible_alias = "trial")]
    Live {
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..=30))]
        seconds: u64,
        #[arg(long)]
        enable_motor: bool,
        #[command(flatten)]
        control: ControlArgs,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Baseline,
    Brake,
    Assist,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum Side {
    Left,
    Right,
}

impl Side {
    const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, clap::Args, Deserialize, Serialize)]
struct ControlArgs {
    #[arg(long, value_enum)]
    mode: Mode,
    /// Physical wheel, not a logical A/B/C/D deck.
    #[arg(long, value_enum, default_value = "left")]
    side: Side,
    /// Raw magnitude units per count/ms; not a Traktor tension value.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=100))]
    gain: u32,
    /// Uncalibrated magnitude ceiling; live trials further limit this to 500.
    #[arg(long, visible_alias = "force-cap", default_value_t = 100, value_parser = clap::value_parser!(u16).range(1..=1000))]
    max_force: u16,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    match args.command {
        Command::Capture {
            out,
            seconds,
            analog,
        } => live::capture(&out, seconds, analog),
        Command::Replay { input, control } => replay(&input, control),
        Command::Live {
            out,
            seconds,
            enable_motor,
            control,
        } => {
            ensure!(
                enable_motor,
                "live trials require --enable-motor, including baseline"
            );
            ensure!(control.max_force <= 500, "live --max-force must be <=500");
            live::trial(&out, seconds, control)
        }
    }
}

fn replay(path: &std::path::Path, control: ControlArgs) -> anyhow::Result<()> {
    let input = BufReader::new(File::open(path).context("open raw capture")?);
    let mut output = BufWriter::new(std::io::stdout().lock());
    let mut decoder = Decoder::default();
    let controller = Controller::new(control);
    let mut summary = Summary::default();
    emit(&mut output, &LogRecord::session("replay", Some(control)))?;
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
            let motion = decoder.decode(&frame)?;
            summary.observe(&motion);
            let decision = controller.decide(&motion);
            emit(&mut output, &LogRecord::Motion { motion, decision })?;
        }
    }
    ensure!(summary.samples > 0, "capture contains no raw samples");
    emit(&mut output, &LogRecord::end("replay_eof", summary))
}
