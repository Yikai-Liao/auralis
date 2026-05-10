use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f64, reject_extra_arguments,
    render_f64,
};
use crate::{Stretch, StretchFade};

pub(super) fn parse_stretch(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    reject_extra_arguments(effect, args.get(5..).unwrap_or_default())?;

    let factor = parse_optional_f64(effect, args, 0, "factor", 1.0)?;
    let window_ms = parse_optional_f64(effect, args, 1, "window", 20.0)?;
    let fade = parse_fade(effect, args.get(2).copied())?;
    let default_shift = if factor <= 1.0 { 1.0 } else { 0.8 };
    let shift = parse_optional_f64(effect, args, 3, "shift", default_shift)?;
    let default_fading = if factor < 1.0 {
        1.0 - (factor * shift)
    } else {
        1.0 - shift
    }
    .min(0.5);
    let fading = parse_optional_f64(effect, args, 4, "fading", default_fading)?;

    Stretch::with_options(factor, window_ms, fade, shift, fading)
        .map(EffectCommand::Stretch)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "stretch",
            source,
        })
}

pub(super) fn render_stretch(stretch: Stretch) -> Vec<String> {
    vec![
        "stretch".to_owned(),
        render_f64(stretch.factor),
        render_f64(stretch.window_ms),
        render_fade(stretch.fade).to_owned(),
        render_f64(stretch.shift),
        render_f64(stretch.fading),
    ]
}

fn parse_optional_f64(
    effect: &'static str,
    args: &[&str],
    index: usize,
    argument: &'static str,
    default: f64,
) -> CommandResult<f64> {
    match args.get(index).copied() {
        Some(value) => parse_f64(effect, argument, value),
        None => Ok(default),
    }
}

fn parse_fade(effect: &'static str, value: Option<&str>) -> CommandResult<StretchFade> {
    match value {
        None | Some("l" | "L" | "linear" | "Linear") => Ok(StretchFade::Linear),
        Some("s" | "S" | "sqrt" | "Sqrt") => Ok(StretchFade::Sqrt),
        Some("h" | "H" | "half" | "Half") => Ok(StretchFade::HalfCosine),
        Some("q" | "Q" | "quarter" | "Quarter") => Ok(StretchFade::QuarterCosine),
        Some(_) => Err(EffectCommandParseError::InvalidOptionCombination {
            effect,
            options: "fade must be l, s, h, or q",
        }),
    }
}

const fn render_fade(fade: StretchFade) -> &'static str {
    match fade {
        StretchFade::Linear => "l",
        StretchFade::Sqrt => "s",
        StretchFade::HalfCosine => "h",
        StretchFade::QuarterCosine => "q",
    }
}

#[cfg(test)]
mod tests {
    use super::parse_stretch;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Stretch, StretchFade};

    #[test]
    fn parses_defaults_and_explicit_options() {
        assert_eq!(
            parse_stretch("stretch", &[]).unwrap(),
            EffectCommand::Stretch(Stretch::default())
        );

        assert_eq!(
            parse_stretch("stretch", &["1.5", "10", "q", "0.75", "0.25"]).unwrap(),
            EffectCommand::Stretch(
                Stretch::with_options(1.5, 10.0, StretchFade::QuarterCosine, 0.75, 0.25).unwrap()
            )
        );
    }

    #[test]
    fn renders_canonical_full_option_set() {
        assert_eq!(
            parse_stretch("stretch", &["1.5", "10", "q", "0.75", "0.25"])
                .unwrap()
                .render_tokens(),
            ["stretch", "1.5", "10", "q", "0.75", "0.25"]
        );
    }

    #[test]
    fn rejects_invalid_values_and_extra_arguments() {
        assert_eq!(
            parse_stretch("stretch", &["1", "0"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "stretch",
                argument: "stretch",
                source: EffectError::InvalidStretch,
            }
        );
        assert_eq!(
            parse_stretch("stretch", &["1", "20", "bad"]).unwrap_err(),
            EffectCommandParseError::InvalidOptionCombination {
                effect: "stretch",
                options: "fade must be l, s, h, or q",
            }
        );
        assert_eq!(
            parse_stretch("stretch", &["1", "20", "l", "1", "0", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "stretch",
                argument: "extra".to_owned(),
            }
        );
    }
}
