use crate::Tremolo;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f64, reject_extra_arguments,
    render_f64, required_arg,
};

pub(super) fn parse_tremolo(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let speed_hz = parse_f64(effect, "speed", required_arg(effect, args, "speed")?)?;
    let depth_percent = args
        .get(1)
        .map(|value| parse_f64(effect, "depth", value))
        .transpose()?
        .unwrap_or(40.0);
    reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;

    Tremolo::new(speed_hz, depth_percent)
        .map(EffectCommand::Tremolo)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "tremolo",
            source,
        })
}

pub(super) fn render_tremolo(tremolo: Tremolo) -> Vec<String> {
    vec![
        "tremolo".to_owned(),
        render_f64(tremolo.speed_hz),
        render_f64(tremolo.depth_percent),
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_tremolo;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Tremolo};

    #[test]
    fn parses_default_and_explicit_depth() {
        assert_eq!(
            parse_tremolo("tremolo", &["5"]).unwrap(),
            EffectCommand::Tremolo(Tremolo::with_default_depth(5.0).unwrap())
        );
        assert_eq!(
            parse_tremolo("tremolo", &["5", "75"]).unwrap(),
            EffectCommand::Tremolo(Tremolo::new(5.0, 75.0).unwrap())
        );
        assert_eq!(
            parse_tremolo("tremolo", &["5"]).unwrap().render_tokens(),
            ["tremolo", "5", "40"]
        );
    }

    #[test]
    fn rejects_invalid_values_and_extra_arguments() {
        assert_eq!(
            parse_tremolo("tremolo", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "tremolo",
                argument: "speed",
            }
        );
        assert_eq!(
            parse_tremolo("tremolo", &["5", "0"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "tremolo",
                argument: "tremolo",
                source: EffectError::InvalidTremolo,
            }
        );
        assert_eq!(
            parse_tremolo("tremolo", &["5", "40", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "tremolo",
                argument: "extra".to_owned(),
            }
        );
    }
}
