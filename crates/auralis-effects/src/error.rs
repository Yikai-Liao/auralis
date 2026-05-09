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
}
