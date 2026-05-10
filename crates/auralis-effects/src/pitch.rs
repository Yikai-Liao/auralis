use auralis_core::{AudioBuffer, AudioSpec, SampleRate};

use crate::{EffectError, Result, Tempo, TempoProfile};

/// SoX-ng-style pitch shift that preserves duration.
///
/// `Pitch` accepts a shift in cents. Positive values raise pitch and negative
/// values lower it. It mirrors SoX-ng's `pitch` effect by running the existing
/// overlap-search tempo core with the inverse cents factor, then changing the
/// output sample-rate metadata to the cents factor.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Pitch;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(4096),
///     (0_u16..4096).map(|frame| f32::from(frame % 128) / 128.0).collect(),
/// )?;
///
/// let shifted = Pitch::new(1200.0)?.process_buffer(&audio)?;
///
/// assert_eq!(shifted.spec().sample_rate().as_u32(), 96_000);
/// assert!(shifted.frames().as_u64() > audio.frames().as_u64());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidPitchShift`] when the cents
/// shift is not finite or maps outside SoX-ng's supported pitch factor range.
/// [`Self::with_tuning`] returns [`EffectError::InvalidPitchTuning`] for
/// segment/search/overlap values outside SoX-ng's accepted ranges.
/// [`Self::process_buffer`] returns [`EffectError::PitchRateOutOfRange`] when
/// the shifted output sample rate cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pitch {
    /// Pitch shift in cents.
    pub cents: f64,

    /// Whether to use SoX-ng's hierarchical quick overlap search.
    pub quick_search: bool,

    /// Explicit segment size in milliseconds.
    pub segment_ms: Option<f64>,

    /// Explicit overlap-search span in milliseconds.
    pub search_ms: Option<f64>,

    /// Explicit overlap size in milliseconds.
    pub overlap_ms: Option<f64>,
}

impl Pitch {
    /// Creates a default-profile `pitch shift` processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidPitchShift`] when `cents` is not finite or
    /// maps outside SoX-ng's supported pitch factor range.
    pub fn new(cents: f64) -> Result<Self> {
        Self::with_tuning(cents, false, None, None, None)
    }

    /// Creates a pitch processor with explicit SoX-ng tuning options.
    ///
    /// `segment_ms`, `search_ms`, and `overlap_ms` are optional positional
    /// values in milliseconds and use SoX-ng's tempo ranges: segment
    /// `10..=120`, search `0..=30`, and overlap `0..=30`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidPitchShift`] when `cents` is invalid or
    /// [`EffectError::InvalidPitchTuning`] when a supplied timing value is
    /// outside SoX-ng's supported range.
    pub fn with_tuning(
        cents: f64,
        quick_search: bool,
        segment_ms: Option<f64>,
        search_ms: Option<f64>,
        overlap_ms: Option<f64>,
    ) -> Result<Self> {
        let tempo_factor = pitch_tempo_factor(cents)?;
        validate_optional_range(segment_ms, 10.0, 120.0)?;
        validate_optional_range(search_ms, 0.0, 30.0)?;
        validate_optional_range(overlap_ms, 0.0, 30.0)?;

        Tempo::with_tuning(
            tempo_factor,
            quick_search,
            TempoProfile::Default,
            segment_ms,
            search_ms,
            overlap_ms,
        )
        .map_err(|_| EffectError::InvalidPitchTuning)?;

        Ok(Self {
            cents,
            quick_search,
            segment_ms,
            search_ms,
            overlap_ms,
        })
    }

    /// Applies pitch shifting while preserving approximate input duration.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::PitchRateOutOfRange`] when the shifted sample rate
    /// cannot be represented, or the same shape errors as the tempo core when
    /// the transformed buffer cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let factor = pitch_factor(self.cents)?;
        let tempo = Tempo::with_tuning(
            1.0 / factor,
            self.quick_search,
            TempoProfile::Default,
            self.segment_ms,
            self.search_ms,
            self.overlap_ms,
        )
        .map_err(|_| EffectError::InvalidPitchTuning)?;
        let shifted = tempo.process_buffer(audio)?;
        let output_rate = pitch_sample_rate(audio.spec().sample_rate(), factor)?;
        let spec = AudioSpec::new(
            output_rate,
            shifted.channels(),
            shifted.spec().sample_format(),
        );

        Ok(AudioBuffer::from_planar_f32(
            spec,
            shifted.frames(),
            shifted.as_planar_f32().to_vec(),
        )?)
    }
}

fn pitch_tempo_factor(cents: f64) -> Result<f64> {
    let factor = pitch_factor(cents)?;
    Ok(1.0 / factor)
}

fn pitch_factor(cents: f64) -> Result<f64> {
    if !cents.is_finite() {
        return Err(EffectError::InvalidPitchShift);
    }

    let factor = 2.0_f64.powf(cents / 1200.0);
    if !factor.is_finite() || !(0.01..=10.0).contains(&factor) {
        return Err(EffectError::InvalidPitchShift);
    }

    Ok(factor)
}

fn validate_optional_range(value: Option<f64>, min: f64, max: f64) -> Result<()> {
    if let Some(value) = value
        && (!value.is_finite() || !(min..=max).contains(&value))
    {
        return Err(EffectError::InvalidPitchTuning);
    }

    Ok(())
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "range and finiteness are validated before rounding into Auralis' integer sample-rate type"
)]
fn pitch_sample_rate(input_rate: SampleRate, factor: f64) -> Result<SampleRate> {
    let output = f64::from(input_rate.as_u32()) * factor;
    if !output.is_finite() || output < 1.0 || output > f64::from(u32::MAX) {
        return Err(EffectError::PitchRateOutOfRange);
    }

    let rounded = output.round() as u32;
    SampleRate::new(rounded).map_err(EffectError::Core)
}

#[cfg(test)]
mod tests {
    use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};

    use super::Pitch;
    use crate::EffectError;

    #[test]
    fn shifts_pitch_by_changing_frames_and_sample_rate() {
        let audio = audio_buffer(
            48_000,
            (0_u16..4096)
                .map(|frame| f32::from(frame) / 4096.0)
                .collect(),
        );
        let shifted = Pitch::new(1200.0).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(shifted.spec().sample_rate().as_u32(), 96_000);
        assert_eq!(shifted.frames().as_u64(), 8192);
        assert_eq!(shifted.channels(), audio.channels());
    }

    #[test]
    fn factor_zero_is_identity() {
        let audio = audio_buffer(48_000, vec![0.0, 0.25, -0.5]);
        let shifted = Pitch::new(0.0).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(shifted.spec().sample_rate(), audio.spec().sample_rate());
        assert_eq!(shifted.frames(), audio.frames());
        assert_eq!(shifted.as_planar_f32(), audio.as_planar_f32());
    }

    #[test]
    fn rejects_invalid_shift_tuning_and_output_rate() {
        assert_eq!(
            Pitch::new(5000.0).unwrap_err(),
            EffectError::InvalidPitchShift
        );
        assert_eq!(
            Pitch::with_tuning(0.0, false, Some(9.0), None, None).unwrap_err(),
            EffectError::InvalidPitchTuning
        );

        let audio = audio_buffer(u32::MAX, vec![0.0]);
        assert_eq!(
            Pitch::new(1200.0)
                .unwrap()
                .process_buffer(&audio)
                .unwrap_err(),
            EffectError::PitchRateOutOfRange
        );
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
