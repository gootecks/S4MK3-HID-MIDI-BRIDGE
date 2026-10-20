use std::process::Command;

#[test]
fn free_play_cli_replays_assistance_and_braking_in_one_capture() -> anyhow::Result<()> {
    // Given a fixture with lower, center and upper selections.
    let output = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
        .args([
            "replay",
            "--input",
            "examples/jog-diagnostic-assist-replay.jsonl",
            "--mode",
            "fader-play",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    // When the real command processes both halves in one session.
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows = String::from_utf8(output.stdout)?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(rows[0]["control"]["assist_force"], 2000);
    assert_eq!(rows[0]["control"]["max_force"], 30000);
    let ticks: Vec<_> = rows.iter().filter(|r| r["kind"] == "tick").collect();
    // Then lower assists and upper brakes, with independent caps and right zero.
    assert!(
        ticks
            .iter()
            .any(|r| r["decision"]["fader"]["zone"] == "assist"
                && r["decision"]["torque"][0].as_i64().is_some_and(|v| v > 0))
    );
    assert!(
        ticks
            .iter()
            .any(|r| r["decision"]["fader"]["zone"] == "brake"
                && r["decision"]["torque"][0].as_i64().is_some_and(|v| v < 0))
    );
    for r in ticks {
        assert_eq!(r["decision"]["torque"][1], 0);
        let force = r["decision"]["torque"][0]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("force missing"))?;
        let cap = r["decision"]["fader"]["selected_force"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("cap missing"))?;
        assert!(force.unsigned_abs() <= cap);
        if r["decision"]["fader"]["zone"] == "neutral" {
            assert_eq!(force, 0);
        }
    }
    Ok(())
}

#[test]
fn free_play_invalid_controls_and_missing_opt_in_stop_before_hid() -> anyhow::Result<()> {
    // Given incompatible controls and valid manual flags without motor opt-in.
    for flags in [
        &["--enable-motor", "--gain", "1"][..],
        &["--enable-motor", "--assist-force", "8001"][..],
        &["--enable-motor", "--max-force", "60001"][..],
        &["--start-after-neutral", "--until-stopped"][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
            .args([
                "live",
                "--out",
                "/dev/null/never-created-play.jsonl",
                "--mode",
                "fader-play",
            ])
            .args(flags)
            .output()?;
        // When CLI validation runs, then it fails before file creation or HID.
        assert!(!output.status.success());
        let error = String::from_utf8(output.stderr)?;
        assert!(!error.contains("create new capture"));
        assert!(!error.contains("DIAGNOSTIC_HID_OPEN"));
    }
    Ok(())
}

#[test]
fn explicit_free_play_startup_reaches_log_creation_before_hid() -> anyhow::Result<()> {
    // Given the explicit startup comparison and valid unbounded preparation.
    let output = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
        .args([
            "live",
            "--out",
            "/dev/null/never-created-play-startup.jsonl",
            "--mode",
            "fader-play",
            "--max-force",
            "30000",
            "--assist-force",
            "7000",
            "--initialize-rings",
            "--start-after-neutral",
            "--until-stopped",
            "--enable-motor",
        ])
        .output()?;
    // When the actual CLI runs, its deliberate invalid path stops before HID.
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr)?;
    assert!(error.contains("create new capture"));
    assert!(!error.contains("DIAGNOSTIC_HID_OPEN"));
    Ok(())
}
