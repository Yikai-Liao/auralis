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
//! use auralis_effects::{DcShift, Gain};
//!
//! let gain = Gain::new(Decibels::new(-3.0)?);
//! let mut samples = [0.25, -0.5, 1.0];
//! gain.process_samples(&mut samples);
//! DcShift::new(-0.25)?.process_samples(&mut samples);
//!
//! assert!(samples[2] > 0.45 && samples[2] < 0.46);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use auralis_core::{AudioBuffer, Decibels, FrameCount};
use auralis_dsp::{dc_shift_in_place, fade_in_place, gain_in_place};
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

    /// A DC shift amount was not finite or not in the supported normalized range.
    #[error("dc shift must be finite and in the range -2.0..=2.0")]
    InvalidDcShift,

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

/// Constant DC offset effect processor.
///
/// `DcShift` adds a normalized full-scale offset to every sample. For example,
/// `0.25` adds one quarter of full scale and `0.0` is identity. The accepted
/// shift range is `-2.0..=2.0`, matching SoX-ng's single-argument `dcshift`
/// command. The processor does not clip, normalize, allocate, or inspect
/// channel boundaries; samples outside `[-1.0, 1.0]` are clipped only by later
/// boundary encoders such as PCM16 WAV output. Finite samples never become
/// `NaN`.
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidDcShift`] when the shift is
/// `NaN`, infinite, or outside `-2.0..=2.0`.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::DcShift;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(3),
///     vec![-0.5, 0.0, 0.5],
/// )?;
///
/// DcShift::new(0.25)?.process_buffer(&mut audio);
///
/// assert_eq!(audio.as_planar_f32(), &[-0.25, 0.25, 0.75]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DcShift {
    /// Normalized full-scale offset to add to every sample.
    pub shift: f32,
}

impl DcShift {
    /// Creates a DC shift processor from a normalized full-scale offset.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidDcShift`] when `shift` is not finite or
    /// outside the supported `-2.0..=2.0` range.
    pub fn new(shift: f32) -> Result<Self> {
        if shift.is_finite() && (-2.0..=2.0).contains(&shift) {
            Ok(Self { shift })
        } else {
            Err(EffectError::InvalidDcShift)
        }
    }

    /// Applies the DC shift to all samples in an audio buffer.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        self.process_samples(audio.as_planar_f32_mut());
    }

    /// Applies the DC shift to a planar sample slice.
    ///
    /// This method is suitable for streaming or chunked processing because each
    /// sample is transformed independently.
    pub fn process_samples(self, samples: &mut [f32]) {
        dc_shift_in_place(samples, self.shift);
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

/// Linear fade-in and fade-out effect processor.
///
/// `Fade` applies independent linear envelopes to the start and end of every
/// channel in a planar `f32` [`AudioBuffer`]. Fade lengths are measured in
/// frames. A fade-in length of `4` uses coefficients
/// `[0.0, 0.25, 0.5, 0.75]`; the first frame after the fade reaches `1.0`.
/// A fade-out length of `4` applies `[0.75, 0.5, 0.25, 0.0]` to the final four
/// frames. If fade-in and fade-out overlap, their coefficients are multiplied.
/// Zero-length fades are identity transforms.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Fade;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(4),
///     vec![1.0, 1.0, 1.0, 1.0],
/// )?;
///
/// Fade::new(FrameCount::new(2), FrameCount::new(2)).process_buffer(&mut audio);
///
/// assert_eq!(audio.as_planar_f32(), &[0.0, 0.5, 0.5, 0.0]);
/// # Ok::<(), auralis_core::AuralisError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fade {
    /// Frames over which to ramp from silence to unity at the start.
    pub fade_in: FrameCount,

    /// Frames over which to ramp from unity to silence at the end.
    pub fade_out: FrameCount,
}

impl Fade {
    /// Creates a linear fade processor.
    #[must_use]
    pub const fn new(fade_in: FrameCount, fade_out: FrameCount) -> Self {
        Self { fade_in, fade_out }
    }

    /// Applies the fade envelope in place to every channel of an audio buffer.
    ///
    /// Processing is deterministic, non-allocating, and preserves channel
    /// grouping. Zero fade lengths are identity transforms.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        let total_frames = audio.frames().as_u64();
        for channel_index in 0..audio.channels().as_usize() {
            if let Some(channel) = audio.channel_mut(channel_index) {
                self.process_channel_segment(channel, total_frames, FrameCount::new(0));
            }
        }
    }

    /// Applies the fade to a contiguous channel segment with a known frame
    /// offset in the full signal.
    ///
    /// This method exists so streaming callers and tests can process chunks
    /// while still using full-signal frame positions. Passing every segment in
    /// order produces the same result as [`Self::process_buffer`].
    pub fn process_channel_segment(
        self,
        samples: &mut [f32],
        total_frames: u64,
        start_frame: FrameCount,
    ) {
        fade_in_place(
            samples,
            total_frames,
            start_frame.as_u64(),
            self.fade_in.as_u64(),
            self.fade_out.as_u64(),
        );
    }
}

/// Frame-level reverse effect processor.
///
/// `Reverse` reverses the frame order independently within every channel of a
/// planar `f32` [`AudioBuffer`]. It never swaps channels: a stereo buffer with
/// left channel `[L0, L1]` and right channel `[R0, R1]` becomes `[L1, L0]` and
/// `[R1, R0]`. Processing is deterministic, in-place, non-allocating, and an
/// identity transform for zero-length and one-frame buffers.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Reverse;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(2)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(3),
///     vec![0.0, 0.25, 0.5, -0.5, -0.25, 0.0],
/// )?;
///
/// Reverse::new().process_buffer(&mut audio);
///
/// assert_eq!(audio.as_planar_f32(), &[0.5, 0.25, 0.0, 0.0, -0.25, -0.5]);
/// # Ok::<(), auralis_core::AuralisError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Reverse;

impl Reverse {
    /// Creates a frame-level reverse processor.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Reverses frame order in every channel of an audio buffer.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        for channel_index in 0..audio.channels().as_usize() {
            if let Some(channel) = audio.channel_mut(channel_index) {
                self.process_channel(channel);
            }
        }
    }

    /// Reverses a single planar channel in place.
    ///
    /// This method is provided for tests and low-level callers that already
    /// hold channel views. It is not a streaming transform: reversing separate
    /// chunks is not equivalent to reversing the full signal.
    pub fn process_channel(self, samples: &mut [f32]) {
        samples.reverse();
    }
}

#[cfg(test)]
mod tests {
    use super::{DcShift, EffectError, Fade, Gain, Pad, Reverse, Trim};
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
    fn zero_dc_shift_is_identity() {
        let mut audio = audio_buffer(vec![-0.5, 0.0, 0.5]);

        DcShift::new(0.0).unwrap().process_buffer(&mut audio);

        assert_eq!(audio.as_planar_f32(), &[-0.5, 0.0, 0.5]);
    }

    #[test]
    fn dc_shift_adds_positive_and_negative_offsets() {
        let mut positive = audio_buffer(vec![-0.5, 0.0, 0.5]);
        let mut negative = audio_buffer(vec![-0.5, 0.0, 0.5]);

        DcShift::new(0.25).unwrap().process_buffer(&mut positive);
        DcShift::new(-0.25).unwrap().process_buffer(&mut negative);

        assert_eq!(positive.as_planar_f32(), &[-0.25, 0.25, 0.75]);
        assert_eq!(negative.as_planar_f32(), &[-0.75, -0.25, 0.25]);
    }

    #[test]
    fn dc_shift_does_not_clip_and_preserves_stereo_shape() {
        let mut audio = stereo_audio_buffer(vec![0.75, 1.0, -0.75, -1.0]);

        DcShift::new(0.5).unwrap().process_buffer(&mut audio);

        assert_eq!(audio.frames(), FrameCount::new(2));
        assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(audio.as_planar_f32(), &[1.25, 1.5, -0.25, -0.5]);
    }

    #[test]
    fn dc_shift_rejects_non_finite_and_out_of_range_offsets() {
        assert_eq!(DcShift::new(f32::NAN), Err(EffectError::InvalidDcShift));
        assert_eq!(
            DcShift::new(f32::INFINITY),
            Err(EffectError::InvalidDcShift)
        );
        assert_eq!(DcShift::new(2.000_001), Err(EffectError::InvalidDcShift));
        assert_eq!(DcShift::new(-2.000_001), Err(EffectError::InvalidDcShift));
        assert!(DcShift::new(2.0).is_ok());
        assert!(DcShift::new(-2.0).is_ok());
    }

    #[test]
    fn dc_shift_chunked_processing_matches_whole_slice() {
        let source = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
        let mut whole = source.clone();
        let mut chunked = source;

        DcShift::new(0.125).unwrap().process_samples(&mut whole);
        for chunk in chunked.chunks_mut(2) {
            DcShift::new(0.125).unwrap().process_samples(chunk);
        }

        assert_eq!(whole, chunked);
    }

    #[test]
    fn finite_dc_shift_inputs_do_not_produce_nan() {
        let mut audio = audio_buffer(vec![-1.0, -0.0, 0.0, 1.0, f32::MAX]);

        DcShift::new(2.0).unwrap().process_buffer(&mut audio);

        assert!(audio.as_planar_f32().iter().all(|sample| !sample.is_nan()));
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

    #[test]
    fn fade_envelope_coefficients_are_linear() {
        let mut audio = audio_buffer(vec![1.0; 6]);

        Fade::new(FrameCount::new(4), FrameCount::new(3)).process_buffer(&mut audio);

        assert_samples_close(
            audio.as_planar_f32(),
            &[0.0, 0.25, 0.5, 0.5, 1.0 / 3.0, 0.0],
        );
    }

    #[test]
    fn zero_length_fade_is_identity() {
        let source = stereo_audio_buffer(vec![-0.5, 0.0, 0.5, 0.25, -0.25, 1.0]);
        let mut actual = source.clone();

        Fade::new(FrameCount::new(0), FrameCount::new(0)).process_buffer(&mut actual);

        assert_eq!(actual, source);
    }

    #[test]
    fn fade_in_only_and_fade_out_only_apply_one_side() {
        let mut fade_in = audio_buffer(vec![1.0; 5]);
        let mut fade_out = audio_buffer(vec![1.0; 5]);

        Fade::new(FrameCount::new(4), FrameCount::new(0)).process_buffer(&mut fade_in);
        Fade::new(FrameCount::new(0), FrameCount::new(4)).process_buffer(&mut fade_out);

        assert_samples_close(fade_in.as_planar_f32(), &[0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_samples_close(fade_out.as_planar_f32(), &[1.0, 0.75, 0.5, 0.25, 0.0]);
    }

    #[test]
    fn stereo_fade_preserves_channel_grouping() {
        let mut audio = stereo_audio_buffer(vec![1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0]);

        Fade::new(FrameCount::new(2), FrameCount::new(2)).process_buffer(&mut audio);

        assert_eq!(audio.frames(), FrameCount::new(4));
        assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
        assert_samples_close(
            audio.as_planar_f32(),
            &[0.0, 0.5, 0.5, 0.0, -0.0, -0.5, -0.5, -0.0],
        );
    }

    #[test]
    fn fade_segment_processing_matches_whole_channel() {
        let source = vec![1.0; 9];
        let fade = Fade::new(FrameCount::new(4), FrameCount::new(4));
        let mut whole = audio_buffer(source.clone());
        let mut chunked = source;
        let mut start = 0_u64;

        fade.process_buffer(&mut whole);
        for chunk in chunked.chunks_mut(3) {
            fade.process_channel_segment(chunk, 9, FrameCount::new(start));
            start += u64::try_from(chunk.len()).unwrap();
        }

        assert_samples_close(whole.as_planar_f32(), &chunked);
    }

    #[test]
    fn reverse_mono_exact_frame_order() {
        let mut audio = audio_buffer(vec![-0.75, -0.25, 0.25, 0.75]);

        Reverse::new().process_buffer(&mut audio);

        assert_eq!(audio.as_planar_f32(), &[0.75, 0.25, -0.25, -0.75]);
    }

    #[test]
    fn reverse_stereo_preserves_channel_grouping() {
        let mut audio = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 1.0, -0.25, -0.5]);

        Reverse::new().process_buffer(&mut audio);

        assert_eq!(audio.frames(), FrameCount::new(3));
        assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(audio.as_planar_f32(), &[0.5, 0.25, 0.0, -0.5, -0.25, 1.0]);
    }

    #[test]
    fn reverse_twice_is_identity() {
        let source = stereo_audio_buffer(vec![0.0, 0.25, 0.5, -0.5, -0.25, 1.0]);
        let mut actual = source.clone();

        Reverse::new().process_buffer(&mut actual);
        Reverse::new().process_buffer(&mut actual);

        assert_eq!(actual, source);
    }

    #[test]
    fn reverse_zero_length_and_one_frame_are_identity() {
        let mut empty = audio_buffer(Vec::new());
        let mut one_frame = stereo_audio_buffer(vec![0.25, -0.25]);

        Reverse::new().process_buffer(&mut empty);
        Reverse::new().process_buffer(&mut one_frame);

        assert!(empty.as_planar_f32().is_empty());
        assert_eq!(one_frame.as_planar_f32(), &[0.25, -0.25]);
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
