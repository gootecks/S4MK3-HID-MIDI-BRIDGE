//! S4 MK3 corrections to the pinned Encdr descriptor.

use anyhow::Context as _;
use encdr::DeviceDescriptor;
use encdr::core::descriptor::{ButtonItemDesc, HexU16, InputItemDesc, SliderItemDesc};

pub fn control_descriptor(original: &DeviceDescriptor) -> DeviceDescriptor {
    let mut descriptor = original.clone();
    descriptor
        .interfaces
        .retain(|interface| interface.id == "control");
    descriptor.screens.clear();
    descriptor.leds.retain(|group| group.id == "buttons");
    // Payload sizes from this S4's HID report descriptor (report ID excluded).
    // The pinned Encdr descriptor has incorrect sizes for buttons and jogs.
    for packet in &mut descriptor.input_packets {
        match packet.id.as_str() {
            "buttons" => packet.size = 22,
            "jogwheels" => packet.size = 59,
            _ => {}
        }
        // S4 mixer faders use 12-bit values in 16-bit fields (Mixxx: 4095).
        // Correct only this stage's mapped faders, not the broader descriptor.
        for item in &mut packet.items {
            if let InputItemDesc::Slider(slider) = item
                && matches!(
                    slider.name.as_str(),
                    "ch1_fader" | "ch2_fader" | "ch3_fader" | "ch4_fader" | "crossfader"
                )
            {
                slider.max_value = Some(4095);
            }
        }
    }
    descriptor
}

pub fn full_control_descriptor(original: &DeviceDescriptor) -> anyhow::Result<DeviceDescriptor> {
    let mut descriptor = control_descriptor(original);
    descriptor.leds.extend(
        original
            .leds
            .iter()
            .filter(|group| matches!(group.id.as_str(), "vu_meters" | "wheel_leds"))
            .cloned(),
    );
    for packet in &mut descriptor.input_packets {
        for item in &mut packet.items {
            match item {
                InputItemDesc::Slider(slider) => slider.max_value = Some(4095),
                InputItemDesc::Button(button) => {
                    // Captured physical presses: only the LEFT names are reversed.
                    match button.name.as_str() {
                        "left_loop_encoder_press" => {
                            "left_move_encoder_press".clone_into(&mut button.name);
                        }
                        "left_move_encoder_press" => {
                            "left_loop_encoder_press".clone_into(&mut button.name);
                        }
                        _ => {}
                    }
                    if button.name.starts_with("left_pad_") || button.name.starts_with("right_pad_")
                    {
                        let index = button
                            .name
                            .rsplit('_')
                            .next()
                            .context("pad number missing")?
                            .parse::<usize>()?
                            .checked_sub(1)
                            .context("pad numbers start at 1")?;
                        let bit = [5_u8, 4, 7, 6, 3, 2, 1, 0]
                            .get(index)
                            .context("pad number outside 1-8")?;
                        button.mask = HexU16(1_u16 << bit);
                    }
                }
                InputItemDesc::Encoder(encoder) => {
                    // All four rotation names are reversed; preserve wire polarity.
                    let name = match encoder.name.as_str() {
                        "left_loop_encoder" => Some("left_move_encoder"),
                        "left_move_encoder" => Some("left_loop_encoder"),
                        "right_loop_encoder" => Some("right_move_encoder"),
                        "right_move_encoder" => Some("right_loop_encoder"),
                        _ => None,
                    };
                    if let Some(name) = name {
                        name.clone_into(&mut encoder.name);
                    }
                }
                InputItemDesc::EncoderFine(_) | InputItemDesc::Touch(_) => {}
            }
        }
        if packet.id == "buttons" {
            for (name, byte, mask) in [
                ("mixer_ch3_ext", 2, 0x02),
                ("mixer_ch1_ext", 11, 0x01),
                ("mixer_ch2_ext", 11, 0x02),
                ("mixer_ch4_ext", 11, 0x04),
            ] {
                packet.items.push(InputItemDesc::Button(ButtonItemDesc {
                    name: name.to_owned(),
                    byte,
                    mask: HexU16(mask),
                    category: None,
                }));
            }
        }
        if packet.id == "sliders" {
            packet.items.push(InputItemDesc::Slider(SliderItemDesc {
                name: "headphone_gain".to_owned(),
                byte: None,
                bytes: Some(vec![26, 27]),
                bits: 16,
                normalize: true,
                max_value: Some(4095),
            }));
        }
    }
    Ok(descriptor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::S4_MK3;
    use encdr::device::loader::DescriptorRegistry;

    #[test]
    fn full_descriptor_corrects_all_pots_and_physical_pad_order() -> anyhow::Result<()> {
        // Given the pinned hardware layout.
        let mut registry = DescriptorRegistry::new();
        registry.load_builtins()?;
        let original = registry
            .find(S4_MK3.vendor_id, S4_MK3.product_id)
            .context("S4 descriptor missing")?;
        // When the full profile prepares it.
        let descriptor = full_control_descriptor(original)?;
        let mut pads = 0;
        let mut fields = 0;
        // Then every pot uses full ADC range and pad masks follow physical order.
        for item in descriptor
            .input_packets
            .iter()
            .flat_map(|packet| &packet.items)
        {
            fields += 1;
            match item {
                InputItemDesc::Slider(slider) => assert_eq!(slider.max_value, Some(4095)),
                InputItemDesc::Button(button)
                    if button.name.starts_with("left_pad_")
                        || button.name.starts_with("right_pad_") =>
                {
                    let index = button
                        .name
                        .rsplit('_')
                        .next()
                        .context("pad suffix missing")?
                        .parse::<usize>()?
                        - 1;
                    assert_eq!(button.mask.0, 1 << [5, 4, 7, 6, 3, 2, 1, 0][index]);
                    pads += 1;
                }
                _ => {}
            }
        }
        assert_eq!(pads, 16);
        assert_eq!(fields, 147);
        Ok(())
    }

    #[test]
    fn mapped_mixer_faders_use_the_s4_adc_range() -> anyhow::Result<()> {
        // Given the pinned S4 descriptor.
        let mut registry = DescriptorRegistry::new();
        registry.load_builtins()?;
        let original = registry
            .find(S4_MK3.vendor_id, S4_MK3.product_id)
            .context("S4 descriptor missing")?;
        // When the probe prepares its control descriptor.
        let descriptor = control_descriptor(original);
        // Then the five mapped faders normalize 12-bit full scale to 1.0.
        let mut checked = 0_u8;
        for item in descriptor
            .input_packets
            .iter()
            .flat_map(|packet| &packet.items)
        {
            if let InputItemDesc::Slider(slider) = item
                && matches!(
                    slider.name.as_str(),
                    "ch1_fader" | "ch2_fader" | "ch3_fader" | "ch4_fader" | "crossfader"
                )
            {
                assert!(slider.normalize);
                assert_eq!(slider.bits, 16);
                assert_eq!(slider.max_value, Some(4095));
                checked = checked.saturating_add(1);
            }
        }
        assert_eq!(checked, 5);
        Ok(())
    }

    #[test]
    fn probe_restricts_usb_access_and_output_to_controls() -> anyhow::Result<()> {
        // Given the actual descriptor from the pinned dependency.
        let mut registry = DescriptorRegistry::new();
        registry.load_builtins()?;
        let original = registry
            .find(S4_MK3.vendor_id, S4_MK3.product_id)
            .context("S4 descriptor missing")?;

        // When the probe narrows it for this stage.
        let descriptor = control_descriptor(original);

        // Then audio/screens/motor outputs cannot be claimed or written.
        assert_eq!(descriptor.interfaces.len(), 1);
        let interface = descriptor
            .interfaces
            .first()
            .context("control interface missing")?;
        assert_eq!(interface.number, 3);
        assert!(descriptor.screens.is_empty());
        assert_eq!(descriptor.leds.len(), 1);
        assert!(descriptor.leds.iter().all(|group| group.id == "buttons"));
        assert!(
            descriptor
                .input_packets
                .iter()
                .all(|packet| packet.interface == interface.id)
        );
        assert!(descriptor.quirks.init_writes.is_empty());
        assert!(descriptor.quirks.feature_report_leds.is_none());
        Ok(())
    }
}
