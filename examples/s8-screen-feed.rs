//! Writes `S8FR` frame records of the stamped test pattern to stdout, for
//! `s8-screen-feed … | s4mk3-hid-midi-bridge --midi full --s8-screens-stdin`.
//!
//! The tool never opens a device. Each panel gets `--frames` records, paced
//! at `--rate` records per second per panel (alternating panels interleave, so
//! the stream carries twice that). Record layout: `docs/s8/screen-transport.md`.
//! With `--dry-run` nothing is paced and stdout carries `seq panel length
//! sha256` per record instead of the records. `--out PATH` writes the records
//! to a file instead of stdout. At most 3,600 records per invocation.
//!
//! With `--png DIR --name NAME` the tool also draws pictures of what the
//! bridge would put on the panels: each record goes through the bridge's own
//! record reader and frame builder (`wire::read_record`, `screen::full`), the
//! frame bytes are applied to a decoded framebuffer, and the frames named by
//! `--png-frames` (counted from 0 per panel) are written as
//! `DIR/NAME-{left|right}-{frame}.png`. `--png-exit` adds the exit frame
//! (`worker::EXIT_PIXELS`) and writes the first panel to `DIR/NAME.png`.
//! `--png-check` compares with the files instead of writing them.

#[path = "common/picture_file.rs"]
mod picture_file;

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, Cursor, Write};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};
use clap::{Parser, ValueEnum};
use s4mk3_hid_midi_bridge::device::s8::screen::decode::Panels as Framebuffers;
use s4mk3_hid_midi_bridge::device::s8::screen::wire::{HEADER_BYTES, MAGIC, read_record};
use s4mk3_hid_midi_bridge::device::s8::screen::worker::EXIT_PIXELS;
use s4mk3_hid_midi_bridge::device::s8::screen::{self, PIXEL_BYTES, Panel, pattern};
use sha2::{Digest as _, Sha256};

const MAX_RECORDS: u32 = 3_600;
const MAX_RATE: u32 = 120;

#[derive(Clone, Copy, ValueEnum)]
enum Panels {
    Left,
    Right,
    Alternate,
}

impl Panels {
    const fn list(self) -> &'static [Panel] {
        match self {
            Self::Left => &[Panel::Left],
            Self::Right => &[Panel::Right],
            Self::Alternate => &[Panel::Left, Panel::Right],
        }
    }
}

#[derive(Parser)]
#[command(about = "Write S8FR records of the stamped test pattern to stdout")]
struct Args {
    /// Panel or panels that receive frames.
    #[arg(long, value_enum, default_value = "left")]
    panels: Panels,

    /// Records per second per panel.
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..=i64::from(MAX_RATE)))]
    rate: u32,

    /// Records per panel.
    #[arg(long, default_value_t = 50)]
    frames: u32,

    /// Pace nothing and print `seq panel length sha256` per record.
    #[arg(long)]
    dry_run: bool,

    /// Write the records to this file instead of stdout.
    #[arg(long, value_name = "PATH")]
    out: Option<PathBuf>,

    /// Draw pictures of the frames the bridge would send into this directory.
    #[arg(long, value_name = "DIR", requires = "name")]
    png: Option<PathBuf>,

    /// Start of the picture file names, e.g. the runner step.
    #[arg(long, requires = "png")]
    name: Option<String>,

    /// Frame numbers to draw, counted from 0 per panel.
    #[arg(long, value_delimiter = ',', requires = "png")]
    png_frames: Vec<u32>,

    /// Also draw the first panel after the exit frame, as NAME.png.
    #[arg(long, requires = "png")]
    png_exit: bool,

    /// Compare the pictures with the files instead of writing them.
    #[arg(long, requires = "png")]
    png_check: bool,
}

/// Pictures of the frames the bridge would send, see the module comment.
struct Shots {
    dir: PathBuf,
    name: String,
    frames: Vec<u32>,
    check: bool,
    exit: bool,
    buffers: Framebuffers,
    seen: [u32; 2],
    first: Option<Panel>,
    written: u32,
}

impl Shots {
    fn new(args: &Args) -> Option<Self> {
        let dir = args.png.clone()?;
        Some(Self {
            dir,
            name: args.name.clone().unwrap_or_default(),
            frames: args.png_frames.clone(),
            check: args.png_check,
            exit: args.png_exit,
            buffers: Framebuffers::default(),
            seen: [0; 2],
            first: None,
            written: 0,
        })
    }

    /// Reads `record` the way the bridge does, builds the frame it would send
    /// and applies it; draws the picture when the frame number was asked for.
    fn observe(&mut self, record: &[u8]) -> anyhow::Result<()> {
        let mut pixels = pattern::pixel_buffer();
        let panel = read_record(&mut Cursor::new(record), &mut pixels)?
            .context("the feeder wrote an empty record")?;
        let mut frame = Vec::with_capacity(screen::FRAME_BYTES);
        screen::full(&mut frame, panel, &pixels);
        let applied = self.buffers.apply(&frame)?;
        if applied != panel {
            bail!("frame addressed to another panel than the record");
        }
        self.first.get_or_insert(panel);
        let index = usize::from(panel.index());
        let number = self.seen[index];
        self.seen[index] += 1;
        if self.frames.contains(&number) {
            self.picture(panel, &format!("{}-{}-{number}", self.name, side(panel)))?;
        }
        Ok(())
    }

    /// Applies the exit frame the bridge sends to every panel that got a
    /// frame and draws the first panel.
    fn finish(&mut self) -> anyhow::Result<()> {
        if !self.exit {
            return Ok(());
        }
        let mut frame = Vec::with_capacity(screen::FRAME_BYTES);
        for panel in [Panel::Left, Panel::Right] {
            if self.seen[usize::from(panel.index())] > 0 {
                screen::solid(&mut frame, panel, EXIT_PIXELS);
                self.buffers.apply(&frame)?;
            }
        }
        let first = self.first.context("no record, so no exit picture")?;
        self.picture(first, &self.name.clone())
    }

    fn picture(&mut self, panel: Panel, stem: &str) -> anyhow::Result<()> {
        let path = self.dir.join(format!("{stem}.png"));
        let rgb = self.buffers.rgb888(panel);
        if self.check {
            picture_file::compare(&path, &rgb)?;
        } else {
            fs::create_dir_all(&self.dir)?;
            picture_file::write(&path, &rgb)?;
        }
        self.written += 1;
        Ok(())
    }

    fn report(&self) -> String {
        let verb = if self.check { "checked" } else { "wrote" };
        format!("{verb} {} pictures in {}", self.written, self.dir.display())
    }
}

const fn side(panel: Panel) -> &'static str {
    match panel {
        Panel::Left => "left",
        Panel::Right => "right",
    }
}

fn header(panel: Panel) -> anyhow::Result<[u8; HEADER_BYTES]> {
    let length = u32::try_from(PIXEL_BYTES).context("payload length")?;
    let mut bytes = [0_u8; HEADER_BYTES];
    bytes[..4].copy_from_slice(&MAGIC);
    bytes[4] = panel.index();
    bytes[8..].copy_from_slice(&length.to_le_bytes());
    Ok(bytes)
}

/// Writes every record of the stream to `records`, calling `each` with the
/// sequence number, panel and bytes of each one just before it is written.
fn feed(
    args: &Args,
    records: &mut impl Write,
    mut each: impl FnMut(u32, Panel, &[u8]) -> anyhow::Result<()>,
    mut pace: impl FnMut(u32),
) -> anyhow::Result<()> {
    let panels = args.panels.list();
    let mut canvases: Vec<Vec<u16>> = panels.iter().map(|panel| pattern::canvas(*panel)).collect();
    let mut pixels = pattern::pixel_buffer();
    let mut record = Vec::with_capacity(HEADER_BYTES + PIXEL_BYTES);
    let mut seq = 0_u32;
    for frame in 0..args.frames {
        for (panel, canvas) in panels.iter().zip(canvases.iter_mut()) {
            pattern::stamp(canvas, frame);
            pattern::wire(canvas, false, &mut pixels);
            record.clear();
            record.extend_from_slice(&header(*panel)?);
            record.extend_from_slice(pixels.as_slice());
            pace(seq);
            each(seq, *panel, &record)?;
            records.write_all(&record)?;
            records.flush()?;
            seq += 1;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let total = args
        .frames
        .checked_mul(u32::try_from(args.panels.list().len())?)
        .filter(|total| (1..=MAX_RECORDS).contains(total));
    let Some(total) = total else {
        bail!("--frames must give 1 to {MAX_RECORDS} records in total");
    };
    let mut sink: Box<dyn Write> = match &args.out {
        Some(path) => Box::new(File::create(path).with_context(|| path.display().to_string())?),
        None if args.dry_run => Box::new(io::sink()),
        None => Box::new(io::stdout().lock()),
    };
    let per_second = u64::from(args.rate) * u64::from(total / args.frames);
    let step = Duration::from_nanos(1_000_000_000 / per_second);
    let start = Instant::now();
    let pace = |seq: u32| {
        if !args.dry_run {
            let due = start + step * seq;
            thread::sleep(due.saturating_duration_since(Instant::now()));
        }
    };
    let dry_run = args.dry_run;
    let mut shots = Shots::new(&args);
    feed(
        &args,
        &mut sink,
        |seq, panel, record| {
            if dry_run {
                let digest = Sha256::digest(record);
                let mut hex = String::new();
                for byte in digest {
                    write!(hex, "{byte:02x}")?;
                }
                println!("{seq} {} {} {hex}", panel.index(), record.len());
            }
            shots.as_mut().map_or(Ok(()), |shots| shots.observe(record))
        },
        pace,
    )?;
    if let Some(shots) = &mut shots {
        shots.finish()?;
        println!("{}", shots.report());
    }
    eprintln!("wrote {total} records");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use s4mk3_hid_midi_bridge::device::s8::screen::wire::read_record;

    use super::*;

    fn args(panels: Panels, frames: u32) -> Args {
        Args {
            panels,
            rate: 30,
            frames,
            dry_run: true,
            out: None,
            png: None,
            name: None,
            png_frames: Vec::new(),
            png_exit: false,
            png_check: false,
        }
    }

    #[test]
    fn records_parse_back_through_the_bridge_reader() -> anyhow::Result<()> {
        let mut stream = Vec::new();
        let mut seen = Vec::new();
        feed(
            &args(Panels::Alternate, 3),
            &mut stream,
            |seq, panel, _| {
                seen.push((seq, panel));
                Ok(())
            },
            |_| {},
        )?;
        assert_eq!(stream.len(), 6 * (HEADER_BYTES + PIXEL_BYTES));
        let mut reader = Cursor::new(stream);
        let mut pixels = pattern::pixel_buffer();
        let mut panels = Vec::new();
        while let Some(panel) = read_record(&mut reader, &mut pixels)? {
            panels.push(panel);
        }
        assert_eq!(
            panels,
            [
                Panel::Left,
                Panel::Right,
                Panel::Left,
                Panel::Right,
                Panel::Left,
                Panel::Right
            ]
        );
        assert_eq!(seen.len(), 6);
        Ok(())
    }

    #[test]
    fn the_last_record_carries_the_last_stamp() -> anyhow::Result<()> {
        let mut stream = Vec::new();
        feed(
            &args(Panels::Right, 4),
            &mut stream,
            |_, _, _| Ok(()),
            |_| {},
        )?;
        let mut expected = pattern::canvas(Panel::Right);
        pattern::stamp(&mut expected, 3);
        let mut bytes = pattern::pixel_buffer();
        pattern::wire(&expected, false, &mut bytes);
        let last = &stream[stream.len() - PIXEL_BYTES..];
        assert!(last == bytes.as_slice());
        Ok(())
    }

    #[test]
    fn pictures_are_named_per_panel_and_the_exit_frame_is_black() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("s8-feed-pictures-{}", std::process::id()));
        let mut options = args(Panels::Alternate, 3);
        options.png = Some(dir.clone());
        options.name = Some("t".to_owned());
        options.png_frames = vec![1];
        options.png_exit = true;
        let mut shots = Shots::new(&options).context("pictures requested")?;
        feed(
            &options,
            &mut io::sink(),
            |_, _, record| shots.observe(record),
            |_| {},
        )?;
        shots.finish()?;
        assert_eq!(shots.written, 3);
        let black = shots.buffers.rgb888(Panel::Left);
        assert!(black.iter().all(|byte| *byte == 0));
        for name in ["t-left-1", "t-right-1", "t"] {
            assert!(dir.join(format!("{name}.png")).exists(), "{name}");
        }
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn the_header_is_the_documented_layout() -> anyhow::Result<()> {
        assert_eq!(
            header(Panel::Right)?,
            [b'S', b'8', b'F', b'R', 1, 0, 0, 0, 0x00, 0xfc, 0x03, 0x00]
        );
        Ok(())
    }
}
