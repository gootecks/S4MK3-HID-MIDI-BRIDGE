//! Logging goes to stdout; a closed stdout and stderr must not make it panic.

use std::io::Write as _;
use std::process::{Command, Stdio};

#[test]
fn logging_with_stdout_and_stderr_closed_does_not_panic() -> anyhow::Result<()> {
    // Given a bridge that starts only after the reader of both streams is gone.
    let dir = std::env::temp_dir().join(format!("s8-closed-output-{}", std::process::id()));
    std::fs::create_dir(&dir)?;
    let result = (|| -> anyhow::Result<()> {
        let mut child = Command::new("sh")
            .args(["-c", "read gate; exec \"$0\" \"$@\""])
            .arg(env!("CARGO_BIN_EXE_s4mk3-hid-midi-bridge"))
            .args(["--device", "s8", "--generate-mapping"])
            .arg(&dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        drop(child.stdout.take());
        drop(child.stderr.take());
        // When it logs the exported mapping.
        child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("no stdin"))?
            .write_all(b"go\n")?;
        // Then it still exits normally.
        let status = child.wait()?;
        assert!(status.success(), "{status}");
        Ok(())
    })();
    std::fs::remove_dir_all(&dir)?;
    result
}
