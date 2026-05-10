use crate::{
    Flanger, FlangerInterpolation, FlangerWave,
    command::{CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64},
};

pub(super) fn parse_flanger(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut index = 0;
    let mut interpolation = FlangerInterpolation::Linear;
    let mut wave = FlangerWave::Sine;

    while let Some(option) = args.get(index).copied().and_then(parse_option) {
        match option {
            FlangerOption::Interpolation(mode) => interpolation = mode,
            FlangerOption::Wave(shape) => wave = shape,
        }
        index += 1;
    }

    let delay_ms = parse_optional_number(effect, args, &mut index, "delay", 0.0)?;
    let depth_ms = parse_optional_number(effect, args, &mut index, "depth", 2.0)?;
    let regen_percent = parse_optional_number(effect, args, &mut index, "regen", 0.0)?;
    let width_percent = parse_optional_number(effect, args, &mut index, "width", 71.0)?;
    let speed_hz = parse_optional_number(effect, args, &mut index, "speed", 0.5)?;

    if let Some(shape) = args.get(index).copied().and_then(parse_wave) {
        wave = shape;
        index += 1;
    }

    let phase_percent = parse_optional_number(effect, args, &mut index, "phase", 25.0)?;

    if let Some(mode) = args.get(index).copied().and_then(parse_interpolation) {
        interpolation = mode;
        index += 1;
    }

    if let Some(argument) = args.get(index) {
        return Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: (*argument).to_owned(),
        });
    }

    Ok(EffectCommand::Flanger(
        Flanger::new(
            delay_ms,
            depth_ms,
            regen_percent,
            width_percent,
            speed_hz,
            wave,
            phase_percent,
            interpolation,
        )
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "flanger",
            source,
        })?,
    ))
}

pub(super) fn render_flanger(flanger: Flanger) -> Vec<String> {
    let mut tokens = vec!["flanger".to_owned()];
    match flanger.interpolation() {
        FlangerInterpolation::None => tokens.push("-n".to_owned()),
        FlangerInterpolation::Linear => {}
        FlangerInterpolation::Quadratic => tokens.push("-q".to_owned()),
    }
    if flanger.wave() == FlangerWave::Triangle {
        tokens.push("-t".to_owned());
    }
    tokens.push(render_f64(flanger.delay_ms()));
    tokens.push(render_f64(flanger.depth_ms()));
    tokens.push(render_f64(flanger.regen_percent()));
    tokens.push(render_f64(flanger.width_percent()));
    tokens.push(render_f64(flanger.speed_hz()));
    tokens.push(match flanger.wave() {
        FlangerWave::Sine => "sine".to_owned(),
        FlangerWave::Triangle => "triangle".to_owned(),
    });
    tokens.push(render_f64(flanger.phase_percent()));
    tokens
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlangerOption {
    Interpolation(FlangerInterpolation),
    Wave(FlangerWave),
}

fn parse_option(value: &str) -> Option<FlangerOption> {
    match value {
        "-n" => Some(FlangerOption::Interpolation(FlangerInterpolation::None)),
        "-l" => Some(FlangerOption::Interpolation(FlangerInterpolation::Linear)),
        "-q" => Some(FlangerOption::Interpolation(
            FlangerInterpolation::Quadratic,
        )),
        "-s" => Some(FlangerOption::Wave(FlangerWave::Sine)),
        "-t" => Some(FlangerOption::Wave(FlangerWave::Triangle)),
        _ => None,
    }
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
    if parse_wave(value).is_some() || parse_interpolation(value).is_some() {
        return Ok(default);
    }
    if value.parse::<f64>().is_err() {
        return Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: value.to_owned(),
        });
    }

    *index += 1;
    parse_f64(effect, argument, value)
}

fn parse_wave(value: &str) -> Option<FlangerWave> {
    match value {
        "s" | "sine" => Some(FlangerWave::Sine),
        "t" | "triangle" => Some(FlangerWave::Triangle),
        _ => None,
    }
}

fn parse_interpolation(value: &str) -> Option<FlangerInterpolation> {
    match value {
        "n" | "none" => Some(FlangerInterpolation::None),
        "l" | "linear" => Some(FlangerInterpolation::Linear),
        "q" | "quadratic" => Some(FlangerInterpolation::Quadratic),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_flanger;
    use crate::{EffectCommand, Flanger, FlangerInterpolation, FlangerWave};

    #[test]
    fn parses_bare_defaults() {
        assert_eq!(
            parse_flanger("flanger", &[]).unwrap(),
            EffectCommand::Flanger(Flanger::with_defaults().unwrap())
        );
    }

    #[test]
    fn parses_flags_and_positional_shape_interpolation() {
        assert_eq!(
            parse_flanger(
                "flanger",
                &["-q", "-t", "1", "2", "25", "100", "1", "sine", "50", "none"],
            )
            .unwrap(),
            EffectCommand::Flanger(
                Flanger::new(
                    1.0,
                    2.0,
                    25.0,
                    100.0,
                    1.0,
                    FlangerWave::Sine,
                    50.0,
                    FlangerInterpolation::None,
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn render_flanger_uses_canonical_tokens() {
        assert_eq!(
            parse_flanger("flanger", &["-n", "1", "2", "0", "71", "0.5"])
                .unwrap()
                .render_tokens(),
            ["flanger", "-n", "1", "2", "0", "71", "0.5", "sine", "25"]
        );
    }
}
