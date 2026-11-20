use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use super::*;

#[test]
fn layouts_keep_distinct_mapping_names_and_the_same_bridge_endpoint() -> Result<()> {
    let evan_name = Layout::Evan.mapping_file_name();
    assert_eq!(evan_name, "Kontrol S8 Bridge Evan.djayMidiMapping");
    assert_ne!(evan_name, Layout::Default.mapping_file_name());
    assert_ne!(
        evan_name,
        "Evan's Traktor Kontrol S8 1.1 DynColor.djayMidiMapping"
    );
    for layout in [Layout::Default, Layout::Evan] {
        let document = generate(layout)?;
        let endpoint = document
            .as_dictionary()
            .and_then(|root| root.get("endpointName"))
            .and_then(Value::as_string);
        assert_eq!(endpoint, Some("Kontrol S8 MIDI Full"));
        assert_ne!(endpoint, Some("Traktor Kontrol S8"));
        assert_ne!(endpoint, Some("Traktor Kontrol S8 Input"));
    }
    Ok(())
}

#[test]
fn default_mapping_is_byte_identical_to_the_frozen_r8() -> Result<()> {
    let mut actual = Vec::new();
    generate(Layout::Default)?.to_writer_xml(&mut actual)?;
    assert_eq!(
        actual.as_slice(),
        include_bytes!("../../../docs/s8/mappings/Kontrol S8 Bridge r8 2026-11-11.djayMidiMapping")
    );
    Ok(())
}

#[test]
fn evan_mapping_round_trips_every_action_address_and_feedback() -> Result<()> {
    type Address = (u64, u64, u64);
    let mut xml = Vec::new();
    generate(Layout::Evan)?.to_writer_xml(&mut xml)?;
    let root: Dictionary = plist::from_reader(Cursor::new(xml))?;
    let controls = root
        .get("controls")
        .and_then(Value::as_array)
        .context("controls array")?;
    let mut rows = BTreeMap::new();
    let mut feedback = BTreeSet::<Address>::new();
    for row in controls {
        let row = row.as_dictionary().context("control dictionary")?;
        let number = |key| {
            row.get(key)
                .and_then(Value::as_unsigned_integer)
                .context(key)
        };
        let address = (
            number("midiChannel")?,
            number("midiMessageType")?,
            number("midiData")?,
        );
        let key = row
            .get("keyPath")
            .and_then(Value::as_string)
            .context("keyPath")?;
        let output = row.get("output").and_then(Value::as_dictionary);
        if let Some(output) = output {
            assert!(
                feedback.insert(address),
                "duplicate feedback address {address:?}"
            );
            if (40..=47).contains(&address.2) {
                assert!(
                    output.is_empty(),
                    "cue colour tokens must not use a value range"
                );
            }
        }
        assert!(
            rows.insert((address, key.to_owned()), output.is_some())
                .is_none()
        );
    }
    for entry in catalog::entries(Layout::Evan) {
        let address = (
            entry.channel().into(),
            entry.binding.kind.message_type().into(),
            entry.binding.number.into(),
        );
        assert_eq!(
            rows.remove(&(address, entry.key_path())),
            Some(catalog::has_embedded_output(&entry.binding))
        );
    }
    assert!(rows.is_empty());
    for deck in 0..4 {
        for cue in 40..=47 {
            assert!(feedback.contains(&(deck, 1, cue)));
        }
    }
    Ok(())
}
