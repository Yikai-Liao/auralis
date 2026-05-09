//! Core vocabulary for Auralis.
//!
//! The types in this crate encode the small set of audio invariants shared by
//! codecs, DSP kernels, effects, and command-line entry points. Constructors
//! validate user-provided values once so downstream code can rely on typed
//! units instead of raw primitives.
//!
//! # Examples
//!
//! ```
//! use auralis_core::{AudioSpec, ChannelCount, SampleFormat, SampleRate};
//!
//! let spec = AudioSpec::new(
//!     SampleRate::new(48_000)?,
//!     ChannelCount::new(2)?,
//!     SampleFormat::Float32,
//! );
//!
//! assert_eq!(spec.sample_rate().as_u32(), 48_000);
//! assert_eq!(spec.channels().as_u16(), 2);
//! # Ok::<(), auralis_core::AuralisError>(())
//! ```

use std::fmt;

use thiserror::Error;

/// Crate-local result type using [`AuralisError`].
pub type Result<T> = std::result::Result<T, AuralisError>;

/// Errors produced by core Auralis value constructors.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum AuralisError {
    /// A sample rate of zero was provided.
    #[error("sample rate must be greater than zero")]
    InvalidSampleRate,

    /// A channel count of zero was provided.
    #[error("channel count must be greater than zero")]
    InvalidChannelCount,

    /// A non-finite or negative frequency was provided.
    #[error("frequency must be finite and non-negative")]
    InvalidHertz,

    /// A non-finite decibel value was provided.
    #[error("decibels must be finite")]
    InvalidDecibels,

    /// A non-finite or negative time value was provided.
    #[error("time in seconds must be finite and non-negative")]
    InvalidTimeSeconds,

    /// Planar audio data length did not match its declared shape.
    #[error("audio buffer data length does not match channel and frame count")]
    InvalidAudioBufferShape,
}

/// Audio sample rate in frames per second.
///
/// `SampleRate` is deterministic and exact: it stores the integer rate supplied
/// by the caller and performs no normalization.
///
/// # Errors
///
/// Returns [`AuralisError::InvalidSampleRate`] when `frames_per_second` is zero.
///
/// # Examples
///
/// ```
/// use auralis_core::SampleRate;
///
/// let rate = SampleRate::new(44_100)?;
/// assert_eq!(rate.as_u32(), 44_100);
/// # Ok::<(), auralis_core::AuralisError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SampleRate(u32);

impl SampleRate {
    /// Creates a sample rate from frames per second.
    ///
    /// # Errors
    ///
    /// Returns [`AuralisError::InvalidSampleRate`] when `frames_per_second` is
    /// zero.
    pub const fn new(frames_per_second: u32) -> Result<Self> {
        if frames_per_second == 0 {
            Err(AuralisError::InvalidSampleRate)
        } else {
            Ok(Self(frames_per_second))
        }
    }

    /// Returns the sample rate as frames per second.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SampleRate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} Hz", self.0)
    }
}

/// Number of interdependent audio channels.
///
/// # Errors
///
/// Returns [`AuralisError::InvalidChannelCount`] when `channels` is zero.
///
/// # Examples
///
/// ```
/// use auralis_core::ChannelCount;
///
/// let channels = ChannelCount::new(2)?;
/// assert_eq!(channels.as_u16(), 2);
/// # Ok::<(), auralis_core::AuralisError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChannelCount(u16);

impl ChannelCount {
    /// Creates a channel count.
    ///
    /// # Errors
    ///
    /// Returns [`AuralisError::InvalidChannelCount`] when `channels` is zero.
    pub const fn new(channels: u16) -> Result<Self> {
        if channels == 0 {
            Err(AuralisError::InvalidChannelCount)
        } else {
            Ok(Self(channels))
        }
    }

    /// Returns the number of channels.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// Returns the number of channels as a `usize` for indexing and allocation.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for ChannelCount {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ch", self.0)
    }
}

/// Number of audio frames.
///
/// A frame contains one sample per channel. `FrameCount` allows zero so empty
/// buffers and silent ranges can be represented explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameCount(u64);

impl FrameCount {
    /// Creates a frame count.
    #[must_use]
    pub const fn new(frames: u64) -> Self {
        Self(frames)
    }

    /// Returns the frame count.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for FrameCount {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} frames", self.0)
    }
}

/// Frequency in hertz.
///
/// Zero hertz is valid for contexts that model DC or a lower bound. Callers
/// that require strictly positive frequencies should validate that separately.
///
/// # Errors
///
/// Returns [`AuralisError::InvalidHertz`] when `hertz` is negative, NaN, or
/// infinite.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Hertz(f64);

impl Hertz {
    /// Creates a frequency in hertz.
    ///
    /// # Errors
    ///
    /// Returns [`AuralisError::InvalidHertz`] when `hertz` is negative, NaN, or
    /// infinite.
    pub fn new(hertz: f64) -> Result<Self> {
        if hertz.is_finite() && hertz >= 0.0 {
            Ok(Self(hertz))
        } else {
            Err(AuralisError::InvalidHertz)
        }
    }

    /// Returns the frequency in hertz.
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0
    }
}

impl fmt::Display for Hertz {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} Hz", self.0)
    }
}

/// Gain or level value in decibels.
///
/// Negative, positive, and zero values are all valid. NaN and infinity are
/// rejected so numerical behavior stays deterministic.
///
/// # Errors
///
/// Returns [`AuralisError::InvalidDecibels`] when `decibels` is NaN or infinite.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Decibels(f64);

impl Decibels {
    /// Creates a decibel value.
    ///
    /// # Errors
    ///
    /// Returns [`AuralisError::InvalidDecibels`] when `decibels` is NaN or
    /// infinite.
    pub fn new(decibels: f64) -> Result<Self> {
        if decibels.is_finite() {
            Ok(Self(decibels))
        } else {
            Err(AuralisError::InvalidDecibels)
        }
    }

    /// Returns the value in decibels.
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0
    }
}

impl fmt::Display for Decibels {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} dB", self.0)
    }
}

/// Duration or timestamp measured in seconds.
///
/// Time values are non-negative and finite. This type does not encode whether a
/// value is absolute or relative; API names should describe that semantic role.
///
/// # Errors
///
/// Returns [`AuralisError::InvalidTimeSeconds`] when `seconds` is negative, NaN,
/// or infinite.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct TimeSeconds(f64);

impl TimeSeconds {
    /// Creates a time value in seconds.
    ///
    /// # Errors
    ///
    /// Returns [`AuralisError::InvalidTimeSeconds`] when `seconds` is negative,
    /// NaN, or infinite.
    pub fn new(seconds: f64) -> Result<Self> {
        if seconds.is_finite() && seconds >= 0.0 {
            Ok(Self(seconds))
        } else {
            Err(AuralisError::InvalidTimeSeconds)
        }
    }

    /// Returns the time value in seconds.
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0
    }
}

impl fmt::Display for TimeSeconds {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} s", self.0)
    }
}

/// Sample representation used at an audio boundary.
///
/// The internal DSP format is expected to be planar `f32`; this enum describes
/// external or declared sample formats without exposing codec implementation
/// details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SampleFormat {
    /// Signed 16-bit integer PCM.
    Pcm16,

    /// 32-bit IEEE floating-point samples.
    Float32,
}

impl fmt::Display for SampleFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pcm16 => formatter.write_str("pcm16"),
            Self::Float32 => formatter.write_str("float32"),
        }
    }
}

/// Audio stream shape and boundary sample format.
///
/// `AudioSpec` contains only stream metadata. It does not include frame count,
/// channel labels, codec metadata, or container-specific fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioSpec {
    sample_rate: SampleRate,
    channels: ChannelCount,
    sample_format: SampleFormat,
}

impl AudioSpec {
    /// Creates an audio specification from already-validated values.
    #[must_use]
    pub const fn new(
        sample_rate: SampleRate,
        channels: ChannelCount,
        sample_format: SampleFormat,
    ) -> Self {
        Self {
            sample_rate,
            channels,
            sample_format,
        }
    }

    /// Returns the sample rate.
    #[must_use]
    pub const fn sample_rate(self) -> SampleRate {
        self.sample_rate
    }

    /// Returns the channel count.
    #[must_use]
    pub const fn channels(self) -> ChannelCount {
        self.channels
    }

    /// Returns the boundary sample format.
    #[must_use]
    pub const fn sample_format(self) -> SampleFormat {
        self.sample_format
    }
}

/// Internal planar `f32` audio storage.
///
/// Samples are stored by channel, then by frame: all frames for channel `0`,
/// followed by all frames for channel `1`, and so on. This layout is
/// deterministic and exact; construction only validates shape and never
/// normalizes sample values.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(2)?,
///     SampleFormat::Float32,
/// );
/// let mut buffer = AudioBuffer::zeroed(spec, FrameCount::new(3))?;
/// buffer.channel_mut(1).unwrap()[2] = 0.5;
///
/// assert_eq!(buffer.sample(1, 2), Some(0.5));
/// # Ok::<(), auralis_core::AuralisError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    spec: AudioSpec,
    frames: FrameCount,
    data: Vec<f32>,
}

impl AudioBuffer {
    /// Creates a buffer from already-planar `f32` data.
    ///
    /// The expected data length is `channels * frames`, with each channel
    /// occupying one contiguous `frames`-sample region. Zero frames are valid
    /// and require an empty data vector.
    ///
    /// # Errors
    ///
    /// Returns [`AuralisError::InvalidAudioBufferShape`] when `frames` cannot
    /// be represented as `usize`, when `channels * frames` overflows, or when
    /// `data.len()` does not match the declared shape.
    pub fn from_planar_f32(spec: AudioSpec, frames: FrameCount, data: Vec<f32>) -> Result<Self> {
        let expected_len = planar_sample_count(spec.channels(), frames)?;

        if data.len() == expected_len {
            Ok(Self { spec, frames, data })
        } else {
            Err(AuralisError::InvalidAudioBufferShape)
        }
    }

    /// Creates a zero-initialized planar buffer for `spec` and `frames`.
    ///
    /// # Errors
    ///
    /// Returns [`AuralisError::InvalidAudioBufferShape`] when `frames` cannot
    /// be represented as `usize` or when `channels * frames` overflows.
    pub fn zeroed(spec: AudioSpec, frames: FrameCount) -> Result<Self> {
        let sample_count = planar_sample_count(spec.channels(), frames)?;

        Ok(Self {
            spec,
            frames,
            data: vec![0.0; sample_count],
        })
    }

    /// Returns the buffer audio specification.
    #[must_use]
    pub const fn spec(&self) -> AudioSpec {
        self.spec
    }

    /// Returns the number of frames in each channel.
    #[must_use]
    pub const fn frames(&self) -> FrameCount {
        self.frames
    }

    /// Returns the number of channels.
    #[must_use]
    pub const fn channels(&self) -> ChannelCount {
        self.spec.channels()
    }

    /// Returns the planar sample data.
    #[must_use]
    pub fn as_planar_f32(&self) -> &[f32] {
        &self.data
    }

    /// Returns the mutable planar sample data.
    #[must_use]
    pub fn as_planar_f32_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }

    /// Returns a read-only view of one channel.
    #[must_use]
    pub fn channel(&self, channel_index: usize) -> Option<&[f32]> {
        self.channel_range(channel_index)
            .map(|range| &self.data[range])
    }

    /// Returns a mutable view of one channel.
    #[must_use]
    pub fn channel_mut(&mut self, channel_index: usize) -> Option<&mut [f32]> {
        self.channel_range(channel_index)
            .map(|range| &mut self.data[range])
    }

    /// Returns one sample by channel and frame index.
    #[must_use]
    pub fn sample(&self, channel_index: usize, frame_index: usize) -> Option<f32> {
        self.sample_index(channel_index, frame_index)
            .map(|index| self.data[index])
    }

    /// Returns a mutable reference to one sample by channel and frame index.
    #[must_use]
    pub fn sample_mut(&mut self, channel_index: usize, frame_index: usize) -> Option<&mut f32> {
        self.sample_index(channel_index, frame_index)
            .map(|index| &mut self.data[index])
    }

    fn channel_range(&self, channel_index: usize) -> Option<std::ops::Range<usize>> {
        let frames = usize::try_from(self.frames.as_u64()).ok()?;

        if channel_index >= self.channels().as_usize() {
            return None;
        }

        let start = channel_index.checked_mul(frames)?;
        let end = start.checked_add(frames)?;
        Some(start..end)
    }

    fn sample_index(&self, channel_index: usize, frame_index: usize) -> Option<usize> {
        let range = self.channel_range(channel_index)?;

        if frame_index < range.len() {
            Some(range.start + frame_index)
        } else {
            None
        }
    }
}

fn planar_sample_count(channels: ChannelCount, frames: FrameCount) -> Result<usize> {
    let frames =
        usize::try_from(frames.as_u64()).map_err(|_| AuralisError::InvalidAudioBufferShape)?;

    channels
        .as_usize()
        .checked_mul(frames)
        .ok_or(AuralisError::InvalidAudioBufferShape)
}

#[cfg(test)]
mod tests {
    use super::{
        AudioBuffer, AudioSpec, AuralisError, ChannelCount, Decibels, FrameCount, Hertz,
        SampleFormat, SampleRate, TimeSeconds,
    };

    #[test]
    fn sample_rate_rejects_zero() {
        assert_eq!(SampleRate::new(0), Err(AuralisError::InvalidSampleRate));
    }

    #[test]
    fn sample_rate_accepts_positive_values() {
        let rate = SampleRate::new(96_000).unwrap();

        assert_eq!(rate.as_u32(), 96_000);
        assert_eq!(rate.to_string(), "96000 Hz");
        assert_eq!(format!("{rate:?}"), "SampleRate(96000)");
    }

    #[test]
    fn channel_count_rejects_zero() {
        assert_eq!(ChannelCount::new(0), Err(AuralisError::InvalidChannelCount));
    }

    #[test]
    fn channel_count_accepts_positive_values() {
        let channels = ChannelCount::new(2).unwrap();

        assert_eq!(channels.as_u16(), 2);
        assert_eq!(channels.as_usize(), 2);
        assert_eq!(channels.to_string(), "2 ch");
        assert_eq!(format!("{channels:?}"), "ChannelCount(2)");
    }

    #[test]
    fn frame_count_allows_zero() {
        let frames = FrameCount::new(0);

        assert_eq!(frames.as_u64(), 0);
        assert_eq!(frames.to_string(), "0 frames");
    }

    #[test]
    fn hertz_rejects_non_finite_or_negative_values() {
        assert_eq!(Hertz::new(-1.0), Err(AuralisError::InvalidHertz));
        assert_eq!(Hertz::new(f64::NAN), Err(AuralisError::InvalidHertz));
        assert_eq!(Hertz::new(f64::INFINITY), Err(AuralisError::InvalidHertz));
    }

    #[test]
    fn hertz_accepts_zero_and_positive_values() {
        let frequency = Hertz::new(440.0).unwrap();

        assert_eq!(frequency.as_f64().to_bits(), 440.0_f64.to_bits());
        assert_eq!(frequency.to_string(), "440 Hz");
    }

    #[test]
    fn decibels_rejects_non_finite_values() {
        assert_eq!(Decibels::new(f64::NAN), Err(AuralisError::InvalidDecibels));
        assert_eq!(
            Decibels::new(f64::NEG_INFINITY),
            Err(AuralisError::InvalidDecibels)
        );
    }

    #[test]
    fn decibels_accepts_finite_values() {
        let level = Decibels::new(-3.0).unwrap();

        assert_eq!(level.as_f64().to_bits(), (-3.0_f64).to_bits());
        assert_eq!(level.to_string(), "-3 dB");
    }

    #[test]
    fn time_seconds_rejects_non_finite_or_negative_values() {
        assert_eq!(
            TimeSeconds::new(-0.1),
            Err(AuralisError::InvalidTimeSeconds)
        );
        assert_eq!(
            TimeSeconds::new(f64::NAN),
            Err(AuralisError::InvalidTimeSeconds)
        );
        assert_eq!(
            TimeSeconds::new(f64::INFINITY),
            Err(AuralisError::InvalidTimeSeconds)
        );
    }

    #[test]
    fn time_seconds_accepts_zero_and_positive_values() {
        let time = TimeSeconds::new(1.25).unwrap();

        assert_eq!(time.as_f64().to_bits(), 1.25_f64.to_bits());
        assert_eq!(time.to_string(), "1.25 s");
    }

    #[test]
    fn sample_format_has_stable_display_values() {
        assert_eq!(SampleFormat::Pcm16.to_string(), "pcm16");
        assert_eq!(SampleFormat::Float32.to_string(), "float32");
        assert_eq!(format!("{:?}", SampleFormat::Pcm16), "Pcm16");
    }

    #[test]
    fn audio_spec_exposes_stream_shape() {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(2).unwrap(),
            SampleFormat::Float32,
        );

        assert_eq!(spec.sample_rate().as_u32(), 48_000);
        assert_eq!(spec.channels().as_u16(), 2);
        assert_eq!(spec.sample_format(), SampleFormat::Float32);
        assert_eq!(
            format!("{spec:?}"),
            "AudioSpec { sample_rate: SampleRate(48000), channels: ChannelCount(2), sample_format: Float32 }"
        );
    }

    #[test]
    fn core_errors_have_actionable_display_messages() {
        assert_eq!(
            AuralisError::InvalidSampleRate.to_string(),
            "sample rate must be greater than zero"
        );
        assert_eq!(
            AuralisError::InvalidChannelCount.to_string(),
            "channel count must be greater than zero"
        );
        assert_eq!(
            AuralisError::InvalidAudioBufferShape.to_string(),
            "audio buffer data length does not match channel and frame count"
        );
    }

    #[test]
    fn mono_audio_buffer_preserves_planar_layout() {
        let buffer = AudioBuffer::from_planar_f32(
            test_spec(1),
            FrameCount::new(4),
            vec![0.0, 0.25, -0.5, 1.0],
        )
        .unwrap();

        assert_eq!(buffer.spec(), test_spec(1));
        assert_eq!(buffer.frames().as_u64(), 4);
        assert_eq!(buffer.channels().as_u16(), 1);
        assert_eq!(buffer.as_planar_f32(), &[0.0, 0.25, -0.5, 1.0]);
        assert_eq!(buffer.channel(0), Some([0.0, 0.25, -0.5, 1.0].as_slice()));
        assert_eq!(buffer.sample(0, 3), Some(1.0));
        assert_eq!(buffer.sample(1, 0), None);
    }

    #[test]
    fn stereo_audio_buffer_uses_channel_major_storage() {
        let buffer = AudioBuffer::from_planar_f32(
            test_spec(2),
            FrameCount::new(3),
            vec![1.0, 2.0, 3.0, -1.0, -2.0, -3.0],
        )
        .unwrap();

        assert_eq!(buffer.channel(0), Some([1.0, 2.0, 3.0].as_slice()));
        assert_eq!(buffer.channel(1), Some([-1.0, -2.0, -3.0].as_slice()));
        assert_eq!(buffer.sample(0, 2), Some(3.0));
        assert_eq!(buffer.sample(1, 2), Some(-3.0));
        assert_eq!(buffer.sample(1, 3), None);
    }

    #[test]
    fn channel_views_can_mutate_one_channel_without_touching_others() {
        let mut buffer = AudioBuffer::zeroed(test_spec(2), FrameCount::new(2)).unwrap();

        buffer.channel_mut(1).unwrap().copy_from_slice(&[0.5, -0.5]);
        *buffer.sample_mut(0, 1).unwrap() = 0.25;

        assert_eq!(buffer.channel(0), Some([0.0, 0.25].as_slice()));
        assert_eq!(buffer.channel(1), Some([0.5, -0.5].as_slice()));
        assert_eq!(buffer.as_planar_f32_mut(), &mut [0.0, 0.25, 0.5, -0.5]);
    }

    #[test]
    fn invalid_audio_buffer_data_length_is_rejected() {
        assert_eq!(
            AudioBuffer::from_planar_f32(test_spec(2), FrameCount::new(3), vec![0.0; 5]),
            Err(AuralisError::InvalidAudioBufferShape)
        );
        assert_eq!(
            AudioBuffer::from_planar_f32(test_spec(1), FrameCount::new(u64::MAX), Vec::new()),
            Err(AuralisError::InvalidAudioBufferShape)
        );
    }

    #[test]
    fn zero_length_audio_buffer_has_empty_channel_views() {
        let mut buffer = AudioBuffer::zeroed(test_spec(2), FrameCount::new(0)).unwrap();

        assert_eq!(buffer.as_planar_f32(), &[]);
        assert_eq!(buffer.channel(0), Some([].as_slice()));
        assert_eq!(buffer.channel(1), Some([].as_slice()));
        assert_eq!(buffer.channel(2), None);
        assert_eq!(buffer.channel_mut(0), Some([].as_mut_slice()));
        assert_eq!(buffer.sample(0, 0), None);
    }

    fn test_spec(channels: u16) -> AudioSpec {
        AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(channels).unwrap(),
            SampleFormat::Float32,
        )
    }
}
