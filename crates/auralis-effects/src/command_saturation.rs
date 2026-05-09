use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, parse_f32,
    reject_extra_arguments, render_f32,
};
use crate::{Saturation, SaturationType};

pub(super) fn parse_saturation(
    effect: &'static str,
    args: &[&str],
) -> CommandResult<EffectCommand> {
    let Some((type_arg, rest)) = args.split_first() else {
        return Ok(EffectCommand::Saturation(Saturation::default()));
    };
    if is_option_like(type_arg) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: (*type_arg).to_owned(),
        });
    }

    let saturation_type = parse_saturation_type(effect, type_arg)?;
    let blend = rest
        .first()
        .map(|value| parse_f32(effect, "blend", value))
        .transpose()?
        .unwrap_or(1.0);
    let offset = rest
        .get(1)
        .map(|value| parse_f32(effect, "offset", value))
        .transpose()?
        .unwrap_or(0.0);
    let parameter_name = parameter_name(saturation_type);
    let parameter = rest
        .get(2)
        .map(|value| parse_f32(effect, parameter_name, value))
        .transpose()?
        .unwrap_or_else(|| default_parameter(saturation_type));
    reject_extra_arguments(effect, rest.get(3..).unwrap_or_default())?;

    Saturation::new(saturation_type, blend, offset, parameter)
        .map(EffectCommand::Saturation)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "saturation",
            source,
        })
}

pub(super) fn render_saturation(saturation: Saturation) -> Vec<String> {
    vec![
        "saturation".to_owned(),
        render_saturation_type(saturation.saturation_type).to_owned(),
        render_f32(saturation.blend),
        render_f32(saturation.offset),
        render_f32(saturation.parameter),
    ]
}

fn parse_saturation_type(effect: &'static str, value: &str) -> CommandResult<SaturationType> {
    match value {
        "tanh" => Ok(SaturationType::Tanh),
        "sqrt" => Ok(SaturationType::Sqrt),
        "diode" => Ok(SaturationType::Diode),
        _ => Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: value.to_owned(),
        }),
    }
}

fn render_saturation_type(saturation_type: SaturationType) -> &'static str {
    match saturation_type {
        SaturationType::Tanh => "tanh",
        SaturationType::Sqrt => "sqrt",
        SaturationType::Diode => "diode",
    }
}

fn parameter_name(saturation_type: SaturationType) -> &'static str {
    match saturation_type {
        SaturationType::Tanh => "drive",
        SaturationType::Sqrt => "color",
        SaturationType::Diode => "threshold",
    }
}

fn default_parameter(saturation_type: SaturationType) -> f32 {
    match saturation_type {
        SaturationType::Tanh => 1.0,
        SaturationType::Sqrt | SaturationType::Diode => 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_saturation;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Saturation, SaturationType};

    #[test]
    fn parses_default_and_explicit_arguments() {
        assert_eq!(
            parse_saturation("saturation", &[]).unwrap(),
            EffectCommand::Saturation(Saturation::default())
        );
        assert_eq!(
            parse_saturation("saturation", &["sqrt", "0.75", "0.1", "0.25"]).unwrap(),
            EffectCommand::Saturation(
                Saturation::new(SaturationType::Sqrt, 0.75, 0.1, 0.25).unwrap()
            )
        );
        assert_eq!(
            parse_saturation("saturation", &["diode"])
                .unwrap()
                .render_tokens(),
            ["saturation", "diode", "1", "0", "0.5"]
        );
    }

    #[test]
    fn rejects_invalid_values_and_extra_arguments() {
        assert_eq!(
            parse_saturation("saturation", &["tanh", "1", "0", "0.5"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "saturation",
                argument: "saturation",
                source: EffectError::InvalidSaturation,
            }
        );
        assert_eq!(
            parse_saturation("saturation", &["unknown"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "saturation",
                argument: "unknown".to_owned(),
            }
        );
        assert_eq!(
            parse_saturation("saturation", &["sqrt", "1", "0", "0.5", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "saturation",
                argument: "extra".to_owned(),
            }
        );
    }
}
