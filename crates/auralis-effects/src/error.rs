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
