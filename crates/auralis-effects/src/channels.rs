use auralis_core::{AudioBuffer, AudioSpec, ChannelCount};
use auralis_simd::BackendKind;

use crate::{EffectError, Result};

/// SoX-ng-style explicit channel-count conversion.
///
/// `Channels` preserves sample rate, sample format, and frame count while
/// changing the decoded planar channel layout. Matching channel counts are an
/// identity copy. Downmixing averages deterministic input-channel groups in
/// the same pattern as SoX-ng's `channels` effect, while upmixing duplicates
/// input channels in round-robin order.
///
/// # Errors
///
/// [`Self::process_buffer`] returns [`EffectError::Core`] if the output buffer
/// shape cannot be represented, or [`EffectError::ChannelMix`] if the
/// backend-dispatched downmix kernel rejects generated channel slices.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Channels;
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
/// let stereo = Channels::new(ChannelCount::new(2)?).process_buffer(&audio)?;
///
/// assert_eq!(stereo.as_planar_f32(), &[0.25, -0.5, 0.25, -0.5]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channels {
    /// Target output channel count.
    pub target_channels: ChannelCount,
}

impl Channels {
    /// Creates an explicit channel-count converter.
    #[must_use]
    pub const fn new(target_channels: ChannelCount) -> Self {
        Self { target_channels }
    }

    /// Converts `audio` using the scalar reference backend.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::process_buffer_with_backend`].
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        self.process_buffer_with_backend(audio, BackendKind::Scalar)
    }

    /// Converts `audio` using a requested backend for downmix averaging.
    ///
    /// Upmixing is a structural channel copy and does not select a SIMD kernel.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::Core`] if the converted buffer shape cannot be
    /// represented, or [`EffectError::ChannelMix`] if a backend-dispatched
    /// downmix rejects generated channel slices.
    pub fn process_buffer_with_backend(
        self,
        audio: &AudioBuffer,
        requested_backend: BackendKind,
    ) -> Result<AudioBuffer> {
        let input_channels = audio.channels();
        if input_channels == self.target_channels {
            return Ok(audio.clone());
        }

        let output_spec = AudioSpec::new(
            audio.spec().sample_rate(),
            self.target_channels,
            audio.spec().sample_format(),
        );
        let mut output = AudioBuffer::zeroed(output_spec, audio.frames())?;

        if input_channels < self.target_channels {
            duplicate_channels(audio, &mut output)?;
        } else {
            downmix_channels_with_backend(audio, &mut output, requested_backend)?;
        }

        Ok(output)
    }
}

fn duplicate_channels(input: &AudioBuffer, output: &mut AudioBuffer) -> Result<()> {
    let input_channels = input.channels().as_usize();
    for output_channel_index in 0..output.channels().as_usize() {
        let input_channel_index = output_channel_index % input_channels;
        let input_channel = input
            .channel(input_channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        let output_channel = output
            .channel_mut(output_channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        output_channel.copy_from_slice(input_channel);
    }

    Ok(())
}

fn downmix_channels_with_backend(
    input: &AudioBuffer,
    output: &mut AudioBuffer,
    requested_backend: BackendKind,
) -> Result<()> {
    let input_channels = input.channels().as_usize();
    let output_channels = output.channels().as_usize();
    let selection = auralis_simd::select_backend(requested_backend);

    for output_channel_index in 0..output_channels {
        let input_channels_per_output =
            (input_channels + output_channels - 1 - output_channel_index) / output_channels;
        let mut channel_inputs = Vec::with_capacity(input_channels_per_output);
        for input_group_index in 0..input_channels_per_output {
            let input_channel_index = input_group_index * output_channels + output_channel_index;
            channel_inputs.push(
                input
                    .channel(input_channel_index)
                    .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?,
            );
        }

        let output_channel = output
            .channel_mut(output_channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        auralis_simd::mix_f32_with_backend(
            selection,
            &channel_inputs,
            output_channel,
            reciprocal_usize(input_channels_per_output),
        )
        .map_err(EffectError::ChannelMix)?;
    }

    Ok(())
}

fn reciprocal_usize(value: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "Channel conversion scales small channel groups as f32 sample arithmetic."
    )]
    {
        1.0 / value as f32
    }
}

#[cfg(test)]
mod tests {
    use super::Channels;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };
    use auralis_simd::BackendKind;

    #[test]
    fn downmixes_three_channels_with_sox_grouping() {
        let audio = audio_buffer(3, vec![1.0, 3.0, 10.0, 30.0, 100.0, 300.0]);

        let actual = Channels::new(ChannelCount::new(2).unwrap())
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.as_planar_f32(), &[50.5, 151.5, 10.0, 30.0]);
    }

    #[test]
    fn duplicates_mono_to_stereo_round_robin() {
        let audio = audio_buffer(1, vec![0.25, -0.5, 0.75]);

        let actual = Channels::new(ChannelCount::new(2).unwrap())
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(
            actual.as_planar_f32(),
            &[0.25, -0.5, 0.75, 0.25, -0.5, 0.75]
        );
    }

    #[test]
    fn matching_channel_count_is_identity_copy() {
        let audio = audio_buffer(2, vec![0.25, -0.5, 0.75, 0.0]);

        let actual = Channels::new(ChannelCount::new(2).unwrap())
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(actual, audio);
    }

    #[test]
    fn requested_simd_downmix_matches_scalar() {
        let audio = audio_buffer(4, vec![1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0]);
        let target = ChannelCount::new(2).unwrap();

        let scalar = Channels::new(target)
            .process_buffer_with_backend(&audio, BackendKind::Scalar)
            .unwrap();
        let simd = Channels::new(target)
            .process_buffer_with_backend(&audio, BackendKind::Simd)
            .unwrap();

        assert_eq!(simd, scalar);
    }

    fn audio_buffer(channels: u16, samples: Vec<f32>) -> AudioBuffer {
        let channels = ChannelCount::new(channels).unwrap();
        let frames = samples.len() / channels.as_usize();
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            channels,
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
