//! Record S8 HID input and every S8 `CoreMIDI` endpoint without sending anything.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::num::NonZeroU64;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context as _;
use clap::Parser;
use core_foundation::base::TCFType as _;
use core_foundation::runloop::CFRunLoop;
use core_foundation::string::CFString;
use crossbeam_channel::{Receiver, Sender};
use hidapi::HidApi;
use midir::{MidiInput, MidiInputConnection, MidiOutput};
use s4mk3_hid_midi_bridge::device::S8;
use serde_json::{Value, json};

static STARTED: LazyLock<Instant> = LazyLock::new(Instant::now);

#[derive(Parser)]
struct Args {
    #[arg(long)]
    out: PathBuf,
    #[arg(long, default_value = "120")]
    seconds: NonZeroU64,
    /// Substring that selects `CoreMIDI` endpoints to record.
    #[arg(long, default_value = "Kontrol S8")]
    midi_match: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let started = *STARTED;
    let mut out = BufWriter::new(File::create(&args.out).context("create recorder output")?);
    let (sender, receiver) = crossbeam_channel::unbounded::<Value>();
    let running = Arc::new(AtomicBool::new(true));
    let stop = Arc::clone(&running);
    ctrlc::set_handler(move || stop.store(false, Ordering::SeqCst))
        .context("install Ctrl-C handler")?;

    let readers = open_hid(&sender, &running)?;
    // CoreMIDI refreshes this process's endpoint list only while the run loop
    // of the thread that created the first client processes notifications.
    let lister = (
        MidiInput::new("S8 passive recorder").context("create MIDI client")?,
        MidiOutput::new("S8 passive recorder").context("create MIDI client")?,
    );
    let mode = CFString::from_static_string("kCFRunLoopDefaultMode");
    let mut connections = BTreeMap::new();
    let mut endpoints = (Vec::new(), Vec::new());
    let deadline = Duration::from_secs(args.seconds.get());
    while running.load(Ordering::SeqCst) && started.elapsed() < deadline {
        poll_midi(
            &lister,
            &args.midi_match,
            &sender,
            &mut connections,
            &mut endpoints,
        )?;
        CFRunLoop::run_in_mode(
            mode.as_concrete_TypeRef(),
            Duration::from_millis(250),
            false,
        );
        drain(&receiver, &mut out)?;
    }
    running.store(false, Ordering::SeqCst);
    drop(connections);
    for reader in readers {
        reader
            .join()
            .map_err(|_| anyhow::anyhow!("HID reader thread panicked"))?;
    }
    send(&sender, json!({"src": "end"}));
    drain(&receiver, &mut out)
}

fn open_hid(
    sender: &Sender<Value>,
    running: &Arc<AtomicBool>,
) -> anyhow::Result<Vec<thread::JoinHandle<()>>> {
    let api = HidApi::new().context("initialize macOS-native HID access")?;
    let mut readers = Vec::new();
    for info in api
        .device_list()
        .filter(|info| S8.matches(info.vendor_id(), info.product_id()))
    {
        let device = api
            .open_path(info.path())
            .context("open S8 HID collection in shared mode")?;
        send(
            sender,
            json!({"src": "hid-open", "usage_page": info.usage_page(), "usage": info.usage()}),
        );
        let sender = sender.clone();
        let running = Arc::clone(running);
        readers.push(thread::spawn(move || {
            let mut buffer = [0_u8; 512];
            while running.load(Ordering::SeqCst) {
                match device.read_timeout(&mut buffer, 100) {
                    Ok(0) => {}
                    Ok(length) => {
                        let report = &buffer[..length];
                        send(
                            &sender,
                            json!({"src": "hid", "id": report[0], "len": length, "hex": hex(report)}),
                        );
                    }
                    Err(error) => {
                        send(
                            &sender,
                            json!({"src": "hid-error", "error": error.to_string()}),
                        );
                        return;
                    }
                }
            }
        }));
    }
    if readers.is_empty() {
        send(sender, json!({"src": "hid-absent"}));
    }
    Ok(readers)
}

fn poll_midi(
    (input, output): &(MidiInput, MidiOutput),
    matcher: &str,
    sender: &Sender<Value>,
    connections: &mut BTreeMap<String, MidiInputConnection<()>>,
    endpoints: &mut (Vec<String>, Vec<String>),
) -> anyhow::Result<()> {
    let sources: Vec<String> = input
        .ports()
        .iter()
        .filter_map(|port| input.port_name(port).ok())
        .filter(|name| name.contains(matcher))
        .collect();
    let destinations: Vec<String> = output
        .ports()
        .iter()
        .filter_map(|port| output.port_name(port).ok())
        .filter(|name| name.contains(matcher))
        .collect();
    if endpoints.0 != sources || endpoints.1 != destinations {
        send(
            sender,
            json!({"src": "midi-endpoints", "sources": sources, "destinations": destinations}),
        );
        *endpoints = (sources.clone(), destinations);
    }
    connections.retain(|name, _| sources.contains(name));
    for name in sources {
        if connections.contains_key(&name) {
            continue;
        }
        let input = MidiInput::new("S8 passive recorder").context("create MIDI client")?;
        let Some(port) = input.ports().into_iter().find(|port| {
            input
                .port_name(port)
                .is_ok_and(|candidate| candidate == name)
        }) else {
            continue;
        };
        let messages = sender.clone();
        let source = name.clone();
        match input.connect(
            &port,
            "S8 passive recorder input",
            move |_, bytes, ()| {
                send(
                    &messages,
                    json!({"src": "midi", "port": source, "hex": hex(bytes)}),
                );
            },
            (),
        ) {
            Ok(connection) => {
                send(sender, json!({"src": "midi-connect", "port": name}));
                connections.insert(name, connection);
            }
            Err(error) => send(
                sender,
                json!({"src": "midi-connect-error", "port": name, "error": error.to_string()}),
            ),
        }
    }
    Ok(())
}

fn drain(receiver: &Receiver<Value>, out: &mut BufWriter<File>) -> anyhow::Result<()> {
    let mut next = receiver.try_recv().ok();
    while let Some(record) = next {
        serde_json::to_writer(&mut *out, &record).context("write record")?;
        out.write_all(b"\n").context("write record")?;
        next = receiver.try_recv().ok();
    }
    out.flush().context("flush recorder output")
}

fn send(sender: &Sender<Value>, mut record: Value) {
    if let Some(fields) = record.as_object_mut() {
        fields.insert("t".into(), json!(STARTED.elapsed().as_secs_f64()));
        let unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis());
        fields.insert("unix_ms".into(), json!(unix));
    }
    if sender.send(record).is_err() {
        eprintln!("recorder output closed");
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}
