use super::*;
use anyhow::Context as _;

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
        analog: None,
    }
}

pub fn with_fader(input: RawFrame, raw: u16) -> RawFrame {
    let mut analog = vec![0; 79];
    analog[0] = 2;
    analog[3..5].copy_from_slice(&raw.to_le_bytes());
    RawFrame {
        analog: Some(analog),
        ..input
    }
}

#[test]
fn deck1_fader_uses_its_little_endian_field_not_neighbors() -> anyhow::Result<()> {
    // Given channel 1 at each meaningful position and distinct adjacent controls.
    for raw in [0, 2048, 4095] {
        let mut input = with_fader(frame(0, 1, [0, 0]), raw);
        if let Some(analog) = &mut input.analog {
            analog[1..3].copy_from_slice(&4095_u16.to_le_bytes());
            analog[5..7].copy_from_slice(&1234_u16.to_le_bytes());
        }
        // When the real numbered analog report is decoded.
        let motion = Decoder::default().decode(&input)?;
        // Then only the physical channel-1 field controls its decoded level.
        assert_eq!(motion.deck1_fader, Some(raw));
    }
    Ok(())
}

#[test]
fn malformed_analog_framing_and_range_fail_before_motion() -> anyhow::Result<()> {
    // Given a short report, another report ID, and an invalid 12-bit value.
    for (length, id, raw) in [(5, 2, 1), (79, 1, 1), (79, 2, 4096)] {
        let mut input = with_fader(frame(0, 1, [0, 0]), raw);
        let analog = input.analog.as_mut().context("analog fixture missing")?;
        analog.truncate(length);
        analog[0] = id;
        // When the real decoder sees invalid analog evidence.
        // Then it rejects the frame rather than producing a usable fader level.
        assert!(Decoder::default().decode(&input).is_err());
    }
    Ok(())
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
