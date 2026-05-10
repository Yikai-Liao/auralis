//! Deterministic audio corpus cases shared by Auralis tests.
//!
//! The corpus is generated programmatically from stable string identifiers.
//! Samples are planar `f32` values in full-scale units, matching the internal
//! test representation used by Auralis DSP and golden tests.

use std::{error::Error, fmt};

/// Default sample rate used by corpus cases unless the identifier documents a
/// more specific rate.
pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;

/// Stable identifiers for the reusable deterministic corpus.
///
/// The list includes the README L0 signal families plus fixture identifiers
/// used by existing golden manifests.
pub const CORPUS_IDS: &[&str] = &[
    "l0/silence_mono_16",
    "l0/silence_stereo_16",
    "l0/impulse_mono_16",
    "l0/step_mono_16",
    "l0/sine_mono_32",
    "l0/sine_mono_8192",
    "l0/sweep_mono_64",
    "l0/noise_mono_32_seed_1",
    "l0/full_scale_mono_8",
    "l0/near_zero_mono_8",
    "l0/odd_length_mono_17",
    "l0/sine_stereo_32",
    "l0/sine_stereo_44100_32",
    "l0/opposite_phase_stereo_32",
    "l0/opposite_phase_stereo_8192",
    "l0/short_mono_3",
    "l0/short_stereo_2",
    "chains/stereo_steps",
    "chains/stereo_multitone",
    "chains/stereo_ramp_tone",
    "combine/mono_short",
    "combine/mono_long",
    "combine/stereo_front",
    "combine/stereo_tail",
    "auto/rate_48k_silence",
    "auto/stereo_channels",
    "auto/mono_channels",
    "auto/level_headroom",
];

/// One generated corpus case.
#[derive(Debug, Clone, PartialEq)]
pub struct CorpusCase {
    id: &'static str,
    sample_rate: u32,
    channels: u16,
    frames: usize,
    planar_samples: Vec<f32>,
}

impl CorpusCase {
    /// Creates a corpus case from already-planar samples.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::InvalidShape`] if `channels` is zero, if the
    /// planar sample count is not divisible by `channels`, or if any sample is
    /// non-finite.
    pub fn from_planar_samples(
        id: &'static str,
        sample_rate: u32,
        channels: u16,
        planar_samples: Vec<f32>,
    ) -> Result<Self, CorpusError> {
        let channel_count = usize::from(channels);
        if channel_count == 0
            || !planar_samples.len().is_multiple_of(channel_count)
            || planar_samples.iter().any(|sample| !sample.is_finite())
        {
            return Err(CorpusError::InvalidShape { id });
        }
        let frames = planar_samples.len() / channel_count;

        Ok(Self {
            id,
            sample_rate,
            channels,
            frames,
            planar_samples,
        })
    }

    /// Returns the stable corpus identifier.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Returns the corpus sample rate in frames per second.
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Returns the channel count.
    #[must_use]
    pub const fn channels(&self) -> u16 {
        self.channels
    }

    /// Returns the frame count per channel.
    #[must_use]
    pub const fn frames(&self) -> usize {
        self.frames
    }

    /// Returns planar full-scale samples.
    #[must_use]
    pub fn planar_samples(&self) -> &[f32] {
        &self.planar_samples
    }
}

/// Errors from deterministic corpus lookup or construction.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CorpusError {
    /// No corpus case exists for the requested identifier.
    UnknownId {
        /// Requested stable corpus identifier.
        id: String,
    },

    /// A built-in corpus case has an invalid planar shape.
    InvalidShape {
        /// Built-in stable corpus identifier.
        id: &'static str,
    },
}

impl fmt::Display for CorpusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownId { id } => write!(formatter, "unknown corpus id `{id}`"),
            Self::InvalidShape { id } => {
                write!(formatter, "corpus id `{id}` has invalid sample shape")
            }
        }
    }
}

impl Error for CorpusError {}

/// Returns whether a corpus identifier is known.
#[must_use]
pub fn is_known_corpus_id(id: &str) -> bool {
    CORPUS_IDS.contains(&id)
}

/// Generates a deterministic corpus case by stable identifier.
///
/// # Errors
///
/// Returns [`CorpusError::UnknownId`] when `id` is not listed in
/// [`CORPUS_IDS`].
pub fn corpus_case(id: &str) -> Result<CorpusCase, CorpusError> {
    let case = match id {
        "l0/silence_mono_16" => mono(id, vec![0.0; 16]),
        "l0/silence_stereo_16" => stereo(id, vec![0.0; 16], vec![0.0; 16]),
        "l0/impulse_mono_16" => {
            let mut samples = vec![0.0; 16];
            samples[0] = 1.0;
            mono(id, samples)
        }
        "l0/step_mono_16" => mono(
            id,
            (0..16)
                .map(|frame| if frame < 8 { -0.5 } else { 0.5 })
                .collect(),
        ),
        "l0/sine_mono_32" => mono(id, sine(32, 1_000.0, 0.5, 0.0)),
        "l0/sine_mono_8192" => mono(id, sine(8192, 750.0, 0.5, 0.0)),
        "l0/sweep_mono_64" => mono(id, sweep(64, 200.0, 4_000.0, 0.45)),
        "l0/noise_mono_32_seed_1" => mono(id, seeded_noise(32, 1, 0.5)),
        "l0/full_scale_mono_8" => mono(id, vec![-1.0, 1.0, -1.0, 1.0, 0.999, -0.999, 0.0, -0.0]),
        "l0/near_zero_mono_8" => mono(
            id,
            vec![
                0.0, 0.000_001, -0.000_001, 0.000_01, -0.000_01, 0.000_1, -0.000_1, -0.0,
            ],
        ),
        "l0/odd_length_mono_17" => mono(
            id,
            (0..17)
                .map(|frame| f64_to_f32(-0.4 + f64::from(frame) * 0.8 / 16.0))
                .collect(),
        ),
        "l0/sine_stereo_32" => stereo(id, sine(32, 1_000.0, 0.5, 0.0), sine(32, 500.0, 0.25, 0.25)),
        "l0/sine_stereo_44100_32" => CorpusCase::from_planar_samples(
            static_id(id),
            44_100,
            2,
            stereo_samples(sine(32, 1_000.0, 0.5, 0.0), sine(32, 500.0, 0.25, 0.25)),
        ),
        "l0/opposite_phase_stereo_32" => {
            let left = sine(32, 750.0, 0.5, 0.0);
            let right = left.iter().map(|sample| -*sample).collect();
            stereo(id, left, right)
        }
        "l0/opposite_phase_stereo_8192" => {
            let left = sine(8192, 750.0, 0.5, 0.0);
            let right = left.iter().map(|sample| -*sample).collect();
            stereo(id, left, right)
        }
        "l0/short_mono_3" => mono(id, vec![0.25, -0.25, 0.0]),
        "l0/short_stereo_2" => stereo(id, vec![0.25, -0.25], vec![-0.5, 0.5]),
        "chains/stereo_steps" => {
            let left = vec![-0.6, -0.45, -0.3, -0.15, 0.0, 0.15, 0.3, 0.45, 0.6, 0.75];
            let right = left.iter().map(|sample| -sample / 2.0).collect();
            stereo(id, left, right)
        }
        "chains/stereo_multitone" => stereo(id, stereo_multitone_left(), stereo_multitone_right()),
        "chains/stereo_ramp_tone" => stereo(id, ramp(32, -0.5, 0.5), sine(32, 6_000.0, 0.35, 0.0)),
        "combine/mono_short" => mono(id, vec![0.25, -0.5]),
        "combine/mono_long" => mono(id, vec![0.75, 0.0, -0.25]),
        "combine/stereo_front" => stereo(id, vec![-0.5, -0.25, 0.0], vec![0.5, 0.25, 0.0]),
        "combine/stereo_tail" => stereo(id, vec![0.25, 0.5], vec![-0.25, -0.5]),
        "auto/rate_48k_silence" => mono(id, vec![0.0; 4]),
        "auto/stereo_channels" => stereo(id, vec![0.25, -0.5], vec![0.75, 0.5]),
        "auto/mono_channels" => mono(id, vec![0.25, -0.5, 0.0]),
        "auto/level_headroom" => mono(id, vec![0.0, 0.25, -0.5, 0.75]),
        _ => {
            return Err(CorpusError::UnknownId { id: id.to_owned() });
        }
    }?;

    Ok(case)
}

fn mono(id: &str, samples: Vec<f32>) -> Result<CorpusCase, CorpusError> {
    CorpusCase::from_planar_samples(static_id(id), DEFAULT_SAMPLE_RATE, 1, samples)
}

fn stereo(id: &str, left: Vec<f32>, right: Vec<f32>) -> Result<CorpusCase, CorpusError> {
    let samples = stereo_samples(left, right);
    CorpusCase::from_planar_samples(static_id(id), DEFAULT_SAMPLE_RATE, 2, samples)
}

fn stereo_samples(left: Vec<f32>, right: Vec<f32>) -> Vec<f32> {
    let mut samples = left;
    samples.extend(right);
    samples
}

fn static_id(id: &str) -> &'static str {
    CORPUS_IDS
        .iter()
        .copied()
        .find(|known| *known == id)
        .expect("corpus id was matched before construction")
}

fn sine(frames: usize, frequency_hz: f64, amplitude: f64, phase: f64) -> Vec<f32> {
    (0..frames)
        .map(|frame| {
            let radians = 2.0 * std::f64::consts::PI * frequency_hz * usize_to_f64(frame)
                / f64::from(DEFAULT_SAMPLE_RATE)
                + phase;
            f64_to_f32(radians.sin() * amplitude)
        })
        .collect()
}

fn sweep(frames: usize, start_hz: f64, end_hz: f64, amplitude: f64) -> Vec<f32> {
    let denominator = usize_to_f64(frames.saturating_sub(1).max(1));
    (0..frames)
        .scan(0.0_f64, |phase, frame| {
            let fraction = usize_to_f64(frame) / denominator;
            let frequency = start_hz + (end_hz - start_hz) * fraction;
            let sample = phase.sin() * amplitude;
            *phase += 2.0 * std::f64::consts::PI * frequency / f64::from(DEFAULT_SAMPLE_RATE);
            Some(f64_to_f32(sample))
        })
        .collect()
}

fn seeded_noise(frames: usize, seed: u32, amplitude: f32) -> Vec<f32> {
    (0..frames)
        .scan(seed, |state, _| {
            *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let unit = f64::from(*state >> 8) / 16_777_215.0;
            Some(f64_to_f32((unit * 2.0 - 1.0) * f64::from(amplitude)))
        })
        .collect()
}

fn ramp(frames: usize, start: f32, end: f32) -> Vec<f32> {
    let denominator = usize_to_f32(frames.saturating_sub(1).max(1));
    (0..frames)
        .map(|frame| start + (end - start) * usize_to_f32(frame) / denominator)
        .collect()
}

fn stereo_multitone_left() -> Vec<f32> {
    (0..64)
        .map(|frame| {
            let frame = f64::from(frame);
            let first = 0.25 * (2.0 * std::f64::consts::PI * frame / 16.0).sin();
            let second = 0.1 * (2.0 * std::f64::consts::PI * frame / 7.0).sin();
            f64_to_f32(first + second)
        })
        .collect()
}

fn stereo_multitone_right() -> Vec<f32> {
    (0..64)
        .map(|frame| {
            let frame = f64::from(frame);
            let first = 0.20 * (2.0 * std::f64::consts::PI * frame / 11.0).cos();
            let second = 0.05 * (2.0 * std::f64::consts::PI * frame / 5.0).sin();
            f64_to_f32(first - second)
        })
        .collect()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "corpus frame counts are intentionally tiny and bounded by checked-in IDs"
)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_precision_loss,
    reason = "corpus frame counts are intentionally tiny and bounded by checked-in IDs"
)]
fn usize_to_f32(value: usize) -> f32 {
    value as f32
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "corpus samples are defined as planar f32 values shared with Python"
)]
fn f64_to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::{CORPUS_IDS, corpus_case, is_known_corpus_id};

    #[test]
    fn every_listed_corpus_id_generates_a_well_formed_case() {
        for &id in CORPUS_IDS {
            let case = corpus_case(id).unwrap();
            let expected_sample_rate = if id == "l0/sine_stereo_44100_32" {
                44_100
            } else {
                48_000
            };

            assert_eq!(case.id(), id);
            assert_eq!(case.sample_rate(), expected_sample_rate);
            assert_eq!(
                case.planar_samples().len(),
                case.channels() as usize * case.frames()
            );
            assert!(
                case.planar_samples()
                    .iter()
                    .all(|sample| sample.is_finite())
            );
            assert!(is_known_corpus_id(id));
        }
    }

    #[test]
    fn corpus_signal_families_have_stable_samples() {
        let impulse = corpus_case("l0/impulse_mono_16").unwrap();
        assert_samples_close(&impulse.planar_samples()[..4], &[1.0, 0.0, 0.0, 0.0]);

        let step = corpus_case("l0/step_mono_16").unwrap();
        assert_close(step.planar_samples()[7], -0.5);
        assert_close(step.planar_samples()[8], 0.5);

        let sine = corpus_case("l0/sine_mono_32").unwrap();
        assert_close(sine.planar_samples()[0], 0.0);
        assert_close(sine.planar_samples()[1], 0.065_263_09);

        let noise = corpus_case("l0/noise_mono_32_seed_1").unwrap();
        assert_close(noise.planar_samples()[0], -0.263_544_5);
    }

    #[test]
    fn golden_fixture_ids_are_available_as_corpus_cases() {
        let case = corpus_case("chains/stereo_steps").unwrap();

        assert_eq!(case.channels(), 2);
        assert_eq!(case.frames(), 10);
        assert_close(case.planar_samples()[0], -0.6);
        assert_close(case.planar_samples()[10], 0.3);
    }

    #[test]
    fn unknown_id_reports_the_requested_identifier() {
        let error = corpus_case("missing").unwrap_err();

        assert_eq!(error.to_string(), "unknown corpus id `missing`");
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (&actual, &expected) in actual.iter().zip(expected) {
            assert_close(actual, expected);
        }
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= f32::EPSILON,
            "expected {expected}, got {actual}"
        );
    }
}
