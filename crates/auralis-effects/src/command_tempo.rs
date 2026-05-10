use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, parse_f64,
    reject_extra_arguments, render_f64, required_arg,
};
use crate::{Tempo, TempoProfile};

pub(super) fn parse_tempo(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (quick_search, profile, args) = parse_options(effect, args)?;
    let factor = required_arg(effect, args, "factor")?;
    if is_option_like(factor) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: factor.to_owned(),
        });
    }
    reject_extra_arguments(effect, args.get(4..).unwrap_or_default())?;
    let segment_ms = parse_optional_f64(effect, args, 1, "segment")?;
    let search_ms = parse_optional_f64(effect, args, 2, "search")?;
    let overlap_ms = parse_optional_f64(effect, args, 3, "overlap")?;

    Tempo::with_tuning(
        parse_f64(effect, "factor", factor)?,
        quick_search,
        profile,
        segment_ms,
        search_ms,
        overlap_ms,
    )
    .map(EffectCommand::Tempo)
    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "tempo",
        source,
    })
}

pub(super) fn render_tempo(tempo: Tempo) -> Vec<String> {
    let mut tokens = vec!["tempo".to_owned()];
    if tempo.quick_search {
        tokens.push("-q".to_owned());
    }
    if let Some(option) = profile_option(tempo.profile) {
        tokens.push(option.to_owned());
    }
    tokens.push(render_f64(tempo.factor));
    render_optional_timing(tempo, &mut tokens);
    tokens
}

fn parse_options<'args>(
    effect: &'static str,
    args: &'args [&'args str],
) -> CommandResult<(bool, TempoProfile, &'args [&'args str])> {
    let mut quick_search = false;
    let mut profile = TempoProfile::Default;
    let mut index = 0;

    while let Some(option) = args.get(index).copied() {
        match option {
            "-q" => quick_search = true,
            "-m" => profile = TempoProfile::Music,
            "-s" => profile = TempoProfile::Speech,
            "-l" => profile = TempoProfile::Linear,
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

    Ok((quick_search, profile, &args[index..]))
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

fn render_optional_timing(tempo: Tempo, tokens: &mut Vec<String>) {
    if tempo.segment_ms.is_none() && tempo.search_ms.is_none() && tempo.overlap_ms.is_none() {
        return;
    }

    let segment_ms = tempo
        .segment_ms
        .unwrap_or_else(|| tempo.profile.default_segment_ms(tempo.factor));
    tokens.push(render_f64(segment_ms));

    if tempo.search_ms.is_none() && tempo.overlap_ms.is_none() {
        return;
    }

    let search_ms = tempo
        .search_ms
        .unwrap_or_else(|| tempo.profile.default_search_ms(segment_ms));
    tokens.push(render_f64(search_ms));

    if let Some(overlap_ms) = tempo.overlap_ms {
        tokens.push(render_f64(overlap_ms));
    }
}

fn profile_option(profile: TempoProfile) -> Option<&'static str> {
    match profile {
        TempoProfile::Default => None,
        TempoProfile::Music => Some("-m"),
        TempoProfile::Speech => Some("-s"),
        TempoProfile::Linear => Some("-l"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_tempo;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Tempo, TempoProfile};

    #[test]
    fn parses_and_renders_basic_factor() {
        assert_eq!(
            parse_tempo("tempo", &["1.25"]).unwrap(),
            EffectCommand::Tempo(Tempo::new(1.25).unwrap())
        );
        assert_eq!(
            parse_tempo("tempo", &["1.25"]).unwrap().render_tokens(),
            ["tempo", "1.25"]
        );
    }

    #[test]
    fn parses_tuning_flags_and_explicit_timing_values() {
        assert_eq!(
            parse_tempo("tempo", &["-q", "-s", "1.25"]).unwrap(),
            EffectCommand::Tempo(
                Tempo::with_tuning(1.25, true, TempoProfile::Speech, None, None, None).unwrap()
            )
        );
        assert_eq!(
            parse_tempo("tempo", &["-m", "1.5", "60", "10", "8"])
                .unwrap()
                .render_tokens(),
            ["tempo", "-m", "1.5", "60", "10", "8"]
        );
        assert_eq!(
            parse_tempo("tempo", &["-l", "0.75", "20"])
                .unwrap()
                .render_tokens(),
            ["tempo", "-l", "0.75", "20"]
        );
    }

    #[test]
    fn rejects_missing_options_invalid_values_and_extra_arguments() {
        assert_eq!(
            parse_tempo("tempo", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "tempo",
                argument: "factor",
            }
        );
        assert_eq!(
            parse_tempo("tempo", &["1.25", "9"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "tempo",
                argument: "tempo",
                source: EffectError::InvalidTempoTuning,
            }
        );
        assert_eq!(
            parse_tempo("tempo", &["-x", "1.25"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "tempo",
                option: "-x".to_owned(),
            }
        );
        assert_eq!(
            parse_tempo("tempo", &["0.01"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "tempo",
                argument: "tempo",
                source: EffectError::InvalidTempoFactor,
            }
        );
        assert_eq!(
            parse_tempo("tempo", &["1.25", "82", "10", "8", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "tempo",
                argument: "extra".to_owned(),
            }
        );
    }
}
