use auralis_core::{AudioBuffer, AudioSpec, SampleRate};

use crate::{EffectError, Result};

/// SoX-ng-style speed adjustment.
///
/// `Speed` changes pitch and tempo together by changing the buffer sample-rate
/// metadata while preserving decoded samples and frame count. It does not
/// resample audio; the higher-quality `rate` command is planned as a later
/// feature.
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidSpeedFactor`] for non-finite or
/// non-positive factors. [`Self::process_buffer`] returns
/// [`EffectError::SpeedRateOutOfRange`] when `input_rate * factor` cannot be
/// represented as a positive integer sample rate.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Speed;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(3),
///     vec![0.25, -0.5, 1.0],
/// )?;
///
/// let sped_up = Speed::new(1.5)?.process_buffer(&audio)?;
///
/// assert_eq!(sped_up.spec().sample_rate().as_u32(), 72_000);
/// assert_eq!(sped_up.as_planar_f32(), &[0.25, -0.5, 1.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Speed {
    /// Ratio of new speed to old speed.
    pub factor: f64,
}

impl Speed {
    /// Creates a speed processor from a positive ratio.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSpeedFactor`] when `factor` is non-finite
    /// or not greater than zero.
    pub fn new(factor: f64) -> Result<Self> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(EffectError::InvalidSpeedFactor);
        }

        Ok(Self { factor })
    }

    /// Creates a speed processor from cents.
    ///
    /// Positive values speed up playback and negative values slow it down.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSpeedFactor`] when `cents` is non-finite
    /// or cannot produce a finite positive factor.
    pub fn from_cents(cents: f64) -> Result<Self> {
        Self::new(2.0_f64.powf(cents / 1200.0))
    }

    /// Applies the speed metadata transform to an audio buffer.
    ///
    /// Factor `1` is an identity copy. Other factors preserve channel count,
    /// frame count, sample format, and sample values while rounding the
    /// resulting sample rate to the nearest integer hertz.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::SpeedRateOutOfRange`] when the output sample rate
    /// rounds below 1 Hz or above `u32::MAX`.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let output_rate = speed_sample_rate(audio.spec().sample_rate(), self.factor)?;
        let spec = AudioSpec::new(output_rate, audio.channels(), audio.spec().sample_format());

        Ok(AudioBuffer::from_planar_f32(
            spec,
            audio.frames(),
            audio.as_planar_f32().to_vec(),
        )?)
    }
}

fn speed_sample_rate(input_rate: SampleRate, factor: f64) -> Result<SampleRate> {
    let output = f64::from(input_rate.as_u32()) * factor;
    if !output.is_finite() || output < 0.5 || output > f64::from(u32::MAX) {
        return Err(EffectError::SpeedRateOutOfRange);
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "range and finiteness are validated before rounding into Auralis' integer sample-rate type"
    )]
    let rounded = output.round() as u32;
    SampleRate::new(rounded).map_err(EffectError::Core)
}

#[cfg(test)]
mod tests {
    use super::Speed;
    use crate::EffectError;
    use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};

    #[test]
    fn changes_sample_rate_without_touching_samples_or_frames() {
        let audio = audio_buffer(48_000, vec![0.0, 0.25, -0.5, 1.0]);

        let sped_up = Speed::new(1.5).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(sped_up.spec().sample_rate().as_u32(), 72_000);
        assert_eq!(sped_up.frames(), FrameCount::new(4));
        assert_eq!(sped_up.as_planar_f32(), &[0.0, 0.25, -0.5, 1.0]);
    }

    #[test]
    fn converts_cents_to_speed_factor() {
        let audio = audio_buffer(48_000, vec![0.0]);

        let sped_up = Speed::from_cents(100.0)
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(sped_up.spec().sample_rate().as_u32(), 50_854);
    }

    #[test]
    fn factor_one_is_identity_copy() {
        let audio = audio_buffer(48_000, vec![-0.5, 0.0, 0.5]);

        let unchanged = Speed::new(1.0).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(unchanged, audio);
    }

    #[test]
    fn rejects_invalid_factor_and_unrepresentable_rate() {
        assert_eq!(
            Speed::new(0.0).unwrap_err(),
            EffectError::InvalidSpeedFactor
        );
        assert_eq!(
            Speed::new(f64::NAN).unwrap_err(),
            EffectError::InvalidSpeedFactor
        );

        let audio = audio_buffer(u32::MAX, vec![0.0]);
        assert_eq!(
            Speed::new(2.0).unwrap().process_buffer(&audio).unwrap_err(),
            EffectError::SpeedRateOutOfRange
        );
    }

    fn audio_buffer(sample_rate: u32, samples: Vec<f32>) -> auralis_core::AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(sample_rate).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        );
        auralis_core::AudioBuffer::from_planar_f32(
            spec,
            FrameCount::new(u64::try_from(samples.len()).unwrap()),
            samples,
        )
        .unwrap()
    }
}
