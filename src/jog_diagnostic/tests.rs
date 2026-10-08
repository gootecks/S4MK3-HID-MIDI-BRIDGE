use super::*;

#[test]
fn invalid_live_trial_fails_before_device_or_file_access() -> anyhow::Result<()> {
    // Given a live command without authorization and an unwritable path.
    let args = Args::try_parse_from([
        "diagnostic",
        "live",
        "--out",
        "/invalid/never-created.jsonl",
        "--mode",
        "assist",
        "--seconds",
        "5",
    ])?;
    // When the command runs.
    let error = run(args).err().context("missing motor opt-in must fail")?;
    // Then the validation gate, not HID or file creation, is the failure.
    assert!(error.to_string().contains("--enable-motor"));
    Ok(())
}

#[test]
fn cli_rejects_unbounded_and_invalid_control_values() {
    // Given out-of-range diagnostic options.
    for (flag, value) in [
        ("--seconds", "0"),
        ("--seconds", "31"),
        ("--gain", "0"),
        ("--gain", "101"),
        ("--max-force", "0"),
        ("--max-force", "1001"),
    ] {
        // When the standalone argument parser receives them.
        let result = Args::try_parse_from([
            "diagnostic",
            "live",
            "--out",
            "unused.jsonl",
            "--mode",
            "assist",
            "--enable-motor",
            flag,
            value,
        ]);
        // Then no live command can be constructed.
        assert!(result.is_err());
    }
}

#[test]
fn live_ceiling_is_stricter_than_offline_replay() -> anyhow::Result<()> {
    // Given a force supported for offline analysis but not a live trial.
    let args = Args::try_parse_from([
        "diagnostic",
        "live",
        "--out",
        "/invalid/never-created.jsonl",
        "--mode",
        "assist",
        "--enable-motor",
        "--max-force",
        "501",
    ])?;
    // When validation runs.
    let error = run(args).err().context("live cap must fail")?;
    // Then it fails before touching the unwritable path or HID.
    assert!(error.to_string().contains("<=500"));
    Ok(())
}

#[test]
fn raw_json_rejects_unrecognized_fields() {
    // Given a typo in a raw-frame schema.
    let raw = r#"{"host_us":0,"buttons":[],"jog":[],"ignored":true}"#;
    // When evidence is parsed, then the error is visible.
    assert!(serde_json::from_str::<capture::RawFrame>(raw).is_err());
}
