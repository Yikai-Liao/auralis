use crate::Equalizer;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, parse_f64,
    reject_extra_arguments, render_f64, required_arg,
};
use crate::command_filter::{parse_frequency_hz, parse_width, render_width};

pub(super) fn parse_equalizer(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    if let Some(option) = args.first().copied().filter(|arg| is_option_like(arg)) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: option.to_owned(),
        });
    }

    let frequency_hz = parse_frequency_hz(effect, required_arg(effect, args, "frequency")?)?;
    let width = parse_width(effect, required_arg(effect, &args[1..], "width")?)?;
    let gain_db = parse_f64(effect, "gain", required_arg(effect, &args[2..], "gain")?)?;
    reject_extra_arguments(effect, args.get(3..).unwrap_or_default())?;

    Equalizer::new(frequency_hz, width, gain_db)
        .map(EffectCommand::Equalizer)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "filter-design",
            source,
        })
}

pub(super) fn render_equalizer(equalizer: Equalizer) -> Vec<String> {
    vec![
        "equalizer".to_owned(),
        render_f64(equalizer.frequency_hz),
        render_width(equalizer.width),
        render_f64(equalizer.gain_db),
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_equalizer;
    use crate::{BiquadWidth, EffectCommand, EffectCommandParseError, EffectError, Equalizer};

    #[test]
    fn parses_frequency_width_and_gain_forms() {
        assert_eq!(
            parse_equalizer("equalizer", &["1k", "0.707q", "+6"]).unwrap(),
            EffectCommand::Equalizer(Equalizer::new(1_000.0, BiquadWidth::q(0.707), 6.0).unwrap())
        );
        assert_eq!(
            parse_equalizer("equalizer", &["1000", "500", "-3"])
                .unwrap()
                .render_tokens(),
            ["equalizer", "1000", "500h", "-3"]
        );
        assert_eq!(
            parse_equalizer("equalizer", &["1000", "1o", "6"])
                .unwrap()
                .render_tokens(),
            ["equalizer", "1000", "1o", "6"]
        );
    }

    #[test]
    fn rejects_bad_command_shapes_and_designs() {
        assert_eq!(
            parse_equalizer("equalizer", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "equalizer",
                argument: "frequency",
            }
        );
        assert_eq!(
            parse_equalizer("equalizer", &["1000"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "equalizer",
                argument: "width",
            }
        );
        assert_eq!(
            parse_equalizer("equalizer", &["1000", "1q"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "equalizer",
                argument: "gain",
            }
        );
        assert_eq!(
            parse_equalizer("equalizer", &["-x", "1000", "1q", "6"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "equalizer",
                option: "-x".to_owned(),
            }
        );
        assert_eq!(
            parse_equalizer("equalizer", &["1000", "1s", "6"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "equalizer",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_equalizer("equalizer", &["0", "1q", "6"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "equalizer",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_equalizer("equalizer", &["1000", "1q", "6", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "equalizer",
                argument: "extra".to_owned(),
            }
        );
    }
}
