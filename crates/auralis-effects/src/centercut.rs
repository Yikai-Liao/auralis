use std::f64::consts::TAU;

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount};
use rustfft::{FftPlanner, num_complex::Complex};

use crate::{EffectError, Result};

const DEFAULT_WINDOW_SIZE: usize = 8192;
const OVERLAP_COUNT: usize = 4;
const OVERLAP_EPSILON: f64 = 1.0e-18;
const SPECTRAL_EPSILON: f64 = 1.0e-15;
const POST_WINDOW_SCALE: f64 = 1.6;
const MIN_WINDOW_SIZE: usize = 8;
const MAX_WINDOW_SIZE: usize = 32_768;

/// Core SoX-ng-style center-cut stereo separation.
///
/// `Centercut` estimates the common stereo center in overlapping spectral
/// windows, subtracts that estimate from the original left and right channels,
/// and returns three output channels: left residual, right residual, then
/// extracted center. The optional output gain matches SoX-ng's `-a`, the
/// optional bass-to-sides mode keeps bins below 200 Hz out of the center
/// estimate like `-b`, and the window size follows the validated `-w` command
/// range.
///
/// The processor requires exactly stereo input and preserves sample rate,
/// sample format, and frame count.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Centercut;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(2)?,
///     SampleFormat::Float32,
/// );
/// let input = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(2),
///     vec![0.25, -0.25, 0.25, -0.25],
/// )?;
///
/// let output = Centercut::new().process_buffer(&input)?;
///
/// assert_eq!(output.channels(), ChannelCount::new(3)?);
/// assert_eq!(output.frames(), input.frames());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Centercut {
    /// Output gain applied to the left residual, right residual, and center.
    pub gain: f32,

    /// Whether low-frequency bins below 200 Hz are kept in the side channels.
    pub bass_to_sides: bool,

    /// Requested spectral window size.
    pub window_size: usize,
}

impl Centercut {
    /// Creates a default center-cut processor.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            gain: 1.0,
            bass_to_sides: false,
            window_size: DEFAULT_WINDOW_SIZE,
        }
    }

    /// Creates a center-cut processor with explicit SoX-ng-style options.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidCentercutGain`] when `gain` is not finite,
    /// or [`EffectError::InvalidCentercutWindowSize`] when `window_size` is not
    /// a power of two in `8..=32768`.
    pub fn with_options(gain: f32, bass_to_sides: bool, window_size: usize) -> Result<Self> {
        if !gain.is_finite() {
            return Err(EffectError::InvalidCentercutGain);
        }
        if !valid_window_size(window_size) {
            return Err(EffectError::InvalidCentercutWindowSize);
        }

        Ok(Self {
            gain,
            bass_to_sides,
            window_size,
        })
    }

    /// Applies center-cut stereo separation and returns a three-channel buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::CentercutRequiresStereo`] when the input is not
    /// exactly stereo, or [`EffectError::Core`] if the output buffer shape
    /// cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        if audio.channels().as_usize() != 2 {
            return Err(EffectError::CentercutRequiresStereo);
        }

        let output_channels = ChannelCount::new(3).map_err(EffectError::from)?;
        let output_spec = AudioSpec::new(
            audio.spec().sample_rate(),
            output_channels,
            audio.spec().sample_format(),
        );
        let frames = usize::try_from(audio.frames().as_u64())
            .map_err(|_| auralis_core::AuralisError::InvalidAudioBufferShape)?;
        let left = audio
            .channel(0)
            .ok_or(EffectError::CentercutRequiresStereo)?;
        let right = audio
            .channel(1)
            .ok_or(EffectError::CentercutRequiresStereo)?;
        let center = extract_center(
            left,
            right,
            self.window_size,
            audio.spec().sample_rate().as_u32(),
            self.bass_to_sides,
        );
        let mut output = Vec::with_capacity(
            frames
                .checked_mul(3)
                .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?,
        );

        output.extend(
            left.iter()
                .zip(&center)
                .map(|(&sample, &center)| (sample - center) * self.gain),
        );
        output.extend(
            right
                .iter()
                .zip(&center)
                .map(|(&sample, &center)| (sample - center) * self.gain),
        );
        output.extend(center.into_iter().map(|sample| sample * self.gain));

        AudioBuffer::from_planar_f32(output_spec, audio.frames(), output).map_err(EffectError::from)
    }
}

impl Default for Centercut {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "The spectral core evaluates bounded f64 FFT windows and returns Auralis f32 samples."
)]
fn extract_center(
    left: &[f32],
    right: &[f32],
    requested_window_size: usize,
    sample_rate: u32,
    bass_to_sides: bool,
) -> Vec<f32> {
    debug_assert_eq!(left.len(), right.len());

    if left.is_empty() {
        return Vec::new();
    }

    let window_size = left
        .len()
        .next_power_of_two()
        .clamp(MIN_WINDOW_SIZE, requested_window_size);
    let bass_cutoff_bin = if bass_to_sides {
        200_usize.saturating_mul(window_size) / usize::try_from(sample_rate).unwrap_or(usize::MAX)
    } else {
        0
    };
    let hop = window_size / OVERLAP_COUNT;
    let analysis = raised_cosine_window(window_size, 1.0);
    let post = raised_cosine_window(window_size, 2.0)
        .into_iter()
        .map(|sample| sample * POST_WINDOW_SCALE)
        .collect::<Vec<_>>();
    let mut planner = FftPlanner::<f64>::new();
    let forward = planner.plan_fft_forward(window_size);
    let inverse = planner.plan_fft_inverse(window_size);
    let mut left_spectrum = vec![Complex::default(); window_size];
    let mut right_spectrum = vec![Complex::default(); window_size];
    let mut center_spectrum = vec![Complex::default(); window_size];
    let mut center_accumulator = vec![0.0_f64; left.len()];
    let mut weight_accumulator = vec![0.0_f64; left.len()];
    let mut block_start = 0_usize;

    while block_start < left.len() {
        fill_windowed_spectrum(&mut left_spectrum, left, block_start, &analysis);
        fill_windowed_spectrum(&mut right_spectrum, right, block_start, &analysis);

        forward.process(&mut left_spectrum);
        forward.process(&mut right_spectrum);
        fill_center_spectrum(
            &left_spectrum,
            &right_spectrum,
            &mut center_spectrum,
            bass_cutoff_bin,
        );
        inverse.process(&mut center_spectrum);

        let active_len = (left.len() - block_start).min(window_size);
        for window_index in 0..active_len {
            let center_bin = center_spectrum[window_index];
            let analysis_weight = analysis[window_index];
            let post_weight = post[window_index];
            let output_index = block_start + window_index;
            center_accumulator[output_index] += center_bin.re * post_weight / window_size as f64;
            weight_accumulator[output_index] += analysis_weight * post_weight;
        }

        block_start += hop;
    }

    center_accumulator
        .into_iter()
        .zip(weight_accumulator)
        .map(|(center, weight)| {
            if weight > OVERLAP_EPSILON {
                (center / weight) as f32
            } else {
                0.0
            }
        })
        .collect()
}

fn fill_windowed_spectrum(
    spectrum: &mut [Complex<f64>],
    samples: &[f32],
    block_start: usize,
    window: &[f64],
) {
    let active_len = (samples.len() - block_start).min(spectrum.len());
    for index in 0..active_len {
        spectrum[index] =
            Complex::new(f64::from(samples[block_start + index]) * window[index], 0.0);
    }
    for bin in &mut spectrum[active_len..] {
        *bin = Complex::default();
    }
}

fn fill_center_spectrum(
    left: &[Complex<f64>],
    right: &[Complex<f64>],
    center: &mut [Complex<f64>],
    bass_cutoff_bin: usize,
) {
    center.fill(Complex::default());

    for bin in 1..left.len() / 2 {
        if bin < bass_cutoff_bin {
            continue;
        }

        let sum = left[bin] + right[bin];
        let diff = left[bin] - right[bin];
        let sum_energy = sum.norm_sqr();
        let diff_energy = diff.norm_sqr();
        let alpha = if sum_energy > SPECTRAL_EPSILON {
            0.5 - (diff_energy / sum_energy).sqrt() * 0.5
        } else {
            0.0
        };
        let center_bin = sum * alpha;
        center[bin] = center_bin;
        center[left.len() - bin] = center_bin.conj();
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Window sizes are bounded to 8192 samples before conversion to f64."
)]
fn raised_cosine_window(size: usize, power: f64) -> Vec<f64> {
    (0..size)
        .map(|index| (0.5 * (1.0 - (TAU * (index as f64 + 0.5) / size as f64).cos())).powf(power))
        .collect()
}

fn valid_window_size(window_size: usize) -> bool {
    (MIN_WINDOW_SIZE..=MAX_WINDOW_SIZE).contains(&window_size) && window_size.is_power_of_two()
}

#[cfg(test)]
mod tests {
    use super::Centercut;
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn emits_left_right_and_center_channels() {
        let audio = stereo_buffer(vec![0.25; 64], vec![0.25; 64]);

        let actual = Centercut::new().process_buffer(&audio).unwrap();

        assert_eq!(actual.channels(), ChannelCount::new(3).unwrap());
        assert_eq!(actual.frames(), FrameCount::new(64));
    }

    #[test]
    fn extracts_identical_stereo_as_center() {
        let samples = sine(256, 3.0, 0.5);
        let audio = stereo_buffer(samples.clone(), samples);

        let actual = Centercut::new().process_buffer(&audio).unwrap();
        let left = actual.channel(0).unwrap();
        let right = actual.channel(1).unwrap();
        let center = actual.channel(2).unwrap();

        let input_rms = rms(audio.channel(0).unwrap());
        let side_rms = rms(left).max(rms(right));
        let center_rms = rms(center);

        assert!(
            side_rms < input_rms * 0.5,
            "expected side rms below half of input rms: input={input_rms}, side={side_rms}"
        );
        assert!(
            center_rms > input_rms * 0.5,
            "expected center rms above half of input rms: input={input_rms}, center={center_rms}"
        );
        assert!(center.iter().any(|sample| sample.abs() > 0.4));
    }

    #[test]
    fn applies_output_gain_to_all_channels() {
        let samples = sine(256, 3.0, 0.5);
        let audio = stereo_buffer(samples.clone(), samples);

        let unity = Centercut::new().process_buffer(&audio).unwrap();
        let halved = Centercut::with_options(0.5, false, 8192)
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_close_scaled(
            halved.as_planar_f32(),
            unity.as_planar_f32(),
            0.5,
            0.000_001,
        );
    }

    #[test]
    fn validates_window_size_and_gain_options() {
        assert_eq!(
            Centercut::with_options(f32::NAN, false, 8192).unwrap_err(),
            EffectError::InvalidCentercutGain
        );
        assert_eq!(
            Centercut::with_options(1.0, false, 7).unwrap_err(),
            EffectError::InvalidCentercutWindowSize
        );
        assert_eq!(
            Centercut::with_options(1.0, false, 12).unwrap_err(),
            EffectError::InvalidCentercutWindowSize
        );
        assert!(Centercut::with_options(1.0, true, 32_768).is_ok());
    }

    #[test]
    fn leaves_opposite_phase_audio_on_sides() {
        let left = sine(256, 3.0, 0.5);
        let right = left.iter().map(|sample| -*sample).collect();
        let audio = stereo_buffer(left.clone(), right);

        let actual = Centercut::new().process_buffer(&audio).unwrap();
        let center = actual.channel(2).unwrap();

        assert!(center.iter().all(|sample| sample.abs() < 0.001));
        assert_close(actual.channel(0).unwrap(), &left, 0.001);
    }

    #[test]
    fn rejects_non_stereo_input() {
        let audio = audio_buffer(1, vec![0.0; 64]);

        assert_eq!(
            Centercut::new().process_buffer(&audio).unwrap_err(),
            EffectError::CentercutRequiresStereo
        );
    }

    fn stereo_buffer(left: Vec<f32>, right: Vec<f32>) -> AudioBuffer {
        assert_eq!(left.len(), right.len());
        let mut samples = left;
        samples.extend(right);
        audio_buffer(2, samples)
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

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "Test fixtures use small frame counts and intentional f32 sample output."
    )]
    fn sine(frames: usize, cycles: f64, amplitude: f32) -> Vec<f32> {
        (0..frames)
            .map(|frame| {
                (std::f64::consts::TAU * cycles * frame as f64 / frames as f64).sin() as f32
                    * amplitude
            })
            .collect()
    }

    fn assert_close(actual: &[f32], expected: &[f32], epsilon: f32) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= epsilon,
                "sample {index}: expected {expected}, got {actual}"
            );
        }
    }

    fn assert_close_scaled(actual: &[f32], expected: &[f32], scale: f32, epsilon: f32) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected * scale).abs() <= epsilon,
                "sample {index}: expected {}, got {actual}",
                expected * scale
            );
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "Test fixtures use small sample counts for RMS comparisons."
    )]
    fn rms(samples: &[f32]) -> f32 {
        let sum_squares = samples.iter().map(|sample| sample * sample).sum::<f32>();
        (sum_squares / samples.len() as f32).sqrt()
    }
}
