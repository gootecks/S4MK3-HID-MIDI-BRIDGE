use super::*;
use crate::jog_diagnostic::decode::tests::{frame, with_fader};
use crate::jog_diagnostic::motor::neutral_report;
use crate::jog_diagnostic::{Mode, Side};

fn control() -> ControlArgs {
    ControlArgs {
        mode: Mode::Assist,
        side: Side::Left,
        gain: 1,
        max_force: 100,
    }
}

#[test]
fn writer_watchdog_neutralizes_without_reader_completion() -> anyhow::Result<()> {
    // Given nonzero torque and a live, silent input/completion channel.
    let (mut reader, _frames_tx, _done_tx) = Reader::channel_only();
    let (_stop_tx, stop) = bounded(1);
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
        session.handle(Event::Sample(frame(0, 1, [0, 0])), 0)?;
        session.handle(Event::Sample(frame(2000, 2, [200, 0])), 2000)?;
        // When the actual production select loop observes its input timeout.
        // Time is the behavior under test; no sleeps or polling are used.
        let error = trial_loop(&mut session, &mut reader, (&stop, Instant::now(), None))
            .err()
            .context("silent reader must trigger watchdog")?;
        // Then it terminates independently of the stalled reader.
        assert!(error.to_string().contains("no fresh input"));
    }
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    assert!(!reader.completed);
    Ok(())
}

#[test]
fn stop_and_duration_take_priority_over_queued_motion() -> anyhow::Result<()> {
    // Given queued motion and either a pending Stop or an expired duration.
    for seconds in [0, 10] {
        let (mut reader, frames_tx, _done_tx) = Reader::channel_only();
        frames_tx.send(frame(0, 1, [0, 0]))?;
        let (stop_tx, stop) = bounded(1);
        stop_tx.send(())?;
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
            // When the real select loop handles the terminal condition.
            trial_loop(
                &mut session,
                &mut reader,
                (&stop, Instant::now(), Some(seconds)),
            )?;
        }
        // Then no queued input is consumed and every actual write is neutral.
        assert_eq!(reader.frames.len(), 1);
        assert!(writes.iter().all(|report| report == &neutral_report()));
    }
    Ok(())
}

#[test]
fn reader_error_takes_priority_over_pending_motion() -> anyhow::Result<()> {
    // Given an error notification and queued input, subscribed before execution.
    let (mut reader, frames_tx, done_tx) = Reader::channel_only();
    frames_tx.send(frame(0, 1, [0, 0]))?;
    done_tx.send(Err(anyhow::anyhow!("snapshot queue overflow")))?;
    let (_stop_tx, stop) = bounded(1);
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
        // When the production select loop receives the terminal error.
        let error = trial_loop(&mut session, &mut reader, (&stop, Instant::now(), Some(10)))
            .err()
            .context("reader failure must propagate")?;
        // Then it preserves the failure instead of applying queued torque.
        assert!(error.to_string().contains("snapshot queue overflow"));
    }
    assert_eq!(reader.frames.len(), 1);
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    Ok(())
}

#[test]
fn continuous_fader_stop_neutralizes_before_queued_input() -> anyhow::Result<()> {
    // Given active fader torque and a Stop channel subscribed before triggering it.
    let (mut reader, frames_tx, _done_tx) = Reader::channel_only();
    frames_tx.send(with_fader(frame(4000, 3, [300, 0]), 4095))?;
    let (stop_tx, stop) = bounded(1);
    let mut writes = Vec::new();
    let mut log = Vec::new();
    {
        let mut session = Session::new(
            ControlArgs {
                mode: Mode::FaderBrake,
                side: Side::Left,
                gain: 1,
                max_force: 5000,
            },
            |report| {
                writes.push(report.to_vec());
                Ok(report.len())
            },
            &mut log,
        )?;
        session.handle(Event::Sample(with_fader(frame(0, 1, [0, 0]), 0)), 0)?;
        session.handle(
            Event::Sample(with_fader(frame(2000, 2, [200, 0]), 4095)),
            2000,
        )?;
        stop_tx.send(())?;
        // When the actual continuous production loop receives Stop.
        trial_loop(&mut session, &mut reader, (&stop, Instant::now(), None))?;
    }
    // Then torque existed, Stop skipped the pending input, and the final write is zero.
    assert!(
        writes
            .iter()
            .any(|report| report == &[49, 1, 224, 254, 136, 19, 1, 32, 1, 0, 0])
    );
    assert_eq!(reader.frames.len(), 1);
    assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
    Ok(())
}
