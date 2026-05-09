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

use auralis_core::{AudioBuffer, Decibels};
use auralis_dsp::gain_in_place;

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

#[cfg(test)]
mod tests {
    use super::Gain;
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

    fn audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        let frames = samples.len().try_into().unwrap();
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(1).unwrap(),
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
