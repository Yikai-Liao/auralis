//! FLAC decode support for Auralis.
//!
//! This crate adapts the pure Rust `claxon` decoder behind Auralis-owned error
//! and buffer types. It decodes supported FLAC integer streams into Auralis'
//! planar `f32` processing model.

use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use auralis_codec::{CodecError, CodecKind};
use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat as AuralisSampleFormat,
    SampleRate,
};
use claxon::FlacReader;
use thiserror::Error;

/// Crate-local result type using [`FlacError`].
pub type Result<T> = std::result::Result<T, FlacError>;

/// Errors produced while reading FLAC streams.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum FlacError {
    /// The input FLAC stream declared an unsupported bits-per-sample value.
    #[error("unsupported FLAC bits per sample: {bits_per_sample}")]
    UnsupportedBitsPerSample {
        /// Bits per sample declared by the FLAC streaminfo block.
        bits_per_sample: u32,
    },

    /// The input FLAC stream declared an invalid sample rate.
    #[error("FLAC sample rate must be greater than zero")]
    InvalidSampleRate,

    /// The input FLAC stream declared an invalid channel count.
    #[error("FLAC channel count must be positive and fit Auralis' channel model")]
    InvalidChannelCount,

    /// The decoded stream shape cannot be represented by Auralis.
    #[error("decoded FLAC data does not match a valid audio buffer shape")]
    InvalidBufferShape,

    /// The stream could not be parsed as well-formed FLAC.
    #[error("malformed FLAC input: {message}")]
    Malformed {
        /// Human-readable parser failure detail.
        message: String,
    },

    /// The input path could not be opened.
    #[error("could not open FLAC input: {message}")]
    OpenFailed {
        /// Human-readable I/O failure detail.
        message: String,
    },
}

impl From<FlacError> for CodecError {
    fn from(error: FlacError) -> Self {
        Self::DecodeFailed {
            kind: CodecKind::Flac,
            message: error.to_string(),
        }
    }
}

/// Decodes an entire supported FLAC stream.
///
/// The returned [`AudioBuffer`] uses planar `f32` samples. Integer FLAC samples
/// are normalized by `2^(bits_per_sample - 1)`, matching Auralis' WAV PCM
/// decode convention.
///
/// # Errors
///
/// Returns [`FlacError::UnsupportedBitsPerSample`] for streams outside the
/// currently supported 4-bit through 32-bit integer range.
pub fn decode_flac<R>(reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    let mut reader = FlacReader::new(reader).map_err(map_claxon_error)?;
    let streaminfo = reader.streaminfo();
    validate_bits_per_sample(streaminfo.bits_per_sample)?;

    let sample_rate =
        SampleRate::new(streaminfo.sample_rate).map_err(|_| FlacError::InvalidSampleRate)?;
    let channels = ChannelCount::new(
        u16::try_from(streaminfo.channels).map_err(|_| FlacError::InvalidChannelCount)?,
    )
    .map_err(|_| FlacError::InvalidChannelCount)?;
    let channels_usize = channels.as_usize();

    let interleaved = reader
        .samples()
        .map(|sample| sample.map_err(map_claxon_error))
        .collect::<Result<Vec<_>>>()?;
    if interleaved.len() % channels_usize != 0 {
        return Err(FlacError::InvalidBufferShape);
    }

    let frames = interleaved.len() / channels_usize;
    let frame_count =
        FrameCount::new(u64::try_from(frames).map_err(|_| FlacError::InvalidBufferShape)?);
    if streaminfo
        .samples
        .is_some_and(|samples| usize::try_from(samples) != Ok(frames))
    {
        return Err(FlacError::InvalidBufferShape);
    }

    let mut planar = vec![0.0; interleaved.len()];
    let denominator = sample_denominator(streaminfo.bits_per_sample);
    for (sample_index, sample) in interleaved.into_iter().enumerate() {
        let frame_index = sample_index / channels_usize;
        let channel_index = sample_index % channels_usize;
        #[allow(
            clippy::cast_precision_loss,
            reason = "FLAC decode narrows integer samples into Auralis' f32 processing buffer."
        )]
        {
            planar[channel_index * frames + frame_index] = sample as f32 / denominator;
        }
    }

    let spec = AudioSpec::new(sample_rate, channels, AuralisSampleFormat::Float32);
    AudioBuffer::from_planar_f32(spec, frame_count, planar)
        .map_err(|_| FlacError::InvalidBufferShape)
}

/// Decodes a supported FLAC file.
///
/// # Errors
///
/// Returns the same parsing and format errors as [`decode_flac`].
pub fn decode_flac_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| FlacError::OpenFailed {
        message: error.to_string(),
    })?;
    decode_flac(BufReader::new(file))
}

fn validate_bits_per_sample(bits_per_sample: u32) -> Result<()> {
    if (4..=32).contains(&bits_per_sample) {
        Ok(())
    } else {
        Err(FlacError::UnsupportedBitsPerSample { bits_per_sample })
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "The denominator is a power of two within FLAC's supported 32-bit sample range."
)]
fn sample_denominator(bits_per_sample: u32) -> f32 {
    (1_u64 << (bits_per_sample - 1)) as f32
}

fn map_claxon_error(error: claxon::Error) -> FlacError {
    match error {
        claxon::Error::IoError(io_error) => FlacError::Malformed {
            message: io_error.to_string(),
        },
        other => FlacError::Malformed {
            message: other.to_string(),
        },
    }
}
