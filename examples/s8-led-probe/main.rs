//! Send the LED check plan's output reports to the S8, one step at a time.
//!
//! `--dump` and `--list` never open the device. Only `--step` and `--clear`
//! write, and only `0x80` and `0x81` output reports, plus `0x82` with
//! `--allow-mixer`. There is no feature report path.

mod hid;
mod plan;

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::{Context as _, bail};
use clap::Parser;
use hidapi::HidDevice;

use plan::{Plan, allowed_reports, clear_wires, dump};

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
struct Mode {
    /// Validate the plan and print every step; opens no device
    #[arg(long)]
    dump: bool,
    /// Print the step ids, one per line; opens no device
    #[arg(long)]
    list: bool,
    /// Send one step's writes in order
    #[arg(long, value_name = "ID")]
    step: Option<String>,
    /// Send all-zero reports
    #[arg(long)]
    clear: bool,
}

#[derive(Parser)]
struct Args {
    #[command(flatten)]
    mode: Mode,
    /// Plan file
    #[arg(long, default_value = "docs/s8/led-check-plan.json")]
    plan: PathBuf,
    /// With --list: include optional steps
    #[arg(long, requires = "list")]
    all: bool,
    /// Accept the 0x82 mixer report in the plan and in --clear
    #[arg(long)]
    allow_mixer: bool,
    /// Pause between the writes of one step
    #[arg(long, default_value_t = 500)]
    pause_ms: u64,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let reports = allowed_reports(args.allow_mixer);
    let load = || -> anyhow::Result<Plan> {
        let text = std::fs::read_to_string(&args.plan)
            .with_context(|| format!("read {}", args.plan.display()))?;
        Plan::parse(&text, reports).with_context(|| format!("plan {}", args.plan.display()))
    };
    if args.mode.dump {
        print!("{}", dump(&load()?)?);
    } else if args.mode.list {
        for id in load()?.ids(args.all) {
            println!("{id}");
        }
    } else if let Some(id) = &args.mode.step {
        let plan = load()?;
        let Some(step) = plan.step(id) else {
            bail!("no step {id} in {}", args.plan.display());
        };
        let wires: Vec<&[u8]> = step
            .writes
            .iter()
            .map(|write| write.wire.as_slice())
            .collect();
        send(
            &hid::open()?,
            &wires,
            Duration::from_millis(args.pause_ms),
            true,
        )?;
    } else {
        let wires = clear_wires(reports);
        let slices: Vec<&[u8]> = wires.iter().map(Vec::as_slice).collect();
        send(&hid::open()?, &slices, Duration::ZERO, false)?;
    }
    Ok(())
}

fn send(
    device: &HidDevice,
    wires: &[&[u8]],
    pause: Duration,
    stop_on_error: bool,
) -> anyhow::Result<()> {
    let mut failures = Vec::new();
    for (index, wire) in wires.iter().enumerate() {
        if index > 0 {
            thread::sleep(pause);
        }
        let report = wire.first().copied().unwrap_or_default();
        match device.write(wire) {
            Ok(written) => {
                println!(
                    "write {}: report 0x{report:02x}, requested {}, returned {written}",
                    index + 1,
                    wire.len()
                );
                if written != wire.len() {
                    failures.push(format!("report 0x{report:02x}: short write {written}"));
                }
            }
            Err(error) => {
                println!("write {}: report 0x{report:02x}, error {error}", index + 1);
                failures.push(format!("report 0x{report:02x}: {error}"));
            }
        }
        if stop_on_error && !failures.is_empty() {
            break;
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        bail!("{}", failures.join("; "))
    }
}
