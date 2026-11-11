//! Kontrol S8 HID input to the `Kontrol S8 MIDI Full` virtual source. The
//! bridge reads the control interface; only `--s8-leds` writes deck LED reports.

use std::time::{Duration, Instant};

use anyhow::{Context as _, bail, ensure};
use encdr::device::parser::PacketParser;
use hidapi::HidDevice;
use midir::MidiOutput;
use midir::os::unix::VirtualOutput as _;
use s4mk3_hid_midi_bridge::device::S8;
use s4mk3_hid_midi_bridge::device::s8::ScreenEncoders;
use tracing::{info, warn};

use super::app_events;
use super::feedback::FeedbackState;
use super::led_output::LedOutput;
use super::led_state::{Local, present};
use super::leds::Side;
use super::output;
use super::translator::Translator;
use crate::full::feedback::FeedbackInput;
use crate::{Args, MidiProfile, ProbeCounts, record_event, report_probe_counts, stop_signal};

const READ_TIMEOUT_MS: i32 = 100;
/// With LEDs on, the loop wakes this often to apply Djay feedback while idle.
const LED_READ_TIMEOUT_MS: i32 = 5;

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

fn announce_open(leds: bool) -> i32 {
    if leds {
        info!(
            "S8_LEDS_ENABLED: reports 0x{:02x} and 0x{:02x} will be written to the controller",
            Side::Left.report_id(),
            Side::Right.report_id()
        );
        LED_READ_TIMEOUT_MS
    } else {
        info!(
            "{}_HID_OPEN: native HID access; nothing is written to the controller",
            S8.tag
        );
        READ_TIMEOUT_MS
    }
}

pub fn run(args: &Args, hid: &HidDevice, parser: &mut PacketParser) -> anyhow::Result<()> {
    let (stop_tx, stop_rx) = stop_signal()?;
    let _screens = args
        .screens
        .s8_stdin
        .then(|| super::screens::start(stop_tx.clone()))
        .transpose()?;
    // Djay can return initial state as soon as the source appears.
    let feedback_input = args
        .s8_leds
        .then(|| FeedbackInput::new(S8.full_endpoint))
        .transpose()?;
    let mut midi = MidiOutput::new("Kontrol S8 full bridge")?
        .create_virtual(S8.full_endpoint)
        .map_err(|error| anyhow::anyhow!("create S8 MIDI source: {error}"))?;
    info!(port = S8.full_endpoint, "MIDI_FULL_READY");
    let mut leds = feedback_input.map(|input| Leds {
        input,
        state: FeedbackState::default(),
        output: LedOutput::new(hid),
    });
    let read_timeout = announce_open(leds.is_some());
    let mut translator = Translator::new();
    let mut counts = ProbeCounts::default();
    let mut screen_encoders = ScreenEncoders::default();
    let mut seen = Vec::with_capacity(S8.input_reports.len());
    let mut buffer = [0_u8; 128];
    let mut events = Vec::with_capacity(128);
    let mut messages = Vec::with_capacity(32);
    let started = Instant::now();
    let mut reader_gone = false;
    while args.until_stopped || started.elapsed() < Duration::from_secs(args.seconds.get()) {
        match stop_rx.try_recv() {
            Ok(()) => break,
            Err(crossbeam_channel::TryRecvError::Empty) => {}
            Err(crossbeam_channel::TryRecvError::Disconnected) => bail!("stop channel closed"),
        }
        if let Some(leds) = leds.as_mut() {
            leds.update(&translator, started.elapsed())?;
        }
        let length = hid
            .read_timeout(&mut buffer, read_timeout)
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
                    reader_gone |= emit_line(&line);
                }
            }
        }
        for event in &events {
            record_event(event, None, &mut counts, false)?;
            translator.handle(event, resting, &mut messages);
            if args.app_events
                && let Some(line) = app_events::display_button(event, resting)
            {
                reader_gone |= emit_line(&line);
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
        if reader_gone {
            break;
        }
    }
    if let Some(mut leds) = leds {
        leds.output.finish()?;
    }
    report_probe_counts(args, &S8, &counts)
}

/// True when stdout has no reader left.
fn emit_line(line: &str) -> bool {
    match output::emit(line) {
        Ok(()) => false,
        Err(error) => {
            warn!(%error, "S8 control reader is gone, stopping");
            true
        }
    }
}

/// The Djay feedback path and LED writer, present only with `--s8-leds`.
struct Leds<'a> {
    input: FeedbackInput,
    state: FeedbackState,
    output: LedOutput<'a, HidDevice>,
}

impl Leds<'_> {
    fn update(&mut self, translator: &Translator, now: Duration) -> anyhow::Result<()> {
        let Self {
            input,
            state,
            output,
        } = self;
        input.drain_messages(|message| {
            state.apply(message, now);
        })?;
        let leds =
            Side::ALL.map(|side| present(side, state, &Local::from_translator(translator, side)));
        output.tick(&leds, now)?;
        Ok(())
    }
}
