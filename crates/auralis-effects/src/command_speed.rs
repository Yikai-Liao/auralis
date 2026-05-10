use crate::Speed;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f64, reject_extra_arguments,
    render_f64, required_arg,
};

pub(super) fn parse_speed(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let value = required_arg(effect, args, "factor")?;
    reject_extra_arguments(effect, args.get(1..).unwrap_or_default())?;

    let speed = if let Some(cents) = value.strip_suffix('c') {
        let cents = parse_f64(effect, "factor", cents)?;
        Speed::from_cents(cents)
    } else {
        let factor = parse_f64(effect, "factor", value)?;
        Speed::new(factor)
    };

    speed
        .map(EffectCommand::Speed)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "factor",
            source,
        })
}

pub(super) fn render_speed(speed: Speed) -> Vec<String> {
    vec!["speed".to_owned(), render_f64(speed.factor)]
}

#[cfg(test)]
mod tests {
    use super::parse_speed;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Speed};

    #[test]
    fn parses_ratio_and_cents_factor() {
        assert_eq!(
            parse_speed("speed", &["1.5"]).unwrap(),
            EffectCommand::Speed(Speed::new(1.5).unwrap())
        );

        let cents = parse_speed("speed", &["100c"]).unwrap();
        let EffectCommand::Speed(speed) = cents else {
            panic!("expected speed command");
        };
        assert!((speed.factor - 2.0_f64.powf(100.0 / 1200.0)).abs() < 1.0e-12);
    }

    #[test]
    fn renders_as_canonical_ratio() {
        assert_eq!(
            parse_speed("speed", &["1.5"]).unwrap().render_tokens(),
            ["speed", "1.5"]
        );
    }

    #[test]
    fn rejects_missing_invalid_and_extra_arguments() {
        assert_eq!(
            parse_speed("speed", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "speed",
                argument: "factor",
            }
        );
        assert_eq!(
            parse_speed("speed", &["0"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "speed",
                argument: "factor",
                source: EffectError::InvalidSpeedFactor,
            }
        );
        assert!(matches!(
            parse_speed("speed", &["bad"]).unwrap_err(),
            EffectCommandParseError::InvalidNumber {
                effect: "speed",
                argument: "factor",
                ..
            }
        ));
        assert_eq!(
            parse_speed("speed", &["1.5", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "speed",
                argument: "extra".to_owned(),
            }
        );
    }
}
