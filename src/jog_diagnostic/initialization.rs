//! Separately opted-in native jog-ring startup, under the neutral guard.

use anyhow::ensure;

use super::super::capture::{LogRecord, emit};
use super::Session;

impl<F: FnMut(&[u8]) -> anyhow::Result<usize>> Session<'_, F> {
    pub fn initialize_rings(&mut self) -> anyhow::Result<()> {
        for side in 0..2 {
            let mut report = [0_u8; 28];
            report[..4].copy_from_slice(&[0x30, side, 1, 3]);
            let result = (self.writer)(&report);
            let written = result.as_ref().ok().copied();
            let error = match &result {
                Ok(length) if *length == report.len() => None,
                Ok(length) => Some(format!("short initialization write: {length}/28")),
                Err(error) => Some(format!("{error:#}")),
            };
            let hardware = result.and_then(|length| {
                ensure!(
                    length == report.len(),
                    "short initialization write: {length}/28"
                );
                Ok(())
            });
            let logging = emit(
                self.log,
                &LogRecord::Initialization {
                    report,
                    written,
                    error,
                },
            );
            let result = match (hardware, logging) {
                (Ok(()), logging) => logging,
                (Err(error), Ok(())) => Err(error),
                (Err(error), Err(logging)) => {
                    Err(error.context(format!("initialization log also failed: {logging:#}")))
                }
            };
            if let Err(error) = result {
                if let Err(cleanup) = self.finish("initialization_error") {
                    return Err(error.context(format!("neutral cleanup also failed: {cleanup:#}")));
                }
                return Err(error);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jog_diagnostic::motor::neutral_report;
    use crate::jog_diagnostic::{ControlArgs, Mode, Side};
    use anyhow::{Context as _, bail};

    fn control() -> ControlArgs {
        ControlArgs {
            mode: Mode::FaderContinuity,
            side: Side::Left,
            gain: 1,
            max_force: 30000,
            assist_force: Some(8000),
        }
    }

    #[test]
    fn separate_initialization_is_bracketed_by_motor_neutral() -> anyhow::Result<()> {
        // Given a guarded session, with initialization deliberately requested.
        let mut writes = Vec::new();
        let mut log = Vec::new();
        {
            let mut session = Session::new(
                control(),
                |report| {
                    writes.push(report.to_vec());
                    Ok(report.len())
                },
                &mut log,
            )?;
            // When the isolated initialization comparison runs.
            session.initialize_rings()?;
            session.finish("test")?;
        }
        // Then only the two exact native ring reports intervene between zeros.
        let mut left = vec![0_u8; 28];
        left[..4].copy_from_slice(&[0x30, 0, 1, 3]);
        let mut right = left.clone();
        right[1] = 1;
        assert_eq!(
            writes,
            [
                neutral_report().to_vec(),
                left,
                right,
                neutral_report().to_vec()
            ]
        );
        let records = String::from_utf8(log)?
            .lines()
            .map(serde_json::from_str::<LogRecord>)
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            records
                .iter()
                .filter(|r| matches!(
                    r,
                    LogRecord::Initialization {
                        written: Some(28),
                        error: None,
                        ..
                    }
                ))
                .count(),
            2
        );
        Ok(())
    }

    #[test]
    fn failed_or_short_initialization_preserves_error_and_neutralizes() -> anyhow::Result<()> {
        // Given a failing transport on either initialization report.
        for failure_side in 0..2 {
            for short in [false, true] {
                let mut writes = Vec::new();
                let mut log = Vec::new();
                {
                    let mut session = Session::new(
                        control(),
                        |report| {
                            writes.push(report.to_vec());
                            if report[0] == 0x30 && report[1] == failure_side {
                                if short {
                                    return Ok(3);
                                }
                                bail!("initialization transport failed");
                            }
                            Ok(report.len())
                        },
                        &mut log,
                    )?;
                    // When either of the independent startup writes fails.
                    let error = session.initialize_rings().err().context("must fail")?;
                    assert!(error.to_string().contains(if short {
                        "short initialization write"
                    } else {
                        "initialization transport failed"
                    }));
                }
                // Then a real neutral attempt follows the failed initialization.
                assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
            }
        }
        Ok(())
    }

    #[test]
    fn free_play_startup_is_opt_in_and_preserves_preparation_zero_gate() -> anyhow::Result<()> {
        use crate::jog_diagnostic::assist_tests::input;
        use crate::jog_diagnostic::session::Event;

        // Given free play with internally qualifying movement during preparation.
        for initialize in [false, true] {
            let mut settings = control();
            settings.mode = Mode::FaderPlay;
            settings.assist_force = Some(7000);
            let mut writes = Vec::new();
            let mut log = Vec::new();
            {
                let mut session = Session::new(
                    settings,
                    |r| {
                        writes.push(r.to_vec());
                        Ok(r.len())
                    },
                    &mut log,
                )?;
                session.prepare_manual()?;
                // When the existing startup pair is explicitly requested or omitted.
                if initialize {
                    session.initialize_rings()?;
                }
                for index in 0..=12 {
                    let at = u64::from(index) * 2000;
                    session.handle(Event::Sample(input(index, 1000 + 2 * index, true)), at)?;
                    if index > 0 {
                        session.handle(Event::OutputTick, at)?;
                    }
                }
                session.finish("test")?;
            }
            // Then only opt-in emits the exact pair, and all motor writes stay zero.
            let reports: Vec<_> = writes.iter().filter(|r| r.len() == 28).collect();
            assert_eq!(reports.len(), if initialize { 2 } else { 0 });
            for (report, side) in reports.iter().zip(0_u8..2) {
                assert_eq!(report[..4], [0x30, side, 1, 3]);
                assert!(report[4..].iter().all(|byte| *byte == 0));
            }
            assert!(
                writes
                    .iter()
                    .filter(|r| r.len() == 11)
                    .all(|r| r == &neutral_report())
            );
            assert_eq!(writes.first(), Some(&neutral_report().to_vec()));
            assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
        }
        Ok(())
    }
}
