//! Screen phases 2 and 4: send one frame to one S8 panel, then release.
//!
//! One bulk OUT transfer on interface 6, endpoint `0x04`, 500 ms timeout, no
//! retries. No configuration change, reset, alternate setting or interface 5
//! output. Refuses to run while `NIHardwareAgent` holds the panels. With
//! `--dump`, writes the frame bytes to a file and opens no device.

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail, ensure};
use clap::{Parser, ValueEnum};
use nusb::MaybeFuture as _;
use nusb::transfer::{Bulk, Out};
use s4mk3_hid_midi_bridge::device::S8;
use s4mk3_hid_midi_bridge::screens::{self, Display, Frame, rgb};

const AGENT: &str = "/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app/Contents/MacOS/NIHardwareAgent";
const SCREEN_INTERFACE: u8 = 6;
const SCREEN_ENDPOINT: u8 = 0x04;
const TIMEOUT: Duration = Duration::from_millis(500);
const WIDTH: usize = 480;
const HEIGHT: usize = 272;

/// kontrol-s8-protocol `BLACK_0`: REPEAT of one black pixel pair over the left panel.
const BLACK_0: [u8; 32] = [
    0x84, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xe0, 0x01, 0x10,
    0x01, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
];

#[derive(Clone, Copy, ValueEnum)]
enum Kind {
    /// 32-byte REPEAT of black (`BLACK_0` on the left panel).
    Black,
    /// 32-byte REPEAT of pixel `F8 00`: red if RGB565, blue if BGR565.
    F800,
    /// Full 261,148-byte LITERAL test pattern.
    Pattern,
}

#[derive(Clone, Copy, ValueEnum)]
enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, ValueEnum)]
enum Order {
    Rgb,
    Bgr,
}

#[derive(Parser)]
struct Args {
    frame: Kind,
    #[arg(long, value_enum, default_value = "left")]
    screen: Side,
    /// Wire colour order for the pattern.
    #[arg(long, value_enum, default_value = "rgb")]
    order: Order,
    /// Write the frame bytes here instead of sending them.
    #[arg(long)]
    dump: Option<PathBuf>,
    /// Write the pattern as intended (logical RGB) to this PPM file.
    #[arg(long)]
    preview: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let screen = match args.screen {
        Side::Left => 0,
        Side::Right => 1,
    };
    ensure!(
        repeat(0, [0; 4]) == BLACK_0,
        "frame builder does not reproduce BLACK_0"
    );
    let frame = match args.frame {
        Kind::Black => repeat(screen, [0; 4]),
        Kind::F800 => repeat(screen, [0xf8, 0x00, 0xf8, 0x00]),
        Kind::Pattern => {
            let pixels = pattern(args.screen);
            if let Some(path) = &args.preview {
                write_ppm(path, &pixels)?;
            }
            literal(screen, &pixels, args.order)
        }
    };
    if let Some(path) = &args.dump {
        File::create_new(path)
            .and_then(|mut file| file.write_all(&frame))
            .with_context(|| format!("write {}", path.display()))?;
        println!("wrote {} bytes to {}", frame.len(), path.display());
        return Ok(());
    }
    send(&frame)
}

const fn header(screen: u8) -> [u8; 16] {
    [
        0x84, 0x00, screen, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xe0, 0x01,
        0x10,
    ]
}

const fn footer(screen: u8) -> [u8; 8] {
    [0x03, 0x00, 0x00, 0x00, 0x40, 0x00, screen, 0x00]
}

/// One command covering all 0xFF00 pixel pairs of the panel.
const fn command(opcode: u8) -> [u8; 4] {
    [opcode, 0x00, 0xff, 0x00]
}

fn repeat(screen: u8, pair: [u8; 4]) -> Vec<u8> {
    [&header(screen)[..], &command(0x01), &pair, &footer(screen)].concat()
}

fn literal(screen: u8, pixels: &[u16], order: Order) -> Vec<u8> {
    let mut frame = Vec::with_capacity(16 + 4 + pixels.len() * 2 + 8);
    frame.extend_from_slice(&header(screen));
    frame.extend_from_slice(&command(0x00));
    for pixel in pixels {
        let wire = match order {
            Order::Rgb => *pixel,
            Order::Bgr => ((pixel & 0x1f) << 11) | (pixel & 0x07e0) | (pixel >> 11),
        };
        frame.extend_from_slice(&wire.to_be_bytes());
    }
    frame.extend_from_slice(&footer(screen));
    frame
}

/// Labelled colour bars from the S4 raster, centred on the S8 panel, with a
/// grey border and white corner markers (large top left).
fn pattern(side: Side) -> Vec<u16> {
    let background = rgb(12, 16, 24);
    let white = rgb(255, 255, 255);
    let mut labels = Frame::default();
    labels.clear(background);
    let title = match side {
        Side::Left => "LEFT S8 SCREEN",
        Side::Right => "RIGHT S8 SCREEN",
    };
    labels.text(16, 16, title, 2, white);
    for (index, (label, color)) in [
        ("RED", rgb(255, 0, 0)),
        ("GREEN", rgb(0, 255, 0)),
        ("BLUE", rgb(0, 0, 255)),
        ("WHITE", white),
    ]
    .into_iter()
    .enumerate()
    {
        labels.rect(16 + index * 76, 64, 60, 110, color);
        labels.text(16 + index * 76, 184, label, 1, white);
    }
    labels.text(16, 216, "480 X 272 LITERAL", 1, white);
    let mut packet = Vec::new();
    labels.packet(Display::Left, &mut packet);

    let mut canvas = vec![background; WIDTH * HEIGHT];
    let (left, top) = ((WIDTH - screens::WIDTH) / 2, (HEIGHT - screens::HEIGHT) / 2);
    for (index, [high, low]) in packet[16..16 + screens::PIXEL_BYTES]
        .as_chunks::<2>()
        .0
        .iter()
        .enumerate()
    {
        let (x, y) = (index % screens::WIDTH, index / screens::WIDTH);
        canvas[(top + y) * WIDTH + left + x] = u16::from_be_bytes([*high, *low]);
    }
    let grey = rgb(128, 128, 128);
    fill(&mut canvas, 0, 0, WIDTH, 1, grey);
    fill(&mut canvas, 0, HEIGHT - 1, WIDTH, 1, grey);
    fill(&mut canvas, 0, 0, 1, HEIGHT, grey);
    fill(&mut canvas, WIDTH - 1, 0, 1, HEIGHT, grey);
    fill(&mut canvas, 0, 0, 16, 16, white);
    fill(&mut canvas, WIDTH - 8, 0, 8, 8, white);
    fill(&mut canvas, 0, HEIGHT - 8, 8, 8, white);
    fill(&mut canvas, WIDTH - 8, HEIGHT - 8, 8, 8, white);
    canvas
}

fn fill(canvas: &mut [u16], x: usize, y: usize, width: usize, height: usize, color: u16) {
    for row in y..y + height {
        canvas[row * WIDTH + x..row * WIDTH + x + width].fill(color);
    }
}

fn write_ppm(path: &Path, pixels: &[u16]) -> anyhow::Result<()> {
    let mut bytes = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
    for pixel in pixels {
        let scale = |value: u16, max: u16| u8::try_from(value * 255 / max);
        bytes.extend_from_slice(&[
            scale(pixel >> 11, 31)?,
            scale((pixel >> 5) & 63, 63)?,
            scale(pixel & 31, 31)?,
        ]);
    }
    File::create_new(path)
        .and_then(|mut file| file.write_all(&bytes))
        .with_context(|| format!("write {}", path.display()))
}

fn send(frame: &[u8]) -> anyhow::Result<()> {
    let agent = Command::new("pgrep")
        .args(["-f", AGENT])
        .output()
        .context("run pgrep")?;
    if !agent.stdout.is_empty() {
        bail!(
            "NIHardwareAgent is running (pid {}); stop it first",
            String::from_utf8_lossy(&agent.stdout).trim()
        );
    }

    let mut devices = nusb::list_devices()
        .wait()?
        .filter(|device| S8.matches(device.vendor_id(), device.product_id()));
    let info = devices.next().context("no connected Kontrol S8")?;
    ensure!(devices.next().is_none(), "connect only one Kontrol S8");

    let started = Instant::now();
    let device = info.open().wait().context("open S8 USB device")?;
    let interface = device
        .claim_interface(SCREEN_INTERFACE)
        .wait()
        .context("claim S8 interface 6")?;
    let mut endpoint = interface
        .endpoint::<Bulk, Out>(SCREEN_ENDPOINT)
        .context("open S8 screen bulk OUT 0x04")?;
    println!(
        "{:.3} claimed interface {SCREEN_INTERFACE}; sending {} bytes",
        started.elapsed().as_secs_f64(),
        frame.len()
    );
    let completion = endpoint.transfer_blocking(frame.into(), TIMEOUT);
    println!(
        "{:.3} transfer {:?}, {} of {} bytes",
        started.elapsed().as_secs_f64(),
        completion.status,
        completion.actual_len,
        frame.len()
    );
    drop(endpoint);
    drop(interface);
    drop(device);
    println!("{:.3} released", started.elapsed().as_secs_f64());
    completion.status.context("S8 screen bulk transfer")?;
    ensure!(
        completion.actual_len == frame.len(),
        "short S8 screen write: {}/{}",
        completion.actual_len,
        frame.len()
    );
    Ok(())
}
