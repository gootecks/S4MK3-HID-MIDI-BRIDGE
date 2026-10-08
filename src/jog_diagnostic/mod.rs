//! Offline and explicitly opted-in native wheel-tension diagnostics.

mod capture;
mod decode;
mod live;
mod motor;
mod ownership;
mod reader;
mod session;
#[cfg(test)]
mod tension_tests;
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
        control: ControlOptions,
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
        /// Fader modes only: run under supervision until Ctrl-C.
        #[arg(long, conflicts_with = "seconds")]
        until_stopped: bool,
        #[command(flatten)]
        control: ControlOptions,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Baseline,
    Brake,
    Assist,
    FaderBrake,
    FaderTension,
}

impl Mode {
    const fn uses_fader(self) -> bool {
        match self {
            Self::FaderBrake | Self::FaderTension => true,
            Self::Baseline | Self::Brake | Self::Assist => false,
        }
    }
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

#[derive(Debug, clap::Args)]
struct ControlOptions {
    #[arg(long, value_enum)]
    mode: Mode,
    /// Physical wheel, not a logical A/B/C/D deck.
    #[arg(long, value_enum, default_value = "left")]
    side: Side,
    /// Fixed modes only: raw units per count/ms (default 1).
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=100))]
    gain: Option<u32>,
    /// Braking ceiling: default 100 fixed / 5000 fader-brake / 30000 fader-tension.
    #[arg(long, visible_alias = "force-cap", value_parser = clap::value_parser!(u16).range(1..=60000))]
    max_force: Option<u16>,
    /// Fader-tension only: held-touch assistance ceiling, default 8000; zero disables.
    #[arg(long, value_parser = clap::value_parser!(u16).range(0..=8000))]
    assist_force: Option<u16>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct ControlArgs {
    mode: Mode,
    side: Side,
    gain: u32,
    max_force: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    assist_force: Option<u16>,
}

impl ControlOptions {
    fn resolve(self) -> anyhow::Result<ControlArgs> {
        ensure!(
            matches!(self.mode, Mode::FaderTension) || self.assist_force.is_none(),
            "--assist-force requires --mode fader-tension"
        );
        if self.mode.uses_fader() {
            ensure!(
                matches!(self.side, Side::Left),
                "fader modes require --side left and the physical deck1 fader"
            );
            ensure!(
                self.gain.is_none(),
                "fader modes use direct force; omit --gain"
            );
        }
        let max_force = match self.mode {
            Mode::FaderBrake => {
                let force = self.max_force.unwrap_or(5000);
                ensure!(force <= 25000, "fader-brake --max-force must be <=25000");
                force
            }
            Mode::FaderTension => self.max_force.unwrap_or(30000),
            Mode::Baseline | Mode::Brake | Mode::Assist => {
                let max_force = self.max_force.unwrap_or(100);
                ensure!(max_force <= 1000, "fixed-mode --max-force must be <=1000");
                max_force
            }
        };
        Ok(ControlArgs {
            mode: self.mode,
            side: self.side,
            gain: self.gain.unwrap_or(1),
            max_force,
            assist_force: match self.mode {
                Mode::FaderTension => Some(self.assist_force.unwrap_or(8000)),
                Mode::Baseline | Mode::Brake | Mode::Assist | Mode::FaderBrake => None,
            },
        })
    }
}

pub fn run(args: Args) -> anyhow::Result<()> {
    match args.command {
        Command::Capture {
            out,
            seconds,
            analog,
        } => live::capture(&out, seconds, analog),
        Command::Replay { input, control } => replay(&input, control.resolve()?),
        Command::Live {
            out,
            seconds,
            enable_motor,
            until_stopped,
            control,
        } => {
            ensure!(
                enable_motor,
                "live trials require --enable-motor, including baseline"
            );
            let control = control.resolve()?;
            ensure!(
                !until_stopped || control.mode.uses_fader(),
                "--until-stopped requires a fader mode"
            );
            ensure!(
                control.mode.uses_fader() || control.max_force <= 500,
                "fixed live --max-force must be <=500"
            );
            live::trial(
                &out,
                if until_stopped { None } else { Some(seconds) },
                control,
            )
        }
    }
}

fn replay(path: &std::path::Path, control: ControlArgs) -> anyhow::Result<()> {
    let input = BufReader::new(File::open(path).context("open raw capture")?);
    let mut output = BufWriter::new(std::io::stdout().lock());
    let mut decoder = Decoder::default();
    let mut controller = Controller::new(control);
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
            let decision = controller.decide(&motion)?;
            emit(&mut output, &LogRecord::Motion { motion, decision })?;
        }
    }
    ensure!(summary.samples > 0, "capture contains no raw samples");
    emit(&mut output, &LogRecord::end("replay_eof", summary))
}
