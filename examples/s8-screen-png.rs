//! Draws what the S8 panels show after each `*.s8fr` record file, from the
//! bytes the bridge would send.
//!
//! `s8-screen-png [--check] --out DIR FILE...` never opens a device. The files
//! hold `S8FR` records as the bridge reads them with `--s8-screens-stdin`
//! (`wire::read_record`); each record becomes the frame `screen::full` builds
//! and is applied to one decoded framebuffer (`Panels`) that carries over from
//! file to file, in argument order. After each file both panels are written
//! as `DIR/<file stem>-{left|right}.png`; a panel without a record is black.
//! `--check` writes nothing and compares with the stored pictures instead.

#[path = "common/picture_file.rs"]
mod picture_file;

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use clap::Parser;
use s4mk3_hid_midi_bridge::device::s8::screen::decode::Panels;
use s4mk3_hid_midi_bridge::device::s8::screen::wire::read_record;
use s4mk3_hid_midi_bridge::device::s8::screen::{self, Panel, pattern};

#[derive(Parser)]
#[command(about = "Draw PNG pictures of the S8 panels from S8FR record files")]
struct Args {
    /// Compare with the stored pictures instead of writing them
    #[arg(long)]
    check: bool,

    /// Directory for the pictures
    #[arg(long, value_name = "DIR")]
    out: PathBuf,

    /// Record files, applied in this order
    #[arg(required = true, value_name = "FILE")]
    files: Vec<PathBuf>,
}

const fn side(panel: Panel) -> &'static str {
    match panel {
        Panel::Left => "left",
        Panel::Right => "right",
    }
}

/// Applies every record of `file` to `panels`; returns the records per panel.
fn apply_file(panels: &mut Panels, file: &Path) -> anyhow::Result<[u32; 2]> {
    let mut reader = BufReader::new(File::open(file).with_context(|| file.display().to_string())?);
    let mut pixels = pattern::pixel_buffer();
    let mut frame = Vec::with_capacity(screen::FRAME_BYTES);
    let mut counts = [0; 2];
    while let Some(panel) = read_record(&mut reader, &mut pixels)
        .with_context(|| format!("{}: bad record", file.display()))?
    {
        screen::full(&mut frame, panel, &pixels);
        if panels.apply(&frame)? != panel {
            bail!("frame addressed to another panel than the record");
        }
        counts[usize::from(panel.index())] += 1;
    }
    Ok(counts)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if !args.check {
        fs::create_dir_all(&args.out).with_context(|| args.out.display().to_string())?;
    }
    let mut panels = Panels::default();
    for file in &args.files {
        let stem = file
            .file_stem()
            .and_then(OsStr::to_str)
            .with_context(|| format!("{}: no usable file name", file.display()))?;
        let counts = apply_file(&mut panels, file)?;
        for panel in [Panel::Left, Panel::Right] {
            let path = args.out.join(format!("{stem}-{}.png", side(panel)));
            let rgb = panels.rgb888(panel);
            if args.check {
                picture_file::compare(&path, &rgb)?;
            } else {
                picture_file::write(&path, &rgb)?;
            }
        }
        println!(
            "{stem}: left {} record(s), right {} record(s)",
            counts[0], counts[1]
        );
    }
    Ok(())
}
