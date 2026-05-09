use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use auralis_codec::{AudioReader, CodecError, CodecKind};
use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, SampleFormat, SampleRate};
use auralis_simd::BackendKind;

use crate::{
    Result, WavError,
    format::{ensure_pcm16, frame_count, malformed},
    sample_conversion::pcm16_to_f32_with_backend,
};

/// Decodes an entire PCM16 WAV stream into a planar `f32` buffer.
///
/// Samples are scaled by dividing each signed 16-bit value by `32768.0`, so
/// `-32768` maps exactly to `-1.0` and `32767` maps to `0.9999695`.
/// Interleaved WAV frames are converted into channel-major planar storage.
/// The returned [`AudioSpec`] uses [`SampleFormat::Float32`] because that is
/// Auralis' internal processing format.
///
/// # Errors
///
/// Returns [`WavError::UnsupportedSampleFormat`] for any WAV stream that is not
/// integer PCM with 16 bits per sample. Returns [`WavError::Malformed`] when the
/// RIFF/WAVE container or sample payload cannot be parsed.
pub fn decode_pcm16<R>(reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    decode_pcm16_with_backend(reader, BackendKind::Scalar)
}

/// Decodes an entire PCM16 WAV stream with an explicit sample-conversion backend.
///
/// The decoded audio is identical to [`decode_pcm16`]. `requested_backend`
/// controls only the PCM16-to-`f32` conversion kernel; unsupported SIMD
/// requests fall back through `auralis-simd` backend selection metadata before
/// decoding continues. This is intended for backend conformance tests and
/// deterministic backend-specific validation.
///
/// # Errors
///
/// Returns the same parsing, format, and shape errors as [`decode_pcm16`].
pub fn decode_pcm16_with_backend<R>(
    reader: R,
    requested_backend: BackendKind,
) -> Result<AudioBuffer>
where
    R: Read,
{
    Pcm16WavReader::new(reader)?.read_pcm16_with_backend(requested_backend)
}

/// Decodes a PCM16 WAV file from disk into a planar `f32` buffer.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_pcm16`].
pub fn decode_pcm16_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    decode_pcm16_path_with_backend(path, BackendKind::Scalar)
}

/// Decodes a PCM16 WAV file from disk with an explicit sample-conversion backend.
///
/// The decoded audio is identical to [`decode_pcm16_path`]. `requested_backend`
/// controls only the PCM16-to-`f32` conversion kernel; unsupported SIMD
/// requests fall back through `auralis-simd` backend selection metadata before
/// decoding continues.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_pcm16_with_backend`].
pub fn decode_pcm16_path_with_backend(
    path: impl AsRef<Path>,
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| WavError::OpenFailed {
        message: error.to_string(),
    })?;

    decode_pcm16_with_backend(BufReader::new(file), requested_backend)
}

/// Reader for PCM16 WAV streams.
///
/// The reader owns the underlying stream and decodes it at most once. It
/// implements [`AudioReader`] so callers that only need the codec boundary can
/// use it through the shared trait, while WAV-specific callers can use
/// [`Self::read_pcm16`] to preserve typed [`WavError`] values.
pub struct Pcm16WavReader<R>
where
    R: Read,
{
    inner: hound::WavReader<R>,
}

impl<R> Pcm16WavReader<R>
where
    R: Read,
{
    /// Creates a PCM16 WAV reader from a readable byte stream.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::Malformed`] when the stream cannot be parsed as a
    /// WAV container.
    pub fn new(reader: R) -> Result<Self> {
        let inner = hound::WavReader::new(reader).map_err(|error| malformed(&error))?;

        Ok(Self { inner })
    }

    /// Reads the entire stream into Auralis' internal planar `f32` buffer.
    ///
    /// # Errors
    ///
    /// Returns typed [`WavError`] values for unsupported sample formats,
    /// malformed samples, and invalid buffer shape metadata.
    pub fn read_pcm16(&mut self) -> Result<AudioBuffer> {
        self.read_pcm16_with_backend(BackendKind::Scalar)
    }

    /// Reads the entire stream into Auralis' internal planar `f32` buffer using
    /// an explicit sample-conversion backend.
    ///
    /// # Errors
    ///
    /// Returns typed [`WavError`] values for unsupported sample formats,
    /// malformed samples, and invalid buffer shape metadata.
    pub fn read_pcm16_with_backend(
        &mut self,
        requested_backend: BackendKind,
    ) -> Result<AudioBuffer> {
        let hound_spec = self.inner.spec();
        ensure_pcm16(hound_spec)?;

        let sample_rate =
            SampleRate::new(hound_spec.sample_rate).map_err(|_| WavError::InvalidSampleRate)?;
        let channels =
            ChannelCount::new(hound_spec.channels).map_err(|_| WavError::InvalidChannelCount)?;
        let frame_count = frame_count(self.inner.duration());
        let spec = AudioSpec::new(sample_rate, channels, SampleFormat::Float32);

        let frames =
            usize::try_from(frame_count.as_u64()).map_err(|_| WavError::InvalidBufferShape)?;
        let sample_capacity = frames
            .checked_mul(channels.as_usize())
            .ok_or(WavError::InvalidBufferShape)?;
        let mut planar = vec![0.0; sample_capacity];
        let mut interleaved_pcm16 = Vec::with_capacity(sample_capacity);

        for sample in self.inner.samples::<i16>() {
            interleaved_pcm16.push(sample.map_err(|error| malformed(&error))?);
        }

        let mut interleaved_f32 = vec![0.0; interleaved_pcm16.len()];
        pcm16_to_f32_with_backend(requested_backend, &interleaved_pcm16, &mut interleaved_f32)?;

        for (sample_index, sample) in interleaved_f32.into_iter().enumerate() {
            let frame_index = sample_index / channels.as_usize();
            let channel_index = sample_index % channels.as_usize();
            let planar_index = channel_index
                .checked_mul(frames)
                .and_then(|start| start.checked_add(frame_index))
                .ok_or(WavError::InvalidBufferShape)?;
            let destination = planar
                .get_mut(planar_index)
                .ok_or(WavError::InvalidBufferShape)?;

            *destination = sample;
        }

        AudioBuffer::from_planar_f32(spec, frame_count, planar)
            .map_err(|_| WavError::InvalidBufferShape)
    }
}

impl<R> AudioReader for Pcm16WavReader<R>
where
    R: Read,
{
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn read_audio(&mut self) -> auralis_codec::Result<AudioBuffer> {
        self.read_pcm16().map_err(CodecError::from)
    }
}
