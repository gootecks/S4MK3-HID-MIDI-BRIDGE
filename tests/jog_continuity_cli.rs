//! Real executable checks for fader-continuity replay; no HID access.

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
        "examples/jog-diagnostic-continuity-replay.jsonl",
        "--mode",
        "fader-continuity",
    ];
    args.extend_from_slice(extra);
    run(&args)
}

fn records(output: &Output) -> anyhow::Result<Vec<Value>> {
    Ok(std::str::from_utf8(&output.stdout)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}

#[test]
fn replay_ticks_fade_motion_and_clear_assistance_on_release() -> anyhow::Result<()> {
    let output = replay(&[])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows = records(&output)?;
    let motions: Vec<_> = rows.iter().filter(|row| row["kind"] == "motion").collect();
    let ticks: Vec<_> = rows.iter().filter(|row| row["kind"] == "tick").collect();
    assert_eq!(motions.len(), 16);
    assert!(motions.iter().all(|row| row["decision"]["torque"][1] == 0));
    assert_eq!(motions[1]["motion"]["delta"][0], 10);
    assert_eq!(motions[1]["decision"]["torque"][0], 8000);
    assert_eq!(motions[2]["motion"]["status"][0], "duplicate");
    assert_eq!(motions[2]["decision"]["torque"][0], 6000);
    assert_eq!(motions[5]["motion"]["touch"], json!([true, false]));
    assert_eq!(motions[5]["decision"]["torque"], json!([0, 0]));
    assert_eq!(motions[6]["decision"]["torque"], json!([-8000, 0]));
    assert_eq!(motions[7]["motion"]["touch"], json!([false, false]));
    assert_eq!(motions[7]["decision"]["torque"], json!([0, 0]));
    assert_eq!(motions[8]["decision"]["torque"], json!([0, 0]));
    assert_eq!(motions[11]["decision"]["torque"], json!([-30000, 0]));
    assert_eq!(motions[13]["decision"]["torque"], json!([0, 0]));

    let timed: Vec<_> = ticks
        .iter()
        .map(|row| -> anyhow::Result<_> {
            Ok((
                row["host_us"].as_u64().context("tick time")?,
                row["decision"]["torque"][0]
                    .as_i64()
                    .context("tick torque")?,
            ))
        })
        .collect::<anyhow::Result<_>>()?;
    assert!(timed.windows(2).all(|pair| pair[1].0 - pair[0].0 == 2000));
    assert_eq!(timed[0], (2000, 8000));
    assert_eq!(timed[1], (4000, 6000));
    assert_eq!(timed[2], (6000, 4000));
    assert_eq!(timed[3], (8000, 2000));
    assert_eq!(timed[4], (10000, 0));
    assert_eq!(
        rows.last().context("end record missing")?["summary"]["samples"],
        16
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("DIAGNOSTIC_HID_OPEN"));
    Ok(())
}

#[test]
fn continuity_rejects_gain_right_side_and_offline_ring_initialization() -> anyhow::Result<()> {
    for args in [
        &[
            "replay",
            "--input",
            "examples/jog-diagnostic-continuity-replay.jsonl",
            "--mode",
            "fader-continuity",
            "--gain",
            "1",
        ][..],
        &[
            "replay",
            "--input",
            "examples/jog-diagnostic-continuity-replay.jsonl",
            "--mode",
            "fader-continuity",
            "--side",
            "right",
        ][..],
        &[
            "replay",
            "--input",
            "examples/jog-diagnostic-continuity-replay.jsonl",
            "--mode",
            "fader-continuity",
            "--initialize-rings",
        ][..],
        &[
            "capture",
            "--out",
            "/invalid/no-hardware-continuity.jsonl",
            "--initialize-rings",
        ][..],
        &[
            "live",
            "--out",
            "/invalid/no-hardware-continuity.jsonl",
            "--mode",
            "fader-continuity",
            "--initialize-rings",
        ][..],
    ] {
        let output = run(args)?;
        assert!(!output.status.success());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("DIAGNOSTIC_HID_OPEN"));
    }
    Ok(())
}
