use auralis_core::{AudioBuffer, AudioSpec, FrameCount, SampleRate};

use crate::{EffectError, Result};

/// SoX-ng-style sample-rate conversion.
///
/// `Rate` changes decoded audio to an explicit target sample rate. SoX-ng
/// quality modes and SoX-ng override options are part of the typed command
/// model and currently share the deterministic scalar linear resampler; later
/// implementation work can use the typed metadata to select sharper pass-band
/// or aliasing behavior.
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rate {
    /// Target sample rate in frames per second.
    pub target_sample_rate: SampleRate,
    /// Requested SoX-ng quality family.
    pub quality: RateQuality,
    /// Requested SoX-ng rate control and override options.
    pub options: RateOptions,
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
    /// SoX-ng `-m` / `-Q 2` medium-quality mode.
    Medium,
    /// SoX-ng `-g` / `-Q 3` generic-quality mode.
    Generic,
    /// SoX-ng `-h` / `-Q 4` high-quality mode.
    High,
    /// SoX-ng `-e` / `-Q 5` extreme-quality mode.
    Extreme,
    /// SoX-ng `-v` / `-Q 6` very-high-quality mode.
    VeryHigh,
    /// SoX-ng `-u` / `-Q 7` ultra-quality mode.
    Ultra,
}

/// SoX-ng `rate` control and override options.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RateOptions {
    /// Forced interpolator coefficient order from `-i`.
    pub interpolator: Option<i8>,
    /// Coefficient-memory budget in KiB from `-c`.
    pub coefficient_budget_kib: Option<u32>,
    /// Compact flags for no-value SoX-ng `rate` options.
    pub flags: RateOptionFlags,
    /// Optional phase-response override from `-M`, `-I`, `-L`, or `-p`.
    pub phase: Option<RatePhase>,
    /// Optional pass-band override from `-s`, `-b`, or `-B`.
    pub bandwidth: RateBandwidth,
    /// Optional alias-free bandwidth percentage from `-A`.
    pub anti_aliasing_percent: Option<f32>,
    /// Optional accuracy override from `-d` or `-R`.
    pub precision: RatePrecision,
}

/// SoX-ng `rate` phase-response override.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RatePhase {
    /// Minimum phase, selected by `-M` or `-p 0`.
    Minimum,
    /// Intermediate phase, selected by `-I` or `-p 25`.
    Intermediate,
    /// Linear phase, selected by `-L` or `-p 50`.
    Linear,
    /// Maximum phase, selected by `-p 100`.
    Maximum,
    /// Custom phase percentage in `0..=100`.
    Percent(f32),
}

/// SoX-ng `rate` pass-band override.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RateBandwidth {
    /// Use the SoX-ng quality-mode default.
    #[default]
    Default,
    /// Steep 99% 3 dB bandwidth from `-s`.
    Steep,
    /// Explicit 3 dB bandwidth percentage from `-b`.
    ThreeDbPercent(f32),
    /// Explicit 0 dB pass-band percentage from `-B`.
    ZeroDbPercent(f32),
}

/// SoX-ng `rate` accuracy override.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RatePrecision {
    /// Use the SoX-ng quality-mode default.
    #[default]
    Default,
    /// Required bit accuracy from `-d`.
    BitDepth(f32),
    /// Required rejection in dB from `-R`.
    RejectionDb(f32),
}

/// Compact flags for no-value SoX-ng `rate` options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RateOptionFlags {
    bits: u8,
}

impl Rate {
    /// Creates a sample-rate conversion using the default quality mode.
    #[must_use]
    pub const fn new(target_sample_rate: SampleRate) -> Self {
        Self {
            target_sample_rate,
            quality: RateQuality::Default,
            options: RateOptions::DEFAULT,
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

    /// Creates a sample-rate conversion using SoX-ng `-m` medium-quality mode.
    #[must_use]
    pub const fn medium(target_sample_rate: SampleRate) -> Self {
        Self::with_quality(target_sample_rate, RateQuality::Medium)
    }

    /// Creates a sample-rate conversion using SoX-ng `-g` generic-quality mode.
    #[must_use]
    pub const fn generic(target_sample_rate: SampleRate) -> Self {
        Self::with_quality(target_sample_rate, RateQuality::Generic)
    }

    /// Creates a sample-rate conversion using SoX-ng `-h` high-quality mode.
    #[must_use]
    pub const fn high(target_sample_rate: SampleRate) -> Self {
        Self::with_quality(target_sample_rate, RateQuality::High)
    }

    /// Creates a sample-rate conversion using SoX-ng `-e` extreme-quality mode.
    #[must_use]
    pub const fn extreme(target_sample_rate: SampleRate) -> Self {
        Self::with_quality(target_sample_rate, RateQuality::Extreme)
    }

    /// Creates a sample-rate conversion using SoX-ng `-v` very-high-quality mode.
    #[must_use]
    pub const fn very_high(target_sample_rate: SampleRate) -> Self {
        Self::with_quality(target_sample_rate, RateQuality::VeryHigh)
    }

    /// Creates a sample-rate conversion using SoX-ng `-u` ultra-quality mode.
    #[must_use]
    pub const fn ultra(target_sample_rate: SampleRate) -> Self {
        Self::with_quality(target_sample_rate, RateQuality::Ultra)
    }

    /// Creates a sample-rate conversion with an explicit quality family.
    #[must_use]
    pub const fn with_quality(target_sample_rate: SampleRate, quality: RateQuality) -> Self {
        Self {
            target_sample_rate,
            quality,
            options: RateOptions::DEFAULT,
        }
    }

    /// Creates a sample-rate conversion with explicit SoX-ng control and
    /// override options.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidRateOptions`] when a quick or low-quality
    /// mode is combined with a SoX-ng override that only applies to medium or
    /// higher quality conversion, or when aliasing is combined with an
    /// out-of-range bandwidth override.
    pub fn with_options(
        target_sample_rate: SampleRate,
        quality: RateQuality,
        options: RateOptions,
    ) -> Result<Self> {
        options.validate_for_quality(quality)?;
        Ok(Self {
            target_sample_rate,
            quality,
            options,
        })
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

impl RateOptions {
    /// The default SoX-ng `rate` options.
    pub const DEFAULT: Self = Self {
        interpolator: None,
        coefficient_budget_kib: None,
        flags: RateOptionFlags::DEFAULT,
        phase: None,
        bandwidth: RateBandwidth::Default,
        anti_aliasing_percent: None,
        precision: RatePrecision::Default,
    };

    /// Returns true when any high-quality-only SoX-ng override is configured.
    #[must_use]
    pub fn has_high_quality_override(self) -> bool {
        self.phase.is_some()
            || !matches!(self.bandwidth, RateBandwidth::Default)
            || self.anti_aliasing_percent.is_some()
            || self.flags.allow_aliasing()
            || !matches!(self.precision, RatePrecision::Default)
    }

    fn validate_for_quality(self, quality: RateQuality) -> Result<()> {
        if self.has_high_quality_override()
            && matches!(quality, RateQuality::Quick | RateQuality::Low)
        {
            return Err(EffectError::InvalidRateOptions);
        }

        if self.flags.allow_aliasing() {
            match self.bandwidth {
                RateBandwidth::ThreeDbPercent(percent) if percent < 85.0 => {
                    return Err(EffectError::InvalidRateOptions);
                }
                RateBandwidth::ZeroDbPercent(percent) if percent < 74.0 => {
                    return Err(EffectError::InvalidRateOptions);
                }
                _ => {}
            }
        }

        Ok(())
    }
}

impl RateOptionFlags {
    const NO_ROLLOFF: u8 = 1 << 0;
    const NO_INTEGER_OPTIMIZATION: u8 = 1 << 1;
    const HIGH_PRECISION_CLOCK: u8 = 1 << 2;
    const ALLOW_ALIASING: u8 = 1 << 3;

    /// No compact rate option flags.
    pub const DEFAULT: Self = Self { bits: 0 };

    /// Returns a copy with SoX-ng `-f` enabled.
    #[must_use]
    pub const fn with_no_rolloff(mut self) -> Self {
        self.bits |= Self::NO_ROLLOFF;
        self
    }

    /// Returns a copy with SoX-ng `-n` enabled.
    #[must_use]
    pub const fn with_no_integer_optimization(mut self) -> Self {
        self.bits |= Self::NO_INTEGER_OPTIMIZATION;
        self
    }

    /// Returns a copy with SoX-ng `-t` enabled.
    #[must_use]
    pub const fn with_high_precision_clock(mut self) -> Self {
        self.bits |= Self::HIGH_PRECISION_CLOCK;
        self
    }

    /// Returns a copy with SoX-ng `-a` enabled.
    #[must_use]
    pub const fn with_allow_aliasing(mut self) -> Self {
        self.bits |= Self::ALLOW_ALIASING;
        self
    }

    /// Returns true when SoX-ng `-f` is enabled.
    #[must_use]
    pub const fn no_rolloff(self) -> bool {
        self.bits & Self::NO_ROLLOFF != 0
    }

    /// Returns true when SoX-ng `-n` is enabled.
    #[must_use]
    pub const fn no_integer_optimization(self) -> bool {
        self.bits & Self::NO_INTEGER_OPTIMIZATION != 0
    }

    /// Returns true when SoX-ng `-t` is enabled.
    #[must_use]
    pub const fn high_precision_clock(self) -> bool {
        self.bits & Self::HIGH_PRECISION_CLOCK != 0
    }

    /// Returns true when SoX-ng `-a` is enabled.
    #[must_use]
    pub const fn allow_aliasing(self) -> bool {
        self.bits & Self::ALLOW_ALIASING != 0
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
    use super::{
        Rate, RateBandwidth, RateOptionFlags, RateOptions, RatePhase, RatePrecision, RateQuality,
        converted_frame_count,
    };
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
    fn quality_modes_use_the_same_deterministic_scaffold() {
        let audio = audio_buffer(48_000, vec![0.0, 1.0, 0.0]);

        let baseline = Rate::quick(SampleRate::new(96_000).unwrap())
            .process_buffer(&audio)
            .unwrap();
        let expected = &[0.0, 0.5, 1.0, 0.5, 0.0, 0.0];
        assert_eq!(baseline.as_planar_f32(), expected);

        for quality in [
            RateQuality::Quick,
            RateQuality::Low,
            RateQuality::Medium,
            RateQuality::Generic,
            RateQuality::High,
            RateQuality::Extreme,
            RateQuality::VeryHigh,
            RateQuality::Ultra,
        ] {
            let rate = Rate::with_quality(SampleRate::new(96_000).unwrap(), quality);
            assert_eq!(rate.quality, quality);
            assert_eq!(
                rate.process_buffer(&audio).unwrap().as_planar_f32(),
                expected
            );
        }
    }

    #[test]
    fn override_options_are_typed_metadata_on_the_current_scaffold() {
        let audio = audio_buffer(48_000, vec![0.0, 1.0, 0.0]);
        let options = RateOptions {
            phase: Some(RatePhase::Minimum),
            bandwidth: RateBandwidth::Steep,
            precision: RatePrecision::RejectionDb(120.0),
            flags: RateOptionFlags::DEFAULT.with_allow_aliasing(),
            ..RateOptions::DEFAULT
        };
        let rate = Rate::with_options(SampleRate::new(96_000).unwrap(), RateQuality::High, options)
            .unwrap();

        assert_eq!(rate.options, options);
        assert_eq!(
            rate.process_buffer(&audio).unwrap().as_planar_f32(),
            &[0.0, 0.5, 1.0, 0.5, 0.0, 0.0]
        );
    }

    #[test]
    fn rejects_override_options_with_quick_or_low_quality() {
        let options = RateOptions {
            phase: Some(RatePhase::Linear),
            ..RateOptions::DEFAULT
        };

        assert_eq!(
            Rate::with_options(
                SampleRate::new(48_000).unwrap(),
                RateQuality::Quick,
                options
            )
            .unwrap_err(),
            EffectError::InvalidRateOptions
        );
        assert_eq!(
            Rate::with_options(SampleRate::new(48_000).unwrap(), RateQuality::Low, options)
                .unwrap_err(),
            EffectError::InvalidRateOptions
        );
    }

    #[test]
    fn rejects_aliasing_with_too_narrow_bandwidth_overrides() {
        let three_db = RateOptions {
            bandwidth: RateBandwidth::ThreeDbPercent(84.9),
            flags: RateOptionFlags::DEFAULT.with_allow_aliasing(),
            ..RateOptions::DEFAULT
        };
        let zero_db = RateOptions {
            bandwidth: RateBandwidth::ZeroDbPercent(73.9),
            flags: RateOptionFlags::DEFAULT.with_allow_aliasing(),
            ..RateOptions::DEFAULT
        };

        for options in [three_db, zero_db] {
            assert_eq!(
                Rate::with_options(SampleRate::new(48_000).unwrap(), RateQuality::High, options)
                    .unwrap_err(),
                EffectError::InvalidRateOptions
            );
        }
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
