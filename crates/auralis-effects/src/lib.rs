//! Typed audio effects for Auralis.
//!
//! Effects in this crate own validated configuration and delegate numerical
//! work to deterministic DSP kernels. They operate on Auralis' planar `f32`
//! audio buffers and are chunk-invariant unless their documentation says
//! otherwise.
//!
//! # Examples
//!
//! ```
//! use auralis_core::Decibels;
//! use auralis_effects::Gain;
//!
//! let gain = Gain::new(Decibels::new(-3.0)?);
//! let mut samples = [0.25, -0.5, 1.0];
//! gain.process_samples(&mut samples);
//!
//! assert!(samples[2] > 0.70 && samples[2] < 0.71);
//! # Ok::<(), auralis_core::AuralisError>(())
//! ```

use auralis_core::{AudioBuffer, Decibels, FrameCount};
use auralis_dsp::gain_in_place;
use thiserror::Error;

/// Crate-local result type using [`EffectError`].
pub type Result<T> = std::result::Result<T, EffectError>;

/// Errors produced by typed effect processors.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum EffectError {
    /// The requested frame range has `start > end`.
    #[error("trim start frame must be less than or equal to trim end frame")]
    InvalidTrimOrder,

    /// The requested frame range extends beyond the buffer being trimmed.
    #[error("trim frame range must be within the input duration")]
    TrimRangeOutOfBounds,

    /// The requested padding would create a buffer shape that cannot be represented.
    #[error("pad frame count exceeds representable audio buffer length")]
    PadLengthOverflow,

    /// A buffer with an invalid shape was produced while applying an effect.
    #[error(transparent)]
    Core(#[from] auralis_core::AuralisError),
}

/// Constant-gain effect processor.
///
/// `Gain` multiplies every sample by `10^(db / 20)` using the scalar
/// [`auralis_dsp::gain_in_place`] reference kernel. The processor does not
/// clip, normalize, allocate, or inspect channel boundaries, so processing a
/// whole buffer and processing the same samples in chunks produce identical
/// results.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Gain;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(3),
///     vec![0.25, -0.5, 1.0],
/// )?;
///
/// Gain::new(Decibels::new(6.0)?).process_buffer(&mut audio);
///
/// assert!(audio.as_planar_f32()[0] > 0.49);
/// # Ok::<(), auralis_core::AuralisError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gain {
    /// Gain amount in decibels.
    pub db: Decibels,
}

impl Gain {
    /// Creates a gain processor from a validated decibel value.
    #[must_use]
    pub const fn new(db: Decibels) -> Self {
        Self { db }
    }

    /// Applies gain to all samples in an audio buffer.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        self.process_samples(audio.as_planar_f32_mut());
    }

    /// Applies gain to a planar sample slice.
    ///
    /// This method is suitable for streaming or chunked processing because each
    /// sample is transformed independently.
    pub fn process_samples(self, samples: &mut [f32]) {
        gain_in_place(samples, self.db);
    }
}

/// Frame-range trim effect processor.
///
/// `Trim` keeps the half-open frame range `start..end` from every channel and
/// returns a new planar `f32` [`AudioBuffer`]. The range is measured in frames,
/// not individual samples, so stereo and larger channel layouts preserve frame
/// grouping. `start == end` is valid and produces an empty buffer with the same
/// audio specification.
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidTrimOrder`] when `start > end`.
/// [`Self::process_buffer`] returns [`EffectError::TrimRangeOutOfBounds`] when
/// the validated range is outside the input buffer.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Trim;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(4),
///     vec![0.0, 0.25, 0.5, 0.75],
/// )?;
///
/// let trimmed = Trim::new(FrameCount::new(1), FrameCount::new(3))?
///     .process_buffer(&audio)?;
///
/// assert_eq!(trimmed.as_planar_f32(), &[0.25, 0.5]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trim {
    /// First frame to keep.
    pub start: FrameCount,

    /// End-exclusive frame index.
    pub end: FrameCount,
}

impl Trim {
    /// Creates a frame-range trim processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidTrimOrder`] when `start > end`.
    pub fn new(start: FrameCount, end: FrameCount) -> Result<Self> {
        if start <= end {
            Ok(Self { start, end })
        } else {
            Err(EffectError::InvalidTrimOrder)
        }
    }

    /// Applies the trim to an audio buffer and returns the retained range.
    ///
    /// The output keeps the input specification and stores samples in the same
    /// planar channel-major layout. Processing is deterministic and allocates
    /// exactly `channels * (end - start)` samples.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::TrimRangeOutOfBounds`] when `end` is greater than
    /// the input frame count or the frame indices cannot be represented on the
    /// current platform.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let frames = audio.frames().as_u64();
        if self.end.as_u64() > frames {
            return Err(EffectError::TrimRangeOutOfBounds);
        }

        let start =
            usize::try_from(self.start.as_u64()).map_err(|_| EffectError::TrimRangeOutOfBounds)?;
        let end =
            usize::try_from(self.end.as_u64()).map_err(|_| EffectError::TrimRangeOutOfBounds)?;
        let output_frames = FrameCount::new(self.end.as_u64() - self.start.as_u64());
        let mut output = Vec::with_capacity(
            audio
                .channels()
                .as_usize()
                .checked_mul(end - start)
                .ok_or(EffectError::TrimRangeOutOfBounds)?,
        );

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::TrimRangeOutOfBounds)?;
            output.extend_from_slice(&channel[start..end]);
        }

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            output_frames,
            output,
        )?)
    }
}

/// Start/end silence padding effect processor.
///
/// `Pad` adds zero-valued frames before and after every channel and returns a
/// new planar `f32` [`AudioBuffer`]. Padding lengths are measured in frames, so
/// multi-channel audio keeps channel grouping intact. `0` start and end frames
/// is an identity transform aside from allocating a new buffer with identical
/// contents.
///
/// # Errors
///
/// [`Self::process_buffer`] returns [`EffectError::PadLengthOverflow`] when the
/// requested output frame count or planar sample count cannot be represented on
/// the current platform.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Pad;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(2),
///     vec![0.25, -0.5],
/// )?;
///
/// let padded = Pad::new(FrameCount::new(1), FrameCount::new(1))
///     .process_buffer(&audio)?;
///
/// assert_eq!(padded.as_planar_f32(), &[0.0, 0.25, -0.5, 0.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pad {
    /// Silent frames to add before the input audio.
    pub start: FrameCount,

    /// Silent frames to add after the input audio.
    pub end: FrameCount,
}

impl Pad {
    /// Creates a start/end zero-padding processor.
    #[must_use]
    pub const fn new(start: FrameCount, end: FrameCount) -> Self {
        Self { start, end }
    }

    /// Applies zero padding to an audio buffer and returns the padded output.
    ///
    /// The output keeps the input specification and stores samples in the same
    /// planar channel-major layout. Processing is deterministic and allocates
    /// exactly `channels * (start + input_frames + end)` samples.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::PadLengthOverflow`] when the output frame count
    /// or allocation length cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let output_frames = self
            .start
            .as_u64()
            .checked_add(audio.frames().as_u64())
            .and_then(|frames| frames.checked_add(self.end.as_u64()))
            .ok_or(EffectError::PadLengthOverflow)?;
        let output_frames = FrameCount::new(output_frames);
        let start =
            usize::try_from(self.start.as_u64()).map_err(|_| EffectError::PadLengthOverflow)?;
        let end = usize::try_from(self.end.as_u64()).map_err(|_| EffectError::PadLengthOverflow)?;
        let output_frames_usize =
            usize::try_from(output_frames.as_u64()).map_err(|_| EffectError::PadLengthOverflow)?;
        let capacity = audio
            .channels()
            .as_usize()
            .checked_mul(output_frames_usize)
            .ok_or(EffectError::PadLengthOverflow)?;
        let mut output = Vec::with_capacity(capacity);

        for channel_index in 0..audio.channels().as_usize() {
            output.resize(output.len() + start, 0.0);
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::PadLengthOverflow)?;
            output.extend_from_slice(channel);
            output.resize(output.len() + end, 0.0);
        }

        debug_assert_eq!(output.len(), capacity);

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            output_frames,
            output,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::{EffectError, Gain, Pad, Trim};
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
    };
    use auralis_dsp::gain_in_place;

    #[test]
    fn gain_effect_output_matches_scalar_kernel() {
        let mut actual = audio_buffer(vec![-1.0, -0.25, 0.0, 0.5, 1.0]);
        let mut expected = actual.as_planar_f32().to_vec();
        let db = db(-6.0);

        Gain::new(db).process_buffer(&mut actual);
        gain_in_place(&mut expected, db);

        assert_samples_close(actual.as_planar_f32(), &expected);
    }

    #[test]
    fn whole_buffer_and_chunked_processing_match() {
        let db = db(6.0);
        let source = vec![-1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0];
        let mut whole = audio_buffer(source.clone());
        let mut chunked = source;

        Gain::new(db).process_buffer(&mut whole);
        for chunk in chunked.chunks_mut(4) {
            Gain::new(db).process_samples(chunk);
        }

        assert_samples_close(whole.as_planar_f32(), &chunked);
    }

    #[test]
    fn empty_buffer_is_accepted() {
        let mut audio = audio_buffer(Vec::new());

        Gain::new(db(12.0)).process_buffer(&mut audio);

        assert!(audio.as_planar_f32().is_empty());
    }

    #[test]
    fn trim_exact_frame_range() {
        let audio = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 0.75, 1.0, -0.25, -0.5, -0.75]);

        let trimmed = Trim::new(FrameCount::new(1), FrameCount::new(3))
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(trimmed.frames(), FrameCount::new(2));
        assert_eq!(trimmed.channels(), audio.channels());
        assert_eq!(trimmed.as_planar_f32(), &[0.25, 0.5, -0.25, -0.5]);
    }

    #[test]
    fn trim_full_range_is_identity() {
        let audio = audio_buffer(vec![-0.5, 0.0, 0.5]);

        let trimmed = Trim::new(FrameCount::new(0), audio.frames())
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(trimmed, audio);
    }

    #[test]
    fn trim_empty_range_keeps_shape_metadata() {
        let audio = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 0.75]);

        let trimmed = Trim::new(FrameCount::new(2), FrameCount::new(2))
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(trimmed.frames(), FrameCount::new(0));
        assert_eq!(trimmed.channels(), audio.channels());
        assert!(trimmed.as_planar_f32().is_empty());
    }

    #[test]
    fn trim_rejects_reversed_or_out_of_bounds_ranges() {
        assert_eq!(
            Trim::new(FrameCount::new(3), FrameCount::new(2)),
            Err(EffectError::InvalidTrimOrder)
        );

        let error = Trim::new(FrameCount::new(0), FrameCount::new(4))
            .unwrap()
            .process_buffer(&audio_buffer(vec![0.0, 0.5]))
            .unwrap_err();

        assert_eq!(error, EffectError::TrimRangeOutOfBounds);
    }

    #[test]
    fn zero_pad_is_identity() {
        let audio = audio_buffer(vec![-0.5, 0.0, 0.5]);

        let padded = Pad::new(FrameCount::new(0), FrameCount::new(0))
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(padded, audio);
    }

    #[test]
    fn start_and_end_pad_insert_exact_zeros() {
        let audio = audio_buffer(vec![0.25, -0.5]);

        let padded = Pad::new(FrameCount::new(2), FrameCount::new(1))
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(padded.frames(), FrameCount::new(5));
        assert_eq!(padded.as_planar_f32(), &[0.0, 0.0, 0.25, -0.5, 0.0]);
    }

    #[test]
    fn stereo_pad_preserves_channel_shape() {
        let audio = stereo_audio_buffer(vec![0.25, 0.5, -0.25, -0.5]);

        let padded = Pad::new(FrameCount::new(1), FrameCount::new(2))
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(padded.frames(), FrameCount::new(5));
        assert_eq!(padded.channels(), audio.channels());
        assert_eq!(
            padded.as_planar_f32(),
            &[0.0, 0.25, 0.5, 0.0, 0.0, 0.0, -0.25, -0.5, 0.0, 0.0]
        );
    }

    #[test]
    fn pad_rejects_unrepresentable_frame_count() {
        let error = Pad::new(FrameCount::new(u64::MAX), FrameCount::new(1))
            .process_buffer(&audio_buffer(vec![0.0]))
            .unwrap_err();

        assert_eq!(error, EffectError::PadLengthOverflow);
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

    fn db(value: f64) -> Decibels {
        Decibels::new(value).unwrap()
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());

        for (actual, expected) in actual.iter().zip(expected) {
            let tolerance = 1.0e-6;
            let difference = (actual - expected).abs();

            assert!(
                difference <= tolerance,
                "expected {actual} to be within {tolerance} of {expected}, difference was {difference}"
            );
        }
    }
}
