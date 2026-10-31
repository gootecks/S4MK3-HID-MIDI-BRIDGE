//! Field tables for the S8 input recorder: kontrol-s8-protocol and Encdr descriptors.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Context as _;
use serde_json::Value;

/// One decoded field: `len` little-endian bytes at `offset` (report ID is byte 0), masked.
pub struct Group {
    pub offset: usize,
    pub len: usize,
    pub mask: u32,
    pub modulus: Option<u32>,
    pub heartbeat: bool,
    pub names: Vec<String>,
}

impl Group {
    const fn shift(&self) -> u32 {
        self.mask.trailing_zeros()
    }

    pub fn value(&self, report: &[u8]) -> Option<u32> {
        let bytes = report.get(self.offset..self.offset + self.len)?;
        let raw = bytes.iter().enumerate().fold(0_u32, |acc, (index, byte)| {
            acc | (u32::from(*byte) << (8 * index))
        });
        Some((raw & self.mask) >> self.shift())
    }

    pub const fn is_analog(&self) -> bool {
        self.mask.count_ones() > 1 && self.modulus.is_none()
    }
}

pub type Key = (u8, usize, usize, u32);

#[derive(Default)]
pub struct Fields {
    pub groups: BTreeMap<Key, Group>,
}

impl Fields {
    fn add(
        &mut self,
        (report, offset, len, mask): Key,
        modulus: Option<u32>,
        heartbeat: bool,
        name: String,
    ) {
        let group = self
            .groups
            .entry((report, offset, len, mask))
            .or_insert_with(|| Group {
                offset,
                len,
                mask,
                modulus,
                heartbeat,
                names: Vec::new(),
            });
        group.heartbeat |= heartbeat;
        group.modulus = group.modulus.or(modulus);
        group.names.push(name);
    }

    pub fn load_protocol(&mut self, path: &Path) -> anyhow::Result<()> {
        let rows: Value = read_json(path)?;
        for row in rows.as_array().context("HID_INPUT.json is not an array")? {
            let (Some(id), Some(offset), Some(width), Some(name)) = (
                row["report_id"].as_u64(),
                row["byte_offset_with_report_id"].as_u64(),
                row["field_width"].as_u64(),
                row["control_id"].as_str(),
            ) else {
                continue;
            };
            let (id, offset, width) = (
                u8::try_from(id)?,
                usize::try_from(offset)?,
                usize::try_from(width)?,
            );
            let full = if width >= 2 { 0xFFFF } else { 0xFF };
            // Without an explicit mask, share the mask of a descriptor field at the same bytes
            // (Encdr gives 12/10-bit widths) so one physical field prints one line.
            let shared = self
                .groups
                .keys()
                .find(|key| (key.0, key.1, key.2) == (id, offset, width))
                .map(|key| key.3);
            let fallback = shared.unwrap_or(full);
            let mask = row["bit_mask"]
                .as_u64()
                .map_or(fallback, |mask| u32::try_from(mask).unwrap_or(full));
            let modulus = row["modulus"].as_u64().and_then(|m| u32::try_from(m).ok());
            self.add(
                (id, offset, width, mask),
                modulus,
                false,
                format!("P:{name}"),
            );
        }
        Ok(())
    }

    pub fn load_encdr(&mut self, path: &Path) -> anyhow::Result<()> {
        let descriptor: Value = read_json(path)?;
        for packet in descriptor["input_packets"].as_array().into_iter().flatten() {
            let report = match packet["id"].as_str() {
                Some("buttons") => 1,
                Some("sliders") => 2,
                _ => continue,
            };
            for item in packet["items"].as_array().into_iter().flatten() {
                self.load_encdr_item(report, item);
            }
        }
        let heartbeat = &descriptor["quirks"]["touchstrip"]["heartbeat_bytes"];
        for (name, bytes) in heartbeat.as_object().into_iter().flatten() {
            if let Some(offset) = bytes[0].as_u64().and_then(|b| usize::try_from(b).ok()) {
                self.add(
                    (1, offset, 2, 0xFFFF),
                    None,
                    true,
                    format!("E:{name}_heartbeat"),
                );
            }
        }
        Ok(())
    }

    fn load_encdr_item(&mut self, report: u8, item: &Value) {
        let Some(name) = item["name"].as_str() else {
            return;
        };
        let bits = item["bits"].as_u64().and_then(|b| u32::try_from(b).ok());
        let bit_offset = item["bit_offset"]
            .as_u64()
            .and_then(|b| u32::try_from(b).ok())
            .unwrap_or(0);
        let low = item["bytes"][0].as_u64().or_else(|| item["byte"].as_u64());
        let Some(offset) = low.and_then(|b| usize::try_from(b).ok()) else {
            return;
        };
        let len = if item["bytes"].is_array() { 2 } else { 1 };
        let kind = item["type"].as_str().unwrap_or_default();
        let full = if len == 2 { 0xFFFF } else { 0xFF };
        let (mask, modulus) = match (kind, bits) {
            ("encoder_fine", _) => (0xFFFF, Some(0x1_0000)),
            ("encoder", Some(bits)) => (((1_u32 << bits) - 1) << bit_offset, Some(1 << bits)),
            (_, Some(bits)) if bits < 16 => ((1_u32 << bits) - 1, None),
            (_, _) => (
                item["mask"]
                    .as_str()
                    .and_then(|m| u32::from_str_radix(m.trim_start_matches("0x"), 16).ok())
                    .unwrap_or(full),
                None,
            ),
        };
        self.add(
            (report, offset, len, mask),
            modulus,
            false,
            format!("E:{name}"),
        );
        let touch = &item["on_touch"];
        if let (Some(byte), Some(mask)) = (
            touch["byte"].as_u64().and_then(|b| usize::try_from(b).ok()),
            touch["mask"]
                .as_str()
                .and_then(|m| u32::from_str_radix(m.trim_start_matches("0x"), 16).ok()),
        ) {
            self.add(
                (report, byte, 1, mask),
                None,
                false,
                format!("E:{name}_touch"),
            );
        }
    }
}

fn read_json(path: &Path) -> anyhow::Result<Value> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}
