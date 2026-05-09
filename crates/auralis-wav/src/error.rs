use auralis_codec::{CodecError, CodecKind};
use thiserror::Error;

use crate::WavSampleEncoding;

/// Crate-local result type using [`WavError`].
pub type Result<T> = std::result::Result<T, WavError>;

/// Errors produced while decoding or encoding WAV input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum WavError {
    /// The WAV sample format is not the PCM16 format implemented by this
    /// milestone.
    #[error("unsupported WAV sample format: {encoding} with {bits_per_sample} bits per sample")]
    UnsupportedSampleFormat {
        /// The integer bit depth declared by the WAV stream.
        bits_per_sample: u16,
        /// The sample encoding declared by the WAV stream.
        encoding: WavSampleEncoding,
    },

    /// The WAV header declared a zero sample rate.
    #[error("WAV sample rate must be greater than zero")]
    InvalidSampleRate,

    /// The WAV header declared a zero channel count.
    #[error("WAV channel count must be greater than zero")]
    InvalidChannelCount,

    /// The decoded stream shape cannot be represented by Auralis' buffer
    /// model.
    #[error("decoded WAV data does not match a valid audio buffer shape")]
    InvalidBufferShape,

    /// The input could not be parsed as a well-formed WAV stream.
    #[error("malformed WAV input: {message}")]
    Malformed {
        /// Human-readable parser failure detail.
        message: String,
    },

    /// The input path could not be opened.
    #[error("could not open WAV input: {message}")]
    OpenFailed {
        /// Human-readable I/O failure detail.
        message: String,
    },

    /// The output path could not be created.
    #[error("could not create WAV output: {message}")]
    CreateFailed {
        /// Human-readable I/O failure detail.
        message: String,
    },

    /// The output stream already consumed its writer.
    #[error("WAV writer has already written an output stream")]
    WriterAlreadyUsed,

    /// The input buffer contained a non-finite sample value.
    #[error("WAV sample at channel {channel_index}, frame {frame_index} must be finite")]
    NonFiniteSample {
        /// Zero-based channel index of the invalid sample.
        channel_index: usize,
        /// Zero-based frame index of the invalid sample.
        frame_index: usize,
    },

    /// The WAV output stream could not be written or finalized.
    #[error("could not write WAV output: {message}")]
    WriteFailed {
        /// Human-readable writer failure detail.
        message: String,
    },
}

impl From<WavError> for CodecError {
    fn from(error: WavError) -> Self {
        Self::DecodeFailed {
            kind: CodecKind::Wav,
            message: error.to_string(),
        }
    }
}
