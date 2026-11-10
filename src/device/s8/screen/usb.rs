//! The S8 screen interface over nusb.
//!
//! `claim` opens the one connected S8 and claims interface 6 without touching
//! the configuration, resetting the device or detaching a driver. `send` is
//! one bulk OUT transfer on endpoint `0x04` with a 500 ms timeout and no
//! retry. `release` drops the endpoint, interface and device.

use std::time::Duration;

use anyhow::{Context as _, ensure};
use nusb::transfer::{Buffer, Bulk, Completion, Out};
use nusb::{Device, Endpoint, Interface, MaybeFuture as _};

use super::worker::Transport;
use super::{ENDPOINT, FRAME_BYTES, INTERFACE};
use crate::device::S8;

const TIMEOUT: Duration = Duration::from_millis(500);

/// Fields drop in order: endpoint, interface, device.
struct Claim {
    endpoint: Endpoint<Bulk, Out>,
    buffer: Option<Buffer>,
    _interface: Interface,
    _device: Device,
}

#[derive(Default)]
pub struct Usb {
    claim: Option<Claim>,
}

impl Transport for Usb {
    fn claim(&mut self) -> anyhow::Result<()> {
        if self.claim.is_some() {
            return Ok(());
        }
        let mut devices = nusb::list_devices()
            .wait()
            .context("list USB devices")?
            .filter(|device| S8.matches(device.vendor_id(), device.product_id()));
        let info = devices.next().context("no connected Kontrol S8")?;
        ensure!(devices.next().is_none(), "connect only one Kontrol S8");
        let device = info.open().wait().context("open S8 USB device")?;
        let interface = device
            .claim_interface(INTERFACE)
            .wait()
            .context("claim S8 screen interface 6; NIHardwareAgent may hold it")?;
        let endpoint = interface
            .endpoint::<Bulk, Out>(ENDPOINT)
            .context("open S8 screen bulk OUT 0x04")?;
        let buffer = Some(endpoint.allocate(FRAME_BYTES));
        self.claim = Some(Claim {
            endpoint,
            buffer,
            _interface: interface,
            _device: device,
        });
        Ok(())
    }

    fn send(&mut self, frame: &[u8]) -> anyhow::Result<()> {
        let claim = self
            .claim
            .as_mut()
            .context("S8 screen interface is not claimed")?;
        let mut buffer = claim
            .buffer
            .take()
            .unwrap_or_else(|| claim.endpoint.allocate(FRAME_BYTES));
        ensure!(
            frame.len() <= buffer.capacity(),
            "S8 screen frame of {} bytes exceeds the {FRAME_BYTES}-byte buffer",
            frame.len()
        );
        buffer.clear();
        buffer.extend_from_slice(frame);
        let Completion {
            buffer,
            actual_len,
            status,
        } = claim.endpoint.transfer_blocking(buffer, TIMEOUT);
        claim.buffer = Some(buffer);
        status.context("S8 screen bulk transfer")?;
        ensure!(
            actual_len == frame.len(),
            "short S8 screen write: {actual_len}/{}",
            frame.len()
        );
        Ok(())
    }

    fn release(&mut self) {
        self.claim = None;
    }
}
