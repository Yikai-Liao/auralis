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

#[cfg(test)]
mod tests {
    use super::{
        AudioSpec, AuralisError, ChannelCount, Decibels, FrameCount, Hertz, SampleFormat,
        SampleRate, TimeSeconds,
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
    }
}
