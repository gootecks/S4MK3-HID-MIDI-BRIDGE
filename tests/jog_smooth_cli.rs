use std::process::Command;

#[test]
fn actual_replay_advances_smoothing_on_equal_input_and_output_times() -> anyhow::Result<()> {
    // Given the shared valid fixture with all input times on the output grid.
    let result = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
        .args([
            "replay",
            "--input",
            "examples/jog-diagnostic-damped-replay.jsonl",
            "--mode",
            "fader-smooth",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    // When the actual executable replays, then clock integration is not skipped.
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let rows = String::from_utf8(result.stdout)?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    let smoothing = rows
        .iter()
        .find(|r| r["kind"] == "smoothing")
        .ok_or_else(|| anyhow::anyhow!("config missing"))?;
    assert_eq!(smoothing["time_constant_us"], 20000);
    assert_eq!(smoothing["rest_cutoff_us"], 8000);
    assert_eq!(smoothing["zero_sample_dedup"], true);
    let ticks: Vec<_> = rows.iter().filter(|r| r["kind"] == "tick").collect();
    let first = ticks
        .iter()
        .find(|r| r["decision"]["torque"][0] != 0)
        .ok_or_else(|| anyhow::anyhow!("clock never emitted force"))?;
    assert_eq!(first["host_us"], 16000);
    assert_eq!(first["decision"]["torque"][0], -682);
    assert!(ticks.iter().all(|r| r["decision"]["torque"][1] == 0));
    assert_eq!(rows[0]["control"]["assist_force"], 0);
    Ok(())
}

#[test]
fn smooth_cli_guards_invalid_controls_and_requires_motor_opt_in_before_hid() -> anyhow::Result<()> {
    // Given invalid combinations and a valid manual pair without motor opt-in.
    for extra in [
        &["--enable-motor", "--side", "right"][..],
        &["--enable-motor", "--assist-force", "1"][..],
        &["--enable-motor", "--initialize-rings"][..],
        &["--enable-motor", "--gain", "1"][..],
        &["--enable-motor", "--max-force", "60001"][..],
        &["--enable-motor", "--until-stopped", "--seconds", "30"][..],
        &["--start-after-neutral", "--until-stopped"][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
            .args([
                "live",
                "--out",
                "/invalid/never-created-smooth.jsonl",
                "--mode",
                "fader-smooth",
            ])
            .args(extra)
            .output()?;
        // When validation runs, then no log creation or HID opening occurs.
        assert!(!output.status.success());
        let error = String::from_utf8(output.stderr)?;
        assert!(!error.contains("DIAGNOSTIC_HID_OPEN"));
        assert!(!error.contains("create new capture"));
        if !extra.contains(&"--enable-motor") {
            assert!(error.contains("--enable-motor"));
        }
    }
    Ok(())
}
