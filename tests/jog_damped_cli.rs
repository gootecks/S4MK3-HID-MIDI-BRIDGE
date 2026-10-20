//! Exercise the new opt-in surface without opening HID.
use std::process::Command;

#[test]
fn manual_neutral_preparation_parses_but_still_requires_motor_opt_in() -> anyhow::Result<()> {
    // Given the complete new manual/preparation pair without motor authorization.
    let output = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
        .args([
            "live",
            "--out",
            "/invalid/never-created-damping.jsonl",
            "--mode",
            "fader-damped",
            "--start-after-neutral",
            "--until-stopped",
        ])
        .output()?;
    // When it parses, then execution stops at the motor opt-in boundary, not HID.
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr)?;
    assert!(error.contains("--enable-motor"));
    assert!(!error.contains("DIAGNOSTIC_HID_OPEN"));
    Ok(())
}

#[test]
fn real_replay_suppresses_jitter_but_preserves_qualified_braking() -> anyhow::Result<()> {
    // Given the bundled report fixture and the actual standalone executable.
    for cap in [30000, 60000] {
        let output = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
            .args([
                "replay",
                "--input",
                "examples/jog-diagnostic-damped-replay.jsonl",
                "--mode",
                "fader-damped",
                "--max-force",
                &cap.to_string(),
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()?;
        // When it replays, then machine-consumed control, force and config agree.
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let rows = String::from_utf8(output.stdout)?
            .lines()
            .map(serde_json::from_str::<serde_json::Value>)
            .collect::<Result<Vec<_>, _>>()?;
        let motions: Vec<_> = rows.iter().filter(|r| r["kind"] == "motion").collect();
        assert_eq!(rows[0]["control"]["assist_force"], 0);
        assert_eq!(rows[1]["kind"], "damping");
        assert_eq!(rows[1]["qualification_us"], 6000);
        assert!(
            motions[..8]
                .iter()
                .all(|r| r["decision"]["torque"] == serde_json::json!([0, 0]))
        );
        assert!(motions.iter().any(|r| r["decision"]["torque"][0] == -cap));
        assert!(motions.iter().any(|r| r["decision"]["torque"][0] == cap));
        assert!(
            motions
                .iter()
                .filter(|r| r["motion"]["deck1_fader"] == 0)
                .all(|r| r["decision"]["torque"] == serde_json::json!([0, 0]))
        );
    }
    Ok(())
}

#[test]
fn invalid_damped_controls_and_legacy_manual_preparation_fail_before_hid() -> anyhow::Result<()> {
    // Given invalid combinations with an impossible destination.
    for extra in [
        &["--mode", "fader-damped", "--side", "right"][..],
        &["--mode", "fader-damped", "--gain", "1"][..],
        &["--mode", "fader-damped", "--assist-force", "1"][..],
        &["--mode", "fader-damped", "--initialize-rings"][..],
        &["--mode", "fader-damped", "--max-force", "60001"][..],
        &[
            "--mode",
            "fader-damped",
            "--until-stopped",
            "--seconds",
            "30",
        ][..],
        &[
            "--mode",
            "fader-tension",
            "--until-stopped",
            "--start-after-neutral",
        ][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
            .args([
                "live",
                "--out",
                "/invalid/never-created-damping.jsonl",
                "--enable-motor",
            ])
            .args(extra)
            .output()?;
        // When parsing/validation runs, then hardware/file opening is unreachable.
        assert!(!output.status.success());
        let error = String::from_utf8(output.stderr)?;
        assert!(!error.contains("DIAGNOSTIC_HID_OPEN"));
        assert!(!error.contains("create new capture"));
    }
    Ok(())
}
