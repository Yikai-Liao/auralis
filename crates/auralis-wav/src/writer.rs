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

use crate::{
    Result, WavError,
    format::hound_spec,
    sample_conversion::{f32_to_pcm8, f32_to_pcm16_with_backend},
};

/// Encodes a planar `f32` buffer as a PCM16 WAV stream.
///
/// Samples are clipped to `[-1.0, 1.0]` before conversion. Negative samples use
/// a `32768.0` scale, then the quantized integer is clipped to the `i16` range.
/// This maps `-1.0` to `i16::MIN`, `1.0` to `i16::MAX`, and already-decoded
/// PCM16 values back to their original integer representation where possible.
/// Intermediate values are rounded to the nearest integer. The input buffer is
/// never modified.
///
/// # Errors
///
/// Returns [`WavError::NonFiniteSample`] if any sample is NaN or infinite.
/// Returns [`WavError::WriteFailed`] if the WAV header, sample payload, or
/// finalization step cannot be written.
pub fn encode_pcm16<W>(writer: W, audio: &AudioBuffer) -> Result<()>
where
    W: Write + Seek,
{
    encode_pcm16_with_backend(writer, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as a PCM16 WAV stream with an explicit
/// sample-conversion backend.
///
/// The encoded audio is identical to [`encode_pcm16`]. `requested_backend`
/// controls only the `f32`-to-PCM16 conversion kernel; unsupported SIMD
/// requests fall back through `auralis-simd` backend selection metadata before
/// encoding continues. This is intended for backend conformance tests and
/// deterministic backend-specific validation.
///
/// # Errors
///
/// Returns the same sample validation and write errors as [`encode_pcm16`].
pub fn encode_pcm16_with_backend<W>(
    writer: W,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek,
{
    Pcm16WavWriter::new(writer).write_pcm16_with_backend(audio, requested_backend)
}

/// Encodes a planar `f32` buffer as a PCM8 WAV stream.
///
/// Samples are clipped to `[-1.0, 1.0]`, quantized to signed 8-bit PCM, then
/// written through WAV's unsigned-offset byte representation.
///
/// # Errors
///
/// Returns [`WavError::NonFiniteSample`] if any sample is NaN or infinite.
/// Returns [`WavError::WriteFailed`] if the WAV header, sample payload, or
/// finalization step cannot be written.
pub fn encode_pcm8<W>(writer: W, audio: &AudioBuffer) -> Result<()>
where
    W: Write + Seek,
{
    encode_pcm8_with_backend(writer, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as a PCM8 WAV stream with an explicit backend
/// request.
///
/// PCM8 quantization is currently scalar, so `requested_backend` does not
/// change the produced samples.
///
/// # Errors
///
/// Returns the same sample validation and write errors as [`encode_pcm8`].
pub fn encode_pcm8_with_backend<W>(
    writer: W,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek,
{
    Pcm8WavWriter::new(writer).write_pcm8_with_backend(audio, requested_backend)
}

/// Encodes a planar `f32` buffer as a PCM16 WAV file on disk.
///
/// Existing files at `path` are overwritten. See [`encode_pcm16`] for the
/// clipping and quantization rules.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created. Propagates
/// the same sample validation and write errors as [`encode_pcm16`].
pub fn encode_pcm16_path(path: impl AsRef<Path>, audio: &AudioBuffer) -> Result<()> {
    encode_pcm16_path_with_backend(path, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as a PCM16 WAV file with an explicit
/// sample-conversion backend.
///
/// Existing files at `path` are overwritten. See [`encode_pcm16`] for the
/// clipping and quantization rules and [`encode_pcm16_with_backend`] for
/// backend selection behavior.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created. Propagates
/// the same sample validation and write errors as [`encode_pcm16`].
pub fn encode_pcm16_path_with_backend(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()> {
    let writer = hound::WavWriter::create(path, hound_spec(audio, WavSampleFormat::Pcm16))
        .map_err(|error| WavError::CreateFailed {
            message: error.to_string(),
        })?;

    write_pcm16_samples_with_backend(writer, audio, requested_backend)
}

/// Encodes a planar `f32` buffer as a PCM8 WAV file on disk.
///
/// Existing files at `path` are overwritten. See [`encode_pcm8`] for the
/// clipping and quantization rules.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created. Propagates
/// the same sample validation and write errors as [`encode_pcm8`].
pub fn encode_pcm8_path(path: impl AsRef<Path>, audio: &AudioBuffer) -> Result<()> {
    encode_pcm8_path_with_backend(path, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as a PCM8 WAV file with an explicit backend
/// request.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created. Propagates
/// the same sample validation and write errors as [`encode_pcm8_with_backend`].
pub fn encode_pcm8_path_with_backend(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()> {
    let writer = hound::WavWriter::create(path, hound_spec(audio, WavSampleFormat::Pcm8)).map_err(
        |error| WavError::CreateFailed {
            message: error.to_string(),
        },
    )?;

    write_pcm8_samples_with_backend(writer, audio, requested_backend)
}

/// Configured PCM16 WAV encoder behind the Auralis codec boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pcm16WavEncoder {
    _options: WavEncodeOptions,
    requested_backend: BackendKind,
}

impl Pcm16WavEncoder {
    /// Creates a configured PCM16 WAV encoder.
    #[must_use]
    pub const fn new(options: WavEncodeOptions, requested_backend: BackendKind) -> Self {
        Self {
            _options: options,
            requested_backend,
        }
    }
}

impl AudioEncoder for Pcm16WavEncoder {
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn encode(
        &self,
        input: &AudioBuffer,
        output: &mut dyn AudioOutput,
    ) -> auralis_codec::Result<EncodeSummary> {
        let writer = hound::WavWriter::new(output, hound_spec(input, WavSampleFormat::Pcm16))
            .map_err(|error| CodecError::EncodeFailed {
                kind: CodecKind::Wav,
                message: error.to_string(),
            })?;
        write_pcm16_samples_with_backend(writer, input, self.requested_backend).map_err(
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

/// Configured PCM8 WAV encoder behind the Auralis codec boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pcm8WavEncoder {
    _options: WavEncodeOptions,
    requested_backend: BackendKind,
}

impl Pcm8WavEncoder {
    /// Creates a configured PCM8 WAV encoder.
    #[must_use]
    pub const fn new(options: WavEncodeOptions, requested_backend: BackendKind) -> Self {
        Self {
            _options: options,
            requested_backend,
        }
    }
}

impl AudioEncoder for Pcm8WavEncoder {
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn encode(
        &self,
        input: &AudioBuffer,
        output: &mut dyn AudioOutput,
    ) -> auralis_codec::Result<EncodeSummary> {
        let writer = hound::WavWriter::new(output, hound_spec(input, WavSampleFormat::Pcm8))
            .map_err(|error| CodecError::EncodeFailed {
                kind: CodecKind::Wav,
                message: error.to_string(),
            })?;
        write_pcm8_samples_with_backend(writer, input, self.requested_backend).map_err(
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

/// Writer for PCM16 WAV streams.
///
/// The writer is intentionally one-shot: WAV headers are finalized after
/// [`Self::write_pcm16`] so later write attempts return
/// [`WavError::WriterAlreadyUsed`] instead of silently appending invalid data.
pub struct Pcm16WavWriter<W>
where
    W: Write + Seek,
{
    destination: Option<W>,
}

impl<W> Pcm16WavWriter<W>
where
    W: Write + Seek,
{
    /// Creates a PCM16 WAV writer around a seekable byte stream.
    ///
    /// The stream is expected to be positioned at the start of the output.
    #[must_use]
    pub const fn new(writer: W) -> Self {
        Self {
            destination: Some(writer),
        }
    }

    /// Writes and finalizes one PCM16 WAV stream.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_pcm16`].
    pub fn write_pcm16(&mut self, audio: &AudioBuffer) -> Result<()> {
        self.write_pcm16_with_backend(audio, BackendKind::Scalar)
    }

    /// Writes and finalizes one PCM16 WAV stream using an explicit
    /// sample-conversion backend.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_pcm16`].
    pub fn write_pcm16_with_backend(
        &mut self,
        audio: &AudioBuffer,
        requested_backend: BackendKind,
    ) -> Result<()> {
        let Some(destination) = self.destination.take() else {
            return Err(WavError::WriterAlreadyUsed);
        };
        let writer = hound::WavWriter::new(destination, hound_spec(audio, WavSampleFormat::Pcm16))
            .map_err(|error| WavError::WriteFailed {
                message: error.to_string(),
            })?;

        write_pcm16_samples_with_backend(writer, audio, requested_backend)
    }
}

impl<W> AudioWriter for Pcm16WavWriter<W>
where
    W: Write + Seek,
{
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn write_audio(&mut self, audio: &AudioBuffer) -> auralis_codec::Result<()> {
        self.write_pcm16(audio)
            .map_err(|error| CodecError::EncodeFailed {
                kind: CodecKind::Wav,
                message: error.to_string(),
            })
    }
}

/// Writer for PCM8 WAV streams.
pub struct Pcm8WavWriter<W>
where
    W: Write + Seek,
{
    destination: Option<W>,
}

impl<W> Pcm8WavWriter<W>
where
    W: Write + Seek,
{
    /// Creates a PCM8 WAV writer around a seekable byte stream.
    #[must_use]
    pub const fn new(writer: W) -> Self {
        Self {
            destination: Some(writer),
        }
    }

    /// Writes and finalizes one PCM8 WAV stream.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_pcm8`].
    pub fn write_pcm8(&mut self, audio: &AudioBuffer) -> Result<()> {
        self.write_pcm8_with_backend(audio, BackendKind::Scalar)
    }

    /// Writes and finalizes one PCM8 WAV stream using an explicit backend
    /// request.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_pcm8`].
    pub fn write_pcm8_with_backend(
        &mut self,
        audio: &AudioBuffer,
        requested_backend: BackendKind,
    ) -> Result<()> {
        let Some(destination) = self.destination.take() else {
            return Err(WavError::WriterAlreadyUsed);
        };
        let writer = hound::WavWriter::new(destination, hound_spec(audio, WavSampleFormat::Pcm8))
            .map_err(|error| WavError::WriteFailed {
            message: error.to_string(),
        })?;

        write_pcm8_samples_with_backend(writer, audio, requested_backend)
    }
}

impl<W> AudioWriter for Pcm8WavWriter<W>
where
    W: Write + Seek,
{
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn write_audio(&mut self, audio: &AudioBuffer) -> auralis_codec::Result<()> {
        self.write_pcm8(audio)
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

fn write_pcm16_samples_with_backend<W>(
    mut writer: hound::WavWriter<W>,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek,
{
    let channels = audio.channels().as_usize();
    let interleaved_f32 = interleaved_samples(audio)?;
    let mut interleaved_pcm16 = vec![0; interleaved_f32.len()];
    f32_to_pcm16_with_backend(
        requested_backend,
        &interleaved_f32,
        &mut interleaved_pcm16,
        channels,
    )?;

    for sample in interleaved_pcm16 {
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

fn write_pcm8_samples_with_backend<W>(
    mut writer: hound::WavWriter<W>,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek,
{
    let channels = audio.channels().as_usize();
    let interleaved_f32 = interleaved_samples(audio)?;
    let mut interleaved_pcm8 = vec![0; interleaved_f32.len()];
    let _ = requested_backend;
    f32_to_pcm8(&interleaved_f32, &mut interleaved_pcm8, channels)?;

    for sample in interleaved_pcm8 {
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
