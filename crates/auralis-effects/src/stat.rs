use std::fmt::Write;

use auralis_core::AudioBuffer;

use crate::{EffectError, Result};

const DEFAULT_SCALE: f64 = 1.0;
const ROUND_31_BIT: f64 = 2_147_483_648.0;

/// SoX-ng-style `stat` analyzer configuration.
///
/// `stat` is an analysis effect: chain execution passes audio through
/// unchanged, while [`Self::report`] returns deterministic sample statistics
/// over the decoded stream.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stat {
    scale: f64,
    scale_to_rms: bool,
    volume_only: bool,
    json: bool,
}

impl Stat {
    /// Creates the default `stat` analyzer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            scale: DEFAULT_SCALE,
            scale_to_rms: false,
            volume_only: false,
            json: false,
        }
    }

    /// Creates a `stat -s scale` analyzer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidStat`] when `scale` is not finite or is
    /// zero.
    pub fn with_scale(mut self, scale: f64) -> Result<Self> {
        if !scale.is_finite() || scale.abs() <= f64::EPSILON {
            return Err(EffectError::InvalidStat);
        }
        self.scale = scale;
        Ok(self)
    }

    /// Enables `stat -rms` output scaling.
    #[must_use]
    pub const fn with_rms_scaling(mut self) -> Self {
        self.scale_to_rms = true;
        self
    }

    /// Enables `stat -v` volume-adjustment-only rendering.
    #[must_use]
    pub const fn volume_only(mut self) -> Self {
        self.volume_only = true;
        self
    }

    /// Enables `stat -j` JSON report rendering.
    #[must_use]
    pub const fn json(mut self) -> Self {
        self.json = true;
        self
    }

    /// Returns the configured sample scale divisor.
    #[must_use]
    pub const fn scale(self) -> f64 {
        self.scale
    }

    /// Returns whether `-rms` scaling is enabled.
    #[must_use]
    pub const fn scale_to_rms(self) -> bool {
        self.scale_to_rms
    }

    /// Returns whether `-v` rendering is enabled.
    #[must_use]
    pub const fn is_volume_only(self) -> bool {
        self.volume_only
    }

    /// Returns whether `-j` rendering is enabled.
    #[must_use]
    pub const fn is_json(self) -> bool {
        self.json
    }

    /// Collects deterministic statistics from decoded audio.
    ///
    /// Statistics are computed in frame-major order to match SoX-ng's stream
    /// order even though Auralis stores buffers in planar channel-major form.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidStat`] if the input contains non-finite
    /// samples or RMS scaling is requested for silence.
    pub fn report(self, audio: &AudioBuffer) -> Result<StatReport> {
        StatReport::from_audio(self, audio)
    }

    /// Applies pass-through chain behavior.
    pub fn process_buffer(self, _audio: &mut AudioBuffer) {}
}

impl Default for Stat {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic report emitted by the `stat` analyzer.
#[derive(Debug, Clone, PartialEq)]
pub struct StatReport {
    samples_read: u64,
    length_seconds: f64,
    scaled_by: f64,
    scaled_by_rms: Option<f64>,
    maximum_amplitude: f64,
    minimum_amplitude: f64,
    midline_amplitude: f64,
    mean_norm: f64,
    mean_amplitude: f64,
    rms_amplitude: f64,
    maximum_delta: f64,
    minimum_delta: f64,
    mean_delta: f64,
    rms_delta: f64,
    rough_frequency_hz: u64,
    volume_adjustment: Option<f64>,
}

impl StatReport {
    fn from_audio(stat: Stat, audio: &AudioBuffer) -> Result<Self> {
        let mut samples = frame_major_scaled_samples(audio, stat.scale)?;
        let samples_read = u64::try_from(samples.len()).map_err(|_| EffectError::InvalidStat)?;
        let length_seconds =
            f64_from_u64(audio.frames().as_u64()) / f64::from(audio.spec().sample_rate().as_u32());

        let sample_rate = f64::from(audio.spec().sample_rate().as_u32());
        let mut report = collect_report_fields(
            samples_read,
            length_seconds,
            stat.scale,
            sample_rate,
            &samples,
        );
        if stat.scale_to_rms {
            if report.rms_amplitude <= f64::EPSILON {
                return Err(EffectError::InvalidStat);
            }
            let rms = report.rms_amplitude;
            for sample in &mut samples {
                *sample /= rms;
            }
            report = collect_report_fields(
                samples_read,
                length_seconds,
                stat.scale * rms,
                sample_rate,
                &samples,
            );
            report.scaled_by_rms = Some(rms);
        }

        Ok(report)
    }

    /// Returns the flattened stream sample count.
    #[must_use]
    pub const fn samples_read(&self) -> u64 {
        self.samples_read
    }

    /// Returns the audio length in seconds.
    #[must_use]
    pub const fn length_seconds(&self) -> f64 {
        self.length_seconds
    }

    /// Returns the maximum scaled sample value.
    #[must_use]
    pub const fn maximum_amplitude(&self) -> f64 {
        self.maximum_amplitude
    }

    /// Returns the minimum scaled sample value.
    #[must_use]
    pub const fn minimum_amplitude(&self) -> f64 {
        self.minimum_amplitude
    }

    /// Returns the RMS amplitude.
    #[must_use]
    pub const fn rms_amplitude(&self) -> f64 {
        self.rms_amplitude
    }

    /// Returns the SoX-ng-style volume adjustment, if peak amplitude is nonzero.
    #[must_use]
    pub const fn volume_adjustment(&self) -> Option<f64> {
        self.volume_adjustment
    }

    /// Renders the report in SoX-ng's plain text shape.
    #[must_use]
    pub fn render_text(&self) -> String {
        let mut output = String::new();
        let _ = writeln!(output, "Samples read:      {:12}", self.samples_read);
        let _ = writeln!(output, "Length (seconds):  {:12.6}", self.length_seconds);
        if let Some(rms) = self.scaled_by_rms {
            let _ = writeln!(output, "Scaled by rms:     {rms:12.6}");
        } else {
            let _ = writeln!(output, "Scaled by:         {:12.1}", self.scaled_by);
        }
        let _ = writeln!(output, "Maximum amplitude: {:12.6}", self.maximum_amplitude);
        let _ = writeln!(output, "Minimum amplitude: {:12.6}", self.minimum_amplitude);
        let _ = writeln!(output, "Midline amplitude: {:12.6}", self.midline_amplitude);
        let _ = writeln!(output, "Mean    norm:      {:12.6}", self.mean_norm);
        let _ = writeln!(output, "Mean    amplitude: {:12.6}", self.mean_amplitude);
        let _ = writeln!(output, "RMS     amplitude: {:12.6}", self.rms_amplitude);
        let _ = writeln!(output, "Maximum delta:     {:12.6}", self.maximum_delta);
        let _ = writeln!(output, "Minimum delta:     {:12.6}", self.minimum_delta);
        let _ = writeln!(output, "Mean    delta:     {:12.6}", self.mean_delta);
        let _ = writeln!(output, "RMS     delta:     {:12.6}", self.rms_delta);
        let _ = writeln!(output, "Rough   frequency: {:12}", self.rough_frequency_hz);
        if let Some(volume) = self.volume_adjustment {
            let _ = writeln!(output, "Volume adjustment: {volume:12.3}");
        }
        output
    }

    /// Renders the report as deterministic JSON text.
    #[must_use]
    pub fn render_json(&self) -> String {
        let mut output = String::new();
        output.push_str("{\n");
        let _ = writeln!(output, "  \"samples_read\": {},", self.samples_read);
        let _ = writeln!(
            output,
            "  \"length\": {},",
            json_number(self.length_seconds)
        );
        if let Some(rms) = self.scaled_by_rms {
            let _ = writeln!(output, "  \"scaled_by_rms\": {},", json_number(rms));
        } else {
            let _ = writeln!(output, "  \"scaled_by\": {},", json_number(self.scaled_by));
        }
        let _ = writeln!(
            output,
            "  \"maximum_amplitude\": {},",
            json_number(self.maximum_amplitude)
        );
        let _ = writeln!(
            output,
            "  \"minimum_amplitude\": {},",
            json_number(self.minimum_amplitude)
        );
        let _ = writeln!(
            output,
            "  \"midline_amplitude\": {},",
            json_number(self.midline_amplitude)
        );
        let _ = writeln!(output, "  \"mean_norm\": {},", json_number(self.mean_norm));
        let _ = writeln!(
            output,
            "  \"mean_amplitude\": {},",
            json_number(self.mean_amplitude)
        );
        let _ = writeln!(
            output,
            "  \"rms_amplitude\": {},",
            json_number(self.rms_amplitude)
        );
        let _ = writeln!(
            output,
            "  \"maximum_delta\": {},",
            json_number(self.maximum_delta)
        );
        let _ = writeln!(
            output,
            "  \"minimum_delta\": {},",
            json_number(self.minimum_delta)
        );
        let _ = writeln!(
            output,
            "  \"mean_delta\": {},",
            json_number(self.mean_delta)
        );
        let _ = writeln!(output, "  \"rms_delta\": {},", json_number(self.rms_delta));
        let _ = writeln!(
            output,
            "  \"rough_frequency\": {},",
            self.rough_frequency_hz
        );
        match self.volume_adjustment {
            Some(volume) => {
                let _ = writeln!(output, "  \"volume_adjustment\": {}", json_number(volume));
            }
            None => output.push_str("  \"volume_adjustment\": null\n"),
        }
        output.push_str("}\n");
        output
    }

    /// Renders only the volume adjustment value used by `stat -v`.
    #[must_use]
    pub fn render_volume_adjustment(&self) -> String {
        self.volume_adjustment
            .map_or_else(String::new, |volume| format!("{volume:.3}\n"))
    }

    /// Renders according to `stat` command options.
    #[must_use]
    pub fn render_for(&self, stat: Stat) -> String {
        if stat.is_json() {
            self.render_json()
        } else if stat.is_volume_only() {
            self.render_volume_adjustment()
        } else {
            self.render_text()
        }
    }
}

fn collect_report_fields(
    samples_read: u64,
    length_seconds: f64,
    scale: f64,
    sample_rate: f64,
    samples: &[f64],
) -> StatReport {
    if samples.is_empty() {
        return StatReport {
            samples_read,
            length_seconds,
            scaled_by: scale,
            scaled_by_rms: None,
            maximum_amplitude: 0.0,
            minimum_amplitude: 0.0,
            midline_amplitude: 0.0,
            mean_norm: 0.0,
            mean_amplitude: 0.0,
            rms_amplitude: 0.0,
            maximum_delta: 0.0,
            minimum_delta: 0.0,
            mean_delta: 0.0,
            rms_delta: 0.0,
            rough_frequency_hz: 0,
            volume_adjustment: None,
        };
    }

    let mut minimum = samples[0];
    let mut maximum = samples[0];
    let mut absolute_sum = 0.0;
    let mut sum = 0.0;
    let mut sum_squares = 0.0;
    let mut maximum_delta = 0.0_f64;
    let minimum_delta = 0.0;
    let mut delta_sum = 0.0;
    let mut delta_squares = 0.0;
    let mut previous = samples[0];

    for &sample in samples {
        minimum = minimum.min(sample);
        maximum = maximum.max(sample);
        absolute_sum += sample.abs();
        sum += sample;
        sum_squares += sample * sample;

        let delta = (sample - previous).abs();
        maximum_delta = maximum_delta.max(delta);
        delta_sum += delta;
        delta_squares += delta * delta;
        previous = sample;
    }

    let count = f64_from_usize(samples.len());
    let delta_count = f64_from_usize(samples.len().saturating_sub(1));
    let rms_amplitude = (sum_squares / count).sqrt();
    let peak = maximum.max(-minimum);
    let rough_frequency_hz = if sum_squares > 0.0 {
        rough_frequency((delta_squares / sum_squares).sqrt(), sample_rate)
    } else {
        0
    };

    StatReport {
        samples_read,
        length_seconds,
        scaled_by: scale,
        scaled_by_rms: None,
        maximum_amplitude: maximum,
        minimum_amplitude: minimum,
        midline_amplitude: minimum / 2.0 + maximum / 2.0,
        mean_norm: absolute_sum / count,
        mean_amplitude: round_to_31_bit(sum / count),
        rms_amplitude,
        maximum_delta,
        minimum_delta,
        mean_delta: if delta_count > 0.0 {
            delta_sum / delta_count
        } else {
            0.0
        },
        rms_delta: if delta_count > 0.0 {
            (delta_squares / delta_count).sqrt()
        } else {
            0.0
        },
        rough_frequency_hz,
        volume_adjustment: (peak > 0.0).then_some(1.0 / (peak * scale)),
    }
}

fn frame_major_scaled_samples(audio: &AudioBuffer, scale: f64) -> Result<Vec<f64>> {
    let frames = usize::try_from(audio.frames().as_u64()).map_err(|_| EffectError::InvalidStat)?;
    let channels = audio.channels().as_usize();
    let mut samples = Vec::with_capacity(audio.as_planar_f32().len());

    for frame in 0..frames {
        for channel in 0..channels {
            let sample = f64::from(
                audio
                    .sample(channel, frame)
                    .ok_or(EffectError::InvalidStat)?,
            ) / scale;
            if !sample.is_finite() {
                return Err(EffectError::InvalidStat);
            }
            samples.push(sample);
        }
    }

    Ok(samples)
}

fn round_to_31_bit(value: f64) -> f64 {
    (value * ROUND_31_BIT).round() / ROUND_31_BIT
}

fn json_number(value: f64) -> String {
    if value.abs() <= f64::EPSILON {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "stat reports mirror SoX-ng's f64 summary calculations"
)]
fn f64_from_u64(value: u64) -> f64 {
    value as f64
}

#[allow(
    clippy::cast_precision_loss,
    reason = "stat reports mirror SoX-ng's f64 summary calculations"
)]
fn f64_from_usize(value: usize) -> f64 {
    value as f64
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "SoX-ng rough frequency is an integer truncation of a non-negative estimate"
)]
fn rough_frequency(delta_ratio: f64, sample_rate: f64) -> u64 {
    (delta_ratio * sample_rate / (std::f64::consts::PI * 2.0)) as u64
}

#[cfg(test)]
mod tests {
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    use super::Stat;
    use crate::EffectError;

    #[test]
    fn report_counts_frame_major_samples_and_amplitudes() {
        let audio = stereo_audio_buffer(vec![0.5, -0.5, 0.25, -0.25]);
        let report = Stat::new().report(&audio).unwrap();

        assert_eq!(report.samples_read(), 4);
        assert!((report.length_seconds() - 2.0 / 48_000.0).abs() < 1.0e-12);
        assert!((report.maximum_amplitude() - 0.5).abs() < f64::EPSILON);
        assert!((report.minimum_amplitude() + 0.5).abs() < f64::EPSILON);
        assert!((report.rms_amplitude() - 0.395_284_707_5).abs() < 1.0e-10);
        assert_eq!(report.volume_adjustment(), Some(2.0));
    }

    #[test]
    fn report_rejects_non_finite_samples_and_invalid_scale() {
        let audio = mono_audio_buffer(vec![f32::NAN]);

        assert_eq!(Stat::new().report(&audio), Err(EffectError::InvalidStat));
        assert_eq!(Stat::new().with_scale(0.0), Err(EffectError::InvalidStat));
    }

    #[test]
    fn renderers_emit_stable_shapes() {
        let audio = mono_audio_buffer(vec![0.0, 0.5, -0.5, 0.25]);
        let stat = Stat::new().json();
        let report = stat.report(&audio).unwrap();

        assert!(
            report
                .render_text()
                .contains("Maximum amplitude:     0.500000")
        );
        assert!(report.render_json().contains("\"samples_read\": 4"));
        assert_eq!(report.render_volume_adjustment(), "2.000\n");
        assert_eq!(report.render_for(stat), report.render_json());
    }

    fn mono_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        audio_buffer(samples, 1)
    }

    fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        audio_buffer(samples, 2)
    }

    fn audio_buffer(samples: Vec<f32>, channels: u16) -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(channels).unwrap(),
            SampleFormat::Float32,
        );
        AudioBuffer::from_planar_f32(
            spec,
            FrameCount::new(u64::try_from(samples.len() / usize::from(channels)).unwrap()),
            samples,
        )
        .unwrap()
    }
}
