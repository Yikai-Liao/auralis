use crate::Contrast;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f64, reject_extra_arguments,
    render_f64,
};

pub(super) fn parse_contrast(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let amount = args
        .first()
        .map(|value| parse_f64(effect, "amount", value))
        .transpose()?;
    reject_extra_arguments(effect, args.get(1..).unwrap_or_default())?;

    let contrast = match amount {
        Some(amount) => Contrast::new(amount).map_err(|source| {
            EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "amount",
                source,
            }
        })?,
        None => Contrast::default_amount(),
    };

    Ok(EffectCommand::Contrast(contrast))
}

pub(super) fn render_contrast(contrast: Contrast) -> Vec<String> {
    vec!["contrast".to_owned(), render_f64(contrast.amount)]
}

#[cfg(test)]
mod tests {
    use super::parse_contrast;
    use crate::{Contrast, EffectCommand, EffectCommandParseError, EffectError};

    #[test]
    fn parses_default_and_explicit_amount() {
        assert_eq!(
            parse_contrast("contrast", &[]).unwrap(),
            EffectCommand::Contrast(Contrast::default_amount())
        );
        assert_eq!(
            parse_contrast("contrast", &["25"]).unwrap(),
            EffectCommand::Contrast(Contrast::new(25.0).unwrap())
        );
        assert_eq!(
            parse_contrast("contrast", &[]).unwrap().render_tokens(),
            ["contrast", "75"]
        );
    }

    #[test]
    fn rejects_invalid_amounts_and_extra_arguments() {
        assert_eq!(
            parse_contrast("contrast", &["101"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "contrast",
                argument: "amount",
                source: EffectError::InvalidContrastAmount,
            }
        );
        assert_eq!(
            parse_contrast("contrast", &["75", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "contrast",
                argument: "extra".to_owned(),
            }
        );
    }
}
