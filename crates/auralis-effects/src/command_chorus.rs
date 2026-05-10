use crate::{
    Chorus, ChorusInterpolation, ChorusStage, ChorusWave,
    command::{CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64},
};

pub(super) fn parse_chorus(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut index = 0;
    let mut interpolation = ChorusInterpolation::None;
    let mut default_wave = ChorusWave::Sine;

    while let Some(option) = args.get(index).copied().and_then(parse_global_option) {
        match option {
            ChorusOption::Interpolation(mode) => interpolation = mode,
            ChorusOption::Wave(wave) => default_wave = wave,
        }
        index += 1;
    }

    let gain_in = parse_optional_number(effect, args, &mut index, "gain-in", 0.5)?;
    let gain_out = parse_optional_number(effect, args, &mut index, "gain-out", 1.0)?;
    let stages = parse_stages(effect, &args[index..], default_wave)?;

    Ok(EffectCommand::Chorus(
        Chorus::with_options(gain_in, gain_out, stages, interpolation).map_err(|source| {
            EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "chorus",
                source,
            }
        })?,
    ))
}

pub(super) fn render_chorus(chorus: &Chorus) -> Vec<String> {
    let mut tokens = vec!["chorus".to_owned()];
    match chorus.interpolation() {
        ChorusInterpolation::None => {}
        ChorusInterpolation::Linear => tokens.push("-l".to_owned()),
        ChorusInterpolation::Quadratic => tokens.push("-q".to_owned()),
    }
    tokens.push(render_f64(chorus.gain_in()));
    tokens.push(render_f64(chorus.gain_out()));
    for stage in chorus.stages() {
        tokens.push(render_f64(stage.delay_ms()));
        tokens.push(render_f64(stage.decay()));
        tokens.push(render_f64(stage.speed_hz()));
        tokens.push(render_f64(stage.depth_ms()));
        if stage.wave() == ChorusWave::Triangle {
            tokens.push("-triangle".to_owned());
        }
    }
    tokens
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChorusOption {
    Interpolation(ChorusInterpolation),
    Wave(ChorusWave),
}

fn parse_global_option(value: &str) -> Option<ChorusOption> {
    match value {
        "-n" => Some(ChorusOption::Interpolation(ChorusInterpolation::None)),
        "-l" => Some(ChorusOption::Interpolation(ChorusInterpolation::Linear)),
        "-q" => Some(ChorusOption::Interpolation(ChorusInterpolation::Quadratic)),
        "-s" => Some(ChorusOption::Wave(ChorusWave::Sine)),
        "-t" => Some(ChorusOption::Wave(ChorusWave::Triangle)),
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
    if is_stage_wave(value) || value.parse::<f64>().is_err() {
        return Ok(default);
    }
    *index += 1;
    parse_f64(effect, argument, value)
}

fn parse_stages(
    effect: &'static str,
    args: &[&str],
    default_wave: ChorusWave,
) -> CommandResult<Vec<ChorusStage>> {
    if args.is_empty() {
        return Ok(vec![ChorusStage::default_stage()]);
    }

    let mut stages = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let delay_ms = parse_stage_number(effect, args, &mut index, "delay", 50.0)?;
        let decay = parse_stage_number(effect, args, &mut index, "decay", 0.5)?;
        let speed_hz = parse_stage_number(effect, args, &mut index, "speed", 0.25)?;
        let depth_ms = parse_stage_number(effect, args, &mut index, "depth", 2.0)?;
        let wave = parse_stage_wave(args.get(index).copied()).unwrap_or(default_wave);
        if parse_stage_wave(args.get(index).copied()).is_some() {
            index += 1;
        }

        stages.push(
            ChorusStage::with_wave(delay_ms, decay, speed_hz, depth_ms, wave).map_err(
                |source| EffectCommandParseError::InvalidEffectConfig {
                    effect,
                    argument: "stage",
                    source,
                },
            )?,
        );
    }

    Ok(stages)
}

fn parse_stage_number(
    effect: &'static str,
    args: &[&str],
    index: &mut usize,
    argument: &'static str,
    default: f64,
) -> CommandResult<f64> {
    let Some(value) = args.get(*index).copied() else {
        return Ok(default);
    };
    if is_stage_wave(value) {
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

fn parse_stage_wave(value: Option<&str>) -> Option<ChorusWave> {
    match value {
        Some("-sine") => Some(ChorusWave::Sine),
        Some("-triangle") => Some(ChorusWave::Triangle),
        _ => None,
    }
}

fn is_stage_wave(value: &str) -> bool {
    parse_stage_wave(Some(value)).is_some()
}

#[cfg(test)]
mod tests {
    use super::parse_chorus;
    use crate::{Chorus, ChorusInterpolation, ChorusStage, ChorusWave, EffectCommand};

    #[test]
    fn parses_bare_defaults() {
        assert_eq!(
            parse_chorus("chorus", &[]).unwrap(),
            EffectCommand::Chorus(Chorus::with_defaults().unwrap())
        );
    }

    #[test]
    fn parses_options_and_multiple_stages() {
        assert_eq!(
            parse_chorus(
                "chorus",
                &[
                    "-l", "-t", "0.6", "0.8", "1", "0.25", "1", "0", "2", "-0.125", "1", "0",
                    "-sine",
                ],
            )
            .unwrap(),
            EffectCommand::Chorus(
                Chorus::with_options(
                    0.6,
                    0.8,
                    [
                        ChorusStage::with_wave(1.0, 0.25, 1.0, 0.0, ChorusWave::Triangle).unwrap(),
                        ChorusStage::with_wave(2.0, -0.125, 1.0, 0.0, ChorusWave::Sine).unwrap(),
                    ],
                    ChorusInterpolation::Linear,
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn render_chorus_uses_canonical_stage_tokens() {
        assert_eq!(
            parse_chorus("chorus", &["-q", "0.5", "1", "1", "0.25", "1", "0"])
                .unwrap()
                .render_tokens(),
            ["chorus", "-q", "0.5", "1", "1", "0.25", "1", "0"]
        );
    }
}
