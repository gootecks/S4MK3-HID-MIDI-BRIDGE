//! Where frames go: the S8's screen endpoint or a dry run that only hashes.

use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail, ensure};
use nusb::transfer::{Bulk, Out};
use nusb::{Device, Interface, MaybeFuture as _};
use s4mk3_hid_midi_bridge::device::S8;
use s4mk3_hid_midi_bridge::device::s8::screen::{ENDPOINT, INTERFACE, Panel};
use sha2::{Digest as _, Sha256};

use crate::pictures::Pictures;

const AGENT: &str = "/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app/Contents/MacOS/NIHardwareAgent";
pub const TIMEOUT: Duration = Duration::from_millis(500);

pub trait Transport {
    /// Sends one frame and returns how long the transfer took.
    fn send(&mut self, seq: usize, panel: Panel, frame: &[u8]) -> anyhow::Result<Duration>;

    fn wait_until(&self, deadline: Instant);

    /// Called before each stage of a job.
    fn stage(&mut self, _info: &StageInfo) {}
}

/// What a stage is about to send.
pub struct StageInfo<'a> {
    pub index: usize,
    pub count: usize,
    pub tag: &'a str,
    /// Transfers in the stage, all panels together.
    pub total: usize,
    pub panels: usize,
}

/// Interface 6 of the one connected S8, claimed once and held until dropped.
pub struct Session {
    endpoint: nusb::Endpoint<Bulk, Out>,
    _interface: Interface,
    _device: Device,
}

impl Session {
    /// Refuses while `NIHardwareAgent` runs or unless exactly one S8 is
    /// connected.
    pub fn open() -> anyhow::Result<Self> {
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

        let device = info.open().wait().context("open S8 USB device")?;
        let interface = device
            .claim_interface(INTERFACE)
            .wait()
            .context("claim S8 interface 6")?;
        let endpoint = interface
            .endpoint::<Bulk, Out>(ENDPOINT)
            .context("open S8 screen bulk OUT 0x04")?;
        Ok(Self {
            endpoint,
            _interface: interface,
            _device: device,
        })
    }
}

impl Transport for Session {
    fn send(&mut self, _seq: usize, _panel: Panel, frame: &[u8]) -> anyhow::Result<Duration> {
        let started = Instant::now();
        let completion = self.endpoint.transfer_blocking(frame.into(), TIMEOUT);
        let took = started.elapsed();
        completion.status.context("S8 screen bulk transfer")?;
        ensure!(
            completion.actual_len == frame.len(),
            "short S8 screen write: {}/{}",
            completion.actual_len,
            frame.len()
        );
        Ok(took)
    }

    fn wait_until(&self, deadline: Instant) {
        sleep(deadline.saturating_duration_since(Instant::now()));
    }
}

/// Opens nothing; records `stage seq panel length sha256` for every frame.
#[derive(Default)]
pub struct DryRun {
    pub rows: Vec<String>,
    pub pictures: Option<Pictures>,
    stage: usize,
}

impl Transport for DryRun {
    fn send(&mut self, seq: usize, panel: Panel, frame: &[u8]) -> anyhow::Result<Duration> {
        self.rows.push(format!(
            "{} {seq} {} {} {:x}",
            self.stage,
            panel.index(),
            frame.len(),
            Sha256::digest(frame)
        ));
        if let Some(pictures) = &mut self.pictures {
            pictures.observe(panel, frame)?;
        }
        Ok(Duration::ZERO)
    }

    fn wait_until(&self, _deadline: Instant) {}

    fn stage(&mut self, info: &StageInfo) {
        self.stage = info.index;
        if let Some(pictures) = &mut self.pictures {
            pictures.stage(info);
        }
    }
}
