use crate::Tempo;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, parse_f64,
    reject_extra_arguments, render_f64, required_arg,
};

pub(super) fn parse_tempo(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let factor = required_arg(effect, args, "factor")?;
    if is_option_like(factor) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: factor.to_owned(),
        });
    }
    reject_extra_arguments(effect, args.get(1..).unwrap_or_default())?;

    Tempo::new(parse_f64(effect, "factor", factor)?)
        .map(EffectCommand::Tempo)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "factor",
            source,
        })
}

pub(super) fn render_tempo(tempo: Tempo) -> Vec<String> {
    vec!["tempo".to_owned(), render_f64(tempo.factor)]
}

#[cfg(test)]
mod tests {
    use super::parse_tempo;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Tempo};

    #[test]
    fn parses_and_renders_basic_factor() {
        assert_eq!(
            parse_tempo("tempo", &["1.25"]).unwrap(),
            EffectCommand::Tempo(Tempo::new(1.25).unwrap())
        );
        assert_eq!(
            parse_tempo("tempo", &["1.25"]).unwrap().render_tokens(),
            ["tempo", "1.25"]
        );
    }

    #[test]
    fn rejects_missing_options_invalid_values_and_extra_arguments() {
        assert_eq!(
            parse_tempo("tempo", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "tempo",
                argument: "factor",
            }
        );
        assert_eq!(
            parse_tempo("tempo", &["-q", "1.25"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "tempo",
                option: "-q".to_owned(),
            }
        );
        assert_eq!(
            parse_tempo("tempo", &["0.01"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "tempo",
                argument: "factor",
                source: EffectError::InvalidTempoFactor,
            }
        );
        assert_eq!(
            parse_tempo("tempo", &["1.25", "82"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "tempo",
                argument: "82".to_owned(),
            }
        );
    }
}
