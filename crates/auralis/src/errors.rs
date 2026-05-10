use crate::{
    ChannelConversionError, InputCombineError, OutputDitherError, OutputLevelError,
    SampleRateConversionError,
};
use thiserror::Error;

/// Errors produced by the high-level Auralis facade.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A shared core value constructor rejected an input.
    #[error(transparent)]
    Core(#[from] auralis_core::AuralisError),

    /// WAV decoding or encoding failed.
    #[error(transparent)]
    Wav(#[from] auralis_wav::WavError),

    /// A typed effect processor rejected its configuration or input buffer.
    #[error(transparent)]
    Effect(#[from] auralis_effects::EffectError),

    /// A sequential effect chain failed while applying one command.
    #[error(transparent)]
    Chain(#[from] auralis_effects::EffectChainError),

    /// Input combination failed before effects were applied.
    #[error(transparent)]
    InputCombine(#[from] InputCombineError),

    /// Output channel conversion failed before encoding.
    #[error(transparent)]
    ChannelConversion(#[from] ChannelConversionError),

    /// Output sample-rate conversion failed before encoding.
    #[error(transparent)]
    SampleRateConversion(#[from] SampleRateConversionError),

    /// Output level adjustment failed before encoding.
    #[error(transparent)]
    OutputLevel(#[from] OutputLevelError),

    /// Output dither insertion failed before encoding.
    #[error(transparent)]
    OutputDither(#[from] OutputDitherError),

    /// A seconds-based trim range could not be represented as frames.
    #[error("trim seconds range cannot be represented as frame positions")]
    InvalidTrimSecondsRange,
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Core(left), Self::Core(right)) => left == right,
            (Self::Wav(left), Self::Wav(right)) => left == right,
            (Self::Effect(left), Self::Effect(right)) => left == right,
            (Self::Chain(left), Self::Chain(right)) => left == right,
            (Self::InputCombine(left), Self::InputCombine(right)) => left == right,
            (Self::ChannelConversion(left), Self::ChannelConversion(right)) => left == right,
            (Self::SampleRateConversion(left), Self::SampleRateConversion(right)) => left == right,
            (Self::OutputLevel(left), Self::OutputLevel(right)) => left == right,
            (Self::OutputDither(left), Self::OutputDither(right)) => left == right,
            (Self::InvalidTrimSecondsRange, Self::InvalidTrimSecondsRange) => true,
            _ => false,
        }
    }
}
