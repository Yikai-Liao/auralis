use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, reject_extra_arguments, render_f64,
    required_arg,
};
use crate::command_filter::{parse_frequency_hz, parse_width, render_width};
use crate::{Band, BandMode};

pub(super) fn parse_band(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (mode, args) = match args.first().copied() {
        Some("-n") => (BandMode::Unpitched, &args[1..]),
        Some(option) if crate::command::is_option_like(option) => {
            return Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: option.to_owned(),
            });
        }
        _ => (BandMode::Pitched, args),
    };

    let frequency_hz = parse_frequency_hz(effect, required_arg(effect, args, "frequency")?)?;
    let width = args
        .get(1)
        .map(|value| parse_width(effect, value))
        .transpose()?;
    reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;

    match mode {
        BandMode::Pitched => Band::new(frequency_hz, width),
        BandMode::Unpitched => Band::unpitched(frequency_hz, width),
    }
    .map(EffectCommand::Band)
    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "filter-design",
        source,
    })
}

pub(super) fn render_band(band: Band) -> Vec<String> {
    let mut tokens = vec!["band".to_owned()];
    if band.mode == BandMode::Unpitched {
        tokens.push("-n".to_owned());
    }
    tokens.push(render_f64(band.frequency_hz));
    if let Some(width) = band.width {
        tokens.push(render_width(width));
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::parse_band;
    use crate::{Band, BiquadWidth, EffectCommand, EffectCommandParseError, EffectError};

    #[test]
    fn parses_default_and_unpitched_forms() {
        assert_eq!(
            parse_band("band", &["1k"]).unwrap(),
            EffectCommand::Band(Band::new(1_000.0, None).unwrap())
        );
        assert_eq!(
            parse_band("band", &["-n", "1000", "0.5k"])
                .unwrap()
                .render_tokens(),
            ["band", "-n", "1000", "0.5k"]
        );
    }

    #[test]
    fn parses_width_suffixes_and_hertz_default() {
        assert_eq!(
            parse_band("band", &["1000", "500"])
                .unwrap()
                .render_tokens(),
            ["band", "1000", "500h"]
        );
        assert_eq!(
            parse_band("band", &["1000", "2q"]).unwrap().render_tokens(),
            ["band", "1000", "2q"]
        );
        assert_eq!(
            parse_band("band", &["1000", "1o"]).unwrap().render_tokens(),
            ["band", "1000", "1o"]
        );
    }

    #[test]
    fn rejects_bad_command_shapes_and_designs() {
        assert_eq!(
            parse_band("band", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "band",
                argument: "frequency",
            }
        );
        assert_eq!(
            parse_band("band", &["-x", "1000"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "band",
                option: "-x".to_owned(),
            }
        );
        assert_eq!(
            parse_band("band", &["1000", "1s"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "band",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_band("band", &["0", "500"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "band",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_band("band", &["1000", "500", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "band",
                argument: "extra".to_owned(),
            }
        );
    }

    #[test]
    fn parsed_configs_match_typed_constructors() {
        assert_eq!(
            parse_band("band", &["1000", "2q"]).unwrap(),
            EffectCommand::Band(Band::new(1_000.0, Some(BiquadWidth::q(2.0))).unwrap())
        );
    }
}
