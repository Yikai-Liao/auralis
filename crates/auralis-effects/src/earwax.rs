use auralis_core::{AudioBuffer, AudioSpec};

use crate::{EffectError, Result};

const REQUIRED_SAMPLE_RATE: u32 = 44_100;
const REQUIRED_CHANNELS: u16 = 2;

// SoX-ng's earwax coefficient table is from a file-local permissive notice:
// "freely redistributable and may be used for any purpose".
const FILTER: [f64; 64] = [
    4.0, -6.0, 4.0, -11.0, -1.0, -5.0, 3.0, 3.0, -2.0, 5.0, -5.0, 0.0, 9.0, 1.0, 6.0, 3.0, -4.0,
    -1.0, -5.0, -3.0, -2.0, -5.0, -7.0, 1.0, 6.0, -7.0, 30.0, -29.0, 12.0, -3.0, -11.0, 4.0, -3.0,
    7.0, -20.0, 23.0, 2.0, 0.0, 1.0, -6.0, -14.0, -5.0, 15.0, -18.0, 6.0, 7.0, 15.0, -10.0, -14.0,
    22.0, -7.0, -2.0, -4.0, 9.0, 6.0, -12.0, 6.0, -6.0, 0.0, -11.0, 0.0, -5.0, 4.0, 0.0,
];

/// SoX-ng-style headphone-cue filter for CD audio.
///
/// `earwax` applies a fixed stereo FIR that moves headphone playback cues
/// outward and forward. SoX-ng accepts only stereo 44.1 kHz input for this
/// effect, so Auralis rejects all other channel and sample-rate shapes.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Earwax;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(44_100)?,
///     ChannelCount::new(2)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(2),
///     vec![1.0, 0.0, 0.0, 0.0],
/// )?;
/// Earwax::new().process_buffer(&mut audio)?;
///
/// assert_eq!(audio.as_planar_f32()[0], 4.0 / 64.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Earwax;

impl Earwax {
    /// Creates an `earwax` headphone-cue filter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Applies the fixed headphone-cue filter in place.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidEarwaxInput`] when the buffer is not
    /// stereo 44.1 kHz audio, or [`EffectError::Core`] if the output buffer
    /// shape cannot be represented.
    pub fn process_buffer(self, audio: &mut AudioBuffer) -> Result<()> {
        validate_shape(audio)?;

        let frames = frame_count(audio)?;
        let left = audio.channel(0).ok_or(EffectError::InvalidEarwaxInput)?;
        let right = audio.channel(1).ok_or(EffectError::InvalidEarwaxInput)?;
        let mut output = vec![0.0; audio.as_planar_f32().len()];
        let (output_left, output_right) = output.split_at_mut(frames);
        let mut state = EarwaxState::new();

        for frame in 0..frames {
            let (left_sample, right_sample) = state.process_frame(left[frame], right[frame]);
            output_left[frame] = left_sample;
            output_right[frame] = right_sample;
        }

        let spec = AudioSpec::new(
            audio.spec().sample_rate(),
            audio.spec().channels(),
            audio.spec().sample_format(),
        );
        *audio = AudioBuffer::from_planar_f32(spec, audio.frames(), output)?;
        Ok(())
    }
}

/// Streaming state for SoX-ng-style `earwax` processing.
#[derive(Debug, Clone, PartialEq)]
pub struct EarwaxState {
    taps: [f64; FILTER.len()],
}

impl EarwaxState {
    /// Creates zeroed filter delay state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            taps: [0.0; FILTER.len()],
        }
    }

    /// Processes one stereo frame and preserves state for the next frame.
    #[must_use]
    pub fn process_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        (self.process_sample(left), self.process_sample(right))
    }

    fn process_sample(&mut self, sample: f32) -> f32 {
        let mut output = 0.0;
        for index in (1..FILTER.len()).rev() {
            self.taps[index] = self.taps[index - 1];
            output += self.taps[index] * FILTER[index];
        }
        self.taps[0] = f64::from(sample) / 64.0;
        output += self.taps[0] * FILTER[0];

        f64_to_f32_clamped(output)
    }
}

impl Default for EarwaxState {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_shape(audio: &AudioBuffer) -> Result<()> {
    if audio.spec().sample_rate().as_u32() == REQUIRED_SAMPLE_RATE
        && audio.spec().channels().as_u16() == REQUIRED_CHANNELS
    {
        Ok(())
    } else {
        Err(EffectError::InvalidEarwaxInput)
    }
}

fn frame_count(audio: &AudioBuffer) -> Result<usize> {
    usize::try_from(audio.frames().as_u64())
        .map_err(|_| auralis_core::AuralisError::InvalidAudioBufferShape.into())
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "earwax clamps to normalized f32 full scale before storing sample output"
)]
fn f64_to_f32_clamped(sample: f64) -> f32 {
    sample.clamp(-1.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::{Earwax, EarwaxState};
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn applies_sox_ng_interleaved_filter_to_stereo_impulse() {
        let mut audio = stereo_44100(vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 0.0, 0.0, 0.0]);

        Earwax::new().process_buffer(&mut audio).unwrap();

        assert_sample_bits_eq(
            audio.as_planar_f32(),
            &[
                4.0 / 64.0,
                4.0 / 64.0,
                -1.0 / 64.0,
                3.0 / 64.0,
                -6.0 / 64.0,
                -11.0 / 64.0,
                -5.0 / 64.0,
                3.0 / 64.0,
            ],
        );
    }

    #[test]
    fn clips_large_filtered_samples() {
        let mut state = EarwaxState::new();
        let mut last = 0.0;

        for _ in 0..40 {
            let (left, right) = state.process_frame(1.0, -1.0);
            last = left.max(right);
        }

        assert!(last <= 1.0);
    }

    #[test]
    fn rejects_non_cd_stereo_shapes() {
        let mut wrong_rate = audio_buffer(48_000, 2, vec![0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            Earwax::new().process_buffer(&mut wrong_rate).unwrap_err(),
            EffectError::InvalidEarwaxInput
        );

        let mut mono = audio_buffer(44_100, 1, vec![0.0, 0.0]);
        assert_eq!(
            Earwax::new().process_buffer(&mut mono).unwrap_err(),
            EffectError::InvalidEarwaxInput
        );
    }

    fn stereo_44100(left: Vec<f32>, right: Vec<f32>) -> AudioBuffer {
        let mut samples = left;
        samples.extend(right);
        audio_buffer(44_100, 2, samples)
    }

    fn audio_buffer(sample_rate: u32, channels: u16, samples: Vec<f32>) -> AudioBuffer {
        let frames = samples.len() / usize::from(channels);
        let spec = AudioSpec::new(
            SampleRate::new(sample_rate).unwrap(),
            ChannelCount::new(channels).unwrap(),
            SampleFormat::Float32,
        );

        AudioBuffer::from_planar_f32(
            spec,
            FrameCount::new(u64::try_from(frames).unwrap()),
            samples,
        )
        .unwrap()
    }

    fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "sample {index}: actual={actual} expected={expected}"
            );
        }
    }
}
