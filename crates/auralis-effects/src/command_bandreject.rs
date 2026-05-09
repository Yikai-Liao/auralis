use crate::BandReject;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, reject_extra_arguments,
    required_arg,
};
use crate::command_filter::{parse_frequency_hz, parse_width, render_width};

pub(super) fn parse_bandreject(
    effect: &'static str,
    args: &[&str],
) -> CommandResult<EffectCommand> {
    if let Some(option) = args.first().copied().filter(|arg| is_option_like(arg)) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: option.to_owned(),
        });
    }

    let frequency_hz = parse_frequency_hz(effect, required_arg(effect, args, "frequency")?)?;
    let width = parse_width(effect, required_arg(effect, &args[1..], "width")?)?;
    reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;

    BandReject::new(frequency_hz, width)
        .map(EffectCommand::BandReject)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "filter-design",
            source,
        })
}

pub(super) fn render_bandreject(band_reject: BandReject) -> Vec<String> {
    vec![
        "bandreject".to_owned(),
        crate::command::render_f64(band_reject.frequency_hz),
        render_width(band_reject.width),
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_bandreject;
    use crate::{BandReject, BiquadWidth, EffectCommand, EffectCommandParseError, EffectError};

    #[test]
    fn parses_frequency_and_width_forms() {
        assert_eq!(
            parse_bandreject("bandreject", &["1k", "0.707q"]).unwrap(),
            EffectCommand::BandReject(BandReject::new(1_000.0, BiquadWidth::q(0.707)).unwrap())
        );
        assert_eq!(
            parse_bandreject("bandreject", &["1000", "500"])
                .unwrap()
                .render_tokens(),
            ["bandreject", "1000", "500h"]
        );
        assert_eq!(
            parse_bandreject("bandreject", &["1000", "1o"])
                .unwrap()
                .render_tokens(),
            ["bandreject", "1000", "1o"]
        );
    }

    #[test]
    fn rejects_bad_command_shapes_and_designs() {
        assert_eq!(
            parse_bandreject("bandreject", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "bandreject",
                argument: "frequency",
            }
        );
        assert_eq!(
            parse_bandreject("bandreject", &["1000"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "bandreject",
                argument: "width",
            }
        );
        assert_eq!(
            parse_bandreject("bandreject", &["-x", "1000", "500"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "bandreject",
                option: "-x".to_owned(),
            }
        );
        assert_eq!(
            parse_bandreject("bandreject", &["1000", "1s"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "bandreject",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_bandreject("bandreject", &["0", "500"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "bandreject",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_bandreject("bandreject", &["1000", "500", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "bandreject",
                argument: "extra".to_owned(),
            }
        );
    }
}
