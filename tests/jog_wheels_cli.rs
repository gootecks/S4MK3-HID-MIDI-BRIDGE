use std::process::Command;

#[test]
fn actual_cli_drives_only_selected_wheels_from_independent_reports() -> anyhow::Result<()> {
    // Given raw physical motion on both sides, including different touch states.
    for side in ["left", "right", "both"] {
        let result = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
            .args([
                "replay",
                "--input",
                "examples/jog-diagnostic-wheels-replay.jsonl",
                "--mode",
                "fader-play",
                "--side",
                side,
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()?;
        // When the real executable processes the new selection.
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let rows = String::from_utf8(result.stdout)?
            .lines()
            .map(serde_json::from_str::<serde_json::Value>)
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(rows[0]["control"]["side"], side);
        let ticks: Vec<_> = rows.iter().filter(|r| r["kind"] == "tick").collect();
        // Then selected wheels are real, bounded outputs and peers remain neutral.
        for index in 0..2 {
            let selected = side == "both" || side == if index == 0 { "left" } else { "right" };
            let values = ticks
                .iter()
                .map(|r| {
                    r["decision"]["torque"][index]
                        .as_i64()
                        .ok_or_else(|| anyhow::anyhow!("missing torque"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if selected {
                assert!(values.iter().any(|v| *v > 0));
                assert!(values.iter().any(|v| *v < 0));
            } else {
                assert!(values.iter().all(|v| *v == 0));
            }
        }
        for r in ticks {
            let cap = r["decision"]["fader"]["selected_force"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("cap"))?;
            for index in 0..2 {
                let value = r["decision"]["torque"][index]
                    .as_i64()
                    .ok_or_else(|| anyhow::anyhow!("torque"))?;
                assert!(value.unsigned_abs() <= cap);
            }
            if side == "both" {
                let at = r["host_us"]
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("time"))?;
                if (42000..54000).contains(&at) {
                    assert_eq!(r["decision"]["torque"][1], 0);
                }
                if (54000..66000).contains(&at) {
                    assert_eq!(r["decision"]["torque"][0], 0);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn wheel_selection_is_admitted_only_by_the_intended_profile() -> anyhow::Result<()> {
    // Given new right/both free-play paths and legacy profiles that must not change.
    for (mode, side, admitted) in [
        ("fader-play", "right", true),
        ("fader-play", "both", true),
        ("fader-smooth", "right", false),
        ("fader-assist", "right", false),
        ("fader-continuity", "both", false),
        ("assist", "both", false),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_s4mk3-jog-diagnostic"))
            .args([
                "live",
                "--out",
                "/dev/null/never-created-wheels.jsonl",
                "--mode",
                mode,
                "--side",
                side,
                "--enable-motor",
            ])
            .args(if admitted {
                &[
                    "--initialize-rings",
                    "--start-after-neutral",
                    "--until-stopped",
                ][..]
            } else {
                &[][..]
            })
            .output()?;
        // When validation runs, then valid paths reach log creation, never HID.
        assert!(!result.status.success());
        let error = String::from_utf8(result.stderr)?;
        assert_eq!(error.contains("create new capture"), admitted);
        assert!(!error.contains("DIAGNOSTIC_HID_OPEN"));
    }
    Ok(())
}
