use thiserror::Error;

/// Crate-local result type using [`EffectError`].
pub type Result<T> = std::result::Result<T, EffectError>;

/// Errors produced by typed effect processors.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum EffectError {
    /// The requested frame range has `start > end`.
    #[error("trim start frame must be less than or equal to trim end frame")]
    InvalidTrimOrder,

    /// The requested frame range extends beyond the buffer being trimmed.
    #[error("trim frame range must be within the input duration")]
    TrimRangeOutOfBounds,

    /// The requested padding would create a buffer shape that cannot be represented.
    #[error("pad frame count exceeds representable audio buffer length")]
    PadLengthOverflow,

    /// A positioned pad requested insertions that are not strictly ordered.
    #[error("pad positions must be in ascending order")]
    PadPositionsOutOfOrder,

    /// A positioned pad requested an insertion after the input duration.
    #[error("pad position must be within the input duration")]
    PadPositionOutOfBounds,

    /// A repeat count was outside SoX-ng's finite supported range.
    #[error("repeat count must be in the finite SoX-ng range 0..=4294967294")]
    InvalidRepeatCount,

    /// A repeat command would create a buffer shape that cannot be represented.
    #[error("repeat output frame count exceeds representable audio buffer length")]
    RepeatLengthOverflow,

    /// A delay position was not finite or was negative.
    #[error("delay positions must be finite and non-negative")]
    InvalidDelayPosition,

    /// A delay command supplied more positions than the input has channels.
    #[error("delay cannot specify more positions than the input channel count")]
    DelayTooManyPositions,

    /// A delay command would create a buffer shape that cannot be represented.
    #[error("delay output frame count exceeds representable audio buffer length")]
    DelayLengthOverflow,

    /// A dither command had invalid target precision.
    #[error("dither precision must be in the implemented SoX-ng range 2..=24 bits")]
    InvalidDither,

    /// An echo command had invalid gains, delay taps, or no delay taps.
    #[error(
        "echo gains and decays must be finite, delays must be finite and non-negative, and at least one delay-decay pair is required"
    )]
    InvalidEcho,

    /// An echo command would create a buffer shape that cannot be represented.
    #[error("echo output frame count exceeds representable audio buffer length")]
    EchoLengthOverflow,

    /// An echos command had invalid gains, delay taps, or no delay taps.
    #[error(
        "echos gains must be finite, delays must be finite and resolve to at least one frame, decays must be finite in 0..=1, and at least one delay-decay pair is required"
    )]
    InvalidEchos,

    /// An echos command would create a buffer shape that cannot be represented.
    #[error("echos output frame count exceeds representable audio buffer length")]
    EchosLengthOverflow,

    /// A chorus processor had invalid gains, delay parameters, or modulation settings.
    #[error(
        "chorus gains and decay must be finite in -1..=1, delay/depth must be finite and non-negative, speed must be finite and positive, and the resolved delay line must fit the output buffer"
    )]
    InvalidChorus,

    /// A chorus processor would create a buffer shape that cannot be represented.
    #[error("chorus output frame count exceeds representable audio buffer length")]
    ChorusLengthOverflow,

    /// A flanger processor had invalid gains, delay parameters, or modulation settings.
    #[error(
        "flanger delay/depth must be finite in 0..=1000 ms, regen in -100..=100, width non-negative, speed finite and positive, phase in 0..=100, and the resolved delay line must fit the output buffer"
    )]
    InvalidFlanger,

    /// A flanger processor would create a delay-line shape that cannot be represented.
    #[error("flanger delay line exceeds representable audio buffer length")]
    FlangerLengthOverflow,

    /// A phaser processor had invalid gains, delay parameters, or modulation settings.
    #[error(
        "phaser gains and regen must be finite in -1..=1, delay must be finite in 0..=1000 ms, speed finite and positive, and the resolved delay line must fit the output buffer"
    )]
    InvalidPhaser,

    /// A phaser processor would create a delay-line shape that cannot be represented.
    #[error("phaser delay line exceeds representable audio buffer length")]
    PhaserLengthOverflow,

    /// A reverb processor had invalid command parameters.
    #[error(
        "reverb percent parameters must be finite in 0..=100, pre-delay in 0..=500 ms, and wet gain in -10..=10 dB"
    )]
    InvalidReverb,

    /// A reverb processor would create a delay or output shape that cannot be represented.
    #[error("reverb delay or output shape exceeds representable audio buffer length")]
    ReverbLengthOverflow,

    /// A downsample factor was outside SoX-ng's supported range.
    #[error("downsample factor must be in the SoX-ng range 1..=16384")]
    InvalidDownsampleFactor,

    /// A downsample command would produce an unrepresentable sample rate.
    #[error("downsample output sample rate must be at least 1 Hz")]
    DownsampleRateTooLow,

    /// An upsample factor was outside SoX-ng's supported range.
    #[error("upsample factor must be in the SoX-ng range 1..=256")]
    InvalidUpsampleFactor,

    /// An upsample command would produce an unrepresentable sample rate.
    #[error("upsample output sample rate exceeds representable rate")]
    UpsampleRateOverflow,

    /// A speed factor was not finite or not positive.
    #[error("speed factor must be finite and greater than zero")]
    InvalidSpeedFactor,

    /// A speed command would produce an unrepresentable sample rate.
    #[error("speed output sample rate must round into the representable positive rate range")]
    SpeedRateOutOfRange,

    /// A stretch command had invalid window, factor, shift, or fading settings.
    #[error(
        "stretch factor/window/shift/fading must be finite, factor must be non-negative, window at least 1 ms, shift in (0, 1], and fading in 0..=0.5"
    )]
    InvalidStretch,

    /// A stretch command would create an unrepresentable state or output shape.
    #[error("stretch state or output frame count exceeds representable audio buffer length")]
    StretchLengthOverflow,

    /// A tempo factor was outside SoX-ng's supported range.
    #[error("tempo factor must be finite and in the SoX-ng range 0.1..=100")]
    InvalidTempoFactor,

    /// A tempo command had invalid tuning parameters.
    #[error(
        "tempo segment/search/overlap must be finite and in the SoX-ng ranges segment 10..=120 ms, search 0..=30 ms, and overlap 0..=30 ms"
    )]
    InvalidTempoTuning,

    /// A tempo command would create an unrepresentable state or output shape.
    #[error("tempo state or output frame count exceeds representable audio buffer length")]
    TempoLengthOverflow,

    /// A pitch shift was outside SoX-ng's supported cents-derived factor range.
    #[error("pitch shift must be finite and map to a factor in the SoX-ng range 0.01..=10")]
    InvalidPitchShift,

    /// A pitch command had invalid tuning parameters.
    #[error(
        "pitch segment/search/overlap must be finite and in the SoX-ng ranges segment 10..=120 ms, search 0..=30 ms, and overlap 0..=30 ms"
    )]
    InvalidPitchTuning,

    /// A pitch command would produce an unrepresentable sample rate.
    #[error("pitch output sample rate must round into the representable positive rate range")]
    PitchRateOutOfRange,

    /// A bend command had invalid options, positions, or pitch shift.
    #[error(
        "bend options must be in SoX-ng ranges -f 10..=80 and -o 4..=32, positions must be ordered, and cents must map to factor 0.01..=10"
    )]
    InvalidBend,

    /// A bend command would create an unrepresentable STFT state or position.
    #[error("bend STFT state or resolved position exceeds representable audio buffer length")]
    BendLengthOverflow,

    /// A splice command had invalid positions, excess, or leeway.
    #[error(
        "splice requires at least one point, finite non-negative positions, strictly increasing starts, and excess no longer than the splice position"
    )]
    InvalidSplice,

    /// A splice command would create a buffer shape that cannot be represented.
    #[error("splice state or output frame count exceeds representable audio buffer length")]
    SpliceLengthOverflow,

    /// A compand command had invalid timing, transfer, gain, volume, or delay parameters.
    #[error(
        "compand attack/decay times, transfer points, gain, initial volume, and delay must be finite SoX-ng-compatible values"
    )]
    InvalidCompand,

    /// An mcompand command had invalid bands, crossover frequencies, or unsupported delays.
    #[error(
        "mcompand bands must use valid compand settings, ascending positive crossover frequencies, and zero delay"
    )]
    InvalidMCompand,

    /// A noise profile command had invalid profile output metadata.
    #[error("noiseprof profile output path and collected channel profile must be valid")]
    InvalidNoiseProfile,

    /// A noise reduction command had invalid profile input, amount, or channel shape.
    #[error("noisered profile input, amount, and channel shape must be valid")]
    InvalidNoiseReduction,

    /// A noise reduction command would create a buffer shape that cannot be represented.
    #[error("noisered output frame count exceeds representable audio buffer length")]
    NoiseReductionLengthOverflow,

    /// A loudness command had invalid gain, reference level, or filter length.
    #[error("loudness gain must be in -50..=15 dB, reference in 50..=75 dB, and n in 127..=2047")]
    InvalidLoudness,

    /// A FIR coefficient source had invalid numeric coefficients or path metadata.
    #[error("fir coefficients must be finite numbers and coefficient file paths must be non-empty")]
    InvalidFirCoefficients,

    /// A FIR response-fitting source had invalid knot data or path metadata.
    #[error(
        "firfit knots must be finite frequency/gain pairs with strictly increasing non-negative frequencies"
    )]
    InvalidFirFit,

    /// A FIR command would create a buffer shape that cannot be represented.
    #[error("fir output frame count exceeds representable audio buffer length")]
    FirLengthOverflow,

    /// A Hilbert transform command had an invalid tap count.
    #[error("hilbert taps must be an odd value in the SoX-ng range 3..=1073741823")]
    InvalidHilbert,

    /// A sinc FIR filter command had invalid frequencies or design options.
    #[error("sinc frequency and design options must be finite and in SoX-ng-compatible ranges")]
    InvalidSinc,

    /// A silence command had invalid periods, durations, thresholds, or options.
    #[error(
        "silence periods, durations, thresholds, and -l usage must match SoX-ng-compatible ranges"
    )]
    InvalidSilence,

    /// A silence command would create a buffer shape that cannot be represented.
    #[error("silence output frame count exceeds representable audio buffer length")]
    SilenceLengthOverflow,

    /// A VAD processor had invalid threshold or timing settings.
    #[error("vad threshold must be finite in 0..=1 and timing settings must be representable")]
    InvalidVad,

    /// A VAD command would create a buffer shape that cannot be represented.
    #[error("vad output frame count exceeds representable audio buffer length")]
    VadLengthOverflow,

    /// A rate command would create a buffer shape that cannot be represented.
    #[error("rate output frame count exceeds representable audio buffer length")]
    RateLengthOverflow,

    /// A rate command combined incompatible SoX-ng quality and override options.
    #[error(
        "rate override options require medium or higher quality and must satisfy SoX-ng bandwidth constraints"
    )]
    InvalidRateOptions,

    /// A biquad coefficient was not finite or had an invalid `a0` normalizer.
    #[error("biquad coefficients must be finite and a0 must be nonzero")]
    InvalidBiquadCoefficients,

    /// A biquad coefficient helper received an invalid design parameter.
    #[error(
        "biquad design parameters must be finite, positive, and below Nyquist where applicable"
    )]
    InvalidBiquadDesign,

    /// A remix command used an invalid basic routing specification.
    #[error("remix output specifications must contain channel numbers, ranges, or a standalone 0")]
    InvalidRemixRouting,

    /// A remix command requested more output channels than Auralis can represent.
    #[error("remix output channel count exceeds representable audio buffer length")]
    RemixOutputChannelsOverflow,

    /// A remix command referenced an input channel that is not present.
    #[error("remix input channel is outside the input channel count")]
    RemixInputChannelOutOfBounds,

    /// A centercut command received anything other than stereo input.
    #[error("centercut can only process stereo input")]
    CentercutRequiresStereo,

    /// A centercut output gain was not finite.
    #[error("centercut output gain must be finite")]
    InvalidCentercutGain,

    /// A centercut window size was outside SoX-ng's supported range.
    #[error("centercut window size must be a power of two from 8 to 32768")]
    InvalidCentercutWindowSize,

    /// A DC shift amount was not finite or not in the supported normalized range.
    #[error("dc shift must be finite and in the range -2.0..=2.0")]
    InvalidDcShift,

    /// A `gain -r` command did not have prior reclaimable headroom metadata.
    #[error("gain -r requires prior gain -h headroom below full scale")]
    MissingGainHeadroom,

    /// A gain scan encountered a non-finite sample.
    #[error("gain scan encountered non-finite sample at flattened sample {sample_index}")]
    NonFiniteGainSample {
        /// Zero-based flattened sample index in planar channel order.
        sample_index: usize,
    },

    /// A norm scan encountered a non-finite sample.
    #[error("norm scan encountered non-finite sample at flattened sample {sample_index}")]
    NonFiniteNormSample {
        /// Zero-based flattened sample index in planar channel order.
        sample_index: usize,
    },

    /// A `vol` gain value was not finite.
    #[error("vol gain must be finite")]
    InvalidVolGain,

    /// A `vol` limiter gain was outside SoX-ng's supported range.
    #[error(
        "vol limiter gain must be finite, greater than 0, less than 1, and require absolute gain >= 1"
    )]
    InvalidVolLimiterGain,

    /// A `contrast` amount was not finite or outside SoX-ng's supported range.
    #[error("contrast amount must be finite and in the range 0..=100")]
    InvalidContrastAmount,

    /// A `softvol` value was not finite or outside SoX-ng's supported range.
    #[error("softvol volume, double-time, and headroom must be finite and non-negative")]
    InvalidSoftVol,

    /// A `tremolo` value was not finite or outside SoX-ng's supported range.
    #[error(
        "tremolo speed must be finite and non-negative, and depth must be finite and in the range 0<depth<=100"
    )]
    InvalidTremolo,

    /// An `overdrive` value was not finite or outside SoX-ng's supported range.
    #[error("overdrive gain and color must be finite and in the range 0..=100")]
    InvalidOverdrive,

    /// A `saturation` value was not finite or outside SoX-ng's supported range.
    #[error(
        "saturation blend and offset must be finite in 0..=1, tanh drive must be finite and >= 1, and sqrt color or diode threshold must be finite in 0..=1"
    )]
    InvalidSaturation,

    /// A SoX-ng positional fade requested overlapping fade-in and fade-out regions.
    #[error("fade-out overlaps fade-in")]
    FadeRegionsOverlap,

    /// A positional fade would create a buffer shape that cannot be represented.
    #[error("fade stop position exceeds representable audio buffer length")]
    FadeLengthOverflow,

    /// A buffer with an invalid shape was produced while applying an effect.
    #[error(transparent)]
    Core(#[from] auralis_core::AuralisError),

    /// A backend-dispatched channel downmix kernel rejected channel slices.
    #[error(transparent)]
    ChannelMix(#[from] auralis_simd::MixError),
}
