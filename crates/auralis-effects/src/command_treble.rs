use crate::Treble;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f64, reject_extra_arguments,
    render_f64, required_arg,
};
use crate::command_filter::{parse_frequency_hz, parse_shelf_width, render_width};

pub(super) fn parse_treble(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let gain_db = parse_f64(effect, "gain", required_arg(effect, args, "gain")?)?;
    let frequency_hz = match args.get(1).copied() {
        Some(value) => parse_frequency_hz(effect, value)?,
        None => Treble::DEFAULT_FREQUENCY_HZ,
    };
    let width = match args.get(2).copied() {
        Some(value) => parse_shelf_width(effect, value)?,
        None => Treble::DEFAULT_WIDTH,
    };
    reject_extra_arguments(effect, args.get(3..).unwrap_or_default())?;

    Treble::with_width(gain_db, frequency_hz, width)
        .map(EffectCommand::Treble)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "filter-design",
            source,
        })
}

pub(super) fn render_treble(treble: Treble) -> Vec<String> {
    vec![
        "treble".to_owned(),
        render_f64(treble.gain_db),
        render_f64(treble.frequency_hz),
        render_width(treble.width),
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_treble;
    use crate::{BiquadWidth, EffectCommand, EffectCommandParseError, EffectError, Treble};

    #[test]
    fn parses_required_gain_and_optional_defaults() {
        assert_eq!(
            parse_treble("treble", &["+6"]).unwrap(),
            EffectCommand::Treble(Treble::new(6.0).unwrap())
        );
        assert_eq!(
            parse_treble("treble", &["-3", "3500"])
                .unwrap()
                .render_tokens(),
            ["treble", "-3", "3500", "0.5s"]
        );
        assert_eq!(
            parse_treble("treble", &["6", "3k", "0.707q"])
                .unwrap()
                .render_tokens(),
            ["treble", "6", "3000", "0.707q"]
        );
        assert_eq!(
            parse_treble("treble", &["6", "3000", "1o"])
                .unwrap()
                .render_tokens(),
            ["treble", "6", "3000", "1o"]
        );
        assert_eq!(
            parse_treble("treble", &["6", "3000", "0.5s"]).unwrap(),
            EffectCommand::Treble(
                Treble::with_width(6.0, 3000.0, BiquadWidth::slope(0.5)).unwrap()
            )
        );
    }

    #[test]
    fn rejects_bad_command_shapes_and_designs() {
        assert_eq!(
            parse_treble("treble", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "treble",
                argument: "gain",
            }
        );
        assert_eq!(
            parse_treble("treble", &["-b", "24"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "treble",
                option: "-b".to_owned(),
            }
        );
        assert_eq!(
            parse_treble("treble", &["6", "3000", "2s"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "treble",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_treble("treble", &["6", "3000", "1x"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "treble",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_treble("treble", &["6", "0"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "treble",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            }
        );
        assert_eq!(
            parse_treble("treble", &["6", "3000", "0.5s", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "treble",
                argument: "extra".to_owned(),
            }
        );
    }
}
