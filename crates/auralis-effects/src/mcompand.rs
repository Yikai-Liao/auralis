use std::f64::consts::FRAC_1_SQRT_2;

use auralis_core::{AudioBuffer, FrameCount};

use crate::{BiquadCoefficients, BiquadState, Compand, EffectError, Result};

/// One SoX-ng `mcompand` band.
#[derive(Debug, Clone, PartialEq)]
pub struct MCompandBand {
    compand: Compand,
    top_frequency_hz: Option<f64>,
}

impl MCompandBand {
    /// Creates a multiband compander band.
    ///
    /// `top_frequency_hz` is the crossover frequency to the next band. The
    /// final band must use `None`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidMCompand`] when a crossover frequency is
    /// not finite and positive, or when the embedded compander uses a nonzero
    /// delay. SoX-ng parses such delays but its mcompand flow path rejects them
    /// at runtime, so Auralis reports the unsupported shape before processing.
    pub fn new(compand: Compand, top_frequency_hz: Option<f64>) -> Result<Self> {
        if top_frequency_hz.is_some_and(|frequency| !frequency.is_finite() || frequency <= 0.0)
            || compand.delay_seconds != 0.0
        {
            return Err(EffectError::InvalidMCompand);
        }

        Ok(Self {
            compand,
            top_frequency_hz,
        })
    }

    /// Returns this band's compander.
    #[must_use]
    pub const fn compand(&self) -> &Compand {
        &self.compand
    }

    /// Returns this band's upper crossover frequency, if it has one.
    #[must_use]
    pub const fn top_frequency_hz(&self) -> Option<f64> {
        self.top_frequency_hz
    }
}

/// SoX-ng-style multiband compander.
///
/// The command shape is
/// `mcompand quoted_compand_args {crossover_frequency quoted_compand_args}`.
/// Each quoted compand argument group uses the same inner syntax as
/// [`Compand`]. Processing splits the input through fourth-order
/// Linkwitz-Riley-style low/high crossover pairs, compands each band, and sums
/// the bands back to the original channel count and frame count.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::MCompand;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(2),
///     vec![0.25, -0.25],
/// )?;
/// let mcompand = MCompand::parse_sox_args(&["0,0 -60,-60,0,0"])?;
///
/// mcompand.process_buffer(&mut audio)?;
///
/// assert_eq!(audio.as_planar_f32(), &[0.25, -0.25]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MCompand {
    bands: Vec<MCompandBand>,
}

impl MCompand {
    /// Creates a typed multiband compander.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidMCompand`] when no bands are supplied, a
    /// non-final band lacks a crossover, the final band has one, or crossover
    /// frequencies are not strictly ascending.
    pub fn new<I>(bands: I) -> Result<Self>
    where
        I: IntoIterator<Item = MCompandBand>,
    {
        let bands = bands.into_iter().collect::<Vec<_>>();
        if bands.is_empty() {
            return Err(EffectError::InvalidMCompand);
        }

        let mut previous_top = 0.0;
        for (index, band) in bands.iter().enumerate() {
            match (index + 1 == bands.len(), band.top_frequency_hz) {
                (true, Some(_)) | (false, None) => return Err(EffectError::InvalidMCompand),
                (false, Some(frequency)) if frequency <= previous_top => {
                    return Err(EffectError::InvalidMCompand);
                }
                (false, Some(frequency)) => previous_top = frequency,
                (true, None) => {}
            }
        }

        Ok(Self { bands })
    }

    /// Parses SoX-ng `mcompand` arguments.
    ///
    /// Each band argument must be one shell-quoted token containing the inner
    /// compand arguments separated by spaces. Crossover frequencies accept the
    /// same `k`/`K` kilohertz shorthand used by Auralis filter commands.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidMCompand`] when the command shape,
    /// crossover order, or any embedded compand configuration is invalid.
    pub fn parse_sox_args(args: &[&str]) -> Result<Self> {
        if args.is_empty() || args.len().is_multiple_of(2) {
            return Err(EffectError::InvalidMCompand);
        }

        let mut bands = Vec::with_capacity(args.len().div_ceil(2));
        for band_index in (0..args.len()).step_by(2) {
            let compand = parse_quoted_compand(args[band_index])?;
            let top_frequency_hz = args.get(band_index + 1).map(|value| parse_frequency(value));
            bands.push(MCompandBand::new(compand, top_frequency_hz.transpose()?)?);
        }

        Self::new(bands)
    }

    /// Returns the parsed bands in processing order.
    #[must_use]
    pub fn bands(&self) -> &[MCompandBand] {
        &self.bands
    }

    /// Applies the multiband compander in place.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidMCompand`] when a crossover frequency is
    /// invalid for the input sample rate, or when a band processor rejects the
    /// current channel count.
    pub fn process_buffer(&self, audio: &mut AudioBuffer) -> Result<()> {
        let frames =
            usize::try_from(audio.frames().as_u64()).map_err(|_| EffectError::InvalidMCompand)?;
        let channels = audio.channels().as_usize();
        let sample_rate_hz = f64::from(audio.spec().sample_rate().as_u32());
        let mut remaining = planar_to_interleaved(audio.as_planar_f32(), frames, channels);
        let mut summed = vec![0.0; remaining.len()];

        for band in &self.bands {
            let band_input = if let Some(frequency) = band.top_frequency_hz {
                let (low, high) = split_crossover(&remaining, channels, sample_rate_hz, frequency)?;
                remaining = high;
                low
            } else {
                remaining.clone()
            };

            let mut band_audio = AudioBuffer::from_planar_f32(
                audio.spec(),
                audio.frames(),
                interleaved_to_planar(&band_input, frames, channels),
            )
            .map_err(|_| EffectError::InvalidMCompand)?;
            band.compand
                .process_buffer(&mut band_audio)
                .map_err(|_| EffectError::InvalidMCompand)?;
            let band_output = planar_to_interleaved(band_audio.as_planar_f32(), frames, channels);
            add_clipped(&mut summed, &band_output);
        }

        *audio = AudioBuffer::from_planar_f32(
            audio.spec(),
            FrameCount::new(audio.frames().as_u64()),
            interleaved_to_planar(&summed, frames, channels),
        )
        .map_err(|_| EffectError::InvalidMCompand)?;
        Ok(())
    }
}

fn parse_quoted_compand(value: &str) -> Result<Compand> {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    Compand::parse_sox_args(&parts).map_err(|_| EffectError::InvalidMCompand)
}

fn parse_frequency(value: &str) -> Result<f64> {
    let parsed = if let Some(kilohertz) = value.strip_suffix(['k', 'K']) {
        kilohertz.parse::<f64>().map(|frequency| frequency * 1000.0)
    } else {
        value.parse::<f64>()
    }
    .map_err(|_| EffectError::InvalidMCompand)?;

    if parsed.is_finite() && parsed > 0.0 {
        Ok(parsed)
    } else {
        Err(EffectError::InvalidMCompand)
    }
}

fn split_crossover(
    input: &[f32],
    channels: usize,
    sample_rate_hz: f64,
    frequency_hz: f64,
) -> Result<(Vec<f32>, Vec<f32>)> {
    let low_coefficients = BiquadCoefficients::rbj_low_pass(
        sample_rate_hz,
        frequency_hz,
        crate::BiquadWidth::q(FRAC_1_SQRT_2),
    )
    .map_err(|_| EffectError::InvalidMCompand)?;
    let high_coefficients = BiquadCoefficients::rbj_high_pass(
        sample_rate_hz,
        frequency_hz,
        crate::BiquadWidth::q(FRAC_1_SQRT_2),
    )
    .map_err(|_| EffectError::InvalidMCompand)?;
    let mut low_states = vec![
        [
            BiquadState::new(low_coefficients),
            BiquadState::new(low_coefficients),
        ];
        channels
    ];
    let mut high_states = vec![
        [
            BiquadState::new(high_coefficients),
            BiquadState::new(high_coefficients),
        ];
        channels
    ];
    let mut low = vec![0.0; input.len()];
    let mut high = vec![0.0; input.len()];

    for (index, &sample) in input.iter().enumerate() {
        let channel = index % channels;
        let low_state = &mut low_states[channel];
        let low_first = low_state[0].process_sample(sample);
        low[index] = low_state[1].process_sample(low_first);

        let high_state = &mut high_states[channel];
        let high_first = high_state[0].process_sample(sample);
        high[index] = high_state[1].process_sample(high_first);
    }

    Ok((low, high))
}

fn add_clipped(output: &mut [f32], input: &[f32]) {
    for (output, input) in output.iter_mut().zip(input) {
        *output = (*output + *input).clamp(-1.0, 1.0);
    }
}

fn planar_to_interleaved(input: &[f32], frames: usize, channels: usize) -> Vec<f32> {
    let mut output = vec![0.0; input.len()];
    for frame in 0..frames {
        for channel in 0..channels {
            output[frame * channels + channel] = input[channel * frames + frame];
        }
    }
    output
}

fn interleaved_to_planar(input: &[f32], frames: usize, channels: usize) -> Vec<f32> {
    let mut output = vec![0.0; input.len()];
    for frame in 0..frames {
        for channel in 0..channels {
            output[channel * frames + frame] = input[frame * channels + channel];
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{MCompand, MCompandBand};
    use crate::{Compand, EffectError};
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn parses_single_and_multiband_commands() {
        let single = MCompand::parse_sox_args(&["0,0 -60,-60,0,0"]).unwrap();
        assert_eq!(single.bands().len(), 1);

        let multiband =
            MCompand::parse_sox_args(&["0,0 -60,-60,0,0", "1k", "0.01,0.1 -70,-60,0,-3 -1 -20"])
                .unwrap();

        assert_eq!(multiband.bands().len(), 2);
        assert_eq!(multiband.bands()[0].top_frequency_hz(), Some(1000.0));
        assert_eq!(multiband.bands()[1].top_frequency_hz(), None);
    }

    #[test]
    fn rejects_invalid_shape_and_delays() {
        assert_eq!(
            MCompand::parse_sox_args(&[]).unwrap_err(),
            EffectError::InvalidMCompand
        );
        assert_eq!(
            MCompand::parse_sox_args(&["0,0 -60,-60,0,0", "100"]).unwrap_err(),
            EffectError::InvalidMCompand
        );
        assert_eq!(
            MCompand::parse_sox_args(&[
                "0,0 -60,-60,0,0",
                "1000",
                "0,0 -60,-60,0,0",
                "900",
                "0,0 -60,-60,0,0"
            ])
            .unwrap_err(),
            EffectError::InvalidMCompand
        );
        assert_eq!(
            MCompand::parse_sox_args(&["0,0 -60,-60,0,0 0 0 0.1"]).unwrap_err(),
            EffectError::InvalidMCompand
        );
    }

    #[test]
    fn single_band_matches_embedded_compand() {
        let mut mcompand_audio = audio_buffer(vec![0.25, 0.5, -0.25, -0.5]);
        let mut compand_audio = mcompand_audio.clone();
        let mcompand = MCompand::parse_sox_args(&["0,0 -60,-60,0,-6"]).unwrap();
        let compand = Compand::parse_sox_args(&["0,0", "-60,-60,0,-6"]).unwrap();

        mcompand.process_buffer(&mut mcompand_audio).unwrap();
        compand.process_buffer(&mut compand_audio).unwrap();

        assert_eq!(
            mcompand_audio.as_planar_f32(),
            compand_audio.as_planar_f32()
        );
    }

    #[test]
    fn multiband_identity_preserves_shape_and_finite_samples() {
        let mut audio = audio_buffer(vec![0.0, 0.5, -0.5, 0.25, -0.25, 0.0]);
        let mcompand =
            MCompand::parse_sox_args(&["0,0 -60,-60,0,0", "1200", "0,0 -60,-60,0,0"]).unwrap();

        mcompand.process_buffer(&mut audio).unwrap();

        assert_eq!(audio.frames(), FrameCount::new(6));
        assert!(
            audio
                .as_planar_f32()
                .iter()
                .all(|sample| sample.is_finite())
        );
    }

    #[test]
    fn typed_new_rejects_final_crossover() {
        let compand = Compand::parse_sox_args(&["0,0", "-60,-60,0,0"]).unwrap();

        assert_eq!(
            MCompand::new([MCompandBand::new(compand, Some(1000.0)).unwrap()]).unwrap_err(),
            EffectError::InvalidMCompand
        );
    }

    fn audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        AudioBuffer::from_planar_f32(
            AudioSpec::new(
                SampleRate::new(48_000).unwrap(),
                ChannelCount::new(1).unwrap(),
                SampleFormat::Float32,
            ),
            FrameCount::new(u64::try_from(samples.len()).unwrap()),
            samples,
        )
        .unwrap()
    }
}
