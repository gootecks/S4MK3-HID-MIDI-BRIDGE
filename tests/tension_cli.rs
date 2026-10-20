use std::process::Command;

#[test]
fn tension_config_rejects_nonruntime_profiles_before_hid() -> anyhow::Result<()> {
    // Given commands that must never activate runtime hardware tension.
    for args in [
        vec!["--midi", "minimal"],
        vec!["--list"],
        vec!["--generate-mapping", "/dev/null/not-created.plist"],
        vec![],
    ] {
        // When the actual CLI receives the runtime-only configuration option.
        let output = Command::new(env!("CARGO_BIN_EXE_s4mk3-hid-midi-bridge"))
            .args(args)
            .args(["--tension-config", "/dev/null/not-read.json"])
            .output()?;
        // Then it fails argument/profile admission before scanning or opening HID.
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8(output.stderr)?.contains("S4_HID_OPEN"));
    }
    Ok(())
}

#[test]
fn full_runtime_validates_config_before_enumeration_or_hid() -> anyhow::Result<()> {
    // Given an unreadable configuration path with an otherwise valid full profile.
    // When the actual CLI starts.
    let output = Command::new(env!("CARGO_BIN_EXE_s4mk3-hid-midi-bridge"))
        .args([
            "--midi",
            "full",
            "--until-stopped",
            "--tension-config",
            "/dev/null/not-read.json",
        ])
        .output()?;
    // Then configuration loading fails before it can enumerate/open the controller.
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr)?;
    assert!(error.contains("/dev/null/not-read.json"));
    assert!(!error.contains("S4_HID_OPEN"));
    Ok(())
}
