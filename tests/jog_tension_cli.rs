//! Real executable checks for fader-tension replay; no mocks or HID access.

use std::process::{Command, Output};

use anyhow::Context as _;
use serde_json::{Value, json};

fn run(args: &[&str]) -> anyhow::Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?)
}

fn replay(extra: &[&str]) -> anyhow::Result<Output> {
    let mut args = vec![
        "replay",
        "--input",
        "examples/jog-diagnostic-tension-replay.jsonl",
        "--mode",
        "fader-tension",
    ];
    args.extend_from_slice(extra);
    run(&args)
}

fn rows(output: &Output) -> anyhow::Result<Vec<Value>> {
    Ok(std::str::from_utf8(&output.stdout)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}

#[test]
fn default_replay_emits_the_full_tension_trace() -> anyhow::Result<()> {
    // Given valid numbered reports with threshold edges, touch changes and rest.
    let output = replay(&[])?;

    // When the actual diagnostic executable replays in default tension mode.
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let records = rows(&output)?;
    let motions: Vec<_> = records
        .iter()
        .filter(|record| record["kind"] == "motion")
        .collect();

    // Then arming, zones, selected forces, and torque match the physical contract.
    assert_eq!(motions.len(), 16);
    assert_eq!(
        json!(
            motions
                .iter()
                .map(|r| r["motion"]["deck1_fader"].clone())
                .collect::<Vec<_>>()
        ),
        json!([
            4095, 2048, 0, 1008, 1008, 1008, 2015, 2016, 2080, 2081, 4095, 4095, 4095, 3000, 3000,
            2016
        ])
    );
    assert_eq!(
        json!(
            motions
                .iter()
                .map(|r| r["decision"]["fader"]["armed"].clone())
                .collect::<Vec<_>>()
        ),
        json!([
            false, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true
        ])
    );
    assert_eq!(
        json!(
            motions
                .iter()
                .map(|r| r["decision"]["fader"]["zone"].clone())
                .collect::<Vec<_>>()
        ),
        json!([
            "brake", "neutral", "assist", "assist", "assist", "assist", "assist", "neutral",
            "neutral", "brake", "brake", "brake", "brake", "brake", "brake", "neutral"
        ])
    );
    assert_eq!(
        json!(
            motions
                .iter()
                .map(|r| r["decision"]["fader"]["selected_force"].clone())
                .collect::<Vec<_>>()
        ),
        json!([
            30000, 0, 8000, 4000, 4000, 4000, 3, 0, 0, 14, 30000, 30000, 30000, 13697, 13697, 0
        ])
    );
    assert_eq!(
        json!(
            motions
                .iter()
                .map(|r| r["decision"]["torque"][0].clone())
                .collect::<Vec<_>>()
        ),
        json!([
            0, 0, 8000, -4000, 0, 4000, 3, 0, 0, -14, -30000, 30000, -30000, 0, 0, 0
        ])
    );
    assert!(
        motions
            .iter()
            .all(|record| record["decision"]["torque"][1] == 0)
    );
    assert_eq!(motions[4]["motion"]["touch"], json!([false, false]));
    assert_eq!(motions[12]["motion"]["touch"], json!([false, false]));
    assert_eq!(motions[13]["motion"]["status"][0], "rest");
    assert_eq!(motions[14]["motion"]["status"][0], "duplicate");
    assert_eq!(motions[15]["motion"]["delta"][1], 10);
    assert_eq!(motions[15]["motion"]["deck1_fader"], 2016);
    assert_eq!(records[0]["control"]["assist_force"], 8000);
    assert_eq!(records[0]["control"]["max_force"], 30000);
    assert_eq!(
        records.last().context("end record missing")?["summary"]["samples"],
        16
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("DIAGNOSTIC_HID_OPEN"));
    Ok(())
}

#[test]
fn explicit_force_limits_are_used_by_the_real_executable() -> anyhow::Result<()> {
    // Given the settled fixture and distinct override values.
    for (flags, brake, assist) in [
        (&["--max-force", "60000"][..], 60000, 8000),
        (&["--assist-force", "0"][..], 30000, 0),
    ] {
        // When the real binary replays with one explicit tension ceiling.
        let output = replay(flags)?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let records = rows(&output)?;
        let motions: Vec<_> = records
            .iter()
            .filter(|record| record["kind"] == "motion")
            .collect();

        // Then the chosen value changes only its associated zone.
        assert_eq!(records[0]["control"]["max_force"], brake);
        assert_eq!(records[0]["control"]["assist_force"], assist);
        assert_eq!(
            motions[9]["decision"]["fader"]["selected_force"],
            brake / 2015
        );
        assert_eq!(motions[10]["decision"]["torque"][0], -brake);
        assert_eq!(motions[2]["decision"]["torque"][0], assist);
        assert_eq!(motions[3]["decision"]["torque"][0], -(assist / 2));
    }
    Ok(())
}

#[test]
fn tension_cli_rejects_invalid_options_and_unarmed_live_without_hid() -> anyhow::Result<()> {
    // Given incompatible options, range violations, and live without opt-in.
    for args in [
        &[
            "replay",
            "--input",
            "examples/jog-diagnostic-tension-replay.jsonl",
            "--mode",
            "fader-tension",
            "--gain",
            "1",
        ][..],
        &[
            "replay",
            "--input",
            "examples/jog-diagnostic-tension-replay.jsonl",
            "--mode",
            "fader-tension",
            "--side",
            "right",
        ][..],
        &[
            "replay",
            "--input",
            "examples/jog-diagnostic-tension-replay.jsonl",
            "--mode",
            "fader-brake",
            "--assist-force",
            "1",
        ][..],
        &[
            "replay",
            "--input",
            "examples/jog-diagnostic-tension-replay.jsonl",
            "--mode",
            "baseline",
            "--max-force",
            "60000",
        ][..],
        &[
            "replay",
            "--input",
            "examples/jog-diagnostic-tension-replay.jsonl",
            "--mode",
            "fader-tension",
            "--max-force",
            "0",
        ][..],
        &[
            "replay",
            "--input",
            "examples/jog-diagnostic-tension-replay.jsonl",
            "--mode",
            "fader-tension",
            "--max-force",
            "60001",
        ][..],
        &[
            "replay",
            "--input",
            "examples/jog-diagnostic-tension-replay.jsonl",
            "--mode",
            "fader-tension",
            "--assist-force",
            "8001",
        ][..],
        &[
            "live",
            "--out",
            "/invalid/no-hardware-tension.jsonl",
            "--mode",
            "fader-tension",
            "--seconds",
            "5",
        ][..],
    ] {
        // When the actual executable receives each invalid invocation.
        let output = run(args)?;

        // Then it rejects the command without attempting to open diagnostic HID.
        assert!(!output.status.success());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("DIAGNOSTIC_HID_OPEN"));
    }
    Ok(())
}

#[test]
fn malformed_existing_fixtures_stay_rejected_in_tension_mode() -> anyhow::Result<()> {
    // Given existing malformed raw and analog evidence fixtures.
    for input in [
        "tests/fixtures/jog-diagnostic-malformed.jsonl",
        "tests/fixtures/jog-diagnostic-fader-missing.jsonl",
        "tests/fixtures/jog-diagnostic-fader-short.jsonl",
        "tests/fixtures/jog-diagnostic-fader-wrong-id.jsonl",
        "tests/fixtures/jog-diagnostic-fader-range.jsonl",
    ] {
        // When the real offline executable replays each fixture in tension mode.
        let output = run(&["replay", "--input", input, "--mode", "fader-tension"])?;

        // Then malformed evidence is rejected and no motion is emitted.
        assert!(!output.status.success(), "{input}");
        for line in std::str::from_utf8(&output.stdout)?.lines() {
            assert_ne!(serde_json::from_str::<Value>(line)?["kind"], "motion");
        }
        assert!(!String::from_utf8_lossy(&output.stderr).contains("DIAGNOSTIC_HID_OPEN"));
    }
    Ok(())
}
