//! Effect name registry and SoX-ng coverage status.
//!
//! The registry maps stable effect names and aliases to Auralis typed effect
//! descriptors. It deliberately stores metadata only: typed processors such as
//! [`crate::Gain`] and [`crate::Trim`] remain the primary API for constructing
//! effects.
//!
//! # Examples
//!
//! ```
//! use auralis_effects::{EffectKind, EffectRegistry};
//!
//! let descriptor = EffectRegistry::resolve("dc-shift")?;
//! assert_eq!(descriptor.kind(), EffectKind::DcShift);
//! assert_eq!(descriptor.canonical_name(), "dcshift");
//! # Ok::<(), auralis_effects::EffectNameError>(())
//! ```

use std::{error::Error, fmt};

const MAX_SUGGESTIONS: usize = 3;

/// Stable typed identifier for an implemented Auralis effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EffectKind {
    /// SoX-ng-style all-pass filter family.
    AllPass,
    /// SoX-ng-style resonator band-pass filter.
    Band,
    /// SoX-ng-style phase-vocoder pitch bend.
    Bend,
    /// SoX-ng-style RBJ band-pass filter.
    BandPass,
    /// SoX-ng-style RBJ band-reject filter.
    BandReject,
    /// SoX-ng-style bass tone control.
    Bass,
    /// SoX-ng-style direct coefficient biquad IIR filter.
    Biquad,
    /// SoX-ng-style center-cut stereo separation.
    Centercut,
    /// SoX-ng-style explicit channel-count conversion.
    Channels,
    /// SoX-ng-style chorus modulation.
    Chorus,
    /// SoX-ng-style dynamic-range compander.
    Compand,
    /// SoX-ng-style phase contrast enhancement.
    Contrast,
    /// Constant normalized full-scale offset.
    DcShift,
    /// SoX-ng-style per-channel delay.
    Delay,
    /// SoX-ng-style decimating downsample.
    Downsample,
    /// SoX-ng-style parallel echo delay line.
    Echo,
    /// SoX-ng-style cascaded echo delay line.
    Echos,
    /// SoX-ng-style CD/DAT de-emphasis filter.
    Deemph,
    /// SoX-ng-style peaking equalizer filter.
    Equalizer,
    /// SoX-ng-style fade-in and optional positional fade-out envelope.
    Fade,
    /// SoX-ng-style finite impulse response filter.
    Fir,
    /// SoX-ng-style swept-delay flanger.
    Flanger,
    /// Constant gain in decibels.
    Gain,
    /// SoX-ng-style high-pass filter family.
    HighPass,
    /// SoX-ng-style ISO 226 loudness compensation.
    Loudness,
    /// SoX-ng-style low-pass filter family.
    LowPass,
    /// SoX-ng-style multiband dynamic-range compander.
    MCompand,
    /// SoX-ng-style noise profile analyzer.
    NoiseProf,
    /// SoX-ng-style spectral noise reducer.
    NoiseRed,
    /// Whole-buffer peak normalization.
    Norm,
    /// SoX-ng-style out-of-phase stereo extraction.
    Oops,
    /// SoX-ng-style overdrive distortion.
    Overdrive,
    /// Zero padding before and after the input.
    Pad,
    /// SoX-ng-style phaser swept delay with feedback.
    Phaser,
    /// SoX-ng-style pitch shift that preserves duration.
    Pitch,
    /// SoX-ng-style sample-rate conversion scaffold.
    Rate,
    /// SoX-ng-style stereo reverberation.
    Reverb,
    /// SoX-ng-style finite output repetition.
    Repeat,
    /// SoX-ng-style basic channel routing.
    Remix,
    /// Frame-order reversal within each channel.
    Reverse,
    /// SoX-ng-style RIAA vinyl playback equalization filter.
    Riaa,
    /// SoX-ng-style saturation distortion.
    Saturation,
    /// SoX-ng-style silence trimming.
    Silence,
    /// SoX-ng-style soft volume control.
    SoftVol,
    #[doc = "SoX-ng-style speed adjustment."]
    Speed,
    #[doc = "SoX-ng-style cross-faded audio splice."]
    Splice,
    #[doc = "SoX-ng-style basic time stretcher."]
    Stretch,
    #[doc = "SoX-ng-style adjacent channel-pair swapping."]
    Swap,
    #[doc = "SoX-ng-style tempo adjustment that preserves pitch."]
    Tempo,
    #[doc = "SoX-ng-style treble tone control."]
    Treble,
    #[doc = "SoX-ng-style sinusoidal tremolo modulation."]
    Tremolo,
    /// End-exclusive frame range selection.
    Trim,
    /// SoX-ng-style zero-stuffing upsample.
    Upsample,
    /// SoX-ng-style voice activity leading trim.
    Vad,
    /// SoX-ng-style volume scaling.
    Vol,
}

/// Metadata for one implemented effect name.
///
/// Descriptors are intentionally small and static. They identify the typed API
/// that should be used for construction without storing parsed effect options
/// or untyped configuration values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectDescriptor {
    kind: EffectKind,
    canonical_name: &'static str,
    aliases: &'static [&'static str],
    typed_api: &'static str,
    sox_ng_syntax: &'static str,
    summary: &'static str,
}

impl EffectDescriptor {
    /// Returns the typed effect kind.
    #[must_use]
    pub const fn kind(self) -> EffectKind {
        self.kind
    }

    /// Returns the stable canonical effect name.
    ///
    /// Canonical names match the SoX-ng effect name when Auralis currently
    /// covers the same command family.
    #[must_use]
    pub const fn canonical_name(self) -> &'static str {
        self.canonical_name
    }

    /// Returns additional names accepted by the registry.
    #[must_use]
    pub const fn aliases(self) -> &'static [&'static str] {
        self.aliases
    }

    /// Returns the public typed API entry point for this effect.
    #[must_use]
    pub const fn typed_api(self) -> &'static str {
        self.typed_api
    }

    /// Returns the covered SoX-ng command syntax family.
    #[must_use]
    pub const fn sox_ng_syntax(self) -> &'static str {
        self.sox_ng_syntax
    }

    /// Returns a short human-readable effect summary.
    #[must_use]
    pub const fn summary(self) -> &'static str {
        self.summary
    }

    #[must_use]
    const fn new(
        kind: EffectKind,
        canonical_name: &'static str,
        aliases: &'static [&'static str],
        typed_api: &'static str,
        sox_ng_syntax: &'static str,
        summary: &'static str,
    ) -> Self {
        Self {
            kind,
            canonical_name,
            aliases,
            typed_api,
            sox_ng_syntax,
            summary,
        }
    }

    #[must_use]
    fn matches_name(self, name: &str) -> bool {
        self.canonical_name == name || self.aliases.contains(&name)
    }
}

/// Implemented effects known to Auralis, in deterministic canonical-name order.
pub const SUPPORTED_EFFECTS: &[EffectDescriptor] = &[
    EffectDescriptor::new(
        EffectKind::AllPass,
        "allpass",
        &[],
        "AllPass",
        "allpass [-1|-2] frequency width",
        "apply a phase-shifting all-pass filter",
    ),
    EffectDescriptor::new(
        EffectKind::Band,
        "band",
        &[],
        "Band",
        "band [-n] frequency [width]",
        "apply a resonator band-pass filter",
    ),
    EffectDescriptor::new(
        EffectKind::Bend,
        "bend",
        &[],
        "Bend",
        "bend [-f frame-rate] [-o oversample] {start(+),cents,end(+)}",
        "apply a phase-vocoder pitch bend while preserving duration",
    ),
    EffectDescriptor::new(
        EffectKind::BandPass,
        "bandpass",
        &[],
        "BandPass",
        "bandpass [-c] frequency width",
        "apply an RBJ band-pass filter",
    ),
    EffectDescriptor::new(
        EffectKind::BandReject,
        "bandreject",
        &[],
        "BandReject",
        "bandreject frequency width",
        "apply an RBJ band-reject filter",
    ),
    EffectDescriptor::new(
        EffectKind::Bass,
        "bass",
        &[],
        "Bass",
        "bass gain [frequency [width]]",
        "apply a low-shelf bass tone control",
    ),
    EffectDescriptor::new(
        EffectKind::Biquad,
        "biquad",
        &[],
        "Biquad",
        "biquad b0 b1 b2 a0 a1 a2",
        "apply a direct coefficient second-order IIR filter",
    ),
    EffectDescriptor::new(
        EffectKind::Centercut,
        "centercut",
        &[],
        "Centercut",
        "centercut [-a gain] [-b] [-w size]",
        "separate stereo input into left residual, right residual, and center channels",
    ),
    EffectDescriptor::new(
        EffectKind::Channels,
        "channels",
        &[],
        "Channels",
        "channels number",
        "convert decoded audio to an explicit channel count",
    ),
    EffectDescriptor::new(
        EffectKind::Chorus,
        "chorus",
        &[],
        "Chorus",
        "chorus [-n|-l|-q] [-s|-t] [gain-in [gain-out [delay decay speed depth [-sine|-triangle]]...]]",
        "apply one or more modulated chorus delay lines",
    ),
    EffectDescriptor::new(
        EffectKind::Compand,
        "compand",
        &[],
        "Compand",
        "compand attack,decay{,attack,decay} [soft-knee-dB:]in-dB1[,out-dB1]{,in-dB2,out-dB2} [gain [initial-volume-dB [delay]]]",
        "apply dynamic-range companding with optional look-ahead delay",
    ),
    EffectDescriptor::new(
        EffectKind::MCompand,
        "mcompand",
        &[],
        "MCompand",
        "mcompand quoted_compand_args {crossover_frequency quoted_compand_args}",
        "apply dynamic-range companding independently across crossover bands",
    ),
    EffectDescriptor::new(
        EffectKind::NoiseProf,
        "noiseprof",
        &[],
        "NoiseProf",
        "noiseprof [profile-file(-)]",
        "collect a SoX-ng-style spectral noise profile while passing audio through",
    ),
    EffectDescriptor::new(
        EffectKind::NoiseRed,
        "noisered",
        &[],
        "NoiseRed",
        "noisered [profile-file(-) [amount(0.5)]]",
        "apply spectral noise reduction from a noise profile",
    ),
    EffectDescriptor::new(
        EffectKind::Contrast,
        "contrast",
        &[],
        "Contrast",
        "contrast [amount]",
        "apply phase contrast enhancement",
    ),
    EffectDescriptor::new(
        EffectKind::DcShift,
        "dcshift",
        &["dc-shift", "dc_shift"],
        "DcShift",
        "dcshift shift [limiter-gain]",
        "add a constant normalized full-scale offset",
    ),
    EffectDescriptor::new(
        EffectKind::Delay,
        "delay",
        &[],
        "Delay",
        "delay {position}",
        "delay decoded channels by independent positions",
    ),
    EffectDescriptor::new(
        EffectKind::Downsample,
        "downsample",
        &[],
        "Downsample",
        "downsample [factor]",
        "drop frames by a fixed integer decimation factor",
    ),
    EffectDescriptor::new(
        EffectKind::Echo,
        "echo",
        &[],
        "Echo",
        "echo gain-in gain-out <delay decay>",
        "add one or more parallel delayed echoes",
    ),
    EffectDescriptor::new(
        EffectKind::Echos,
        "echos",
        &[],
        "Echos",
        "echos gain-in gain-out <delay decay>",
        "add one or more cascaded delayed echoes",
    ),
    EffectDescriptor::new(
        EffectKind::Deemph,
        "deemph",
        &[],
        "Deemph",
        "deemph",
        "apply a CD/DAT de-emphasis filter",
    ),
    EffectDescriptor::new(
        EffectKind::Equalizer,
        "equalizer",
        &["eq"],
        "Equalizer",
        "equalizer frequency width gain",
        "apply an RBJ peaking equalizer filter",
    ),
    EffectDescriptor::new(
        EffectKind::Fade,
        "fade",
        &[],
        "Fade",
        "fade [type] fade-in-length [stop-position [fade-out-length]]",
        "apply a typed SoX-ng fade-in and fade-out envelope",
    ),
    EffectDescriptor::new(
        EffectKind::Fir,
        "fir",
        &[],
        "Fir",
        "fir [coefs-file | coef <coef>]",
        "apply a finite impulse response filter from coefficients",
    ),
    EffectDescriptor::new(
        EffectKind::Flanger,
        "flanger",
        &[],
        "Flanger",
        "flanger [-n|-l|-q] [-s|-t] [delay [depth [regen [width [speed [shape [phase [interp]]]]]]]]",
        "apply a swept-delay flanger with feedback",
    ),
    EffectDescriptor::new(
        EffectKind::Gain,
        "gain",
        &["gain-db", "gain_db"],
        "Gain",
        "gain [options] [gain-dB]",
        "apply gain with optional SoX-ng level management",
    ),
    EffectDescriptor::new(
        EffectKind::HighPass,
        "highpass",
        &[],
        "HighPass",
        "highpass [-1|-2] frequency [width]",
        "apply a high-pass filter",
    ),
    EffectDescriptor::new(
        EffectKind::Loudness,
        "loudness",
        &[],
        "Loudness",
        "loudness [gain [reference [n]]]",
        "apply ISO 226 equal-loudness compensation",
    ),
    EffectDescriptor::new(
        EffectKind::LowPass,
        "lowpass",
        &[],
        "LowPass",
        "lowpass [-1|-2] frequency [width]",
        "apply a low-pass filter",
    ),
    EffectDescriptor::new(
        EffectKind::Norm,
        "norm",
        &["normalize", "normalise"],
        "Norm",
        "norm [level]",
        "normalize peak level at this point in the effect chain",
    ),
    EffectDescriptor::new(
        EffectKind::Oops,
        "oops",
        &[],
        "Oops",
        "oops",
        "extract out-of-phase stereo by subtracting channel 2 from channel 1",
    ),
    EffectDescriptor::new(
        EffectKind::Overdrive,
        "overdrive",
        &[],
        "Overdrive",
        "overdrive [gain [color]]",
        "apply stateful cubic soft-clipping overdrive",
    ),
    EffectDescriptor::new(
        EffectKind::Pad,
        "pad",
        &[],
        "Pad",
        "pad {length[@position]}",
        "add zero-valued frames before, after, or inside the input",
    ),
    EffectDescriptor::new(
        EffectKind::Phaser,
        "phaser",
        &[],
        "Phaser",
        "phaser [-n|-l|-q] [-s|-t] [gain-in [gain-out [delay [regen [speed [-s|-t]]]]]]",
        "apply a swept-delay phaser with feedback",
    ),
    EffectDescriptor::new(
        EffectKind::Pitch,
        "pitch",
        &[],
        "Pitch",
        "pitch [-q] shift [segment [search [overlap]]]",
        "shift pitch in cents while preserving duration",
    ),
    EffectDescriptor::new(
        EffectKind::Rate,
        "rate",
        &[],
        "Rate",
        "rate [quality/options] frequency",
        "convert decoded audio to an explicit sample rate with SoX-ng-style quality metadata",
    ),
    EffectDescriptor::new(
        EffectKind::Repeat,
        "repeat",
        &[],
        "Repeat",
        "repeat [count]",
        "append finite copies of the input audio",
    ),
    EffectDescriptor::new(
        EffectKind::Reverb,
        "reverb",
        &[],
        "Reverb",
        "reverb [-w] [reverberance [HF-damping [room-scale [stereo-depth [pre-delay [wet-gain]]]]]]",
        "apply stereo reverberation with optional wet-only output",
    ),
    EffectDescriptor::new(
        EffectKind::Remix,
        "remix",
        &[],
        "Remix",
        "remix out-spec...",
        "route and mix decoded input channels into explicit output channels",
    ),
    EffectDescriptor::new(
        EffectKind::Reverse,
        "reverse",
        &[],
        "Reverse",
        "reverse",
        "reverse frame order within each channel",
    ),
    EffectDescriptor::new(
        EffectKind::Riaa,
        "riaa",
        &[],
        "Riaa",
        "riaa",
        "apply RIAA vinyl playback equalization",
    ),
    EffectDescriptor::new(
        EffectKind::Saturation,
        "saturation",
        &[],
        "Saturation",
        "saturation [type [blend [offset [drive|color|threshold]]]]",
        "apply nonlinear saturation distortion",
    ),
    EffectDescriptor::new(
        EffectKind::Silence,
        "silence",
        &[],
        "Silence",
        "silence [-l] above-periods [duration threshold] [below-periods duration threshold]",
        "trim leading, trailing, or middle silence",
    ),
    EffectDescriptor::new(
        EffectKind::Vad,
        "vad",
        &[],
        "Vad",
        "vad [-b time] [-N time] [-n time] [-r amount] [-f freq] [-m time] [-M time] [-h freq] [-l freq] [-H freq] [-L freq] [-T time] [-t level] [-s time] [-g time] [-p time]",
        "trim leading non-voice audio using a SoX-ng-style VAD command profile",
    ),
    EffectDescriptor::new(
        EffectKind::SoftVol,
        "softvol",
        &["soft-volume", "soft_volume"],
        "SoftVol",
        "softvol [volume [double-time [headroom]]]",
        "apply soft volume scaling that avoids clipping",
    ),
    EffectDescriptor::new(
        EffectKind::Speed,
        "speed",
        &[],
        "Speed",
        "speed factor[c]",
        "change pitch and tempo together by adjusting sample-rate metadata",
    ),
    EffectDescriptor::new(
        EffectKind::Splice,
        "splice",
        &[],
        "Splice",
        "splice [-h|-t|-q] {position[,excess[,leeway]]}",
        "remove excess audio around splice points and cross-fade the joins",
    ),
    EffectDescriptor::new(
        EffectKind::Stretch,
        "stretch",
        &[],
        "Stretch",
        "stretch [factor [window [fade [shift [fading]]]]]",
        "change duration with SoX-ng's basic windowed cross-fade stretcher",
    ),
    EffectDescriptor::new(
        EffectKind::Swap,
        "swap",
        &[],
        "Swap",
        "swap",
        "swap adjacent decoded channel pairs",
    ),
    EffectDescriptor::new(
        EffectKind::Tempo,
        "tempo",
        &[],
        "Tempo",
        "tempo factor",
        "change tempo while preserving pitch with the default profile",
    ),
    EffectDescriptor::new(
        EffectKind::Treble,
        "treble",
        &[],
        "Treble",
        "treble gain [frequency [width]]",
        "apply a high-shelf treble tone control",
    ),
    EffectDescriptor::new(
        EffectKind::Tremolo,
        "tremolo",
        &[],
        "Tremolo",
        "tremolo speed [depth]",
        "apply sinusoidal low-frequency amplitude modulation",
    ),
    EffectDescriptor::new(
        EffectKind::Trim,
        "trim",
        &[],
        "Trim",
        "trim start [length]",
        "keep an end-exclusive frame or seconds range",
    ),
    EffectDescriptor::new(
        EffectKind::Upsample,
        "upsample",
        &[],
        "Upsample",
        "upsample [factor]",
        "insert zero-valued frames by a fixed integer factor",
    ),
    EffectDescriptor::new(
        EffectKind::Vol,
        "vol",
        &["volume"],
        "Vol",
        "vol gain [a|p|d(a) [limitergain]]",
        "apply volume scaling with optional limiter gain",
    ),
];
pub use crate::registry_known::KNOWN_SOX_NG_EFFECTS;
/// Registry namespace for effect name resolution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectRegistry;
impl EffectRegistry {
    /// Creates a registry value.
    ///
    /// The registry is stateless; associated methods may also be called
    /// directly when a value is not needed.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
    /// Returns descriptors for all implemented effects.
    #[must_use]
    pub const fn supported_effects() -> &'static [EffectDescriptor] {
        SUPPORTED_EFFECTS
    }
    /// Returns the SoX-ng effect surface tracked by Auralis.
    #[must_use]
    pub const fn known_sox_ng_effects() -> &'static [&'static str] {
        KNOWN_SOX_NG_EFFECTS
    }
    /// Resolves a name or alias to an implemented typed effect descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectNameError::EmptyName`] for an empty name,
    /// [`EffectNameError::UnsupportedSoxNgEffect`] for a known SoX-ng effect
    /// without Auralis coverage, and [`EffectNameError::UnknownEffect`] for
    /// arbitrary unknown names. Unknown-name diagnostics include deterministic
    /// suggestions when a close name is available.
    pub fn resolve(name: &str) -> Result<&'static EffectDescriptor, EffectNameError> {
        if name.is_empty() {
            return Err(EffectNameError::EmptyName);
        }

        if let Some(descriptor) = descriptor_for_name(name) {
            return Ok(descriptor);
        }
        if Self::is_known_sox_ng_effect(name) {
            return Err(EffectNameError::UnsupportedSoxNgEffect {
                name: name.to_owned(),
            });
        }

        Err(EffectNameError::UnknownEffect {
            name: name.to_owned(),
            suggestions: suggest_effect_names(name),
        })
    }

    /// Returns whether `name` is in the tracked SoX-ng effect surface.
    #[must_use]
    pub fn is_known_sox_ng_effect(name: &str) -> bool {
        KNOWN_SOX_NG_EFFECTS.contains(&name)
    }
}

/// Resolves a name or alias to an implemented typed effect descriptor.
///
/// This is a convenience wrapper around [`EffectRegistry::resolve`].
///
/// # Errors
///
/// Returns [`EffectNameError`] when the name is empty, unsupported, or unknown.
pub fn resolve_effect_name(name: &str) -> Result<&'static EffectDescriptor, EffectNameError> {
    EffectRegistry::resolve(name)
}

/// Errors produced by effect name resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EffectNameError {
    /// The caller supplied an empty effect name.
    EmptyName,

    /// The name is a known SoX-ng effect, but Auralis has no coverage entry yet.
    UnsupportedSoxNgEffect {
        /// The unsupported SoX-ng effect name.
        name: String,
    },

    /// The name is not known to Auralis or the tracked SoX-ng surface.
    UnknownEffect {
        /// The unknown effect name.
        name: String,

        /// Suggested canonical effect names.
        suggestions: Vec<&'static str>,
    },
}

impl EffectNameError {
    /// Returns suggested canonical effect names for unknown names.
    #[must_use]
    pub fn suggestions(&self) -> &[&'static str] {
        match self {
            Self::UnknownEffect { suggestions, .. } => suggestions,
            Self::EmptyName | Self::UnsupportedSoxNgEffect { .. } => &[],
        }
    }
}

impl fmt::Display for EffectNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("effect name cannot be empty"),
            Self::UnsupportedSoxNgEffect { name } => write!(
                formatter,
                "known SoX-ng effect `{name}` is not implemented by Auralis; missing SoX-ng coverage entry for `{name}`"
            ),
            Self::UnknownEffect { name, suggestions } if suggestions.is_empty() => {
                write!(formatter, "unknown effect `{name}`")
            }
            Self::UnknownEffect { name, suggestions } if suggestions.len() == 1 => {
                write!(
                    formatter,
                    "unknown effect `{name}`; did you mean `{}`?",
                    suggestions[0]
                )
            }
            Self::UnknownEffect { name, suggestions } => {
                write!(formatter, "unknown effect `{name}`; did you mean one of ")?;
                write_suggestions(formatter, suggestions)?;
                formatter.write_str("?")
            }
        }
    }
}

impl Error for EffectNameError {}

fn descriptor_for_name(name: &str) -> Option<&'static EffectDescriptor> {
    SUPPORTED_EFFECTS
        .iter()
        .find(|descriptor| descriptor.matches_name(name))
}

fn suggest_effect_names(name: &str) -> Vec<&'static str> {
    let mut ranked = Vec::new();

    for descriptor in SUPPORTED_EFFECTS {
        consider_suggestion(
            &mut ranked,
            name,
            descriptor.canonical_name(),
            descriptor.canonical_name(),
        );
        for alias in descriptor.aliases() {
            consider_suggestion(&mut ranked, name, alias, descriptor.canonical_name());
        }
    }

    ranked.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(right.0)));
    ranked.truncate(MAX_SUGGESTIONS);

    ranked
        .into_iter()
        .map(|(suggestion, _distance)| suggestion)
        .collect()
}

fn consider_suggestion(
    ranked: &mut Vec<(&'static str, usize)>,
    query: &str,
    candidate_name: &'static str,
    suggestion: &'static str,
) {
    let distance = edit_distance(query, candidate_name);
    if distance > suggestion_threshold(candidate_name)
        && !candidate_name.starts_with(query)
        && !query.starts_with(candidate_name)
    {
        return;
    }

    if let Some((_existing, existing_distance)) = ranked
        .iter_mut()
        .find(|(existing, _distance)| *existing == suggestion)
    {
        *existing_distance = (*existing_distance).min(distance);
    } else {
        ranked.push((suggestion, distance));
    }
}

fn suggestion_threshold(candidate: &str) -> usize {
    match candidate.chars().count() {
        0..=3 => 1,
        4..=8 => 2,
        _ => 3,
    }
}

fn edit_distance(left: &str, right: &str) -> usize {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut costs = (0..=right_chars.len()).collect::<Vec<_>>();

    for (left_index, left_char) in left_chars.iter().enumerate() {
        let mut previous_diagonal = costs[0];
        costs[0] = left_index + 1;

        for (right_index, right_char) in right_chars.iter().enumerate() {
            let previous_row = costs[right_index + 1];
            let insertion = previous_row + 1;
            let deletion = costs[right_index] + 1;
            let substitution = previous_diagonal + usize::from(left_char != right_char);

            costs[right_index + 1] = insertion.min(deletion).min(substitution);
            previous_diagonal = previous_row;
        }
    }

    costs[right_chars.len()]
}

fn write_suggestions(
    formatter: &mut fmt::Formatter<'_>,
    suggestions: &[&'static str],
) -> fmt::Result {
    for (index, suggestion) in suggestions.iter().enumerate() {
        if index > 0 {
            formatter.write_str(", ")?;
        }
        write!(formatter, "`{suggestion}`")?;
    }

    Ok(())
}
