//! Kontrol S8 HID input to the `Kontrol S8 MIDI Full` virtual source. The
//! bridge reads the control interface and writes nothing to the controller.

use std::time::{Duration, Instant};

use anyhow::{Context as _, bail, ensure};
use encdr::device::parser::PacketParser;
use hidapi::HidDevice;
use midir::MidiOutput;
use midir::os::unix::VirtualOutput as _;
use s4mk3_hid_midi_bridge::device::S8;
use s4mk3_hid_midi_bridge::device::s8::ScreenEncoders;
use tracing::info;

use super::app_events;
use super::translator::Translator;
use crate::{Args, MidiProfile, ProbeCounts, record_event, report_probe_counts, stop_signal};

const READ_TIMEOUT_MS: i32 = 100;

/// Refuse the minimal profile and the options that drive S4 MK3 outputs.
pub fn check_args(args: &Args) -> anyhow::Result<()> {
    ensure!(
        matches!(args.midi, Some(MidiProfile::Full)),
        "the Kontrol S8 has the full MIDI profile only; use --midi full"
    );
    ensure!(
        args.led_config.is_none()
            && args.tension_config.is_none()
            && args.jog_config.is_none()
            && !args.screens.enabled
            && !args.screens.control_stdin,
        "--led-config, --tension-config, --jog-config, --screens and \
         --screen-control-stdin apply to the S4 MK3 only"
    );
    Ok(())
}

pub fn run(args: &Args, hid: &HidDevice, parser: &mut PacketParser) -> anyhow::Result<()> {
    let (_stop_tx, stop_rx) = stop_signal()?;
    let mut midi = MidiOutput::new("Kontrol S8 full bridge")?
        .create_virtual(S8.full_endpoint)
        .map_err(|error| anyhow::anyhow!("create S8 MIDI source: {error}"))?;
    info!(port = S8.full_endpoint, "MIDI_FULL_READY");
    info!(
        "{}_HID_OPEN: native HID access; nothing is written to the controller",
        S8.tag
    );
    let mut translator = Translator::new();
    let mut counts = ProbeCounts::default();
    let mut screen_encoders = ScreenEncoders::default();
    let mut seen = Vec::with_capacity(S8.input_reports.len());
    let mut buffer = [0_u8; 128];
    let mut events = Vec::with_capacity(128);
    let mut messages = Vec::with_capacity(32);
    let started = Instant::now();
    while args.until_stopped || started.elapsed() < Duration::from_secs(args.seconds.get()) {
        match stop_rx.try_recv() {
            Ok(()) => break,
            Err(crossbeam_channel::TryRecvError::Empty) => {}
            Err(crossbeam_channel::TryRecvError::Disconnected) => bail!("stop channel closed"),
        }
        let length = hid
            .read_timeout(&mut buffer, READ_TIMEOUT_MS)
            .context("read S8 HID input")?;
        let Some((id, report)) = buffer
            .get(..length)
            .and_then(|report| Some((*report.first()?, report)))
        else {
            continue;
        };
        if args.raw {
            info!(length, bytes = %format_args!("{report:02x?}"), "HID_RAW");
        }
        let Some(payload) = S8.input_payload(report)? else {
            continue;
        };
        // The S8 reports only when a control moves, so the first report of
        // each ID is the only source of the positions the controls rest in.
        let resting = !seen.contains(&id);
        if resting {
            seen.push(id);
        }
        events.clear();
        messages.clear();
        parser.parse_from("control", payload, &mut events);
        if args.app_events {
            for turn in screen_encoders.update(report) {
                if let Some(line) = app_events::screen_encoder(&turn) {
                    println!("{line}");
                }
            }
        }
        for event in &events {
            record_event(event, None, &mut counts, false)?;
            translator.handle(event, resting, &mut messages);
            if args.app_events
                && let Some(line) = app_events::display_button(event, resting)
            {
                println!("{line}");
            }
        }
        translator.end_report(&mut messages);
        for message in &messages {
            midi.send(message)
                .map_err(|error| anyhow::anyhow!("send S8 MIDI: {error}"))?;
        }
        counts.midi_messages = counts
            .midi_messages
            .saturating_add(u64::try_from(messages.len())?);
    }
    report_probe_counts(args, &S8, &counts)
}
