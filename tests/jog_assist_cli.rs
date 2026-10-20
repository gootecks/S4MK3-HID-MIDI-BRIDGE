use std::process::Command;

#[test]
fn actual_assist_replay_exercises_release_retouch_and_both_directions() -> anyhow::Result<()> {
    // Given raw frames with precise touch, counter and fader transitions.
    let result = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
        .args([
            "replay",
            "--input",
            "examples/jog-diagnostic-assist-replay.jsonl",
            "--mode",
            "fader-assist",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    // When the actual executable decodes and computes its timed output.
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let rows = String::from_utf8(result.stdout)?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    let ticks: Vec<_> = rows.iter().filter(|r| r["kind"] == "tick").collect();
    let force = |r: &&serde_json::Value| r["decision"]["torque"][0].as_i64().unwrap_or(i64::MAX);
    // Then exact-time clock slots integrate and assist in both directions.
    let first = ticks
        .iter()
        .find(|r| force(r) != 0)
        .ok_or_else(|| anyhow::anyhow!("no assistance"))?;
    assert_eq!(first["host_us"], 8000);
    assert_eq!(first["decision"]["torque"][0], 45);
    assert!(ticks.iter().any(|r| force(r) < 0));
    assert!(ticks.iter().any(|r| force(r) > 0));
    assert!(
        ticks
            .iter()
            .all(|r| force(r).unsigned_abs() <= 2000 && r["decision"]["torque"][1] == 0)
    );
    assert_eq!(rows[0]["control"]["assist_force"], 2000);
    assert_eq!(rows[0]["control"]["max_force"], 0);
    for (from, to) in [(34000, 62000), (96000, 132_000), (162_000, 202_000)] {
        let window: Vec<_> = ticks
            .iter()
            .filter(|r| {
                r["host_us"]
                    .as_u64()
                    .is_some_and(|at| (from..to).contains(&at))
            })
            .collect();
        assert!(!window.is_empty());
        assert!(
            window.iter().all(|r| force(r) == 0),
            "force in gated interval {from}..{to}"
        );
    }
    assert!(ticks.iter().any(|r| {
        r["host_us"]
            .as_u64()
            .is_some_and(|at| (138_000..162_000).contains(&at))
            && force(r) > 0
    }));
    assert_eq!(
        rows.last().ok_or_else(|| anyhow::anyhow!("end missing"))?["reason"],
        "replay_eof"
    );
    Ok(())
}

#[test]
fn assist_cli_rejects_braking_and_other_incompatible_flags_before_hid() -> anyhow::Result<()> {
    // Given invalid motor controls, plus valid manual flags without motor opt-in.
    for extra in [
        &["--enable-motor", "--max-force", "30000"][..],
        &["--enable-motor", "--side", "right"][..],
        &["--enable-motor", "--gain", "1"][..],
        &["--enable-motor", "--assist-force", "8001"][..],
        &["--enable-motor", "--initialize-rings"][..],
        &["--enable-motor", "--until-stopped", "--seconds", "30"][..],
        &["--start-after-neutral", "--until-stopped"][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
            .args([
                "live",
                "--out",
                "/invalid/never-created-assist.jsonl",
                "--mode",
                "fader-assist",
            ])
            .args(extra)
            .output()?;
        // When validation runs, then it fails before log creation or HID opening.
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
