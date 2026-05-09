use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f32, parse_f64,
    reject_extra_arguments, render_f32,
};
use crate::{Vol, VolGainType};

pub(super) fn parse_vol(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let gain = crate::command::required_arg(effect, args, "gain")?;
    let (gain, gain_type, rest) = parse_gain_and_type(effect, gain, &args[1..])?;
    let limiter_gain = rest
        .first()
        .map(|value| parse_f32(effect, "limiter-gain", value))
        .transpose()?;
    reject_extra_arguments(effect, rest.get(1..).unwrap_or_default())?;

    let mut vol = Vol::from_gain(gain_type, gain).map_err(|source| {
        EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "gain",
            source,
        }
    })?;

    if let Some(limiter_gain) = limiter_gain {
        vol = vol.with_limiter_gain(limiter_gain).map_err(|source| {
            EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "limiter-gain",
                source,
            }
        })?;
    }

    Ok(EffectCommand::Vol(vol))
}

pub(super) fn render_vol(vol: Vol) -> Vec<String> {
    let mut tokens = vec!["vol".to_owned(), render_f32(vol.gain)];
    match vol.gain_type {
        VolGainType::Amplitude => {}
        VolGainType::Power => tokens.push("power".to_owned()),
        VolGainType::Decibels => tokens.push("dB".to_owned()),
    }
    if let Some(limiter_gain) = vol.limiter_gain {
        tokens.push(render_f32(limiter_gain));
    }
    tokens
}

fn parse_gain_and_type<'args>(
    effect: &'static str,
    gain: &str,
    rest: &'args [&'args str],
) -> CommandResult<(f32, VolGainType, &'args [&'args str])> {
    if let Some((gain_text, gain_type)) = split_suffix_type(gain) {
        let gain = parse_f32(effect, "gain", gain_text)?;
        return Ok((gain, gain_type, rest));
    }

    let gain = parse_f32(effect, "gain", gain)?;
    if let Some((maybe_type, remaining)) = rest.split_first() {
        if let Some(gain_type) = parse_gain_type(maybe_type) {
            return Ok((gain, gain_type, remaining));
        }
        if parse_f64(effect, "limiter-gain", maybe_type).is_err() {
            return Err(EffectCommandParseError::UnexpectedArgument {
                effect,
                argument: (*maybe_type).to_owned(),
            });
        }
    }

    Ok((gain, VolGainType::Amplitude, rest))
}

fn split_suffix_type(value: &str) -> Option<(&str, VolGainType)> {
    for (suffix, gain_type) in [
        ("amplitude", VolGainType::Amplitude),
        ("power", VolGainType::Power),
        ("dB", VolGainType::Decibels),
        ("db", VolGainType::Decibels),
        ("a", VolGainType::Amplitude),
        ("p", VolGainType::Power),
        ("d", VolGainType::Decibels),
    ] {
        let Some(gain) = value.strip_suffix(suffix) else {
            continue;
        };
        if !gain.is_empty() {
            return Some((gain, gain_type));
        }
    }
    None
}

fn parse_gain_type(value: &str) -> Option<VolGainType> {
    match value {
        "a" | "amplitude" => Some(VolGainType::Amplitude),
        "p" | "power" => Some(VolGainType::Power),
        "d" | "dB" | "db" => Some(VolGainType::Decibels),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_vol;
    use crate::{EffectCommand, EffectCommandParseError, Vol, VolGainType};

    #[test]
    fn parses_sox_ng_vol_gain_types_and_limiter_gain() {
        assert_eq!(
            parse_vol("vol", &["0.5"]).unwrap(),
            EffectCommand::Vol(Vol::amplitude(0.5).unwrap())
        );
        assert_eq!(
            parse_vol("vol", &["0.25", "power"]).unwrap(),
            EffectCommand::Vol(Vol::power(0.25).unwrap())
        );
        assert_eq!(
            parse_vol("vol", &["-6dB"]).unwrap(),
            EffectCommand::Vol(Vol::from_gain(VolGainType::Decibels, -6.0).unwrap())
        );
        assert_eq!(
            parse_vol("vol", &["2", "amplitude", "0.05"]).unwrap(),
            EffectCommand::Vol(
                Vol::amplitude(2.0)
                    .unwrap()
                    .with_limiter_gain(0.05)
                    .unwrap()
            )
        );
    }

    #[test]
    fn renders_canonical_vol_tokens() {
        let command = parse_vol("vol", &["2a", "0.05"]).unwrap();

        assert_eq!(command.render_tokens(), ["vol", "2", "0.05"]);
    }

    #[test]
    fn rejects_unknown_type_and_invalid_limiter() {
        assert_eq!(
            parse_vol("vol", &["1", "loud"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "vol",
                argument: "loud".to_owned(),
            }
        );
        assert!(parse_vol("vol", &["0.5", "0.05"]).is_err());
    }
}
