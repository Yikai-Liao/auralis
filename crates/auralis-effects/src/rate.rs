use auralis_core::{AudioBuffer, AudioSpec, FrameCount, SampleRate};

use crate::{EffectError, Result};

/// SoX-ng-style sample-rate conversion.
///
/// `Rate` changes decoded audio to an explicit target sample rate. Quick and
/// low-quality modes are part of the typed command model and currently share
/// the deterministic scalar linear resampler; later `rate` features own the
/// higher-quality filter families, override options, and sharper pass-band or
/// aliasing behavior.
///
/// # Errors
///
/// [`Self::process_buffer`] returns [`EffectError::RateLengthOverflow`] when
/// the converted frame count cannot be represented, or [`EffectError::Core`]
/// if the output buffer shape cannot be represented.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Rate;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(3),
///     vec![0.0, 0.5, 1.0],
/// )?;
///
/// let converted = Rate::quick(SampleRate::new(96_000)?).process_buffer(&audio)?;
///
/// assert_eq!(converted.spec().sample_rate().as_u32(), 96_000);
/// assert_eq!(converted.frames(), FrameCount::new(6));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    /// Target sample rate in frames per second.
    pub target_sample_rate: SampleRate,
    /// Requested SoX-ng quality family.
    pub quality: RateQuality,
}

/// SoX-ng `rate` quality modes implemented by Auralis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateQuality {
    /// Default Auralis scaffold quality, selected when no SoX-ng quality flag
    /// is present.
    Default,
    /// SoX-ng `-q` / `-Q 0` quick mode.
    Quick,
    /// SoX-ng `-l` / `-Q 1` low-quality mode.
    Low,
}

impl Rate {
    /// Creates a sample-rate conversion using the default quality mode.
    #[must_use]
    pub const fn new(target_sample_rate: SampleRate) -> Self {
        Self {
            target_sample_rate,
            quality: RateQuality::Default,
        }
    }

    /// Creates a sample-rate conversion using SoX-ng `-q` quick mode.
    #[must_use]
    pub const fn quick(target_sample_rate: SampleRate) -> Self {
        Self::with_quality(target_sample_rate, RateQuality::Quick)
    }

    /// Creates a sample-rate conversion using SoX-ng `-l` low-quality mode.
    #[must_use]
    pub const fn low(target_sample_rate: SampleRate) -> Self {
        Self::with_quality(target_sample_rate, RateQuality::Low)
    }

    /// Creates a sample-rate conversion with an explicit quality family.
    #[must_use]
    pub const fn with_quality(target_sample_rate: SampleRate, quality: RateQuality) -> Self {
        Self {
            target_sample_rate,
            quality,
        }
    }

    /// Converts `audio` to the configured target sample rate.
    ///
    /// Matching rates return an identity copy. Other rates preserve channel
    /// count and sample format, compute `round(input_frames * target / source)`
    /// output frames, and sample each channel with linear interpolation at
    /// `output_frame * source_rate / target_rate`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::RateLengthOverflow`] when the converted frame
    /// count cannot fit in `u64`, or [`EffectError::Core`] if the output buffer
    /// shape cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let input_sample_rate = audio.spec().sample_rate();
        if input_sample_rate == self.target_sample_rate {
            return Ok(audio.clone());
        }

        let output_frames =
            converted_frame_count(audio.frames(), input_sample_rate, self.target_sample_rate)?;
        let output_spec = AudioSpec::new(
            self.target_sample_rate,
            audio.channels(),
            audio.spec().sample_format(),
        );
        let mut output = AudioBuffer::zeroed(output_spec, output_frames)?;

        for channel_index in 0..audio.channels().as_usize() {
            let input_channel = audio
                .channel(channel_index)
                .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
            let output_channel = output
                .channel_mut(channel_index)
                .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
            resample_channel_linear(
                input_channel,
                output_channel,
                input_sample_rate,
                self.target_sample_rate,
            );
        }

        Ok(output)
    }
}

fn converted_frame_count(
    frames: FrameCount,
    source: SampleRate,
    target: SampleRate,
) -> Result<FrameCount> {
    let frame_count = frames.as_u64();
    if frame_count == 0 {
        return Ok(FrameCount::new(0));
    }

    let numerator = u128::from(frame_count)
        .checked_mul(u128::from(target.as_u32()))
        .ok_or(EffectError::RateLengthOverflow)?;
    let denominator = u128::from(source.as_u32());
    let rounded = numerator
        .checked_add(denominator / 2)
        .ok_or(EffectError::RateLengthOverflow)?
        / denominator;
    let output_frames = u64::try_from(rounded).map_err(|_| EffectError::RateLengthOverflow)?;

    Ok(FrameCount::new(output_frames))
}

fn resample_channel_linear(
    input: &[f32],
    output: &mut [f32],
    source: SampleRate,
    target: SampleRate,
) {
    if input.is_empty() || output.is_empty() {
        return;
    }

    let last_input_index = input.len() - 1;
    let source = f64::from(source.as_u32());
    let target = f64::from(target.as_u32());
    for (output_index, output_sample) in output.iter_mut().enumerate() {
        #[allow(
            clippy::cast_precision_loss,
            reason = "Sample-rate conversion maps usize frame positions into f64 time coordinates."
        )]
        let source_position = output_index as f64 * source / target;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "source_position is finite and non-negative because sample rates are positive."
        )]
        let source_floor_index = source_position.floor() as usize;

        if source_floor_index >= last_input_index {
            *output_sample = input[last_input_index];
            continue;
        }

        #[allow(
            clippy::cast_precision_loss,
            reason = "The source frame index is subtracted in the same f64 coordinate system used for interpolation."
        )]
        let source_floor = source_floor_index as f64;
        let fraction = source_position - source_floor;
        #[allow(
            clippy::cast_possible_truncation,
            reason = "The interpolation fraction is in 0.0..1.0 and represented as f32 sample arithmetic."
        )]
        let fraction = fraction as f32;
        let left = input[source_floor_index];
        let right = input[source_floor_index + 1];
        *output_sample = left.mul_add(1.0 - fraction, right * fraction);
    }
}

#[cfg(test)]
mod tests {
    use super::{Rate, RateQuality, converted_frame_count};
    use crate::EffectError;
    use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};

    #[test]
    fn upsamples_with_linear_interpolation_and_updates_rate() {
        let audio = audio_buffer(48_000, vec![0.0, 1.0, 0.0]);

        let converted = Rate::new(SampleRate::new(96_000).unwrap())
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(converted.spec().sample_rate().as_u32(), 96_000);
        assert_eq!(converted.frames(), FrameCount::new(6));
        assert_eq!(converted.as_planar_f32(), &[0.0, 0.5, 1.0, 0.5, 0.0, 0.0]);
    }

    #[test]
    fn downsamples_with_linear_sampling_positions() {
        let audio = audio_buffer(48_000, vec![0.0, 0.25, 0.5, 0.75, 1.0]);

        let converted = Rate::new(SampleRate::new(24_000).unwrap())
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(converted.spec().sample_rate().as_u32(), 24_000);
        assert_eq!(converted.frames(), FrameCount::new(3));
        assert_eq!(converted.as_planar_f32(), &[0.0, 0.5, 1.0]);
    }

    #[test]
    fn matching_rate_is_identity_copy() {
        let audio = audio_buffer(48_000, vec![-0.5, 0.0, 0.5]);

        let converted = Rate::new(SampleRate::new(48_000).unwrap())
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(converted, audio);
    }

    #[test]
    fn quick_and_low_modes_use_the_same_deterministic_scaffold() {
        let audio = audio_buffer(48_000, vec![0.0, 1.0, 0.0]);

        let quick = Rate::quick(SampleRate::new(96_000).unwrap())
            .process_buffer(&audio)
            .unwrap();
        let low = Rate::low(SampleRate::new(96_000).unwrap())
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(
            Rate::quick(SampleRate::new(96_000).unwrap()).quality,
            RateQuality::Quick
        );
        assert_eq!(
            Rate::low(SampleRate::new(96_000).unwrap()).quality,
            RateQuality::Low
        );
        assert_eq!(quick.as_planar_f32(), &[0.0, 0.5, 1.0, 0.5, 0.0, 0.0]);
        assert_eq!(low.as_planar_f32(), quick.as_planar_f32());
    }

    #[test]
    fn detects_unrepresentable_output_frame_count() {
        let error = converted_frame_count(
            FrameCount::new(u64::MAX),
            SampleRate::new(1).unwrap(),
            SampleRate::new(u32::MAX).unwrap(),
        )
        .unwrap_err();

        assert_eq!(error, EffectError::RateLengthOverflow);
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
