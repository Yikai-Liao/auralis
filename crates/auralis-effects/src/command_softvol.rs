use crate::SoftVol;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f32, reject_extra_arguments,
    render_f32,
};

pub(super) fn parse_softvol(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let volume = args
        .first()
        .map(|value| parse_f32(effect, "volume", value))
        .transpose()?
        .unwrap_or(1.0);
    let double_time_seconds = args
        .get(1)
        .map(|value| parse_f32(effect, "double-time", value))
        .transpose()?
        .unwrap_or(0.0);
    let headroom_db = args
        .get(2)
        .map(|value| parse_f32(effect, "headroom", value))
        .transpose()?
        .unwrap_or(0.0);
    reject_extra_arguments(effect, args.get(3..).unwrap_or_default())?;

    SoftVol::new(volume, double_time_seconds, headroom_db)
        .map(EffectCommand::SoftVol)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "softvol",
            source,
        })
}

pub(super) fn render_softvol(softvol: SoftVol) -> Vec<String> {
    vec![
        "softvol".to_owned(),
        render_f32(softvol.volume),
        render_f32(softvol.double_time_seconds),
        render_f32(softvol.headroom_db),
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_softvol;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, SoftVol};

    #[test]
    fn parses_defaults_and_explicit_arguments() {
        assert_eq!(
            parse_softvol("softvol", &[]).unwrap(),
            EffectCommand::SoftVol(SoftVol::default())
        );
        assert_eq!(
            parse_softvol("softvol", &["2", "10", "0.1"]).unwrap(),
            EffectCommand::SoftVol(SoftVol::new(2.0, 10.0, 0.1).unwrap())
        );
        assert_eq!(
            parse_softvol("softvol", &[]).unwrap().render_tokens(),
            ["softvol", "1", "0", "0"]
        );
    }

    #[test]
    fn rejects_invalid_values_and_extra_arguments() {
        assert_eq!(
            parse_softvol("softvol", &["-1"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "softvol",
                argument: "softvol",
                source: EffectError::InvalidSoftVol,
            }
        );
        assert_eq!(
            parse_softvol("softvol", &["1", "0", "0", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "softvol",
                argument: "extra".to_owned(),
            }
        );
    }
}
