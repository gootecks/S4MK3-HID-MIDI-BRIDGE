//! Passive S8 HID input recorder: prints one line per changed control field.
//!
//! Opens only the S8 control interface (5) in macOS shared mode and reads input
//! reports. Nothing is ever written to the device: no output or feature
//! reports, no control transfers, no screen traffic. Field names come from the
//! kontrol-s8-protocol `HID_INPUT.json` (CC BY 4.0, `P:`) and the Encdr
//! `ni_kontrol_s8.json` descriptor (`E:`); `?` marks bits neither describes.
//!
//! Type a label and press Enter to stamp a step marker into the output and the
//! JSONL log, so each physical control's events can be grouped afterwards.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead as _, BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context as _;
use clap::Parser;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use fields::{Fields, Group, Key};
use hidapi::HidApi;
use s4mk3_hid_midi_bridge::device::S8;
use serde_json::{Value, json};

mod fields;

/// The S8 control HID interface. Interface 4 and the screen interface 6 are never opened.
const CONTROL_INTERFACE: i32 = 5;

#[derive(Parser)]
#[command(about = "Passively diff S8 HID input reports into per-field change lines")]
struct Args {
    /// Raw JSONL log (reports, changes and step markers). Default: /tmp/s8-input-<epoch>.jsonl
    #[arg(long)]
    log: Option<PathBuf>,
    /// Suppress analog changes of this many counts or fewer from the last printed value.
    #[arg(long, default_value = "3")]
    deadband: u32,
    /// Label active from the start, before any stdin marker.
    #[arg(long)]
    step: Option<String>,
    /// Stop after this many seconds (default: until Ctrl-C).
    #[arg(long)]
    seconds: Option<u64>,
    /// kontrol-s8-protocol `HID_INPUT.json`.
    #[arg(long, default_value = "../kontrol-s8-protocol/data/HID_INPUT.json")]
    protocol: PathBuf,
    /// Encdr `ni_kontrol_s8.json` descriptor.
    #[arg(long, default_value = "../Encdr/encdr/descriptors/ni_kontrol_s8.json")]
    encdr: PathBuf,
    /// Also print the free-running heartbeat bytes (Encdr touchstrip quirk).
    #[arg(long)]
    show_heartbeat: bool,
    /// List S8 HID enumeration entries and exit without opening anything.
    #[arg(long)]
    list: bool,
    /// Decode an earlier JSONL log instead of opening the S8 (no hardware access).
    #[arg(long)]
    replay: Option<PathBuf>,
}

enum Event {
    Report(Duration, Vec<u8>),
    Step(String),
    Error(String),
}

struct Recorder<'a> {
    fields: Fields,
    deadband: u32,
    show_heartbeat: bool,
    step: String,
    previous: BTreeMap<u8, Vec<u8>>,
    reported: BTreeMap<Key, u32>,
    log: BufWriter<File>,
    out: &'a mut dyn std::io::Write,
}

impl Recorder<'_> {
    fn log(&mut self, record: &Value) -> anyhow::Result<()> {
        writeln!(self.log, "{record}").context("write log")
    }

    fn report(&mut self, at: Duration, report: &[u8]) -> anyhow::Result<()> {
        let Some(&id) = report.first() else {
            return Ok(());
        };
        let t = at.as_secs_f64();
        let wall_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        self.log(
            &json!({"src": "hid", "t": t, "wall_ms": wall_ms, "step": self.step,
            "id": id, "len": report.len(), "hex": hex(report)}),
        )?;
        let previous = self.previous.insert(id, report.to_vec());
        let Some(previous) = previous.filter(|p| p.len() == report.len()) else {
            writeln!(self.out, "{t:9.3}s r{id} baseline len={}", report.len())?;
            for (key, group) in &self.fields.groups {
                if key.0 == id
                    && let Some(value) = group.value(report)
                {
                    self.reported.insert(*key, value);
                }
            }
            return Ok(());
        };
        let hide_heartbeat = !self.show_heartbeat;
        let mut covered = vec![0_u8; report.len()];
        let mut lines = Vec::new();
        for (key, group) in self.fields.groups.iter().filter(|(key, _)| key.0 == id) {
            for (index, byte) in covered
                .iter_mut()
                .enumerate()
                .skip(group.offset)
                .take(group.len)
            {
                *byte |= u8::try_from((group.mask >> (8 * (index - group.offset))) & 0xFF)
                    .unwrap_or(0xFF);
            }
            let (Some(old), Some(new)) = (group.value(&previous), group.value(report)) else {
                continue;
            };
            if old == new || (group.heartbeat && hide_heartbeat) {
                continue;
            }
            if group.is_analog() {
                let last = self.reported.get(key).copied().unwrap_or(old);
                if new.abs_diff(last) <= self.deadband {
                    continue;
                }
            }
            self.reported.insert(*key, new);
            lines.push(describe(id, group, old, new));
        }
        for (index, (&old, &new)) in previous.iter().zip(report).enumerate().skip(1) {
            let unknown = (old ^ new) & !covered[index];
            if unknown != 0 {
                lines.push(format!(
                    "r{id} [{index}] mask {unknown:#04x} {:#04x}->{:#04x} ?",
                    old & unknown,
                    new & unknown
                ));
            }
        }
        for line in lines {
            writeln!(self.out, "{t:9.3}s {line}")?;
            self.log(&json!({"src": "change", "t": t, "wall_ms": wall_ms,
                "step": self.step, "line": line}))?;
        }
        Ok(())
    }

    fn step(&mut self, at: Duration, label: String) -> anyhow::Result<()> {
        writeln!(self.out, "\n=== step: {label} ===")?;
        self.log(&json!({"src": "step", "t": at.as_secs_f64(), "label": label}))?;
        self.step = label;
        Ok(())
    }
}

fn describe(id: u8, group: &Group, old: u32, new: u32) -> String {
    let range = if group.len == 1 {
        format!("[{}]", group.offset)
    } else {
        format!("[{}..{}]", group.offset, group.offset + group.len - 1)
    };
    let mask = if group.mask.is_power_of_two() || group.len == 1 && group.mask != 0xFF {
        format!(" mask {:#04x}", group.mask)
    } else if group.mask != 0xFFFF && group.mask != 0xFF {
        format!(" mask {:#06x}", group.mask)
    } else {
        String::new()
    };
    let delta = match group.modulus {
        Some(modulus) => {
            let half = i64::from(modulus / 2);
            let wrapped =
                (i64::from(new) - i64::from(old) + half).rem_euclid(i64::from(modulus)) - half;
            format!(" ({wrapped:+} mod {modulus})")
        }
        None if group.is_analog() => format!(" ({:+})", i64::from(new) - i64::from(old)),
        None => String::new(),
    };
    format!(
        "r{id} {range}{mask} {old}->{new}{delta} {}",
        group.names.join(" | ")
    )
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        for nibble in [byte >> 4, byte & 0x0F] {
            out.extend(char::from_digit(u32::from(nibble), 16));
        }
    }
    out
}

fn open_reader(
    sender: &Sender<Event>,
    running: &Arc<AtomicBool>,
    started: Instant,
) -> anyhow::Result<thread::JoinHandle<()>> {
    let api = HidApi::new().context("initialize macOS-native HID access")?;
    let info = api
        .device_list()
        .find(|info| {
            S8.matches(info.vendor_id(), info.product_id())
                && info.interface_number() == CONTROL_INTERFACE
        })
        .context("S8 control interface 5 not found (run with --list)")?;
    let device = api
        .open_path(info.path())
        .context("open S8 control interface in shared mode")?;
    let sender = sender.clone();
    let running = Arc::clone(running);
    Ok(thread::spawn(move || {
        let mut buffer = [0_u8; 512];
        while running.load(Ordering::SeqCst) {
            match device.read_timeout(&mut buffer, 100) {
                Ok(0) => {}
                Ok(length) => {
                    let report = Event::Report(started.elapsed(), buffer[..length].to_vec());
                    if sender.send(report).is_err() {
                        return;
                    }
                }
                Err(error) => {
                    sender.send(Event::Error(error.to_string())).ok();
                    return;
                }
            }
        }
    }))
}

fn list() -> anyhow::Result<()> {
    let api = HidApi::new().context("initialize macOS-native HID access")?;
    for info in api
        .device_list()
        .filter(|info| S8.matches(info.vendor_id(), info.product_id()))
    {
        println!(
            "interface {} usage_page {:#06x} usage {:#06x}",
            info.interface_number(),
            info.usage_page(),
            info.usage()
        );
    }
    Ok(())
}

fn spawn_stdin(sender: Sender<Event>) {
    thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            let label = line.trim();
            if !label.is_empty() && sender.send(Event::Step(label.to_owned())).is_err() {
                return;
            }
        }
    });
}

/// Feed the `hid` and `step` records of an earlier JSONL log through the decoder (no hardware).
fn replay(path: &Path, sender: &Sender<Event>) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    for record in text
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
    {
        match record["src"].as_str() {
            Some("hid") => {
                let hex = record["hex"].as_str().unwrap_or_default();
                let bytes = (0..hex.len() / 2)
                    .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16))
                    .collect::<Result<Vec<_>, _>>()?;
                let at = Duration::from_secs_f64(record["t"].as_f64().unwrap_or_default());
                sender.send(Event::Report(at, bytes))?;
            }
            Some("step") => {
                let label = record["label"].as_str().unwrap_or_default().to_owned();
                sender.send(Event::Step(label))?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.list {
        return list();
    }
    let started = Instant::now();
    let log_path = args.log.unwrap_or_else(|| {
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        PathBuf::from(format!("/tmp/s8-input-{epoch}.jsonl"))
    });
    let mut fields = Fields::default();
    fields.load_encdr(&args.encdr)?;
    fields.load_protocol(&args.protocol)?;
    let log = BufWriter::new(File::create(&log_path).context("create log")?);
    let mut stdout = std::io::stdout().lock();
    let mut recorder = Recorder {
        fields,
        deadband: args.deadband,
        show_heartbeat: args.show_heartbeat,
        step: String::new(),
        previous: BTreeMap::new(),
        reported: BTreeMap::new(),
        log,
        out: &mut stdout,
    };
    writeln!(
        recorder.out,
        "log: {}\n{} field groups; deadband {}; type a label + Enter for a step marker; Ctrl-C stops",
        log_path.display(),
        recorder.fields.groups.len(),
        args.deadband
    )?;

    let running = Arc::new(AtomicBool::new(true));
    let stop = Arc::clone(&running);
    ctrlc::set_handler(move || stop.store(false, Ordering::SeqCst))
        .context("install Ctrl-C handler")?;
    let (sender, receiver): (Sender<Event>, Receiver<Event>) = crossbeam_channel::unbounded();
    let reader = if let Some(path) = &args.replay {
        replay(path, &sender)?;
        drop(sender);
        None
    } else {
        let reader = open_reader(&sender, &running, started)?;
        spawn_stdin(sender);
        Some(reader)
    };
    if let Some(label) = args.step {
        recorder.step(started.elapsed(), label)?;
    }
    let deadline = args.seconds.map(Duration::from_secs);
    while running.load(Ordering::SeqCst) && deadline.is_none_or(|d| started.elapsed() < d) {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(Event::Report(at, report)) => recorder.report(at, &report)?,
            Ok(Event::Step(label)) => recorder.step(started.elapsed(), label)?,
            Ok(Event::Error(error)) => anyhow::bail!("HID read failed: {error}"),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        recorder.log.flush()?;
    }
    running.store(false, Ordering::SeqCst);
    if let Some(reader) = reader {
        reader
            .join()
            .map_err(|_| anyhow::anyhow!("HID reader thread panicked"))?;
    }
    recorder.log.flush()?;
    Ok(())
}
