//! PNG files of decoded S8 panels, shared by the screen example tools: one is
//! written from, or compared with, the 8-bit RGB bytes of a `Panels` buffer.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use anyhow::{Context as _, bail};
use s4mk3_hid_midi_bridge::device::s8::screen::{HEIGHT, WIDTH};

fn size() -> anyhow::Result<(u32, u32)> {
    Ok((u32::try_from(WIDTH)?, u32::try_from(HEIGHT)?))
}

pub fn write(path: &Path, rgb: &[u8]) -> anyhow::Result<()> {
    let (width, height) = size()?;
    let file = File::create(path).with_context(|| format!("write {}", path.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgb)?;
    Ok(())
}

pub fn compare(path: &Path, rgb: &[u8]) -> anyhow::Result<()> {
    let file = File::open(path).with_context(|| format!("missing picture {}", path.display()))?;
    let mut reader = png::Decoder::new(BufReader::new(file)).read_info()?;
    let mut stored = vec![0; reader.output_buffer_size().unwrap_or(rgb.len())];
    let info = reader.next_frame(&mut stored)?;
    let (width, height) = size()?;
    if (info.width, info.height) != (width, height)
        || info.color_type != png::ColorType::Rgb
        || stored[..info.buffer_size()] != *rgb
    {
        bail!("{} differs from what the tool produces now", path.display());
    }
    Ok(())
}
