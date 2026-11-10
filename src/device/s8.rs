//! Kontrol S8 corrections to the pinned Encdr descriptor, and the screen
//! encoders Encdr cannot decode.
//!
//! Byte offsets include the report ID, as Encdr's S8 packets count them.
//! Evidence for every field is in `docs/s8/input-capture.md`.

use encdr::DeviceDescriptor;
use encdr::core::descriptor::{
    ButtonItemDesc, EncoderEncoding, EncoderItemDesc, HexU16, InputItemDesc, QuirksDesc,
    SliderItemDesc, TouchItemDesc,
};

/// Touch caps of the left deck; the right deck repeats them eight bytes on.
const TOUCH_CAPS: [(&str, usize, u16); 14] = [
    ("loop_encoder_touch", 10, 0x01),
    ("screen_encoder_touch_1", 10, 0x02),
    ("screen_encoder_touch_2", 10, 0x04),
    ("screen_encoder_touch_3", 10, 0x08),
    ("screen_encoder_touch_4", 10, 0x10),
    ("browse_encoder_touch", 10, 0x20),
    ("fx_dial_touch_1", 10, 0x40),
    ("fx_dial_touch_2", 10, 0x80),
    ("fader_touch_1", 11, 0x01),
    ("fader_touch_2", 11, 0x02),
    ("fader_touch_3", 11, 0x04),
    ("fader_touch_4", 11, 0x08),
    ("fx_dial_touch_3", 11, 0x10),
    ("fx_dial_touch_4", 11, 0x20),
];

/// Crossfader assign per channel A-D: the bit set at the left position and
/// the bit set in the middle. The right position sets neither.
const CROSSFADER_ASSIGN: [(u16, u16); 4] = [(0x20, 0x10), (0x01, 0x02), (0x80, 0x40), (0x04, 0x08)];

/// Input only: without an LED group, screen or quirk the bridge has nothing
/// to write to the S8.
#[must_use]
pub fn control_descriptor(original: &DeviceDescriptor) -> DeviceDescriptor {
    let mut descriptor = original.clone();
    descriptor
        .interfaces
        .retain(|interface| interface.id == "control");
    descriptor.screens.clear();
    descriptor.leds.clear();
    descriptor.quirks = QuirksDesc::default();
    for packet in &mut descriptor.input_packets {
        match packet.id.as_str() {
            "buttons" => correct_buttons(&mut packet.items),
            "sliders" => correct_sliders(&mut packet.items),
            _ => {}
        }
    }
    descriptor
}

fn correct_buttons(items: &mut Vec<InputItemDesc>) {
    // Encdr's touchstrip touch bits are a pad and a counter, and its
    // crossfader assign masks swap channels B and D.
    items.retain(|item| match item {
        InputItemDesc::Touch(_) => false,
        InputItemDesc::Button(button) => !button.name.starts_with("xfader_assign_"),
        _ => true,
    });
    for item in items.iter_mut() {
        if let InputItemDesc::Slider(strip) = item {
            // A touchstrip position reaches 1024.
            strip.bits = 11;
        }
    }
    for (side, encoders, deck, strip) in [("left", 1, 0, 28), ("right", 2, 8, 34)] {
        items.push(encoder(format!("{side}_browse_encoder"), encoders, 4));
        items.push(encoder(format!("{side}_loop_encoder"), encoders, 0));
        items.push(button(
            format!("{side}_browse_encoder_press"),
            4 + deck,
            0x08,
        ));
        items.push(button(format!("{side}_loop_encoder_press"), 7 + deck, 0x10));
        for (name, byte, mask) in TOUCH_CAPS {
            items.push(InputItemDesc::Touch(TouchItemDesc {
                name: format!("{side}_{name}"),
                byte: Some(byte + deck),
                bytes: None,
                mask: Some(HexU16(mask)),
            }));
        }
        // The strip has no touch bit: a position other than 0 is a touch.
        items.push(InputItemDesc::Touch(TouchItemDesc {
            name: format!("{side}_touchstrip_touch"),
            byte: None,
            bytes: Some(vec![strip, strip + 1]),
            mask: None,
        }));
    }
    for (index, (channel, (left, middle))) in ["a", "b", "c", "d"]
        .into_iter()
        .zip(CROSSFADER_ASSIGN)
        .enumerate()
    {
        items.push(button(format!("xfader_assign_left_{channel}"), 24, left));
        items.push(button(
            format!("xfader_assign_middle_{channel}"),
            24,
            middle,
        ));
        // Toggle states the S8 keeps itself, flipped once per press.
        items.push(button(
            format!("mixer_filter_on_{channel}_latch"),
            38,
            0x10 << index,
        ));
        items.push(button(
            format!("mixer_cue_{channel}_latch"),
            39,
            0x01 << index,
        ));
    }
    items.push(button("mixer_mic_1_latch".to_owned(), 39, 0x10));
    items.push(button("mixer_mic_2_latch".to_owned(), 39, 0x20));
}

fn correct_sliders(items: &mut Vec<InputItemDesc>) {
    // Screen encoders wrap at 1000 (`ScreenEncoders`). Encdr's FX dial touch
    // offsets are position bytes here; the caps are in the buttons report.
    items.retain(|item| matches!(item, InputItemDesc::Slider(_)));
    // Encdr has no entry for the four knobs between the channel strips.
    for (name, low_byte) in [
        ("mixer_main", 59),
        ("mixer_booth", 61),
        ("mixer_cue_mix", 63),
        ("mixer_cue_volume", 65),
    ] {
        items.push(InputItemDesc::Slider(SliderItemDesc {
            name: name.to_owned(),
            byte: None,
            bytes: Some(vec![low_byte, low_byte + 1]),
            bits: 12,
            normalize: true,
            max_value: None,
        }));
    }
    for item in items {
        let InputItemDesc::Slider(slider) = item else {
            continue;
        };
        let low_byte = match slider.name.as_str() {
            "mixer_eq_low_c" => Some(95),
            "mixer_eq_low_d" => Some(105),
            _ => None,
        };
        if let Some(low_byte) = low_byte {
            slider.byte = None;
            slider.bytes = Some(vec![low_byte, low_byte + 1]);
            slider.bits = 12;
        }
        // Full scale is 4074-4095 depending on the control.
        slider.max_value = Some(4095);
    }
}

const fn button(name: String, byte: usize, mask: u16) -> InputItemDesc {
    InputItemDesc::Button(ButtonItemDesc {
        name,
        byte,
        mask: HexU16(mask),
        category: None,
    })
}

const fn encoder(name: String, byte: usize, bit_offset: u8) -> InputItemDesc {
    InputItemDesc::Encoder(EncoderItemDesc {
        name,
        byte,
        bits: 4,
        bit_offset,
        encoding: EncoderEncoding::Wrap16,
    })
}

/// Counts in one turn of a screen encoder; the position wraps here.
pub const SCREEN_ENCODER_COUNTS: i32 = 1000;

const SCREEN_ENCODERS: [&str; 8] = [
    "left_screen_encoder_1",
    "left_screen_encoder_2",
    "left_screen_encoder_3",
    "left_screen_encoder_4",
    "right_screen_encoder_1",
    "right_screen_encoder_2",
    "right_screen_encoder_3",
    "right_screen_encoder_4",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenEncoderTurn {
    pub name: &'static str,
    /// Counts moved by the shortest path; clockwise is positive.
    pub delta: i32,
    pub position: u16,
}

/// Decodes the eight screen encoders from report 2, where each holds a
/// little-endian position from 0 to 999.
#[derive(Debug, Default)]
pub struct ScreenEncoders {
    positions: Option<[u16; 8]>,
}

impl ScreenEncoders {
    /// The encoders that moved since the previous report 2. The first report
    /// only records where they are, and other reports are ignored.
    pub fn update(&mut self, report: &[u8]) -> impl Iterator<Item = ScreenEncoderTurn> + use<> {
        let moved = positions(report)
            .and_then(|positions| Some((self.positions.replace(positions)?, positions)));
        moved.into_iter().flat_map(|(previous, positions)| {
            SCREEN_ENCODERS
                .into_iter()
                .zip(previous)
                .zip(positions)
                .filter_map(|((name, before), position)| {
                    let half = SCREEN_ENCODER_COUNTS / 2;
                    let delta = (i32::from(position) - i32::from(before) + half)
                        .rem_euclid(SCREEN_ENCODER_COUNTS)
                        - half;
                    (delta != 0).then_some(ScreenEncoderTurn {
                        name,
                        delta,
                        position,
                    })
                })
        })
    }
}

fn positions(report: &[u8]) -> Option<[u16; 8]> {
    let fields = report.strip_prefix(&[2])?.first_chunk::<16>()?;
    let mut positions = [0_u16; 8];
    for (position, bytes) in positions.iter_mut().zip(fields.as_chunks::<2>().0) {
        *position = u16::from_le_bytes(*bytes);
    }
    Some(positions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::S8;
    use anyhow::Context as _;
    use encdr::device::loader::DescriptorRegistry;

    #[test]
    fn the_s8_descriptor_leaves_nothing_to_write_or_claim_beyond_hid_input() -> anyhow::Result<()> {
        // Given the S8 descriptor from the pinned dependency.
        let mut registry = DescriptorRegistry::new();
        registry.load_builtins()?;
        let original = registry
            .find(S8.vendor_id, S8.product_id)
            .context("S8 descriptor missing")?;

        // When the bridge prepares it.
        let descriptor = control_descriptor(original);

        // Then only HID interface 5 remains and no output is described.
        let [interface] = descriptor.interfaces.as_slice() else {
            anyhow::bail!("expected one interface");
        };
        assert_eq!(interface.number, 5);
        assert!(descriptor.leds.is_empty());
        assert!(descriptor.screens.is_empty());
        assert!(descriptor.quirks.feature_report_leds.is_none());
        assert!(descriptor.quirks.init_writes.is_empty());
        assert!(
            descriptor
                .input_packets
                .iter()
                .all(|packet| packet.interface == interface.id)
        );
        Ok(())
    }

    fn report(positions: [u16; 8]) -> Vec<u8> {
        let mut report = vec![2];
        for position in positions {
            report.extend(position.to_le_bytes());
        }
        report.resize(109, 0);
        report
    }

    #[test]
    fn screen_encoders_take_the_short_way_round_the_1000_count_ring() {
        // Given encoders resting at known positions.
        let mut encoders = ScreenEncoders::default();
        let seeded = encoders
            .update(&report([998, 2, 500, 0, 0, 0, 0, 999]))
            .count();
        assert_eq!(seeded, 0, "the first report only records positions");

        // When one passes 999 clockwise, one passes 0 anticlockwise, one
        // moves mid-range and a button report arrives in between.
        let mut turns: Vec<_> = encoders.update(&[1; 41]).collect();
        turns.extend(encoders.update(&report([3, 997, 480, 0, 0, 0, 0, 999])));

        // Then each reports its signed movement and the rest stay silent.
        assert_eq!(
            turns,
            [
                ScreenEncoderTurn {
                    name: "left_screen_encoder_1",
                    delta: 5,
                    position: 3
                },
                ScreenEncoderTurn {
                    name: "left_screen_encoder_2",
                    delta: -5,
                    position: 997
                },
                ScreenEncoderTurn {
                    name: "left_screen_encoder_3",
                    delta: -20,
                    position: 480
                },
            ]
        );
    }
}
