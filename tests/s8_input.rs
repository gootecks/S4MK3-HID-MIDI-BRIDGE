//! Replays Kontrol S8 input reports captured from the controller through the
//! corrected descriptor, the Encdr parser and the screen encoder decoder.
//!
//! Each fixture line is one step of the attended capture: the reports kept
//! from it, and the control that was moved. `docs/s8/input-capture.md`
//! describes the capture and how the steps were reduced.

use anyhow::{Context as _, ensure};
use encdr::device::loader::DescriptorRegistry;
use encdr::device::parser::PacketParser;
use encdr::{DeviceId, Event};
use s4mk3_hid_midi_bridge::device::S8;
use s4mk3_hid_midi_bridge::device::s8::ScreenEncoders;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

const FIXTURE: &str = include_str!("fixtures/s8-input-reports.jsonl");

/// Decoded but not captured: the cap of right deck fader 1 never reported a
/// touch in the capture session.
const UNVERIFIED: [&str; 1] = ["right_fader_touch_1"];

#[derive(Deserialize)]
struct Step {
    #[serde(rename = "step")]
    label: String,
    reports: Vec<String>,
    /// Button, touch and encoder events in order; sorted within one report.
    events: Vec<String>,
    /// Lowest and highest value of each slider that was moved.
    sliders: BTreeMap<String, (f32, f32)>,
}

struct Decoder {
    descriptor: encdr::DeviceDescriptor,
    names: HashMap<String, &'static str>,
}

impl Decoder {
    fn new() -> anyhow::Result<Self> {
        let mut registry = DescriptorRegistry::new();
        registry.load_builtins()?;
        let original = registry
            .find(S8.vendor_id, S8.product_id)
            .context("S8 descriptor missing")?;
        let descriptor = (S8.descriptor.control)(original);
        let names = registry.intern_descriptor_names(&descriptor);
        Ok(Self { descriptor, names })
    }

    /// Decode one step as the bridge does. The first report of each ID only
    /// sets the resting state, as the reports before a control moves do.
    fn replay(&self, step: &Step) -> anyhow::Result<Decoded> {
        let mut parser =
            PacketParser::new(DeviceId::default(), &self.descriptor, self.names.clone());
        let mut screen_encoders = ScreenEncoders::default();
        let mut seen = BTreeSet::new();
        let mut parsed = Vec::new();
        let mut decoded = Decoded::default();
        for hex in &step.reports {
            let report = bytes(hex)?;
            let payload = S8
                .input_payload(&report)?
                .context("report ID the S8 profile does not know")?;
            let resting = seen.insert(report.first().copied());
            let mut events: Vec<String> = screen_encoders
                .update(&report)
                .map(|turn| format!("{}={:+}", turn.name, turn.delta))
                .collect();
            parsed.clear();
            parser.parse_from("control", payload, &mut parsed);
            for event in &parsed {
                match event {
                    Event::Slider { name, value, .. } => {
                        let range = decoded.sliders.entry(name).or_insert((*value, *value));
                        *range = (range.0.min(*value), range.1.max(*value));
                    }
                    Event::Button { name, pressed, .. } => {
                        events.push(format!("{name}={}", if *pressed { "down" } else { "up" }));
                    }
                    Event::Touch { name, touched, .. } => {
                        let state = if *touched { "touch" } else { "release" };
                        events.push(format!("{name}={state}"));
                    }
                    Event::Encoder { name, delta, .. } => events.push(format!("{name}={delta:+}")),
                    other => events.push(format!("{other:?}")),
                }
            }
            if !resting {
                events.sort();
                decoded.events.extend(events);
            }
        }
        Ok(decoded)
    }
}

#[derive(Default)]
struct Decoded {
    events: Vec<String>,
    sliders: BTreeMap<&'static str, (f32, f32)>,
}

fn steps() -> anyhow::Result<Vec<Step>> {
    FIXTURE
        .lines()
        .map(|line| serde_json::from_str(line).context("fixture line"))
        .collect()
}

fn bytes(hex: &str) -> anyhow::Result<Vec<u8>> {
    (0..hex.len())
        .step_by(2)
        .map(|index| {
            let pair = hex.get(index..index + 2).context("odd hex length")?;
            u8::from_str_radix(pair, 16).context("hex digit")
        })
        .collect()
}

#[test]
fn captured_s8_reports_decode_to_the_control_that_was_moved() -> anyhow::Result<()> {
    // Given the corrected S8 descriptor and every step of the capture.
    let decoder = Decoder::new()?;
    for step in steps()? {
        // When the step's reports are decoded.
        let actual = decoder.replay(&step)?;

        // Then the moved control is reported with its captured edges, and
        // nothing else is.
        ensure!(
            actual.events == step.events,
            "{}: decoded {:?}, expected {:?}",
            step.label,
            actual.events,
            step.events
        );
        // And a moved slider spans its captured range while the rest hold still.
        for (name, (low, high)) in &actual.sliders {
            let (expected_low, expected_high) =
                step.sliders.get(*name).copied().unwrap_or((*low, *low));
            ensure!(
                (low - expected_low).abs() <= 0.01 && (high - expected_high).abs() <= 0.01,
                "{}: {name} spanned {low}..{high}, expected {expected_low}..{expected_high}",
                step.label
            );
        }
        for name in step.sliders.keys() {
            ensure!(
                actual.sliders.contains_key(name.as_str()),
                "{}: {name} was not decoded",
                step.label
            );
        }
    }
    Ok(())
}

#[test]
fn every_decoded_s8_control_except_one_dead_cap_has_a_captured_report() -> anyhow::Result<()> {
    // Given the names the corrected descriptor can emit.
    let decoder = Decoder::new()?;
    let decodable: BTreeSet<&str> = decoder.names.keys().map(String::as_str).collect();

    // When they are compared with the controls the capture moved.
    let steps = steps()?;
    let captured: BTreeSet<&str> = steps
        .iter()
        .flat_map(|step| {
            let events = step
                .events
                .iter()
                .filter_map(|event| event.split('=').next());
            events.chain(step.sliders.keys().map(String::as_str))
        })
        .collect();

    // Then only the documented exception lacks evidence.
    let unverified: Vec<&str> = decodable.difference(&captured).copied().collect();
    assert_eq!(unverified, UNVERIFIED);
    Ok(())
}
