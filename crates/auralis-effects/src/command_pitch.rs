use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, parse_f64, render_f64,
    required_arg,
};
use crate::{Pitch, TempoProfile};

pub(super) fn parse_pitch(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (quick_search, args) = parse_options(effect, args)?;
    let shift = required_arg(effect, args, "shift")?;
    if is_option_like(shift) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: shift.to_owned(),
        });
    }
    crate::command::reject_extra_arguments(effect, args.get(4..).unwrap_or_default())?;
    let segment_ms = parse_optional_f64(effect, args, 1, "segment")?;
    let search_ms = parse_optional_f64(effect, args, 2, "search")?;
    let overlap_ms = parse_optional_f64(effect, args, 3, "overlap")?;

    Pitch::with_tuning(
        parse_f64(effect, "shift", shift)?,
        quick_search,
        segment_ms,
        search_ms,
        overlap_ms,
    )
    .map(EffectCommand::Pitch)
    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "pitch",
        source,
    })
}

pub(super) fn render_pitch(pitch: Pitch) -> Vec<String> {
    let mut tokens = vec!["pitch".to_owned()];
    if pitch.quick_search {
        tokens.push("-q".to_owned());
    }
    tokens.push(render_f64(pitch.cents));
    render_optional_timing(pitch, &mut tokens);
    tokens
}

fn parse_options<'args>(
    effect: &'static str,
    args: &'args [&'args str],
) -> CommandResult<(bool, &'args [&'args str])> {
    let mut quick_search = false;
    let mut index = 0;

    while let Some(option) = args.get(index).copied() {
        match option {
            "-q" => quick_search = true,
            option if is_option_like(option) => {
                return Err(EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: option.to_owned(),
                });
            }
            _ => break,
        }
        index += 1;
    }

    Ok((quick_search, &args[index..]))
}

fn parse_optional_f64(
    effect: &'static str,
    args: &[&str],
    index: usize,
    argument: &'static str,
) -> CommandResult<Option<f64>> {
    args.get(index)
        .map(|value| parse_f64(effect, argument, value))
        .transpose()
}

fn render_optional_timing(pitch: Pitch, tokens: &mut Vec<String>) {
    if pitch.segment_ms.is_none() && pitch.search_ms.is_none() && pitch.overlap_ms.is_none() {
        return;
    }

    let tempo_factor = 1.0 / 2.0_f64.powf(pitch.cents / 1200.0);
    let segment_ms = pitch
        .segment_ms
        .unwrap_or_else(|| TempoProfile::Default.default_segment_ms(tempo_factor));
    tokens.push(render_f64(segment_ms));

    if pitch.search_ms.is_none() && pitch.overlap_ms.is_none() {
        return;
    }

    let search_ms = pitch
        .search_ms
        .unwrap_or_else(|| TempoProfile::Default.default_search_ms(segment_ms));
    tokens.push(render_f64(search_ms));

    if let Some(overlap_ms) = pitch.overlap_ms {
        tokens.push(render_f64(overlap_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::parse_pitch;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Pitch};

    #[test]
    fn parses_shift_and_quick_tuning() {
        assert_eq!(
            parse_pitch("pitch", &["1200"]).unwrap(),
            EffectCommand::Pitch(Pitch::new(1200.0).unwrap())
        );
        assert_eq!(
            parse_pitch("pitch", &["-q", "-1200", "60", "10", "8"]).unwrap(),
            EffectCommand::Pitch(
                Pitch::with_tuning(-1200.0, true, Some(60.0), Some(10.0), Some(8.0)).unwrap()
            )
        );
    }

    #[test]
    fn renders_canonical_tokens() {
        assert_eq!(
            parse_pitch("pitch", &["-q", "-1200", "60", "10", "8"])
                .unwrap()
                .render_tokens(),
            ["pitch", "-q", "-1200", "60", "10", "8"]
        );
    }

    #[test]
    fn rejects_missing_unsupported_invalid_and_extra_arguments() {
        assert_eq!(
            parse_pitch("pitch", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "pitch",
                argument: "shift",
            }
        );
        assert_eq!(
            parse_pitch("pitch", &["-m", "1200"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "pitch",
                option: "-m".to_owned(),
            }
        );
        assert_eq!(
            parse_pitch("pitch", &["5000"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "pitch",
                argument: "pitch",
                source: EffectError::InvalidPitchShift,
            }
        );
        assert_eq!(
            parse_pitch("pitch", &["1200", "9"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "pitch",
                argument: "pitch",
                source: EffectError::InvalidPitchTuning,
            }
        );
        assert_eq!(
            parse_pitch("pitch", &["1200", "60", "10", "8", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "pitch",
                argument: "extra".to_owned(),
            }
        );
    }
}
