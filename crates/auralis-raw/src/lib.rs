//! Headerless raw PCM codec support for Auralis.
//!
//! Raw PCM carries no metadata, so callers must supply stream shape out of band
//! when decoding in a future feature. This crate currently implements
//! deterministic signed and unsigned integer PCM export from Auralis' internal
//! planar `f32` buffer into interleaved headerless bytes.

use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

use auralis_codec::{
    AudioEncoder, AudioOutput, CodecError, CodecKind, EncodeSummary, RawPcmEncodeOptions,
    RawPcmSampleFormat,
};
use auralis_core::AudioBuffer;
use thiserror::Error;

/// Crate-local result type using [`RawPcmError`].
pub type Result<T> = std::result::Result<T, RawPcmError>;

/// Errors produced while writing raw PCM bytes.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RawPcmError {
    /// The input contained a non-finite sample.
    #[error("raw PCM sample at channel {channel_index}, frame {frame_index} is not finite")]
    NonFiniteSample {
        /// Zero-based channel index.
        channel_index: usize,
        /// Zero-based frame index.
        frame_index: usize,
    },

    /// The byte stream could not be written.
    #[error("raw PCM write failed: {message}")]
    WriteFailed {
        /// Human-readable failure detail from the writer.
        message: String,
    },
}

impl From<io::Error> for RawPcmError {
    fn from(error: io::Error) -> Self {
        Self::WriteFailed {
            message: error.to_string(),
        }
    }
}

impl From<RawPcmError> for CodecError {
    fn from(error: RawPcmError) -> Self {
        Self::EncodeFailed {
            kind: CodecKind::RawPcm,
            message: error.to_string(),
        }
    }
}

/// Encodes `audio` as interleaved headerless raw PCM bytes.
///
/// Multi-byte integer samples are written little-endian until the raw endian
/// roadmap leaf adds explicit options.
///
/// # Errors
///
/// Returns [`RawPcmError::NonFiniteSample`] for NaN or infinite input samples,
/// or [`RawPcmError::WriteFailed`] when the destination rejects bytes.
pub fn encode_raw_pcm<W>(
    mut writer: W,
    audio: &AudioBuffer,
    options: RawPcmEncodeOptions,
) -> Result<()>
where
    W: Write,
{
    let channels = audio.channels().as_usize();
    let frames =
        usize::try_from(audio.frames().as_u64()).map_err(|_| RawPcmError::WriteFailed {
            message: "frame count does not fit in memory on this platform".to_owned(),
        })?;

    for frame_index in 0..frames {
        for channel_index in 0..channels {
            let sample = audio.sample(channel_index, frame_index).ok_or_else(|| {
                RawPcmError::WriteFailed {
                    message: "audio buffer shape changed during raw PCM write".to_owned(),
                }
            })?;
            write_sample(
                &mut writer,
                sample,
                options.sample_format(),
                channel_index,
                frame_index,
            )?;
        }
    }

    Ok(())
}

/// Encodes `audio` as a raw PCM file at `path`.
///
/// Existing files at `path` are overwritten.
///
/// # Errors
///
/// Returns the same sample validation and write errors as [`encode_raw_pcm`].
pub fn encode_raw_pcm_path(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    options: RawPcmEncodeOptions,
) -> Result<()> {
    let file = File::create(path)?;
    encode_raw_pcm(file, audio, options)
}

/// Codec-boundary encoder for headerless raw PCM output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawPcmEncoder {
    options: RawPcmEncodeOptions,
}

impl RawPcmEncoder {
    /// Creates a raw PCM encoder with `options`.
    #[must_use]
    pub const fn new(options: RawPcmEncodeOptions) -> Self {
        Self { options }
    }

    /// Returns the encoder options.
    #[must_use]
    pub const fn options(self) -> RawPcmEncodeOptions {
        self.options
    }
}

impl AudioEncoder for RawPcmEncoder {
    fn codec_kind(&self) -> CodecKind {
        CodecKind::RawPcm
    }

    fn encode(
        &self,
        input: &AudioBuffer,
        output: &mut dyn AudioOutput,
    ) -> auralis_codec::Result<EncodeSummary> {
        encode_raw_pcm(output, input, self.options)?;
        Ok(EncodeSummary::new(
            CodecKind::RawPcm,
            input.spec(),
            input.frames(),
        ))
    }
}

fn write_sample<W>(
    writer: &mut W,
    sample: f32,
    format: RawPcmSampleFormat,
    channel_index: usize,
    frame_index: usize,
) -> Result<()>
where
    W: Write + ?Sized,
{
    if !sample.is_finite() {
        return Err(RawPcmError::NonFiniteSample {
            channel_index,
            frame_index,
        });
    }

    match format {
        RawPcmSampleFormat::Signed8 => {
            let sample = i8::try_from(quantize_signed(sample, 8)).map_err(quantization_error)?;
            writer.write_all(&[sample.cast_unsigned()])?;
        }
        RawPcmSampleFormat::Unsigned8 => {
            let sample = u8::try_from(quantize_unsigned(sample, 8)).map_err(quantization_error)?;
            writer.write_all(&[sample])?;
        }
        RawPcmSampleFormat::Signed16 => {
            let sample = i16::try_from(quantize_signed(sample, 16)).map_err(quantization_error)?;
            writer.write_all(&sample.to_le_bytes())?;
        }
        RawPcmSampleFormat::Unsigned16 => {
            let sample =
                u16::try_from(quantize_unsigned(sample, 16)).map_err(quantization_error)?;
            writer.write_all(&sample.to_le_bytes())?;
        }
        RawPcmSampleFormat::Signed24 => write_i24_le(writer, quantize_signed(sample, 24))?,
        RawPcmSampleFormat::Unsigned24 => write_u24_le(writer, quantize_unsigned(sample, 24))?,
        RawPcmSampleFormat::Signed32 => {
            let sample = i32::try_from(quantize_signed(sample, 32)).map_err(quantization_error)?;
            writer.write_all(&sample.to_le_bytes())?;
        }
        RawPcmSampleFormat::Unsigned32 => {
            let sample =
                u32::try_from(quantize_unsigned(sample, 32)).map_err(quantization_error)?;
            writer.write_all(&sample.to_le_bytes())?;
        }
        _ => {
            return Err(RawPcmError::WriteFailed {
                message: "raw PCM sample format is not supported".to_owned(),
            });
        }
    }

    Ok(())
}

fn quantize_signed(sample: f32, bits: u32) -> i64 {
    let (scale, min, max) = match bits {
        8 => (128.0, -128, 127),
        16 => (32_768.0, -32_768, 32_767),
        24 => (8_388_608.0, -8_388_608, 8_388_607),
        32 => (2_147_483_648.0, i64::from(i32::MIN), i64::from(i32::MAX)),
        _ => unreachable!("unsupported raw PCM signed bit depth"),
    };
    let quantized = f64::from(sample.clamp(-1.0, 1.0)) * scale;

    #[allow(
        clippy::cast_possible_truncation,
        reason = "The value is rounded and clamped before converting to the requested PCM integer width."
    )]
    {
        (quantized.round() as i64).clamp(min, max)
    }
}

fn quantize_unsigned(sample: f32, bits: u32) -> u64 {
    let (max, max_f64) = match bits {
        8 => (u64::from(u8::MAX), 255.0),
        16 => (u64::from(u16::MAX), 65_535.0),
        24 => (16_777_215, 16_777_215.0),
        32 => (u64::from(u32::MAX), 4_294_967_295.0),
        _ => unreachable!("unsupported raw PCM unsigned bit depth"),
    };
    let scaled = ((f64::from(sample.clamp(-1.0, 1.0)) + 1.0) * 0.5) * max_f64;

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "The value is rounded and clamped before converting to the requested PCM integer width."
    )]
    {
        (scaled.round() as u64).min(max)
    }
}

fn write_i24_le<W>(writer: &mut W, sample: i64) -> Result<()>
where
    W: Write + ?Sized,
{
    let sample = i32::try_from(sample).map_err(quantization_error)?;
    let bytes = sample.to_le_bytes();
    writer.write_all(&bytes[..3])?;
    Ok(())
}

fn write_u24_le<W>(writer: &mut W, sample: u64) -> Result<()>
where
    W: Write + ?Sized,
{
    let sample = u32::try_from(sample).map_err(quantization_error)?;
    let bytes = sample.to_le_bytes();
    writer.write_all(&bytes[..3])?;
    Ok(())
}

fn quantization_error(error: impl std::fmt::Display) -> RawPcmError {
    RawPcmError::WriteFailed {
        message: format!("raw PCM sample quantization overflowed: {error}"),
    }
}
