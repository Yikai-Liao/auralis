//! Typed parser for currently implemented effect commands.

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
use crate::command_bend::{parse_bend, render_bend};
use crate::command_biquad::{parse_biquad, render_biquad};
use crate::command_centercut::{parse_centercut, render_centercut};
use crate::command_channels::{parse_channels, render_channels};
use crate::command_chorus::{parse_chorus, render_chorus};
use crate::command_compand::{parse_compand, render_compand};
use crate::command_contrast::{parse_contrast, render_contrast};
use crate::command_dcshift::{parse_dc_shift, render_dc_shift};
use crate::command_deemph::parse_deemph;
use crate::command_delay::{parse_delay, render_delay};
use crate::command_downsample::{parse_downsample, render_downsample};
use crate::command_echo::{parse_echo, render_echo};
use crate::command_echos::{parse_echos, render_echos};
use crate::command_equalizer::{parse_equalizer, render_equalizer};
use crate::command_fade::{parse_fade, render_fade};
use crate::command_fir::{parse_fir, render_fir};
use crate::command_firfit::{parse_firfit, render_firfit};
use crate::command_flanger::{parse_flanger, render_flanger};
use crate::command_gain::{parse_gain, render_gain};
use crate::command_highpass::{parse_highpass, render_highpass};
use crate::command_hilbert::{parse_hilbert, render_hilbert};
use crate::command_loudness::{parse_loudness, render_loudness};
use crate::command_lowpass::{parse_lowpass, render_lowpass};
use crate::command_mcompand::{parse_mcompand, render_mcompand};
use crate::command_noiseprof::{parse_noiseprof, render_noiseprof};
use crate::command_noisered::{parse_noisered, render_noisered};
use crate::command_norm::{parse_norm, render_norm};
use crate::command_oops::parse_oops;
use crate::command_overdrive::{parse_overdrive, render_overdrive};
use crate::command_pad::{parse_pad, render_pad};
use crate::command_phaser::{parse_phaser, render_phaser};
use crate::command_pitch::{parse_pitch, render_pitch};
use crate::command_rate::{parse_rate, render_rate};
use crate::command_remix::{parse_remix, render_remix};
use crate::command_repeat::{parse_repeat, render_repeat};
use crate::command_reverb::{parse_reverb, render_reverb};
use crate::command_reverse::parse_reverse;
use crate::command_riaa::parse_riaa;
use crate::command_saturation::{parse_saturation, render_saturation};
use crate::command_silence::{parse_silence, render_silence};
use crate::command_sinc::{parse_sinc, render_sinc};
use crate::command_softvol::{parse_softvol, render_softvol};
use crate::command_speed::{parse_speed, render_speed};
use crate::command_splice::{parse_splice, render_splice};
use crate::command_stretch::{parse_stretch, render_stretch};
use crate::command_swap::parse_swap;
use crate::command_tempo::{parse_tempo, render_tempo};
use crate::command_treble::{parse_treble, render_treble};
use crate::command_tremolo::{parse_tremolo, render_tremolo};
use crate::command_trim::{parse_trim, render_trim};
use crate::command_upsample::{parse_upsample, render_upsample};
use crate::command_vad::{parse_vad, render_vad};
use crate::command_vol::{parse_vol, render_vol};
use crate::{
    AllPass, Band, BandPass, BandReject, Bass, Bend, Biquad, Centercut, Channels, Chorus, Compand,
    Contrast, DcShift, Deemph, Delay, Downsample, Echo, Echos, EffectError, EffectKind,
    EffectNameError, EffectRegistry, Equalizer, Fade, Fir, FirFit, Flanger, Gain, HighPass,
    Hilbert, Loudness, LowPass, MCompand, NoiseProf, NoiseRed, Norm, Oops, Overdrive, Pad, Phaser,
    Pitch, Rate, Remix, Repeat, Reverb, Reverse, Riaa, Saturation, Silence, Sinc, SoftVol, Speed,
    Splice, Stretch, Swap, Tempo, Treble, Tremolo, Trim, Upsample, Vad, Vol,
};

/// Crate-local result type for command parsing.
pub type CommandResult<T> = std::result::Result<T, EffectCommandParseError>;

/// A typed command for one currently implemented Auralis effect.
///
/// Successful parses contain typed effect processors.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EffectCommand {
    /// SoX-ng-style all-pass filter family.
    AllPass(AllPass),
    /// SoX-ng-style resonator band-pass filter.
    Band(Band),
    /// SoX-ng-style phase-vocoder pitch bend.
    Bend(Bend),
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
    /// SoX-ng-style chorus modulation.
    Chorus(Chorus),
    /// SoX-ng-style dynamic-range compander.
    Compand(Compand),
    /// SoX-ng-style phase contrast enhancement.
    Contrast(Contrast),
    /// Constant normalized full-scale offset.
    DcShift(DcShift),
    /// SoX-ng-style CD/DAT de-emphasis filter.
    Deemph(Deemph),
    /// SoX-ng-style per-channel delay.
    Delay(Delay),
    /// SoX-ng-style decimating downsample.
    Downsample(Downsample),
    /// SoX-ng-style parallel echo delay line.
    Echo(Echo),
    /// SoX-ng-style cascaded echo delay line.
    Echos(Echos),
    /// SoX-ng-style peaking equalizer filter.
    Equalizer(Equalizer),
    /// SoX-ng-style fade curve, fade-in, and optional positional fade-out.
    Fade(Fade),
    /// SoX-ng-style finite impulse response filter.
    Fir(Fir),
    /// SoX-ng-style FIR response-fitting filter.
    FirFit(FirFit),
    /// SoX-ng-style swept-delay flanger.
    Flanger(Flanger),
    /// Constant gain in decibels.
    Gain(Gain),
    /// SoX-ng-style high-pass filter family.
    HighPass(HighPass),
    /// SoX-ng-style Hilbert transform FIR filter.
    Hilbert(Hilbert),
    /// SoX-ng-style ISO 226 loudness compensation.
    Loudness(Loudness),
    /// SoX-ng-style low-pass filter family.
    LowPass(LowPass),
    /// SoX-ng-style multiband dynamic-range compander.
    MCompand(MCompand),
    /// SoX-ng-style noise profile analyzer.
    NoiseProf(NoiseProf),
    /// SoX-ng-style spectral noise reducer.
    NoiseRed(NoiseRed),
    /// SoX-ng-style whole-buffer peak normalization.
    Norm(Norm),
    /// SoX-ng-style out-of-phase stereo extraction.
    Oops(Oops),
    /// SoX-ng-style overdrive distortion.
    Overdrive(Overdrive),
    /// Zero padding measured in frames, including optional positioned insertions.
    Pad(Pad),
    /// SoX-ng-style phaser swept delay with feedback.
    Phaser(Phaser),
    /// SoX-ng-style pitch shift that preserves duration.
    Pitch(Pitch),
    /// SoX-ng-style sample-rate conversion scaffold.
    Rate(Rate),
    /// SoX-ng-style stereo reverberation.
    Reverb(Reverb),
    /// SoX-ng-style finite output repetition.
    Repeat(Repeat),
    /// SoX-ng-style basic channel routing.
    Remix(Remix),
    /// Frame-order reversal within each channel.
    Reverse(Reverse),
    /// SoX-ng-style RIAA vinyl playback equalization filter.
    Riaa(Riaa),
    /// SoX-ng-style saturation distortion.
    Saturation(Saturation),
    /// SoX-ng-style silence trimming.
    Silence(Silence),
    /// SoX-ng-style low-pass or high-pass windowed-sinc FIR filter.
    Sinc(Sinc),
    /// SoX-ng-style soft volume control.
    SoftVol(SoftVol),
    /// SoX-ng-style speed adjustment.
    Speed(Speed),
    /// SoX-ng-style cross-faded audio splice.
    Splice(Splice),
    /// SoX-ng-style basic time stretcher.
    Stretch(Stretch),
    /// SoX-ng-style adjacent channel-pair swapping.
    Swap(Swap),
    /// SoX-ng-style tempo adjustment that preserves pitch.
    Tempo(Tempo),
    /// SoX-ng-style treble tone control.
    Treble(Treble),
    /// SoX-ng-style sinusoidal tremolo modulation.
    Tremolo(Tremolo),
    /// End-exclusive frame range selection.
    Trim(Trim),
    /// SoX-ng-style zero-stuffing upsample.
    Upsample(Upsample),
    /// SoX-ng-style voice activity leading trim.
    Vad(Vad),
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
            EffectKind::Bend => parse_bend(effect, args),
            EffectKind::BandPass => parse_bandpass(effect, args),
            EffectKind::BandReject => parse_bandreject(effect, args),
            EffectKind::Bass => parse_bass(effect, args),
            EffectKind::Biquad => parse_biquad(effect, args),
            EffectKind::Centercut => parse_centercut(effect, args),
            EffectKind::Channels => parse_channels(effect, args),
            EffectKind::Chorus => parse_chorus(effect, args),
            EffectKind::Compand => parse_compand(effect, args),
            EffectKind::Contrast => parse_contrast(effect, args),
            EffectKind::DcShift => parse_dc_shift(effect, args),
            EffectKind::Deemph => parse_deemph(effect, args),
            EffectKind::Delay => parse_delay(effect, args),
            EffectKind::Downsample => parse_downsample(effect, args),
            EffectKind::Echo => parse_echo(effect, args),
            EffectKind::Echos => parse_echos(effect, args),
            EffectKind::Equalizer => parse_equalizer(effect, args),
            EffectKind::Fade => parse_fade(effect, args),
            EffectKind::Fir => parse_fir(effect, args),
            EffectKind::FirFit => parse_firfit(effect, args),
            EffectKind::Flanger => parse_flanger(effect, args),
            EffectKind::Gain => parse_gain(effect, args),
            EffectKind::HighPass => parse_highpass(effect, args),
            EffectKind::Hilbert => parse_hilbert(effect, args),
            EffectKind::Loudness => parse_loudness(effect, args),
            EffectKind::LowPass => parse_lowpass(effect, args),
            EffectKind::MCompand => parse_mcompand(effect, args),
            EffectKind::NoiseProf => parse_noiseprof(effect, args),
            EffectKind::NoiseRed => parse_noisered(effect, args),
            EffectKind::Norm => parse_norm(effect, args),
            EffectKind::Oops => parse_oops(effect, args),
            EffectKind::Overdrive => parse_overdrive(effect, args),
            EffectKind::Pad => parse_pad(effect, args),
            EffectKind::Phaser => parse_phaser(effect, args),
            EffectKind::Pitch => parse_pitch(effect, args),
            EffectKind::Rate => parse_rate(effect, args),
            EffectKind::Reverb => parse_reverb(effect, args),
            EffectKind::Repeat => parse_repeat(effect, args),
            EffectKind::Remix => parse_remix(effect, args),
            EffectKind::Reverse => parse_reverse(effect, args),
            EffectKind::Riaa => parse_riaa(effect, args),
            EffectKind::Saturation => parse_saturation(effect, args),
            EffectKind::Silence => parse_silence(effect, args),
            EffectKind::Sinc => parse_sinc(effect, args),
            EffectKind::SoftVol => parse_softvol(effect, args),
            EffectKind::Speed => parse_speed(effect, args),
            EffectKind::Splice => parse_splice(effect, args),
            EffectKind::Stretch => parse_stretch(effect, args),
            EffectKind::Swap => parse_swap(effect, args),
            EffectKind::Tempo => parse_tempo(effect, args),
            EffectKind::Treble => parse_treble(effect, args),
            EffectKind::Tremolo => parse_tremolo(effect, args),
            EffectKind::Trim => parse_trim(effect, args),
            EffectKind::Upsample => parse_upsample(effect, args),
            EffectKind::Vad => parse_vad(effect, args),
            EffectKind::Vol => parse_vol(effect, args),
        }
    }

    /// Returns the typed effect kind represented by this command.
    #[must_use]
    pub const fn kind(&self) -> EffectKind {
        match self {
            Self::AllPass(_) => EffectKind::AllPass,
            Self::Band(_) => EffectKind::Band,
            Self::Bend(_) => EffectKind::Bend,
            Self::BandPass(_) => EffectKind::BandPass,
            Self::BandReject(_) => EffectKind::BandReject,
            Self::Bass(_) => EffectKind::Bass,
            Self::Biquad(_) => EffectKind::Biquad,
            Self::Centercut(_) => EffectKind::Centercut,
            Self::Channels(_) => EffectKind::Channels,
            Self::Chorus(_) => EffectKind::Chorus,
            Self::Compand(_) => EffectKind::Compand,
            Self::Contrast(_) => EffectKind::Contrast,
            Self::DcShift(_) => EffectKind::DcShift,
            Self::Deemph(_) => EffectKind::Deemph,
            Self::Delay(_) => EffectKind::Delay,
            Self::Downsample(_) => EffectKind::Downsample,
            Self::Echo(_) => EffectKind::Echo,
            Self::Echos(_) => EffectKind::Echos,
            Self::Equalizer(_) => EffectKind::Equalizer,
            Self::Fade(_) => EffectKind::Fade,
            Self::Fir(_) => EffectKind::Fir,
            Self::FirFit(_) => EffectKind::FirFit,
            Self::Flanger(_) => EffectKind::Flanger,
            Self::Gain(_) => EffectKind::Gain,
            Self::HighPass(_) => EffectKind::HighPass,
            Self::Hilbert(_) => EffectKind::Hilbert,
            Self::Loudness(_) => EffectKind::Loudness,
            Self::LowPass(_) => EffectKind::LowPass,
            Self::MCompand(_) => EffectKind::MCompand,
            Self::NoiseProf(_) => EffectKind::NoiseProf,
            Self::NoiseRed(_) => EffectKind::NoiseRed,
            Self::Norm(_) => EffectKind::Norm,
            Self::Oops(_) => EffectKind::Oops,
            Self::Overdrive(_) => EffectKind::Overdrive,
            Self::Pad(_) => EffectKind::Pad,
            Self::Phaser(_) => EffectKind::Phaser,
            Self::Pitch(_) => EffectKind::Pitch,
            Self::Rate(_) => EffectKind::Rate,
            Self::Reverb(_) => EffectKind::Reverb,
            Self::Repeat(_) => EffectKind::Repeat,
            Self::Remix(_) => EffectKind::Remix,
            Self::Reverse(_) => EffectKind::Reverse,
            Self::Riaa(_) => EffectKind::Riaa,
            Self::Saturation(_) => EffectKind::Saturation,
            Self::Silence(_) => EffectKind::Silence,
            Self::Sinc(_) => EffectKind::Sinc,
            Self::SoftVol(_) => EffectKind::SoftVol,
            Self::Speed(_) => EffectKind::Speed,
            Self::Splice(_) => EffectKind::Splice,
            Self::Stretch(_) => EffectKind::Stretch,
            Self::Swap(_) => EffectKind::Swap,
            Self::Tempo(_) => EffectKind::Tempo,
            Self::Treble(_) => EffectKind::Treble,
            Self::Tremolo(_) => EffectKind::Tremolo,
            Self::Trim(_) => EffectKind::Trim,
            Self::Upsample(_) => EffectKind::Upsample,
            Self::Vad(_) => EffectKind::Vad,
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
            Self::Bend(bend) => render_bend(bend),
            Self::BandPass(band_pass) => render_bandpass(*band_pass),
            Self::BandReject(band_reject) => render_bandreject(*band_reject),
            Self::Bass(bass) => render_bass(*bass),
            Self::Biquad(biquad) => render_biquad(*biquad),
            Self::Centercut(centercut) => render_centercut(*centercut),
            Self::Channels(channels) => render_channels(*channels),
            Self::Chorus(chorus) => render_chorus(chorus),
            Self::Compand(compand) => render_compand(compand),
            Self::Contrast(contrast) => render_contrast(*contrast),
            Self::DcShift(dc_shift) => render_dc_shift(*dc_shift),
            Self::Deemph(_) => vec!["deemph".to_owned()],
            Self::Delay(delay) => render_delay(delay),
            Self::Downsample(downsample) => render_downsample(*downsample),
            Self::Echo(echo) => render_echo(echo),
            Self::Echos(echos) => render_echos(echos),
            Self::Equalizer(equalizer) => render_equalizer(*equalizer),
            Self::Fade(fade) => render_fade(*fade),
            Self::Fir(fir) => render_fir(fir),
            Self::FirFit(firfit) => render_firfit(firfit),
            Self::Flanger(flanger) => render_flanger(*flanger),
            Self::Gain(gain) => render_gain(*gain),
            Self::HighPass(high_pass) => render_highpass(*high_pass),
            Self::Hilbert(hilbert) => render_hilbert(*hilbert),
            Self::Loudness(loudness) => render_loudness(*loudness),
            Self::LowPass(low_pass) => render_lowpass(*low_pass),
            Self::MCompand(mcompand) => render_mcompand(mcompand),
            Self::NoiseProf(noiseprof) => render_noiseprof(noiseprof),
            Self::NoiseRed(noisered) => render_noisered(noisered),
            Self::Norm(norm) => render_norm(*norm),
            Self::Oops(_) => vec!["oops".to_owned()],
            Self::Overdrive(overdrive) => render_overdrive(*overdrive),
            Self::Pad(pad) => render_pad(pad),
            Self::Phaser(phaser) => render_phaser(*phaser),
            Self::Pitch(pitch) => render_pitch(*pitch),
            Self::Rate(rate) => render_rate(*rate),
            Self::Reverb(reverb) => render_reverb(*reverb),
            Self::Repeat(repeat) => render_repeat(*repeat),
            Self::Remix(remix) => render_remix(remix),
            Self::Reverse(_) => vec!["reverse".to_owned()],
            Self::Riaa(_) => vec!["riaa".to_owned()],
            Self::Saturation(saturation) => render_saturation(*saturation),
            Self::Silence(silence) => render_silence(silence),
            Self::Sinc(sinc) => render_sinc(*sinc),
            Self::SoftVol(softvol) => render_softvol(*softvol),
            Self::Speed(speed) => render_speed(*speed),
            Self::Splice(splice) => render_splice(splice),
            Self::Stretch(stretch) => render_stretch(*stretch),
            Self::Swap(_) => vec!["swap".to_owned()],
            Self::Tempo(tempo) => render_tempo(*tempo),
            Self::Treble(treble) => render_treble(*treble),
            Self::Tremolo(tremolo) => render_tremolo(*tremolo),
            Self::Trim(trim) => render_trim(trim),
            Self::Upsample(upsample) => render_upsample(*upsample),
            Self::Vad(vad) => render_vad(*vad),
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

    /// A delay position could not be parsed.
    #[error("effect `{effect}` argument `position` must be a SoX-ng position, got `{value}`")]
    InvalidDelayPosition {
        /// Canonical effect name.
        effect: &'static str,

        /// Original argument value.
        value: String,
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
mod tests;
