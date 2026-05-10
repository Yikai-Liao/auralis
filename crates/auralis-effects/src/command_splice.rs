use auralis_core::FrameCount;

use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64, required_arg,
};
use crate::{Splice, SpliceAmount, SpliceFade, SplicePoint, SplicePosition};

pub(super) fn parse_splice(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (fade, args) = parse_options(effect, args)?;
    required_arg(effect, args, "position[,excess[,leeway]]")?;
    let points = args
        .iter()
        .copied()
        .map(|arg| parse_point(effect, arg))
        .collect::<CommandResult<Vec<_>>>()?;

    Splice::with_fade(fade, points)
        .map(EffectCommand::Splice)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "splice",
            source,
        })
}

pub(super) fn render_splice(splice: &Splice) -> Vec<String> {
    let mut tokens = vec!["splice".to_owned()];
    match splice.fade {
        SpliceFade::HalfSine => {}
        SpliceFade::Triangular => tokens.push("-t".to_owned()),
        SpliceFade::QuarterSine => tokens.push("-q".to_owned()),
    }
    tokens.extend(splice.points().iter().copied().map(render_point));
    tokens
}

fn parse_options<'args>(
    effect: &'static str,
    args: &'args [&'args str],
) -> CommandResult<(SpliceFade, &'args [&'args str])> {
    let Some(first) = args.first().copied() else {
        return Ok((SpliceFade::HalfSine, args));
    };
    match first {
        "-h" => Ok((SpliceFade::HalfSine, &args[1..])),
        "-t" => Ok((SpliceFade::Triangular, &args[1..])),
        "-q" => Ok((SpliceFade::QuarterSine, &args[1..])),
        option if crate::command::is_option_like(option) => {
            Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: option.to_owned(),
            })
        }
        _ => Ok((SpliceFade::HalfSine, args)),
    }
}

fn parse_point(effect: &'static str, value: &str) -> CommandResult<SplicePoint> {
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 3 || parts.iter().any(|part| part.is_empty()) {
        return invalid_splice(effect);
    }
    let position = parse_position(effect, parts[0])?;
    let excess = parts
        .get(1)
        .copied()
        .map(|part| parse_amount(effect, part))
        .transpose()?;
    let leeway = parts
        .get(2)
        .copied()
        .map(|part| parse_amount(effect, part))
        .transpose()?;
    Ok(SplicePoint::new(position, excess, leeway))
}

fn parse_position(effect: &'static str, value: &str) -> CommandResult<SplicePosition> {
    let value = value.strip_prefix('=').unwrap_or(value);
    match parse_amount(effect, value)? {
        SpliceAmount::Frames(frames) => Ok(SplicePosition::frames(frames)),
        SpliceAmount::Seconds(seconds) => SplicePosition::seconds(seconds).map_err(|source| {
            EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "splice",
                source,
            }
        }),
    }
}

fn parse_amount(effect: &'static str, value: &str) -> CommandResult<SpliceAmount> {
    if value.is_empty() {
        return invalid_splice(effect);
    }
    if let Some(frames) = value.strip_suffix('s') {
        return frames
            .parse::<u64>()
            .map(|frames| SpliceAmount::Frames(FrameCount::new(frames)))
            .map_err(|_| EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "splice",
                source: crate::EffectError::InvalidSplice,
            });
    }
    let seconds = value.strip_suffix('t').unwrap_or(value);
    if seconds.contains(':') {
        return parse_colon_time(effect, seconds).map(SpliceAmount::Seconds);
    }
    parse_f64(effect, "position", seconds).and_then(|seconds| {
        if seconds.is_finite() && seconds >= 0.0 {
            Ok(SpliceAmount::Seconds(seconds))
        } else {
            invalid_splice(effect)
        }
    })
}

fn parse_colon_time(effect: &'static str, value: &str) -> CommandResult<f64> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() > 3 || parts.iter().any(|part| part.is_empty()) {
        return invalid_splice(effect);
    }
    let mut seconds = 0.0;
    for part in parts {
        seconds = seconds * 60.0 + parse_f64(effect, "position", part)?;
    }
    if seconds.is_finite() && seconds >= 0.0 {
        Ok(seconds)
    } else {
        invalid_splice(effect)
    }
}

fn render_point(point: SplicePoint) -> String {
    let mut rendered = render_position(point.position);
    if let Some(excess) = point.excess {
        rendered.push(',');
        rendered.push_str(&render_amount(excess));
        if let Some(leeway) = point.leeway {
            rendered.push(',');
            rendered.push_str(&render_amount(leeway));
        }
    }
    rendered
}

fn render_position(position: SplicePosition) -> String {
    render_amount(position.amount())
}

fn render_amount(amount: SpliceAmount) -> String {
    match amount {
        SpliceAmount::Frames(frames) => format!("{}s", frames.as_u64()),
        SpliceAmount::Seconds(seconds) => render_f64(seconds),
    }
}

fn invalid_splice<T>(effect: &'static str) -> CommandResult<T> {
    Err(EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "splice",
        source: crate::EffectError::InvalidSplice,
    })
}

#[cfg(test)]
mod tests {
    use auralis_core::FrameCount;

    use super::parse_splice;
    use crate::{
        EffectCommand, EffectCommandParseError, EffectError, Splice, SpliceAmount, SpliceFade,
        SplicePoint, SplicePosition,
    };

    #[test]
    fn parses_default_and_explicit_splice_points() {
        assert_eq!(
            parse_splice("splice", &["1s"]).unwrap(),
            EffectCommand::Splice(
                Splice::new([SplicePoint::new(
                    SplicePosition::frames(FrameCount::new(1)),
                    None,
                    None
                )])
                .unwrap()
            )
        );
        assert_eq!(
            parse_splice("splice", &["-t", "=48s,4s,0s", "96s,2s"]).unwrap(),
            EffectCommand::Splice(
                Splice::with_fade(
                    SpliceFade::Triangular,
                    [
                        SplicePoint::new(
                            SplicePosition::frames(FrameCount::new(48)),
                            Some(SpliceAmount::Frames(FrameCount::new(4))),
                            Some(SpliceAmount::Frames(FrameCount::new(0))),
                        ),
                        SplicePoint::new(
                            SplicePosition::frames(FrameCount::new(96)),
                            Some(SpliceAmount::Frames(FrameCount::new(2))),
                            None,
                        )
                    ]
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn renders_canonical_tokens() {
        assert_eq!(
            parse_splice("splice", &["-q", "1s,4s,0s"])
                .unwrap()
                .render_tokens(),
            ["splice", "-q", "1s,4s,0s"]
        );
    }

    #[test]
    fn rejects_missing_invalid_options_and_malformed_points() {
        assert_eq!(
            parse_splice("splice", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "splice",
                argument: "position[,excess[,leeway]]"
            }
        );
        assert_eq!(
            parse_splice("splice", &["-x", "1s"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "splice",
                option: "-x".to_owned()
            }
        );
        assert_eq!(
            parse_splice("splice", &["1s,"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "splice",
                argument: "splice",
                source: EffectError::InvalidSplice
            }
        );
    }
}
