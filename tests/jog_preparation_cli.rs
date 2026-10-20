//! Invalid live preparation flags fail before native HID is opened.

use std::process::{Command, Output};

fn run(args: &[&str]) -> anyhow::Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?)
}

#[test]
fn invalid_preparation_flags_fail_before_hid_open() -> anyhow::Result<()> {
    for extra in [
        &["--mode", "assist"][..],
        &["--mode", "fader-brake"][..],
        &["--mode", "fader-tension", "--until-stopped"][..],
        &["--mode", "fader-continuity", "--seconds", "0"][..],
        &["--mode", "fader-tension", "--seconds", "31"][..],
    ] {
        let mut args = vec![
            "live",
            "--out",
            "/invalid/never-created-preparation.jsonl",
            "--enable-motor",
            "--start-after-neutral",
        ];
        args.extend_from_slice(extra);

        let output = run(&args)?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{stderr}");
        assert!(
            stderr.contains("--start-after-neutral")
                || stderr.contains("--until-stopped")
                || stderr.contains("--seconds"),
            "{stderr}"
        );
        assert!(!stderr.contains("DIAGNOSTIC_HID_OPEN"), "{stderr}");
    }
    Ok(())
}

#[test]
fn help_exposes_preparation_flag_without_starting_live_trial() -> anyhow::Result<()> {
    let output = run(&["live", "--help"])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(std::str::from_utf8(&output.stdout)?.contains("--start-after-neutral"));
    Ok(())
}
