use auralis_core::{AudioBuffer, AudioSpec, ChannelCount};

use crate::{EffectError, Result};

/// SoX-ng-style out-of-phase stereo extraction.
///
/// `Oops` is SoX-ng's `oops` alias for `remix 1,2i 1,2i`: it subtracts the
/// second decoded channel from the first, clips the difference to normalized
/// full scale, and writes the same difference into both output channels. Extra
/// input channels are ignored. Inputs with fewer than two channels are rejected,
/// matching SoX-ng's "too few input channels" failure.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Oops;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(2)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(2),
///     vec![0.25, 0.75, -0.25, 0.5],
/// )?;
///
/// let output = Oops::new().process_buffer(&audio)?;
///
/// assert_eq!(output.as_planar_f32(), &[0.5, 0.25, 0.5, 0.25]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Oops;

impl Oops {
    /// Creates an out-of-phase stereo processor.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Applies out-of-phase stereo extraction and returns a stereo buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::RemixInputChannelOutOfBounds`] when the input has
    /// fewer than two channels, or [`EffectError::Core`] if the output buffer
    /// shape cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        if audio.channels().as_usize() < 2 {
            return Err(EffectError::RemixInputChannelOutOfBounds);
        }

        let output_channels = ChannelCount::new(2).map_err(EffectError::from)?;
        let output_spec = AudioSpec::new(
            audio.spec().sample_rate(),
            output_channels,
            audio.spec().sample_format(),
        );
        let frames = audio.frames();
        let frame_count = usize::try_from(frames.as_u64())
            .map_err(|_| auralis_core::AuralisError::InvalidAudioBufferShape)?;
        let left = audio
            .channel(0)
            .ok_or(EffectError::RemixInputChannelOutOfBounds)?;
        let right = audio
            .channel(1)
            .ok_or(EffectError::RemixInputChannelOutOfBounds)?;
        let sample_count = frame_count
            .checked_mul(2)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        let mut samples = Vec::with_capacity(sample_count);

        samples.extend(
            left.iter()
                .zip(right)
                .map(|(&left, &right)| (left - right).clamp(-1.0, 1.0)),
        );
        samples.extend_from_within(..frame_count);

        AudioBuffer::from_planar_f32(output_spec, frames, samples).map_err(EffectError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::Oops;
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn subtracts_right_from_left_and_duplicates_output() {
        let audio = audio_buffer(2, vec![0.25, 0.75, -0.25, 0.5]);

        let actual = Oops::new().process_buffer(&audio).unwrap();

        assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.as_planar_f32(), &[0.5, 0.25, 0.5, 0.25]);
    }

    #[test]
    fn clips_difference_and_ignores_extra_channels() {
        let audio = audio_buffer(3, vec![0.75, -0.75, -0.75, 0.75, 0.5, 0.5]);

        let actual = Oops::new().process_buffer(&audio).unwrap();

        assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(actual.as_planar_f32(), &[1.0, -1.0, 1.0, -1.0]);
    }

    #[test]
    fn rejects_mono_input() {
        let audio = audio_buffer(1, vec![0.25, -0.5]);

        assert_eq!(
            Oops::new().process_buffer(&audio).unwrap_err(),
            EffectError::RemixInputChannelOutOfBounds
        );
    }

    fn audio_buffer(channels: u16, samples: Vec<f32>) -> AudioBuffer {
        assert_eq!(samples.len() % usize::from(channels), 0);
        let frames = samples.len() / usize::from(channels);
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
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
}
