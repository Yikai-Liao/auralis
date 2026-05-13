use std::{
    fs::File,
    io::{BufReader, Cursor, Read},
    path::Path,
};

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_simd::BackendKind;

use crate::{Result, WavError, sample_conversion::float64_to_f32};

const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;

#[derive(Debug, Clone, Copy)]
struct Float64Header {
    channels: u16,
    sample_rate: u32,
    data_offset: usize,
    data_len: usize,
}

/// Returns true when `bytes` start with a little-endian RIFF/WAVE float64
/// stream that this adapter can decode.
pub(crate) fn is_float64_wav_bytes(bytes: &[u8]) -> bool {
    parse_float64_header(bytes).is_ok()
}

/// Decodes an entire float64 WAV stream into a planar `f32` buffer.
///
/// Samples are validated as finite 64-bit IEEE floats and then narrowed into
/// Auralis' internal `f32` processing buffer without clipping.
///
/// # Errors
///
/// Returns [`WavError::UnsupportedSampleFormat`] when the stream is not
/// 64-bit IEEE float. Returns [`WavError::Malformed`] when the RIFF/WAVE
/// container or sample payload cannot be parsed.
pub fn decode_float64<R>(mut reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| WavError::Malformed {
            message: error.to_string(),
        })?;

    decode_float64_bytes(&bytes)
}

/// Decodes an entire float64 WAV stream with an explicit backend request.
///
/// Float64 decoding is deterministic scalar logic, so `requested_backend` does
/// not currently change the produced samples.
///
/// # Errors
///
/// Returns the same parsing, format, and shape errors as [`decode_float64`].
pub fn decode_float64_with_backend<R>(
    reader: R,
    requested_backend: BackendKind,
) -> Result<AudioBuffer>
where
    R: Read,
{
    let _ = requested_backend;
    decode_float64(reader)
}

/// Decodes a float64 WAV file from disk into a planar `f32` buffer.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_float64`].
pub fn decode_float64_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    decode_float64_path_with_backend(path, BackendKind::Scalar)
}

/// Decodes a float64 WAV file from disk with an explicit backend request.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_float64_with_backend`].
pub fn decode_float64_path_with_backend(
    path: impl AsRef<Path>,
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| WavError::OpenFailed {
        message: error.to_string(),
    })?;

    decode_float64_with_backend(BufReader::new(file), requested_backend)
}

pub(crate) fn decode_float64_bytes(bytes: &[u8]) -> Result<AudioBuffer> {
    let header = parse_float64_header(bytes)?;
    let channels = ChannelCount::new(header.channels).map_err(|_| WavError::InvalidChannelCount)?;
    let sample_rate =
        SampleRate::new(header.sample_rate).map_err(|_| WavError::InvalidSampleRate)?;
    let sample_count = header
        .data_len
        .checked_div(8)
        .ok_or(WavError::InvalidBufferShape)?;
    if sample_count % channels.as_usize() != 0 {
        return Err(WavError::InvalidBufferShape);
    }

    let frames = sample_count / channels.as_usize();
    let mut interleaved_float64 = Vec::with_capacity(sample_count);
    let payload = bytes
        .get(header.data_offset..header.data_offset + header.data_len)
        .ok_or_else(|| WavError::Malformed {
            message: "data chunk extends past end of file".to_owned(),
        })?;
    for chunk in payload.chunks_exact(8) {
        interleaved_float64.push(f64::from_le_bytes(
            chunk.try_into().map_err(|_| WavError::InvalidBufferShape)?,
        ));
    }

    let mut interleaved_f32 = vec![0.0; sample_count];
    float64_to_f32(
        &interleaved_float64,
        &mut interleaved_f32,
        channels.as_usize(),
    )?;

    let mut planar = vec![0.0; sample_count];
    for (sample_index, sample) in interleaved_f32.into_iter().enumerate() {
        let frame_index = sample_index / channels.as_usize();
        let channel_index = sample_index % channels.as_usize();
        let planar_index = channel_index
            .checked_mul(frames)
            .and_then(|start| start.checked_add(frame_index))
            .ok_or(WavError::InvalidBufferShape)?;
        planar[planar_index] = sample;
    }

    let spec = AudioSpec::new(sample_rate, channels, SampleFormat::Float32);
    AudioBuffer::from_planar_f32(spec, FrameCount::new(frames as u64), planar)
        .map_err(|_| WavError::InvalidBufferShape)
}

fn parse_float64_header(bytes: &[u8]) -> Result<Float64Header> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(WavError::Malformed {
            message: "missing RIFF/WAVE header".to_owned(),
        });
    }

    let mut cursor: usize = 12;
    let mut format: Option<(u16, u16, u32, u16)> = None;
    while cursor.checked_add(8).is_some_and(|end| end <= bytes.len()) {
        let chunk_id = &bytes[cursor..cursor + 4];
        let chunk_len = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap());
        let chunk_len = usize::try_from(chunk_len).map_err(|_| WavError::InvalidBufferShape)?;
        let payload_start = cursor + 8;
        let payload_end = payload_start
            .checked_add(chunk_len)
            .ok_or(WavError::InvalidBufferShape)?;
        if payload_end > bytes.len() {
            return Err(WavError::Malformed {
                message: "chunk extends past end of file".to_owned(),
            });
        }

        match chunk_id {
            b"fmt " => {
                if chunk_len < 16 {
                    return Err(WavError::Malformed {
                        message: "fmt chunk is too short".to_owned(),
                    });
                }
                let payload = &bytes[payload_start..payload_end];
                let format_tag = u16::from_le_bytes(payload[0..2].try_into().unwrap());
                let channels = u16::from_le_bytes(payload[2..4].try_into().unwrap());
                let sample_rate = u32::from_le_bytes(payload[4..8].try_into().unwrap());
                let block_align = u16::from_le_bytes(payload[12..14].try_into().unwrap());
                let bits_per_sample = u16::from_le_bytes(payload[14..16].try_into().unwrap());
                format = Some((format_tag, channels, sample_rate, block_align));
                if format_tag != WAVE_FORMAT_IEEE_FLOAT || bits_per_sample != 64 {
                    return Err(WavError::UnsupportedSampleFormat {
                        bits_per_sample,
                        encoding: crate::WavSampleEncoding::Float,
                    });
                }
            }
            b"data" => {
                let Some((format_tag, channels, sample_rate, block_align)) = format else {
                    return Err(WavError::Malformed {
                        message: "data chunk appeared before fmt chunk".to_owned(),
                    });
                };
                if format_tag != WAVE_FORMAT_IEEE_FLOAT {
                    return Err(WavError::UnsupportedSampleFormat {
                        bits_per_sample: 64,
                        encoding: crate::WavSampleEncoding::Float,
                    });
                }
                if channels == 0 || block_align != channels.saturating_mul(8) || chunk_len % 8 != 0
                {
                    return Err(WavError::InvalidBufferShape);
                }
                return Ok(Float64Header {
                    channels,
                    sample_rate,
                    data_offset: payload_start,
                    data_len: chunk_len,
                });
            }
            _ => {}
        }

        cursor = payload_end + usize::from(chunk_len % 2 != 0);
    }

    Err(WavError::Malformed {
        message: "missing data chunk".to_owned(),
    })
}

pub(crate) fn decode_wav_bytes_or_hound(
    bytes: &[u8],
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    if is_float64_wav_bytes(bytes) {
        decode_float64_bytes(bytes)
    } else {
        crate::reader::AnyPcmWavReader::new(Cursor::new(bytes))?
            .read_wav_with_backend(requested_backend)
    }
}
