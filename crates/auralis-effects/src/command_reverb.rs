use crate::{
    Reverb,
    command::{CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64},
};

pub(super) fn parse_reverb(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut index = 0;
    let mut wet_only = false;

    if matches!(args.get(index), Some(&"-w" | &"--wet-only")) {
        wet_only = true;
        index += 1;
    }

    let reverberance = parse_optional_number(effect, args, &mut index, "reverberance", 50.0)?;
    let hf_damping = parse_optional_number(effect, args, &mut index, "HF-damping", 50.0)?;
    let room_scale = parse_optional_number(effect, args, &mut index, "room-scale", 100.0)?;
    let stereo_depth = parse_optional_number(effect, args, &mut index, "stereo-depth", 100.0)?;
    let pre_delay = parse_optional_number(effect, args, &mut index, "pre-delay", 0.0)?;
    let wet_gain = parse_optional_number(effect, args, &mut index, "wet-gain", 0.0)?;

    if let Some(argument) = args.get(index) {
        return Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: (*argument).to_owned(),
        });
    }

    Ok(EffectCommand::Reverb(
        Reverb::new(
            wet_only,
            reverberance,
            hf_damping,
            room_scale,
            stereo_depth,
            pre_delay,
            wet_gain,
        )
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "reverb",
            source,
        })?,
    ))
}

pub(super) fn render_reverb(reverb: Reverb) -> Vec<String> {
    let mut tokens = vec!["reverb".to_owned()];
    if reverb.wet_only() {
        tokens.push("-w".to_owned());
    }
    tokens.push(render_f64(reverb.reverberance_percent()));
    tokens.push(render_f64(reverb.hf_damping_percent()));
    tokens.push(render_f64(reverb.room_scale_percent()));
    tokens.push(render_f64(reverb.stereo_depth_percent()));
    tokens.push(render_f64(reverb.pre_delay_ms()));
    tokens.push(render_f64(reverb.wet_gain_db()));
    tokens
}

fn parse_optional_number(
    effect: &'static str,
    args: &[&str],
    index: &mut usize,
    argument: &'static str,
    default: f64,
) -> CommandResult<f64> {
    let Some(value) = args.get(*index).copied() else {
        return Ok(default);
    };
    if value.parse::<f64>().is_err() {
        return Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: value.to_owned(),
        });
    }

    *index += 1;
    parse_f64(effect, argument, value)
}

#[cfg(test)]
mod tests {
    use super::parse_reverb;
    use crate::{EffectCommand, Reverb};

    #[test]
    fn parses_bare_defaults() {
        assert_eq!(
            parse_reverb("reverb", &[]).unwrap(),
            EffectCommand::Reverb(Reverb::with_defaults().unwrap())
        );
    }

    #[test]
    fn parses_wet_only_and_all_numbers() {
        assert_eq!(
            parse_reverb("reverb", &["-w", "75", "25", "50", "0", "10", "-3"]).unwrap(),
            EffectCommand::Reverb(Reverb::new(true, 75.0, 25.0, 50.0, 0.0, 10.0, -3.0).unwrap())
        );
    }

    #[test]
    fn render_reverb_uses_canonical_tokens() {
        assert_eq!(
            parse_reverb("reverb", &["--wet-only", "75"])
                .unwrap()
                .render_tokens(),
            ["reverb", "-w", "75", "50", "100", "100", "0", "0"]
        );
    }
}
