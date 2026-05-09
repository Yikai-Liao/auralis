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
//!     .dc_shift(0.125)
//!     .fade_frames(1_000, 2_000)
//!     .reverse()
//!     .pad_frames(0, 48_000)
//!     .write_wav("output.wav")?;
//!
//! # Ok::<(), auralis::Error>(())
//! ```

use std::path::Path;

pub use auralis_core::{AudioBuffer, Decibels, FrameCount, TimeSeconds};
pub use auralis_effects::{EffectChain, EffectChainError, EffectCommand};
pub use auralis_simd::BackendKind;

use auralis_effects::{DcShift, Fade, Gain, Pad, Reverse, Trim};
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

    /// A typed effect processor rejected its configuration or input buffer.
    #[error(transparent)]
    Effect(#[from] auralis_effects::EffectError),

    /// A sequential effect chain failed while applying one command.
    #[error(transparent)]
    Chain(#[from] auralis_effects::EffectChainError),

    /// A seconds-based trim range could not be represented as frames.
    #[error("trim seconds range cannot be represented as frame positions")]
    InvalidTrimSecondsRange,
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Core(left), Self::Core(right)) => left == right,
            (Self::Wav(left), Self::Wav(right)) => left == right,
            (Self::Effect(left), Self::Effect(right)) => left == right,
            (Self::Chain(left), Self::Chain(right)) => left == right,
            (Self::InvalidTrimSecondsRange, Self::InvalidTrimSecondsRange) => true,
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
    requested_backend: BackendKind,
}

impl AudioFile {
    /// Opens a PCM16 WAV file and decodes it into planar `f32` samples.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] when the path cannot be opened, the input is not
    /// a well-formed WAV stream, or the sample format is not supported.
    pub fn open_wav(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_wav_with_backend(path, BackendKind::Scalar)
    }

    /// Opens a PCM16 WAV file using the requested sample-conversion backend.
    ///
    /// The decoded audio is identical to [`Self::open_wav`]. `requested_backend`
    /// controls only backend-aware decode, later backend-aware effect kernels,
    /// and WAV encoding after [`Self::into_pipeline`]. Unsupported SIMD requests
    /// follow Auralis' documented scalar fallback.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] when the path cannot be opened, the input is not
    /// a well-formed WAV stream, or the sample format is not supported.
    pub fn open_wav_with_backend(
        path: impl AsRef<Path>,
        requested_backend: BackendKind,
    ) -> Result<Self> {
        Ok(Self {
            audio: auralis_wav::decode_pcm16_path_with_backend(path, requested_backend)?,
            requested_backend,
        })
    }

    /// Wraps an existing audio buffer in the high-level file type.
    ///
    /// This is primarily useful for tests and applications that decoded audio
    /// through a lower-level crate but still want to use the pipeline builder.
    #[must_use]
    pub const fn from_audio_buffer(audio: AudioBuffer) -> Self {
        Self {
            audio,
            requested_backend: BackendKind::Scalar,
        }
    }

    /// Returns the decoded audio buffer.
    #[must_use]
    pub const fn audio_buffer(&self) -> &AudioBuffer {
        &self.audio
    }

    /// Returns the backend requested for backend-aware processing.
    #[must_use]
    pub const fn requested_backend(&self) -> BackendKind {
        self.requested_backend
    }

    /// Converts the file into a chainable effect pipeline.
    #[must_use]
    pub fn into_pipeline(self) -> Pipeline {
        Pipeline::from_audio_buffer_with_backend(self.audio, self.requested_backend)
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
    requested_backend: BackendKind,
}

impl Pipeline {
    /// Creates a pipeline from an in-memory audio buffer.
    #[must_use]
    pub fn from_audio_buffer(audio: AudioBuffer) -> Self {
        Self::from_audio_buffer_with_backend(audio, BackendKind::Scalar)
    }

    /// Creates a pipeline from an in-memory audio buffer with a requested backend.
    ///
    /// The backend is used by backend-aware effect kernels and WAV boundary
    /// conversions. Current scalar-only effects keep their scalar behavior until
    /// their own SIMD retrofit features are implemented.
    #[must_use]
    pub fn from_audio_buffer_with_backend(
        audio: AudioBuffer,
        requested_backend: BackendKind,
    ) -> Self {
        Self {
            audio: Ok(audio),
            requested_backend,
        }
    }

    /// Sets the requested backend for later backend-aware pipeline stages.
    ///
    /// Requesting [`BackendKind::Scalar`] forces the scalar reference path.
    /// Requesting [`BackendKind::Simd`] uses SIMD where supported and falls back
    /// to scalar through Auralis backend selection metadata otherwise.
    #[must_use]
    pub const fn with_backend(mut self, requested_backend: BackendKind) -> Self {
        self.requested_backend = requested_backend;
        self
    }

    /// Applies constant gain measured in decibels.
    ///
    /// The gain multiplier is `10^(db / 20)`. Processing is deterministic,
    /// in-place, non-allocating, and uses the requested backend for the
    /// backend-aware `Gain` effect. Scalar remains the default. Output is not
    /// clipped until a boundary writer, such as PCM16 WAV encoding, applies its
    /// documented conversion rules.
    #[must_use]
    pub fn gain_db(mut self, db: f64) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        match Decibels::new(db) {
            Ok(db) => Gain::new(db).process_buffer_with_backend(audio, self.requested_backend),
            Err(error) => self.audio = Err(error.into()),
        }

        self
    }

    /// Applies a constant normalized DC offset.
    ///
    /// `shift` is measured in full-scale sample units, where `0.25` adds one
    /// quarter of full scale and `0.0` is identity. The valid range is
    /// `-2.0..=2.0`, matching SoX-ng's single-argument `dcshift` command.
    /// Processing is deterministic, in-place, non-allocating, and uses the
    /// requested backend for the backend-aware `DcShift` effect. Scalar remains
    /// the default. The effect does not clip; samples outside `[-1.0, 1.0]`
    /// are clipped by boundary writers such as PCM16 WAV encoding.
    #[must_use]
    pub fn dc_shift(mut self, shift: f32) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        match DcShift::new(shift) {
            Ok(dc_shift) => dc_shift.process_buffer_with_backend(audio, self.requested_backend),
            Err(error) => self.audio = Err(error.into()),
        }

        self
    }

    /// Keeps the half-open frame range `start..end` from every channel.
    ///
    /// Frame positions are end-exclusive and measured in audio frames, not
    /// individual samples. The transform preserves channel grouping and returns
    /// an empty buffer when `start == end`. Processing is deterministic and
    /// allocates a new planar buffer for the retained range.
    #[must_use]
    pub fn trim_frames(mut self, start: u64, end: u64) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        match Trim::new(FrameCount::new(start), FrameCount::new(end))
            .and_then(|trim| trim.process_buffer(audio))
        {
            Ok(trimmed) => *audio = trimmed,
            Err(error) => self.audio = Err(error.into()),
        }

        self
    }

    /// Keeps the half-open seconds range `start..end` from every channel.
    ///
    /// Seconds are non-negative finite values. A seconds position is converted
    /// to a frame position by flooring `seconds * sample_rate`; for example,
    /// `0.0000625` at `48_000 Hz` becomes frame `3`. This deterministic rule is
    /// documented so fractional-frame requests never depend on floating-point
    /// rounding mode or CLI formatting. The resulting frame range follows the
    /// same validation and end-exclusive behavior as [`Self::trim_frames`].
    #[must_use]
    pub fn trim_seconds(mut self, start: f64, end: f64) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        let result = TimeSeconds::new(start)
            .map_err(Error::from)
            .and_then(|start| {
                TimeSeconds::new(end)
                    .map_err(Error::from)
                    .map(|end| (start, end))
            })
            .and_then(|(start, end)| {
                let sample_rate = audio.spec().sample_rate().as_u32();
                seconds_to_frame(start, sample_rate)
                    .zip(seconds_to_frame(end, sample_rate))
                    .ok_or(Error::InvalidTrimSecondsRange)
            })
            .and_then(|(start, end)| {
                Trim::new(start, end)
                    .and_then(|trim| trim.process_buffer(audio))
                    .map_err(Error::from)
            });

        match result {
            Ok(trimmed) => *audio = trimmed,
            Err(error) => self.audio = Err(error),
        }

        self
    }

    /// Adds zero-valued frames before and after every channel.
    ///
    /// Padding lengths are measured in audio frames, not individual samples.
    /// The transform preserves channel grouping and returns the original audio
    /// unchanged when both lengths are zero. Processing is deterministic and
    /// allocates a new planar buffer for the padded output.
    #[must_use]
    pub fn pad_frames(mut self, start: u64, end: u64) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        match Pad::new(FrameCount::new(start), FrameCount::new(end)).process_buffer(audio) {
            Ok(padded) => *audio = padded,
            Err(error) => self.audio = Err(error.into()),
        }

        self
    }

    /// Applies linear fade-in and fade-out envelopes measured in frames.
    ///
    /// `fade_in` ramps the start from silence toward unity, and `fade_out`
    /// ramps the end from unity toward silence. A fade length of `4` uses
    /// coefficients `[0.0, 0.25, 0.5, 0.75]` for fade-in and
    /// `[0.75, 0.5, 0.25, 0.0]` for fade-out. If the fade regions overlap,
    /// their coefficients are multiplied. Passing `0` for either side leaves
    /// that side unchanged. Processing is deterministic, in-place,
    /// non-allocating, and uses the requested backend for the backend-aware
    /// `Fade` effect. Scalar remains the default.
    #[must_use]
    pub fn fade_frames(mut self, fade_in: u64, fade_out: u64) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        Fade::new(FrameCount::new(fade_in), FrameCount::new(fade_out))
            .process_buffer_with_backend(audio, self.requested_backend);

        self
    }

    /// Reverses the frame order within each channel.
    ///
    /// This transform is deterministic, in-place, and non-allocating. It is
    /// measured in frames rather than individual interleaved samples, so stereo
    /// and larger channel layouts keep their channel grouping. Applying
    /// `reverse` twice restores the original audio exactly.
    #[must_use]
    pub fn reverse(mut self) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        Reverse::new().process_buffer(audio);

        self
    }

    /// Applies a typed in-memory effect chain in its stored command order.
    ///
    /// This is the command-model counterpart to the fluent methods such as
    /// [`Self::gain_db`] and [`Self::reverse`]. Backend-aware effects inside
    /// the chain use the pipeline's requested backend; structural effects keep
    /// their deterministic scalar behavior. If one command fails, later
    /// commands are skipped and the deferred [`Error::Chain`] identifies the
    /// failing command index, canonical command tokens, argument family, and
    /// typed source error.
    #[must_use]
    pub fn apply_effect_chain(mut self, chain: &EffectChain) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        if let Err(error) = chain.process_buffer_with_backend(audio, self.requested_backend) {
            self.audio = Err(error.into());
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
        auralis_wav::encode_pcm16_path_with_backend(path, &audio, self.requested_backend)?;

        Ok(())
    }
}

fn seconds_to_frame(seconds: TimeSeconds, sample_rate: u32) -> Option<FrameCount> {
    let scaled = seconds.as_f64() * f64::from(sample_rate);
    #[allow(
        clippy::cast_precision_loss,
        reason = "This boundary check only needs the representable f64 magnitude of u64::MAX before the explicit saturating cast below."
    )]
    let max_frame = u64::MAX as f64;
    if !scaled.is_finite() || scaled > max_frame {
        return None;
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "TimeSeconds validates finite non-negative input; floor defines fractional-frame behavior before a checked range comparison."
    )]
    let frame = scaled.floor() as u64;

    Some(FrameCount::new(frame))
}

#[cfg(test)]
mod tests {
    use super::{AudioFile, BackendKind, EffectChain, EffectCommand, Error};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
    };
    use auralis_effects::{DcShift, Fade, Gain, Reverse, Trim};

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
    fn chain_gain_matches_under_forced_scalar_and_requested_simd() {
        let source = audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -0.5,
            -0.0,
            0.0,
            0.5,
            0.999_984_74,
            1.0,
        ]);

        let scalar = AudioFile::from_audio_buffer(source.clone())
            .into_pipeline()
            .with_backend(BackendKind::Scalar)
            .gain_db(-3.0)
            .into_audio_buffer()
            .unwrap();
        let simd = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .with_backend(BackendKind::Simd)
            .gain_db(-3.0)
            .into_audio_buffer()
            .unwrap();

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn chain_dc_shift_preserves_stereo_frame_grouping() {
        let source = stereo_audio_buffer(vec![0.75, 1.0, -0.75, -1.0]);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .dc_shift(0.5)
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.as_planar_f32(), &[1.25, 1.5, -0.25, -0.5]);
    }

    #[test]
    fn chain_dc_shift_matches_under_forced_scalar_and_requested_simd() {
        let source = audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -f32::MIN_POSITIVE,
            -f32::from_bits(1),
            -0.0,
            0.0,
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            0.999_984_74,
            1.0,
        ]);

        let scalar = AudioFile::from_audio_buffer(source.clone())
            .into_pipeline()
            .with_backend(BackendKind::Scalar)
            .dc_shift(0.125)
            .into_audio_buffer()
            .unwrap();
        let simd = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .with_backend(BackendKind::Simd)
            .dc_shift(0.125)
            .into_audio_buffer()
            .unwrap();

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn chain_trim_frames_preserves_stereo_frame_grouping() {
        let source = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 0.75, 1.0, -0.25, -0.5, -0.75]);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .trim_frames(1, 3)
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.as_planar_f32(), &[0.25, 0.5, -0.25, -0.5]);
    }

    #[test]
    fn chain_trim_seconds_uses_floor_conversion() {
        let actual = AudioFile::from_audio_buffer(audio_buffer(vec![0.0, 0.25, 0.5, 0.75]))
            .into_pipeline()
            .trim_seconds(1.0 / 48_000.0, 3.9 / 48_000.0)
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.as_planar_f32(), &[0.25, 0.5]);
    }

    #[test]
    fn chain_pad_frames_preserves_stereo_frame_grouping() {
        let source = stereo_audio_buffer(vec![0.25, 0.5, -0.25, -0.5]);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .pad_frames(1, 1)
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(4));
        assert_eq!(
            actual.as_planar_f32(),
            &[0.0, 0.25, 0.5, 0.0, 0.0, -0.25, -0.5, 0.0]
        );
    }

    #[test]
    fn chain_fade_frames_preserves_stereo_frame_grouping() {
        let source = stereo_audio_buffer(vec![1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0]);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .fade_frames(2, 2)
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(4));
        assert_samples_close(
            actual.as_planar_f32(),
            &[0.0, 0.5, 0.5, 0.0, -0.0, -0.5, -0.5, -0.0],
        );
    }

    #[test]
    fn chain_fade_frames_matches_under_forced_scalar_and_requested_simd() {
        let source = stereo_audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -0.5,
            -0.0,
            0.0,
            0.5,
            0.999_984_74,
            1.0,
            1.0,
            0.999_984_74,
            0.5,
            0.0,
            -0.0,
            -0.5,
            -0.999_984_74,
            -1.0,
        ]);

        let scalar = AudioFile::from_audio_buffer(source.clone())
            .into_pipeline()
            .with_backend(BackendKind::Scalar)
            .fade_frames(5, 7)
            .into_audio_buffer()
            .unwrap();
        let simd = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .with_backend(BackendKind::Simd)
            .fade_frames(5, 7)
            .into_audio_buffer()
            .unwrap();

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn chain_reverse_preserves_stereo_frame_grouping() {
        let source = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 1.0, -0.25, -0.5]);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .reverse()
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(3));
        assert_eq!(actual.as_planar_f32(), &[0.5, 0.25, 0.0, -0.5, -0.25, 1.0]);
    }

    #[test]
    fn apply_effect_chain_matches_repeated_fluent_pipeline_calls() {
        let source = stereo_audio_buffer(vec![0.25, -0.5, 0.75, 1.0, -0.25, 0.5, -0.75, -1.0]);
        let chain = EffectChain::new(vec![
            EffectCommand::Gain(Gain::new(Decibels::new(-3.0).unwrap())),
            EffectCommand::DcShift(DcShift::new(0.125).unwrap()),
            EffectCommand::Fade(Fade::new(FrameCount::new(2), FrameCount::new(2))),
            EffectCommand::Trim(Trim::new(FrameCount::new(1), FrameCount::new(3)).unwrap()),
            EffectCommand::Reverse(Reverse::new()),
        ]);

        let actual = AudioFile::from_audio_buffer(source.clone())
            .into_pipeline()
            .apply_effect_chain(&chain)
            .into_audio_buffer()
            .unwrap();
        let expected = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .gain_db(-3.0)
            .dc_shift(0.125)
            .fade_frames(2, 2)
            .trim_frames(1, 3)
            .reverse()
            .into_audio_buffer()
            .unwrap();

        assert_samples_close(actual.as_planar_f32(), expected.as_planar_f32());
        assert_eq!(actual.frames(), expected.frames());
        assert_eq!(actual.channels(), expected.channels());
    }

    #[test]
    fn apply_effect_chain_matches_under_forced_scalar_and_requested_simd() {
        let source = stereo_audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -0.5,
            -0.0,
            0.0,
            0.5,
            0.999_984_74,
            1.0,
            1.0,
            0.999_984_74,
            0.5,
            0.0,
            -0.0,
            -0.5,
            -0.999_984_74,
            -1.0,
        ]);
        let chain = EffectChain::new(vec![
            EffectCommand::Gain(Gain::new(Decibels::new(-3.0).unwrap())),
            EffectCommand::DcShift(DcShift::new(0.125).unwrap()),
            EffectCommand::Fade(Fade::new(FrameCount::new(5), FrameCount::new(7))),
            EffectCommand::Reverse(Reverse::new()),
        ]);

        let scalar = AudioFile::from_audio_buffer(source.clone())
            .into_pipeline()
            .with_backend(BackendKind::Scalar)
            .apply_effect_chain(&chain)
            .into_audio_buffer()
            .unwrap();
        let simd = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .with_backend(BackendKind::Simd)
            .apply_effect_chain(&chain)
            .into_audio_buffer()
            .unwrap();

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn apply_effect_chain_errors_include_failing_command_context() {
        let chain = EffectChain::new(vec![EffectCommand::Trim(
            Trim::new(FrameCount::new(0), FrameCount::new(3)).unwrap(),
        )]);

        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25, -0.5]))
            .into_pipeline()
            .apply_effect_chain(&chain)
            .into_audio_buffer()
            .unwrap_err();

        assert!(matches!(
            error,
            Error::Chain(auralis_effects::EffectChainError::CommandFailed {
                index: 0,
                command: EffectCommand::Trim(_),
                argument: "frame-range",
                source: auralis_effects::EffectError::TrimRangeOutOfBounds,
            })
        ));
        assert_eq!(
            error.to_string(),
            "effect chain command 0 (`trim 0 3`) failed while applying `frame-range`: trim frame range must be within the input duration"
        );
    }

    #[test]
    fn invalid_trim_range_propagates_without_panic() {
        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
            .into_pipeline()
            .trim_frames(2, 1)
            .into_audio_buffer()
            .unwrap_err();

        assert_eq!(
            error,
            Error::Effect(auralis_effects::EffectError::InvalidTrimOrder)
        );
    }

    #[test]
    fn invalid_trim_seconds_propagates_without_panic() {
        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
            .into_pipeline()
            .trim_seconds(f64::NAN, 1.0)
            .into_audio_buffer()
            .unwrap_err();

        assert_eq!(
            error,
            Error::Core(auralis_core::AuralisError::InvalidTimeSeconds)
        );
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
    fn invalid_dc_shift_propagates_without_panic() {
        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
            .into_pipeline()
            .dc_shift(f32::NAN)
            .into_audio_buffer()
            .unwrap_err();

        assert_eq!(
            error,
            Error::Effect(auralis_effects::EffectError::InvalidDcShift)
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

    fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        assert_eq!(samples.len() % 2, 0);
        let frames = u64::try_from(samples.len() / 2).unwrap();
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(2).unwrap(),
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

    fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());

        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "sample {index} differed: {actual} != {expected}"
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
