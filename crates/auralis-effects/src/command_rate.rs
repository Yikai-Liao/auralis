use auralis_core::SampleRate;

use crate::Rate;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, reject_extra_arguments,
    required_arg,
};

pub(super) fn parse_rate(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let value = required_arg(effect, args, "frequency")?;
    if is_option_like(value) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: value.to_owned(),
        });
    }
    reject_extra_arguments(effect, args.get(1..).unwrap_or_default())?;

    parse_sample_rate(effect, value).map(|sample_rate| EffectCommand::Rate(Rate::new(sample_rate)))
}

pub(super) fn render_rate(rate: Rate) -> Vec<String> {
    vec![
        "rate".to_owned(),
        rate.target_sample_rate.as_u32().to_string(),
    ]
}

fn parse_sample_rate(effect: &'static str, value: &str) -> CommandResult<SampleRate> {
    let (number_text, multiplier) = if let Some(number) = value.strip_suffix('k') {
        (number, 1_000.0)
    } else if let Some(number) = value.strip_suffix('K') {
        (number, 1_000.0)
    } else {
        (value, 1.0)
    };
    let number =
        number_text
            .parse::<f64>()
            .map_err(|source| EffectCommandParseError::InvalidNumber {
                effect,
                argument: "frequency",
                value: value.to_owned(),
                source,
            })?;
    let sample_rate = number * multiplier;
    if !sample_rate.is_finite() || sample_rate < 0.5 || sample_rate > f64::from(u32::MAX) {
        return Err(EffectCommandParseError::InvalidCoreValue {
            effect,
            argument: "frequency",
            source: auralis_core::AuralisError::InvalidSampleRate,
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "range and finiteness are validated before rounding into Auralis' integer sample-rate type"
    )]
    let rounded = sample_rate.round() as u32;
    SampleRate::new(rounded).map_err(|source| EffectCommandParseError::InvalidCoreValue {
        effect,
        argument: "frequency",
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_rate;
    use crate::{EffectCommand, EffectCommandParseError, Rate};
    use auralis_core::SampleRate;

    #[test]
    fn parses_integer_and_kilohertz_frequency() {
        assert_eq!(
            parse_rate("rate", &["24000"]).unwrap(),
            EffectCommand::Rate(Rate::new(SampleRate::new(24_000).unwrap()))
        );
        assert_eq!(
            parse_rate("rate", &["44.1k"]).unwrap(),
            EffectCommand::Rate(Rate::new(SampleRate::new(44_100).unwrap()))
        );
        assert_eq!(
            parse_rate("rate", &["44.1k"]).unwrap().render_tokens(),
            ["rate", "44100"]
        );
    }

    #[test]
    fn rejects_missing_invalid_option_and_extra_arguments() {
        assert_eq!(
            parse_rate("rate", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "rate",
                argument: "frequency",
            }
        );
        assert!(matches!(
            parse_rate("rate", &["bad"]).unwrap_err(),
            EffectCommandParseError::InvalidNumber {
                effect: "rate",
                argument: "frequency",
                ..
            }
        ));
        assert_eq!(
            parse_rate("rate", &["0"]).unwrap_err(),
            EffectCommandParseError::InvalidCoreValue {
                effect: "rate",
                argument: "frequency",
                source: auralis_core::AuralisError::InvalidSampleRate,
            }
        );
        assert_eq!(
            parse_rate("rate", &["-q"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "rate",
                option: "-q".to_owned(),
            }
        );
        assert_eq!(
            parse_rate("rate", &["24000", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "rate",
                argument: "extra".to_owned(),
            }
        );
    }
}
