//! Exact Djay MIDI key values paired with labels by the paused-track probe.

pub const fn label(value: u8) -> Option<&'static str> {
    match value {
        0 => Some("--"),
        5 => Some("8B"),
        10 => Some("8A"),
        15 => Some("3B"),
        20 => Some("3A"),
        25 => Some("10B"),
        30 => Some("10A"),
        36 => Some("5B"),
        41 => Some("5A"),
        46 => Some("12B"),
        51 => Some("12A"),
        56 => Some("7B"),
        61 => Some("7A"),
        66 => Some("2B"),
        71 => Some("2A"),
        76 => Some("9B"),
        81 => Some("9A"),
        86 => Some("4B"),
        91 => Some("4A"),
        97 => Some("11B"),
        102 => Some("11A"),
        107 => Some("6B"),
        112 => Some("6A"),
        117 => Some("1B"),
        122 => Some("1A"),
        _ => None,
    }
}
