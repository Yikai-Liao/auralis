//! FLAC codec support for Auralis.
//!
//! This crate adapts pure Rust FLAC backends behind Auralis-owned error and
//! buffer types. It decodes supported FLAC integer streams into Auralis'
//! planar `f32` processing model and exports deterministic 16-bit integer FLAC
//! streams from that model.

use std::{
    fs::File,
    io::{self, BufReader, Read, Write},
    path::Path,
};

use auralis_codec::{
    AudioEncoder, AudioOutput, CodecError, CodecKind, EncodeSummary, FlacEncodeOptions,
};
use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat as AuralisSampleFormat,
    SampleRate,
};
use claxon::FlacReader;
use flacenc::{bitsink::ByteSink, component::BitRepr, error::Verify, source::MemSource};
use thiserror::Error;

/// Crate-local result type using [`FlacError`].
pub type Result<T> = std::result::Result<T, FlacError>;

/// Errors produced while reading or writing FLAC streams.
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

    /// The input buffer is too short for the current fixed-block FLAC encoder.
    #[error("FLAC encode requires at least 16 frames, got {frames}")]
    FrameCountTooSmall {
        /// Number of frames in the input buffer.
        frames: u64,
    },

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

    /// The input buffer contained a non-finite sample value.
    #[error("FLAC sample at channel {channel_index}, frame {frame_index} must be finite")]
    NonFiniteSample {
        /// Zero-based channel index of the invalid sample.
        channel_index: usize,
        /// Zero-based frame index of the invalid sample.
        frame_index: usize,
    },

    /// The output path could not be created.
    #[error("could not create FLAC output: {message}")]
    CreateFailed {
        /// Human-readable I/O failure detail.
        message: String,
    },

    /// The FLAC output stream could not be encoded or written.
    #[error("could not write FLAC output: {message}")]
    WriteFailed {
        /// Human-readable writer failure detail.
        message: String,
    },
}

impl From<FlacError> for CodecError {
    fn from(error: FlacError) -> Self {
        let message = error.to_string();
        match error {
            FlacError::NonFiniteSample { .. }
            | FlacError::CreateFailed { .. }
            | FlacError::FrameCountTooSmall { .. }
            | FlacError::WriteFailed { .. } => Self::EncodeFailed {
                kind: CodecKind::Flac,
                message,
            },
            _ => Self::DecodeFailed {
                kind: CodecKind::Flac,
                message,
            },
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

/// Encodes `audio` as a deterministic 16-bit integer FLAC stream.
///
/// The current encoder boundary intentionally uses a conservative PCM16 FLAC
/// profile while richer FLAC encode options remain future work.
///
/// # Errors
///
/// Returns [`FlacError::NonFiniteSample`] when any input sample is NaN or
/// infinite, or [`FlacError::WriteFailed`] when encoding or output writing
/// fails.
pub fn encode_flac<W>(mut writer: W, audio: &AudioBuffer, _options: FlacEncodeOptions) -> Result<()>
where
    W: Write,
{
    validate_encode_frame_count(audio.frames())?;
    let samples = interleaved_pcm16_samples(audio)?;
    let channels = audio.channels().as_usize();
    let sample_rate = audio.spec().sample_rate().as_u32();
    let bits_per_sample = 16;
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|error| FlacError::WriteFailed {
            message: format!("{error:?}"),
        })?;
    let block_size = config.block_size;
    let source = MemSource::from_samples(&samples, channels, bits_per_sample, sample_rate as usize);
    let stream =
        flacenc::encode_with_fixed_block_size(&config, source, block_size).map_err(|error| {
            FlacError::WriteFailed {
                message: error.to_string(),
            }
        })?;
    let mut sink = ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|error| FlacError::WriteFailed {
            message: error.to_string(),
        })?;
    writer.write_all(sink.as_slice())?;

    Ok(())
}

/// Encodes `audio` as a FLAC file at `path`.
///
/// Existing files at `path` are overwritten.
///
/// # Errors
///
/// Returns the same validation and encode errors as [`encode_flac`], or
/// [`FlacError::CreateFailed`] when the output file cannot be created.
pub fn encode_flac_path(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    options: FlacEncodeOptions,
) -> Result<()> {
    let file = File::create(path).map_err(|error| FlacError::CreateFailed {
        message: error.to_string(),
    })?;
    encode_flac(file, audio, options)
}

/// Codec-boundary encoder for FLAC output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlacEncoder {
    options: FlacEncodeOptions,
}

impl FlacEncoder {
    /// Creates a FLAC encoder with `options`.
    #[must_use]
    pub const fn new(options: FlacEncodeOptions) -> Self {
        Self { options }
    }

    /// Returns the encoder options.
    #[must_use]
    pub const fn options(self) -> FlacEncodeOptions {
        self.options
    }
}

impl AudioEncoder for FlacEncoder {
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Flac
    }

    fn encode(
        &self,
        input: &AudioBuffer,
        output: &mut dyn AudioOutput,
    ) -> auralis_codec::Result<EncodeSummary> {
        encode_flac(output, input, self.options)?;
        Ok(EncodeSummary::new(
            CodecKind::Flac,
            input.spec(),
            input.frames(),
        ))
    }
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

impl From<io::Error> for FlacError {
    fn from(error: io::Error) -> Self {
        Self::WriteFailed {
            message: error.to_string(),
        }
    }
}

fn interleaved_pcm16_samples(audio: &AudioBuffer) -> Result<Vec<i32>> {
    let channels = audio.channels().as_usize();
    let frames = usize::try_from(audio.frames().as_u64()).map_err(|_| FlacError::WriteFailed {
        message: "frame count does not fit in memory on this platform".to_owned(),
    })?;
    let mut samples = Vec::with_capacity(frames.saturating_mul(channels));
    for frame_index in 0..frames {
        for channel_index in 0..channels {
            let sample =
                audio
                    .sample(channel_index, frame_index)
                    .ok_or_else(|| FlacError::WriteFailed {
                        message: "audio buffer shape changed during FLAC write".to_owned(),
                    })?;
            samples.push(quantize_pcm16(sample, channel_index, frame_index)?);
        }
    }
    Ok(samples)
}

fn validate_encode_frame_count(frames: FrameCount) -> Result<()> {
    if frames.as_u64() < 16 {
        return Err(FlacError::FrameCountTooSmall {
            frames: frames.as_u64(),
        });
    }
    Ok(())
}

fn quantize_pcm16(sample: f32, channel_index: usize, frame_index: usize) -> Result<i32> {
    if !sample.is_finite() {
        return Err(FlacError::NonFiniteSample {
            channel_index,
            frame_index,
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "Rounded and clamped PCM16 samples are guaranteed to fit i32."
    )]
    let quantized = (sample.clamp(-1.0, 1.0) * 32_768.0)
        .round()
        .clamp(-32_768.0, 32_767.0) as i32;
    Ok(quantized)
}
