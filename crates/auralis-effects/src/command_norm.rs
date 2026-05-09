use auralis_core::Decibels;

use crate::Norm;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_decibels, reject_extra_arguments,
    render_f64,
};

pub(super) fn parse_norm(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let target = match args {
        [] => Decibels::new(0.0).map_err(|source| EffectCommandParseError::InvalidCoreValue {
            effect,
            argument: "level",
            source,
        })?,
        [level] => parse_decibels(effect, "level", level)?,
        [level, rest @ ..] => {
            reject_extra_arguments(effect, rest)?;
            parse_decibels(effect, "level", level)?
        }
    };

    Ok(EffectCommand::Norm(Norm::new(target)))
}

pub(super) fn render_norm(norm: Norm) -> Vec<String> {
    vec!["norm".to_owned(), render_f64(norm.target.as_f64())]
}

#[cfg(test)]
mod tests {
    use super::parse_norm;
    use crate::{EffectCommand, EffectCommandParseError, Norm};
    use auralis_core::Decibels;

    #[test]
    fn parses_default_and_explicit_levels() {
        assert_eq!(
            parse_norm("norm", &[]).unwrap(),
            EffectCommand::Norm(Norm::new(Decibels::new(0.0).unwrap()))
        );
        assert_eq!(
            parse_norm("norm", &["-6"]).unwrap().render_tokens(),
            ["norm", "-6"]
        );
    }

    #[test]
    fn rejects_options_and_extra_arguments() {
        assert_eq!(
            parse_norm("norm", &["-b"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "norm",
                option: "-b".to_owned(),
            }
        );
        assert_eq!(
            parse_norm("norm", &["-3", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "norm",
                argument: "extra".to_owned(),
            }
        );
    }
}
