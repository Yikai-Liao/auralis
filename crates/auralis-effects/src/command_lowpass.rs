use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, reject_extra_arguments,
    render_f64, required_arg,
};
use crate::command_filter::{parse_frequency_hz, parse_width, render_width};
use crate::{LowPass, LowPassMode};

pub(super) fn parse_lowpass(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    match args.first().copied() {
        Some("-1") => {
            let frequency_hz =
                parse_frequency_hz(effect, required_arg(effect, &args[1..], "frequency")?)?;
            reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;
            LowPass::one_pole(frequency_hz)
        }
        Some("-2") => {
            let frequency_hz =
                parse_frequency_hz(effect, required_arg(effect, &args[1..], "frequency")?)?;
            let width = args
                .get(2)
                .map(|value| parse_width(effect, value))
                .transpose()?
                .unwrap_or(LowPass::DEFAULT_WIDTH);
            reject_extra_arguments(effect, args.get(3..).unwrap_or_default())?;
            LowPass::with_width(frequency_hz, width)
        }
        Some(option) if is_option_like(option) => {
            return Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: option.to_owned(),
            });
        }
        _ => {
            let frequency_hz =
                parse_frequency_hz(effect, required_arg(effect, args, "frequency")?)?;
            let width = args
                .get(1)
                .map(|value| parse_width(effect, value))
                .transpose()?
                .unwrap_or(LowPass::DEFAULT_WIDTH);
            reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;
            LowPass::with_width(frequency_hz, width)
        }
    }
    .map(EffectCommand::LowPass)
    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "filter-design",
        source,
    })
}

pub(super) fn render_lowpass(low_pass: LowPass) -> Vec<String> {
    match low_pass.mode {
        LowPassMode::RbjTwoPole { width } => vec![
            "lowpass".to_owned(),
            render_f64(low_pass.frequency_hz),
            render_width(width),
        ],
        LowPassMode::OnePole => vec![
            "lowpass".to_owned(),
            "-1".to_owned(),
            render_f64(low_pass.frequency_hz),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::parse_lowpass;
    use crate::{BiquadWidth, EffectCommand, EffectCommandParseError, EffectError, LowPass};

    #[test]
    fn parses_default_and_explicit_two_pole_forms() {
        assert_eq!(
            parse_lowpass("lowpass", &["1k"]).unwrap(),
            EffectCommand::LowPass(LowPass::new(1_000.0).unwrap())
        );
        assert_eq!(
            parse_lowpass("lowpass", &["-2", "1000", "0.5k"])
                .unwrap()
                .render_tokens(),
            ["lowpass", "1000", "0.5k"]
        );
    }

    #[test]
    fn parses_one_pole_form() {
        assert_eq!(
            parse_lowpass("lowpass", &["-1", "500"])
                .unwrap()
                .render_tokens(),
            ["lowpass", "-1", "500"]
        );
    }

    #[test]
    fn parses_width_suffixes_and_hertz_default() {
        assert_eq!(
            parse_lowpass("lowpass", &["1000", "500"])
                .unwrap()
                .render_tokens(),
            ["lowpass", "1000", "500h"]
        );
        assert_eq!(
            parse_lowpass("lowpass", &["1000", "2q"])
                .unwrap()
                .render_tokens(),
            ["lowpass", "1000", "2q"]
        );
        assert_eq!(
            parse_lowpass("lowpass", &["1000", "1o"])
                .unwrap()
                .render_tokens(),
            ["lowpass", "1000", "1o"]
        );
    }

    #[test]
    fn rejects_bad_command_shapes_and_designs() {
        assert_eq!(
            parse_lowpass("lowpass", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "lowpass",
                argument: "frequency",
            }
        );
        assert_eq!(
            parse_lowpass("lowpass", &["-x", "1000"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "lowpass",
                option: "-x".to_owned(),
            }
        );
        assert_eq!(
            parse_lowpass("lowpass", &["1000", "1s"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "lowpass",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_lowpass("lowpass", &["0", "1q"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "lowpass",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_lowpass("lowpass", &["-1", "1000", "1q"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "lowpass",
                argument: "1q".to_owned(),
            }
        );
        assert_eq!(
            parse_lowpass("lowpass", &["1000", "500", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "lowpass",
                argument: "extra".to_owned(),
            }
        );
    }

    #[test]
    fn parsed_configs_match_typed_constructors() {
        assert_eq!(
            parse_lowpass("lowpass", &["1000", "2q"]).unwrap(),
            EffectCommand::LowPass(LowPass::with_width(1_000.0, BiquadWidth::q(2.0)).unwrap())
        );
    }
}
