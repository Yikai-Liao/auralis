use std::{
    io::{Seek, Write},
    path::Path,
};

use auralis_codec::{
    AudioEncoder, AudioOutput, AudioWriter, CodecError, CodecKind, EncodeSummary, WavEncodeOptions,
};
use auralis_core::AudioBuffer;
use auralis_simd::BackendKind;

use crate::{Result, WavError, sample_conversion::f32_to_float64};

/// Encodes a planar `f32` buffer as a float64 WAV stream.
///
/// Samples are widened to `f64` without quantization or clipping.
///
/// # Errors
///
/// Returns [`WavError::NonFiniteSample`] if any sample is NaN or infinite.
/// Returns [`WavError::WriteFailed`] if the WAV stream cannot be written.
pub fn encode_float64<W>(writer: W, audio: &AudioBuffer) -> Result<()>
where
    W: Write + Seek,
{
    encode_float64_with_backend(writer, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as a float64 WAV stream with an explicit
/// backend request.
///
/// Float64 widening is deterministic scalar logic, so `requested_backend` does
/// not currently change the produced samples.
///
/// # Errors
///
/// Returns the same sample validation and write errors as [`encode_float64`].
pub fn encode_float64_with_backend<W>(
    writer: W,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek,
{
    Float64WavWriter::new(writer).write_float64_with_backend(audio, requested_backend)
}

/// Encodes a planar `f32` buffer as a float64 WAV file on disk.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created. Propagates
/// the same sample validation and write errors as [`encode_float64`].
pub fn encode_float64_path(path: impl AsRef<Path>, audio: &AudioBuffer) -> Result<()> {
    encode_float64_path_with_backend(path, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as a float64 WAV file with an explicit backend
/// request.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created. Propagates
/// the same sample validation and write errors as [`encode_float64_with_backend`].
pub fn encode_float64_path_with_backend(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()> {
    let writer = std::fs::File::create(path).map_err(|error| WavError::CreateFailed {
        message: error.to_string(),
    })?;
    encode_float64_with_backend(writer, audio, requested_backend)
}

/// Configured float64 WAV encoder behind the Auralis codec boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Float64WavEncoder {
    _options: WavEncodeOptions,
    requested_backend: BackendKind,
}

impl Float64WavEncoder {
    /// Creates a configured float64 WAV encoder.
    #[must_use]
    pub const fn new(options: WavEncodeOptions, requested_backend: BackendKind) -> Self {
        Self {
            _options: options,
            requested_backend,
        }
    }
}

impl AudioEncoder for Float64WavEncoder {
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn encode(
        &self,
        input: &AudioBuffer,
        output: &mut dyn AudioOutput,
    ) -> auralis_codec::Result<EncodeSummary> {
        write_float64_samples_with_backend(output, input, self.requested_backend).map_err(
            |error| CodecError::EncodeFailed {
                kind: CodecKind::Wav,
                message: error.to_string(),
            },
        )?;

        Ok(EncodeSummary::new(
            CodecKind::Wav,
            input.spec(),
            input.frames(),
        ))
    }
}

/// Writer for float64 WAV streams.
pub struct Float64WavWriter<W>
where
    W: Write + Seek,
{
    destination: Option<W>,
}

impl<W> Float64WavWriter<W>
where
    W: Write + Seek,
{
    /// Creates a float64 WAV writer around a seekable byte stream.
    #[must_use]
    pub const fn new(writer: W) -> Self {
        Self {
            destination: Some(writer),
        }
    }

    /// Writes and finalizes one float64 WAV stream.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_float64`].
    pub fn write_float64(&mut self, audio: &AudioBuffer) -> Result<()> {
        self.write_float64_with_backend(audio, BackendKind::Scalar)
    }

    /// Writes and finalizes one float64 WAV stream using an explicit backend
    /// request.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_float64`].
    pub fn write_float64_with_backend(
        &mut self,
        audio: &AudioBuffer,
        requested_backend: BackendKind,
    ) -> Result<()> {
        let Some(mut destination) = self.destination.take() else {
            return Err(WavError::WriterAlreadyUsed);
        };
        write_float64_samples_with_backend(&mut destination, audio, requested_backend)
    }
}

impl<W> AudioWriter for Float64WavWriter<W>
where
    W: Write + Seek,
{
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn write_audio(&mut self, audio: &AudioBuffer) -> auralis_codec::Result<()> {
        self.write_float64(audio)
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

fn write_float64_samples_with_backend<W>(
    writer: &mut W,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek + ?Sized,
{
    let channels = audio.channels().as_usize();
    let interleaved_f32 = interleaved_samples(audio)?;
    let mut float64_samples = vec![0.0; interleaved_f32.len()];
    let _ = requested_backend;
    f32_to_float64(&interleaved_f32, &mut float64_samples, channels)?;
    write_header(writer, audio, float64_samples.len())?;

    for sample in float64_samples {
        writer
            .write_all(&sample.to_le_bytes())
            .map_err(|error| WavError::WriteFailed {
                message: error.to_string(),
            })?;
    }

    writer.flush().map_err(|error| WavError::WriteFailed {
        message: error.to_string(),
    })
}

fn write_header<W>(writer: &mut W, audio: &AudioBuffer, sample_count: usize) -> Result<()>
where
    W: Write + ?Sized,
{
    let channels = audio.channels().as_u16();
    let sample_rate = audio.spec().sample_rate().as_u32();
    let data_bytes = sample_count
        .checked_mul(8)
        .and_then(|bytes| u32::try_from(bytes).ok())
        .ok_or_else(|| WavError::WriteFailed {
            message: "float64 WAV data is too large for a RIFF data chunk".to_owned(),
        })?;
    let byte_rate = sample_rate
        .checked_mul(u32::from(channels))
        .and_then(|rate| rate.checked_mul(8))
        .ok_or_else(|| WavError::WriteFailed {
            message: "float64 WAV byte rate overflowed".to_owned(),
        })?;
    let block_align = channels
        .checked_mul(8)
        .ok_or_else(|| WavError::WriteFailed {
            message: "float64 WAV block alignment overflowed".to_owned(),
        })?;
    let riff_size = 36_u32
        .checked_add(data_bytes)
        .ok_or_else(|| WavError::WriteFailed {
            message: "float64 WAV RIFF size overflowed".to_owned(),
        })?;

    let result = (|| -> std::io::Result<()> {
        writer.write_all(b"RIFF")?;
        writer.write_all(&riff_size.to_le_bytes())?;
        writer.write_all(b"WAVE")?;
        writer.write_all(b"fmt ")?;
        writer.write_all(&16_u32.to_le_bytes())?;
        writer.write_all(&3_u16.to_le_bytes())?;
        writer.write_all(&channels.to_le_bytes())?;
        writer.write_all(&sample_rate.to_le_bytes())?;
        writer.write_all(&byte_rate.to_le_bytes())?;
        writer.write_all(&block_align.to_le_bytes())?;
        writer.write_all(&64_u16.to_le_bytes())?;
        writer.write_all(b"data")?;
        writer.write_all(&data_bytes.to_le_bytes())
    })();

    result.map_err(|error| WavError::WriteFailed {
        message: error.to_string(),
    })
}
