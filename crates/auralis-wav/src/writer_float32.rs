use std::{
    io::{Seek, Write},
    path::Path,
};

use auralis_codec::{
    AudioEncoder, AudioOutput, AudioWriter, CodecError, CodecKind, EncodeSummary, WavEncodeOptions,
    WavSampleFormat,
};
use auralis_core::AudioBuffer;
use auralis_simd::BackendKind;

use crate::{Result, WavError, format::hound_spec, sample_conversion::f32_to_float32};

/// Encodes a planar `f32` buffer as a float32 WAV stream.
///
/// Samples are written without quantization or clipping.
///
/// # Errors
///
/// Returns [`WavError::NonFiniteSample`] if any sample is NaN or infinite.
/// Returns [`WavError::WriteFailed`] if the WAV stream cannot be written.
pub fn encode_float32<W>(writer: W, audio: &AudioBuffer) -> Result<()>
where
    W: Write + Seek,
{
    encode_float32_with_backend(writer, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as a float32 WAV stream with an explicit
/// backend request.
///
/// Float32 copying is deterministic scalar logic, so `requested_backend` does
/// not currently change the produced samples.
///
/// # Errors
///
/// Returns the same sample validation and write errors as [`encode_float32`].
pub fn encode_float32_with_backend<W>(
    writer: W,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek,
{
    Float32WavWriter::new(writer).write_float32_with_backend(audio, requested_backend)
}

/// Encodes a planar `f32` buffer as a float32 WAV file on disk.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created. Propagates
/// the same sample validation and write errors as [`encode_float32`].
pub fn encode_float32_path(path: impl AsRef<Path>, audio: &AudioBuffer) -> Result<()> {
    encode_float32_path_with_backend(path, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as a float32 WAV file with an explicit backend
/// request.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created. Propagates
/// the same sample validation and write errors as [`encode_float32_with_backend`].
pub fn encode_float32_path_with_backend(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()> {
    let writer = hound::WavWriter::create(path, hound_spec(audio, WavSampleFormat::Float32))
        .map_err(|error| WavError::CreateFailed {
            message: error.to_string(),
        })?;

    write_float32_samples_with_backend(writer, audio, requested_backend)
}

/// Configured float32 WAV encoder behind the Auralis codec boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Float32WavEncoder {
    _options: WavEncodeOptions,
    requested_backend: BackendKind,
}

impl Float32WavEncoder {
    /// Creates a configured float32 WAV encoder.
    #[must_use]
    pub const fn new(options: WavEncodeOptions, requested_backend: BackendKind) -> Self {
        Self {
            _options: options,
            requested_backend,
        }
    }
}

impl AudioEncoder for Float32WavEncoder {
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn encode(
        &self,
        input: &AudioBuffer,
        output: &mut dyn AudioOutput,
    ) -> auralis_codec::Result<EncodeSummary> {
        let writer = hound::WavWriter::new(output, hound_spec(input, WavSampleFormat::Float32))
            .map_err(|error| CodecError::EncodeFailed {
                kind: CodecKind::Wav,
                message: error.to_string(),
            })?;
        write_float32_samples_with_backend(writer, input, self.requested_backend).map_err(
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

/// Writer for float32 WAV streams.
pub struct Float32WavWriter<W>
where
    W: Write + Seek,
{
    destination: Option<W>,
}

impl<W> Float32WavWriter<W>
where
    W: Write + Seek,
{
    /// Creates a float32 WAV writer around a seekable byte stream.
    #[must_use]
    pub const fn new(writer: W) -> Self {
        Self {
            destination: Some(writer),
        }
    }

    /// Writes and finalizes one float32 WAV stream.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_float32`].
    pub fn write_float32(&mut self, audio: &AudioBuffer) -> Result<()> {
        self.write_float32_with_backend(audio, BackendKind::Scalar)
    }

    /// Writes and finalizes one float32 WAV stream using an explicit backend
    /// request.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_float32`].
    pub fn write_float32_with_backend(
        &mut self,
        audio: &AudioBuffer,
        requested_backend: BackendKind,
    ) -> Result<()> {
        let Some(destination) = self.destination.take() else {
            return Err(WavError::WriterAlreadyUsed);
        };
        let writer =
            hound::WavWriter::new(destination, hound_spec(audio, WavSampleFormat::Float32))
                .map_err(|error| WavError::WriteFailed {
                    message: error.to_string(),
                })?;

        write_float32_samples_with_backend(writer, audio, requested_backend)
    }
}

impl<W> AudioWriter for Float32WavWriter<W>
where
    W: Write + Seek,
{
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn write_audio(&mut self, audio: &AudioBuffer) -> auralis_codec::Result<()> {
        self.write_float32(audio)
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

fn write_float32_samples_with_backend<W>(
    mut writer: hound::WavWriter<W>,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek,
{
    let channels = audio.channels().as_usize();
    let interleaved_f32 = interleaved_samples(audio)?;
    let mut float32_samples = vec![0.0; interleaved_f32.len()];
    let _ = requested_backend;
    f32_to_float32(&interleaved_f32, &mut float32_samples, channels)?;

    for sample in float32_samples {
        writer
            .write_sample(sample)
            .map_err(|error| WavError::WriteFailed {
                message: error.to_string(),
            })?;
    }

    writer.finalize().map_err(|error| WavError::WriteFailed {
        message: error.to_string(),
    })
}
