//! Generates the native Djay MIDI mapping plist of the Kontrol S8 from the
//! S8 catalog. The assignments are described in `docs/s8/mapping.md`.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;

use anyhow::{Context as _, Result, ensure};
use plist::{Dictionary, Value};

use s4mk3_hid_midi_bridge::device::S8;

use super::catalog::{self, Entry, Kind};

/// The control that owns each address, and the actions already bound to it.
type Addresses = BTreeMap<(u8, u8, u8), (&'static str, Vec<String>)>;

pub fn write(path: &Path) -> Result<()> {
    let mapping = generate()?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("create mapping {}; choose a new file path", path.display()))?;
    mapping
        .to_writer_xml(&mut output)
        .context("serialize Djay mapping")?;
    output.flush().context("flush Djay mapping")?;
    Ok(())
}

fn generate() -> Result<Value> {
    let mut controls = Vec::new();
    let mut addresses = Addresses::new();
    for entry in catalog::entries() {
        push_control(&mut controls, &mut addresses, &entry)?;
    }

    let mut root = Dictionary::new();
    root.insert(
        String::from("endpointName"),
        Value::String(S8.full_endpoint.to_owned()),
    );
    root.insert(String::from("schemeVersion"), Value::Integer(1.into()));
    root.insert(String::from("version"), Value::Integer(0.into()));
    root.insert(String::from("controls"), Value::Array(controls));
    root.insert(
        String::from("outputs"),
        Value::Array(root_outputs(&addresses)?),
    );
    root.insert(
        String::from("userInfo"),
        Value::Dictionary(Dictionary::new()),
    );
    Ok(Value::Dictionary(root))
}

fn push_control(controls: &mut Vec<Value>, addresses: &mut Addresses, entry: &Entry) -> Result<()> {
    let binding = entry.binding;
    let channel = entry.channel();
    let key_path = entry.key_path();
    let message_type = match binding.kind {
        Kind::Note => 1,
        Kind::Absolute | Kind::Relative | Kind::RotaryAbsolute => 3,
    };
    ensure!(
        channel <= 15 && binding.number <= 127 && !key_path.contains('{'),
        "invalid generated MIDI binding for {key_path}"
    );
    let (owner, actions) = addresses
        .entry((channel, message_type, binding.number))
        .or_insert_with(|| (binding.input, Vec::new()));
    ensure!(
        *owner == binding.input && !actions.contains(&key_path),
        "MIDI address of {owner} reused: channel {channel}, type {message_type}, data {} ({} {key_path})",
        binding.number,
        binding.input
    );
    actions.push(key_path.clone());

    let mut control = Dictionary::new();
    control.insert(String::from("keyPath"), Value::String(key_path));
    control.insert(
        String::from("midiChannel"),
        Value::Integer(i64::from(channel).into()),
    );
    control.insert(
        String::from("midiData"),
        Value::Integer(i64::from(binding.number).into()),
    );
    control.insert(
        String::from("midiMessageType"),
        Value::Integer(message_type.into()),
    );
    let control_type = match binding.kind {
        Kind::Relative => Some("rotary-64"),
        Kind::RotaryAbsolute => Some("rotary-absolute"),
        Kind::Note | Kind::Absolute => None,
    };
    if let Some(control_type) = control_type {
        control.insert(
            String::from("controlType"),
            Value::String(control_type.to_owned()),
        );
    }
    if binding.pickup {
        control.insert(String::from("pickupMode"), Value::Boolean(true));
    }
    if binding.flipped {
        control.insert(String::from("flipped"), Value::Boolean(true));
    }
    if let Some(sensitivity) = binding.sensitivity {
        control.insert(String::from("rotarySensitivity"), Value::Real(sensitivity));
    }
    if let Some(acceleration) = binding.acceleration {
        control.insert(
            String::from("rotaryAcceleration"),
            Value::Integer(i64::from(acceleration).into()),
        );
    }
    if catalog::has_embedded_output(&binding) {
        control.insert(
            String::from("output"),
            Value::Dictionary(embedded_output(binding.number)),
        );
    }
    controls.push(Value::Dictionary(control));
    Ok(())
}

/// Djay returns feedback for a control on the control's own address. Cue
/// colour tokens ride in the velocity, so their dictionary stays empty.
fn embedded_output(number: u8) -> Dictionary {
    let mut output = Dictionary::new();
    if !catalog::CUE_COLOUR_NOTES.contains(&number) {
        output.insert(String::from("midiMinValue"), Value::Integer(0.into()));
        output.insert(String::from("midiMaxValue"), Value::Integer(127.into()));
    }
    output
}

/// Feedback with no control of its own, on the dedicated feedback channel.
fn root_outputs(addresses: &Addresses) -> Result<Vec<Value>> {
    let mut outputs = Vec::new();
    for root in catalog::ROOT_OUTPUTS {
        for deck in catalog::DECKS {
            let (channel, message_type, number) = root.address(deck);
            let key_path = root.key_path(deck);
            ensure!(
                !addresses.contains_key(&(channel, message_type, number)),
                "feedback address of {key_path} is an input: channel {channel}, type {message_type}, data {number}"
            );
            let mut output = Dictionary::new();
            output.insert(String::from("keyPath"), Value::String(key_path));
            output.insert(
                String::from("midiChannel"),
                Value::Integer(i64::from(channel).into()),
            );
            output.insert(
                String::from("midiData"),
                Value::Integer(i64::from(number).into()),
            );
            output.insert(
                String::from("midiMessageType"),
                Value::Integer(i64::from(message_type).into()),
            );
            output.insert(String::from("midiMinValue"), Value::Integer(0.into()));
            output.insert(String::from("midiMaxValue"), Value::Integer(127.into()));
            outputs.push(Value::Dictionary(output));
        }
    }
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s8::catalog::{Layer, Scope};
    use std::collections::BTreeSet;
    use std::io::Cursor;

    fn reparsed() -> Result<Dictionary> {
        let mut bytes = Vec::new();
        generate()?
            .to_writer_xml(&mut bytes)
            .context("serialize generated mapping")?;
        let value: Value = plist::from_reader(Cursor::new(bytes)).context("parse mapping")?;
        value.into_dictionary().context("root is a dictionary")
    }

    fn controls_of(root: &Dictionary) -> Result<Vec<&Dictionary>> {
        root.get("controls")
            .and_then(Value::as_array)
            .context("controls is an array")?
            .iter()
            .map(|control| control.as_dictionary().context("control is a dictionary"))
            .collect()
    }

    fn uint(control: &Dictionary, key: &str) -> Result<u8> {
        let value = control
            .get(key)
            .and_then(Value::as_unsigned_integer)
            .with_context(|| format!("{key} is an integer"))?;
        u8::try_from(value).with_context(|| format!("{key} fits a byte"))
    }

    type ByAddress<'a> = BTreeMap<(u8, u8, u8), Vec<&'a Dictionary>>;

    fn by_address<'a>(controls: &[&'a Dictionary]) -> Result<ByAddress<'a>> {
        let mut map = ByAddress::new();
        for control in controls {
            let address = (
                uint(control, "midiChannel")?,
                uint(control, "midiMessageType")?,
                uint(control, "midiData")?,
            );
            map.entry(address).or_default().push(*control);
        }
        Ok(map)
    }

    fn has_key_path(controls: &[&Dictionary], key_path: &str) -> bool {
        controls
            .iter()
            .any(|control| key_path_of(control) == Some(key_path))
    }

    fn message_type(kind: Kind) -> u8 {
        if kind == Kind::Note { 1 } else { 3 }
    }

    fn key_path_of(control: &Dictionary) -> Option<&str> {
        control.get("keyPath").and_then(Value::as_string)
    }

    #[test]
    fn every_catalog_entry_is_generated_once_at_its_address() -> Result<()> {
        let root = reparsed()?;
        let controls = controls_of(&root)?;
        let map = by_address(&controls)?;
        let mut count = 0;
        for entry in catalog::entries() {
            count += 1;
            let address = (
                entry.channel(),
                message_type(entry.binding.kind),
                entry.binding.number,
            );
            let at_address = map
                .get(&address)
                .with_context(|| format!("no control at {address:?}"))?;
            assert!(
                has_key_path(at_address, &entry.key_path()),
                "{} missing at {address:?}",
                entry.key_path()
            );
        }
        assert_eq!(controls.len(), count);
        Ok(())
    }

    #[test]
    fn shift_rows_use_the_shifted_channel_and_the_base_number() -> Result<()> {
        let root = reparsed()?;
        let map = by_address(&controls_of(&root)?)?;
        let mut shifted = 0;
        for deck in catalog::DECKS {
            for shift in catalog::DECK
                .iter()
                .filter(|binding| binding.layer == Layer::Shift)
            {
                let base = catalog::DECK
                    .iter()
                    .find(|binding| {
                        binding.layer == Layer::Base
                            && binding.input == shift.input
                            && binding.number == shift.number
                            && binding.kind == shift.kind
                    })
                    .with_context(|| format!("{} has no base row", shift.input))?;
                let kind = message_type(shift.kind);
                let at_base = map
                    .get(&(deck - 1, kind, shift.number))
                    .with_context(|| format!("{} base missing", shift.input))?;
                let at_shift = map
                    .get(&(deck - 1 + 8, kind, shift.number))
                    .with_context(|| format!("{} shift missing", shift.input))?;
                assert!(has_key_path(at_base, &base.key_path(deck)));
                assert!(has_key_path(at_shift, &shift.key_path(deck)));
                shifted += 1;
            }
        }
        assert!(shifted >= 4);
        Ok(())
    }

    #[test]
    fn encoders_are_relative_and_strips_and_faders_are_not() -> Result<()> {
        let root = reparsed()?;
        let map = by_address(&controls_of(&root)?)?;
        let mut relative = 0;
        for entry in catalog::entries() {
            let binding = entry.binding;
            let address = (entry.channel(), message_type(binding.kind), binding.number);
            for control in map.get(&address).context("entry generated")? {
                let control_type = control.get("controlType").and_then(Value::as_string);
                match binding.kind {
                    Kind::Relative => {
                        relative += 1;
                        assert_eq!(control_type, Some("rotary-64"));
                        assert_eq!(
                            control.get("rotarySensitivity").and_then(Value::as_real),
                            binding.sensitivity
                        );
                    }
                    Kind::Absolute | Kind::Note => assert_eq!(control_type, None),
                    Kind::RotaryAbsolute => assert_ne!(control_type, Some("rotary-64")),
                }
            }
        }
        assert!(relative > 0);
        let tempo = map
            .get(&(catalog::MIXER_CHANNEL, 3, 26))
            .and_then(|controls| controls.first())
            .context("tempo encoder")?;
        assert_eq!(
            tempo.get("rotarySensitivity").and_then(Value::as_real),
            Some(3.5)
        );
        Ok(())
    }

    #[test]
    fn rows_of_one_control_and_layer_share_one_address() {
        let tables = [catalog::DECK, catalog::FX_LEFT, catalog::FX_RIGHT]
            .into_iter()
            .chain(catalog::PadMode::ALL.map(catalog::pads));
        for table in tables {
            let mut numbers = BTreeMap::new();
            let mut actions = BTreeSet::new();
            for row in table {
                let layer = row.layer == Layer::Shift;
                let number = *numbers
                    .entry((row.input, message_type(row.kind), layer))
                    .or_insert(row.number);
                assert_eq!(number, row.number, "{} has two addresses", row.input);
                assert!(actions.insert((row.input, layer, row.target)));
            }
        }
    }

    #[test]
    fn mixer_and_deck_select_rows_land_on_their_channels() -> Result<()> {
        let root = reparsed()?;
        let controls = controls_of(&root)?;
        let mut mixer = 0;
        let mut select = 0;
        for entry in catalog::entries() {
            let channel = match entry.binding.scope {
                Scope::Mixer => {
                    mixer += 1;
                    4
                }
                Scope::DeckSelect => {
                    select += 1;
                    5
                }
                Scope::Deck => continue,
            };
            let key_path = entry.key_path();
            let control = controls
                .iter()
                .find(|control| key_path_of(control) == Some(key_path.as_str()))
                .with_context(|| format!("{key_path} generated"))?;
            assert_eq!(uint(control, "midiChannel")?, channel);
        }
        assert!(mixer > 0);
        assert_eq!(select, 4);
        Ok(())
    }

    #[test]
    fn root_names_the_s8_endpoint() -> Result<()> {
        let root = reparsed()?;
        assert_eq!(
            root.get("endpointName").and_then(Value::as_string),
            Some("Kontrol S8 MIDI Full")
        );
        Ok(())
    }

    fn outputs_of(root: &Dictionary) -> Result<Vec<&Dictionary>> {
        root.get("outputs")
            .and_then(Value::as_array)
            .context("outputs is an array")?
            .iter()
            .map(|output| output.as_dictionary().context("output is a dictionary"))
            .collect()
    }

    #[test]
    fn root_outputs_are_unique_and_clear_of_every_input_address() -> Result<()> {
        let root = reparsed()?;
        let inputs = by_address(&controls_of(&root)?)?;
        let outputs = outputs_of(&root)?;
        assert_eq!(
            outputs.len(),
            catalog::ROOT_OUTPUTS.len() * catalog::DECKS.len()
        );
        let mut seen = BTreeSet::new();
        for output in &outputs {
            let address = (
                uint(output, "midiChannel")?,
                uint(output, "midiMessageType")?,
                uint(output, "midiData")?,
            );
            assert!(seen.insert(address), "duplicate output {address:?}");
            assert!(!inputs.contains_key(&address), "{address:?} is an input");
            assert_eq!(address.0, catalog::FEEDBACK_CHANNEL);
            assert_eq!(uint(output, "midiMinValue")?, 0);
            assert_eq!(uint(output, "midiMaxValue")?, 127);
        }
        for root_output in catalog::ROOT_OUTPUTS {
            for deck in catalog::DECKS {
                let address = root_output.address(deck);
                let key_path = root_output.key_path(deck);
                assert!(seen.contains(&address));
                assert!(
                    outputs
                        .iter()
                        .any(|output| key_path_of(output) == Some(key_path.as_str())
                            && uint(output, "midiData").ok() == Some(address.2))
                );
            }
        }
        Ok(())
    }

    #[test]
    fn embedded_outputs_sit_on_exactly_the_evidenced_base_rows() -> Result<()> {
        let root = reparsed()?;
        let controls = controls_of(&root)?;
        let mut embedded = 0;
        for entry in catalog::entries() {
            let key_path = entry.key_path();
            let channel = entry.channel();
            let control = controls
                .iter()
                .find(|control| {
                    key_path_of(control) == Some(key_path.as_str())
                        && uint(control, "midiChannel").ok() == Some(channel)
                        && uint(control, "midiData").ok() == Some(entry.binding.number)
                })
                .with_context(|| format!("{key_path} generated"))?;
            let output = control.get("output").and_then(Value::as_dictionary);
            if catalog::has_embedded_output(&entry.binding) {
                embedded += 1;
                let output = output.with_context(|| format!("{key_path} has an output"))?;
                if catalog::CUE_COLOUR_NOTES.contains(&entry.binding.number) {
                    assert!(output.is_empty(), "{key_path} carries a colour token");
                } else {
                    assert_eq!(uint(output, "midiMinValue")?, 0);
                    assert_eq!(uint(output, "midiMaxValue")?, 127);
                }
                assert!(channel < 4 && entry.binding.kind == Kind::Note);
            } else {
                assert!(output.is_none(), "{key_path} has no evidenced output");
            }
        }
        assert!(embedded > 0);
        Ok(())
    }

    #[test]
    fn written_file_reads_back_as_the_generated_mapping() -> Result<()> {
        let path = std::env::temp_dir().join(format!(
            "s8-mapping-test-{}.djayMidiMapping",
            std::process::id()
        ));
        write(&path)?;
        assert!(write(&path).is_err());
        let read = Value::from_file(&path).context("read written mapping");
        std::fs::remove_file(&path).context("remove temp mapping")?;
        assert_eq!(read?, generate()?);
        Ok(())
    }
}
