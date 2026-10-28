//! A bounded S4 MK3 HID probe with optional minimal MIDI output.

mod bridge_runtime;
mod full;
mod input;
mod midi;
#[path = "jog_diagnostic/ownership.rs"]
mod ownership;
mod screens_control;

use std::num::NonZeroU64;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail, ensure};
use clap::{Parser, ValueEnum};
use encdr::device::led_builder::LedBuilder;
use encdr::device::loader::DescriptorRegistry;
use encdr::device::parser::PacketParser;
use encdr::usb::hotplug;
use encdr::{Event, LedValue};
use hidapi::{HidApi, HidDevice};
use s4mk3_hid_midi_bridge::device::{self, Device, DeviceProfile};
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum MidiProfile {
    Minimal,
    Full,
}

#[derive(Debug, Parser)]
#[command(
    version,
    about,
    group(clap::ArgGroup::new("jog_context").args(["generate_mapping", "midi"]))
)]
struct Args {
    /// List attached supported controllers without opening their USB interfaces.
    #[arg(long)]
    list: bool,

    /// Controller to use; detected when omitted. Mapping export defaults to s4mk3.
    #[arg(long, value_enum, conflicts_with = "generate_screen_previews")]
    device: Option<Device>,

    /// Log every complete input HID report in hex, including its report ID.
    #[arg(long)]
    raw: bool,

    /// Publish virtual MIDI; bare --midi retains the verified minimal profile.
    #[arg(long, value_enum, num_args = 0..=1, default_missing_value = "minimal", conflicts_with_all = ["list", "generate_mapping"])]
    midi: Option<MidiProfile>,

    /// JSON LED preferences for the full profile; restart after editing.
    #[arg(long, value_name = "PATH", requires = "midi", conflicts_with_all = ["list", "generate_mapping"])]
    led_config: Option<PathBuf>,

    /// Saved hardware tension for the full profile; restart after editing.
    #[arg(long, value_name = "PATH", requires = "midi", conflicts_with_all = ["list", "generate_mapping"])]
    tension_config: Option<PathBuf>,

    #[command(flatten)]
    screens: ScreenArgs,

    /// Export the full native Djay mapping without opening HID or MIDI; a
    /// directory receives the controller's standard mapping file name.
    #[arg(long, value_name = "PATH", conflicts_with_all = ["midi", "list", "raw"])]
    generate_mapping: Option<PathBuf>,

    /// JSON jog preferences for mapping export or full-profile backspin behavior.
    #[arg(
        long,
        value_name = "PATH",
        requires = "jog_context",
        conflicts_with = "list"
    )]
    jog_config: Option<PathBuf>,

    /// Maximum probe duration; stop earlier with Ctrl-C.
    #[arg(long, default_value = "30")]
    seconds: NonZeroU64,

    /// Run until Ctrl-C instead of stopping after a fixed duration.
    #[arg(long, conflicts_with = "seconds")]
    until_stopped: bool,
}

#[derive(Debug, clap::Args)]
struct ScreenArgs {
    /// Experimental Djay feedback and reactive STEMS on both S4 screens (full profile).
    #[arg(long = "screens", requires = "midi", conflicts_with_all = ["list", "generate_mapping"])]
    enabled: bool,

    /// Accept live screen ON/OFF/RETRY commands on stdin; EOF stops the bridge.
    #[arg(long = "screen-control-stdin", requires = "midi", conflicts_with_all = ["list", "generate_mapping", "generate_screen_previews"])]
    control_stdin: bool,

    /// Export screen routing, feedback, and held-pad rasters without USB or MIDI.
    #[arg(long, value_name = "DIR", conflicts_with_all = ["midi", "list", "raw", "generate_mapping", "jog_config"])]
    generate_screen_previews: Option<PathBuf>,
}

#[derive(Debug, Default)]
struct ProbeCounts {
    input_events: u64,
    led_writes: u64,
    midi_messages: u64,
    feedback_messages: u64,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    ensure!(
        args.jog_config.is_none()
            || args.generate_mapping.is_some()
            || matches!(args.midi, Some(MidiProfile::Full)),
        "--jog-config requires mapping export or --midi full"
    );
    ensure!(
        args.led_config.is_none() || matches!(args.midi, Some(MidiProfile::Full)),
        "--led-config requires --midi full"
    );
    ensure!(
        args.tension_config.is_none() || matches!(args.midi, Some(MidiProfile::Full)),
        "--tension-config requires --midi full"
    );
    ensure!(
        !args.screens.enabled || matches!(args.midi, Some(MidiProfile::Full)),
        "--screens requires --midi full"
    );
    ensure!(
        !args.screens.control_stdin || matches!(args.midi, Some(MidiProfile::Full)),
        "--screen-control-stdin requires --midi full"
    );
    let tension = args
        .tension_config
        .as_deref()
        .map(|path| s4mk3_hid_midi_bridge::tension::Config::load(Some(path)))
        .transpose()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?)
        .with_target(false)
        .with_ansi(false)
        .try_init()
        .map_err(|error| anyhow::anyhow!("initialize logging: {error}"))?;

    if let Some(path) = &args.generate_mapping {
        let profile = args.device.unwrap_or(Device::S4Mk3).profile();
        require_full_catalog(profile)?;
        let path = &mapping_path(path, profile);
        full::mapping::write(path, args.jog_config.as_deref())?;
        for control in full::catalog::LOCAL_CONTROLS {
            info!(
                input = control.name,
                reason = control.reason,
                "LOCAL_CONTROL"
            );
        }
        for control in full::catalog::UNSUPPORTED_CONTROLS {
            info!(
                input = control.name,
                reason = control.reason,
                "UNASSIGNED_CONTROL"
            );
        }
        info!(path = %path.display(), "DJAY_MAPPING_EXPORTED");
        return Ok(());
    }
    if let Some(path) = &args.screens.generate_screen_previews {
        full::screens::write_previews(path)?;
        info!(path = %path.display(), "SCREEN_PREVIEWS_EXPORTED: no USB or MIDI opened");
        return Ok(());
    }
    let jog = matches!(args.midi, Some(MidiProfile::Full))
        .then(|| full::jog::Config::load(args.jog_config.as_deref()))
        .transpose()?;
    if let Some(jog) = jog {
        jog.validate()?;
    }
    let tension = tension.or_else(|| {
        jog.filter(full::jog::Config::motor_assist_enabled)
            .map(|_| s4mk3_hid_midi_bridge::tension::Config::default())
    });
    open_and_run(&args, tension, jog)
}

fn open_and_run(
    args: &Args,
    tension: Option<s4mk3_hid_midi_bridge::tension::Config>,
    jog: Option<full::jog::Config>,
) -> anyhow::Result<()> {
    let mut registry = DescriptorRegistry::new();
    registry.load_builtins().context("load Encdr descriptors")?;
    let scanned = hotplug::scan_devices(&registry);
    if args.list {
        let listed = args.device.map_or_else(
            || device::PROFILES.to_vec(),
            |device| vec![device.profile()],
        );
        for profile in listed {
            let count = found(profile, &scanned).len();
            info!(
                count,
                "{} enumeration complete; no interfaces opened", profile.tag
            );
        }
        return Ok(());
    }
    let (profile, device) = select_device(args.device, &scanned)?;
    let Some(corrections) = profile.descriptor else {
        return identify(profile);
    };
    let descriptor = if matches!(args.midi, Some(MidiProfile::Full)) {
        require_full_catalog(profile)?;
        (corrections.full)(&device.descriptor)?
    } else {
        (corrections.control)(&device.descriptor)
    };
    let names = registry.intern_descriptor_names(&descriptor);
    let mut parser = PacketParser::new(device.device_id, &descriptor, names);
    let api = HidApi::new().context("initialize macOS-native HID access")?;
    let mut collections = api
        .device_list()
        .filter(|collection| profile.matches(collection.vendor_id(), collection.product_id()));
    let collection = collections
        .next()
        .with_context(|| format!("no {} HID collection found", profile.tag))?;
    ensure!(
        collections.next().is_none(),
        "multiple {} HID collections found; collection selection requires investigation",
        profile.tag
    );
    ownership::check()?;
    let hid = api
        .open_path(collection.path())
        .with_context(|| format!("open {} HID collection", profile.tag))?;
    let tension = tension
        .map(|config| {
            api.open_path(collection.path())
                .context("open S4 tension snapshot reader")
                .map(|reader| (reader, config))
        })
        .transpose()?;
    let interface = descriptor
        .interface_by_id("control")
        .context("control interface missing")?;
    let buttons = descriptor
        .leds
        .first()
        .context("button LED layout missing")?;
    let mut leds = LedBuilder::new(buttons, interface);
    info!(
        "{}_HID_OPEN: native HID access; audio interfaces are not claimed",
        profile.tag
    );
    if let Some((reader, config)) = tension {
        bridge_runtime::run(
            args,
            &hid,
            reader,
            &mut parser,
            &mut leds,
            config,
            jog.context("motor output requires the full MIDI profile")?,
        )
    } else {
        run_probe(args, profile, &hid, &mut parser, &mut leds, jog)
    }
}

/// Log and return the scanned devices that are this controller.
fn found<'a>(
    profile: &DeviceProfile,
    scanned: &'a [hotplug::DetectedDevice],
) -> Vec<&'a hotplug::DetectedDevice> {
    let devices: Vec<_> = scanned
        .iter()
        .filter(|found| {
            profile.matches(found.descriptor.vendor_id.0, found.descriptor.product_id.0)
        })
        .collect();
    for found in &devices {
        info!(
            device = %found.device_id,
            "{}_USB_FOUND: {:04x}:{:04x}", profile.tag, profile.vendor_id, profile.product_id
        );
    }
    devices
}

fn select_device(
    forced: Option<Device>,
    scanned: &[hotplug::DetectedDevice],
) -> anyhow::Result<(&'static DeviceProfile, &hotplug::DetectedDevice)> {
    let profile = device::select(
        forced,
        scanned
            .iter()
            .map(|found| (found.descriptor.vendor_id.0, found.descriptor.product_id.0)),
    )?;
    let mut devices = found(profile, scanned).into_iter();
    let device = devices.next().with_context(|| {
        format!(
            "no {} found; connect USB and the {} power adapter, then run the probe again",
            profile.name, profile.tag
        )
    })?;
    ensure!(
        devices.next().is_none(),
        "multiple {} devices found; connect just one for this probe",
        profile.name
    );
    Ok((profile, device))
}

/// Report a controller the bridge cannot drive yet from enumeration alone.
fn identify(profile: &DeviceProfile) -> anyhow::Result<()> {
    let api = HidApi::new().context("initialize macOS-native HID access")?;
    let mut collections = 0_usize;
    for collection in api
        .device_list()
        .filter(|collection| profile.matches(collection.vendor_id(), collection.product_id()))
    {
        collections += 1;
        info!(
            product = ?collection.product_string(),
            manufacturer = ?collection.manufacturer_string(),
            release = %format_args!("{:04x}", collection.release_number()),
            interface = collection.interface_number(),
            usage_page = %format_args!("{:04x}", collection.usage_page()),
            usage = %format_args!("{:04x}", collection.usage()),
            "{}_HID_IDENTIFIED: enumerated only; not opened", profile.tag
        );
    }
    bail!(
        "{} identified ({collections} HID collections); the bridge cannot translate its input yet, so nothing was opened or sent",
        profile.name
    )
}

fn mapping_path(path: &std::path::Path, profile: &DeviceProfile) -> PathBuf {
    if path.is_dir() {
        path.join(profile.mapping_file_name)
    } else {
        path.to_path_buf()
    }
}

fn require_full_catalog(profile: &DeviceProfile) -> anyhow::Result<()> {
    ensure!(
        profile.device == full::PROFILE.device,
        "no Djay mapping catalog for {} yet",
        profile.name
    );
    Ok(())
}

fn run_probe(
    args: &Args,
    profile: &DeviceProfile,
    hid: &HidDevice,
    parser: &mut PacketParser,
    leds: &mut LedBuilder,
    jog: Option<full::jog::Config>,
) -> anyhow::Result<()> {
    let led_config = matches!(args.midi, Some(MidiProfile::Full))
        .then(|| full::leds::LedConfig::load(args.led_config.as_deref()))
        .transpose()?;
    let (stop_tx, stop_rx) = stop_signal()?;
    let mut midi = matches!(args.midi, Some(MidiProfile::Minimal))
        .then(midi::MidiBridge::new)
        .transpose()?;
    let mut full = jog
        .map(|jog| screens_control::bridge(args, stop_tx, jog))
        .transpose()?;
    let mut full_leds = full
        .as_ref()
        .map(|_| full::output::LedOutput::new(hid))
        .transpose()?;
    if let Some(bridge) = &mut full {
        initialize_full_analog(hid, parser, bridge)?;
    }
    info!(
        seconds = args.seconds.get(),
        until_stopped = args.until_stopped,
        "Press/release left Play; move a fader and jog."
    );
    let started = Instant::now();
    let mut counts = ProbeCounts::default();
    let mut buffers = [[0_u8; 128]; 3];
    let mut events = Vec::with_capacity(128);
    while args.until_stopped || started.elapsed() < Duration::from_secs(args.seconds.get()) {
        match stop_rx.try_recv() {
            Ok(()) => break,
            Err(crossbeam_channel::TryRecvError::Empty) => {}
            Err(crossbeam_channel::TryRecvError::Disconnected) => bail!("stop channel closed"),
        }
        let lengths = input::read_batch(hid, full.is_some(), &mut buffers)?;
        for (buffer, length) in buffers.iter().zip(lengths) {
            if length == 0 {
                continue;
            }
            let report = buffer
                .get(..length)
                .context("HID report exceeds input buffer")?;
            debug!(length, report_id = ?report.first(), "HID_REPORT");
            if args.raw {
                info!(length, bytes = %format_args!("{report:02x?}"), "HID_RAW");
            }
            let Some(payload) = profile.input_payload(report)? else {
                continue;
            };
            events.clear();
            parser.parse_from("control", payload, &mut events);
            for event in &events {
                record_event(event, leds, &mut counts, full.is_none())?;
                if let Some(bridge) = &mut midi
                    && bridge.send(event)?
                {
                    counts.midi_messages = counts.midi_messages.saturating_add(1);
                }
            }
            if let Some(bridge) = &mut full {
                send_full_report(bridge, &events, report, &mut counts, started.elapsed())?;
            }
            if full.is_none()
                && let Some(report) = leds.flush()
            {
                let written = hid.write(&report).context("write S4 button LED report")?;
                ensure!(
                    written == report.len(),
                    "short S4 LED write: {written} bytes"
                );
                counts.led_writes = counts.led_writes.saturating_add(1);
                info!(
                    written,
                    "LED_WRITE_OK: native HID accepted report; confirm light physically"
                );
            }
        }
        if let (Some(bridge), Some(output), Some(config)) = (&mut full, &mut full_leds, &led_config)
        {
            let now = started.elapsed();
            let (received, sent) = bridge.drain_feedback(now)?;
            counts.feedback_messages = counts
                .feedback_messages
                .saturating_add(u64::try_from(received)?);
            counts.midi_messages = counts.midi_messages.saturating_add(u64::try_from(sent)?);
            let writes = output.tick(
                config,
                bridge.feedback_state(),
                &bridge.local_led_state(),
                now,
            )?;
            counts.led_writes = counts.led_writes.saturating_add(u64::try_from(writes)?);
            bridge.publish_screens();
        }
    }
    report_probe_counts(args, &counts)
}

fn stop_signal() -> anyhow::Result<(
    crossbeam_channel::Sender<()>,
    crossbeam_channel::Receiver<()>,
)> {
    let (sender, receiver) = crossbeam_channel::bounded(1);
    let interrupt = sender.clone();
    ctrlc::set_handler(move || {
        if let Err(error) = interrupt.try_send(()) {
            warn!(%error, "could not enqueue stop signal");
        }
    })
    .context("install Ctrl-C handler")?;
    Ok((sender, receiver))
}

fn initialize_full_analog(
    hid: &HidDevice,
    parser: &mut PacketParser,
    bridge: &mut full::FullBridge,
) -> anyhow::Result<()> {
    let report = input::read_analog(hid)?;
    let mut events = Vec::with_capacity(48);
    // Seed the real parser once, including zero-valued sliders, without sending
    // startup FX, gain, filter, crossfader-position or tempo commands.
    parser.parse_from("control", &report[1..], &mut events);
    bridge.seed_analog(&events)
}

fn send_full_report(
    bridge: &mut full::FullBridge,
    events: &[Event],
    report: &[u8],
    counts: &mut ProbeCounts,
    now: Duration,
) -> anyhow::Result<()> {
    let (received, sent) = bridge.send_report(events, report, now)?;
    counts.feedback_messages = counts
        .feedback_messages
        .saturating_add(u64::try_from(received)?);
    counts.midi_messages = counts.midi_messages.saturating_add(u64::try_from(sent)?);
    Ok(())
}

fn report_probe_counts(args: &Args, counts: &ProbeCounts) -> anyhow::Result<()> {
    info!(
        input_events = counts.input_events,
        led_writes = counts.led_writes,
        midi_messages = counts.midi_messages,
        feedback_messages = counts.feedback_messages,
        "Probe ended; LED visibility and audio coexistence need physical confirmation"
    );
    // Continuous service use may be idle; bounded probes still require activity.
    if args.until_stopped {
        return Ok(());
    }
    ensure!(
        counts.input_events > 0,
        "no decoded S4 input received during the probe"
    );
    ensure!(
        args.midi.is_none()
            || counts.midi_messages > 0
            || (matches!(args.midi, Some(MidiProfile::Full)) && counts.feedback_messages > 0),
        "no mapped MIDI controls received; press Play/Cue or move a channel fader/crossfader"
    );
    Ok(())
}

fn record_event(
    event: &Event,
    leds: &mut LedBuilder,
    counts: &mut ProbeCounts,
    mirror_play: bool,
) -> anyhow::Result<()> {
    match event {
        Event::DeviceConnected { id, .. } => {
            info!(device = %id, "device event; waiting for actual controls");
            return Ok(());
        }
        Event::DeviceDisconnected { id } => {
            bail!("S4 disconnected: {id}");
        }
        Event::Button { name, pressed, .. } => {
            info!(control = name, pressed, "BUTTON");
            if mirror_play && *name == "left_play" {
                ensure!(
                    leds.set(
                        name,
                        if *pressed {
                            LedValue::Single(127)
                        } else {
                            LedValue::Off
                        }
                    ),
                    "left_play LED missing from Encdr layout"
                );
            }
        }
        Event::Slider { name, value, .. } => {
            info!(control = name, value, "SLIDER");
        }
        Event::Encoder { name, delta, .. } => {
            info!(control = name, delta, "ENCODER");
        }
        Event::EncoderFine { name, delta, .. } => {
            debug!(control = name, delta, "ENCODER_FINE");
        }
        Event::Touch { name, touched, .. } => {
            info!(control = name, touched, "TOUCH");
        }
        Event::Grid {
            name,
            index,
            pressure,
            ..
        } => {
            info!(control = name, index, pressure, "GRID");
        }
    }
    counts.input_events = counts.input_events.saturating_add(1);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_profiles_preserve_the_bare_flag_and_export_is_exclusive() -> anyhow::Result<()> {
        // Given the old bare MIDI command and the new explicit full profile.
        let minimal = Args::try_parse_from(["probe", "--midi"])?;
        let full = Args::try_parse_from(["probe", "--midi", "full"])?;
        // Then both select their intended profile.
        assert!(matches!(minimal.midi, Some(MidiProfile::Minimal)));
        assert!(matches!(full.midi, Some(MidiProfile::Full)));
        // Export cannot accidentally also open the controller.
        assert!(
            Args::try_parse_from([
                "probe",
                "--generate-mapping",
                "mapping.djayMidiMapping",
                "--midi"
            ])
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn device_choice_parses_and_only_the_s4_has_a_djay_catalog() -> anyhow::Result<()> {
        // Given each forced controller, for runtime and mapping export.
        for (name, device, has_catalog) in
            [("s4mk3", Device::S4Mk3, true), ("s8", Device::S8, false)]
        {
            let args = Args::try_parse_from(["probe", "--device", name, "--list"])?;
            assert_eq!(args.device, Some(device));
            let export = Args::try_parse_from([
                "probe",
                "--generate-mapping",
                "mapping.djayMidiMapping",
                "--device",
                name,
            ])?;
            // Then only a controller with a catalog may export or run the full profile.
            let device = export.device.context("device missing")?;
            assert_eq!(require_full_catalog(device.profile()).is_ok(), has_catalog);
        }
        // Screen previews are S4 artwork, independent of any controller choice.
        assert!(
            Args::try_parse_from([
                "probe",
                "--generate-screen-previews",
                "previews",
                "--device",
                "s8"
            ])
            .is_err()
        );
        assert!(Args::try_parse_from(["probe", "--device", "s3"]).is_err());
        Ok(())
    }

    #[test]
    fn raw_logging_is_enabled_only_when_requested() -> anyhow::Result<()> {
        for (arguments, expected) in [(&["probe"][..], false), (&["probe", "--raw"][..], true)] {
            // Given command-line arguments with or without the diagnostic flag.
            // When the CLI parses them.
            let args = Args::try_parse_from(arguments)?;
            // Then raw logging is enabled only by the flag.
            assert_eq!(args.raw, expected);
        }
        Ok(())
    }

    #[test]
    fn continuous_run_is_explicit_and_conflicts_with_a_duration() -> anyhow::Result<()> {
        assert!(!Args::try_parse_from(["probe"])?.until_stopped);
        assert!(
            Args::try_parse_from(["probe", "--midi", "full", "--until-stopped"])?.until_stopped
        );
        assert!(Args::try_parse_from(["probe", "--until-stopped", "--seconds", "60"]).is_err());
        Ok(())
    }

    #[test]
    fn managed_screen_stdin_accepts_full_profile_with_or_without_initial_screens()
    -> anyhow::Result<()> {
        for enabled in [false, true] {
            // Given an explicitly requested full controller runtime.
            let mut arguments = vec!["probe", "--midi", "full", "--screen-control-stdin"];
            if enabled {
                arguments.push("--screens");
            }
            // When the CLI parses the managed screen invocation.
            let args = Args::try_parse_from(arguments)?;
            // Then both enabled and initially-off generations are admitted.
            assert!(args.screens.control_stdin);
            assert_eq!(args.screens.enabled, enabled);
        }
        Ok(())
    }

    #[test]
    fn jog_preferences_are_accepted_for_export_and_full_runtime() -> anyhow::Result<()> {
        // Given a jog configuration supplied to export and runtime commands.
        let valid = [
            "probe",
            "--generate-mapping",
            "new.plist",
            "--jog-config",
            "jog.json",
        ];
        // When the real argument parser accepts the supported invocation.
        let args = Args::try_parse_from(valid)?;
        // Then export and full runtime retain the path, while bare probe use fails.
        assert_eq!(args.jog_config, Some(PathBuf::from("jog.json")));
        assert!(Args::try_parse_from(["probe", "--jog-config", "jog.json"]).is_err());
        let full = Args::try_parse_from(["probe", "--midi", "full", "--jog-config", "jog.json"])?;
        assert_eq!(full.jog_config, Some(PathBuf::from("jog.json")));
        Ok(())
    }

    #[test]
    fn continuous_stop_accepts_idle_counts_but_bounded_probe_requires_activity()
    -> anyhow::Result<()> {
        let continuous = Args::try_parse_from(["probe", "--midi", "full", "--until-stopped"])?;
        assert!(report_probe_counts(&continuous, &ProbeCounts::default()).is_ok());
        let bounded = Args::try_parse_from(["probe", "--midi", "full", "--seconds", "60"])?;
        assert!(report_probe_counts(&bounded, &ProbeCounts::default()).is_err());
        Ok(())
    }
}
