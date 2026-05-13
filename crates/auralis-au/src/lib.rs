//! AU/SND codec support for Auralis.
//!
//! This crate implements a small Auralis-owned Sun/NeXT AU adapter. It supports
//! the classic `.snd` big-endian container with linear PCM, IEEE floating-point,
//! u-law, and A-law sample payloads.

use std::{
    fs::File,
    io::{self, BufReader, Read, Write},
    path::Path,
};

use auralis_codec::{
    AuEncodeOptions, AuSampleFormat, AudioEncoder, AudioOutput, CodecError, CodecKind,
    EncodeSummary,
};
use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat as AuralisSampleFormat,
    SampleRate,
};
use thiserror::Error;

const AU_MAGIC: u32 = 0x2e73_6e64;
const HEADER_LEN: u32 = 24;
const UNKNOWN_SIZE: u32 = 0xffff_ffff;

/// Crate-local result type using [`AuError`].
pub type Result<T> = std::result::Result<T, AuError>;

/// Errors produced while reading or writing AU/SND streams.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum AuError {
    /// The input does not start with the AU/SND magic number.
    #[error("malformed AU/SND input: missing .snd magic")]
    MissingMagic,

    /// The AU/SND header is structurally invalid.
    #[error("malformed AU/SND input: {message}")]
    Malformed {
        /// Human-readable parser failure detail.
        message: String,
    },

    /// The input declared an unsupported AU/SND encoding code.
    #[error("unsupported AU/SND encoding code: {encoding}")]
    UnsupportedEncoding {
        /// AU/SND encoding code from the file header.
        encoding: u32,
    },

    /// The input or output sample rate could not be represented by Auralis.
    #[error("AU/SND sample rate must be greater than zero")]
    InvalidSampleRate,

    /// The input or output channel count could not be represented by Auralis.
    #[error("AU/SND channel count must be positive and fit Auralis' channel model")]
    InvalidChannelCount,

    /// The input buffer shape is inconsistent with its metadata.
    #[error("AU/SND data does not match a valid audio buffer shape")]
    InvalidBufferShape,

    /// The input path could not be opened.
    #[error("could not open AU/SND input: {message}")]
    OpenFailed {
        /// Human-readable I/O failure detail.
        message: String,
    },

    /// The output buffer contained a non-finite sample value.
    #[error("AU/SND sample at channel {channel_index}, frame {frame_index} must be finite")]
    NonFiniteSample {
        /// Zero-based channel index of the invalid sample.
        channel_index: usize,
        /// Zero-based frame index of the invalid sample.
        frame_index: usize,
    },

    /// The output path could not be created.
    #[error("could not create AU/SND output: {message}")]
    CreateFailed {
        /// Human-readable I/O failure detail.
        message: String,
    },

    /// The AU/SND output stream could not be written.
    #[error("could not write AU/SND output: {message}")]
    WriteFailed {
        /// Human-readable writer failure detail.
        message: String,
    },
}

impl From<AuError> for CodecError {
    fn from(error: AuError) -> Self {
        let message = error.to_string();
        match error {
            AuError::NonFiniteSample { .. }
            | AuError::CreateFailed { .. }
            | AuError::WriteFailed { .. } => Self::EncodeFailed {
                kind: CodecKind::Au,
                message,
            },
            _ => Self::DecodeFailed {
                kind: CodecKind::Au,
                message,
            },
        }
    }
}

/// Decodes an entire supported AU/SND stream into planar `f32` samples.
///
/// The adapter accepts `.snd` streams using AU encoding codes 1 through 7 and
/// 27: u-law, signed 8/16/24/32-bit integer PCM, IEEE float32/float64, and
/// A-law. Container annotations are ignored for now.
///
/// # Errors
///
/// Returns [`AuError::UnsupportedEncoding`] for AU/SND encodings outside the
/// supported PCM, float, and G.711 set.
pub fn decode_au<R>(mut reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| AuError::Malformed {
            message: error.to_string(),
        })?;
    parse_au_bytes(&bytes)
}

/// Decodes a supported AU/SND file.
///
/// # Errors
///
/// Returns the same parsing and format errors as [`decode_au`].
pub fn decode_au_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| AuError::OpenFailed {
        message: error.to_string(),
    })?;
    decode_au(BufReader::new(file))
}

/// Encodes `audio` as a Sun/NeXT AU/SND stream.
///
/// The emitted stream uses a 24-byte header with no annotation bytes and a
/// known data size. Multi-byte samples are big-endian, matching the AU/SND
/// container convention.
///
/// # Errors
///
/// Returns [`AuError::NonFiniteSample`] when any input sample is NaN or
/// infinite, or [`AuError::WriteFailed`] when output writing fails.
pub fn encode_au<W>(mut writer: W, audio: &AudioBuffer, options: AuEncodeOptions) -> Result<()>
where
    W: Write,
{
    let payload = encode_payload(audio, options.sample_format())?;
    let data_size = u32::try_from(payload.len()).map_err(|_| AuError::WriteFailed {
        message: "AU/SND payload is too large for the 32-bit data-size header".to_owned(),
    })?;
    write_header(
        &mut writer,
        data_size,
        encoding_code(options.sample_format())?,
        audio.spec().sample_rate().as_u32(),
        u32::from(audio.channels().as_u16()),
    )?;
    writer.write_all(&payload)?;
    Ok(())
}

/// Encodes `audio` as an AU/SND file at `path`.
///
/// Existing files at `path` are overwritten.
///
/// # Errors
///
/// Returns the same validation and encode errors as [`encode_au`], or
/// [`AuError::CreateFailed`] when the output file cannot be created.
pub fn encode_au_path(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    options: AuEncodeOptions,
) -> Result<()> {
    let file = File::create(path).map_err(|error| AuError::CreateFailed {
        message: error.to_string(),
    })?;
    encode_au(file, audio, options)
}

/// Codec-boundary encoder for AU/SND output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AuEncoder {
    options: AuEncodeOptions,
}

impl AuEncoder {
    /// Creates an AU/SND encoder with `options`.
    #[must_use]
    pub const fn new(options: AuEncodeOptions) -> Self {
        Self { options }
    }

    /// Returns the encoder options.
    #[must_use]
    pub const fn options(self) -> AuEncodeOptions {
        self.options
    }
}

impl AudioEncoder for AuEncoder {
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Au
    }

    fn encode(
        &self,
        input: &AudioBuffer,
        output: &mut dyn AudioOutput,
    ) -> auralis_codec::Result<EncodeSummary> {
        encode_au(output, input, self.options)?;
        Ok(EncodeSummary::new(
            CodecKind::Au,
            input.spec(),
            input.frames(),
        ))
    }
}

fn parse_au_bytes(bytes: &[u8]) -> Result<AudioBuffer> {
    if bytes.len() < usize::try_from(HEADER_LEN).unwrap() {
        return Err(AuError::Malformed {
            message: "header is shorter than 24 bytes".to_owned(),
        });
    }
    if read_u32(bytes, 0)? != AU_MAGIC {
        return Err(AuError::MissingMagic);
    }

    let data_offset = read_u32(bytes, 4)?;
    let data_size = read_u32(bytes, 8)?;
    let encoding = read_u32(bytes, 12)?;
    let sample_rate = read_u32(bytes, 16)?;
    let channels = read_u32(bytes, 20)?;
    let offset = usize::try_from(data_offset).map_err(|_| AuError::Malformed {
        message: "data offset does not fit in memory".to_owned(),
    })?;
    if data_offset < HEADER_LEN || offset > bytes.len() {
        return Err(AuError::Malformed {
            message: "data offset is outside the file".to_owned(),
        });
    }

    let available = bytes.len() - offset;
    let data_len = if data_size == UNKNOWN_SIZE {
        available
    } else {
        let requested = usize::try_from(data_size).map_err(|_| AuError::Malformed {
            message: "data size does not fit in memory".to_owned(),
        })?;
        if requested > available {
            return Err(AuError::Malformed {
                message: "data size extends beyond the file".to_owned(),
            });
        }
        requested
    };

    let sample_format = sample_format_from_code(encoding)?;
    let bytes_per_sample = bytes_per_sample(sample_format)?;
    if data_len % bytes_per_sample != 0 {
        return Err(AuError::InvalidBufferShape);
    }
    let sample_rate = SampleRate::new(sample_rate).map_err(|_| AuError::InvalidSampleRate)?;
    let channels =
        ChannelCount::new(u16::try_from(channels).map_err(|_| AuError::InvalidChannelCount)?)
            .map_err(|_| AuError::InvalidChannelCount)?;
    let channels_usize = channels.as_usize();
    let sample_count = data_len / bytes_per_sample;
    if !sample_count.is_multiple_of(channels_usize) {
        return Err(AuError::InvalidBufferShape);
    }
    let frames = sample_count / channels_usize;
    let mut planar = vec![0.0; sample_count];
    let payload = &bytes[offset..offset + data_len];

    for sample_index in 0..sample_count {
        let frame_index = sample_index / channels_usize;
        let channel_index = sample_index % channels_usize;
        let planar_index = channel_index
            .checked_mul(frames)
            .and_then(|base| base.checked_add(frame_index))
            .ok_or(AuError::InvalidBufferShape)?;
        planar[planar_index] = decode_sample(sample_format, payload, sample_index)?;
    }

    let spec = AudioSpec::new(sample_rate, channels, AuralisSampleFormat::Float32);
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(frames).map_err(|_| AuError::InvalidBufferShape)?),
        planar,
    )
    .map_err(|_| AuError::InvalidBufferShape)
}

fn decode_sample(
    sample_format: AuSampleFormat,
    payload: &[u8],
    sample_index: usize,
) -> Result<f32> {
    let offset = sample_index * bytes_per_sample(sample_format)?;
    Ok(match sample_format {
        AuSampleFormat::ULaw => f32::from(ulaw_to_i16(payload[offset])) / 32768.0,
        AuSampleFormat::Signed8 => f32::from(i8::from_be_bytes([payload[offset]])) / 128.0,
        AuSampleFormat::Signed16 => {
            f32::from(i16::from_be_bytes([payload[offset], payload[offset + 1]])) / 32768.0
        }
        AuSampleFormat::Signed24 => {
            let sign = if payload[offset] & 0x80 != 0 {
                0xff
            } else {
                0x00
            };
            #[allow(
                clippy::cast_precision_loss,
                reason = "AU/SND integer decode intentionally narrows samples into Auralis' f32 processing buffer."
            )]
            {
                i32::from_be_bytes([
                    sign,
                    payload[offset],
                    payload[offset + 1],
                    payload[offset + 2],
                ]) as f32
                    / 8_388_608.0
            }
        }
        AuSampleFormat::Signed32 => {
            #[allow(
                clippy::cast_precision_loss,
                reason = "AU/SND integer decode intentionally narrows samples into Auralis' f32 processing buffer."
            )]
            {
                i32::from_be_bytes([
                    payload[offset],
                    payload[offset + 1],
                    payload[offset + 2],
                    payload[offset + 3],
                ]) as f32
                    / 2_147_483_648.0
            }
        }
        AuSampleFormat::Float32 => f32::from_be_bytes([
            payload[offset],
            payload[offset + 1],
            payload[offset + 2],
            payload[offset + 3],
        ]),
        AuSampleFormat::Float64 => {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "AU/SND float64 decode intentionally narrows samples into Auralis' f32 processing buffer."
            )]
            {
                f64::from_be_bytes([
                    payload[offset],
                    payload[offset + 1],
                    payload[offset + 2],
                    payload[offset + 3],
                    payload[offset + 4],
                    payload[offset + 5],
                    payload[offset + 6],
                    payload[offset + 7],
                ]) as f32
            }
        }
        AuSampleFormat::ALaw => f32::from(alaw_to_i16(payload[offset])) / 32768.0,
        _ => return Err(AuError::InvalidBufferShape),
    })
}

fn encode_payload(audio: &AudioBuffer, sample_format: AuSampleFormat) -> Result<Vec<u8>> {
    let channels = audio.channels().as_usize();
    let frames = usize::try_from(audio.frames().as_u64()).map_err(|_| AuError::WriteFailed {
        message: "frame count does not fit in memory on this platform".to_owned(),
    })?;
    let bytes_per_sample = bytes_per_sample(sample_format)?;
    let mut payload = Vec::with_capacity(frames * channels * bytes_per_sample);
    for frame_index in 0..frames {
        for channel_index in 0..channels {
            let sample =
                audio
                    .sample(channel_index, frame_index)
                    .ok_or_else(|| AuError::WriteFailed {
                        message: "audio buffer shape changed during AU/SND write".to_owned(),
                    })?;
            if !sample.is_finite() {
                return Err(AuError::NonFiniteSample {
                    channel_index,
                    frame_index,
                });
            }
            encode_sample(&mut payload, sample, sample_format)?;
        }
    }
    Ok(payload)
}

fn encode_sample(payload: &mut Vec<u8>, sample: f32, sample_format: AuSampleFormat) -> Result<()> {
    match sample_format {
        AuSampleFormat::ULaw => payload.push(i16_to_ulaw(quantize_for_g711(sample, 14))),
        AuSampleFormat::Signed8 => {
            let sample = i8::try_from(quantize_signed(sample, 8)).map_err(quantization_error)?;
            payload.push(sample.cast_unsigned());
        }
        AuSampleFormat::Signed16 => {
            let sample = i16::try_from(quantize_signed(sample, 16)).map_err(quantization_error)?;
            payload.extend_from_slice(&sample.to_be_bytes());
        }
        AuSampleFormat::Signed24 => {
            let sample = i32::try_from(quantize_signed(sample, 24)).map_err(quantization_error)?;
            let bytes = sample.to_be_bytes();
            payload.extend_from_slice(&bytes[1..]);
        }
        AuSampleFormat::Signed32 => {
            let sample = i32::try_from(quantize_signed(sample, 32)).map_err(quantization_error)?;
            payload.extend_from_slice(&sample.to_be_bytes());
        }
        AuSampleFormat::Float32 => payload.extend_from_slice(&sample.to_be_bytes()),
        AuSampleFormat::Float64 => payload.extend_from_slice(&f64::from(sample).to_be_bytes()),
        AuSampleFormat::ALaw => payload.push(i16_to_alaw(quantize_for_g711(sample, 13))),
        _ => {}
    }
    Ok(())
}

fn quantization_error(error: impl std::fmt::Display) -> AuError {
    AuError::WriteFailed {
        message: format!("AU/SND sample quantization failed: {error}"),
    }
}

fn write_header<W>(
    writer: &mut W,
    data_size: u32,
    encoding: u32,
    sample_rate: u32,
    channels: u32,
) -> Result<()>
where
    W: Write + ?Sized,
{
    for value in [
        AU_MAGIC,
        HEADER_LEN,
        data_size,
        encoding,
        sample_rate,
        channels,
    ] {
        writer.write_all(&value.to_be_bytes())?;
    }
    Ok(())
}

impl From<io::Error> for AuError {
    fn from(error: io::Error) -> Self {
        Self::WriteFailed {
            message: error.to_string(),
        }
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let field = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| AuError::Malformed {
            message: "header field is truncated".to_owned(),
        })?;
    Ok(u32::from_be_bytes([field[0], field[1], field[2], field[3]]))
}

fn sample_format_from_code(encoding: u32) -> Result<AuSampleFormat> {
    match encoding {
        1 => Ok(AuSampleFormat::ULaw),
        2 => Ok(AuSampleFormat::Signed8),
        3 => Ok(AuSampleFormat::Signed16),
        4 => Ok(AuSampleFormat::Signed24),
        5 => Ok(AuSampleFormat::Signed32),
        6 => Ok(AuSampleFormat::Float32),
        7 => Ok(AuSampleFormat::Float64),
        27 => Ok(AuSampleFormat::ALaw),
        _ => Err(AuError::UnsupportedEncoding { encoding }),
    }
}

fn encoding_code(sample_format: AuSampleFormat) -> Result<u32> {
    let code = match sample_format {
        AuSampleFormat::ULaw => 1,
        AuSampleFormat::Signed8 => 2,
        AuSampleFormat::Signed16 => 3,
        AuSampleFormat::Signed24 => 4,
        AuSampleFormat::Signed32 => 5,
        AuSampleFormat::Float32 => 6,
        AuSampleFormat::Float64 => 7,
        AuSampleFormat::ALaw => 27,
        _ => {
            return Err(AuError::WriteFailed {
                message: "AU/SND sample format is not supported".to_owned(),
            });
        }
    };
    Ok(code)
}

fn bytes_per_sample(sample_format: AuSampleFormat) -> Result<usize> {
    let bytes = match sample_format {
        AuSampleFormat::ULaw | AuSampleFormat::Signed8 | AuSampleFormat::ALaw => 1,
        AuSampleFormat::Signed16 => 2,
        AuSampleFormat::Signed24 => 3,
        AuSampleFormat::Signed32 | AuSampleFormat::Float32 => 4,
        AuSampleFormat::Float64 => 8,
        _ => {
            return Err(AuError::WriteFailed {
                message: "AU/SND sample format is not supported".to_owned(),
            });
        }
    };
    Ok(bytes)
}

fn quantize_signed(sample: f32, bits: u32) -> i64 {
    let (scale, min, max) = match bits {
        8 => (128.0, -128, 127),
        16 => (32_768.0, -32_768, 32_767),
        24 => (8_388_608.0, -8_388_608, 8_388_607),
        32 => (2_147_483_648.0, i64::from(i32::MIN), i64::from(i32::MAX)),
        _ => unreachable!("unsupported AU/SND signed bit depth"),
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

fn quantize_for_g711(sample: f32, bits: u16) -> i16 {
    let scale = f32::from(1_u16 << (bits - 1));
    let min = -i32::from(1_u16 << (bits - 1));
    let max = i32::from((1_u16 << (bits - 1)) - 1);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "The finite sample is clipped and rounded into the target G.711 input range immediately before the cast."
    )]
    let quantized = (sample.clamp(-1.0, 1.0) * scale).round() as i32;
    i16::try_from(quantized.clamp(min, max)).expect("G.711 quantized sample fits in i16")
}

fn alaw_to_i16(sample: u8) -> i16 {
    let sample = sample ^ 0x55;
    let mut value = i16::from(sample & 0x0f) << 4;
    let segment = (sample & 0x70) >> 4;
    match segment {
        0 => value += 8,
        1 => value += 0x108,
        _ => {
            value += 0x108;
            value <<= i16::from(segment - 1);
        }
    }
    if sample & 0x80 != 0 { value } else { -value }
}

fn ulaw_to_i16(sample: u8) -> i16 {
    let sample = !sample;
    let mut value = (i16::from(sample & 0x0f) << 3) + 0x84;
    value <<= i16::from((sample & 0x70) >> 4);
    if sample & 0x80 != 0 {
        0x84 - value
    } else {
        value - 0x84
    }
}

fn i16_to_alaw(mut sample: i16) -> u8 {
    let mask = if sample >= 0 {
        0xd5
    } else {
        sample = -sample - 1;
        0x55
    };
    let segment = search_segment(
        i32::from(sample),
        &[0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff],
    );
    if segment >= 8 {
        return 0x7f ^ mask;
    }
    let mut encoded = u8::try_from(segment << 4).expect("A-law segment fits in u8");
    if segment < 2 {
        encoded |= u8::try_from((sample >> 1) & 0x0f).expect("A-law quantization fits in u8");
    } else {
        encoded |= u8::try_from((sample >> segment) & 0x0f).expect("A-law quantization fits in u8");
    }
    encoded ^ mask
}

fn i16_to_ulaw(mut sample: i16) -> u8 {
    let mask = if sample < 0 {
        sample = -sample;
        0x7f
    } else {
        0xff
    };
    sample = sample.min(8159) + (0x84 >> 2);
    let segment = search_segment(
        i32::from(sample),
        &[0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff],
    );
    if segment >= 8 {
        return 0x7f ^ mask;
    }
    let segment_shift = u32::try_from(segment + 1).expect("u-law segment shift fits in u32");
    let quantized = (i32::from(sample) >> segment_shift) & 0x0f;
    let encoded = u8::try_from((segment << 4) | usize::try_from(quantized).unwrap())
        .expect("u-law code fits in u8");
    encoded ^ mask
}

fn search_segment(value: i32, ends: &[i32; 8]) -> usize {
    ends.iter()
        .position(|&end| value <= end)
        .unwrap_or(ends.len())
}
