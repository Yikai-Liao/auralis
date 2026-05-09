//! High-level Auralis library API.
//!
//! This crate provides a small facade over the lower-level codec and effect
//! crates. It is intended for applications that want to open supported audio
//! files, build a typed processing chain, and write the result without wiring
//! the codec and effect crates manually.
//!
//! # Examples
//!
//! ```no_run
//! use auralis::AudioFile;
//!
//! AudioFile::open_wav("input.wav")?
//!     .into_pipeline()
//!     .gain_db(-3.0)
//!     .write_wav("output.wav")?;
//!
//! # Ok::<(), auralis::Error>(())
//! ```

use std::path::Path;

pub use auralis_core::{AudioBuffer, Decibels};

use auralis_effects::Gain;
use thiserror::Error;

/// Crate-local result type using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by the high-level Auralis facade.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A shared core value constructor rejected an input.
    #[error(transparent)]
    Core(#[from] auralis_core::AuralisError),

    /// WAV decoding or encoding failed.
    #[error(transparent)]
    Wav(#[from] auralis_wav::WavError),
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Core(left), Self::Core(right)) => left == right,
            (Self::Wav(left), Self::Wav(right)) => left == right,
            _ => false,
        }
    }
}

/// Decoded audio file ready to enter an effect pipeline.
///
/// `AudioFile` currently supports PCM16 WAV input only. Decoding always uses
/// Auralis' internal planar `f32` [`AudioBuffer`] representation.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioFile {
    audio: AudioBuffer,
}

impl AudioFile {
    /// Opens a PCM16 WAV file and decodes it into planar `f32` samples.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] when the path cannot be opened, the input is not
    /// a well-formed WAV stream, or the sample format is not supported.
    pub fn open_wav(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            audio: auralis_wav::decode_pcm16_path(path)?,
        })
    }

    /// Wraps an existing audio buffer in the high-level file type.
    ///
    /// This is primarily useful for tests and applications that decoded audio
    /// through a lower-level crate but still want to use the pipeline builder.
    #[must_use]
    pub const fn from_audio_buffer(audio: AudioBuffer) -> Self {
        Self { audio }
    }

    /// Returns the decoded audio buffer.
    #[must_use]
    pub const fn audio_buffer(&self) -> &AudioBuffer {
        &self.audio
    }

    /// Converts the file into a chainable effect pipeline.
    #[must_use]
    pub fn into_pipeline(self) -> Pipeline {
        Pipeline::from_audio_buffer(self.audio)
    }
}

/// Chainable in-memory audio processing pipeline.
///
/// Effects mutate the internal planar `f32` buffer in order. Methods that take
/// unvalidated user values, such as [`Self::gain_db`], keep the fluent chain
/// shape and defer any validation error until [`Self::write_wav`] or
/// [`Self::into_audio_buffer`] is called. This preserves error propagation
/// without panicking or requiring a `?` after every effect.
#[derive(Debug)]
pub struct Pipeline {
    audio: Result<AudioBuffer>,
}

impl Pipeline {
    /// Creates a pipeline from an in-memory audio buffer.
    #[must_use]
    pub fn from_audio_buffer(audio: AudioBuffer) -> Self {
        Self { audio: Ok(audio) }
    }

    /// Applies constant gain measured in decibels.
    ///
    /// The gain multiplier is `10^(db / 20)`. Processing is deterministic,
    /// in-place, non-allocating, and uses the scalar reference `Gain` effect.
    /// Output is not clipped until a boundary writer, such as PCM16 WAV
    /// encoding, applies its documented conversion rules.
    #[must_use]
    pub fn gain_db(mut self, db: f64) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        match Decibels::new(db) {
            Ok(db) => Gain::new(db).process_buffer(audio),
            Err(error) => self.audio = Err(error.into()),
        }

        self
    }

    /// Returns the processed audio buffer.
    ///
    /// # Errors
    ///
    /// Returns the first deferred configuration or processing error from the
    /// chain.
    pub fn into_audio_buffer(self) -> Result<AudioBuffer> {
        self.audio
    }

    /// Encodes the processed audio as a PCM16 WAV file.
    ///
    /// Existing files at `path` are overwritten. Encoding clips finite samples
    /// to the PCM16 range as documented by [`auralis_wav::encode_pcm16_path`].
    ///
    /// # Errors
    ///
    /// Returns the first deferred configuration error from the chain, or a WAV
    /// write error if output creation, sample validation, sample writing, or
    /// finalization fails.
    pub fn write_wav(self, path: impl AsRef<Path>) -> Result<()> {
        let audio = self.audio?;
        auralis_wav::encode_pcm16_path(path, &audio)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioFile, Error};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
    };
    use auralis_effects::Gain;

    #[test]
    fn chain_gain_matches_direct_effect_execution() {
        let source = audio_buffer(vec![0.25, -0.5, 1.0]);
        let mut expected = source.clone();

        Gain::new(Decibels::new(-3.0).unwrap()).process_buffer(&mut expected);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .gain_db(-3.0)
            .into_audio_buffer()
            .unwrap();

        assert_samples_close(actual.as_planar_f32(), expected.as_planar_f32());
    }

    #[test]
    fn invalid_gain_propagates_without_panic() {
        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
            .into_pipeline()
            .gain_db(f64::NAN)
            .into_audio_buffer()
            .unwrap_err();

        assert_eq!(
            error,
            Error::Core(auralis_core::AuralisError::InvalidDecibels)
        );
    }

    #[test]
    fn wav_chain_round_trips_through_file_boundary() {
        let tempdir = temp_dir();
        fs::create_dir(&tempdir).unwrap();
        let input = tempdir.join("input.wav");
        let output = tempdir.join("output.wav");
        let source = audio_buffer(vec![0.25, -0.5, 0.75]);

        auralis_wav::encode_pcm16_path(&input, &source).unwrap();

        AudioFile::open_wav(&input)
            .unwrap()
            .into_pipeline()
            .gain_db(0.0)
            .write_wav(&output)
            .unwrap();

        let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
        assert_samples_close(decoded.as_planar_f32(), source.as_planar_f32());
        fs::remove_dir_all(tempdir).unwrap();
    }

    #[test]
    fn deferred_error_prevents_later_processing() {
        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
            .into_pipeline()
            .gain_db(f64::INFINITY)
            .gain_db(6.0)
            .into_audio_buffer()
            .unwrap_err();

        assert_eq!(
            error,
            Error::Core(auralis_core::AuralisError::InvalidDecibels)
        );
    }

    fn audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        let frames = samples.len().try_into().unwrap();
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        );

        AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());

        for (actual, expected) in actual.iter().zip(expected) {
            let tolerance = 1.0e-4;
            let difference = (actual - expected).abs();

            assert!(
                difference <= tolerance,
                "expected {actual} to be within {tolerance} of {expected}, difference was {difference}"
            );
        }
    }

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        std::env::temp_dir().join(format!("auralis-chain-api-{nanos}"))
    }
}
