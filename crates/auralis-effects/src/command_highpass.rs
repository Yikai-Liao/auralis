use crate::command::{
    is_option_like, reject_extra_arguments, render_f64, required_arg, CommandResult, EffectCommand,
    EffectCommandParseError,
};
use crate::command_filter::{parse_frequency_hz, parse_width, render_width};
use crate::{HighPass, HighPassMode};

pub(super) fn parse_highpass(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let named_args;
    let positional_args;
    let args = if args.iter().any(|arg| arg.contains('=')) {
        named_args = named_highpass_args(effect, args)?;
        positional_args = named_args.iter().map(String::as_str).collect::<Vec<_>>();
        positional_args.as_slice()
    } else {
        args
    };

    match args.first().copied() {
        Some("-1") => {
            let frequency_hz =
                parse_frequency_hz(effect, required_arg(effect, &args[1..], "frequency")?)?;
            reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;
            HighPass::one_pole(frequency_hz)
        }
        Some("-2") => {
            let frequency_hz =
                parse_frequency_hz(effect, required_arg(effect, &args[1..], "frequency")?)?;
            let width = args
                .get(2)
                .map(|value| parse_width(effect, value))
                .transpose()?
                .unwrap_or(HighPass::DEFAULT_WIDTH);
            reject_extra_arguments(effect, args.get(3..).unwrap_or_default())?;
            HighPass::with_width(frequency_hz, width)
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
                .unwrap_or(HighPass::DEFAULT_WIDTH);
            reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;
            HighPass::with_width(frequency_hz, width)
        }
    }
    .map(EffectCommand::HighPass)
    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "filter-design",
        source,
    })
}

pub(super) fn render_highpass(high_pass: HighPass) -> Vec<String> {
    match high_pass.mode {
        HighPassMode::RbjTwoPole { width } => vec![
            "highpass".to_owned(),
            render_f64(high_pass.frequency_hz),
            render_width(width),
        ],
        HighPassMode::OnePole => vec![
            "highpass".to_owned(),
            "-1".to_owned(),
            render_f64(high_pass.frequency_hz),
        ],
    }
}

fn named_highpass_args(effect: &'static str, args: &[&str]) -> CommandResult<Vec<String>> {
    let mut cutoff = None;
    let mut q = None;

    for arg in args {
        let Some((name, value)) = arg.split_once('=') else {
            return Err(EffectCommandParseError::UnexpectedArgument {
                effect,
                argument: (*arg).to_owned(),
            });
        };
        match name {
            "cutoff" => cutoff = Some(value.to_owned()),
            "q" => q = Some(format!("{value}q")),
            _ => {
                return Err(EffectCommandParseError::UnexpectedArgument {
                    effect,
                    argument: (*arg).to_owned(),
                });
            }
        }
    }

    let cutoff = cutoff.ok_or(EffectCommandParseError::MissingArgument {
        effect,
        argument: "cutoff",
    })?;
    let mut parsed = vec![cutoff];
    if let Some(q) = q {
        parsed.push(q);
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::parse_highpass;
    use crate::{BiquadWidth, EffectCommand, EffectCommandParseError, EffectError, HighPass};

    #[test]
    fn parses_default_and_explicit_two_pole_forms() {
        assert_eq!(
            parse_highpass("highpass", &["1k"]).unwrap(),
            EffectCommand::HighPass(HighPass::new(1_000.0).unwrap())
        );
        assert_eq!(
            parse_highpass("highpass", &["-2", "1000", "0.5k"])
                .unwrap()
                .render_tokens(),
            ["highpass", "1000", "0.5k"]
        );
    }

    #[test]
    fn parses_one_pole_form() {
        assert_eq!(
            parse_highpass("highpass", &["-1", "500"])
                .unwrap()
                .render_tokens(),
            ["highpass", "-1", "500"]
        );
    }

    #[test]
    fn parses_width_suffixes_and_hertz_default() {
        assert_eq!(
            parse_highpass("highpass", &["1000", "500"])
                .unwrap()
                .render_tokens(),
            ["highpass", "1000", "500h"]
        );
        assert_eq!(
            parse_highpass("highpass", &["1000", "2q"])
                .unwrap()
                .render_tokens(),
            ["highpass", "1000", "2q"]
        );
        assert_eq!(
            parse_highpass("highpass", &["1000", "1o"])
                .unwrap()
                .render_tokens(),
            ["highpass", "1000", "1o"]
        );
    }

    #[test]
    fn parses_documented_named_highpass_form() {
        assert_eq!(
            parse_highpass("highpass", &["cutoff=1000Hz", "q=0.707"])
                .unwrap()
                .render_tokens(),
            ["highpass", "1000", "0.707q"]
        );
    }

    #[test]
    fn rejects_bad_command_shapes_and_designs() {
        assert_eq!(
            parse_highpass("highpass", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "highpass",
                argument: "frequency",
            }
        );
        assert_eq!(
            parse_highpass("highpass", &["-x", "1000"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "highpass",
                option: "-x".to_owned(),
            }
        );
        assert_eq!(
            parse_highpass("highpass", &["1000", "1s"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "highpass",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_highpass("highpass", &["0", "1q"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "highpass",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_highpass("highpass", &["-1", "1000", "1q"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "highpass",
                argument: "1q".to_owned(),
            }
        );
        assert_eq!(
            parse_highpass("highpass", &["1000", "500", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "highpass",
                argument: "extra".to_owned(),
            }
        );
    }

    #[test]
    fn parsed_configs_match_typed_constructors() {
        assert_eq!(
            parse_highpass("highpass", &["1000", "2q"]).unwrap(),
            EffectCommand::HighPass(HighPass::with_width(1_000.0, BiquadWidth::q(2.0)).unwrap())
        );
    }
}
