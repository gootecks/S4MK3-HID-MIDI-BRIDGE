//! Mapping-level feedback settings: the pad colour tokens Djay sends for lit cue pads.

use plist::{Dictionary, Value};

use crate::full::feedback::{PAD_COLOR_TOKENS, PAD_WHITE_TOKEN};

/// Without these, Djay sends 127 for every lit cue pad and no colour reaches the S8.
pub(super) fn user_info() -> Dictionary {
    let mut user_info = Dictionary::new();
    user_info.insert(String::from("padColorOff"), Value::Integer(0.into()));
    user_info.insert(
        String::from("padColorWhiteActive"),
        Value::Integer(i64::from(PAD_WHITE_TOKEN).into()),
    );
    user_info.insert(
        String::from("padColorsActive"),
        Value::Array(
            PAD_COLOR_TOKENS
                .iter()
                .map(|token| Value::Integer(i64::from(*token).into()))
                .collect(),
        ),
    );
    user_info
}

#[cfg(test)]
mod tests {
    use anyhow::{Context as _, Result};
    use plist::{Dictionary, Value};

    use super::super::generate;
    use super::{PAD_COLOR_TOKENS, PAD_WHITE_TOKEN};
    use crate::s8::catalog::{ROOT_OUTPUTS, RootFamily};

    fn root() -> Result<Dictionary> {
        generate()?.into_dictionary().context("root dictionary")
    }

    #[test]
    fn user_info_carries_the_pad_colour_tokens_of_the_s4_mapping() -> Result<()> {
        let root = root()?;
        let info = root
            .get("userInfo")
            .and_then(Value::as_dictionary)
            .context("userInfo")?;
        assert_eq!(
            info.get("padColorOff").and_then(Value::as_unsigned_integer),
            Some(0)
        );
        assert_eq!(
            info.get("padColorWhiteActive")
                .and_then(Value::as_unsigned_integer),
            Some(u64::from(PAD_WHITE_TOKEN))
        );
        let colours: Vec<u64> = info
            .get("padColorsActive")
            .and_then(Value::as_array)
            .context("padColorsActive")?
            .iter()
            .filter_map(Value::as_unsigned_integer)
            .collect();
        assert_eq!(
            colours,
            PAD_COLOR_TOKENS.map(u64::from).to_vec(),
            "one token per palette colour"
        );
        Ok(())
    }

    #[test]
    fn only_the_progress_outputs_are_continuous_controls() -> Result<()> {
        let root = root()?;
        let outputs = root
            .get("outputs")
            .and_then(Value::as_array)
            .context("outputs")?;
        for root_output in ROOT_OUTPUTS {
            let key_path = root_output.key_path(1);
            let output = outputs
                .iter()
                .filter_map(Value::as_dictionary)
                .find(|output| {
                    output.get("keyPath").and_then(Value::as_string) == Some(key_path.as_str())
                })
                .with_context(|| format!("{key_path} generated"))?;
            let control_type = output.get("controlType").and_then(Value::as_string);
            if root_output.family == RootFamily::Progress {
                assert_eq!(control_type, Some("control"), "{key_path}");
            } else {
                assert_eq!(control_type, None, "{key_path}");
            }
        }
        Ok(())
    }
}
