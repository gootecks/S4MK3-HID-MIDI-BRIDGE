//! Real executable checks, entirely offline and without synthetic HID access.

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
fn replay_decodes_release_and_rest_with_opposing_or_assisting_torque() -> anyhow::Result<()> {
    // Given a synthetic capture spanning two counters, release, reversal, and rest.
    for (mode, side, expected) in [
        ("brake", "left", json!([[-5, 0], [-10, 0], [10, 0]])),
        ("assist", "right", json!([[0, -5], [0, -10], [0, 10]])),
    ] {
        // When the actual binary replays it with a deliberately small cap.
        let output = run(&[
            "replay",
            "--input",
            "examples/jog-diagnostic-replay.jsonl",
            "--mode",
            mode,
            "--side",
            side,
            "--gain",
            "1",
            "--max-force",
            "10",
        ])?;
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
        // Then raw movement survives release, caps are applied, and rest is neutral.
        assert_eq!(motions.len(), 6);
        assert_eq!(
            json!([
                motions[1]["decision"]["torque"],
                motions[2]["decision"]["torque"],
                motions[3]["decision"]["torque"]
            ]),
            expected
        );
        assert_eq!(motions[2]["motion"]["touch"], json!([false, false]));
        assert_eq!(motions[4]["decision"]["torque"], json!([0, 0]));
        assert_eq!(motions[5]["decision"]["torque"], json!([0, 0]));
        let end = rows.last().context("end record missing")?;
        assert_eq!(end["summary"]["net_counts"], json!([60, -60]));
        assert_eq!(end["summary"]["released_counts"], json!([50, -50]));
        assert_eq!(
            end["neutral_report"],
            json!([49, 1, 32, 1, 0, 0, 1, 32, 1, 0, 0])
        );
    }
    Ok(())
}

#[test]
fn malformed_capture_and_unarmed_live_command_exit_nonzero() -> anyhow::Result<()> {
    // Given malformed input and a live invocation lacking motor authorization.
    for args in [
        &[
            "replay",
            "--input",
            "tests/fixtures/jog-diagnostic-malformed.jsonl",
            "--mode",
            "brake",
        ][..],
        &[
            "live",
            "--out",
            "/invalid/no-hardware.jsonl",
            "--mode",
            "assist",
            "--seconds",
            "5",
        ][..],
    ] {
        // When the real executable handles these commands.
        let output = run(args)?;
        // Then it fails without initializing or writing HID.
        assert!(!output.status.success());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("DIAGNOSTIC_HID_OPEN"));
    }
    Ok(())
}
