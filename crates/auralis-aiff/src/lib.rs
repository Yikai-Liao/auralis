//! AIFF PCM codec support for Auralis.
//!
//! This crate adapts the pure Rust `aifc` backend behind Auralis-owned codec
//! option and error types. The current implementation supports plain AIFF
//! signed integer PCM plus selected AIFC little-endian, float, and G.711
//! encodings.

use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read, Seek, Write},
    path::Path,
};

use aifc::{AifcReadInfo, AifcReader, AifcWriteInfo, AifcWriter, FileFormat};
use auralis_codec::{
    AiffContainer, AiffEncodeOptions, AiffSampleFormat, AudioEncoder, AudioOutput, CodecError,
    CodecKind, EncodeSummary,
};
use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat as AuralisSampleFormat,
    SampleRate,
};
use thiserror::Error;

/// Crate-local result type using [`AiffError`].
pub type Result<T> = std::result::Result<T, AiffError>;

/// Errors produced while reading or writing AIFF PCM streams.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AiffError {
    /// The input or output request used an unsupported AIFF sample format.
    #[error("unsupported AIFF sample format: {sample_format:?}")]
    UnsupportedSampleFormat {
        /// Backend sample format that is not covered by this AIFF PCM leaf.
        sample_format: aifc::SampleFormat,
    },

    /// The input used an unsupported AIFF-family container.
    #[error("unsupported AIFF container: {file_format:?}")]
    UnsupportedContainer {
        /// Backend file format that is not covered by this AIFF-family adapter.
        file_format: FileFormat,
    },

    /// The AIFF stream declared an invalid sample rate.
    #[error("AIFF sample rate must be finite, positive, and whole-Hz")]
    InvalidSampleRate,

    /// The AIFF stream declared an invalid channel count.
    #[error("AIFF channel count must be positive and fit Auralis' channel model")]
    InvalidChannelCount,

    /// The decoded stream shape cannot be represented by Auralis.
    #[error("decoded AIFF data does not match a valid audio buffer shape")]
    InvalidBufferShape,

    /// The input buffer contained a non-finite sample value.
    #[error("AIFF sample at channel {channel_index}, frame {frame_index} must be finite")]
    NonFiniteSample {
        /// Zero-based channel index of the invalid sample.
        channel_index: usize,
        /// Zero-based frame index of the invalid sample.
        frame_index: usize,
    },

    /// The stream could not be parsed as well-formed AIFF.
    #[error("malformed AIFF input: {message}")]
    Malformed {
        /// Human-readable parser failure detail.
        message: String,
    },

    /// The input path could not be opened.
    #[error("could not open AIFF input: {message}")]
    OpenFailed {
        /// Human-readable I/O failure detail.
        message: String,
    },

    /// The output path could not be created.
    #[error("could not create AIFF output: {message}")]
    CreateFailed {
        /// Human-readable I/O failure detail.
        message: String,
    },

    /// The AIFF output stream could not be written or finalized.
    #[error("could not write AIFF output: {message}")]
    WriteFailed {
        /// Human-readable writer failure detail.
        message: String,
    },
}

impl From<AiffError> for CodecError {
    fn from(error: AiffError) -> Self {
        Self::DecodeFailed {
            kind: CodecKind::Aiff,
            message: error.to_string(),
        }
    }
}

/// Decodes an entire supported AIFF or AIFC stream.
///
/// The returned [`AudioBuffer`] uses planar `f32` samples, matching Auralis'
/// internal processing format.
///
/// # Errors
///
/// Returns [`AiffError::UnsupportedSampleFormat`] for formats outside the
/// supported signed-integer, floating-point, and G.711 set.
pub fn decode_aiff<R>(reader: R) -> Result<AudioBuffer>
where
    R: Read + Seek,
{
    let mut reader = AifcReader::new(reader).map_err(|error| AiffError::Malformed {
        message: format!("{error:?}"),
    })?;
    let info = reader.info();
    validate_pcm_aiff_info(&info)?;

    let channels = ChannelCount::new(
        u16::try_from(info.channels).map_err(|_| AiffError::InvalidChannelCount)?,
    )
    .map_err(|_| AiffError::InvalidChannelCount)?;
    let sample_rate = sample_rate_from_f64(info.sample_rate)?;
    let sample_len = info.sample_len.ok_or(AiffError::InvalidBufferShape)?;
    let frames = sample_len
        .checked_div(u64::from(channels.as_u16()))
        .ok_or(AiffError::InvalidBufferShape)?;
    if frames
        .checked_mul(u64::from(channels.as_u16()))
        .is_none_or(|samples| samples != sample_len)
    {
        return Err(AiffError::InvalidBufferShape);
    }

    let frames_usize = usize::try_from(frames).map_err(|_| AiffError::InvalidBufferShape)?;
    let channels_usize = channels.as_usize();
    let mut planar = vec![0.0; frames_usize * channels_usize];
    for sample_index in 0..usize::try_from(sample_len).map_err(|_| AiffError::InvalidBufferShape)? {
        let frame_index = sample_index / channels_usize;
        let channel_index = sample_index % channels_usize;
        let sample = reader
            .read_sample()
            .map_err(|error| AiffError::Malformed {
                message: format!("{error:?}"),
            })?
            .ok_or(AiffError::InvalidBufferShape)?;
        planar[channel_index * frames_usize + frame_index] = sample_to_f32(sample)?;
    }

    let spec = AudioSpec::new(sample_rate, channels, AuralisSampleFormat::Float32);
    AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), planar)
        .map_err(|_| AiffError::InvalidBufferShape)
}

/// Decodes a supported AIFF or AIFC file.
///
/// # Errors
///
/// Returns the same parsing and format errors as [`decode_aiff`].
pub fn decode_aiff_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| AiffError::OpenFailed {
        message: error.to_string(),
    })?;
    decode_aiff(BufReader::new(file))
}

/// Encodes `audio` as a supported AIFF-family stream.
///
/// # Errors
///
/// Returns [`AiffError::NonFiniteSample`] for NaN or infinite input samples, or
/// [`AiffError::WriteFailed`] when the backend rejects the stream.
pub fn encode_aiff<W>(writer: W, audio: &AudioBuffer, options: AiffEncodeOptions) -> Result<()>
where
    W: Write + Seek,
{
    let Some(sample_format) = backend_sample_format(options.sample_format()) else {
        return Err(AiffError::UnsupportedSampleFormat {
            sample_format: aifc::SampleFormat::Custom(*b"????"),
        });
    };
    let info = AifcWriteInfo {
        file_format: backend_container(options.container()),
        channels: i16::try_from(audio.channels().as_u16())
            .map_err(|_| AiffError::InvalidChannelCount)?,
        sample_rate: f64::from(audio.spec().sample_rate().as_u32()),
        sample_format,
    };
    let mut writer = AifcWriter::new(writer, &info).map_err(|error| AiffError::WriteFailed {
        message: format!("{error:?}"),
    })?;
    match options.sample_format() {
        AiffSampleFormat::Signed8 => writer.write_samples_i8(&interleaved_i8(audio)?),
        AiffSampleFormat::Signed16
        | AiffSampleFormat::Signed16LittleEndian
        | AiffSampleFormat::ULaw
        | AiffSampleFormat::ALaw => writer.write_samples_i16(&interleaved_i16(audio)?),
        AiffSampleFormat::Signed24 => writer.write_samples_i24(&interleaved_i24(audio)?),
        AiffSampleFormat::Signed32 | AiffSampleFormat::Signed32LittleEndian => {
            writer.write_samples_i32(&interleaved_i32(audio)?)
        }
        AiffSampleFormat::Float32 => writer.write_samples_f32(&interleaved_f32(audio)?),
        AiffSampleFormat::Float64 => writer.write_samples_f64(&interleaved_f64(audio)?),
        _ => Err(aifc::AifcError::InvalidSampleFormat),
    }
    .map_err(|error| AiffError::WriteFailed {
        message: format!("{error:?}"),
    })?;
    writer.finalize().map_err(|error| AiffError::WriteFailed {
        message: format!("{error:?}"),
    })
}

/// Encodes `audio` as a supported AIFF-family file.
///
/// Existing files at `path` are overwritten.
///
/// # Errors
///
/// Returns the same sample validation and write errors as [`encode_aiff`].
pub fn encode_aiff_path(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    options: AiffEncodeOptions,
) -> Result<()> {
    let file = File::create(path).map_err(|error| AiffError::CreateFailed {
        message: error.to_string(),
    })?;
    encode_aiff(BufWriter::new(file), audio, options)
}

/// Codec-boundary encoder for supported AIFF-family output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiffPcmEncoder {
    options: AiffEncodeOptions,
}

impl AiffPcmEncoder {
    /// Creates an AIFF PCM encoder with `options`.
    #[must_use]
    pub const fn new(options: AiffEncodeOptions) -> Self {
        Self { options }
    }

    /// Returns the encoder options.
    #[must_use]
    pub const fn options(self) -> AiffEncodeOptions {
        self.options
    }
}

impl AudioEncoder for AiffPcmEncoder {
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Aiff
    }

    fn encode(
        &self,
        input: &AudioBuffer,
        output: &mut dyn AudioOutput,
    ) -> auralis_codec::Result<EncodeSummary> {
        encode_aiff(output, input, self.options).map_err(|error| CodecError::EncodeFailed {
            kind: CodecKind::Aiff,
            message: error.to_string(),
        })?;
        Ok(EncodeSummary::new(
            CodecKind::Aiff,
            input.spec(),
            input.frames(),
        ))
    }
}

fn validate_pcm_aiff_info(info: &AifcReadInfo) -> Result<()> {
    if !matches!(info.file_format, FileFormat::Aiff | FileFormat::Aifc) {
        return Err(AiffError::UnsupportedContainer {
            file_format: info.file_format,
        });
    }
    match info.sample_format {
        aifc::SampleFormat::I8
        | aifc::SampleFormat::I16
        | aifc::SampleFormat::I16LE
        | aifc::SampleFormat::I24
        | aifc::SampleFormat::I32
        | aifc::SampleFormat::I32LE
        | aifc::SampleFormat::F32
        | aifc::SampleFormat::F64
        | aifc::SampleFormat::CompressedUlaw
        | aifc::SampleFormat::CompressedAlaw => Ok(()),
        sample_format => Err(AiffError::UnsupportedSampleFormat { sample_format }),
    }
}

fn sample_rate_from_f64(sample_rate: f64) -> Result<SampleRate> {
    if !sample_rate.is_finite()
        || sample_rate < 1.0
        || sample_rate.fract() != 0.0
        || sample_rate > f64::from(u32::MAX)
    {
        return Err(AiffError::InvalidSampleRate);
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "The value is finite, positive, whole-Hz, and checked against u32::MAX above."
    )]
    let sample_rate = sample_rate as u32;
    SampleRate::new(sample_rate).map_err(|_| AiffError::InvalidSampleRate)
}

fn sample_to_f32(sample: aifc::Sample) -> Result<f32> {
    match sample {
        aifc::Sample::I8(sample) => Ok(f32::from(sample) / 128.0),
        aifc::Sample::I16(sample) => Ok(f32::from(sample) / 32_768.0),
        aifc::Sample::I24(sample) => {
            #[allow(
                clippy::cast_precision_loss,
                reason = "24-bit integer PCM values fit exactly in f32 before normalization."
            )]
            {
                Ok(sample as f32 / 8_388_608.0)
            }
        }
        aifc::Sample::I32(sample) => {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "AIFF PCM32 decode intentionally narrows to Auralis' internal f32 processing format."
            )]
            {
                Ok((f64::from(sample) / 2_147_483_648.0) as f32)
            }
        }
        aifc::Sample::F32(sample) => Ok(sample),
        aifc::Sample::F64(sample) => {
            if !sample.is_finite() || sample < f64::from(f32::MIN) || sample > f64::from(f32::MAX) {
                return Err(AiffError::InvalidBufferShape);
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "AIFC float64 decode intentionally narrows to Auralis' internal f32 processing format after range validation."
            )]
            {
                Ok(sample as f32)
            }
        }
        aifc::Sample::U8(_) => Err(AiffError::InvalidBufferShape),
    }
}

fn backend_container(container: AiffContainer) -> FileFormat {
    if container == AiffContainer::Aiff {
        FileFormat::Aiff
    } else {
        FileFormat::Aifc
    }
}

fn backend_sample_format(sample_format: AiffSampleFormat) -> Option<aifc::SampleFormat> {
    match sample_format {
        AiffSampleFormat::Signed8 => Some(aifc::SampleFormat::I8),
        AiffSampleFormat::Signed16 => Some(aifc::SampleFormat::I16),
        AiffSampleFormat::Signed24 => Some(aifc::SampleFormat::I24),
        AiffSampleFormat::Signed32 => Some(aifc::SampleFormat::I32),
        AiffSampleFormat::Signed16LittleEndian => Some(aifc::SampleFormat::I16LE),
        AiffSampleFormat::Signed32LittleEndian => Some(aifc::SampleFormat::I32LE),
        AiffSampleFormat::Float32 => Some(aifc::SampleFormat::F32),
        AiffSampleFormat::Float64 => Some(aifc::SampleFormat::F64),
        AiffSampleFormat::ULaw => Some(aifc::SampleFormat::CompressedUlaw),
        AiffSampleFormat::ALaw => Some(aifc::SampleFormat::CompressedAlaw),
        _ => None,
    }
}

fn interleaved_i8(audio: &AudioBuffer) -> Result<Vec<i8>> {
    interleaved_quantized(audio, 8).map(|samples| {
        samples
            .into_iter()
            .map(|sample| i8::try_from(sample).expect("signed 8-bit quantization is bounded"))
            .collect()
    })
}

fn interleaved_i16(audio: &AudioBuffer) -> Result<Vec<i16>> {
    interleaved_quantized(audio, 16).map(|samples| {
        samples
            .into_iter()
            .map(|sample| i16::try_from(sample).expect("signed 16-bit quantization is bounded"))
            .collect()
    })
}

fn interleaved_i24(audio: &AudioBuffer) -> Result<Vec<i32>> {
    interleaved_quantized(audio, 24).map(|samples| {
        samples
            .into_iter()
            .map(|sample| i32::try_from(sample).expect("signed 24-bit quantization is bounded"))
            .collect()
    })
}

fn interleaved_i32(audio: &AudioBuffer) -> Result<Vec<i32>> {
    interleaved_quantized(audio, 32).map(|samples| {
        samples
            .into_iter()
            .map(|sample| i32::try_from(sample).expect("signed 32-bit quantization is bounded"))
            .collect()
    })
}

fn interleaved_f32(audio: &AudioBuffer) -> Result<Vec<f32>> {
    interleaved_finite(audio)
}

fn interleaved_f64(audio: &AudioBuffer) -> Result<Vec<f64>> {
    interleaved_finite(audio).map(|samples| samples.into_iter().map(f64::from).collect())
}

fn interleaved_finite(audio: &AudioBuffer) -> Result<Vec<f32>> {
    let channels = audio.channels().as_usize();
    let frames = usize::try_from(audio.frames().as_u64()).map_err(|_| AiffError::WriteFailed {
        message: "frame count does not fit in memory on this platform".to_owned(),
    })?;
    let mut samples = Vec::with_capacity(frames.saturating_mul(channels));

    for frame_index in 0..frames {
        for channel_index in 0..channels {
            let sample =
                audio
                    .sample(channel_index, frame_index)
                    .ok_or_else(|| AiffError::WriteFailed {
                        message: "audio buffer shape changed during AIFF write".to_owned(),
                    })?;
            if !sample.is_finite() {
                return Err(AiffError::NonFiniteSample {
                    channel_index,
                    frame_index,
                });
            }
            samples.push(sample);
        }
    }

    Ok(samples)
}

fn interleaved_quantized(audio: &AudioBuffer, bits: u32) -> Result<Vec<i64>> {
    let channels = audio.channels().as_usize();
    let frames = usize::try_from(audio.frames().as_u64()).map_err(|_| AiffError::WriteFailed {
        message: "frame count does not fit in memory on this platform".to_owned(),
    })?;
    let mut samples = Vec::with_capacity(frames.saturating_mul(channels));

    for frame_index in 0..frames {
        for channel_index in 0..channels {
            let sample =
                audio
                    .sample(channel_index, frame_index)
                    .ok_or_else(|| AiffError::WriteFailed {
                        message: "audio buffer shape changed during AIFF write".to_owned(),
                    })?;
            if !sample.is_finite() {
                return Err(AiffError::NonFiniteSample {
                    channel_index,
                    frame_index,
                });
            }
            samples.push(quantize_signed(sample, bits));
        }
    }

    Ok(samples)
}

fn quantize_signed(sample: f32, bits: u32) -> i64 {
    let (scale, min, max) = match bits {
        8 => (128.0, -128, 127),
        16 => (32_768.0, -32_768, 32_767),
        24 => (8_388_608.0, -8_388_608, 8_388_607),
        32 => (2_147_483_648.0, i64::from(i32::MIN), i64::from(i32::MAX)),
        _ => unreachable!("unsupported AIFF PCM signed bit depth"),
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

impl From<io::Error> for AiffError {
    fn from(error: io::Error) -> Self {
        Self::WriteFailed {
            message: error.to_string(),
        }
    }
}
