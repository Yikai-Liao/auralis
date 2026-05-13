use std::{
    io::{Seek, Write},
    path::Path,
};

use auralis_codec::{
    AudioEncoder, AudioOutput, AudioWriter, CodecError, CodecKind, EncodeSummary, WavEncodeOptions,
};
use auralis_core::AudioBuffer;
use auralis_simd::BackendKind;

use crate::{Result, WavError, g711::G711Kind};

/// Encodes a planar `f32` buffer as a u-law WAV stream.
///
/// Samples are clipped to `[-1.0, 1.0]`, quantized to the G.711 u-law input
/// range, and companded into 8-bit WAV format-tag 7 payload bytes.
///
/// # Errors
///
/// Returns [`WavError::NonFiniteSample`] if any sample is NaN or infinite.
/// Returns [`WavError::WriteFailed`] if the WAV stream cannot be written.
pub fn encode_ulaw<W>(writer: W, audio: &AudioBuffer) -> Result<()>
where
    W: Write + Seek,
{
    encode_ulaw_with_backend(writer, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as u-law WAV with an explicit backend request.
///
/// u-law companding is deterministic scalar logic, so `requested_backend` does
/// not currently change the produced bytes.
///
/// # Errors
///
/// Returns the same sample validation and write errors as [`encode_ulaw`].
pub fn encode_ulaw_with_backend<W>(
    writer: W,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek,
{
    ULawWavWriter::new(writer).write_ulaw_with_backend(audio, requested_backend)
}

/// Encodes a planar `f32` buffer as a u-law WAV file on disk.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created.
pub fn encode_ulaw_path(path: impl AsRef<Path>, audio: &AudioBuffer) -> Result<()> {
    encode_ulaw_path_with_backend(path, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as a u-law WAV file with an explicit backend.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created.
pub fn encode_ulaw_path_with_backend(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()> {
    let writer = std::fs::File::create(path).map_err(|error| WavError::CreateFailed {
        message: error.to_string(),
    })?;
    encode_ulaw_with_backend(writer, audio, requested_backend)
}

/// Encodes a planar `f32` buffer as an A-law WAV stream.
///
/// Samples are clipped to `[-1.0, 1.0]`, quantized to the G.711 A-law input
/// range, and companded into 8-bit WAV format-tag 6 payload bytes.
///
/// # Errors
///
/// Returns [`WavError::NonFiniteSample`] if any sample is NaN or infinite.
/// Returns [`WavError::WriteFailed`] if the WAV stream cannot be written.
pub fn encode_alaw<W>(writer: W, audio: &AudioBuffer) -> Result<()>
where
    W: Write + Seek,
{
    encode_alaw_with_backend(writer, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as A-law WAV with an explicit backend request.
///
/// A-law companding is deterministic scalar logic, so `requested_backend` does
/// not currently change the produced bytes.
///
/// # Errors
///
/// Returns the same sample validation and write errors as [`encode_alaw`].
pub fn encode_alaw_with_backend<W>(
    writer: W,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()>
where
    W: Write + Seek,
{
    ALawWavWriter::new(writer).write_alaw_with_backend(audio, requested_backend)
}

/// Encodes a planar `f32` buffer as an A-law WAV file on disk.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created.
pub fn encode_alaw_path(path: impl AsRef<Path>, audio: &AudioBuffer) -> Result<()> {
    encode_alaw_path_with_backend(path, audio, BackendKind::Scalar)
}

/// Encodes a planar `f32` buffer as an A-law WAV file with an explicit backend.
///
/// # Errors
///
/// Returns [`WavError::CreateFailed`] if `path` cannot be created.
pub fn encode_alaw_path_with_backend(
    path: impl AsRef<Path>,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()> {
    let writer = std::fs::File::create(path).map_err(|error| WavError::CreateFailed {
        message: error.to_string(),
    })?;
    encode_alaw_with_backend(writer, audio, requested_backend)
}

/// Configured u-law WAV encoder behind the Auralis codec boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ULawWavEncoder {
    _options: WavEncodeOptions,
    requested_backend: BackendKind,
}

impl ULawWavEncoder {
    /// Creates a configured u-law WAV encoder.
    #[must_use]
    pub const fn new(options: WavEncodeOptions, requested_backend: BackendKind) -> Self {
        Self {
            _options: options,
            requested_backend,
        }
    }
}

impl AudioEncoder for ULawWavEncoder {
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn encode(
        &self,
        input: &AudioBuffer,
        output: &mut dyn AudioOutput,
    ) -> auralis_codec::Result<EncodeSummary> {
        write_g711_samples_with_backend(output, input, self.requested_backend, G711Kind::ULaw)
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

/// Configured A-law WAV encoder behind the Auralis codec boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ALawWavEncoder {
    _options: WavEncodeOptions,
    requested_backend: BackendKind,
}

impl ALawWavEncoder {
    /// Creates a configured A-law WAV encoder.
    #[must_use]
    pub const fn new(options: WavEncodeOptions, requested_backend: BackendKind) -> Self {
        Self {
            _options: options,
            requested_backend,
        }
    }
}

impl AudioEncoder for ALawWavEncoder {
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn encode(
        &self,
        input: &AudioBuffer,
        output: &mut dyn AudioOutput,
    ) -> auralis_codec::Result<EncodeSummary> {
        write_g711_samples_with_backend(output, input, self.requested_backend, G711Kind::ALaw)
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

/// Writer for u-law WAV streams.
pub struct ULawWavWriter<W>
where
    W: Write + Seek,
{
    destination: Option<W>,
}

impl<W> ULawWavWriter<W>
where
    W: Write + Seek,
{
    /// Creates a u-law WAV writer around a seekable byte stream.
    #[must_use]
    pub const fn new(writer: W) -> Self {
        Self {
            destination: Some(writer),
        }
    }

    /// Writes and finalizes one u-law WAV stream.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_ulaw`].
    pub fn write_ulaw(&mut self, audio: &AudioBuffer) -> Result<()> {
        self.write_ulaw_with_backend(audio, BackendKind::Scalar)
    }

    /// Writes and finalizes one u-law WAV stream using an explicit backend.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_ulaw`].
    pub fn write_ulaw_with_backend(
        &mut self,
        audio: &AudioBuffer,
        requested_backend: BackendKind,
    ) -> Result<()> {
        let Some(mut destination) = self.destination.take() else {
            return Err(WavError::WriterAlreadyUsed);
        };
        write_g711_samples_with_backend(&mut destination, audio, requested_backend, G711Kind::ULaw)
    }
}

impl<W> AudioWriter for ULawWavWriter<W>
where
    W: Write + Seek,
{
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn write_audio(&mut self, audio: &AudioBuffer) -> auralis_codec::Result<()> {
        self.write_ulaw(audio)
            .map_err(|error| CodecError::EncodeFailed {
                kind: CodecKind::Wav,
                message: error.to_string(),
            })
    }
}

/// Writer for A-law WAV streams.
pub struct ALawWavWriter<W>
where
    W: Write + Seek,
{
    destination: Option<W>,
}

impl<W> ALawWavWriter<W>
where
    W: Write + Seek,
{
    /// Creates an A-law WAV writer around a seekable byte stream.
    #[must_use]
    pub const fn new(writer: W) -> Self {
        Self {
            destination: Some(writer),
        }
    }

    /// Writes and finalizes one A-law WAV stream.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_alaw`].
    pub fn write_alaw(&mut self, audio: &AudioBuffer) -> Result<()> {
        self.write_alaw_with_backend(audio, BackendKind::Scalar)
    }

    /// Writes and finalizes one A-law WAV stream using an explicit backend.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::WriterAlreadyUsed`] if called after a previous
    /// write. Propagates the same validation and write errors as
    /// [`encode_alaw`].
    pub fn write_alaw_with_backend(
        &mut self,
        audio: &AudioBuffer,
        requested_backend: BackendKind,
    ) -> Result<()> {
        let Some(mut destination) = self.destination.take() else {
            return Err(WavError::WriterAlreadyUsed);
        };
        write_g711_samples_with_backend(&mut destination, audio, requested_backend, G711Kind::ALaw)
    }
}

impl<W> AudioWriter for ALawWavWriter<W>
where
    W: Write + Seek,
{
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn write_audio(&mut self, audio: &AudioBuffer) -> auralis_codec::Result<()> {
        self.write_alaw(audio)
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

fn write_g711_samples_with_backend<W>(
    writer: &mut W,
    audio: &AudioBuffer,
    requested_backend: BackendKind,
    kind: G711Kind,
) -> Result<()>
where
    W: Write + Seek + ?Sized,
{
    let channels = audio.channels().as_usize();
    let interleaved_f32 = interleaved_samples(audio)?;
    let _ = requested_backend;
    write_header(writer, audio, interleaved_f32.len(), kind)?;

    for (sample_index, sample) in interleaved_f32.into_iter().enumerate() {
        let byte = kind.encode_sample(sample, sample_index, channels)?;
        writer
            .write_all(&[byte])
            .map_err(|error| WavError::WriteFailed {
                message: error.to_string(),
            })?;
    }

    writer.flush().map_err(|error| WavError::WriteFailed {
        message: error.to_string(),
    })
}

fn write_header<W>(
    writer: &mut W,
    audio: &AudioBuffer,
    sample_count: usize,
    kind: G711Kind,
) -> Result<()>
where
    W: Write + ?Sized,
{
    let channels = audio.channels().as_u16();
    let sample_rate = audio.spec().sample_rate().as_u32();
    let data_bytes = u32::try_from(sample_count).map_err(|_| WavError::WriteFailed {
        message: format!(
            "{} WAV data is too large for a RIFF data chunk",
            kind.name()
        ),
    })?;
    let byte_rate = sample_rate
        .checked_mul(u32::from(channels))
        .ok_or_else(|| WavError::WriteFailed {
            message: format!("{} WAV byte rate overflowed", kind.name()),
        })?;
    let block_align = channels;
    let riff_size = 36_u32
        .checked_add(data_bytes)
        .ok_or_else(|| WavError::WriteFailed {
            message: format!("{} WAV RIFF size overflowed", kind.name()),
        })?;

    let result = (|| -> std::io::Result<()> {
        writer.write_all(b"RIFF")?;
        writer.write_all(&riff_size.to_le_bytes())?;
        writer.write_all(b"WAVE")?;
        writer.write_all(b"fmt ")?;
        writer.write_all(&16_u32.to_le_bytes())?;
        writer.write_all(&kind.format_tag().to_le_bytes())?;
        writer.write_all(&channels.to_le_bytes())?;
        writer.write_all(&sample_rate.to_le_bytes())?;
        writer.write_all(&byte_rate.to_le_bytes())?;
        writer.write_all(&block_align.to_le_bytes())?;
        writer.write_all(&8_u16.to_le_bytes())?;
        writer.write_all(b"data")?;
        writer.write_all(&data_bytes.to_le_bytes())
    })();

    result.map_err(|error| WavError::WriteFailed {
        message: error.to_string(),
    })
}
