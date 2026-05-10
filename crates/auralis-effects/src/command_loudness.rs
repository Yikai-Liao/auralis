use crate::Loudness;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f32, reject_extra_arguments,
    render_f32,
};

pub(super) fn parse_loudness(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let gain_db = args
        .first()
        .map(|value| parse_f32(effect, "gain", value))
        .transpose()?
        .unwrap_or(-10.0);
    let reference_db = args
        .get(1)
        .map(|value| parse_f32(effect, "reference", value))
        .transpose()?
        .unwrap_or(65.0);
    let half_points = args
        .get(2)
        .map(|value| parse_half_points(effect, value))
        .transpose()?
        .unwrap_or(1023);
    reject_extra_arguments(effect, args.get(3..).unwrap_or_default())?;

    Loudness::new(gain_db, reference_db, half_points)
        .map(EffectCommand::Loudness)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "loudness",
            source,
        })
}

pub(super) fn render_loudness(loudness: Loudness) -> Vec<String> {
    vec![
        "loudness".to_owned(),
        render_f32(loudness.gain_db),
        render_f32(loudness.reference_db),
        loudness.half_points.to_string(),
    ]
}

fn parse_half_points(effect: &'static str, value: &str) -> CommandResult<u16> {
    let parsed =
        value
            .parse::<u16>()
            .map_err(|source| EffectCommandParseError::InvalidFrameCount {
                effect,
                argument: "n",
                value: value.to_owned(),
                source,
            })?;
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::parse_loudness;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Loudness};

    #[test]
    fn parses_defaults_and_explicit_arguments() {
        assert_eq!(
            parse_loudness("loudness", &[]).unwrap(),
            EffectCommand::Loudness(Loudness::default())
        );
        assert_eq!(
            parse_loudness("loudness", &["-6", "70", "127"]).unwrap(),
            EffectCommand::Loudness(Loudness::new(-6.0, 70.0, 127).unwrap())
        );
        assert_eq!(
            parse_loudness("loudness", &[]).unwrap().render_tokens(),
            ["loudness", "-10", "65", "1023"]
        );
    }

    #[test]
    fn rejects_invalid_values_and_extra_arguments() {
        assert_eq!(
            parse_loudness("loudness", &["-60"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "loudness",
                argument: "loudness",
                source: EffectError::InvalidLoudness,
            }
        );
        assert_eq!(
            parse_loudness("loudness", &["-10", "65", "1023", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "loudness",
                argument: "extra".to_owned(),
            }
        );
    }
}
