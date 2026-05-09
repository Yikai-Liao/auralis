use auralis_core::{AudioBuffer, FrameCount, SampleRate};

use crate::{EffectError, Result};

const DEFAULT_DEPTH_PERCENT: f64 = 40.0;

/// SoX-ng-style sinusoidal tremolo modulation.
///
/// `Tremolo` multiplies each frame by a low-frequency sinusoidal envelope.
/// The user-facing `speed_hz` is the modulation frequency in hertz and
/// `depth_percent` is in SoX-ng's documented `(0, 100]` range, defaulting to
/// `40`. The envelope starts at full volume and ranges from `1 - depth / 100`
/// to `1`, matching SoX-ng's `synth sine fmod` implementation with a 25%
/// phase offset.
///
/// The operation is deterministic and streaming-safe when callers preserve the
/// absolute frame offset between chunks. It currently uses scalar `cos`
/// evaluation because the phase-dependent transcendental transform has no
/// SIMD kernel yet.
///
/// # Examples
///
/// ```
/// use auralis_effects::Tremolo;
///
/// let mut samples = [0.5, 0.5, 0.5];
/// Tremolo::new(0.0, 40.0)?.process_mono_samples(
///     &mut samples,
///     auralis_core::SampleRate::new(48_000)?,
///     auralis_core::FrameCount::new(0),
/// );
///
/// assert_eq!(samples, [0.5, 0.5, 0.5]);
/// # Ok::<(), auralis_effects::EffectError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tremolo {
    /// Modulation frequency in hertz.
    pub speed_hz: f64,

    /// Tremolo depth percentage in the SoX-ng `(0, 100]` range.
    pub depth_percent: f64,

    depth_fraction: f64,
}

impl Tremolo {
    /// Creates a tremolo processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidTremolo`] when `speed_hz` is not finite or
    /// negative, or when `depth_percent` is not finite or is outside
    /// `(0, 100]`.
    pub fn new(speed_hz: f64, depth_percent: f64) -> Result<Self> {
        if !speed_hz.is_finite()
            || speed_hz < 0.0
            || !depth_percent.is_finite()
            || depth_percent <= 0.0
            || depth_percent > 100.0
        {
            return Err(EffectError::InvalidTremolo);
        }

        Ok(Self {
            speed_hz,
            depth_percent,
            depth_fraction: depth_percent / 100.0,
        })
    }

    /// Creates a tremolo processor with SoX-ng's default depth.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidTremolo`] when `speed_hz` is not finite or
    /// negative.
    pub fn with_default_depth(speed_hz: f64) -> Result<Self> {
        Self::new(speed_hz, DEFAULT_DEPTH_PERCENT)
    }

    /// Applies tremolo modulation to all samples in an audio buffer.
    ///
    /// # Panics
    ///
    /// Panics only if a validated [`AudioBuffer`] cannot return one of its
    /// declared channels.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        let sample_rate = audio.spec().sample_rate();
        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel_mut(channel_index)
                .expect("channel index is within the audio shape");
            self.process_mono_samples(channel, sample_rate, FrameCount::new(0));
        }
    }

    /// Applies tremolo modulation to a mono sample segment.
    ///
    /// `start_frame` is the absolute frame offset of `samples` within the
    /// complete stream. Passing the correct offset makes chunked processing
    /// bit-identical to whole-buffer processing for the same channel.
    ///
    /// # Panics
    ///
    /// Panics only on targets where a slice index cannot be represented as
    /// `u64`.
    pub fn process_mono_samples(
        self,
        samples: &mut [f32],
        sample_rate: SampleRate,
        start_frame: FrameCount,
    ) {
        let sample_rate = f64::from(sample_rate.as_u32());
        for (offset, sample) in samples.iter_mut().enumerate() {
            let frame = start_frame.as_u64()
                + u64::try_from(offset).expect("usize offset fits u64 on supported targets");
            *sample *= self.modulator(frame, sample_rate);
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "the public effect sample format is f32, and the SoX-ng phase formula is evaluated in f64 before rounding back to f32 samples"
    )]
    fn modulator(self, frame: u64, sample_rate: f64) -> f32 {
        if self.speed_hz == 0.0 {
            return 1.0;
        }

        let phase = self.speed_hz * frame as f64 / sample_rate;
        let half_depth = self.depth_fraction / 2.0;
        (1.0 - half_depth + half_depth * (std::f64::consts::TAU * phase).cos()) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::Tremolo;
    use crate::EffectError;

    #[test]
    fn zero_speed_preserves_input_at_any_depth() {
        let mut samples = [0.25, -0.5, 1.0];

        Tremolo::new(0.0, 100.0).unwrap().process_mono_samples(
            &mut samples,
            auralis_core::SampleRate::new(48_000).unwrap(),
            auralis_core::FrameCount::new(0),
        );

        assert_samples_close(&samples, &[0.25, -0.5, 1.0]);
    }

    #[test]
    fn quarter_cycle_reaches_mid_depth_and_half_cycle_reaches_minimum() {
        let tremolo = Tremolo::new(1.0, 40.0).unwrap();
        let mut samples = [1.0, 1.0, 1.0];

        tremolo.process_mono_samples(
            &mut samples,
            auralis_core::SampleRate::new(4).unwrap(),
            auralis_core::FrameCount::new(0),
        );

        assert_samples_close(&samples, &[1.0, 0.8, 0.6]);
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert_eq!(
            Tremolo::new(-0.1, 40.0).unwrap_err(),
            EffectError::InvalidTremolo
        );
        assert_eq!(
            Tremolo::new(f64::NAN, 40.0).unwrap_err(),
            EffectError::InvalidTremolo
        );
        assert_eq!(
            Tremolo::new(1.0, 0.0).unwrap_err(),
            EffectError::InvalidTremolo
        );
        assert_eq!(
            Tremolo::new(1.0, 100.1).unwrap_err(),
            EffectError::InvalidTremolo
        );
        assert_eq!(
            Tremolo::new(1.0, f64::INFINITY).unwrap_err(),
            EffectError::InvalidTremolo
        );
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= 0.000_001,
                "sample {index}: actual={actual}, expected={expected}"
            );
        }
    }
}
