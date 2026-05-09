//! Typed parser for currently implemented effect commands.
//!
//! The parser accepts a small SoX-ng-style token shape, where the first token
//! is an effect name and the remaining tokens are positional effect arguments.
//! It resolves names through [`crate::EffectRegistry`] and converts arguments
//! into existing typed effect processors. Unsupported SoX-ng options are
//! reported explicitly instead of being stored as untyped strings.
//!
//! # Examples
//!
//! ```
//! use auralis_core::{Decibels, FrameCount};
//! use auralis_effects::{EffectCommand, Fade, Gain, parse_effect_command};
//!
//! let gain = parse_effect_command(&["gain", "-3"])?;
//! assert_eq!(gain, EffectCommand::Gain(Gain::new(Decibels::new(-3.0)?)));
//! assert_eq!(gain.render_tokens(), ["gain", "-3"]);
//!
//! let fade = EffectCommand::parse("fade", &["l", "10", "20"])?;
//! assert_eq!(fade, EffectCommand::Fade(Fade::new(FrameCount::new(10), FrameCount::new(20))));
//! assert_eq!(fade.render_tokens(), ["fade", "l", "10", "20"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::num::{ParseFloatError, ParseIntError};

use auralis_core::{AuralisError, Decibels, FrameCount};
use thiserror::Error;

use crate::{
    DcShift, EffectError, EffectKind, EffectNameError, EffectRegistry, Fade, Gain, Pad, Reverse,
    Trim,
};

/// Crate-local result type for command parsing.
pub type CommandResult<T> = std::result::Result<T, EffectCommandParseError>;

/// A typed command for one currently implemented Auralis effect.
///
/// This enum is the command-model boundary: parsing may start from tokenized
/// command strings, but successful results contain typed effect processors
/// only. It intentionally models the currently implemented Auralis subset:
/// future SoX-ng options such as gain normalization, fade curve types, dcshift
/// limiter gain, and positioned padding are rejected until their corresponding
/// features are implemented.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum EffectCommand {
    /// Constant normalized full-scale offset.
    DcShift(DcShift),

    /// Linear fade-in and fade-out frame counts.
    Fade(Fade),

    /// Constant gain in decibels.
    Gain(Gain),

    /// Start/end zero padding measured in frames.
    Pad(Pad),

    /// Frame-order reversal within each channel.
    Reverse(Reverse),

    /// End-exclusive frame range selection.
    Trim(Trim),
}

impl EffectCommand {
    /// Parses a command from an effect name and positional arguments.
    ///
    /// # Errors
    ///
    /// Returns [`EffectCommandParseError`] when the effect name is unknown or
    /// unsupported, when required arguments are missing, when an argument cannot
    /// be converted into the typed effect configuration, or when an unsupported
    /// SoX-ng option is present.
    pub fn parse(name: &str, args: &[&str]) -> CommandResult<Self> {
        let descriptor = EffectRegistry::resolve(name)?;
        let effect = descriptor.canonical_name();

        match descriptor.kind() {
            EffectKind::DcShift => parse_dc_shift(effect, args),
            EffectKind::Fade => parse_fade(effect, args),
            EffectKind::Gain => parse_gain(effect, args),
            EffectKind::Pad => parse_pad(effect, args),
            EffectKind::Reverse => parse_reverse(effect, args),
            EffectKind::Trim => parse_trim(effect, args),
        }
    }

    /// Returns the typed effect kind represented by this command.
    #[must_use]
    pub const fn kind(self) -> EffectKind {
        match self {
            Self::DcShift(_) => EffectKind::DcShift,
            Self::Fade(_) => EffectKind::Fade,
            Self::Gain(_) => EffectKind::Gain,
            Self::Pad(_) => EffectKind::Pad,
            Self::Reverse(_) => EffectKind::Reverse,
            Self::Trim(_) => EffectKind::Trim,
        }
    }

    /// Renders this command as canonical SoX-ng-style tokens.
    ///
    /// Rendering uses canonical effect names, explicit default arguments, and
    /// deterministic numeric formatting. It intentionally returns an argument
    /// vector without shell quoting so callers can pass it directly to process
    /// builders or use a display layer that applies stable quoting for failure
    /// reports.
    #[must_use]
    pub fn render_tokens(self) -> Vec<String> {
        match self {
            Self::DcShift(dc_shift) => {
                vec!["dcshift".to_owned(), render_f32(dc_shift.shift)]
            }
            Self::Fade(fade) => vec![
                "fade".to_owned(),
                "l".to_owned(),
                fade.fade_in.as_u64().to_string(),
                fade.fade_out.as_u64().to_string(),
            ],
            Self::Gain(gain) => vec!["gain".to_owned(), render_f64(gain.db.as_f64())],
            Self::Pad(pad) => vec![
                "pad".to_owned(),
                pad.start.as_u64().to_string(),
                pad.end.as_u64().to_string(),
            ],
            Self::Reverse(_) => vec!["reverse".to_owned()],
            Self::Trim(trim) => vec![
                "trim".to_owned(),
                trim.start.as_u64().to_string(),
                trim.end.as_u64().to_string(),
            ],
        }
    }
}

/// Parses a tokenized effect command.
///
/// The first token must be the effect name; remaining tokens are interpreted as
/// positional arguments for the currently implemented Auralis subset.
///
/// # Errors
///
/// Returns [`EffectCommandParseError::EmptyCommand`] for an empty token slice
/// or the same errors as [`EffectCommand::parse`] once an effect name exists.
pub fn parse_effect_command(tokens: &[&str]) -> CommandResult<EffectCommand> {
    let Some((name, args)) = tokens.split_first() else {
        return Err(EffectCommandParseError::EmptyCommand);
    };

    EffectCommand::parse(name, args)
}

/// Errors produced while parsing an effect command.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum EffectCommandParseError {
    /// The caller supplied no command tokens.
    #[error("effect command cannot be empty")]
    EmptyCommand,

    /// The effect name was empty, unknown, or known but unsupported.
    #[error(transparent)]
    EffectName(#[from] EffectNameError),

    /// A required positional argument was missing.
    #[error("effect `{effect}` requires argument `{argument}`")]
    MissingArgument {
        /// Canonical effect name.
        effect: &'static str,

        /// Missing argument name.
        argument: &'static str,
    },

    /// An argument was present after all implemented arguments were parsed.
    #[error("unexpected argument `{argument}` for effect `{effect}`")]
    UnexpectedArgument {
        /// Canonical effect name.
        effect: &'static str,

        /// Unexpected argument value.
        argument: String,
    },

    /// A known SoX-ng option is not part of the currently implemented subset.
    #[error("unsupported option `{option}` for effect `{effect}`")]
    UnsupportedOption {
        /// Canonical effect name.
        effect: &'static str,

        /// Unsupported option text or option family name.
        option: String,
    },

    /// A floating-point argument could not be parsed.
    #[error("effect `{effect}` argument `{argument}` must be a number, got `{value}`")]
    InvalidNumber {
        /// Canonical effect name.
        effect: &'static str,

        /// Argument name.
        argument: &'static str,

        /// Original argument value.
        value: String,

        /// Source parse error.
        #[source]
        source: ParseFloatError,
    },

    /// A frame-count argument could not be parsed.
    #[error(
        "effect `{effect}` argument `{argument}` must be a non-negative frame count, got `{value}`"
    )]
    InvalidFrameCount {
        /// Canonical effect name.
        effect: &'static str,

        /// Argument name.
        argument: &'static str,

        /// Original argument value.
        value: String,

        /// Source parse error.
        #[source]
        source: ParseIntError,
    },

    /// A parsed core value was rejected by its typed constructor.
    #[error("invalid `{argument}` for effect `{effect}`: {source}")]
    InvalidCoreValue {
        /// Canonical effect name.
        effect: &'static str,

        /// Argument name.
        argument: &'static str,

        /// Source typed-constructor error.
        #[source]
        source: AuralisError,
    },

    /// A parsed effect configuration was rejected by its typed constructor.
    #[error("invalid `{argument}` for effect `{effect}`: {source}")]
    InvalidEffectConfig {
        /// Canonical effect name.
        effect: &'static str,

        /// Argument name.
        argument: &'static str,

        /// Source typed-constructor error.
        #[source]
        source: EffectError,
    },
}

fn parse_gain(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let db = match args {
        [] => Decibels::new(0.0).map_err(|source| EffectCommandParseError::InvalidCoreValue {
            effect,
            argument: "gain-dB",
            source,
        })?,
        [db] => parse_decibels(effect, "gain-dB", db)?,
        [db, rest @ ..] => {
            reject_extra_arguments(effect, rest)?;
            parse_decibels(effect, "gain-dB", db)?
        }
    };

    Ok(EffectCommand::Gain(Gain::new(db)))
}

fn parse_dc_shift(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let shift = required_arg(effect, args, "shift")?;
    let shift = parse_f32(effect, "shift", shift)?;
    reject_extra_arguments(effect, &args[1..])?;

    DcShift::new(shift)
        .map(EffectCommand::DcShift)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "shift",
            source,
        })
}

fn parse_trim(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let start = required_arg(effect, args, "start-frame")?;
    let end = args
        .get(1)
        .copied()
        .ok_or(EffectCommandParseError::MissingArgument {
            effect,
            argument: "end-frame",
        })?;
    reject_extra_arguments(effect, &args[2..])?;

    let start = parse_frame_count(effect, "start-frame", start)?;
    let end = parse_frame_count(effect, "end-frame", end)?;

    Trim::new(start, end)
        .map(EffectCommand::Trim)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "frame-range",
            source,
        })
}

fn parse_pad(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (start, end) = match args {
        [] => (FrameCount::new(0), FrameCount::new(0)),
        [start] => (
            parse_frame_count(effect, "start-frame", start)?,
            FrameCount::new(0),
        ),
        [start, end] => (
            parse_frame_count(effect, "start-frame", start)?,
            parse_frame_count(effect, "end-frame", end)?,
        ),
        [start, end, rest @ ..] => {
            reject_extra_arguments(effect, rest)?;
            (
                parse_frame_count(effect, "start-frame", start)?,
                parse_frame_count(effect, "end-frame", end)?,
            )
        }
    };

    Ok(EffectCommand::Pad(Pad::new(start, end)))
}

fn parse_reverse(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    reject_extra_arguments(effect, args)?;

    Ok(EffectCommand::Reverse(Reverse::new()))
}

fn parse_fade(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let args = match args {
        ["l", rest @ ..] => rest,
        [curve @ ("q" | "h" | "t" | "p"), ..] => {
            return Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: (*curve).to_owned(),
            });
        }
        _ => args,
    };

    let fade_in = required_arg(effect, args, "fade-in-frame")?;
    let fade_in = parse_frame_count(effect, "fade-in-frame", fade_in)?;
    let fade_out = match args.get(1).copied() {
        Some(fade_out) => parse_frame_count(effect, "fade-out-frame", fade_out)?,
        None => FrameCount::new(0),
    };
    reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;

    Ok(EffectCommand::Fade(Fade::new(fade_in, fade_out)))
}

fn required_arg<'args>(
    effect: &'static str,
    args: &'args [&'args str],
    argument: &'static str,
) -> CommandResult<&'args str> {
    args.first()
        .copied()
        .ok_or(EffectCommandParseError::MissingArgument { effect, argument })
}

fn parse_decibels(
    effect: &'static str,
    argument: &'static str,
    value: &str,
) -> CommandResult<Decibels> {
    let db = parse_f64(effect, argument, value)?;

    Decibels::new(db).map_err(|source| EffectCommandParseError::InvalidCoreValue {
        effect,
        argument,
        source,
    })
}

fn parse_f64(effect: &'static str, argument: &'static str, value: &str) -> CommandResult<f64> {
    reject_option_like_argument(effect, value)?;

    value
        .parse::<f64>()
        .map_err(|source| EffectCommandParseError::InvalidNumber {
            effect,
            argument,
            value: value.to_owned(),
            source,
        })
}

fn parse_f32(effect: &'static str, argument: &'static str, value: &str) -> CommandResult<f32> {
    reject_option_like_argument(effect, value)?;

    value
        .parse::<f32>()
        .map_err(|source| EffectCommandParseError::InvalidNumber {
            effect,
            argument,
            value: value.to_owned(),
            source,
        })
}

fn parse_frame_count(
    effect: &'static str,
    argument: &'static str,
    value: &str,
) -> CommandResult<FrameCount> {
    reject_option_like_argument(effect, value)?;

    value.parse::<u64>().map(FrameCount::new).map_err(|source| {
        EffectCommandParseError::InvalidFrameCount {
            effect,
            argument,
            value: value.to_owned(),
            source,
        }
    })
}

fn reject_extra_arguments(effect: &'static str, args: &[&str]) -> CommandResult<()> {
    if let Some(option) = args.iter().copied().find(|arg| is_option_like(arg)) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: option.to_owned(),
        });
    }

    if let Some(argument) = args.first() {
        return Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: (*argument).to_owned(),
        });
    }

    Ok(())
}

fn reject_option_like_argument(effect: &'static str, value: &str) -> CommandResult<()> {
    if is_option_like(value) {
        Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: value.to_owned(),
        })
    } else {
        Ok(())
    }
}

fn is_option_like(value: &str) -> bool {
    value.starts_with('-') && value.parse::<f64>().is_err()
}

fn render_f64(value: f64) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

fn render_f32(value: f32) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{EffectCommand, EffectCommandParseError, parse_effect_command};
    use crate::{DcShift, EffectError, Fade, Gain, Pad, Reverse, Trim};
    use auralis_core::{Decibels, FrameCount};

    #[test]
    fn parses_supported_effect_commands_into_typed_configs() {
        let expected = [
            (
                &["gain", "-3"][..],
                EffectCommand::Gain(Gain::new(Decibels::new(-3.0).unwrap())),
            ),
            (
                &["dcshift", "0.25"][..],
                EffectCommand::DcShift(DcShift::new(0.25).unwrap()),
            ),
            (
                &["trim", "1", "3"][..],
                EffectCommand::Trim(Trim::new(FrameCount::new(1), FrameCount::new(3)).unwrap()),
            ),
            (
                &["pad", "2", "1"][..],
                EffectCommand::Pad(Pad::new(FrameCount::new(2), FrameCount::new(1))),
            ),
            (&["reverse"][..], EffectCommand::Reverse(Reverse::new())),
            (
                &["fade", "4", "2"][..],
                EffectCommand::Fade(Fade::new(FrameCount::new(4), FrameCount::new(2))),
            ),
        ];

        for (tokens, command) in expected {
            assert_eq!(parse_effect_command(tokens).unwrap(), command);
            assert_eq!(parse_effect_command(tokens).unwrap().kind(), command.kind());
        }
    }

    #[test]
    fn parses_supported_aliases_through_registry_resolution() {
        assert_eq!(
            parse_effect_command(&["dc-shift", "-0.25"]).unwrap(),
            EffectCommand::DcShift(DcShift::new(-0.25).unwrap())
        );
        assert_eq!(
            parse_effect_command(&["gain-db", "6"]).unwrap(),
            EffectCommand::Gain(Gain::new(Decibels::new(6.0).unwrap()))
        );
    }

    #[test]
    fn defaults_match_existing_identity_configs() {
        assert_eq!(
            parse_effect_command(&["gain"]).unwrap(),
            EffectCommand::Gain(Gain::new(Decibels::new(0.0).unwrap()))
        );
        assert_eq!(
            parse_effect_command(&["pad"]).unwrap(),
            EffectCommand::Pad(Pad::new(FrameCount::new(0), FrameCount::new(0)))
        );
        assert_eq!(
            parse_effect_command(&["fade", "3"]).unwrap(),
            EffectCommand::Fade(Fade::new(FrameCount::new(3), FrameCount::new(0)))
        );
    }

    #[test]
    fn explicit_linear_fade_curve_parses_but_other_curves_are_unsupported() {
        assert_eq!(
            parse_effect_command(&["fade", "l", "3", "2"]).unwrap(),
            EffectCommand::Fade(Fade::new(FrameCount::new(3), FrameCount::new(2)))
        );

        let error = parse_effect_command(&["fade", "h", "3"]).unwrap_err();

        assert_eq!(
            error,
            EffectCommandParseError::UnsupportedOption {
                effect: "fade",
                option: "h".to_owned(),
            }
        );
        assert_eq!(
            error.to_string(),
            "unsupported option `h` for effect `fade`"
        );
    }

    #[test]
    fn unsupported_options_name_the_effect_and_option() {
        let error = parse_effect_command(&["gain", "-n"]).unwrap_err();

        assert_eq!(
            error,
            EffectCommandParseError::UnsupportedOption {
                effect: "gain",
                option: "-n".to_owned(),
            }
        );
        assert_eq!(
            error.to_string(),
            "unsupported option `-n` for effect `gain`"
        );
    }

    #[test]
    fn unsupported_and_unknown_effect_names_use_registry_diagnostics() {
        let unsupported = parse_effect_command(&["allpass"]).unwrap_err();
        assert!(
            unsupported
                .to_string()
                .contains("known SoX-ng effect `allpass`")
        );

        let unknown = parse_effect_command(&["gian"]).unwrap_err();
        assert_eq!(
            unknown.to_string(),
            "unknown effect `gian`; did you mean `gain`?"
        );
    }

    #[test]
    fn missing_and_extra_arguments_are_rejected() {
        assert_eq!(
            parse_effect_command(&[]).unwrap_err(),
            EffectCommandParseError::EmptyCommand
        );
        assert_eq!(
            parse_effect_command(&["dcshift"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "dcshift",
                argument: "shift",
            }
        );
        assert_eq!(
            parse_effect_command(&["reverse", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "reverse",
                argument: "extra".to_owned(),
            }
        );
    }

    #[test]
    fn invalid_numeric_values_are_rejected_before_effect_construction() {
        let error = parse_effect_command(&["trim", "-1", "2"]).unwrap_err();

        assert!(matches!(
            error,
            EffectCommandParseError::InvalidFrameCount {
                effect: "trim",
                argument: "start-frame",
                value,
                ..
            } if value == "-1"
        ));

        let error = parse_effect_command(&["gain", "not-a-number"]).unwrap_err();

        assert!(matches!(
            error,
            EffectCommandParseError::InvalidNumber {
                effect: "gain",
                argument: "gain-dB",
                value,
                ..
            } if value == "not-a-number"
        ));
    }

    #[test]
    fn typed_effect_validation_errors_are_preserved() {
        let error = parse_effect_command(&["dcshift", "2.0001"]).unwrap_err();

        assert_eq!(
            error,
            EffectCommandParseError::InvalidEffectConfig {
                effect: "dcshift",
                argument: "shift",
                source: EffectError::InvalidDcShift,
            }
        );
    }

    #[test]
    fn parsed_typed_configs_preserve_effect_behavior() {
        let command = parse_effect_command(&["gain", "-6"]).unwrap();
        let EffectCommand::Gain(gain) = command else {
            panic!("expected gain command");
        };
        let mut parsed = [0.25, -0.5, 1.0];
        let mut direct = parsed;

        gain.process_samples(&mut parsed);
        Gain::new(Decibels::new(-6.0).unwrap()).process_samples(&mut direct);

        for (parsed, direct) in parsed.iter().zip(direct) {
            assert_eq!(parsed.to_bits(), direct.to_bits());
        }
    }

    #[test]
    fn equivalent_commands_render_to_identical_canonical_tokens() {
        let equivalent_commands = [
            (&["gain"][..], &["gain", "0.0"][..], &["gain", "0"][..]),
            (
                &["gain-db", "1e0"][..],
                &["gain", "1"][..],
                &["gain", "1"][..],
            ),
            (
                &["dc-shift", "-0.0"][..],
                &["dcshift", "0"][..],
                &["dcshift", "0"][..],
            ),
            (&["pad"][..], &["pad", "0", "0"][..], &["pad", "0", "0"][..]),
            (
                &["fade", "3"][..],
                &["fade", "l", "3", "0"][..],
                &["fade", "l", "3", "0"][..],
            ),
        ];

        for (left, right, expected) in equivalent_commands {
            let left = parse_effect_command(left).unwrap().render_tokens();
            let right = parse_effect_command(right).unwrap().render_tokens();
            assert_eq!(left, right);
            assert_eq!(left, expected);
        }
    }

    #[test]
    fn command_rendering_uses_canonical_names_and_explicit_arguments() {
        let commands = [
            (
                parse_effect_command(&["trim", "12", "34"]).unwrap(),
                &["trim", "12", "34"][..],
            ),
            (
                parse_effect_command(&["reverse"]).unwrap(),
                &["reverse"][..],
            ),
            (
                parse_effect_command(&["fade", "l", "4", "2"]).unwrap(),
                &["fade", "l", "4", "2"][..],
            ),
        ];

        for (command, expected) in commands {
            assert_eq!(command.render_tokens(), expected);
        }
    }
}
