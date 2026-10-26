//! Native Djay jog mapping preferences and runtime backspin options.

use std::path::Path;

use anyhow::{Context as _, Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub scratch_speed: f64,
    pub scratch_reaction: u8,
    pub pitch_bend_speed: f64,
    pub pitch_bend_reaction: u8,
    pub backspin_enabled: bool,
    pub backspin_assist_enabled: bool,
    pub backspin_assist_strength: u8,
    pub backspin_acceleration: f64,
    pub backspin_speed_increase: f64,
}

pub const DEFAULT: Config = Config {
    scratch_speed: 2.7,
    scratch_reaction: 150,
    pitch_bend_speed: 2.7,
    pitch_bend_reaction: 17,
    backspin_enabled: true,
    backspin_assist_enabled: false,
    backspin_assist_strength: 100,
    backspin_acceleration: 150_000.0,
    backspin_speed_increase: 0.0,
};

impl Default for Config {
    fn default() -> Self {
        DEFAULT
    }
}

impl Config {
    pub const fn motor_assist_enabled(&self) -> bool {
        self.backspin_enabled && self.backspin_assist_enabled && self.backspin_assist_strength > 0
    }

    pub fn load(path: Option<&Path>) -> Result<Self> {
        match path {
            None => Ok(Self::default()),
            Some(path) => {
                let bytes = std::fs::read(path)
                    .with_context(|| format!("read jog mapping preferences {}", path.display()))?;
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("parse jog mapping preferences {}", path.display()))
            }
        }
    }

    pub fn validate(&self) -> Result<()> {
        for (name, speed) in [
            ("scratch_speed", self.scratch_speed),
            ("pitch_bend_speed", self.pitch_bend_speed),
        ] {
            ensure!(
                speed.is_finite() && speed > 0.0,
                "{name} must be finite and positive"
            );
        }
        for (name, reaction) in [
            ("scratch_reaction", self.scratch_reaction),
            ("pitch_bend_reaction", self.pitch_bend_reaction),
        ] {
            ensure!(reaction <= 150, "{name} must be between 0 and 150");
        }
        ensure!(
            self.backspin_acceleration.is_finite() && self.backspin_acceleration > 0.0,
            "backspin_acceleration must be finite and positive"
        );
        ensure!(
            self.backspin_assist_strength <= 100,
            "backspin_assist_strength must be between 0 and 100"
        );
        ensure!(
            self.backspin_speed_increase.is_finite()
                && self.backspin_speed_increase >= 0.0
                && (self.scratch_speed + self.backspin_speed_increase).is_finite(),
            "backspin_speed_increase must be finite and nonnegative without overflowing scratch speed"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_preferences_match_defaults_and_partial_overrides() -> Result<()> {
        // Given the configuration shipped to the native app.
        let shipped: Config = serde_json::from_str(include_str!("../../examples/jog-config.json"))?;
        // When it is parsed and a partial user configuration is decoded.
        shipped.validate()?;
        let partial: Config = serde_json::from_str(r#"{"scratch_speed":4.5}"#)?;
        // Then shipped defaults agree and omitted user fields retain those defaults.
        assert_eq!(
            serde_json::to_value(shipped)?,
            serde_json::to_value(DEFAULT)?
        );
        let mut expected = DEFAULT;
        expected.scratch_speed = 4.5;
        assert_eq!(
            serde_json::to_value(partial)?,
            serde_json::to_value(expected)?
        );
        assert!(serde_json::from_str::<Config>(r#"{"unknown":true}"#).is_err());
        assert!(serde_json::from_str::<Config>(r#"{"scratch_reaction":-1}"#).is_err());
        assert!(serde_json::from_str::<Config>(r#"{"scratch_reaction":17.5}"#).is_err());
        for input in [
            r#"{"backspin_assist_enabled":1}"#,
            r#"{"backspin_assist_enabled":"true"}"#,
            r#"{"backspin_assist_enabled":null}"#,
            r#"{"backspin_assist_strength":true}"#,
            r#"{"backspin_assist_strength":"50"}"#,
            r#"{"backspin_assist_strength":null}"#,
            r#"{"backspin_assist_strength":50.0}"#,
            r#"{"backspin_assist_strength":5e1}"#,
            r#"{"backspin_assist_strength":-1}"#,
            r#"{"backspin_assist_strength":256}"#,
        ] {
            assert!(serde_json::from_str::<Config>(input).is_err());
        }
        Ok(())
    }

    #[test]
    fn motor_output_requires_detection_opt_in_and_nonzero_strength() -> Result<()> {
        // Given all combinations, including a legacy saved configuration.
        let legacy: Config = serde_json::from_str(r#"{"backspin_speed_increase":5.2}"#)?;
        assert!(!legacy.backspin_assist_enabled);
        assert_eq!(legacy.backspin_assist_strength, 100);
        for detection in [false, true] {
            for assist in [false, true] {
                for strength in [0, 37, 100] {
                    let config = Config {
                        backspin_enabled: detection,
                        backspin_assist_enabled: assist,
                        backspin_assist_strength: strength,
                        ..DEFAULT
                    };
                    // When runtime selects whether a motor owner is needed,
                    // then both enables and nonzero strength are necessary.
                    assert_eq!(
                        config.motor_assist_enabled(),
                        detection && assist && strength > 0
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn strength_round_trips_and_rejects_values_above_the_existing_maximum() -> Result<()> {
        // Given a non-default independently selected assist percentage.
        let config: Config = serde_json::from_str(
            r#"{"backspin_assist_enabled":true,"backspin_assist_strength":37,"backspin_speed_increase":5.2}"#,
        )?;
        config.validate()?;
        // When the saved configuration is encoded and decoded.
        let reloaded: Config = serde_json::from_value(serde_json::to_value(config)?)?;
        // Then strength and software boost remain independent, and 101 is rejected.
        assert_eq!(reloaded.backspin_assist_strength, 37);
        assert!((reloaded.backspin_speed_increase - 5.2).abs() < f64::EPSILON);
        let excessive: Config = serde_json::from_str(r#"{"backspin_assist_strength":101}"#)?;
        assert!(excessive.validate().is_err());
        Ok(())
    }
}
