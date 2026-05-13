use std::{
    fs::File,
    io::{BufReader, Cursor, Read},
    path::Path,
};

use auralis_codec::{AudioReader, CodecError, CodecKind, WavSampleFormat};
use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, SampleFormat, SampleRate};
use auralis_simd::BackendKind;

use crate::{
    Result, WavError,
    format::{ensure_pcm16, frame_count, malformed, wav_sample_format},
    reader_float64::is_float64_wav_bytes,
    reader_g711::{decode_wav_bytes_or_hound, is_g711_wav_bytes},
    rifx::{decode_rifx_bytes, is_rifx_wav_bytes},
    sample_conversion::{
        float32_to_f32, pcm8_to_f32, pcm16_to_f32_with_backend, pcm24_to_f32, pcm32_to_f32,
    },
};

/// Decodes an entire supported WAV stream into a planar `f32` buffer.
///
/// Currently supported sample formats are PCM8, PCM16, PCM24, PCM32, float32,
/// float64, u-law, and A-law in RIFF/WAVE or RIFX/WAVE containers. The
/// returned [`AudioSpec`] always uses
/// [`SampleFormat::Float32`] because that is Auralis' internal processing
/// format.
///
/// # Errors
///
/// Returns [`WavError::UnsupportedSampleFormat`] for any WAV stream that is not
/// PCM8, PCM16, PCM24, PCM32, float32, float64, u-law, or A-law. Returns
/// [`WavError::Malformed`] when the RIFF/WAVE container or sample payload
/// cannot be parsed.
pub fn decode_wav<R>(reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    decode_wav_with_backend(reader, BackendKind::Scalar)
}

/// Decodes an entire supported linear PCM WAV stream with an explicit
/// sample-conversion backend.
///
/// PCM16 uses the requested PCM16-to-`f32` backend. PCM8 and PCM24 conversion
/// are small scalar normalization steps because there is no separate SIMD
/// primitive yet.
///
/// # Errors
///
/// Returns the same parsing, format, and shape errors as [`decode_wav`].
pub fn decode_wav_with_backend<R>(reader: R, requested_backend: BackendKind) -> Result<AudioBuffer>
where
    R: Read,
{
    let mut reader = reader;
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| WavError::Malformed {
            message: error.to_string(),
        })?;
    if is_rifx_wav_bytes(&bytes) {
        decode_rifx_bytes(&bytes, requested_backend)
    } else {
        decode_wav_bytes_or_hound(&bytes, requested_backend)
    }
}

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
    let mut reader = reader;
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| WavError::Malformed {
            message: error.to_string(),
        })?;
    if is_float64_wav_bytes(&bytes) || is_g711_wav_bytes(&bytes) || is_rifx_wav_bytes(&bytes) {
        return Err(unsupported_specific_format(&bytes));
    }

    Pcm16WavReader::new(Cursor::new(bytes))?.read_pcm16_with_backend(requested_backend)
}

fn unsupported_specific_format(bytes: &[u8]) -> WavError {
    if is_float64_wav_bytes(bytes) {
        WavError::UnsupportedSampleFormat {
            bits_per_sample: 64,
            encoding: crate::WavSampleEncoding::Float,
        }
    } else if is_rifx_wav_bytes(bytes) {
        WavError::UnsupportedSampleFormat {
            bits_per_sample: 16,
            encoding: crate::WavSampleEncoding::Integer,
        }
    } else {
        WavError::UnsupportedSampleFormat {
            bits_per_sample: 8,
            encoding: crate::WavSampleEncoding::Companded,
        }
    }
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

/// Decodes a supported linear PCM WAV file from disk into a planar `f32`
/// buffer.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_wav`].
pub fn decode_wav_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    decode_wav_path_with_backend(path, BackendKind::Scalar)
}

/// Decodes a supported linear PCM WAV file from disk with an explicit
/// sample-conversion backend.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_wav_with_backend`].
pub fn decode_wav_path_with_backend(
    path: impl AsRef<Path>,
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| WavError::OpenFailed {
        message: error.to_string(),
    })?;

    decode_wav_with_backend(BufReader::new(file), requested_backend)
}

/// Decodes an entire PCM8 WAV stream into a planar `f32` buffer.
///
/// Samples are first interpreted as signed `i8` values through the WAV
/// unsigned-offset convention, then scaled by dividing by `128.0`.
///
/// # Errors
///
/// Returns [`WavError::UnsupportedSampleFormat`] when the stream is not PCM8.
/// Returns [`WavError::Malformed`] when the RIFF/WAVE container or sample
/// payload cannot be parsed.
pub fn decode_pcm8<R>(reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    decode_pcm8_with_backend(reader, BackendKind::Scalar)
}

/// Decodes an entire PCM8 WAV stream with an explicit sample-conversion
/// backend.
///
/// PCM8 normalization is deterministic scalar logic, so `requested_backend`
/// does not currently change the produced samples.
///
/// # Errors
///
/// Returns the same parsing, format, and shape errors as [`decode_pcm8`].
pub fn decode_pcm8_with_backend<R>(reader: R, requested_backend: BackendKind) -> Result<AudioBuffer>
where
    R: Read,
{
    let mut reader = AnyPcmWavReader::new(reader)?;
    reader.read_expected_format_with_backend(WavSampleFormat::Pcm8, requested_backend)
}

/// Decodes a PCM8 WAV file from disk into a planar `f32` buffer.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_pcm8`].
pub fn decode_pcm8_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    decode_pcm8_path_with_backend(path, BackendKind::Scalar)
}

/// Decodes a PCM8 WAV file from disk with an explicit sample-conversion
/// backend.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_pcm8_with_backend`].
pub fn decode_pcm8_path_with_backend(
    path: impl AsRef<Path>,
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| WavError::OpenFailed {
        message: error.to_string(),
    })?;

    decode_pcm8_with_backend(BufReader::new(file), requested_backend)
}

/// Decodes an entire PCM24 WAV stream into a planar `f32` buffer.
///
/// Samples are interpreted as signed 24-bit PCM values stored in 32-bit
/// containers, then scaled by dividing by `8_388_608.0`.
///
/// # Errors
///
/// Returns [`WavError::UnsupportedSampleFormat`] when the stream is not PCM24.
/// Returns [`WavError::Malformed`] when the RIFF/WAVE container or sample
/// payload cannot be parsed.
pub fn decode_pcm24<R>(reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    decode_pcm24_with_backend(reader, BackendKind::Scalar)
}

/// Decodes an entire PCM24 WAV stream with an explicit sample-conversion
/// backend.
///
/// PCM24 normalization is deterministic scalar logic, so `requested_backend`
/// does not currently change the produced samples.
///
/// # Errors
///
/// Returns the same parsing, format, and shape errors as [`decode_pcm24`].
pub fn decode_pcm24_with_backend<R>(
    reader: R,
    requested_backend: BackendKind,
) -> Result<AudioBuffer>
where
    R: Read,
{
    let mut reader = AnyPcmWavReader::new(reader)?;
    reader.read_expected_format_with_backend(WavSampleFormat::Pcm24, requested_backend)
}

/// Decodes a PCM24 WAV file from disk into a planar `f32` buffer.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_pcm24`].
pub fn decode_pcm24_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    decode_pcm24_path_with_backend(path, BackendKind::Scalar)
}

/// Decodes a PCM24 WAV file from disk with an explicit sample-conversion
/// backend.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_pcm24_with_backend`].
pub fn decode_pcm24_path_with_backend(
    path: impl AsRef<Path>,
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| WavError::OpenFailed {
        message: error.to_string(),
    })?;

    decode_pcm24_with_backend(BufReader::new(file), requested_backend)
}

/// Decodes an entire PCM32 WAV stream into a planar `f32` buffer.
///
/// Samples are interpreted as signed 32-bit PCM values, then scaled by
/// dividing by `2_147_483_648.0`.
///
/// # Errors
///
/// Returns [`WavError::UnsupportedSampleFormat`] when the stream is not PCM32.
/// Returns [`WavError::Malformed`] when the RIFF/WAVE container or sample
/// payload cannot be parsed.
pub fn decode_pcm32<R>(reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    decode_pcm32_with_backend(reader, BackendKind::Scalar)
}

/// Decodes an entire PCM32 WAV stream with an explicit sample-conversion
/// backend.
///
/// PCM32 normalization is deterministic scalar logic, so `requested_backend`
/// does not currently change the produced samples.
///
/// # Errors
///
/// Returns the same parsing, format, and shape errors as [`decode_pcm32`].
pub fn decode_pcm32_with_backend<R>(
    reader: R,
    requested_backend: BackendKind,
) -> Result<AudioBuffer>
where
    R: Read,
{
    let mut reader = AnyPcmWavReader::new(reader)?;
    reader.read_expected_format_with_backend(WavSampleFormat::Pcm32, requested_backend)
}

/// Decodes a PCM32 WAV file from disk into a planar `f32` buffer.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_pcm32`].
pub fn decode_pcm32_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    decode_pcm32_path_with_backend(path, BackendKind::Scalar)
}

/// Decodes a PCM32 WAV file from disk with an explicit sample-conversion
/// backend.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_pcm32_with_backend`].
pub fn decode_pcm32_path_with_backend(
    path: impl AsRef<Path>,
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| WavError::OpenFailed {
        message: error.to_string(),
    })?;

    decode_pcm32_with_backend(BufReader::new(file), requested_backend)
}

/// Decodes an entire float32 WAV stream into a planar `f32` buffer.
///
/// Samples are copied into Auralis' internal processing buffer without
/// quantization or clipping.
///
/// # Errors
///
/// Returns [`WavError::UnsupportedSampleFormat`] when the stream is not
/// 32-bit IEEE float. Returns [`WavError::Malformed`] when the RIFF/WAVE
/// container or sample payload cannot be parsed.
pub fn decode_float32<R>(reader: R) -> Result<AudioBuffer>
where
    R: Read,
{
    decode_float32_with_backend(reader, BackendKind::Scalar)
}

/// Decodes an entire float32 WAV stream with an explicit backend request.
///
/// Float32 copying is deterministic scalar logic, so `requested_backend` does
/// not currently change the produced samples.
///
/// # Errors
///
/// Returns the same parsing, format, and shape errors as [`decode_float32`].
pub fn decode_float32_with_backend<R>(
    reader: R,
    requested_backend: BackendKind,
) -> Result<AudioBuffer>
where
    R: Read,
{
    let mut reader = AnyPcmWavReader::new(reader)?;
    reader.read_expected_format_with_backend(WavSampleFormat::Float32, requested_backend)
}

/// Decodes a float32 WAV file from disk into a planar `f32` buffer.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_float32`].
pub fn decode_float32_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    decode_float32_path_with_backend(path, BackendKind::Scalar)
}

/// Decodes a float32 WAV file from disk with an explicit backend request.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_float32_with_backend`].
pub fn decode_float32_path_with_backend(
    path: impl AsRef<Path>,
    requested_backend: BackendKind,
) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| WavError::OpenFailed {
        message: error.to_string(),
    })?;

    decode_float32_with_backend(BufReader::new(file), requested_backend)
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
    inner: AnyPcmWavReader<R>,
}

/// Reader for supported linear PCM WAV streams.
pub struct AnyPcmWavReader<R>
where
    R: Read,
{
    inner: hound::WavReader<R>,
}

impl<R> AnyPcmWavReader<R>
where
    R: Read,
{
    /// Creates a reader from a readable WAV byte stream.
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
    pub fn read_wav(&mut self) -> Result<AudioBuffer> {
        self.read_wav_with_backend(BackendKind::Scalar)
    }

    /// Reads the entire stream into Auralis' internal planar `f32` buffer with
    /// an explicit sample-conversion backend.
    ///
    /// # Errors
    ///
    /// Returns typed [`WavError`] values for unsupported sample formats,
    /// malformed samples, and invalid buffer shape metadata.
    pub fn read_wav_with_backend(&mut self, requested_backend: BackendKind) -> Result<AudioBuffer> {
        let sample_format = wav_sample_format(self.inner.spec())?;
        self.read_sample_format_with_backend(sample_format, requested_backend)
    }

    fn read_expected_format_with_backend(
        &mut self,
        expected_format: WavSampleFormat,
        requested_backend: BackendKind,
    ) -> Result<AudioBuffer> {
        let actual_format = wav_sample_format(self.inner.spec())?;
        if actual_format != expected_format {
            let spec = self.inner.spec();
            return Err(WavError::UnsupportedSampleFormat {
                bits_per_sample: spec.bits_per_sample,
                encoding: match spec.sample_format {
                    hound::SampleFormat::Int => crate::WavSampleEncoding::Integer,
                    hound::SampleFormat::Float => crate::WavSampleEncoding::Float,
                },
            });
        }

        self.read_sample_format_with_backend(expected_format, requested_backend)
    }

    fn read_sample_format_with_backend(
        &mut self,
        sample_format: WavSampleFormat,
        requested_backend: BackendKind,
    ) -> Result<AudioBuffer> {
        let hound_spec = self.inner.spec();

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
        let mut interleaved_f32 = vec![0.0; sample_capacity];

        match sample_format {
            WavSampleFormat::Pcm8 => {
                let mut interleaved_pcm8 = Vec::with_capacity(sample_capacity);
                for sample in self.inner.samples::<i8>() {
                    interleaved_pcm8.push(sample.map_err(|error| malformed(&error))?);
                }
                pcm8_to_f32(&interleaved_pcm8, &mut interleaved_f32)?;
            }
            WavSampleFormat::Pcm16 => {
                let mut interleaved_pcm16 = Vec::with_capacity(sample_capacity);
                for sample in self.inner.samples::<i16>() {
                    interleaved_pcm16.push(sample.map_err(|error| malformed(&error))?);
                }
                pcm16_to_f32_with_backend(
                    requested_backend,
                    &interleaved_pcm16,
                    &mut interleaved_f32,
                )?;
            }
            WavSampleFormat::Pcm24 => {
                let mut interleaved_pcm24 = Vec::with_capacity(sample_capacity);
                for sample in self.inner.samples::<i32>() {
                    interleaved_pcm24.push(sample.map_err(|error| malformed(&error))?);
                }
                pcm24_to_f32(&interleaved_pcm24, &mut interleaved_f32)?;
            }
            WavSampleFormat::Pcm32 => {
                let mut interleaved_pcm32 = Vec::with_capacity(sample_capacity);
                for sample in self.inner.samples::<i32>() {
                    interleaved_pcm32.push(sample.map_err(|error| malformed(&error))?);
                }
                pcm32_to_f32(&interleaved_pcm32, &mut interleaved_f32)?;
            }
            WavSampleFormat::Float32 => {
                let mut interleaved_float32 = Vec::with_capacity(sample_capacity);
                for sample in self.inner.samples::<f32>() {
                    interleaved_float32.push(sample.map_err(|error| malformed(&error))?);
                }
                float32_to_f32(
                    &interleaved_float32,
                    &mut interleaved_f32,
                    channels.as_usize(),
                )?;
            }
            _ => unreachable!("unsupported WAV sample format already rejected"),
        }

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
        Ok(Self {
            inner: AnyPcmWavReader::new(reader)?,
        })
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
        let hound_spec = self.inner.inner.spec();
        ensure_pcm16(hound_spec)?;
        self.inner
            .read_expected_format_with_backend(WavSampleFormat::Pcm16, requested_backend)
    }
}

impl<R> AudioReader for AnyPcmWavReader<R>
where
    R: Read,
{
    fn codec_kind(&self) -> CodecKind {
        CodecKind::Wav
    }

    fn read_audio(&mut self) -> auralis_codec::Result<AudioBuffer> {
        self.read_wav().map_err(CodecError::from)
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
