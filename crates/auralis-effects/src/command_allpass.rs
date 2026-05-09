use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, parse_f64,
    reject_extra_arguments, render_f64, required_arg,
};
use crate::{AllPass, AllPassMode, BiquadWidth};

pub(super) fn parse_allpass(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    match args.first().copied() {
        Some("-1") => {
            let frequency_hz =
                parse_frequency_hz(effect, required_arg(effect, &args[1..], "frequency")?)?;
            reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;
            AllPass::one_pole(frequency_hz)
        }
        Some("-2") => {
            let frequency_hz =
                parse_frequency_hz(effect, required_arg(effect, &args[1..], "frequency")?)?;
            reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;
            AllPass::two_pole(frequency_hz)
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
            let width = parse_width(effect, required_arg(effect, &args[1..], "width")?)?;
            reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;
            AllPass::new(frequency_hz, width)
        }
    }
    .map(EffectCommand::AllPass)
    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "filter-design",
        source,
    })
}

pub(super) fn render_allpass(all_pass: AllPass) -> Vec<String> {
    match all_pass.mode {
        AllPassMode::RbjTwoPole { width } => vec![
            "allpass".to_owned(),
            render_f64(all_pass.frequency_hz),
            render_width(width),
        ],
        AllPassMode::OnePole => vec![
            "allpass".to_owned(),
            "-1".to_owned(),
            render_f64(all_pass.frequency_hz),
        ],
        AllPassMode::TwoPole => vec![
            "allpass".to_owned(),
            "-2".to_owned(),
            render_f64(all_pass.frequency_hz),
        ],
    }
}

fn parse_frequency_hz(effect: &'static str, value: &str) -> CommandResult<f64> {
    if let Some(kilohertz) = value.strip_suffix(['k', 'K']) {
        Ok(parse_f64(effect, "frequency", kilohertz)? * 1000.0)
    } else {
        parse_f64(effect, "frequency", value)
    }
}

fn parse_width(effect: &'static str, value: &str) -> CommandResult<BiquadWidth> {
    if is_option_like(value) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: value.to_owned(),
        });
    }

    let (number, suffix) = split_width_suffix(value);
    let width = parse_f64(effect, "width", number)?;

    match suffix.unwrap_or('h') {
        'h' => Ok(BiquadWidth::hertz(width)),
        'k' => Ok(BiquadWidth::kilohertz(width)),
        'q' => Ok(BiquadWidth::q(width)),
        'o' => Ok(BiquadWidth::octaves(width)),
        _ => Err(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "width",
            source: crate::EffectError::InvalidBiquadDesign,
        }),
    }
}

fn split_width_suffix(value: &str) -> (&str, Option<char>) {
    let Some(suffix) = value.chars().last().filter(char::is_ascii_alphabetic) else {
        return (value, None);
    };

    (&value[..value.len() - suffix.len_utf8()], Some(suffix))
}

fn render_width(width: BiquadWidth) -> String {
    match width {
        BiquadWidth::Hertz(value) => format!("{}h", render_f64(value)),
        BiquadWidth::Kilohertz(value) => format!("{}k", render_f64(value)),
        BiquadWidth::Q(value) => format!("{}q", render_f64(value)),
        BiquadWidth::Octaves(value) => format!("{}o", render_f64(value)),
        BiquadWidth::Slope(value) => format!("{}s", render_f64(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_allpass;
    use crate::{AllPass, BiquadWidth, EffectCommand, EffectCommandParseError, EffectError};

    #[test]
    fn parses_default_two_pole_with_explicit_width() {
        let command = parse_allpass("allpass", &["1k", "0.707q"]).unwrap();

        assert_eq!(
            command,
            EffectCommand::AllPass(AllPass::new(1_000.0, BiquadWidth::q(0.707)).unwrap())
        );
        assert_eq!(command.render_tokens(), ["allpass", "1000", "0.707q"]);
    }

    #[test]
    fn parses_one_and_two_pole_options() {
        assert_eq!(
            parse_allpass("allpass", &["-1", "500"])
                .unwrap()
                .render_tokens(),
            ["allpass", "-1", "500"]
        );
        assert_eq!(
            parse_allpass("allpass", &["-2", "2k"])
                .unwrap()
                .render_tokens(),
            ["allpass", "-2", "2000"]
        );
    }

    #[test]
    fn parses_width_suffixes_and_hertz_default() {
        assert_eq!(
            parse_allpass("allpass", &["1000", "200"])
                .unwrap()
                .render_tokens(),
            ["allpass", "1000", "200h"]
        );
        assert_eq!(
            parse_allpass("allpass", &["1000", "0.25k"])
                .unwrap()
                .render_tokens(),
            ["allpass", "1000", "0.25k"]
        );
        assert_eq!(
            parse_allpass("allpass", &["1000", "1o"])
                .unwrap()
                .render_tokens(),
            ["allpass", "1000", "1o"]
        );
    }

    #[test]
    fn rejects_missing_width_and_invalid_options() {
        assert_eq!(
            parse_allpass("allpass", &["1000"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "allpass",
                argument: "width",
            }
        );
        assert_eq!(
            parse_allpass("allpass", &["-x", "1000"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "allpass",
                option: "-x".to_owned(),
            }
        );
        assert_eq!(
            parse_allpass("allpass", &["-1", "1000", "1q"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "allpass",
                argument: "1q".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_invalid_design_values() {
        assert_eq!(
            parse_allpass("allpass", &["1000", "1s"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "allpass",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_allpass("allpass", &["0", "1q"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "allpass",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            }
        );
    }
}
