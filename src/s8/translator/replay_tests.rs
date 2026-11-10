use std::collections::{BTreeSet, HashMap};

use anyhow::Context as _;
use encdr::DeviceId;
use encdr::device::loader::DescriptorRegistry;
use encdr::device::parser::PacketParser;
use s4mk3_hid_midi_bridge::device::S8;

use super::Translator;

const FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/s8-input-reports.jsonl"
));

fn bytes(hex: &str) -> anyhow::Result<Vec<u8>> {
    (0..hex.len())
        .step_by(2)
        .map(|index| {
            let pair = hex.get(index..index + 2).context("odd hex length")?;
            u8::from_str_radix(pair, 16).context("hex digit")
        })
        .collect()
}

fn reports(step: &str) -> anyhow::Result<Vec<Vec<u8>>> {
    for line in FIXTURE.lines() {
        let value: serde_json::Value = serde_json::from_str(line)?;
        if value["step"] == step {
            return value["reports"]
                .as_array()
                .context("reports")?
                .iter()
                .map(|hex| bytes(hex.as_str().context("hex string")?))
                .collect();
        }
    }
    anyhow::bail!("no fixture step {step}")
}

/// The MIDI the translator sends after the resting report of a captured step,
/// decoded as the bridge does.
fn replay(step: &str) -> anyhow::Result<Vec<[u8; 3]>> {
    let mut registry = DescriptorRegistry::new();
    registry.load_builtins()?;
    let original = registry
        .find(S8.vendor_id, S8.product_id)
        .context("S8 descriptor missing")?;
    let descriptor = (S8.descriptor.control)(original);
    let names: HashMap<String, &'static str> = registry.intern_descriptor_names(&descriptor);
    let mut parser = PacketParser::new(DeviceId::default(), &descriptor, names);
    let mut translator = Translator::new();
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut events = Vec::new();
    for report in reports(step)? {
        let payload = S8
            .input_payload(&report)?
            .context("report ID the S8 profile does not know")?;
        let resting = seen.insert(report.first().copied());
        events.clear();
        parser.parse_from("control", payload, &mut events);
        let mut sent = Vec::new();
        for event in &events {
            translator.handle(event, resting, &mut sent);
        }
        translator.end_report(&mut sent);
        if !resting {
            out.extend(sent);
        }
    }
    Ok(out)
}

#[test]
fn a_captured_play_press_sends_the_deck_play_note_on_each_side() -> anyhow::Result<()> {
    let twice = |on: [u8; 3], off: [u8; 3]| vec![on, off, on, off];

    assert_eq!(replay("L-play")?, twice([0x90, 0, 127], [0x80, 0, 0]));
    assert_eq!(replay("R-play")?, twice([0x91, 0, 127], [0x81, 0, 0]));
    Ok(())
}

#[test]
fn a_captured_pad_press_sends_the_first_hotcue_note() -> anyhow::Result<()> {
    assert_eq!(
        replay("L-pad-1")?,
        [
            [0x90, 40, 127],
            [0x80, 40, 0],
            [0x90, 40, 127],
            [0x80, 40, 0]
        ]
    );
    Ok(())
}

#[test]
fn a_captured_deck_button_press_toggles_between_decks_a_and_c() -> anyhow::Result<()> {
    assert_eq!(
        replay("L-deck")?,
        [[0x95, 2, 127], [0x85, 2, 0], [0x95, 0, 127], [0x85, 0, 0]]
    );
    Ok(())
}

#[test]
fn a_captured_stem_fader_sweep_sends_changing_values_on_its_cc() -> anyhow::Result<()> {
    let out = replay("L-fader-1")?;

    assert!(!out.is_empty());
    assert!(out.iter().all(|message| message[..2] == [0xb0, 48]));
    assert!(out.windows(2).all(|pair| pair[0][2] != pair[1][2]));
    assert!(out.iter().any(|message| message[2] == 127));
    Ok(())
}

#[test]
fn a_captured_strip_touch_is_bracketed_by_its_touch_note() -> anyhow::Result<()> {
    let out = replay("L-strip-touch")?;
    let (first, rest) = out.split_first().context("no output")?;
    let (last, motion) = rest.split_last().context("no motion")?;

    assert_eq!(*first, [0x90, 7, 127]);
    assert_eq!(*last, [0x80, 7, 0]);
    assert!(!motion.is_empty());
    assert!(motion.iter().all(|message| message[..2] == [0xb0, 5]));
    assert!(motion.iter().any(|message| (62..=64).contains(&message[2])));
    Ok(())
}

#[test]
fn a_captured_crossfader_assign_move_pulses_left_then_ends_in_the_middle() -> anyhow::Result<()> {
    let out = replay("M-A-xf-left")?;
    let pulses: Vec<u8> = out
        .iter()
        .filter(|message| message[0] == 0x94 && (80..=82).contains(&message[1]))
        .map(|message| message[1])
        .collect();

    assert!(pulses.contains(&80));
    assert_eq!(pulses.last(), Some(&81));
    for pair in out.chunks(2) {
        assert_eq!(pair[0][0], 0x94);
        assert_eq!(pair[1], [0x84, pair[0][1], 0]);
    }
    Ok(())
}

#[test]
fn a_captured_filter_on_press_centres_the_filter_knob_of_channel_a() -> anyhow::Result<()> {
    let out = replay("M-A-filteron")?;

    assert_eq!(out.first(), Some(&[0xb4, 21, 64]));
    assert!(out.iter().all(|message| message[..2] == [0xb4, 21]));
    Ok(())
}
