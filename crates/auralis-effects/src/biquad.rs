use auralis_core::AudioBuffer;

use crate::{EffectError, Result};

/// Normalized direct-form biquad coefficients.
///
/// Coefficients are stored with `a0 = 1`, so processing follows:
///
/// `y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]`.
///
/// The primitive is deterministic, scalar, and stateful. Use
/// [`BiquadState`] when processing a long stream in chunks so the delay state
/// is preserved between calls.
///
/// # Examples
///
/// ```
/// use auralis_effects::{Biquad, BiquadCoefficients};
///
/// let filter = Biquad::new(BiquadCoefficients::normalized(0.5, 0.0, 0.0, -0.5, 0.0)?);
/// let mut samples = [1.0, 0.0, 0.0, 0.0];
/// filter.process_mono_samples(&mut samples);
///
/// assert_eq!(samples, [0.5, 0.25, 0.125, 0.0625]);
/// # Ok::<(), auralis_effects::EffectError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoefficients {
    /// Feed-forward coefficient for the current input sample.
    pub b0: f64,

    /// Feed-forward coefficient for the previous input sample.
    pub b1: f64,

    /// Feed-forward coefficient for the input sample two frames ago.
    pub b2: f64,

    /// Feedback coefficient for the previous output sample.
    pub a1: f64,

    /// Feedback coefficient for the output sample two frames ago.
    pub a2: f64,
}

impl BiquadCoefficients {
    /// Creates normalized coefficients with `a0 = 1`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadCoefficients`] if any coefficient is
    /// NaN or infinite.
    pub fn normalized(b0: f64, b1: f64, b2: f64, a1: f64, a2: f64) -> Result<Self> {
        let coefficients = Self { b0, b1, b2, a1, a2 };
        if coefficients.all_finite() {
            Ok(coefficients)
        } else {
            Err(EffectError::InvalidBiquadCoefficients)
        }
    }

    /// Creates coefficients from the conventional raw `b0 b1 b2 a0 a1 a2`
    /// form by dividing every coefficient by `a0`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadCoefficients`] if any coefficient is
    /// NaN or infinite, or if `a0` is zero.
    pub fn from_raw(b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64) -> Result<Self> {
        if ![b0, b1, b2, a0, a1, a2]
            .iter()
            .all(|value| value.is_finite())
            || a0.to_bits() == 0.0_f64.to_bits()
        {
            return Err(EffectError::InvalidBiquadCoefficients);
        }

        Self::normalized(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
    }

    /// Creates an identity coefficient set.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    fn all_finite(self) -> bool {
        self.b0.is_finite()
            && self.b1.is_finite()
            && self.b2.is_finite()
            && self.a1.is_finite()
            && self.a2.is_finite()
    }
}

/// Scalar direct-form biquad processor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Biquad {
    coefficients: BiquadCoefficients,
}

impl Biquad {
    /// Creates a biquad processor from normalized coefficients.
    #[must_use]
    pub const fn new(coefficients: BiquadCoefficients) -> Self {
        Self { coefficients }
    }

    /// Returns the normalized coefficients used by this processor.
    #[must_use]
    pub const fn coefficients(self) -> BiquadCoefficients {
        self.coefficients
    }

    /// Applies the biquad independently to every channel in an audio buffer.
    ///
    /// # Panics
    ///
    /// Panics only if a validated [`AudioBuffer`] cannot return one of its
    /// declared channels.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel_mut(channel_index)
                .expect("channel index is within the audio shape");
            self.process_mono_samples(channel);
        }
    }

    /// Applies the biquad to one mono sample slice with a fresh zero state.
    pub fn process_mono_samples(self, samples: &mut [f32]) {
        let mut state = BiquadState::new(self.coefficients);
        state.process_mono_samples(samples);
    }
}

impl Default for Biquad {
    fn default() -> Self {
        Self::new(BiquadCoefficients::identity())
    }
}

/// Stateful scalar runtime for one biquad-filtered channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadState {
    coefficients: BiquadCoefficients,
    delay_1: f64,
    delay_2: f64,
}

impl BiquadState {
    /// Creates a zero-initialized runtime state for one channel.
    #[must_use]
    pub const fn new(coefficients: BiquadCoefficients) -> Self {
        Self {
            coefficients,
            delay_1: 0.0,
            delay_2: 0.0,
        }
    }

    /// Returns the normalized coefficients used by this state.
    #[must_use]
    pub const fn coefficients(self) -> BiquadCoefficients {
        self.coefficients
    }

    /// Resets the delay state to zero while keeping the same coefficients.
    pub const fn reset(&mut self) {
        self.delay_1 = 0.0;
        self.delay_2 = 0.0;
    }

    /// Processes one sample and updates the filter state.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "Auralis effect samples are f32 while filter state is accumulated in f64 for deterministic scalar precision"
    )]
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        let input = f64::from(sample);
        let output = self.coefficients.b0.mul_add(input, self.delay_1);
        self.delay_1 = self
            .coefficients
            .b1
            .mul_add(input, self.delay_2 - (self.coefficients.a1 * output));
        self.delay_2 = self
            .coefficients
            .b2
            .mul_add(input, -(self.coefficients.a2 * output));
        output as f32
    }

    /// Applies the biquad to a mono sample segment while preserving state.
    pub fn process_mono_samples(&mut self, samples: &mut [f32]) {
        for sample in samples {
            *sample = self.process_sample(*sample);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Biquad, BiquadCoefficients, BiquadState};
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn identity_coefficients_preserve_sample_bits() {
        let mut samples = [0.25, -0.5, 0.0, 1.0];

        Biquad::default().process_mono_samples(&mut samples);

        assert_sample_bits_eq(&samples, &[0.25, -0.5, 0.0, 1.0]);
    }

    #[test]
    fn one_pole_impulse_response_matches_difference_equation() {
        let filter = Biquad::new(BiquadCoefficients::normalized(0.5, 0.0, 0.0, -0.5, 0.0).unwrap());
        let mut samples = [1.0, 0.0, 0.0, 0.0, 0.0];

        filter.process_mono_samples(&mut samples);

        assert_samples_close(&samples, &[0.5, 0.25, 0.125, 0.0625, 0.03125]);
    }

    #[test]
    fn raw_coefficients_are_normalized_by_a0() {
        let normalized = BiquadCoefficients::from_raw(2.0, 1.0, 0.5, 4.0, -1.0, 0.25).unwrap();

        assert_eq!(
            normalized,
            BiquadCoefficients::normalized(0.5, 0.25, 0.125, -0.25, 0.0625).unwrap()
        );
    }

    #[test]
    fn invalid_coefficients_are_rejected() {
        assert_eq!(
            BiquadCoefficients::normalized(f64::NAN, 0.0, 0.0, 0.0, 0.0).unwrap_err(),
            EffectError::InvalidBiquadCoefficients
        );
        assert_eq!(
            BiquadCoefficients::from_raw(1.0, 0.0, 0.0, 0.0, 0.0, 0.0).unwrap_err(),
            EffectError::InvalidBiquadCoefficients
        );
        assert_eq!(
            BiquadCoefficients::from_raw(1.0, 0.0, 0.0, f64::INFINITY, 0.0, 0.0).unwrap_err(),
            EffectError::InvalidBiquadCoefficients
        );
    }

    #[test]
    fn stereo_buffer_uses_independent_channel_state() {
        let mut audio = stereo_audio_buffer(vec![1.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0]);
        let filter = Biquad::new(BiquadCoefficients::normalized(0.5, 0.0, 0.0, -0.5, 0.0).unwrap());

        filter.process_buffer(&mut audio);

        assert_samples_close(
            audio.as_planar_f32(),
            &[0.5, 0.25, 0.125, 0.0625, 0.25, 0.125, 0.0625, 0.03125],
        );
    }

    #[test]
    fn preserving_state_across_chunks_matches_whole_slice() {
        let coefficients = BiquadCoefficients::normalized(0.5, 0.0, 0.0, -0.5, 0.0).unwrap();
        let filter = Biquad::new(coefficients);
        let mut whole = [1.0, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0];
        let mut chunked = whole;

        filter.process_mono_samples(&mut whole);

        let mut state = BiquadState::new(coefficients);
        state.process_mono_samples(&mut chunked[..3]);
        state.process_mono_samples(&mut chunked[3..5]);
        state.process_mono_samples(&mut chunked[5..]);

        assert_sample_bits_eq(&chunked, &whole);
    }

    fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(2).unwrap(),
            SampleFormat::Float32,
        );
        AudioBuffer::from_planar_f32(spec, FrameCount::new(4), samples).unwrap()
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

    fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "sample {index}: actual={actual}, expected={expected}"
            );
        }
    }
}
