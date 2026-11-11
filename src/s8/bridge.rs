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
use super::led_output::{LedOutput, ReportSink};
use super::led_state::{Local, present};
use super::leds::{Side, SideLeds};
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
        output: Some(LedOutput::new(hid)),
    });
    let mut read_timeout = announce_open(leds.is_some());
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
        if let Some(active) = leds.as_mut() {
            active.update(&translator, started.elapsed(), &mut counts);
            if !active.active() {
                read_timeout = READ_TIMEOUT_MS;
                leds = None;
            }
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
    if let Some(mut leds) = leds
        && let Some(output) = leds.output.as_mut()
        && let Err(error) = output.finish()
    {
        warn!(%error, "S8_LEDS_CLEAR_FAILED");
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

/// The Djay feedback path and LED writer, present only with `--s8-leds`. The
/// writer is dropped for the rest of the run after its first failure.
struct Leds<'a> {
    input: FeedbackInput,
    state: FeedbackState,
    output: Option<LedOutput<'a, HidDevice>>,
}

impl Leds<'_> {
    /// False once the LED path has been disabled.
    const fn active(&self) -> bool {
        self.output.is_some()
    }

    /// Adds the feedback messages applied and the reports written to `counts`.
    fn update(&mut self, translator: &Translator, now: Duration, counts: &mut ProbeCounts) {
        let Self {
            input,
            state,
            output,
        } = self;
        let received = match input.drain_messages(|message| {
            state.apply(message, now);
        }) {
            Ok(received) => received,
            Err(error) => {
                disable(output, &error);
                return;
            }
        };
        let leds =
            Side::ALL.map(|side| present(side, state, &Local::from_translator(translator, side)));
        let written = tick_or_disable(output, &leds, now);
        counts.feedback_messages = counts.feedback_messages.saturating_add(count(received));
        counts.led_writes = counts.led_writes.saturating_add(count(written));
    }
}

fn count(messages: usize) -> u64 {
    u64::try_from(messages).unwrap_or(u64::MAX)
}

/// Writes the changed reports and returns how many. On an error the LEDs are
/// cleared as far as the device allows and the writer is dropped; MIDI
/// translation goes on.
fn tick_or_disable<S: ReportSink>(
    output: &mut Option<LedOutput<'_, S>>,
    leds: &[SideLeds; 2],
    now: Duration,
) -> usize {
    let Some(writer) = output.as_mut() else {
        return 0;
    };
    match writer.tick(leds, now) {
        Ok(written) => written,
        Err(error) => {
            disable(output, &error);
            0
        }
    }
}

fn disable<S: ReportSink>(output: &mut Option<LedOutput<'_, S>>, error: &anyhow::Error) {
    warn!("S8_LEDS_DISABLED: {error:#}; the LEDs stay off for the rest of the run");
    if let Some(mut writer) = output.take()
        && let Err(clear) = writer.finish()
    {
        warn!(error = %clear, "S8_LEDS_CLEAR_FAILED");
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use anyhow::Result;

    use super::*;
    use crate::s8::leds::Level;

    #[derive(Default)]
    struct Sink {
        writes: RefCell<Vec<u8>>,
        failing: Cell<bool>,
    }

    impl ReportSink for Sink {
        fn write_report(&self, report: &[u8]) -> Result<usize> {
            self.writes.borrow_mut().push(report[0]);
            if self.failing.get() {
                anyhow::bail!("device gone");
            }
            Ok(report.len())
        }
    }

    fn lit(level: Level) -> [SideLeds; 2] {
        let mut leds = [SideLeds::default(); 2];
        leds[0].play = level;
        leds
    }

    #[test]
    fn a_write_error_disables_the_led_path_and_no_more_writes_follow() {
        let sink = Sink::default();
        let mut output = Some(LedOutput::new(&sink));
        let written = tick_or_disable(&mut output, &lit(Level::On), Duration::ZERO);
        assert!(output.is_some());
        let before = sink.writes.borrow().len();
        assert_eq!(written, before);

        sink.failing.set(true);
        let written = tick_or_disable(&mut output, &lit(Level::Off), Duration::from_millis(50));
        assert_eq!(written, 0);
        assert!(output.is_none());
        let after = sink.writes.borrow().len();
        assert!(
            after > before,
            "the failed write and the clear are attempted"
        );

        sink.failing.set(false);
        let written = tick_or_disable(&mut output, &lit(Level::On), Duration::from_millis(100));
        assert_eq!(written, 0);
        tick_or_disable(&mut output, &lit(Level::Off), Duration::from_millis(200));
        assert_eq!(sink.writes.borrow().len(), after);
    }
}
