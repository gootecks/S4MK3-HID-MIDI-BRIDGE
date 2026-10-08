use super::*;

pub fn frame(host_us: u64, timer: u32, counters: [u32; 2]) -> RawFrame {
    let mut buttons = vec![0; 23];
    buttons[0] = 1;
    buttons[17] = 0x30;
    let mut jog = vec![0; 60];
    jog[0] = 3;
    for (offset, value) in [
        (8, timer),
        (12, counters[0]),
        (36, timer),
        (40, counters[1]),
    ] {
        jog[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    RawFrame {
        host_us,
        buttons,
        jog,
    }
}

#[test]
fn decodes_full_counters_both_sides_and_touch_release() -> anyhow::Result<()> {
    // Given independent counters spanning more than the old 16-bit layout.
    let mut decoder = Decoder::default();
    decoder.decode(&frame(0, 1, [65_535, 100_000]))?;
    let mut released = frame(2000, 2, [65_545, 99_980]);
    released.buttons[17] = 0;
    // When full raw snapshots reach the decoder.
    let motion = decoder.decode(&released)?;
    // Then all motion survives touch release and counter boundaries.
    assert_eq!(motion.delta, [10, -20]);
    assert_eq!(motion.touch, [false, false]);
    Ok(())
}

#[test]
fn signed_counter_and_timer_rollover_preserve_motion() -> anyhow::Result<()> {
    // Given counters and sensor time on their rollover boundaries.
    let mut decoder = Decoder::default();
    decoder.decode(&frame(0, u32::MAX - 1, [u32::MAX - 1, 1]))?;
    // When counters wrap in opposite directions.
    let motion = decoder.decode(&frame(2000, 1, [1, u32::MAX - 1]))?;
    // Then deltas remain small and correctly signed.
    assert_eq!(motion.delta, [3, -3]);
    Ok(())
}

#[test]
fn duplicate_cannot_refresh_deadline() -> anyhow::Result<()> {
    // Given one baseline followed by identical sensor readings.
    let mut decoder = Decoder::default();
    decoder.decode(&frame(0, 1, [0, 0]))?;
    // When copies arrive within and then beyond the deadline.
    let duplicate = decoder.decode(&frame(40_000, 1, [0, 0]))?;
    // Then the duplicate is neutral and cannot keep the baseline alive.
    assert!(matches!(duplicate.status[0], Status::Duplicate));
    assert!(decoder.decode(&frame(50_001, 1, [0, 0])).is_err());
    Ok(())
}

#[test]
fn malformed_order_and_implausible_motion_fail_closed() -> anyhow::Result<()> {
    // Given valid baselines for each independently malformed frame.
    for next in [
        frame(0, 2, [1, 0]),
        frame(2000, 1, [1, 0]),
        frame(2000, 0, [1, 0]),
        frame(2000, 2, [513, 0]),
        frame(50_001, 2, [1, 0]),
    ] {
        let mut decoder = Decoder::default();
        decoder.decode(&frame(0, 1, [0, 0]))?;
        // When host/sensor time, count magnitude, or freshness is invalid.
        // Then no derived motion is accepted.
        assert!(decoder.decode(&next).is_err());
    }
    Ok(())
}

#[test]
fn truncated_reports_are_rejected_before_field_access() {
    // Given an incomplete report.
    let mut bad = frame(0, 1, [0, 0]);
    bad.jog.truncate(44);
    // When the decoder reads it, then framing fails rather than panicking.
    assert!(Decoder::default().decode(&bad).is_err());
}
