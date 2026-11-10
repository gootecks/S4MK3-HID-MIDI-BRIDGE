//! The `--s8-screens-stdin` channel: frame records in on stdin, status lines
//! out on stdout. See `docs/s8/screen-transport.md`.

use std::io;
use std::thread;

use anyhow::ensure;
use crossbeam_channel::Sender;
use s4mk3_hid_midi_bridge::device::Device;
use s4mk3_hid_midi_bridge::device::s8::screen::usb::Usb;
use s4mk3_hid_midi_bridge::device::s8::screen::wire;
use s4mk3_hid_midi_bridge::device::s8::screen::worker::{Config, Worker};

use crate::Args;

/// Refuse the flag for any controller but the S8.
pub fn check_device(args: &Args, device: Device) -> anyhow::Result<()> {
    ensure!(
        device == Device::S8 || !args.screens.s8_stdin,
        "--s8-screens-stdin applies to the Kontrol S8 only"
    );
    Ok(())
}

/// Start the worker and the stdin reader. Dropping the returned worker sends
/// the exit frames, releases the interface and reports the final status.
/// Closing stdin asks the bridge to stop through `stop`.
pub fn start(stop: Sender<()>) -> anyhow::Result<Worker> {
    let worker = Worker::start(Usb::default(), Config::default(), |status| {
        println!("S8_SCREEN_STATUS {status}");
    })?;
    let handle = worker.handle();
    thread::Builder::new()
        .name("s8-screen-stdin".to_owned())
        .spawn(move || {
            wire::serve(io::stdin().lock(), &handle);
            if let Err(error) = stop.try_send(()) {
                tracing::debug!(%error, "S8 screen stdin stop already requested");
            }
        })?;
    Ok(worker)
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    use super::*;

    fn parse(extra: &[&str]) -> anyhow::Result<Args> {
        let mut argv = vec!["bridge", "--midi", "full"];
        argv.extend_from_slice(extra);
        Ok(Args::try_parse_from(argv)?)
    }

    #[test]
    fn the_flag_needs_the_s8() -> anyhow::Result<()> {
        let args = parse(&["--s8-screens-stdin"])?;
        assert!(check_device(&args, Device::S8).is_ok());
        let error = check_device(&args, Device::S4Mk3)
            .err()
            .map(|e| e.to_string());
        assert_eq!(
            error.as_deref(),
            Some("--s8-screens-stdin applies to the Kontrol S8 only")
        );
        assert!(check_device(&parse(&[])?, Device::S4Mk3).is_ok());
        Ok(())
    }

    #[test]
    fn the_flag_needs_midi() {
        assert!(Args::try_parse_from(["bridge", "--s8-screens-stdin"]).is_err());
    }

    #[test]
    fn the_s8_accepts_the_flag_and_still_refuses_the_s4_screen_flags() -> anyhow::Result<()> {
        use crate::s8::bridge::check_args;
        assert!(check_args(&parse(&["--s8-screens-stdin"])?).is_ok());
        assert!(check_args(&parse(&[])?).is_ok());
        for flag in ["--screens", "--screen-control-stdin"] {
            assert!(check_args(&parse(&[flag])?).is_err(), "{flag}");
        }
        Ok(())
    }
}
