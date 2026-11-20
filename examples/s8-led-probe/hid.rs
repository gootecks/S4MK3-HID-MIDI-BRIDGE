use anyhow::{Context as _, bail};
use hidapi::{HidApi, HidDevice};
use s4mk3_hid_midi_bridge::device::S8;

const HID_INTERFACE: i32 = 5;

/// The S8 HID collection on interface 5, opened in shared mode like the recorder.
pub fn open() -> anyhow::Result<HidDevice> {
    let api = HidApi::new().context("initialize macOS-native HID access")?;
    let mut found = api
        .device_list()
        .filter(|info| S8.matches(info.vendor_id(), info.product_id()));
    let first = found.next();
    let mut count = usize::from(first.is_some());
    let match_interface = first
        .filter(|info| info.interface_number() == HID_INTERFACE)
        .or_else(|| {
            found.find(|info| {
                count += 1;
                info.interface_number() == HID_INTERFACE
            })
        });
    let chosen = match match_interface {
        Some(info) => info,
        None => match first {
            Some(only) if count == 1 && only.interface_number() < 0 => only,
            _ => bail!(
                "no S8 HID collection on interface {HID_INTERFACE} ({count} S8 collections seen)"
            ),
        },
    };
    println!(
        "device: interface {}, usage page 0x{:04x}, usage 0x{:04x}",
        chosen.interface_number(),
        chosen.usage_page(),
        chosen.usage()
    );
    api.open_path(chosen.path())
        .context("open S8 HID collection in shared mode")
}
