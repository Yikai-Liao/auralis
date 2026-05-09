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
//! let fade = EffectCommand::parse("fade", &["t", "10", "20"])?;
//! assert_eq!(
//!     fade,
//!     EffectCommand::Fade(Fade::with_stop_position(
//!         auralis_effects::FadeCurve::Linear,
//!         FrameCount::new(10),
//!         FrameCount::new(20),
//!         FrameCount::new(10),
//!     ))
//! );
//! assert_eq!(fade.render_tokens(), ["fade", "t", "10", "20", "10"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::{
    fmt,
    num::{ParseFloatError, ParseIntError},
};

use auralis_core::{AuralisError, Decibels, FrameCount};
use thiserror::Error;

use crate::command_allpass::{parse_allpass, render_allpass};
use crate::command_band::{parse_band, render_band};
use crate::command_bandpass::{parse_bandpass, render_bandpass};
use crate::command_bandreject::{parse_bandreject, render_bandreject};
use crate::command_bass::{parse_bass, render_bass};
use crate::command_biquad::{parse_biquad, render_biquad};
use crate::command_centercut::{parse_centercut, render_centercut};
use crate::command_channels::{parse_channels, render_channels};
use crate::command_contrast::{parse_contrast, render_contrast};
use crate::command_dcshift::{parse_dc_shift, render_dc_shift};
use crate::command_equalizer::{parse_equalizer, render_equalizer};
use crate::command_fade::{parse_fade, render_fade};
use crate::command_gain::{parse_gain, render_gain};
use crate::command_norm::{parse_norm, render_norm};
use crate::command_oops::parse_oops;
use crate::command_overdrive::{parse_overdrive, render_overdrive};
use crate::command_pad::{parse_pad, render_pad};
use crate::command_remix::{parse_remix, render_remix};
use crate::command_repeat::{parse_repeat, render_repeat};
use crate::command_reverse::parse_reverse;
use crate::command_saturation::{parse_saturation, render_saturation};
use crate::command_softvol::{parse_softvol, render_softvol};
use crate::command_swap::parse_swap;
use crate::command_treble::{parse_treble, render_treble};
use crate::command_tremolo::{parse_tremolo, render_tremolo};
use crate::command_trim::{parse_trim, render_trim};
use crate::command_vol::{parse_vol, render_vol};
use crate::{
    AllPass, Band, BandPass, BandReject, Bass, Biquad, Centercut, Channels, Contrast, DcShift,
    EffectError, EffectKind, EffectNameError, EffectRegistry, Equalizer, Fade, Gain, Norm, Oops,
    Overdrive, Pad, Remix, Repeat, Reverse, Saturation, SoftVol, Swap, Treble, Tremolo, Trim, Vol,
};

/// Crate-local result type for command parsing.
pub type CommandResult<T> = std::result::Result<T, EffectCommandParseError>;

/// A typed command for one currently implemented Auralis effect.
///
/// This enum is the command-model boundary: parsing may start from tokenized
/// command strings, but successful results contain typed effect processors
/// only. It intentionally models the currently implemented Auralis subset:
/// future SoX-ng effects are rejected until their corresponding features are
/// implemented.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EffectCommand {
    /// SoX-ng-style all-pass filter family.
    AllPass(AllPass),

    /// SoX-ng-style resonator band-pass filter.
    Band(Band),

    /// SoX-ng-style RBJ band-pass filter.
    BandPass(BandPass),

    /// SoX-ng-style RBJ band-reject filter.
    BandReject(BandReject),

    /// SoX-ng-style bass tone control.
    Bass(Bass),

    /// SoX-ng-style direct coefficient biquad IIR filter.
    Biquad(Biquad),

    /// SoX-ng-style center-cut stereo separation.
    Centercut(Centercut),

    /// SoX-ng-style explicit channel-count conversion.
    Channels(Channels),

    /// SoX-ng-style phase contrast enhancement.
    Contrast(Contrast),

    /// Constant normalized full-scale offset.
    DcShift(DcShift),

    /// SoX-ng-style peaking equalizer filter.
    Equalizer(Equalizer),

    /// SoX-ng-style fade curve, fade-in, and optional positional fade-out.
    Fade(Fade),

    /// Constant gain in decibels.
    Gain(Gain),

    /// SoX-ng-style whole-buffer peak normalization.
    Norm(Norm),

    /// SoX-ng-style out-of-phase stereo extraction.
    Oops(Oops),

    /// SoX-ng-style overdrive distortion.
    Overdrive(Overdrive),

    /// Zero padding measured in frames, including optional positioned insertions.
    Pad(Pad),

    /// SoX-ng-style finite output repetition.
    Repeat(Repeat),

    /// SoX-ng-style basic channel routing.
    Remix(Remix),

    /// Frame-order reversal within each channel.
    Reverse(Reverse),

    /// SoX-ng-style saturation distortion.
    Saturation(Saturation),

    /// SoX-ng-style soft volume control.
    SoftVol(SoftVol),

    /// SoX-ng-style adjacent channel-pair swapping.
    Swap(Swap),

    /// SoX-ng-style treble tone control.
    Treble(Treble),

    /// SoX-ng-style sinusoidal tremolo modulation.
    Tremolo(Tremolo),

    /// End-exclusive frame range selection.
    Trim(Trim),

    /// SoX-ng-style volume scaling with optional limiter gain.
    Vol(Vol),
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
            EffectKind::AllPass => parse_allpass(effect, args),
            EffectKind::Band => parse_band(effect, args),
            EffectKind::BandPass => parse_bandpass(effect, args),
            EffectKind::BandReject => parse_bandreject(effect, args),
            EffectKind::Bass => parse_bass(effect, args),
            EffectKind::Biquad => parse_biquad(effect, args),
            EffectKind::Centercut => parse_centercut(effect, args),
            EffectKind::Channels => parse_channels(effect, args),
            EffectKind::Contrast => parse_contrast(effect, args),
            EffectKind::DcShift => parse_dc_shift(effect, args),
            EffectKind::Equalizer => parse_equalizer(effect, args),
            EffectKind::Fade => parse_fade(effect, args),
            EffectKind::Gain => parse_gain(effect, args),
            EffectKind::Norm => parse_norm(effect, args),
            EffectKind::Oops => parse_oops(effect, args),
            EffectKind::Overdrive => parse_overdrive(effect, args),
            EffectKind::Pad => parse_pad(effect, args),
            EffectKind::Repeat => parse_repeat(effect, args),
            EffectKind::Remix => parse_remix(effect, args),
            EffectKind::Reverse => parse_reverse(effect, args),
            EffectKind::Saturation => parse_saturation(effect, args),
            EffectKind::SoftVol => parse_softvol(effect, args),
            EffectKind::Swap => parse_swap(effect, args),
            EffectKind::Treble => parse_treble(effect, args),
            EffectKind::Tremolo => parse_tremolo(effect, args),
            EffectKind::Trim => parse_trim(effect, args),
            EffectKind::Vol => parse_vol(effect, args),
        }
    }

    /// Returns the typed effect kind represented by this command.
    #[must_use]
    pub const fn kind(&self) -> EffectKind {
        match self {
            Self::AllPass(_) => EffectKind::AllPass,
            Self::Band(_) => EffectKind::Band,
            Self::BandPass(_) => EffectKind::BandPass,
            Self::BandReject(_) => EffectKind::BandReject,
            Self::Bass(_) => EffectKind::Bass,
            Self::Biquad(_) => EffectKind::Biquad,
            Self::Centercut(_) => EffectKind::Centercut,
            Self::Channels(_) => EffectKind::Channels,
            Self::Contrast(_) => EffectKind::Contrast,
            Self::DcShift(_) => EffectKind::DcShift,
            Self::Equalizer(_) => EffectKind::Equalizer,
            Self::Fade(_) => EffectKind::Fade,
            Self::Gain(_) => EffectKind::Gain,
            Self::Norm(_) => EffectKind::Norm,
            Self::Oops(_) => EffectKind::Oops,
            Self::Overdrive(_) => EffectKind::Overdrive,
            Self::Pad(_) => EffectKind::Pad,
            Self::Repeat(_) => EffectKind::Repeat,
            Self::Remix(_) => EffectKind::Remix,
            Self::Reverse(_) => EffectKind::Reverse,
            Self::Saturation(_) => EffectKind::Saturation,
            Self::SoftVol(_) => EffectKind::SoftVol,
            Self::Swap(_) => EffectKind::Swap,
            Self::Treble(_) => EffectKind::Treble,
            Self::Tremolo(_) => EffectKind::Tremolo,
            Self::Trim(_) => EffectKind::Trim,
            Self::Vol(_) => EffectKind::Vol,
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
    pub fn render_tokens(&self) -> Vec<String> {
        match self {
            Self::AllPass(all_pass) => render_allpass(*all_pass),
            Self::Band(band) => render_band(*band),
            Self::BandPass(band_pass) => render_bandpass(*band_pass),
            Self::BandReject(band_reject) => render_bandreject(*band_reject),
            Self::Bass(bass) => render_bass(*bass),
            Self::Biquad(biquad) => render_biquad(*biquad),
            Self::Centercut(centercut) => render_centercut(*centercut),
            Self::Channels(channels) => render_channels(*channels),
            Self::Contrast(contrast) => render_contrast(*contrast),
            Self::DcShift(dc_shift) => render_dc_shift(*dc_shift),
            Self::Equalizer(equalizer) => render_equalizer(*equalizer),
            Self::Fade(fade) => render_fade(*fade),
            Self::Gain(gain) => render_gain(*gain),
            Self::Norm(norm) => render_norm(*norm),
            Self::Oops(_) => vec!["oops".to_owned()],
            Self::Overdrive(overdrive) => render_overdrive(*overdrive),
            Self::Pad(pad) => render_pad(pad),
            Self::Repeat(repeat) => render_repeat(*repeat),
            Self::Remix(remix) => render_remix(remix),
            Self::Reverse(_) => vec!["reverse".to_owned()],
            Self::Saturation(saturation) => render_saturation(*saturation),
            Self::SoftVol(softvol) => render_softvol(*softvol),
            Self::Swap(_) => vec!["swap".to_owned()],
            Self::Treble(treble) => render_treble(*treble),
            Self::Tremolo(tremolo) => render_tremolo(*tremolo),
            Self::Trim(trim) => render_trim(trim),
            Self::Vol(vol) => render_vol(*vol),
        }
    }
}

impl fmt::Display for EffectCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.render_tokens().join(" "))
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

    /// Mutually exclusive options were combined for an effect command.
    #[error("invalid option combination for effect `{effect}`: {options}")]
    InvalidOptionCombination {
        /// Canonical effect name.
        effect: &'static str,

        /// Human-readable option combination.
        options: &'static str,
    },
}

pub(super) fn required_arg<'args>(
    effect: &'static str,
    args: &'args [&'args str],
    argument: &'static str,
) -> CommandResult<&'args str> {
    args.first()
        .copied()
        .ok_or(EffectCommandParseError::MissingArgument { effect, argument })
}

pub(super) fn parse_decibels(
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

pub(super) fn parse_f64(
    effect: &'static str,
    argument: &'static str,
    value: &str,
) -> CommandResult<f64> {
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

pub(super) fn parse_f32(
    effect: &'static str,
    argument: &'static str,
    value: &str,
) -> CommandResult<f32> {
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

pub(super) fn parse_frame_count(
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

pub(super) fn reject_extra_arguments(effect: &'static str, args: &[&str]) -> CommandResult<()> {
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

pub(crate) fn is_option_like(value: &str) -> bool {
    value.starts_with('-') && value.parse::<f64>().is_err()
}

pub(super) fn render_f64(value: f64) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

pub(super) fn render_f32(value: f32) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{EffectCommand, EffectCommandParseError, parse_effect_command};
    use crate::{
        Centercut, Contrast, DcShift, EffectError, Fade, FadeCurve, Gain, GainChannelMode, Pad,
        PositionedPad, Saturation, SoftVol, Trim, TrimPosition,
    };
    use auralis_core::{Decibels, FrameCount};

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
            parse_effect_command(&["contrast"]).unwrap(),
            EffectCommand::Contrast(Contrast::default_amount())
        );
        assert_eq!(
            parse_effect_command(&["centercut"]).unwrap(),
            EffectCommand::Centercut(Centercut::new())
        );
        assert_eq!(
            parse_effect_command(&["pad"]).unwrap(),
            EffectCommand::Pad(Pad::new(FrameCount::new(0), FrameCount::new(0)))
        );
        assert_eq!(
            parse_effect_command(&["softvol"]).unwrap(),
            EffectCommand::SoftVol(SoftVol::default())
        );
        assert_eq!(
            parse_effect_command(&["saturation"]).unwrap(),
            EffectCommand::Saturation(Saturation::default())
        );
        assert_eq!(
            parse_effect_command(&["fade", "3"]).unwrap(),
            EffectCommand::Fade(Fade::with_curve(
                FadeCurve::Logarithmic,
                FrameCount::new(3),
                FrameCount::new(0),
            ))
        );
    }

    #[test]
    fn sox_ng_fade_curve_types_parse_into_typed_configs() {
        let expected = [
            ("q", FadeCurve::QuarterSine),
            ("h", FadeCurve::HalfSine),
            ("l", FadeCurve::Logarithmic),
            ("t", FadeCurve::Linear),
            ("p", FadeCurve::InvertedParabola),
        ];

        for (token, curve) in expected {
            assert_eq!(
                parse_effect_command(&["fade", token, "3", "2"]).unwrap(),
                EffectCommand::Fade(Fade::with_stop_position(
                    curve,
                    FrameCount::new(3),
                    FrameCount::new(2),
                    FrameCount::new(3),
                ))
            );
        }
    }

    #[test]
    fn sox_ng_fade_stop_position_and_fade_out_length_parse_into_typed_config() {
        assert_eq!(
            parse_effect_command(&["fade", "t", "4", "0"]).unwrap(),
            EffectCommand::Fade(Fade::with_stop_position(
                FadeCurve::Linear,
                FrameCount::new(4),
                FrameCount::new(0),
                FrameCount::new(4),
            ))
        );
        assert_eq!(
            parse_effect_command(&["fade", "t", "4", "12", "3"]).unwrap(),
            EffectCommand::Fade(Fade::with_stop_position(
                FadeCurve::Linear,
                FrameCount::new(4),
                FrameCount::new(12),
                FrameCount::new(3),
            ))
        );
        assert_eq!(
            parse_effect_command(&["fade", "t", "4", "-0", "3"]).unwrap(),
            EffectCommand::Fade(Fade::with_stop_position(
                FadeCurve::Linear,
                FrameCount::new(4),
                FrameCount::new(0),
                FrameCount::new(3),
            ))
        );
    }

    #[test]
    fn parses_dc_shift_limiter_gain() {
        assert_eq!(
            parse_effect_command(&["dcshift", "0.5", "0.05"]).unwrap(),
            EffectCommand::DcShift(DcShift::with_limiter_gain(0.5, 0.05).unwrap())
        );
        assert_eq!(
            parse_effect_command(&["dcshift", "5e-1", "5e-2"])
                .unwrap()
                .render_tokens(),
            ["dcshift", "0.5", "0.05"]
        );
    }

    #[test]
    fn parses_sox_ng_positioned_pad_arguments() {
        assert_eq!(
            parse_effect_command(&["pad", "1", "2@3", "4@5", "6"]).unwrap(),
            EffectCommand::Pad(
                Pad::with_positioned(
                    FrameCount::new(1),
                    FrameCount::new(6),
                    [
                        PositionedPad::new(FrameCount::new(2), FrameCount::new(3)),
                        PositionedPad::new(FrameCount::new(4), FrameCount::new(5)),
                    ],
                )
                .unwrap()
            )
        );
        assert_eq!(
            parse_effect_command(&["pad", "2@-0"]).unwrap(),
            EffectCommand::Pad(Pad::new(FrameCount::new(0), FrameCount::new(2)))
        );
        assert_eq!(
            parse_effect_command(&["pad", "1", "2@3", "4"])
                .unwrap()
                .render_tokens(),
            ["pad", "1", "2@3", "4"]
        );
    }

    #[test]
    fn unsupported_options_name_the_effect_and_option() {
        let error = parse_effect_command(&["gain", "-q"]).unwrap_err();

        assert_eq!(
            error,
            EffectCommandParseError::UnsupportedOption {
                effect: "gain",
                option: "-q".to_owned(),
            }
        );
        assert_eq!(
            error.to_string(),
            "unsupported option `-q` for effect `gain`"
        );
    }

    #[test]
    fn parses_gain_channel_equalize_and_balance_options() {
        assert_eq!(
            parse_effect_command(&["gain", "-e", "-3"]).unwrap(),
            EffectCommand::Gain(
                Gain::new(Decibels::new(-3.0).unwrap())
                    .with_channel_mode(GainChannelMode::Equalize)
            )
        );
        assert_eq!(
            parse_effect_command(&["gain", "-B"]).unwrap(),
            EffectCommand::Gain(
                Gain::new(Decibels::new(0.0).unwrap()).with_channel_mode(GainChannelMode::Balance)
            )
        );
        assert_eq!(
            parse_effect_command(&["gain", "-bn", "-6"])
                .unwrap()
                .render_tokens(),
            ["gain", "-b", "-n", "-6"]
        );
    }

    #[test]
    fn parses_gain_normalize_and_limiter_options() {
        assert_eq!(
            parse_effect_command(&["gain", "-n", "-3"]).unwrap(),
            EffectCommand::Gain(Gain::normalize(Decibels::new(-3.0).unwrap()))
        );
        assert_eq!(
            parse_effect_command(&["gain", "-l", "6"]).unwrap(),
            EffectCommand::Gain(Gain::limiter(Decibels::new(6.0).unwrap()))
        );
        assert_eq!(
            parse_effect_command(&["gain", "-nl", "6"])
                .unwrap()
                .render_tokens(),
            ["gain", "-n", "-l", "6"]
        );
    }

    #[test]
    fn gain_rejects_sox_ng_option_combinations_that_are_mutually_exclusive() {
        let normalize_restore = parse_effect_command(&["gain", "-nr"]).unwrap_err();
        assert_eq!(
            normalize_restore,
            EffectCommandParseError::InvalidOptionCombination {
                effect: "gain",
                options: "only one of -n and -r may be given",
            }
        );

        let limiter_headroom = parse_effect_command(&["gain", "-lh", "-3"]).unwrap_err();
        assert_eq!(
            limiter_headroom,
            EffectCommandParseError::InvalidOptionCombination {
                effect: "gain",
                options: "only one of -l and -h may be given",
            }
        );

        let equalize_balance = parse_effect_command(&["gain", "-eB"]).unwrap_err();
        assert_eq!(
            equalize_balance,
            EffectCommandParseError::InvalidOptionCombination {
                effect: "gain",
                options: "only one of -e, -B, -b and -r may be given",
            }
        );
    }

    #[test]
    fn parses_gain_headroom_and_reclaim_options() {
        assert_eq!(
            parse_effect_command(&["gain", "-h", "-6"]).unwrap(),
            EffectCommand::Gain(Gain::reserve_headroom(Decibels::new(-6.0).unwrap()))
        );
        assert_eq!(
            parse_effect_command(&["gain", "-r"]).unwrap(),
            EffectCommand::Gain(Gain::reclaim_headroom(Decibels::new(0.0).unwrap()))
        );
        assert_eq!(
            parse_effect_command(&["gain", "-rh", "-3"])
                .unwrap()
                .render_tokens(),
            ["gain", "-rh", "-3"]
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
    fn parses_sox_ng_trim_positions() {
        assert_eq!(
            parse_effect_command(&["trim", "2"]).unwrap(),
            EffectCommand::Trim(
                Trim::with_positions([TrimPosition::absolute(FrameCount::new(2))]).unwrap()
            )
        );
        assert_eq!(
            parse_effect_command(&["trim", "2", "4", "=10", "-2", "-0"]).unwrap(),
            EffectCommand::Trim(
                Trim::with_positions([
                    TrimPosition::absolute(FrameCount::new(2)),
                    TrimPosition::relative(FrameCount::new(4)),
                    TrimPosition::absolute(FrameCount::new(10)),
                    TrimPosition::before_end(FrameCount::new(2)),
                    TrimPosition::End,
                ])
                .unwrap()
            )
        );
        assert_eq!(
            parse_effect_command(&["trim", "2s", "+4s", "=10s", "-2s", "-0"])
                .unwrap()
                .render_tokens(),
            ["trim", "2", "4", "=10", "-2", "-0"]
        );
    }

    #[test]
    fn invalid_numeric_values_are_rejected_before_effect_construction() {
        let error = parse_effect_command(&["trim", "not-a-frame", "2"]).unwrap_err();

        assert!(matches!(
            error,
            EffectCommandParseError::InvalidFrameCount {
                effect: "trim",
                argument: "position",
                value,
                ..
            } if value == "not-a-frame"
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
                &["gain", "-h"][..],
                &["gain", "-h", "0"][..],
                &["gain", "-h", "0"][..],
            ),
            (
                &["contrast"][..],
                &["contrast", "75"][..],
                &["contrast", "75"][..],
            ),
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
            (
                &["dc-shift", "5e-1", "5e-2"][..],
                &["dcshift", "0.5", "0.05"][..],
                &["dcshift", "0.5", "0.05"][..],
            ),
            (
                &["saturation"][..],
                &["saturation", "tanh", "1", "0", "1"][..],
                &["saturation", "tanh", "1", "0", "1"][..],
            ),
            (&["pad"][..], &["pad", "0", "0"][..], &["pad", "0", "0"][..]),
            (
                &["fade", "t", "3"][..],
                &["fade", "t", "3"][..],
                &["fade", "t", "3"][..],
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
                parse_effect_command(&["trim", "12", "22"]).unwrap(),
                &["trim", "12", "22"][..],
            ),
            (
                parse_effect_command(&["reverse"]).unwrap(),
                &["reverse"][..],
            ),
            (
                parse_effect_command(&["fade", "q", "4", "2"]).unwrap(),
                &["fade", "q", "4", "2", "4"][..],
            ),
            (
                parse_effect_command(&["fade", "q", "4", "0", "2"]).unwrap(),
                &["fade", "q", "4", "0", "2"][..],
            ),
        ];

        for (command, expected) in commands {
            assert_eq!(command.render_tokens(), expected);
        }
    }
}
