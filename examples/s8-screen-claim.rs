//! Screen phase 1: claim S8 interface 6 for five seconds, then release it.
//!
//! Sends nothing: no transfers, no configuration change, no reset, no
//! alternate setting. Refuses to run while `NIHardwareAgent` holds the panels.

use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail, ensure};
use nusb::MaybeFuture as _;
use s4mk3_hid_midi_bridge::device::S8;

const AGENT: &str = "/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app/Contents/MacOS/NIHardwareAgent";
const SCREEN_INTERFACE: u8 = 6;
const HOLD: Duration = Duration::from_secs(5);

fn main() -> anyhow::Result<()> {
    let agent = Command::new("pgrep")
        .args(["-f", AGENT])
        .output()
        .context("run pgrep")?;
    if !agent.stdout.is_empty() {
        bail!(
            "NIHardwareAgent is running (pid {}); stop it first",
            String::from_utf8_lossy(&agent.stdout).trim()
        );
    }

    let mut devices = nusb::list_devices()
        .wait()?
        .filter(|device| S8.matches(device.vendor_id(), device.product_id()));
    let info = devices.next().context("no connected Kontrol S8")?;
    ensure!(devices.next().is_none(), "connect only one Kontrol S8");

    let started = Instant::now();
    let device = info.open().wait().context("open S8 USB device")?;
    println!("{:.3} opened device", started.elapsed().as_secs_f64());
    let interface = device
        .claim_interface(SCREEN_INTERFACE)
        .wait()
        .context("claim S8 interface 6")?;
    println!(
        "{:.3} claimed interface {SCREEN_INTERFACE}; holding {}s",
        started.elapsed().as_secs_f64(),
        HOLD.as_secs()
    );
    thread::sleep(HOLD);
    drop(interface);
    drop(device);
    println!("{:.3} released", started.elapsed().as_secs_f64());
    Ok(())
}
