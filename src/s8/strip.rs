//! Touchstrip position decoding and ownership of one touch.

use super::catalog::Layer;

const POSITIONS: f32 = 1024.0;

/// The raw 10-bit position; 0 means no finger.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn position(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * POSITIONS).round() as u16
}

/// The 7-bit value Djay turns into strip motion.
pub(super) fn cc_value(position: u16) -> u8 {
    u8::try_from((position.saturating_sub(1) >> 3).min(127)).unwrap_or(127)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Owner {
    pub deck: u8,
    pub layer: Layer,
}

/// One strip. The deck and layer in force when the touch began own the touch
/// note and every motion until release.
#[derive(Debug, Default)]
pub(super) struct Strip {
    owner: Option<Owner>,
    last: Option<u8>,
}

impl Strip {
    pub(super) const fn touch(&mut self, owner: Owner) {
        self.owner = Some(owner);
        self.last = None;
    }

    pub(super) const fn release(&mut self) -> Option<Owner> {
        self.last = None;
        self.owner.take()
    }

    /// The motion value for a finger at `position`, once per distinct value.
    pub(super) fn motion(&mut self, position: u16) -> Option<(Owner, u8)> {
        let owner = self.owner?;
        if position == 0 {
            return None;
        }
        let value = cc_value(position);
        if self.last == Some(value) {
            return None;
        }
        self.last = Some(value);
        Some((owner, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_map_to_seven_bit_values_at_the_boundaries() {
        // Given raw positions from the first step to the last.
        let cases = [(1, 0), (8, 0), (9, 1), (512, 63), (1023, 127), (1024, 127)];
        for (raw, expected) in cases {
            // When they are decoded from the normalised slider value.
            let decoded = position(f32::from(raw) / POSITIONS);

            // Then each keeps its position and maps to the expected value.
            assert_eq!(decoded, raw);
            assert_eq!(cc_value(decoded), expected, "position {raw}");
        }
    }

    #[test]
    fn a_released_strip_reports_no_motion() {
        let mut strip = Strip::default();
        strip.touch(Owner {
            deck: 1,
            layer: Layer::Base,
        });
        assert_eq!(strip.motion(0), None);
    }

    #[test]
    fn motion_is_reported_once_per_value_and_restarts_after_release() {
        let owner = Owner {
            deck: 3,
            layer: Layer::Shift,
        };
        let mut strip = Strip::default();
        strip.touch(owner);
        assert_eq!(strip.motion(100), Some((owner, 12)));
        assert_eq!(strip.motion(101), None);
        assert_eq!(strip.motion(200), Some((owner, 24)));
        assert_eq!(strip.release(), Some(owner));
        assert_eq!(strip.motion(200), None);
        strip.touch(owner);
        assert_eq!(strip.motion(200), Some((owner, 24)));
    }
}
