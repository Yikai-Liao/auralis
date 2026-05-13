use std::{
    fs::File,
    io::{BufReader, Read, Seek, Write},
    path::Path,
};

use auralis_codec::{
    AudioEncoder, AudioOutput, AudioWriter, CodecError, CodecKind, EncodeSummary, WavEncodeOptions,
    WavSampleFormat,
};
use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_simd::BackendKind;

use crate::{
    Result, WavError,
    g711::G711Kind,
    sample_conversion::{
        f32_to_float32, f32_to_float64, f32_to_pcm8, f32_to_pcm16_with_backend, f32_to_pcm24,
        f32_to_pcm32, float32_to_f32, float64_to_f32, pcm8_to_f32, pcm16_to_f32_with_backend,
        pcm24_to_f32, pcm32_to_f32,
    },
};

const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;

#[derive(Debug, Clone, Copy)]
struct RifxHeader {
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    sample_format: WavSampleFormat,
    data_offset: usize,
    data_len: usize,
}

/// Returns true when `bytes` start with a big-endian RIFX/WAVE stream.
pub(crate) fn is_rifx_wav_bytes(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFX" && &bytes[8..12] == b"WAVE"
}

/// Decodes an entire RIFX/WAVE stream into a planar `f32` buffer.
///
/// RIFX is the big-endian sibling of RIFF/WAVE. Chunk lengths, `fmt ` fields,
/// and multi-byte sample payloads are interpreted as big-endian values.
///
/// # Errors
///
/// Returns [`WavError::UnsupportedSampleFormat`] when the sample format is not
/// one of the supported WAV formats. Returns [`WavError::Malformed`] when the
/// RIFX/WAVE container or payload cannot be parsed.
pub fn decode_rifx<R>(reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    decode_rifx_with_backend(reader, BackendKind::Scalar)
}

/// Decodes an entire RIFX/WAVE stream with an explicit sample-conversion
/// backend.
///
/// The backend only affects PCM16 sample conversion, matching the existing
/// RIFF/WAVE path.
///
/// # Errors
///
/// Returns the same parsing, format, and shape errors as [`decode_rifx`].
pub fn decode_rifx_with_backend<R>(
    mut reader: R,
    requested_backend: BackendKind,
) -> Result<AudioBuffer>
where
    R: Read,
{
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| WavError::Malformed {
            message: error.to_string(),
        })?;

    decode_rifx_bytes(&bytes, requested_backend)
}

/// Decodes a RIFX/WAVE file from disk.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_rifx`].
pub fn decode_rifx_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    decode_rifx_path_with_backend(path, BackendKind::Scalar)
}

/// Decodes a RIFX/WAVE file from disk with an explicit backend request.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_rifx_with_backend`].
pub fn decode_rifx_path_with_backend(
    path: impl AsRef<Path>,
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| WavError::OpenFailed {
        message: error.to_string(),
    })?;

    decode_rifx_with_backend(BufReader::new(file), requested_backend)
}

pub(crate) fn decode_rifx_bytes(
    bytes: &[u8],
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    let header = parse_rifx_header(bytes)?;
    let channels = ChannelCount::new(header.channels).map_err(|_| WavError::InvalidChannelCount)?;
    let sample_rate =
        SampleRate::new(header.sample_rate).map_err(|_| WavError::InvalidSampleRate)?;
    let bytes_per_sample = usize::from(header.bits_per_sample / 8);
    if bytes_per_sample == 0 || header.data_len % bytes_per_sample != 0 {
        return Err(WavError::InvalidBufferShape);
    }
    let sample_count = header.data_len / bytes_per_sample;
    if !sample_count.is_multiple_of(channels.as_usize()) {
        return Err(WavError::InvalidBufferShape);
    }
    let frames = sample_count / channels.as_usize();
    let payload = bytes
        .get(header.data_offset..header.data_offset + header.data_len)
        .ok_or_else(|| WavError::Malformed {
            message: "data chunk extends past end of file".to_owned(),
        })?;
    let interleaved_f32 = decode_payload(payload, header, sample_count, requested_backend)?;
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

fn decode_payload(
    payload: &[u8],
    header: RifxHeader,
    sample_count: usize,
    requested_backend: BackendKind,
) -> Result<Vec<f32>> {
    let channels = usize::from(header.channels);
    let mut interleaved_f32 = vec![0.0; sample_count];
    match header.sample_format {
        WavSampleFormat::Pcm8 => {
            let pcm8: Vec<i8> = payload
                .iter()
                .map(|byte| i16::from(*byte) - 128)
                .map(|value| i8::try_from(value).expect("RIFX PCM8 offset fits in i8"))
                .collect();
            pcm8_to_f32(&pcm8, &mut interleaved_f32)?;
        }
        WavSampleFormat::Pcm16 => {
            let pcm16: Vec<i16> = payload
                .chunks_exact(2)
                .map(|chunk| i16::from_be_bytes(chunk.try_into().unwrap()))
                .collect();
            pcm16_to_f32_with_backend(requested_backend, &pcm16, &mut interleaved_f32)?;
        }
        WavSampleFormat::Pcm24 => {
            let pcm24: Vec<i32> = payload
                .chunks_exact(3)
                .map(|chunk| {
                    i32::from_be_bytes([
                        if chunk[0] & 0x80 == 0 { 0 } else { 0xff },
                        chunk[0],
                        chunk[1],
                        chunk[2],
                    ])
                })
                .collect();
            pcm24_to_f32(&pcm24, &mut interleaved_f32)?;
        }
        WavSampleFormat::Pcm32 => {
            let pcm32: Vec<i32> = payload
                .chunks_exact(4)
                .map(|chunk| i32::from_be_bytes(chunk.try_into().unwrap()))
                .collect();
            pcm32_to_f32(&pcm32, &mut interleaved_f32)?;
        }
        WavSampleFormat::Float32 => {
            let float32: Vec<f32> = payload
                .chunks_exact(4)
                .map(|chunk| f32::from_be_bytes(chunk.try_into().unwrap()))
                .collect();
            float32_to_f32(&float32, &mut interleaved_f32, channels)?;
        }
        WavSampleFormat::Float64 => {
            let float64: Vec<f64> = payload
                .chunks_exact(8)
                .map(|chunk| f64::from_be_bytes(chunk.try_into().unwrap()))
                .collect();
            float64_to_f32(&float64, &mut interleaved_f32, channels)?;
        }
        WavSampleFormat::ULaw => {
            for (index, byte) in payload.iter().copied().enumerate() {
                interleaved_f32[index] = G711Kind::ULaw.decode_byte(byte);
            }
        }
        WavSampleFormat::ALaw => {
            for (index, byte) in payload.iter().copied().enumerate() {
                interleaved_f32[index] = G711Kind::ALaw.decode_byte(byte);
            }
        }
        _ => unreachable!("unsupported RIFX sample format already rejected"),
    }

    Ok(interleaved_f32)
}

fn parse_rifx_header(bytes: &[u8]) -> Result<RifxHeader> {
    if !is_rifx_wav_bytes(bytes) {
        return Err(WavError::Malformed {
            message: "missing RIFX/WAVE header".to_owned(),
        });
    }

    let mut cursor: usize = 12;
    let mut format: Option<(u16, u16, u32, u16, u16)> = None;
    while cursor.checked_add(8).is_some_and(|end| end <= bytes.len()) {
        let chunk_id = &bytes[cursor..cursor + 4];
        let chunk_len = u32::from_be_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap());
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
                let format_tag = u16::from_be_bytes(payload[0..2].try_into().unwrap());
                let channels = u16::from_be_bytes(payload[2..4].try_into().unwrap());
                let sample_rate = u32::from_be_bytes(payload[4..8].try_into().unwrap());
                let block_align = u16::from_be_bytes(payload[12..14].try_into().unwrap());
                let bits_per_sample = u16::from_be_bytes(payload[14..16].try_into().unwrap());
                format = Some((
                    format_tag,
                    channels,
                    sample_rate,
                    block_align,
                    bits_per_sample,
                ));
            }
            b"data" => {
                let Some((format_tag, channels, sample_rate, block_align, bits_per_sample)) =
                    format
                else {
                    return Err(WavError::Malformed {
                        message: "data chunk appeared before fmt chunk".to_owned(),
                    });
                };
                let sample_format = sample_format(format_tag, bits_per_sample)?;
                validate_shape(channels, block_align, bits_per_sample, chunk_len)?;
                return Ok(RifxHeader {
                    channels,
                    sample_rate,
                    bits_per_sample,
                    sample_format,
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

fn sample_format(format_tag: u16, bits_per_sample: u16) -> Result<WavSampleFormat> {
    match (format_tag, bits_per_sample) {
        (WAVE_FORMAT_PCM, 8) => Ok(WavSampleFormat::Pcm8),
        (WAVE_FORMAT_PCM, 16) => Ok(WavSampleFormat::Pcm16),
        (WAVE_FORMAT_PCM, 24) => Ok(WavSampleFormat::Pcm24),
        (WAVE_FORMAT_PCM, 32) => Ok(WavSampleFormat::Pcm32),
        (WAVE_FORMAT_IEEE_FLOAT, 32) => Ok(WavSampleFormat::Float32),
        (WAVE_FORMAT_IEEE_FLOAT, 64) => Ok(WavSampleFormat::Float64),
        (0x0007, 8) => Ok(WavSampleFormat::ULaw),
        (0x0006, 8) => Ok(WavSampleFormat::ALaw),
        (WAVE_FORMAT_IEEE_FLOAT, _) => Err(WavError::UnsupportedSampleFormat {
            bits_per_sample,
            encoding: crate::WavSampleEncoding::Float,
        }),
        (0x0006 | 0x0007, _) => Err(WavError::UnsupportedSampleFormat {
            bits_per_sample,
            encoding: crate::WavSampleEncoding::Companded,
        }),
        _ => Err(WavError::UnsupportedSampleFormat {
            bits_per_sample,
            encoding: crate::WavSampleEncoding::Integer,
        }),
    }
}

fn validate_shape(
    channels: u16,
    block_align: u16,
    bits_per_sample: u16,
    data_len: usize,
) -> Result<()> {
    if channels == 0 || bits_per_sample == 0 || !bits_per_sample.is_multiple_of(8) {
        return Err(WavError::InvalidBufferShape);
    }
    let expected_align = channels.saturating_mul(bits_per_sample / 8);
    let bytes_per_sample = usize::from(bits_per_sample / 8);
    if block_align != expected_align
        || !data_len.is_multiple_of(bytes_per_sample)
        || !data_len.is_multiple_of(usize::from(block_align))
    {
        return Err(WavError::InvalidBufferShape);
    }
    Ok(())
}

/// Encodes a planar `f32` buffer as a RIFX/WAVE stream.
///
/// # Errors
///
/// Returns [`WavError::NonFiniteSample`] if a floating-point output sample is
/// NaN or infinite. Returns [`WavError::WriteFailed`] if the RIFX stream cannot
/// be written.
pub fn encode_rifx<W>(writer: W, audio: &AudioBuffer, options: WavEncodeOptions) -> Result<()>
where
    W: Write + Seek,
{
    encode_rifx_with_backend(writer, audio, options, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as RIFX/WAVE with an explicit backend request.
///
/// The backend only affects PCM16 conversion, matching the existing RIFF/WAVE
/// writer path.
///
/// # Errors
///
/// Returns the same sample validation and write errors as [`encode_rifx`].
pub fn encode_rifx_with_backend<W>(
    mut writer: W,
    audio: &AudioBuffer,
    options: WavEncodeOptions,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek,
{
    write_rifx_samples_with_backend(
        &mut writer,
        audio,
        options.sample_format(),
        requested_backend,
    )
}

/// Encodes a planar `f32` buffer as a RIFX/WAVE file on disk.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created. Propagates
/// the same sample validation and write errors as [`encode_rifx`].
pub fn encode_rifx_path(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    options: WavEncodeOptions,
) -> Result<()> {
    encode_rifx_path_with_backend(path, audio, options, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as a RIFX/WAVE file with an explicit backend.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created. Propagates
/// the same sample validation and write errors as [`encode_rifx_with_backend`].
pub fn encode_rifx_path_with_backend(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    options: WavEncodeOptions,
    requested_backend: BackendKind,
) -> Result<()> {
    let writer = File::create(path).map_err(|error| WavError::CreateFailed {
        message: error.to_string(),
    })?;
    encode_rifx_with_backend(writer, audio, options, requested_backend)
}

/// Configured RIFX/WAVE encoder behind the Auralis codec boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RifxWavEncoder {
    options: WavEncodeOptions,
    requested_backend: BackendKind,
}

impl RifxWavEncoder {
    /// Creates a configured RIFX/WAVE encoder.
    #[must_use]
    pub const fn new(options: WavEncodeOptions, requested_backend: BackendKind) -> Self {
        Self {
            options,
            requested_backend,
        }
    }
}

impl AudioEncoder for RifxWavEncoder {
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn encode(
        &self,
        input: &AudioBuffer,
        output: &mut dyn AudioOutput,
    ) -> auralis_codec::Result<EncodeSummary> {
        write_rifx_samples_with_backend(
            output,
            input,
            self.options.sample_format(),
            self.requested_backend,
        )
        .map_err(|error| CodecError::EncodeFailed {
            kind: CodecKind::Wav,
            message: error.to_string(),
        })?;

        Ok(EncodeSummary::new(
            CodecKind::Wav,
            input.spec(),
            input.frames(),
        ))
    }
}

/// Writer for RIFX/WAVE streams.
pub struct RifxWavWriter<W>
where
    W: Write + Seek,
{
    destination: Option<W>,
    options: WavEncodeOptions,
}

impl<W> RifxWavWriter<W>
where
    W: Write + Seek,
{
    /// Creates a RIFX/WAVE writer around a seekable byte stream.
    #[must_use]
    pub const fn new(writer: W, options: WavEncodeOptions) -> Self {
        Self {
            destination: Some(writer),
            options,
        }
    }

    /// Writes and finalizes one RIFX/WAVE stream.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_rifx`].
    pub fn write_rifx(&mut self, audio: &AudioBuffer) -> Result<()> {
        self.write_rifx_with_backend(audio, BackendKind::Scalar)
    }

    /// Writes and finalizes one RIFX/WAVE stream using an explicit backend.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_rifx`].
    pub fn write_rifx_with_backend(
        &mut self,
        audio: &AudioBuffer,
        requested_backend: BackendKind,
    ) -> Result<()> {
        let Some(mut destination) = self.destination.take() else {
            return Err(WavError::WriterAlreadyUsed);
        };
        write_rifx_samples_with_backend(
            &mut destination,
            audio,
            self.options.sample_format(),
            requested_backend,
        )
    }
}

impl<W> AudioWriter for RifxWavWriter<W>
where
    W: Write + Seek,
{
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn write_audio(&mut self, audio: &AudioBuffer) -> auralis_codec::Result<()> {
        self.write_rifx(audio)
            .map_err(|error| CodecError::EncodeFailed {
                kind: CodecKind::Wav,
                message: error.to_string(),
            })
    }
}

fn interleaved_samples(audio: &AudioBuffer) -> Result<Vec<f32>> {
    let channels = audio.channels().as_usize();
    let frames = usize::try_from(audio.frames().as_u64()).map_err(|_| WavError::WriteFailed {
        message: "frame count cannot be represented on this platform".to_owned(),
    })?;
    let sample_count = frames
        .checked_mul(channels)
        .ok_or_else(|| WavError::WriteFailed {
            message: "sample count cannot be represented on this platform".to_owned(),
        })?;
    let mut interleaved_f32 = Vec::with_capacity(sample_count);

    for frame_index in 0..frames {
        for channel_index in 0..channels {
            let sample =
                audio
                    .sample(channel_index, frame_index)
                    .ok_or_else(|| WavError::WriteFailed {
                        message: "audio buffer shape changed during encode".to_owned(),
                    })?;
            interleaved_f32.push(sample);
        }
    }

    Ok(interleaved_f32)
}

fn write_rifx_samples_with_backend<W>(
    writer: &mut W,
    audio: &AudioBuffer,
    sample_format: WavSampleFormat,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek + ?Sized,
{
    let channels = audio.channels().as_usize();
    let interleaved_f32 = interleaved_samples(audio)?;
    let payload = encode_payload(&interleaved_f32, channels, sample_format, requested_backend)?;
    write_header(writer, audio, payload.len(), sample_format)?;
    writer
        .write_all(&payload)
        .map_err(|error| WavError::WriteFailed {
            message: error.to_string(),
        })?;
    writer.flush().map_err(|error| WavError::WriteFailed {
        message: error.to_string(),
    })
}

fn encode_payload(
    interleaved_f32: &[f32],
    channels: usize,
    sample_format: WavSampleFormat,
    requested_backend: BackendKind,
) -> Result<Vec<u8>> {
    let mut payload = Vec::new();
    match sample_format {
        WavSampleFormat::Pcm8 => {
            let mut samples = vec![0; interleaved_f32.len()];
            f32_to_pcm8(interleaved_f32, &mut samples, channels)?;
            payload.reserve(samples.len());
            for sample in samples {
                payload.push(u8::try_from(i16::from(sample) + 128).map_err(|_| {
                    WavError::WriteFailed {
                        message: "PCM8 sample offset overflowed".to_owned(),
                    }
                })?);
            }
        }
        WavSampleFormat::Pcm16 => {
            let mut samples = vec![0; interleaved_f32.len()];
            f32_to_pcm16_with_backend(requested_backend, interleaved_f32, &mut samples, channels)?;
            payload.reserve(samples.len() * 2);
            for sample in samples {
                payload.extend_from_slice(&sample.to_be_bytes());
            }
        }
        WavSampleFormat::Pcm24 => {
            let mut samples = vec![0; interleaved_f32.len()];
            f32_to_pcm24(interleaved_f32, &mut samples, channels)?;
            payload.reserve(samples.len() * 3);
            for sample in samples {
                payload.extend_from_slice(&sample.to_be_bytes()[1..4]);
            }
        }
        WavSampleFormat::Pcm32 => {
            let mut samples = vec![0; interleaved_f32.len()];
            f32_to_pcm32(interleaved_f32, &mut samples, channels)?;
            payload.reserve(samples.len() * 4);
            for sample in samples {
                payload.extend_from_slice(&sample.to_be_bytes());
            }
        }
        WavSampleFormat::Float32 => {
            let mut samples = vec![0.0; interleaved_f32.len()];
            f32_to_float32(interleaved_f32, &mut samples, channels)?;
            payload.reserve(samples.len() * 4);
            for sample in samples {
                payload.extend_from_slice(&sample.to_be_bytes());
            }
        }
        WavSampleFormat::Float64 => {
            let mut samples = vec![0.0; interleaved_f32.len()];
            f32_to_float64(interleaved_f32, &mut samples, channels)?;
            payload.reserve(samples.len() * 8);
            for sample in samples {
                payload.extend_from_slice(&sample.to_be_bytes());
            }
        }
        WavSampleFormat::ULaw => {
            payload.reserve(interleaved_f32.len());
            for (sample_index, sample) in interleaved_f32.iter().copied().enumerate() {
                payload.push(G711Kind::ULaw.encode_sample(sample, sample_index, channels)?);
            }
        }
        WavSampleFormat::ALaw => {
            payload.reserve(interleaved_f32.len());
            for (sample_index, sample) in interleaved_f32.iter().copied().enumerate() {
                payload.push(G711Kind::ALaw.encode_sample(sample, sample_index, channels)?);
            }
        }
        _ => unreachable!("unsupported WAV sample format already rejected"),
    }
    Ok(payload)
}

fn write_header<W>(
    writer: &mut W,
    audio: &AudioBuffer,
    data_bytes: usize,
    sample_format: WavSampleFormat,
) -> Result<()>
where
    W: Write + ?Sized,
{
    let channels = audio.channels().as_u16();
    let sample_rate = audio.spec().sample_rate().as_u32();
    let data_bytes = u32::try_from(data_bytes).map_err(|_| WavError::WriteFailed {
        message: "RIFX WAV data is too large for a data chunk".to_owned(),
    })?;
    let bits_per_sample = bits_per_sample(sample_format);
    let bytes_per_sample = u32::from(bits_per_sample / 8);
    let byte_rate = sample_rate
        .checked_mul(u32::from(channels))
        .and_then(|rate| rate.checked_mul(bytes_per_sample))
        .ok_or_else(|| WavError::WriteFailed {
            message: "RIFX WAV byte rate overflowed".to_owned(),
        })?;
    let block_align =
        channels
            .checked_mul(bits_per_sample / 8)
            .ok_or_else(|| WavError::WriteFailed {
                message: "RIFX WAV block alignment overflowed".to_owned(),
            })?;
    let riff_size = 36_u32
        .checked_add(data_bytes)
        .ok_or_else(|| WavError::WriteFailed {
            message: "RIFX WAV size overflowed".to_owned(),
        })?;

    let result = (|| -> std::io::Result<()> {
        writer.write_all(b"RIFX")?;
        writer.write_all(&riff_size.to_be_bytes())?;
        writer.write_all(b"WAVE")?;
        writer.write_all(b"fmt ")?;
        writer.write_all(&16_u32.to_be_bytes())?;
        writer.write_all(&format_tag(sample_format).to_be_bytes())?;
        writer.write_all(&channels.to_be_bytes())?;
        writer.write_all(&sample_rate.to_be_bytes())?;
        writer.write_all(&byte_rate.to_be_bytes())?;
        writer.write_all(&block_align.to_be_bytes())?;
        writer.write_all(&bits_per_sample.to_be_bytes())?;
        writer.write_all(b"data")?;
        writer.write_all(&data_bytes.to_be_bytes())
    })();

    result.map_err(|error| WavError::WriteFailed {
        message: error.to_string(),
    })
}

#[allow(
    clippy::match_same_arms,
    reason = "The explicit PCM16 arm documents current support; the wildcard is only for future non-exhaustive variants."
)]
fn bits_per_sample(sample_format: WavSampleFormat) -> u16 {
    match sample_format {
        WavSampleFormat::Pcm8 | WavSampleFormat::ULaw | WavSampleFormat::ALaw => 8,
        WavSampleFormat::Pcm16 => 16,
        WavSampleFormat::Pcm24 => 24,
        WavSampleFormat::Pcm32 | WavSampleFormat::Float32 => 32,
        WavSampleFormat::Float64 => 64,
        _ => 16,
    }
}

fn format_tag(sample_format: WavSampleFormat) -> u16 {
    match sample_format {
        WavSampleFormat::Float32 | WavSampleFormat::Float64 => WAVE_FORMAT_IEEE_FLOAT,
        WavSampleFormat::ULaw => G711Kind::ULaw.format_tag(),
        WavSampleFormat::ALaw => G711Kind::ALaw.format_tag(),
        _ => WAVE_FORMAT_PCM,
    }
}
