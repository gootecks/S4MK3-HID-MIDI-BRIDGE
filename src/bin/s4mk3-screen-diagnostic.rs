//! Static display probe: no HID, MIDI, audio, or motor commands.

use std::path::PathBuf;

use anyhow::ensure;
use clap::Parser;
use s4mk3_hid_midi_bridge::screens::{Display, UsbScreens, test_pattern};

#[path = "../jog_diagnostic/ownership.rs"]
mod ownership;

#[derive(Parser)]
#[command(about = "Render S4 screen test patterns, optionally writing interface 4 only")]
struct Args {
    /// Send the test images to the connected controller.
    #[arg(long)]
    live: bool,
    /// Save exact raster previews without opening USB unless --live is supplied.
    #[arg(long)]
    output_dir: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    ensure!(
        args.live || args.output_dir.is_some(),
        "choose --live or --output-dir PATH"
    );
    let mut usb = if args.live {
        ownership::check()?;
        Some(UsbScreens::open()?)
    } else {
        None
    };
    if let Some(path) = &args.output_dir {
        std::fs::create_dir_all(path)?;
    }
    for (display, name) in [(Display::Left, "left"), (Display::Right, "right")] {
        let frame = test_pattern(display);
        if let Some(path) = &args.output_dir {
            frame.write_ppm(&path.join(format!("{name}.ppm")))?;
        }
        if let Some(usb) = &mut usb {
            usb.write(display, &frame)?;
            println!("SCREEN_TRANSFER_OK display={name} interface=4 endpoint=0x03 bytes=153620");
        }
    }
    Ok(())
}
