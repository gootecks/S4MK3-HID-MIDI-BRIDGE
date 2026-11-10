//! Process output that survives a closed stdout. Rust ignores SIGPIPE, so a
//! write to a pipe whose reader is gone fails instead of killing the process,
//! and `println!` turns that failure into a panic.

use std::fmt::Display;
use std::io::{self, Write};

use tracing_subscriber::fmt::MakeWriter;

/// One line on stdout, flushed. The error means the reader is gone.
pub fn emit(line: impl Display) -> io::Result<()> {
    write_line(&mut io::stdout().lock(), line)
}

fn write_line(out: &mut impl Write, line: impl Display) -> io::Result<()> {
    writeln!(out, "{line}")?;
    out.flush()
}

/// Log writer for stdout, where the bridge logs. Once the reader is gone it
/// drops the output, because the tracing formatter reports a failed write with
/// `eprintln!`, which panics when stderr is closed as well.
#[derive(Clone, Copy)]
pub struct QuietStdout;

pub struct QuietWriter(io::Stdout);

impl Write for QuietWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(self.0.write(buf).unwrap_or(buf.len()))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush().ok();
        Ok(())
    }
}

impl MakeWriter<'_> for QuietStdout {
    type Writer = QuietWriter;

    fn make_writer(&self) -> QuietWriter {
        QuietWriter(io::stdout())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Closed;

    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }

    #[test]
    fn a_closed_reader_is_an_error_not_a_panic() {
        let error = write_line(&mut Closed, "S8_SCREEN_STATUS active").err();
        assert_eq!(error.map(|e| e.kind()), Some(io::ErrorKind::BrokenPipe));
    }

    #[test]
    fn a_line_is_written_whole_with_a_newline() -> io::Result<()> {
        let mut out = Vec::new();
        write_line(&mut out, "S8_SCREEN_STATUS active")?;
        assert_eq!(out, b"S8_SCREEN_STATUS active\n");
        Ok(())
    }
}
