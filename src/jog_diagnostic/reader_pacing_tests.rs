use super::*;
use std::cell::Cell;

#[test]
fn ready_interrupts_cannot_accelerate_snapshot_polling() -> anyhow::Result<()> {
    let (_cancel_tx, cancel) = bounded(1);
    let now = Cell::new(0_u64);
    let calls = Cell::new(0_u16);
    let mut next = 0;
    let mut reads = Vec::new();
    // Given native acquisition completing quickly because interrupts are ready.
    for _ in 0..8 {
        // When the actual reader's poll gate schedules another acquisition.
        assert!(poll_slot(
            &mut next,
            &cancel,
            (
                || {
                    calls.set(calls.get() + 1);
                    ensure!(calls.get() < 100, "poll gate made no progress");
                    Ok(now.get())
                },
                |wait| {
                    now.set(now.get() + wait);
                    false
                }
            )
        )?);
        reads.push(now.get());
        now.set(now.get() + 100);
    }
    // Then starts remain on the intended rate rather than racing at 100us.
    assert_eq!(
        reads,
        (0..8).map(|i| i * OUTPUT_INTERVAL_US).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn delayed_acquisition_does_not_catch_up_in_a_burst() -> anyhow::Result<()> {
    let (_cancel_tx, cancel) = bounded(1);
    let now = Cell::new(9999_u64);
    let mut next = 2000;
    let mut reads = Vec::new();
    // Given a delayed acquisition after several missed periods.
    for _ in 0..3 {
        // When subsequent acquisitions are immediately available.
        assert!(poll_slot(
            &mut next,
            &cancel,
            (
                || Ok(now.get()),
                |wait| {
                    now.set(now.get() + wait);
                    false
                }
            )
        )?);
        reads.push(now.get());
    }
    // Then no catch-up reads or compressed periods are issued.
    assert_eq!(reads, [9999, 11999, 13999]);
    Ok(())
}

#[test]
fn cancellation_wins_before_or_during_the_poll_wait() -> anyhow::Result<()> {
    // Given either a queued cancel or a cancel reported by the wait itself.
    for prequeued in [false, true] {
        let (tx, cancel) = bounded(1);
        if prequeued {
            tx.send(())?;
        }
        let waited = Cell::new(false);
        let mut next = 2000;
        // When the reader is waiting for its next acquisition slot.
        let acquire = poll_slot(
            &mut next,
            &cancel,
            (
                || Ok(0),
                |_| {
                    waited.set(true);
                    true
                },
            ),
        )?;
        // Then no snapshot read is admitted after cancellation.
        assert!(!acquire);
        assert_eq!(waited.get(), !prequeued);
    }
    Ok(())
}

#[test]
fn genuine_overflow_and_disconnection_remain_fatal_with_snapshot_time() -> anyhow::Result<()> {
    use crate::jog_diagnostic::decode::tests::frame;

    // Given either a genuinely full queue or a lost consumer.
    for disconnected in [false, true] {
        let (tx, rx) = bounded(32);
        for index in 0..32 {
            tx.send(frame(u64::from(index) * 2000, index + 1, [index, 0]))?;
        }
        let (_cancel_tx, cancel) = bounded(1);
        let consumer = if disconnected {
            drop(rx);
            None
        } else {
            Some(rx)
        };
        // When the actual publisher cannot enqueue the next snapshot.
        let error = publish_frame(frame(64_000, 33, [32, 0]), (&tx, &cancel))
            .err()
            .context("publication must fail")?;
        // Then it fails rather than dropping/retrying; timing distinguishes a future stall.
        assert!(error.to_string().contains("rejected_snapshot_us=64000"));
        if let Some(rx) = consumer {
            assert!(error.to_string().contains("capacity=Some(32)"));
            assert_eq!(rx.try_recv()?.host_us, 0);
        } else {
            assert!(error.to_string().contains("snapshot consumer disconnected"));
        }
    }
    Ok(())
}
