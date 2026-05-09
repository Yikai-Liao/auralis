use crate::Overdrive;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f64, reject_extra_arguments,
    render_f64,
};

pub(super) fn parse_overdrive(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let gain_db = args
        .first()
        .map(|value| parse_f64(effect, "gain", value))
        .transpose()?
        .unwrap_or(20.0);
    let color = args
        .get(1)
        .map(|value| parse_f64(effect, "color", value))
        .transpose()?
        .unwrap_or(20.0);
    reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;

    Overdrive::new(gain_db, color)
        .map(EffectCommand::Overdrive)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "overdrive",
            source,
        })
}

pub(super) fn render_overdrive(overdrive: Overdrive) -> Vec<String> {
    vec![
        "overdrive".to_owned(),
        render_f64(overdrive.gain_db),
        render_f64(overdrive.color),
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_overdrive;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Overdrive};

    #[test]
    fn parses_default_and_explicit_arguments() {
        assert_eq!(
            parse_overdrive("overdrive", &[]).unwrap(),
            EffectCommand::Overdrive(Overdrive::default())
        );
        assert_eq!(
            parse_overdrive("overdrive", &["12", "25"]).unwrap(),
            EffectCommand::Overdrive(Overdrive::new(12.0, 25.0).unwrap())
        );
        assert_eq!(
            parse_overdrive("overdrive", &[]).unwrap().render_tokens(),
            ["overdrive", "20", "20"]
        );
    }

    #[test]
    fn rejects_invalid_values_and_extra_arguments() {
        assert_eq!(
            parse_overdrive("overdrive", &["101"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "overdrive",
                argument: "overdrive",
                source: EffectError::InvalidOverdrive,
            }
        );
        assert_eq!(
            parse_overdrive("overdrive", &["20", "20", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "overdrive",
                argument: "extra".to_owned(),
            }
        );
    }
}
