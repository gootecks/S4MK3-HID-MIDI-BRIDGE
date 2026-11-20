//! Attended S8 mixer CUE LED probe. Only an explicit `--mask` sends a report.

#[path = "s8-led-probe/hid.rs"]
mod hid;

use std::time::{Duration, Instant};

use anyhow::{Context as _, ensure};
use clap::Parser;
use hidapi::HidDevice;

const CUE_BITS: u8 = 0x0F;
const BUTTON_REPORT_ID: u8 = 0x01;
const CUE_LATCH_INDEX: usize = 39; // Wire index, including the report ID.

#[derive(Parser)]
#[command(about = "Attended S8 CUE LED probe (writes require explicit approval)")]
#[group(required = true, multiple = true)]
struct Args {
    /// Send one CUE LED mask (0–15 or 0x00–0x0F; A=1, B=2, C=4, D=8)
    #[arg(long, value_parser = parse_mask)]
    mask: Option<u8>,
    /// Read CUE latch changes for this many seconds, after the write if both are set
    #[arg(long, value_name = "SECONDS", value_parser = clap::value_parser!(u32).range(1..))]
    read_latch: Option<u32>,
}

fn parse_mask(value: &str) -> Result<u8, std::num::ParseIntError> {
    value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(|| value.parse(), |hex| u8::from_str_radix(hex, 16))
}

fn cue_frame(mask: u8) -> anyhow::Result<[u8; 33]> {
    ensure!(mask <= CUE_BITS, "CUE LED mask must be 0x00–0x0F");
    let mut frame = [0; 33];
    frame[0] = 0xF4;
    frame[1] = 0x26;
    frame[2] = mask;
    Ok(frame)
}

fn cue_latch(report: &[u8]) -> Option<u8> {
    if report.first() != Some(&BUTTON_REPORT_ID) {
        return None;
    }
    report.get(CUE_LATCH_INDEX).map(|byte| byte & CUE_BITS)
}

fn read_latch(device: &HidDevice, seconds: u32) -> anyhow::Result<()> {
    let mut report = [0; 512];
    let mut last = None;
    let started = Instant::now();
    let duration = Duration::from_secs(u64::from(seconds));
    println!("Reading the CUE latch for {seconds} seconds (A=1, B=2, C=4, D=8)");
    while started.elapsed() < duration {
        let len = device
            .read_timeout(&mut report, 100)
            .context("read S8 input report")?;
        if let Some(mask) = cue_latch(&report[..len])
            && last != Some(mask)
        {
            println!(
                "{:.3}s CUE latch: 0x{mask:02X}",
                started.elapsed().as_secs_f64()
            );
            last = Some(mask);
        }
    }
    ensure!(
        last.is_some(),
        "no S8 button report with a CUE latch received"
    );
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let frame = args.mask.map(cue_frame).transpose()?;
    let device = hid::open()?;
    if let Some(frame) = frame {
        device
            .send_feature_report(&frame)
            .context("send S8 CUE LED feature report")?;
        println!("sent F4 26 {:02X} + 30 zero bytes", frame[2]);
    }
    if let Some(seconds) = args.read_latch {
        read_latch(&device, seconds)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_is_one_feature_report_with_zero_padding() -> anyhow::Result<()> {
        for mask in 0..=0x0F {
            let frame = cue_frame(mask)?;
            assert_eq!(frame.len(), 33);
            assert_eq!(frame[..3], [0xF4, 0x26, mask]);
            assert_eq!(frame[3..], [0; 30]);
        }
        Ok(())
    }

    #[test]
    fn frame_rejects_masks_outside_cue_bits() {
        for mask in [0x10, 0x80, 0xFF] {
            assert!(cue_frame(mask).is_err());
        }
    }

    #[test]
    fn latch_uses_only_button_report_and_low_nibble() {
        let mut report = [0; 41];
        report[0] = 0x01;
        report[39] = 0x35;
        assert_eq!(cue_latch(&report), Some(0x05));
        assert_eq!(cue_latch(&report[..39]), None);
        report[0] = 0x02;
        assert_eq!(cue_latch(&report), None);
    }

    #[test]
    fn cli_requires_a_mode_and_allows_combining_modes() -> anyhow::Result<()> {
        assert!(Args::try_parse_from(["probe"]).is_err());
        assert!(Args::try_parse_from(["probe", "--read-latch", "0"]).is_err());
        let read = Args::try_parse_from(["probe", "--read-latch", "10"])?;
        assert_eq!(read.mask, None);
        assert_eq!(read.read_latch, Some(10));
        let both = Args::try_parse_from(["probe", "--mask", "0x0F", "--read-latch", "10"])?;
        assert_eq!(both.mask, Some(0x0F));
        assert_eq!(both.read_latch, Some(10));
        Ok(())
    }
}
