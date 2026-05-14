use auralis_core::{AudioBuffer, AudioSpec, FrameCount, SampleRate};

use crate::{EffectError, Result};

/// Default SoX-ng `downsample` factor.
pub const DEFAULT_DOWNSAMPLE_FACTOR: u32 = 2;

/// Maximum SoX-ng `downsample` factor accepted by the command parser.
pub const MAX_DOWNSAMPLE_FACTOR: u32 = 16_384;

/// SoX-ng-style decimating downsample effect.
///
/// `Downsample` keeps the first frame of each factor-sized group and discards
/// the intervening frames without anti-alias filtering. It also updates the
/// buffer sample-rate metadata to `input_rate / factor`, matching the simple
/// rate-changing effect semantics rather than the higher-quality `rate`
/// resampler planned for later features.
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidDownsampleFactor`] for factors
/// outside SoX-ng's supported `1..=16384` range. [`Self::process_buffer`]
/// returns [`EffectError::DownsampleRateTooLow`] when the integer sample-rate
/// representation would become zero, or [`EffectError::Core`] if the output
/// buffer shape cannot be represented.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Downsample;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(5),
///     vec![0.0, 0.25, 0.5, 0.75, 1.0],
/// )?;
///
/// let downsampled = Downsample::new(2)?.process_buffer(&audio)?;
///
/// assert_eq!(downsampled.spec().sample_rate().as_u32(), 24_000);
/// assert_eq!(downsampled.as_planar_f32(), &[0.0, 0.5, 1.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Downsample {
    /// Integer decimation factor.
    pub factor: u32,
}

impl Downsample {
    /// Creates a decimating downsample processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidDownsampleFactor`] when `factor` is zero
    /// or greater than [`MAX_DOWNSAMPLE_FACTOR`].
    pub fn new(factor: u32) -> Result<Self> {
        if factor == 0 || factor > MAX_DOWNSAMPLE_FACTOR {
            return Err(EffectError::InvalidDownsampleFactor);
        }

        Ok(Self { factor })
    }

    /// Creates the SoX-ng default `downsample`, equivalent to `downsample 2`.
    #[must_use]
    pub const fn default_factor() -> Self {
        Self {
            factor: DEFAULT_DOWNSAMPLE_FACTOR,
        }
    }

    /// Applies decimating downsample to an audio buffer and returns the output.
    ///
    /// Factor `1` is an identity copy. Larger factors keep input frames
    /// `0, factor, 2 * factor, ...` for each channel and preserve planar
    /// channel grouping.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::DownsampleRateTooLow`] if the output sample rate
    /// cannot be represented as a positive integer, or [`EffectError::Core`] if
    /// the output buffer shape cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        if self.factor == 1 {
            return Ok(audio.clone());
        }

        let input_rate = audio.spec().sample_rate().as_u32();
        let output_rate = input_rate / self.factor;
        if output_rate == 0 {
            return Err(EffectError::DownsampleRateTooLow);
        }

        let output_frames = downsampled_frame_count(audio.frames(), self.factor);
        let output_frames_usize = usize::try_from(output_frames.as_u64())
            .map_err(|_| EffectError::Core(auralis_core::AuralisError::InvalidAudioBufferShape))?;
        let capacity = audio
            .channels()
            .as_usize()
            .checked_mul(output_frames_usize)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        let step = usize::try_from(self.factor)
            .map_err(|_| auralis_core::AuralisError::InvalidAudioBufferShape)?;
        let mut output = Vec::with_capacity(capacity);

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
            let mut frame = 0;
            while frame < channel.len() {
                output.push(channel[frame]);
                frame += step;
            }
        }

        debug_assert_eq!(output.len(), capacity);

        let spec = AudioSpec::new(
            SampleRate::new(output_rate)?,
            audio.channels(),
            audio.spec().sample_format(),
        );
        Ok(AudioBuffer::from_planar_f32(spec, output_frames, output)?)
    }
}

impl Default for Downsample {
    fn default() -> Self {
        Self::default_factor()
    }
}

fn downsampled_frame_count(input_frames: FrameCount, factor: u32) -> FrameCount {
    let frames = input_frames.as_u64();
    if frames == 0 {
        FrameCount::new(0)
    } else {
        FrameCount::new(((frames - 1) / u64::from(factor)) + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::{Downsample, MAX_DOWNSAMPLE_FACTOR};
    use crate::{
        EffectError,
        test_support::{audio_buffer, stereo_audio_buffer},
    };
    use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};

    #[test]
    fn decimates_each_planar_channel_and_updates_rate() {
        let audio = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 0.75, -0.25, -0.5]);

        let downsampled = Downsample::new(2).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(downsampled.frames(), FrameCount::new(2));
        assert_eq!(downsampled.spec().sample_rate().as_u32(), 24_000);
        assert_eq!(downsampled.as_planar_f32(), &[0.0, 0.5, 0.75, -0.5]);
    }

    #[test]
    fn keeps_final_partial_group_first_frame() {
        let audio = audio_buffer(vec![0.0, 0.25, 0.5, 0.75, 1.0]);

        let downsampled = Downsample::new(3).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(downsampled.frames(), FrameCount::new(2));
        assert_eq!(downsampled.spec().sample_rate().as_u32(), 16_000);
        assert_eq!(downsampled.as_planar_f32(), &[0.0, 0.75]);
    }

    #[test]
    fn factor_one_is_identity_copy() {
        let audio = audio_buffer(vec![-0.5, 0.0, 0.5]);

        let downsampled = Downsample::new(1).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(downsampled, audio);
    }

    #[test]
    fn rejects_factor_outside_sox_ng_range() {
        assert_eq!(
            Downsample::new(0).unwrap_err(),
            EffectError::InvalidDownsampleFactor
        );
        assert_eq!(
            Downsample::new(MAX_DOWNSAMPLE_FACTOR + 1).unwrap_err(),
            EffectError::InvalidDownsampleFactor
        );
    }

    #[test]
    fn rejects_zero_output_sample_rate() {
        let spec = AudioSpec::new(
            SampleRate::new(8_000).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        );
        let audio = auralis_core::AudioBuffer::from_planar_f32(spec, FrameCount::new(1), vec![0.0])
            .unwrap();

        assert_eq!(
            Downsample::new(16_384)
                .unwrap()
                .process_buffer(&audio)
                .unwrap_err(),
            EffectError::DownsampleRateTooLow
        );
    }
}
