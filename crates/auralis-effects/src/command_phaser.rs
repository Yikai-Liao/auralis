use crate::{
    Phaser, PhaserInterpolation, PhaserWave,
    command::{CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64},
};

pub(super) fn parse_phaser(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut index = 0;
    let mut interpolation = PhaserInterpolation::None;
    let mut wave = PhaserWave::Sine;

    while let Some(option) = args.get(index).copied().and_then(parse_option) {
        match option {
            PhaserOption::Interpolation(mode) => interpolation = mode,
            PhaserOption::Wave(shape) => wave = shape,
        }
        index += 1;
    }

    let gain_in = parse_optional_number(effect, args, &mut index, "gain-in", 0.4)?;
    let gain_out = parse_optional_number(effect, args, &mut index, "gain-out", 0.74)?;
    let delay_ms = parse_optional_number(effect, args, &mut index, "delay", 3.0)?;
    let regen = parse_optional_number(effect, args, &mut index, "regen", 0.4)?;
    let speed_hz = parse_optional_number(effect, args, &mut index, "speed", 0.5)?;

    if let Some(shape) = args.get(index).copied().and_then(parse_wave_option) {
        wave = shape;
        index += 1;
    }

    if let Some(argument) = args.get(index) {
        return Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: (*argument).to_owned(),
        });
    }

    Ok(EffectCommand::Phaser(
        Phaser::new(
            gain_in,
            gain_out,
            delay_ms,
            regen,
            speed_hz,
            wave,
            interpolation,
        )
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "phaser",
            source,
        })?,
    ))
}

pub(super) fn render_phaser(phaser: Phaser) -> Vec<String> {
    let mut tokens = vec!["phaser".to_owned()];
    match phaser.interpolation() {
        PhaserInterpolation::None => {}
        PhaserInterpolation::Linear => tokens.push("-l".to_owned()),
        PhaserInterpolation::Quadratic => tokens.push("-q".to_owned()),
    }
    if phaser.wave() == PhaserWave::Triangle {
        tokens.push("-t".to_owned());
    }
    tokens.push(render_f64(phaser.gain_in()));
    tokens.push(render_f64(phaser.gain_out()));
    tokens.push(render_f64(phaser.delay_ms()));
    tokens.push(render_f64(phaser.regen()));
    tokens.push(render_f64(phaser.speed_hz()));
    tokens
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhaserOption {
    Interpolation(PhaserInterpolation),
    Wave(PhaserWave),
}

fn parse_option(value: &str) -> Option<PhaserOption> {
    match value {
        "-n" => Some(PhaserOption::Interpolation(PhaserInterpolation::None)),
        "-l" => Some(PhaserOption::Interpolation(PhaserInterpolation::Linear)),
        "-q" => Some(PhaserOption::Interpolation(PhaserInterpolation::Quadratic)),
        "-s" | "-sine" => Some(PhaserOption::Wave(PhaserWave::Sine)),
        "-t" | "-triangle" => Some(PhaserOption::Wave(PhaserWave::Triangle)),
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
    if parse_wave_option(value).is_some() {
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

fn parse_wave_option(value: &str) -> Option<PhaserWave> {
    match value {
        "-s" | "-sine" => Some(PhaserWave::Sine),
        "-t" | "-triangle" => Some(PhaserWave::Triangle),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_phaser;
    use crate::{EffectCommand, Phaser, PhaserInterpolation, PhaserWave};

    #[test]
    fn parses_bare_defaults() {
        assert_eq!(
            parse_phaser("phaser", &[]).unwrap(),
            EffectCommand::Phaser(Phaser::with_defaults().unwrap())
        );
    }

    #[test]
    fn parses_flags_and_trailing_wave_option() {
        assert_eq!(
            parse_phaser(
                "phaser",
                &["-q", "-t", "0.8", "0.74", "3", "0.4", "0.5", "-s"]
            )
            .unwrap(),
            EffectCommand::Phaser(
                Phaser::new(
                    0.8,
                    0.74,
                    3.0,
                    0.4,
                    0.5,
                    PhaserWave::Sine,
                    PhaserInterpolation::Quadratic,
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn render_phaser_uses_canonical_tokens() {
        assert_eq!(
            parse_phaser("phaser", &["-l", "0.4", "0.74", "3", "0.4", "0.5"])
                .unwrap()
                .render_tokens(),
            ["phaser", "-l", "0.4", "0.74", "3", "0.4", "0.5"]
        );
    }
}
