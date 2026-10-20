//! One output owner services motors, feedback and LEDs while snapshots run apart.

mod input;
mod paced;

use anyhow::{Context as _, ensure};
use encdr::device::led_builder::LedBuilder;
use encdr::device::parser::PacketParser;
use hidapi::HidDevice;
use s4mk3_hid_midi_bridge::tension::{Config, Engine};
use std::time::{Duration, Instant};
use tracing::{debug, info};

use crate::{Args, ProbeCounts, full, input_payload, record_event};
use input::Reader;
use paced::Action;

pub fn run(
    args: &Args,
    hid: &HidDevice,
    reader_hid: HidDevice,
    parser: &mut PacketParser,
    leds: &mut LedBuilder,
    config: Config,
) -> anyhow::Result<()> {
    let led_config = full::leds::LedConfig::load(args.led_config.as_deref())?;
    let stop = crate::stop_signal()?;
    let mut bridge = full::FullBridge::new()?;
    let mut output = None;
    let mut counts = ProbeCounts::default();
    let mut events = Vec::with_capacity(128);
    let started = Instant::now();
    let mut engine = Engine::new(config);
    let mut service = paced::Output::new(|action| {
        match action {
            Action::Initialize => {
                output = Some(full::output::LedOutput::new(hid)?);
            }
            Action::Motor(report) => {
                let written = hid.write(&report).context("write S4 tension report")?;
                ensure!(
                    written == report.len(),
                    "short S4 tension write: {written}/11"
                );
                debug!(written, bytes = ?report, "TENSION_WRITE");
                if report == s4mk3_hid_midi_bridge::tension::neutral_report() {
                    debug!("TENSION_NEUTRAL");
                }
            }
            Action::Frame(frame) => {
                for index in 0..3 {
                    let report = frame.report(index);
                    if report.is_empty() {
                        continue;
                    }
                    if args.raw {
                        info!(length = report.len(), bytes = %format_args!("{report:02x?}"), "HID_RAW");
                    }
                    let Some(payload) = input_payload(report)? else {
                        continue;
                    };
                    events.clear();
                    parser.parse_from("control", payload, &mut events);
                    for event in &events {
                        record_event(event, leds, &mut counts, false)?;
                    }
                    let buttons = if report.first() == Some(&1) {
                        Some(
                            payload
                                .try_into()
                                .context("full button payload must have 22 bytes")?,
                        )
                    } else {
                        None
                    };
                    let sent = bridge.send_report(&events, buttons)?;
                    counts.midi_messages =
                        counts.midi_messages.saturating_add(u64::try_from(sent)?);
                }
            }
            Action::Feedback(now_us) => {
                let now = Duration::from_micros(now_us);
                let (received, sent) = bridge.drain_feedback(now)?;
                counts.feedback_messages = counts
                    .feedback_messages
                    .saturating_add(u64::try_from(received)?);
                counts.midi_messages = counts.midi_messages.saturating_add(u64::try_from(sent)?);
                let writes = output
                    .as_mut()
                    .context("LED output not initialized")?
                    .tick(
                        &led_config,
                        bridge.feedback_state(),
                        &bridge.local_led_state(),
                        now,
                    )?;
                counts.led_writes = counts.led_writes.saturating_add(u64::try_from(writes)?);
            }
        }
        Ok(())
    })?;
    service.initialize()?;
    let mut reader = Reader::start(reader_hid, started)?;
    info!(
        tension = config.value(),
        "TENSION_ACTIVE: shared physical wheels; mixer faders unchanged"
    );
    let duration_us = (!args.until_stopped).then(|| args.seconds.get().saturating_mul(1_000_000));
    let result = paced::run(
        &mut engine,
        &mut reader,
        &stop,
        duration_us,
        || Ok(u64::try_from(started.elapsed().as_micros())?),
        &mut service,
    );
    // Clear force before waiting for input teardown or releasing MIDI/LED ownership.
    let neutral = service.finish();
    let cleanup = reader.stop();
    drop(service);
    with_cleanup(
        with_cleanup(result, neutral, "motor neutral"),
        cleanup,
        "reader shutdown",
    )?;
    crate::report_probe_counts(args, &counts)
}

fn with_cleanup(
    result: anyhow::Result<()>,
    cleanup: anyhow::Result<()>,
    operation: &str,
) -> anyhow::Result<()> {
    match (result, cleanup) {
        (Ok(()), cleanup) => cleanup,
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("{operation} also failed: {cleanup:#}")))
        }
    }
}
