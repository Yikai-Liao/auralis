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

    /// A buffer with an invalid shape was produced while applying an effect.
    #[error(transparent)]
    Core(#[from] auralis_core::AuralisError),
}
