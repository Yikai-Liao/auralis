use auralis_core::{AudioBuffer, AudioSpec, FrameCount, SampleRate};

use crate::{EffectError, Result};

/// Default SoX-ng `upsample` factor.
pub const DEFAULT_UPSAMPLE_FACTOR: u32 = 2;

/// Maximum SoX-ng `upsample` factor accepted by the command parser.
pub const MAX_UPSAMPLE_FACTOR: u32 = 256;

/// SoX-ng-style zero-stuffing upsample effect.
///
/// `Upsample` inserts `factor - 1` zero-valued frames after each input frame
/// and updates the buffer sample-rate metadata to `input_rate * factor`. It is
/// a simple rate-changing primitive, not a band-limited resampler; the
/// higher-quality `rate` command is planned as a later feature.
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidUpsampleFactor`] for factors
/// outside SoX-ng's supported `1..=256` range. [`Self::process_buffer`] returns
/// [`EffectError::UpsampleRateOverflow`] when the multiplied sample rate cannot
/// fit in Auralis' integer sample-rate representation, or
/// [`EffectError::Core`] if the output buffer shape cannot be represented.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Upsample;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(24_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(3),
///     vec![0.25, -0.5, 1.0],
/// )?;
///
/// let upsampled = Upsample::new(2)?.process_buffer(&audio)?;
///
/// assert_eq!(upsampled.spec().sample_rate().as_u32(), 48_000);
/// assert_eq!(upsampled.as_planar_f32(), &[0.25, 0.0, -0.5, 0.0, 1.0, 0.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Upsample {
    /// Integer zero-stuffing factor.
    pub factor: u32,
}

impl Upsample {
    /// Creates a zero-stuffing upsample processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidUpsampleFactor`] when `factor` is zero or
    /// greater than [`MAX_UPSAMPLE_FACTOR`].
    pub fn new(factor: u32) -> Result<Self> {
        if factor == 0 || factor > MAX_UPSAMPLE_FACTOR {
            return Err(EffectError::InvalidUpsampleFactor);
        }

        Ok(Self { factor })
    }

    /// Creates the SoX-ng default `upsample`, equivalent to `upsample 2`.
    #[must_use]
    pub const fn default_factor() -> Self {
        Self {
            factor: DEFAULT_UPSAMPLE_FACTOR,
        }
    }

    /// Applies zero-stuffing upsample to an audio buffer and returns the output.
    ///
    /// Factor `1` is an identity copy. Larger factors preserve each input frame
    /// then insert `factor - 1` silent frames per channel.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::UpsampleRateOverflow`] if the output sample rate
    /// cannot be represented as a `u32`, or [`EffectError::Core`] if the output
    /// buffer shape cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        if self.factor == 1 {
            return Ok(audio.clone());
        }

        let output_rate = audio
            .spec()
            .sample_rate()
            .as_u32()
            .checked_mul(self.factor)
            .ok_or(EffectError::UpsampleRateOverflow)?;
        let output_frames = upsampled_frame_count(audio.frames(), self.factor)?;
        let output_frames_usize = usize::try_from(output_frames.as_u64())
            .map_err(|_| EffectError::Core(auralis_core::AuralisError::InvalidAudioBufferShape))?;
        let capacity = audio
            .channels()
            .as_usize()
            .checked_mul(output_frames_usize)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        let factor = usize::try_from(self.factor)
            .map_err(|_| auralis_core::AuralisError::InvalidAudioBufferShape)?;
        let mut output = vec![0.0; capacity];

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
            let channel_start = channel_index
                .checked_mul(output_frames_usize)
                .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
            for (frame, &sample) in channel.iter().enumerate() {
                output[channel_start + frame * factor] = sample;
            }
        }

        let spec = AudioSpec::new(
            SampleRate::new(output_rate)?,
            audio.channels(),
            audio.spec().sample_format(),
        );
        Ok(AudioBuffer::from_planar_f32(spec, output_frames, output)?)
    }
}

impl Default for Upsample {
    fn default() -> Self {
        Self::default_factor()
    }
}

fn upsampled_frame_count(input_frames: FrameCount, factor: u32) -> Result<FrameCount> {
    input_frames
        .as_u64()
        .checked_mul(u64::from(factor))
        .map(FrameCount::new)
        .ok_or(EffectError::Core(
            auralis_core::AuralisError::InvalidAudioBufferShape,
        ))
}

#[cfg(test)]
mod tests {
    use super::{MAX_UPSAMPLE_FACTOR, Upsample};
    use crate::{
        EffectError,
        test_support::{audio_buffer, stereo_audio_buffer},
    };
    use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};

    #[test]
    fn inserts_zero_frames_per_planar_channel_and_updates_rate() {
        let audio = stereo_audio_buffer(vec![0.25, -0.5, 0.75, 1.0]);

        let upsampled = Upsample::new(2).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(upsampled.frames(), FrameCount::new(4));
        assert_eq!(upsampled.spec().sample_rate().as_u32(), 96_000);
        assert_eq!(
            upsampled.as_planar_f32(),
            &[0.25, 0.0, -0.5, 0.0, 0.75, 0.0, 1.0, 0.0]
        );
    }

    #[test]
    fn supports_explicit_factor_larger_than_two() {
        let audio = audio_buffer(vec![0.25, -0.5]);

        let upsampled = Upsample::new(3).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(upsampled.frames(), FrameCount::new(6));
        assert_eq!(upsampled.spec().sample_rate().as_u32(), 144_000);
        assert_eq!(upsampled.as_planar_f32(), &[0.25, 0.0, 0.0, -0.5, 0.0, 0.0]);
    }

    #[test]
    fn factor_one_is_identity_copy() {
        let audio = audio_buffer(vec![-0.5, 0.0, 0.5]);

        let upsampled = Upsample::new(1).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(upsampled, audio);
    }

    #[test]
    fn rejects_factor_outside_sox_ng_range() {
        assert_eq!(
            Upsample::new(0).unwrap_err(),
            EffectError::InvalidUpsampleFactor
        );
        assert_eq!(
            Upsample::new(MAX_UPSAMPLE_FACTOR + 1).unwrap_err(),
            EffectError::InvalidUpsampleFactor
        );
    }

    #[test]
    fn rejects_overflowed_output_sample_rate() {
        let spec = AudioSpec::new(
            SampleRate::new(u32::MAX).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        );
        let audio = auralis_core::AudioBuffer::from_planar_f32(spec, FrameCount::new(1), vec![0.0])
            .unwrap();

        assert_eq!(
            Upsample::new(2)
                .unwrap()
                .process_buffer(&audio)
                .unwrap_err(),
            EffectError::UpsampleRateOverflow
        );
    }
}
