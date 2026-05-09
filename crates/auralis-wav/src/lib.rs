//! WAV codec support for Auralis.
//!
//! This crate currently implements deterministic PCM16 WAV decoding and
//! encoding between RIFF/WAVE files and Auralis' internal planar `f32`
//! [`auralis_core::AudioBuffer`].
//!
//! # Examples
//!
//! ```
//! use std::io::Cursor;
//!
//! use auralis_wav::decode_pcm16;
//!
//! let mut wav_bytes = Vec::new();
//! wav_bytes.extend_from_slice(b"RIFF");
//! wav_bytes.extend_from_slice(&38_u32.to_le_bytes());
//! wav_bytes.extend_from_slice(b"WAVEfmt ");
//! wav_bytes.extend_from_slice(&16_u32.to_le_bytes());
//! wav_bytes.extend_from_slice(&1_u16.to_le_bytes());
//! wav_bytes.extend_from_slice(&1_u16.to_le_bytes());
//! wav_bytes.extend_from_slice(&48_000_u32.to_le_bytes());
//! wav_bytes.extend_from_slice(&96_000_u32.to_le_bytes());
//! wav_bytes.extend_from_slice(&2_u16.to_le_bytes());
//! wav_bytes.extend_from_slice(&16_u16.to_le_bytes());
//! wav_bytes.extend_from_slice(b"data");
//! wav_bytes.extend_from_slice(&2_u32.to_le_bytes());
//! wav_bytes.extend_from_slice(&0_i16.to_le_bytes());
//!
//! let audio = decode_pcm16(Cursor::new(wav_bytes))?;
//! assert_eq!(audio.frames().as_u64(), 1);
//! assert_eq!(audio.sample(0, 0), Some(0.0));
//! # Ok::<(), auralis_wav::WavError>(())
//! ```

use std::{
    fmt,
    fs::File,
    io::{BufReader, Read, Seek, Write},
    path::Path,
};

use auralis_codec::{AudioReader, AudioWriter, CodecError, CodecKind};
use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_simd::{
    BackendKind, SampleConversionError, f32_to_i16_with_backend, i16_to_f32_with_backend,
    select_backend,
};
use thiserror::Error;

/// Crate-local result type using [`WavError`].
pub type Result<T> = std::result::Result<T, WavError>;

/// Errors produced while decoding or encoding WAV input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum WavError {
    /// The WAV sample format is not the PCM16 format implemented by this
    /// milestone.
    #[error("unsupported WAV sample format: {encoding} with {bits_per_sample} bits per sample")]
    UnsupportedSampleFormat {
        /// The integer bit depth declared by the WAV stream.
        bits_per_sample: u16,
        /// The sample encoding declared by the WAV stream.
        encoding: WavSampleEncoding,
    },

    /// The WAV header declared a zero sample rate.
    #[error("WAV sample rate must be greater than zero")]
    InvalidSampleRate,

    /// The WAV header declared a zero channel count.
    #[error("WAV channel count must be greater than zero")]
    InvalidChannelCount,

    /// The decoded stream shape cannot be represented by Auralis' buffer
    /// model.
    #[error("decoded WAV data does not match a valid audio buffer shape")]
    InvalidBufferShape,

    /// The input could not be parsed as a well-formed WAV stream.
    #[error("malformed WAV input: {message}")]
    Malformed {
        /// Human-readable parser failure detail.
        message: String,
    },

    /// The input path could not be opened.
    #[error("could not open WAV input: {message}")]
    OpenFailed {
        /// Human-readable I/O failure detail.
        message: String,
    },

    /// The output path could not be created.
    #[error("could not create WAV output: {message}")]
    CreateFailed {
        /// Human-readable I/O failure detail.
        message: String,
    },

    /// The output stream already consumed its writer.
    #[error("WAV writer has already written an output stream")]
    WriterAlreadyUsed,

    /// The input buffer contained a non-finite sample value.
    #[error("WAV sample at channel {channel_index}, frame {frame_index} must be finite")]
    NonFiniteSample {
        /// Zero-based channel index of the invalid sample.
        channel_index: usize,
        /// Zero-based frame index of the invalid sample.
        frame_index: usize,
    },

    /// The WAV output stream could not be written or finalized.
    #[error("could not write WAV output: {message}")]
    WriteFailed {
        /// Human-readable writer failure detail.
        message: String,
    },
}

/// WAV sample encoding category used in unsupported-format errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavSampleEncoding {
    /// Integer PCM samples.
    Integer,
    /// IEEE floating-point samples.
    Float,
}

impl fmt::Display for WavSampleEncoding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer => formatter.write_str("integer PCM"),
            Self::Float => formatter.write_str("IEEE float"),
        }
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
    let writer = hound::WavWriter::create(path, hound_spec(audio)).map_err(|error| {
        WavError::CreateFailed {
            message: error.to_string(),
        }
    })?;

    write_pcm16_samples_with_backend(writer, audio, requested_backend)
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
        i16_to_f32_with_backend(
            select_backend(requested_backend),
            &interleaved_pcm16,
            &mut interleaved_f32,
        )
        .map_err(|_| WavError::InvalidBufferShape)?;

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
        let writer = hound::WavWriter::new(destination, hound_spec(audio)).map_err(|error| {
            WavError::WriteFailed {
                message: error.to_string(),
            }
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

impl From<WavError> for CodecError {
    fn from(error: WavError) -> Self {
        Self::DecodeFailed {
            kind: CodecKind::Wav,
            message: error.to_string(),
        }
    }
}

fn ensure_pcm16(spec: hound::WavSpec) -> Result<()> {
    if spec.sample_format == hound::SampleFormat::Int && spec.bits_per_sample == 16 {
        Ok(())
    } else {
        Err(WavError::UnsupportedSampleFormat {
            bits_per_sample: spec.bits_per_sample,
            encoding: match spec.sample_format {
                hound::SampleFormat::Int => WavSampleEncoding::Integer,
                hound::SampleFormat::Float => WavSampleEncoding::Float,
            },
        })
    }
}

fn hound_spec(audio: &AudioBuffer) -> hound::WavSpec {
    hound::WavSpec {
        channels: audio.channels().as_u16(),
        sample_rate: audio.spec().sample_rate().as_u32(),
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    }
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

    let mut interleaved_pcm16 = vec![0; interleaved_f32.len()];
    f32_to_i16_with_backend(
        select_backend(requested_backend),
        &interleaved_f32,
        &mut interleaved_pcm16,
    )
    .map_err(|error| sample_conversion_error(&error, channels))?;

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

fn sample_conversion_error(error: &SampleConversionError, channels: usize) -> WavError {
    match error {
        SampleConversionError::NonFiniteSample { sample_index } => WavError::NonFiniteSample {
            channel_index: *sample_index % channels,
            frame_index: *sample_index / channels,
        },
        _ => WavError::WriteFailed {
            message: error.to_string(),
        },
    }
}

fn frame_count(frames_per_channel: u32) -> FrameCount {
    FrameCount::new(u64::from(frames_per_channel))
}

fn malformed(error: &hound::Error) -> WavError {
    WavError::Malformed {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        io::Cursor,
        path::{Path, PathBuf},
        process::Command,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use auralis_codec::{AudioReader, AudioWriter, CodecError, CodecKind};
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };
    use auralis_simd::BackendKind;

    use super::{
        Pcm16WavReader, Pcm16WavWriter, WavError, WavSampleEncoding, decode_pcm16,
        decode_pcm16_path, decode_pcm16_with_backend, encode_pcm16_path,
        encode_pcm16_path_with_backend,
    };

    #[test]
    fn decodes_mono_pcm16_to_planar_f32() {
        let audio = decode_pcm16(Cursor::new(wav_bytes(1, &[-32768, 0, 16_384, 32_767]))).unwrap();

        assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
        assert_eq!(audio.spec().channels().as_u16(), 1);
        assert_eq!(audio.frames().as_u64(), 4);
        assert_eq!(
            audio.channel(0).unwrap(),
            &[-1.0, 0.0, 0.5, f32::from(32_767_i16) / 32768.0]
        );
    }

    #[test]
    fn decodes_stereo_pcm16_to_channel_major_storage() {
        let audio = decode_pcm16(Cursor::new(wav_bytes(
            2,
            &[-32768, 32_767, -16_384, 16_384, 0, 8192],
        )))
        .unwrap();

        assert_eq!(audio.spec().channels().as_u16(), 2);
        assert_eq!(audio.frames().as_u64(), 3);
        assert_eq!(audio.channel(0).unwrap(), &[-1.0, -0.5, 0.0]);
        assert_eq!(
            audio.channel(1).unwrap(),
            &[f32::from(32_767_i16) / 32768.0, 0.5, 0.25]
        );
    }

    #[test]
    fn decode_pcm16_matches_under_forced_scalar_and_requested_simd() {
        let bytes = wav_bytes(
            2,
            &[
                i16::MIN,
                i16::MAX,
                -16_384,
                16_384,
                -1,
                1,
                0,
                8192,
                -8192,
                1234,
            ],
        );

        let scalar =
            decode_pcm16_with_backend(Cursor::new(bytes.clone()), BackendKind::Scalar).unwrap();
        let simd = decode_pcm16_with_backend(Cursor::new(bytes), BackendKind::Simd).unwrap();

        assert_audio_bits_eq(&simd, &scalar);
    }

    #[test]
    fn rejects_unsupported_bit_depth_with_typed_error() {
        let error = decode_pcm16(Cursor::new(wav_bytes_with_bits(1, 24, &[0, 0, 0]))).unwrap_err();

        assert_eq!(
            error,
            WavError::UnsupportedSampleFormat {
                bits_per_sample: 24,
                encoding: WavSampleEncoding::Integer,
            }
        );
    }

    #[test]
    fn rejects_float_wav_with_typed_error() {
        let mut bytes = riff_header(1, 32, 3, 4);
        bytes.extend_from_slice(&0.0_f32.to_le_bytes());
        let error = decode_pcm16(Cursor::new(bytes)).unwrap_err();

        assert_eq!(
            error,
            WavError::UnsupportedSampleFormat {
                bits_per_sample: 32,
                encoding: WavSampleEncoding::Float,
            }
        );
    }

    #[test]
    fn malformed_wav_returns_typed_error() {
        let error = decode_pcm16(Cursor::new(b"not a wav".to_vec())).unwrap_err();

        assert!(matches!(error, WavError::Malformed { .. }));
    }

    #[test]
    fn path_decoder_preserves_sample_rate_and_channels() {
        let path = write_temp_wav("auralis-wav-path", 2, &[-1024, 1024]).unwrap();
        let audio = decode_pcm16_path(&path).unwrap();

        fs::remove_file(path).unwrap();
        assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
        assert_eq!(audio.spec().channels().as_u16(), 2);
    }

    #[test]
    fn implements_codec_reader_boundary() {
        let mut reader = Pcm16WavReader::new(Cursor::new(wav_bytes(1, &[0]))).unwrap();
        let audio = reader.read_audio().unwrap();

        assert_eq!(reader.codec_kind(), CodecKind::Wav);
        assert_eq!(audio.sample(0, 0), Some(0.0));
    }

    #[test]
    fn encodes_mono_pcm16_from_planar_f32() {
        let audio = audio_buffer(1, 4, &[-1.0, 0.0, 0.5, f32::from(32_767_i16) / 32768.0]);
        let path = encode_temp_wav("auralis-wav-encode-mono", &audio);
        let decoded = decode_pcm16_path(&path).unwrap();

        fs::remove_file(path).unwrap();
        assert_eq!(decoded.spec().sample_rate().as_u32(), 48_000);
        assert_eq!(decoded.spec().channels().as_u16(), 1);
        assert_eq!(decoded.frames().as_u64(), 4);
        assert_eq!(
            decoded.channel(0).unwrap(),
            &[-1.0, 0.0, 0.5, f32::from(32_767_i16) / 32768.0]
        );
    }

    #[test]
    fn encodes_stereo_pcm16_by_interleaving_frames() {
        let audio = audio_buffer(2, 3, &[-1.0, -0.5, 0.0, 0.999_969_5, 0.5, 0.25]);
        let path = encode_temp_wav("auralis-wav-encode-stereo", &audio);
        let decoded = decode_pcm16_path(&path).unwrap();

        fs::remove_file(path).unwrap();
        assert_eq!(decoded.spec().channels().as_u16(), 2);
        assert_eq!(decoded.frames().as_u64(), 3);
        assert_eq!(decoded.channel(0).unwrap(), &[-1.0, -0.5, 0.0]);
        assert_eq!(
            decoded.channel(1).unwrap(),
            &[f32::from(32_767_i16) / 32768.0, 0.5, 0.25]
        );
    }

    #[test]
    fn encode_clips_samples_to_pcm16_range() {
        let audio = audio_buffer(1, 4, &[-2.0, -1.0, 1.0, 2.0]);
        let path = encode_temp_wav("auralis-wav-encode-clip", &audio);
        let decoded = decode_pcm16_path(&path).unwrap();

        fs::remove_file(path).unwrap();
        assert_eq!(
            decoded.channel(0).unwrap(),
            &[
                -1.0,
                -1.0,
                f32::from(32_767_i16) / 32768.0,
                f32::from(32_767_i16) / 32768.0,
            ]
        );
    }

    #[test]
    fn encode_pcm16_matches_under_forced_scalar_and_requested_simd() {
        let audio = audio_buffer(
            2,
            5,
            &[
                -1.5,
                -1.0,
                -0.5,
                -0.5 / 32768.0,
                0.0,
                0.5 / 32768.0,
                0.25,
                0.999_984_74,
                1.0,
                1.5,
            ],
        );
        let scalar_path =
            encode_temp_wav_with_backend("auralis-wav-encode-scalar", &audio, BackendKind::Scalar);
        let simd_path =
            encode_temp_wav_with_backend("auralis-wav-encode-simd", &audio, BackendKind::Simd);
        let scalar = decode_pcm16_path(&scalar_path).unwrap();
        let simd = decode_pcm16_path(&simd_path).unwrap();

        fs::remove_file(scalar_path).unwrap();
        fs::remove_file(simd_path).unwrap();
        assert_audio_bits_eq(&simd, &scalar);
    }

    #[test]
    fn encode_rejects_non_finite_samples() {
        let audio = audio_buffer(2, 2, &[0.0, 0.25, 0.5, f32::NAN]);
        let path = temp_path("auralis-wav-encode-nan", "wav");
        let error = encode_pcm16_path(&path, &audio).unwrap_err();

        let _ = fs::remove_file(path);
        assert_eq!(
            error,
            WavError::NonFiniteSample {
                channel_index: 1,
                frame_index: 1,
            }
        );
    }

    #[test]
    fn round_trips_generated_pcm16_signal() {
        let input = audio_buffer(2, 4, &[-0.75, -0.25, 0.25, 0.75, 0.0, 0.125, -0.125, 0.5]);
        let first_path = encode_temp_wav("auralis-wav-roundtrip-first", &input);
        let decoded = decode_pcm16_path(&first_path).unwrap();
        let second_path = encode_temp_wav("auralis-wav-roundtrip-second", &decoded);
        let round_tripped = decode_pcm16_path(&second_path).unwrap();

        fs::remove_file(first_path).unwrap();
        fs::remove_file(second_path).unwrap();
        assert_eq!(round_tripped, decoded);
    }

    #[test]
    fn implements_codec_writer_boundary() {
        let audio = audio_buffer(1, 1, &[0.0]);
        let mut writer = Pcm16WavWriter::new(Cursor::new(Vec::new()));

        writer.write_audio(&audio).unwrap();
        assert_eq!(writer.codec_kind(), CodecKind::Wav);
        assert!(matches!(
            writer.write_audio(&audio),
            Err(CodecError::EncodeFailed {
                kind: CodecKind::Wav,
                ..
            })
        ));
    }

    #[test]
    fn mono_decode_matches_sox_ng_golden_raw_float_reference() {
        compare_sox_ng(1, &[-32768, -1234, 0, 12_345, 32_767]);
    }

    #[test]
    fn stereo_decode_matches_sox_ng_golden_raw_float_reference() {
        compare_sox_ng(2, &[-32768, 32_767, -12_000, 12_000, 0, 4096]);
    }

    #[test]
    fn encoded_stereo_samples_match_sox_ng_pcm16_reference() {
        let channels = 2;
        let audio = audio_buffer(2, 3, &[-0.75, 0.0, 0.75, -0.5, 0.5, 0.999_969_5]);
        let auralis_output = encode_temp_wav("auralis-wav-sox-encode-auralis", &audio);
        let raw_input = temp_path("auralis-wav-sox-encode-input", "f32");
        let sox_output = temp_path("auralis-wav-sox-encode-output", "wav");
        write_le_f32(
            &raw_input,
            &interleave_planar(audio.as_planar_f32(), channels),
        )
        .unwrap();

        let sox_ng = env::var("AURALIS_SOX_NG_BIN").unwrap_or_else(|_| "sox_ng".to_owned());
        let status = Command::new(sox_ng)
            .args([
                "-R",
                "-D",
                "-t",
                "f32",
                "-r",
                "48000",
                "-c",
                "2",
                raw_input.to_str().unwrap(),
                "-b",
                "16",
                "-e",
                "signed-integer",
                sox_output.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(status.success());

        let auralis = decode_pcm16_path(&auralis_output).unwrap();
        let sox = decode_pcm16_path(&sox_output).unwrap();

        fs::remove_file(auralis_output).unwrap();
        fs::remove_file(raw_input).unwrap();
        fs::remove_file(sox_output).unwrap();

        assert_close_by_one_lsb(auralis.as_planar_f32(), sox.as_planar_f32());
    }

    fn compare_sox_ng(channels: u16, samples: &[i16]) {
        let input = write_temp_wav("auralis-wav-sox-input", channels, samples).unwrap();
        let output = temp_path("auralis-wav-sox-output", "f32");
        let sox_ng = env::var("AURALIS_SOX_NG_BIN").unwrap_or_else(|_| "sox_ng".to_owned());
        let status = Command::new(sox_ng)
            .args([
                "-R",
                "-D",
                input.to_str().unwrap(),
                "-t",
                "f32",
                output.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(status.success());

        let audio = decode_pcm16_path(&input).unwrap();
        let sox_samples = read_le_f32(&output).unwrap();

        fs::remove_file(input).unwrap();
        fs::remove_file(output).unwrap();

        assert_eq!(
            interleave_planar(audio.as_planar_f32(), channels),
            sox_samples
        );
    }

    fn wav_bytes(channels: u16, samples: &[i16]) -> Vec<u8> {
        let mut bytes = riff_header(channels, 16, 1, u32::try_from(samples.len() * 2).unwrap());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }

    fn wav_bytes_with_bits(channels: u16, bits_per_sample: u16, payload: &[u8]) -> Vec<u8> {
        let mut bytes = riff_header(
            channels,
            bits_per_sample,
            1,
            u32::try_from(payload.len()).unwrap(),
        );
        bytes.extend_from_slice(payload);
        bytes
    }

    fn riff_header(
        channels: u16,
        bits_per_sample: u16,
        format_tag: u16,
        data_bytes: u32,
    ) -> Vec<u8> {
        let bytes_per_sample = u32::from(bits_per_sample) / 8;
        let byte_rate = 48_000 * u32::from(channels) * bytes_per_sample;
        let block_align = channels * (bits_per_sample / 8);
        let riff_size = 36 + data_bytes;
        let mut bytes = Vec::new();

        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&riff_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&format_tag.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&48_000_u32.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_bytes.to_le_bytes());
        bytes
    }

    fn write_temp_wav(prefix: &str, channels: u16, samples: &[i16]) -> std::io::Result<PathBuf> {
        let path = temp_path(prefix, "wav");
        fs::write(&path, wav_bytes(channels, samples))?;
        Ok(path)
    }

    fn temp_path(prefix: &str, extension: &str) -> PathBuf {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        env::temp_dir().join(format!("{prefix}-{}-{id}.{extension}", std::process::id()))
    }

    fn read_le_f32(path: &Path) -> std::io::Result<Vec<f32>> {
        let bytes = fs::read(path)?;
        Ok(bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
            .collect())
    }

    fn write_le_f32(path: &Path, samples: &[f32]) -> std::io::Result<()> {
        let mut bytes = Vec::with_capacity(samples.len() * 4);
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        fs::write(path, bytes)
    }

    fn interleave_planar(planar: &[f32], channels: u16) -> Vec<f32> {
        let channel_count = usize::from(channels);
        let frames = planar.len() / channel_count;
        let mut interleaved = Vec::with_capacity(planar.len());

        for frame in 0..frames {
            for channel in 0..channel_count {
                interleaved.push(planar[channel * frames + frame]);
            }
        }

        interleaved
    }

    fn audio_buffer(channels: u16, frames: u64, planar: &[f32]) -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(channels).unwrap(),
            SampleFormat::Float32,
        );

        AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), planar.to_vec()).unwrap()
    }

    fn encode_temp_wav(prefix: &str, audio: &AudioBuffer) -> PathBuf {
        let path = temp_path(prefix, "wav");
        encode_pcm16_path(&path, audio).unwrap();
        path
    }

    fn encode_temp_wav_with_backend(
        prefix: &str,
        audio: &AudioBuffer,
        backend: BackendKind,
    ) -> PathBuf {
        let path = temp_path(prefix, "wav");
        encode_pcm16_path_with_backend(&path, audio, backend).unwrap();
        path
    }

    fn assert_audio_bits_eq(actual: &AudioBuffer, expected: &AudioBuffer) {
        assert_eq!(actual.spec(), expected.spec());
        assert_eq!(actual.frames(), expected.frames());

        for channel_index in 0..actual.channels().as_usize() {
            let actual = actual.channel(channel_index).unwrap();
            let expected = expected.channel(channel_index).unwrap();
            assert_eq!(actual.len(), expected.len());

            for (frame_index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
                assert_eq!(
                    actual.to_bits(),
                    expected.to_bits(),
                    "channel {channel_index} frame {frame_index} differed: {actual} != {expected}"
                );
            }
        }
    }

    fn assert_close_by_one_lsb(left: &[f32], right: &[f32]) {
        assert_eq!(left.len(), right.len());
        for (index, (left, right)) in left.iter().zip(right).enumerate() {
            let delta = (left - right).abs();
            assert!(
                delta <= 1.0 / 32768.0,
                "sample {index} differed by {delta}: {left} != {right}"
            );
        }
    }
}
