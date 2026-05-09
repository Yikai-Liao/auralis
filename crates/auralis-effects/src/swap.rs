use auralis_core::AudioBuffer;

/// SoX-ng-style adjacent channel-pair swap.
///
/// `Swap` exchanges channels `1 <-> 2`, `3 <-> 4`, and so on, using SoX-ng's
/// one-based command semantics. Mono input and the final channel of an odd
/// channel-count input are left unchanged. The transform preserves sample
/// rate, sample format, frame count, and channel count.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Swap;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(2)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(2),
///     vec![0.25, 0.5, -0.25, -0.5],
/// )?;
///
/// Swap::new().process_buffer(&mut audio);
///
/// assert_eq!(audio.as_planar_f32(), &[-0.25, -0.5, 0.25, 0.5]);
/// # Ok::<(), auralis_core::AuralisError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Swap;

impl Swap {
    /// Creates an adjacent channel-pair swap processor.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Swaps adjacent channel pairs in place.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        let channels = audio.channels().as_usize();
        if channels < 2 {
            return;
        }

        let frames = audio.as_planar_f32().len() / channels;
        let samples = audio.as_planar_f32_mut();

        for left_channel in (0..channels - 1).step_by(2) {
            let right_channel = left_channel + 1;
            for frame in 0..frames {
                samples.swap(
                    left_channel * frames + frame,
                    right_channel * frames + frame,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Swap;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn swaps_stereo_channel_pair() {
        let mut audio = audio_buffer_with_channels(2, vec![0.25, 0.5, -0.25, -0.5]);

        Swap::new().process_buffer(&mut audio);

        assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(audio.frames(), FrameCount::new(2));
        assert_eq!(audio.as_planar_f32(), &[-0.25, -0.5, 0.25, 0.5]);
    }

    #[test]
    fn leaves_mono_unchanged() {
        let mut audio = audio_buffer_with_channels(1, vec![0.25, -0.5, 0.75]);

        Swap::new().process_buffer(&mut audio);

        assert_eq!(audio.as_planar_f32(), &[0.25, -0.5, 0.75]);
    }

    #[test]
    fn swaps_pairs_and_preserves_odd_trailing_channel() {
        let mut audio =
            audio_buffer_with_channels(5, vec![1.0, 1.1, 2.0, 2.1, 3.0, 3.1, 4.0, 4.1, 5.0, 5.1]);

        Swap::new().process_buffer(&mut audio);

        assert_eq!(
            audio.as_planar_f32(),
            &[2.0, 2.1, 1.0, 1.1, 4.0, 4.1, 3.0, 3.1, 5.0, 5.1]
        );
    }

    #[test]
    fn swap_twice_is_identity() {
        let source = audio_buffer_with_channels(4, vec![1.0, 1.1, 2.0, 2.1, 3.0, 3.1, 4.0, 4.1]);
        let mut actual = source.clone();

        Swap::new().process_buffer(&mut actual);
        Swap::new().process_buffer(&mut actual);

        assert_eq!(actual, source);
    }

    fn audio_buffer_with_channels(channels: u16, samples: Vec<f32>) -> AudioBuffer {
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
