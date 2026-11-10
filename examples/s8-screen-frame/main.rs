//! Screen phases 2, 4 and 5: send frames to the S8 panels, then release.
//!
//! Single frames (`black`, `f800`, `pattern`, `partial`) are one bulk OUT
//! transfer; `stream` paces full frames to one panel or alternating panels;
//! `burst` sends full frames back to back. Interface 6, endpoint `0x04`,
//! claimed once per invocation, 500 ms timeout per transfer, no retries, the
//! first error or timeout ends the run. At most 3,600 transfers and 60 s per
//! invocation. No configuration change, reset, alternate setting or
//! interface 5 output. Refuses to run while `NIHardwareAgent` holds the
//! panels. With `--dump`, nothing is opened: a single frame's bytes, or for
//! `stream` and `burst` the `stage seq panel length sha256` of every frame,
//! go to a file. One JSON summary line per stage goes to stdout.

mod modes;
#[path = "../common/picture_file.rs"]
mod picture_file;
mod pictures;
mod run;
mod transport;

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context as _, bail, ensure};
use clap::{Parser, Subcommand, ValueEnum};
use pictures::{Pick, Pictures};
use run::Job;
use s4mk3_hid_midi_bridge::device::s8::screen::Panel;
use transport::{DryRun, Session};

const WIDTH: usize = 480;
const HEIGHT: usize = 272;

/// kontrol-s8-protocol `BLACK_0`: REPEAT of one black pixel pair over the left panel.
const BLACK_0: [u8; 32] = [
    0x84, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xe0, 0x01, 0x10,
    0x01, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
];

#[derive(Clone, Copy, ValueEnum)]
enum Side {
    Left,
    Right,
}

impl From<Side> for Panel {
    fn from(side: Side) -> Self {
        match side {
            Side::Left => Self::Left,
            Side::Right => Self::Right,
        }
    }
}

/// What the panels already show when the pictures start.
#[derive(Clone, Copy, ValueEnum)]
enum Under {
    Black,
    F800,
}

/// Which panels `stream` feeds.
#[derive(Clone, Copy, ValueEnum)]
enum Panels {
    Left,
    Right,
    /// Left, right, left, ...; the rate counts per panel.
    Alternate,
}

#[derive(Clone, Copy, ValueEnum)]
enum Order {
    Rgb,
    Bgr,
}

#[derive(Clone, Copy, ValueEnum)]
enum Variant {
    /// Full-panel header, SKIP over the top half, LITERAL over the bottom half.
    Skip,
    /// Header naming x 160, y 96, 160x80, LITERAL over that area.
    Rect,
}

#[derive(Subcommand)]
enum Mode {
    /// 32-byte REPEAT of black (`BLACK_0` on the left panel).
    Black,
    /// 32-byte REPEAT of pixel `F8 00`: red if RGB565, blue if BGR565.
    #[command(name = "f800")]
    F800,
    /// Full 261,148-byte LITERAL test pattern.
    Pattern,
    /// Paced full LITERAL frames, each stamped with a counter and moving bar.
    /// One stage per rate, the interface held across stages.
    Stream {
        /// Frames per second on each panel, one stage each (1 to 60), e.g. 10,20,30,60.
        #[arg(long, value_delimiter = ',', required = true)]
        rates: Vec<u32>,
        /// Seconds per stage; stages and 2 s gaps together stay within 60 s.
        #[arg(long)]
        seconds: u32,
        #[arg(long, value_enum, default_value = "left")]
        panels: Panels,
    },

    /// Unpaced full frames to `--screen`, to measure the transfer ceiling.
    Burst {
        /// Number of frames (1 to 3600).
        #[arg(long)]
        frames: u32,
    },
    /// One frame that skips or addresses part of the panel (unverified format).
    Partial {
        #[arg(value_enum)]
        variant: Variant,
    },
}

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    mode: Mode,
    #[arg(long, value_enum, default_value = "left", global = true)]
    screen: Side,
    /// Wire colour order for the pattern, stream and burst frames.
    #[arg(long, value_enum, default_value = "rgb", global = true)]
    order: Order,
    /// Write frame bytes (one frame) or the hash list (several) here instead
    /// of sending.
    #[arg(long, global = true)]
    dump: Option<PathBuf>,
    /// Write the pattern as intended (logical RGB) to this PPM file.
    #[arg(long, global = true)]
    preview: Option<PathBuf>,
    /// Dry run that draws what the panels would show, decoded from the frame
    /// bytes, into this directory as 480x272 PNG files; sends nothing.
    #[arg(long, global = true, requires = "name")]
    png: Option<PathBuf>,
    /// Start of the picture file names, e.g. the runner step.
    #[arg(long, global = true, requires = "png")]
    name: Option<String>,
    /// Draw these frames of every stage (`last` or numbers from 0, per panel)
    /// instead of the first, the last of each stage and a spread.
    #[arg(long, global = true, value_delimiter = ',', requires = "png")]
    png_frames: Option<Vec<Pick>>,
    /// Show the frames over a solid panel instead of black.
    #[arg(long, value_enum, global = true, requires = "png")]
    under: Option<Under>,
    /// Compare the pictures with the files in `--png` instead of writing.
    #[arg(long, global = true, requires = "png")]
    png_check: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let panel = Panel::from(args.screen);
    let swap_rb = matches!(args.order, Order::Bgr);
    ensure!(
        modes::solid(Panel::Left, [0; 4]) == BLACK_0,
        "frame builder does not reproduce BLACK_0"
    );
    let job = match args.mode {
        Mode::Black => modes::single("black", panel, modes::solid(panel, [0; 4])),
        Mode::F800 => modes::single("f800", panel, modes::solid(panel, [0xf8, 0, 0xf8, 0])),
        Mode::Pattern => {
            let (frame, canvas) = modes::pattern_frame(panel, swap_rb);
            if let Some(path) = &args.preview {
                write_ppm(path, &canvas)?;
            }
            modes::single("pattern", panel, frame)
        }
        Mode::Stream {
            rates,
            seconds,
            panels,
        } => {
            let chosen = match panels {
                Panels::Left => vec![Panel::Left],
                Panels::Right => vec![Panel::Right],
                Panels::Alternate => vec![Panel::Left, Panel::Right],
            };
            modes::stream(&chosen, &rates, seconds, swap_rb)?
        }
        Mode::Burst { frames } => modes::burst(panel, frames, swap_rb)?,
        Mode::Partial { variant } => match variant {
            Variant::Skip => modes::single("partial-skip", panel, modes::partial_skip(panel)),
            Variant::Rect => modes::single("partial-rect", panel, modes::partial_rect(panel)),
        },
    };

    let pictures = match args.png {
        Some(dir) => {
            let under = args.under.map(|under| match under {
                Under::Black => [0; 4],
                Under::F800 => [0xf8, 0, 0xf8, 0],
            });
            let options = pictures::Options {
                dir,
                name: args.name.unwrap_or_default(),
                picks: args.png_frames,
                check: args.png_check,
            };
            Some(Pictures::new(options, under)?)
        }
        None => None,
    };
    if args.dump.is_some() || pictures.is_some() {
        dry_run(job, args.dump.as_deref(), pictures)
    } else {
        send(job)
    }
}

fn dry_run(mut job: Job, dump: Option<&Path>, pictures: Option<Pictures>) -> anyhow::Result<()> {
    let raw = job.raw.take();
    let mut dry = DryRun::default();
    dry.pictures = pictures;
    let drawing = dry.pictures.is_some();
    let error = if raw.is_none() || drawing {
        run::run_job(&mut job, &mut dry, true)
    } else {
        None
    };
    if let Some(error) = error {
        bail!("{error}");
    }
    if let Some(pictures) = &dry.pictures {
        println!("{}", pictures.report());
    }
    let Some(path) = dump else {
        return Ok(());
    };
    let bytes = raw.unwrap_or_else(|| (dry.rows.join("\n") + "\n").into_bytes());
    File::create_new(path)
        .and_then(|mut file| file.write_all(&bytes))
        .with_context(|| format!("write {}", path.display()))?;
    println!("wrote {} bytes to {}", bytes.len(), path.display());
    Ok(())
}

fn send(mut job: Job) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut session = Session::open()?;
    println!(
        "{:.3} claimed interface 6; {}",
        started.elapsed().as_secs_f64(),
        job.step
    );
    let error = run::run_job(&mut job, &mut session, false);
    drop(session);
    println!("{:.3} released", started.elapsed().as_secs_f64());
    match error {
        Some(error) => bail!("{error}"),
        None => Ok(()),
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
