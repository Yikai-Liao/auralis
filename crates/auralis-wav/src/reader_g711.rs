use std::{
    fs::File,
    io::{BufReader, Cursor, Read},
    path::Path,
};

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_simd::BackendKind;

use crate::{
    Result, WavError,
    g711::G711Kind,
    reader_float64::{decode_float64_bytes, is_float64_wav_bytes},
};

#[derive(Debug, Clone, Copy)]
struct G711Header {
    channels: u16,
    sample_rate: u32,
    data_offset: usize,
    data_len: usize,
    kind: G711Kind,
}

pub(crate) fn is_g711_wav_bytes(bytes: &[u8]) -> bool {
    parse_g711_header(bytes).is_ok()
}

/// Decodes an entire u-law WAV stream into a planar `f32` buffer.
///
/// # Errors
///
/// Returns [`WavError::UnsupportedSampleFormat`] when the stream is not
/// 8-bit G.711 u-law. Returns [`WavError::Malformed`] when the RIFF/WAVE
/// container or payload cannot be parsed.
pub fn decode_ulaw<R>(reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    decode_g711(reader, G711Kind::ULaw)
}

/// Decodes an entire u-law WAV stream with an explicit backend request.
///
/// u-law expansion is deterministic scalar logic, so `requested_backend` does
/// not currently change the produced samples.
///
/// # Errors
///
/// Returns the same parsing, format, and shape errors as [`decode_ulaw`].
pub fn decode_ulaw_with_backend<R>(reader: R, requested_backend: BackendKind) -> Result<AudioBuffer>
where
    R: Read,
{
    let _ = requested_backend;
    decode_ulaw(reader)
}

/// Decodes a u-law WAV file from disk into a planar `f32` buffer.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened.
pub fn decode_ulaw_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    decode_ulaw_path_with_backend(path, BackendKind::Scalar)
}

/// Decodes a u-law WAV file from disk with an explicit backend request.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened.
pub fn decode_ulaw_path_with_backend(
    path: impl AsRef<Path>,
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| WavError::OpenFailed {
        message: error.to_string(),
    })?;
    decode_ulaw_with_backend(BufReader::new(file), requested_backend)
}

/// Decodes an entire A-law WAV stream into a planar `f32` buffer.
///
/// # Errors
///
/// Returns [`WavError::UnsupportedSampleFormat`] when the stream is not
/// 8-bit G.711 A-law. Returns [`WavError::Malformed`] when the RIFF/WAVE
/// container or payload cannot be parsed.
pub fn decode_alaw<R>(reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    decode_g711(reader, G711Kind::ALaw)
}

/// Decodes an entire A-law WAV stream with an explicit backend request.
///
/// A-law expansion is deterministic scalar logic, so `requested_backend` does
/// not currently change the produced samples.
///
/// # Errors
///
/// Returns the same parsing, format, and shape errors as [`decode_alaw`].
pub fn decode_alaw_with_backend<R>(reader: R, requested_backend: BackendKind) -> Result<AudioBuffer>
where
    R: Read,
{
    let _ = requested_backend;
    decode_alaw(reader)
}

/// Decodes an A-law WAV file from disk into a planar `f32` buffer.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened.
pub fn decode_alaw_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    decode_alaw_path_with_backend(path, BackendKind::Scalar)
}

/// Decodes an A-law WAV file from disk with an explicit backend request.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened.
pub fn decode_alaw_path_with_backend(
    path: impl AsRef<Path>,
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| WavError::OpenFailed {
        message: error.to_string(),
    })?;
    decode_alaw_with_backend(BufReader::new(file), requested_backend)
}

fn decode_g711<R>(mut reader: R, expected: G711Kind) -> Result<AudioBuffer>
where
    R: Read,
{
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| WavError::Malformed {
            message: error.to_string(),
        })?;
    decode_g711_bytes(&bytes, expected)
}

fn decode_g711_bytes(bytes: &[u8], expected: G711Kind) -> Result<AudioBuffer> {
    let header = parse_g711_header(bytes)?;
    if header.kind != expected {
        return Err(WavError::UnsupportedSampleFormat {
            bits_per_sample: 8,
            encoding: crate::WavSampleEncoding::Companded,
        });
    }
    decode_g711_payload(bytes, header)
}

fn decode_g711_payload(bytes: &[u8], header: G711Header) -> Result<AudioBuffer> {
    let channels = ChannelCount::new(header.channels).map_err(|_| WavError::InvalidChannelCount)?;
    let sample_rate =
        SampleRate::new(header.sample_rate).map_err(|_| WavError::InvalidSampleRate)?;
    let sample_count = header.data_len;
    if !sample_count.is_multiple_of(channels.as_usize()) {
        return Err(WavError::InvalidBufferShape);
    }
    let frames = sample_count / channels.as_usize();
    let payload = bytes
        .get(header.data_offset..header.data_offset + header.data_len)
        .ok_or_else(|| WavError::Malformed {
            message: "data chunk extends past end of file".to_owned(),
        })?;
    let mut planar = vec![0.0; sample_count];
    for (sample_index, sample) in payload.iter().copied().enumerate() {
        let frame_index = sample_index / channels.as_usize();
        let channel_index = sample_index % channels.as_usize();
        let planar_index = channel_index
            .checked_mul(frames)
            .and_then(|start| start.checked_add(frame_index))
            .ok_or(WavError::InvalidBufferShape)?;
        planar[planar_index] = header.kind.decode_byte(sample);
    }

    let spec = AudioSpec::new(sample_rate, channels, SampleFormat::Float32);
    AudioBuffer::from_planar_f32(spec, FrameCount::new(frames as u64), planar)
        .map_err(|_| WavError::InvalidBufferShape)
}

fn parse_g711_header(bytes: &[u8]) -> Result<G711Header> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(WavError::Malformed {
            message: "missing RIFF/WAVE header".to_owned(),
        });
    }

    let mut cursor: usize = 12;
    let mut format: Option<(u16, u16, u32, u16, u16)> = None;
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
                format = Some((
                    format_tag,
                    channels,
                    sample_rate,
                    block_align,
                    bits_per_sample,
                ));
                if format_tag != G711Kind::ULaw.format_tag()
                    && format_tag != G711Kind::ALaw.format_tag()
                {
                    return Err(WavError::UnsupportedSampleFormat {
                        bits_per_sample,
                        encoding: crate::WavSampleEncoding::Companded,
                    });
                }
                if bits_per_sample != 8 {
                    return Err(WavError::UnsupportedSampleFormat {
                        bits_per_sample,
                        encoding: crate::WavSampleEncoding::Companded,
                    });
                }
            }
            b"data" => {
                let Some((format_tag, channels, sample_rate, block_align, bits_per_sample)) =
                    format
                else {
                    return Err(WavError::Malformed {
                        message: "data chunk appeared before fmt chunk".to_owned(),
                    });
                };
                let kind = if format_tag == G711Kind::ULaw.format_tag() {
                    G711Kind::ULaw
                } else {
                    G711Kind::ALaw
                };
                if channels == 0
                    || bits_per_sample != 8
                    || block_align != channels
                    || chunk_len % usize::from(channels) != 0
                {
                    return Err(WavError::InvalidBufferShape);
                }
                return Ok(G711Header {
                    channels,
                    sample_rate,
                    data_offset: payload_start,
                    data_len: chunk_len,
                    kind,
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
    } else if is_g711_wav_bytes(bytes) {
        let header = parse_g711_header(bytes)?;
        decode_g711_payload(bytes, header)
    } else {
        crate::reader::AnyPcmWavReader::new(Cursor::new(bytes))?
            .read_wav_with_backend(requested_backend)
    }
}
