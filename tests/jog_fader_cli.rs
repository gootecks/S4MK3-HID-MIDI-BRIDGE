//! Actual fader diagnostic executable checks; none opens HID.

use std::process::{Command, Output};

use anyhow::Context as _;
use serde_json::{Value, json};

fn run(args: &[&str]) -> anyhow::Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?)
}

#[test]
fn fader_replay_arms_scales_and_brakes_only_the_left_wheel() -> anyhow::Result<()> {
    // Given captured mixer snapshots, including independent neighboring controls.
    for (cap, expected, full_packet) in [
        (
            None,
            json!([0, 0, 0, -2500, -2500, -5000, 5000, 0, 0, 0, 0]),
            json!([49, 1, 224, 254, 136, 19, 1, 32, 1, 0, 0]),
        ),
        (
            Some("25000"),
            json!([0, 0, 0, -12503, -12503, -25000, 25000, 0, 0, 0, 0]),
            json!([49, 1, 224, 254, 168, 97, 1, 32, 1, 0, 0]),
        ),
    ] {
        let mut args = vec![
            "replay",
            "--input",
            "examples/jog-diagnostic-fader-replay.jsonl",
            "--mode",
            "fader-brake",
        ];
        if let Some(cap) = cap {
            args.extend_from_slice(&["--max-force", cap]);
        }
        // When the actual binary replays the requested direct-force profile.
        let output = run(&args)?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let rows: Vec<Value> = std::str::from_utf8(&output.stdout)?
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()?;
        let motions: Vec<_> = rows.iter().filter(|row| row["kind"] == "motion").collect();
        // Then both default and full-width cap give the specified physical sequence.
        assert_eq!(
            json!(
                motions
                    .iter()
                    .map(|r| &r["decision"]["torque"][0])
                    .collect::<Vec<_>>()
            ),
            expected
        );
        assert!(motions.iter().all(|r| r["decision"]["torque"][1] == 0));
        assert_eq!(
            json!(
                motions
                    .iter()
                    .map(|r| &r["decision"]["fader"]["armed"])
                    .collect::<Vec<_>>()
            ),
            json!([
                false, false, true, true, true, true, true, true, true, true, true
            ])
        );
        assert_eq!(motions[5]["decision"]["report"], full_packet);
        assert_eq!(motions[6]["motion"]["touch"], json!([false, false]));
        assert_eq!(rows[0]["control"]["mode"], "fader_brake");
        assert_eq!(
            rows[0]["control"]["max_force"],
            if cap.is_some() { 25000 } else { 5000 }
        );
        assert_eq!(
            rows.last().context("missing end")?["neutral_report"],
            json!([49, 1, 32, 1, 0, 0, 1, 32, 1, 0, 0])
        );
        assert!(!String::from_utf8_lossy(&output.stderr).contains("DIAGNOSTIC_HID_OPEN"));
    }
    Ok(())
}

#[test]
fn fader_cli_rejects_bad_analog_and_incompatible_live_flags() -> anyhow::Result<()> {
    // Given missing, short, wrong-ID and out-of-range raw analog evidence.
    for fixture in ["missing", "short", "wrong-id", "range"] {
        let path = format!("tests/fixtures/jog-diagnostic-fader-{fixture}.jsonl");
        // When the actual offline executable parses each malformed fixture.
        let output = run(&["replay", "--input", &path, "--mode", "fader-brake"])?;
        // Then it rejects the input without emitting a motion or touching HID.
        assert!(!output.status.success());
        for line in std::str::from_utf8(&output.stdout)?.lines() {
            assert_ne!(serde_json::from_str::<Value>(line)?["kind"], "motion");
        }
        assert!(!String::from_utf8_lossy(&output.stderr).contains("DIAGNOSTIC_HID_OPEN"));
    }
    for extra in [
        &["--side", "right"][..],
        &["--gain", "1"][..],
        &["--max-force", "25001"][..],
        &["--until-stopped", "--seconds", "10"][..],
    ] {
        let mut args = vec![
            "live",
            "--out",
            "/invalid/never-created-fader.jsonl",
            "--mode",
            "fader-brake",
            "--enable-motor",
        ];
        args.extend_from_slice(extra);
        let output = run(&args)?;
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(extra[0]), "{error}");
        assert!(!error.contains("DIAGNOSTIC_HID_OPEN"));
    }
    Ok(())
}
