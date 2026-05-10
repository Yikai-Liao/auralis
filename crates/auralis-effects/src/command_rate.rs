use auralis_core::SampleRate;

use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, reject_extra_arguments,
    required_arg,
};
use crate::{Rate, RateQuality};

pub(super) fn parse_rate(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (quality, args) = parse_quality(effect, args)?;
    let value = required_arg(effect, args, "frequency")?;
    if is_option_like(value) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: value.to_owned(),
        });
    }
    reject_extra_arguments(effect, args.get(1..).unwrap_or_default())?;

    parse_sample_rate(effect, value)
        .map(|sample_rate| EffectCommand::Rate(Rate::with_quality(sample_rate, quality)))
}

pub(super) fn render_rate(rate: Rate) -> Vec<String> {
    let mut tokens = vec!["rate".to_owned()];
    if let Some(option) = rate.quality.command_option() {
        tokens.push(option.to_owned());
    }
    tokens.push(rate.target_sample_rate.as_u32().to_string());
    tokens
}

fn parse_quality<'a>(
    effect: &'static str,
    args: &'a [&'a str],
) -> CommandResult<(RateQuality, &'a [&'a str])> {
    match args {
        ["-q", rest @ ..] => Ok((RateQuality::Quick, rest)),
        ["-l", rest @ ..] => Ok((RateQuality::Low, rest)),
        ["-Q", value, rest @ ..] => match *value {
            "0" => Ok((RateQuality::Quick, rest)),
            "1" => Ok((RateQuality::Low, rest)),
            _ => Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: format!("-Q {value}"),
            }),
        },
        [value, ..] if matches!(*value, "-m" | "-g" | "-h" | "-e" | "-v" | "-u") => {
            Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: (*value).to_owned(),
            })
        }
        _ => Ok((RateQuality::Default, args)),
    }
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

impl RateQuality {
    fn command_option(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::Quick => Some("-q"),
            Self::Low => Some("-l"),
        }
    }
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
    fn parses_quick_and_low_quality_modes() {
        assert_eq!(
            parse_rate("rate", &["-q", "24000"]).unwrap(),
            EffectCommand::Rate(Rate::quick(SampleRate::new(24_000).unwrap()))
        );
        assert_eq!(
            parse_rate("rate", &["-Q", "0", "24k"])
                .unwrap()
                .render_tokens(),
            ["rate", "-q", "24000"]
        );
        assert_eq!(
            parse_rate("rate", &["-l", "48000"]).unwrap(),
            EffectCommand::Rate(Rate::low(SampleRate::new(48_000).unwrap()))
        );
        assert_eq!(
            parse_rate("rate", &["-Q", "1", "48k"])
                .unwrap()
                .render_tokens(),
            ["rate", "-l", "48000"]
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
            EffectCommandParseError::MissingArgument {
                effect: "rate",
                argument: "frequency",
            }
        );
        assert_eq!(
            parse_rate("rate", &["-Q", "4", "24000"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "rate",
                option: "-Q 4".to_owned(),
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
