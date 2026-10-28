//! Supported controller identities and the per-device seams the bridge selects by.

mod s4mk3;

use anyhow::{Context as _, bail, ensure};
use clap::ValueEnum;
use encdr::DeviceDescriptor;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Device {
    #[value(name = "s4mk3")]
    S4Mk3,
    S8,
}

impl Device {
    #[must_use]
    pub const fn profile(self) -> &'static DeviceProfile {
        match self {
            Self::S4Mk3 => &S4_MK3,
            Self::S8 => &S8,
        }
    }
}

/// A numbered HID input report and its payload length, report ID excluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputReport {
    pub id: u8,
    pub payload_len: usize,
}

/// Corrections applied to the pinned Encdr descriptor before parsing input.
#[derive(Clone, Copy, Debug)]
pub struct DescriptorCorrections {
    /// Probe and minimal MIDI stage: control interface and button LEDs only.
    pub control: fn(&DeviceDescriptor) -> DeviceDescriptor,
    /// Full MIDI profile.
    pub full: fn(&DeviceDescriptor) -> anyhow::Result<DeviceDescriptor>,
}

#[derive(Debug)]
pub struct DeviceProfile {
    pub device: Device,
    /// Human-readable controller name used in messages.
    pub name: &'static str,
    /// Short prefix for machine-readable log tokens such as `S4_USB_FOUND`.
    pub tag: &'static str,
    pub vendor_id: u16,
    pub product_id: u16,
    pub input_reports: &'static [InputReport],
    /// `None` means the bridge only identifies this controller.
    pub descriptor: Option<DescriptorCorrections>,
    /// Virtual MIDI endpoint of the full profile and its Djay mapping.
    pub full_endpoint: &'static str,
    pub mapping_file_name: &'static str,
}

pub const S4_MK3: DeviceProfile = DeviceProfile {
    device: Device::S4Mk3,
    name: "S4 MK3",
    tag: "S4",
    vendor_id: 0x17cc,
    product_id: 0x1720,
    input_reports: &[
        InputReport {
            id: 1,
            payload_len: 22,
        },
        InputReport {
            id: 2,
            payload_len: 78,
        },
        InputReport {
            id: 3,
            payload_len: 59,
        },
    ],
    descriptor: Some(DescriptorCorrections {
        control: s4mk3::control_descriptor,
        full: s4mk3::full_control_descriptor,
    }),
    full_endpoint: "S4 MK3 MIDI Full",
    mapping_file_name: "S4 MK3 Bridge.djayMidiMapping",
};

/// Identification only: input framing and descriptor corrections are unverified.
pub const S8: DeviceProfile = DeviceProfile {
    device: Device::S8,
    name: "Kontrol S8",
    tag: "S8",
    vendor_id: 0x17cc,
    product_id: 0x1370,
    input_reports: &[],
    descriptor: None,
    full_endpoint: "Kontrol S8 MIDI Full",
    mapping_file_name: "Kontrol S8 Bridge.djayMidiMapping",
};

pub const PROFILES: [&DeviceProfile; 2] = [&S4_MK3, &S8];

impl DeviceProfile {
    #[must_use]
    pub const fn matches(&self, vendor_id: u16, product_id: u16) -> bool {
        self.vendor_id == vendor_id && self.product_id == product_id
    }

    /// Strip the report ID from a known input report and check its length.
    ///
    /// # Errors
    /// Returns an error for an empty report or a known report of the wrong length.
    pub fn input_payload<'a>(&self, report: &'a [u8]) -> anyhow::Result<Option<&'a [u8]>> {
        let (id, payload) = report.split_first().context("empty HID report")?;
        let Some(expected) = self
            .input_reports
            .iter()
            .find(|known| known.id == *id)
            .map(|known| known.payload_len)
        else {
            return Ok(None);
        };
        ensure!(
            payload.len() == expected,
            "unexpected {} report {id} payload length: {} (expected {expected})",
            self.tag,
            payload.len()
        );
        Ok(Some(payload))
    }
}

/// Choose the controller to use from connected USB identities.
///
/// A forced device is returned as is; its presence is checked when it is opened.
///
/// # Errors
/// Without a forced device, returns an error when no supported controller or
/// more than one kind of supported controller is connected.
pub fn select(
    forced: Option<Device>,
    connected: impl IntoIterator<Item = (u16, u16)>,
) -> anyhow::Result<&'static DeviceProfile> {
    if let Some(device) = forced {
        return Ok(device.profile());
    }
    let mut found: Vec<&'static DeviceProfile> = Vec::new();
    for (vendor_id, product_id) in connected {
        if let Some(profile) = PROFILES
            .into_iter()
            .find(|profile| profile.matches(vendor_id, product_id))
            && !found.iter().any(|known| known.device == profile.device)
        {
            found.push(profile);
        }
    }
    match found.as_slice() {
        [profile] => Ok(profile),
        [] => bail!(
            "no supported controller found ({}); connect USB and the controller's power adapter, then run the probe again",
            describe(&PROFILES)
        ),
        several => bail!(
            "several supported controllers connected ({}); choose one with --device",
            describe(several)
        ),
    }
}

fn describe(profiles: &[&DeviceProfile]) -> String {
    profiles
        .iter()
        .map(|profile| {
            format!(
                "{} {:04x}:{:04x}",
                profile.name, profile.vendor_id, profile.product_id
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const S4_ID: (u16, u16) = (0x17cc, 0x1720);
    const S8_ID: (u16, u16) = (0x17cc, 0x1370);
    const OTHER_NI: (u16, u16) = (0x17cc, 0x1500);

    #[test]
    fn a_single_connected_kind_is_selected_and_unsupported_devices_are_ignored()
    -> anyhow::Result<()> {
        // Given one supported controller, possibly beside other USB devices.
        for (connected, expected) in [
            (vec![S4_ID], Device::S4Mk3),
            (vec![OTHER_NI, S8_ID], Device::S8),
            (vec![S4_ID, (0x05ac, 0x1720)], Device::S4Mk3),
        ] {
            // When the bridge detects the controller.
            let profile = select(None, connected)?;
            // Then its profile is chosen.
            assert_eq!(profile.device, expected);
        }
        Ok(())
    }

    #[test]
    fn two_units_of_one_kind_select_that_kind_for_the_open_path_to_reject() -> anyhow::Result<()> {
        // Given two S4 MK3 units, detection still names the kind; opening
        // reports the duplicate with the existing per-device message.
        assert_eq!(select(None, [S4_ID, S4_ID])?.device, Device::S4Mk3);
        Ok(())
    }

    #[test]
    fn no_supported_controller_fails_and_names_what_is_supported() {
        // Given only unsupported devices.
        let error = select(None, [OTHER_NI])
            .err()
            .map(|error| error.to_string());
        // Then detection fails clearly.
        let error = error.unwrap_or_default();
        assert!(error.contains("no supported controller found"), "{error}");
        assert!(error.contains("S4 MK3 17cc:1720"), "{error}");
        assert!(error.contains("Kontrol S8 17cc:1370"), "{error}");
    }

    #[test]
    fn different_supported_controllers_require_an_explicit_choice() {
        // Given an S4 MK3 and an S8 connected together.
        let error = select(None, [S8_ID, S4_ID])
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
        // Then detection refuses to guess.
        assert!(error.contains("--device"), "{error}");
        assert!(error.contains("S4 MK3"), "{error}");
        assert!(error.contains("Kontrol S8"), "{error}");
    }

    #[test]
    fn a_forced_device_overrides_detection() -> anyhow::Result<()> {
        for (connected, forced) in [
            (vec![S4_ID, S8_ID], Device::S8),
            (vec![S4_ID, S8_ID], Device::S4Mk3),
            (vec![], Device::S8),
        ] {
            assert_eq!(select(Some(forced), connected)?.device, forced);
        }
        Ok(())
    }

    #[test]
    fn each_profile_has_its_own_identity_endpoint_and_mapping_name() {
        // The S4 identity is unchanged from the original bridge.
        assert_eq!((S4_MK3.vendor_id, S4_MK3.product_id), S4_ID);
        assert_eq!(S4_MK3.full_endpoint, "S4 MK3 MIDI Full");
        assert_eq!(S4_MK3.mapping_file_name, "S4 MK3 Bridge.djayMidiMapping");
        assert_eq!((S8.vendor_id, S8.product_id), S8_ID);
        assert_eq!(S8.full_endpoint, "Kontrol S8 MIDI Full");
        assert_eq!(S8.mapping_file_name, "Kontrol S8 Bridge.djayMidiMapping");
        // The S8 endpoint must not collide with the S8's own native MIDI port,
        // which existing Controller Editor-based Djay mappings bind to.
        assert_ne!(S8.full_endpoint, "Traktor Kontrol S8");
        for (index, profile) in PROFILES.iter().enumerate() {
            assert_eq!(profile.device.profile().device, profile.device);
            for other in PROFILES.iter().skip(index + 1) {
                assert_ne!(profile.full_endpoint, other.full_endpoint);
                assert_ne!(profile.mapping_file_name, other.mapping_file_name);
                assert!(!other.matches(profile.vendor_id, profile.product_id));
            }
        }
    }

    #[test]
    fn s8_is_identification_only() {
        // No input framing or descriptor is trusted until S8 input is verified.
        assert!(S8.descriptor.is_none());
        assert_eq!(S8.input_payload(&[1, 0, 0]).ok().flatten(), None);
    }

    #[test]
    fn numbered_reports_are_stripped_and_checked_before_parsing() -> anyhow::Result<()> {
        // Given numbered S4 reports with their descriptor-defined payload sizes.
        for (id, size) in [(1, 22), (2, 78), (3, 59)] {
            let mut report = vec![id];
            report.resize(size + 1, 0x40);

            // When framing is decoded.
            let payload = S4_MK3
                .input_payload(&report)?
                .context("known report ignored")?;

            // Then only the payload is exposed to Encdr.
            assert_eq!(payload, vec![0x40; size]);
        }
        Ok(())
    }

    #[test]
    fn malformed_known_reports_fail_instead_of_silently_losing_input() {
        // Given a truncated button report.
        let report = [1, 0];
        // When framing is decoded.
        let result = S4_MK3.input_payload(&report);
        // Then malformed input is rejected.
        assert!(result.is_err());
    }
}
