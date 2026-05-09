use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, reject_extra_arguments,
    render_f64, required_arg,
};
use crate::command_filter::{parse_frequency_hz, parse_width, render_width};
use crate::{BandPass, BandPassMode};

pub(super) fn parse_bandpass(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (mode, args) = match args.first().copied() {
        Some("-c") => (BandPassMode::ConstantSkirt, &args[1..]),
        Some(option) if is_option_like(option) => {
            return Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: option.to_owned(),
            });
        }
        _ => (BandPassMode::ConstantPeak, args),
    };

    let frequency_hz = parse_frequency_hz(effect, required_arg(effect, args, "frequency")?)?;
    let width = parse_width(effect, required_arg(effect, &args[1..], "width")?)?;
    reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;

    match mode {
        BandPassMode::ConstantPeak => BandPass::new(frequency_hz, width),
        BandPassMode::ConstantSkirt => BandPass::constant_skirt(frequency_hz, width),
    }
    .map(EffectCommand::BandPass)
    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "filter-design",
        source,
    })
}

pub(super) fn render_bandpass(band_pass: BandPass) -> Vec<String> {
    let mut tokens = vec!["bandpass".to_owned()];
    if band_pass.mode == BandPassMode::ConstantSkirt {
        tokens.push("-c".to_owned());
    }
    tokens.push(render_f64(band_pass.frequency_hz));
    tokens.push(render_width(band_pass.width));
    tokens
}

#[cfg(test)]
mod tests {
    use super::parse_bandpass;
    use crate::{BandPass, BiquadWidth, EffectCommand, EffectCommandParseError, EffectError};

    #[test]
    fn parses_default_and_constant_skirt_forms() {
        assert_eq!(
            parse_bandpass("bandpass", &["1k", "0.707q"]).unwrap(),
            EffectCommand::BandPass(BandPass::new(1_000.0, BiquadWidth::q(0.707)).unwrap())
        );
        assert_eq!(
            parse_bandpass("bandpass", &["-c", "1000", "0.5k"])
                .unwrap()
                .render_tokens(),
            ["bandpass", "-c", "1000", "0.5k"]
        );
    }

    #[test]
    fn parses_width_suffixes_and_hertz_default() {
        assert_eq!(
            parse_bandpass("bandpass", &["1000", "500"])
                .unwrap()
                .render_tokens(),
            ["bandpass", "1000", "500h"]
        );
        assert_eq!(
            parse_bandpass("bandpass", &["1000", "1o"])
                .unwrap()
                .render_tokens(),
            ["bandpass", "1000", "1o"]
        );
    }

    #[test]
    fn rejects_bad_command_shapes_and_designs() {
        assert_eq!(
            parse_bandpass("bandpass", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "bandpass",
                argument: "frequency",
            }
        );
        assert_eq!(
            parse_bandpass("bandpass", &["1000"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "bandpass",
                argument: "width",
            }
        );
        assert_eq!(
            parse_bandpass("bandpass", &["-x", "1000", "500"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "bandpass",
                option: "-x".to_owned(),
            }
        );
        assert_eq!(
            parse_bandpass("bandpass", &["1000", "1s"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "bandpass",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_bandpass("bandpass", &["0", "500"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "bandpass",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_bandpass("bandpass", &["1000", "500", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "bandpass",
                argument: "extra".to_owned(),
            }
        );
    }
}
