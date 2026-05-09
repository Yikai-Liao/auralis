use crate::Bass;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f64, reject_extra_arguments,
    render_f64, required_arg,
};
use crate::command_filter::{parse_frequency_hz, parse_shelf_width, render_width};

pub(super) fn parse_bass(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let gain_db = parse_f64(effect, "gain", required_arg(effect, args, "gain")?)?;
    let frequency_hz = match args.get(1).copied() {
        Some(value) => parse_frequency_hz(effect, value)?,
        None => Bass::DEFAULT_FREQUENCY_HZ,
    };
    let width = match args.get(2).copied() {
        Some(value) => parse_shelf_width(effect, value)?,
        None => Bass::DEFAULT_WIDTH,
    };
    reject_extra_arguments(effect, args.get(3..).unwrap_or_default())?;

    Bass::with_width(gain_db, frequency_hz, width)
        .map(EffectCommand::Bass)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "filter-design",
            source,
        })
}

pub(super) fn render_bass(bass: Bass) -> Vec<String> {
    vec![
        "bass".to_owned(),
        render_f64(bass.gain_db),
        render_f64(bass.frequency_hz),
        render_width(bass.width),
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_bass;
    use crate::{Bass, BiquadWidth, EffectCommand, EffectCommandParseError, EffectError};

    #[test]
    fn parses_required_gain_and_optional_defaults() {
        assert_eq!(
            parse_bass("bass", &["+6"]).unwrap(),
            EffectCommand::Bass(Bass::new(6.0).unwrap())
        );
        assert_eq!(
            parse_bass("bass", &["-3", "150"]).unwrap().render_tokens(),
            ["bass", "-3", "150", "0.5s"]
        );
        assert_eq!(
            parse_bass("bass", &["6", "1k", "0.707q"])
                .unwrap()
                .render_tokens(),
            ["bass", "6", "1000", "0.707q"]
        );
        assert_eq!(
            parse_bass("bass", &["6", "100", "1o"])
                .unwrap()
                .render_tokens(),
            ["bass", "6", "100", "1o"]
        );
        assert_eq!(
            parse_bass("bass", &["6", "100", "0.5s"]).unwrap(),
            EffectCommand::Bass(Bass::with_width(6.0, 100.0, BiquadWidth::slope(0.5)).unwrap())
        );
    }

    #[test]
    fn rejects_bad_command_shapes_and_designs() {
        assert_eq!(
            parse_bass("bass", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "bass",
                argument: "gain",
            }
        );
        assert_eq!(
            parse_bass("bass", &["-b", "24"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "bass",
                option: "-b".to_owned(),
            }
        );
        assert_eq!(
            parse_bass("bass", &["6", "100", "2s"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "bass",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_bass("bass", &["6", "100", "1x"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "bass",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_bass("bass", &["6", "0"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "bass",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_bass("bass", &["6", "100", "0.5s", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "bass",
                argument: "extra".to_owned(),
            }
        );
    }
}
