//! WAV codec support for Auralis.
//!
//! This crate currently implements deterministic PCM16 WAV decoding into
//! Auralis' internal planar `f32` [`auralis_core::AudioBuffer`]. Encoding is
//! intentionally left to a later milestone.
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
    io::{BufReader, Read},
    path::Path,
};

use auralis_codec::{AudioReader, CodecError, CodecKind};
use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use thiserror::Error;

/// Crate-local result type using [`WavError`].
pub type Result<T> = std::result::Result<T, WavError>;

/// Errors produced while decoding WAV input.
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
    Pcm16WavReader::new(reader)?.read_pcm16()
}

/// Decodes a PCM16 WAV file from disk into a planar `f32` buffer.
///
/// # Errors
///
/// Returns [`WavError::OpenFailed`] if `path` cannot be opened. Propagates the
/// same parsing and format errors as [`decode_pcm16`].
pub fn decode_pcm16_path(path: impl AsRef<Path>) -> Result<AudioBuffer> {
    let file = File::open(path).map_err(|error| WavError::OpenFailed {
        message: error.to_string(),
    })?;

    decode_pcm16(BufReader::new(file))
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
        let mut planar = vec![
            0.0;
            frames
                .checked_mul(channels.as_usize())
                .ok_or(WavError::InvalidBufferShape)?
        ];

        for (sample_index, sample) in self.inner.samples::<i16>().enumerate() {
            let sample = sample.map_err(|error| malformed(&error))?;
            let frame_index = sample_index / channels.as_usize();
            let channel_index = sample_index % channels.as_usize();
            let planar_index = channel_index
                .checked_mul(frames)
                .and_then(|start| start.checked_add(frame_index))
                .ok_or(WavError::InvalidBufferShape)?;
            let destination = planar
                .get_mut(planar_index)
                .ok_or(WavError::InvalidBufferShape)?;

            *destination = f32::from(sample) / 32768.0;
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

    use auralis_codec::{AudioReader, CodecKind};

    use super::{Pcm16WavReader, WavError, WavSampleEncoding, decode_pcm16, decode_pcm16_path};

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
    fn mono_decode_matches_sox_ng_golden_raw_float_reference() {
        compare_sox_ng(1, &[-32768, -1234, 0, 12_345, 32_767]);
    }

    #[test]
    fn stereo_decode_matches_sox_ng_golden_raw_float_reference() {
        compare_sox_ng(2, &[-32768, 32_767, -12_000, 12_000, 0, 4096]);
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
}
