//! Managed stdin belongs to a separate blocking reader, not the HID service.

use std::io::{self, BufRead as _};

use crossbeam_channel::Sender;
use s4mk3_hid_midi_bridge::screens::{Control, ControlAction};

fn parse(line: &str) -> Option<(u64, ControlAction)> {
    let mut parts = line.split(' ');
    let ("S4_SCREEN_CONTROL", Some(id), Some(action), None) =
        (parts.next()?, parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let action = match action {
        "ON" => ControlAction::On,
        "OFF" => ControlAction::Off,
        "RETRY" => ControlAction::Retry,
        _ => return None,
    };
    Some((id.parse().ok()?, action))
}

fn serve(reader: impl io::Read, control: &Control, stop: &Sender<()>) -> io::Result<()> {
    for line in io::BufReader::new(reader).lines() {
        let line = line?;
        if let Some((id, action)) = parse(&line) {
            control.request(id, action);
        } else {
            tracing::warn!("invalid S4 screen control command");
        }
    }
    // Closing the app's stdin follows the same stop path as Ctrl-C, including
    // motor neutral and MIDI release. An idle stdin read may block until process
    // exit, so this reader is detached rather than joined during normal Stop.
    if let Err(error) = stop.try_send(()) {
        tracing::debug!(%error, "managed stdin stop already requested");
    }
    Ok(())
}

pub fn start(control: Control, stop: Sender<()>) -> anyhow::Result<()> {
    std::thread::Builder::new()
        .name("s4-screen-stdin".to_owned())
        .spawn(move || {
            if let Err(error) = serve(io::stdin(), &control, &stop) {
                tracing::warn!(%error, "managed stdin failed; stopping bridge");
                if let Err(error) = stop.try_send(()) {
                    tracing::debug!(%error, "managed stdin stop already requested");
                }
            }
        })?;
    Ok(())
}

pub fn bridge(args: &crate::Args, stop: Sender<()>) -> anyhow::Result<crate::full::FullBridge> {
    let bridge = crate::full::FullBridge::new(args.screens.enabled, args.screens.control_stdin)?;
    if let Some(control) = bridge.screen_control() {
        start(control, stop)?;
    }
    Ok(bridge)
}

#[cfg(test)]
mod tests {
    use super::*;
    use s4mk3_hid_midi_bridge::screens::Worker;

    #[test]
    fn decoder_accepts_only_exact_machine_commands() {
        assert_eq!(
            parse("S4_SCREEN_CONTROL 18446744073709551615 RETRY"),
            Some((u64::MAX, ControlAction::Retry))
        );
        for input in [
            "S4_SCREEN_CONTROL -1 ON",
            "S4_SCREEN_CONTROL +1 ON",
            "S4_SCREEN_CONTROL 1 on",
            "S4_SCREEN_CONTROL 1 OFF extra",
            " S4_SCREEN_CONTROL 1 ON",
            "S4_SCREEN_CONTROL 1  ON",
            "S4_SCREEN_CONTROL 18446744073709551616 OFF",
        ] {
            assert!(parse(input).is_none(), "{input}");
        }
    }

    #[test]
    fn managed_stdin_eof_uses_normal_stop_channel() -> anyhow::Result<()> {
        // Given a managed screen worker and a subscribed stop receiver.
        let worker = Worker::start_managed(false)?;
        let (stop_tx, stop_rx) = crossbeam_channel::bounded(1);
        // When the app's pipe reaches EOF without any screen commands.
        serve(io::empty(), &worker.control(), &stop_tx)?;
        // Then the existing runtime stop channel receives the shutdown event.
        assert!(stop_rx.try_recv().is_ok());
        Ok(())
    }
}
