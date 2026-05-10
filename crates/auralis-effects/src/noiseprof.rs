use std::fmt::Write;

use auralis_core::AudioBuffer;
use rustfft::{FftPlanner, num_complex::Complex};

use crate::{EffectError, Result};

/// SoX-ng `noiseprof` FFT window size.
pub const NOISE_PROFILE_WINDOW_SIZE: usize = 2048;

/// SoX-ng `noiseprof` frequency-bin count.
pub const NOISE_PROFILE_FREQ_COUNT: usize = NOISE_PROFILE_WINDOW_SIZE / 2 + 1;

/// SoX-ng-style noise profile collector.
///
/// `noiseprof [profile-file(-)]` passes audio through unchanged and writes a
/// textual per-channel noise profile. Auralis exposes that behavior as a typed
/// analyzer: [`Self::profile`] returns the profile artifact, while effect-chain
/// execution keeps the audio pass-through behavior. The optional output path is
/// retained as command metadata for CLI/file layers that need to decide where
/// to write the rendered profile text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoiseProf {
    output_path: Option<String>,
}

impl NoiseProf {
    /// Creates a `noiseprof` analyzer that writes to the SoX-ng default `-`.
    #[must_use]
    pub const fn stdout() -> Self {
        Self { output_path: None }
    }

    /// Creates a `noiseprof` analyzer with an explicit profile output path.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidNoiseProfile`] when `path` is empty.
    pub fn new(output_path: Option<String>) -> Result<Self> {
        if output_path.as_deref().is_some_and(str::is_empty) {
            return Err(EffectError::InvalidNoiseProfile);
        }

        Ok(Self { output_path })
    }

    /// Returns the configured profile output path, or `None` for SoX-ng's `-`.
    #[must_use]
    pub fn output_path(&self) -> Option<&str> {
        self.output_path.as_deref()
    }

    /// Collects a deterministic SoX-ng-style noise profile from decoded audio.
    ///
    /// The analyzer divides each channel into 2048-frame windows, zero-pads the
    /// final partial window, computes the natural log of positive FFT power for
    /// each frequency bin, and averages those logs per bin.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidNoiseProfile`] if a channel view cannot be
    /// obtained from the input buffer.
    pub fn profile(&self, audio: &AudioBuffer) -> Result<NoiseProfile> {
        NoiseProfile::from_audio(audio)
    }

    /// Applies the SoX-ng pass-through audio behavior.
    pub fn process_buffer(&self, _audio: &mut AudioBuffer) {}
}

impl Default for NoiseProf {
    fn default() -> Self {
        Self::stdout()
    }
}

/// Per-channel noise-profile data rendered by `noiseprof`.
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseProfile {
    channels: Vec<Vec<f64>>,
}

impl NoiseProfile {
    /// Collects a noise profile for every channel in `audio`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidNoiseProfile`] if a channel view cannot be
    /// obtained from the input buffer.
    pub fn from_audio(audio: &AudioBuffer) -> Result<Self> {
        let mut channels = Vec::with_capacity(audio.channels().as_usize());
        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::InvalidNoiseProfile)?;
            channels.push(profile_channel(channel));
        }

        Ok(Self { channels })
    }

    /// Returns channel-major averaged log-power bins.
    #[must_use]
    pub fn channels(&self) -> &[Vec<f64>] {
        &self.channels
    }

    /// Renders the profile in SoX-ng's text format.
    #[must_use]
    pub fn render_text(&self) -> String {
        let mut output = String::new();
        for (channel_index, bins) in self.channels.iter().enumerate() {
            let _ = write!(output, "Channel {channel_index}: ");
            for (bin_index, value) in bins.iter().enumerate() {
                if bin_index > 0 {
                    output.push_str(", ");
                }
                let _ = write!(output, "{value:.6}");
            }
            output.push('\n');
        }

        output
    }
}

fn profile_channel(samples: &[f32]) -> Vec<f64> {
    let mut sums = vec![0.0_f64; NOISE_PROFILE_FREQ_COUNT];
    let mut counts = vec![0_u32; NOISE_PROFILE_FREQ_COUNT];
    let mut window = vec![0.0_f32; NOISE_PROFILE_WINDOW_SIZE];
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(NOISE_PROFILE_WINDOW_SIZE);
    let mut spectrum = vec![Complex::new(0.0, 0.0); NOISE_PROFILE_WINDOW_SIZE];

    if samples.is_empty() {
        return sums;
    }

    for chunk in samples.chunks(NOISE_PROFILE_WINDOW_SIZE) {
        window.fill(0.0);
        window[..chunk.len()].copy_from_slice(chunk);
        collect_window(&window, &fft, &mut spectrum, &mut sums, &mut counts);
    }

    sums.into_iter()
        .zip(counts)
        .map(|(sum, count)| {
            if count == 0 {
                0.0
            } else {
                sum / f64::from(count)
            }
        })
        .collect()
}

fn collect_window(
    window: &[f32],
    fft: &std::sync::Arc<dyn rustfft::Fft<f32>>,
    spectrum: &mut [Complex<f32>],
    sums: &mut [f64],
    counts: &mut [u32],
) {
    for (bin, sample) in spectrum.iter_mut().zip(window) {
        *bin = Complex::new(*sample, 0.0);
    }
    fft.process(spectrum);

    for (index, value) in spectrum
        .iter()
        .take(NOISE_PROFILE_FREQ_COUNT)
        .map(Complex::norm_sqr)
        .enumerate()
    {
        if value > 0.0 {
            sums[index] += f64::from(value.ln());
            counts[index] += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NOISE_PROFILE_FREQ_COUNT, NoiseProf};
    use crate::test_support::{audio_buffer, stereo_audio_buffer};

    #[test]
    fn silence_profile_bins_are_zero() {
        let audio = audio_buffer(vec![0.0; 16]);
        let profile = NoiseProf::default().profile(&audio).unwrap();

        assert_eq!(profile.channels().len(), 1);
        assert_eq!(profile.channels()[0], vec![0.0; NOISE_PROFILE_FREQ_COUNT]);
    }

    #[test]
    fn constant_signal_records_dc_energy() {
        let audio = audio_buffer(vec![0.5; 2048]);
        let profile = NoiseProf::default().profile(&audio).unwrap();
        let bins = &profile.channels()[0];

        assert!(bins[0] > 13.0);
        assert!(bins[1..].iter().all(|value| value.abs() < 1.0e-3));
    }

    #[test]
    fn stereo_profile_has_one_rendered_line_per_channel() {
        let audio = stereo_audio_buffer(vec![0.0, 0.25, 0.5, -0.5]);
        let text = NoiseProf::default().profile(&audio).unwrap().render_text();

        assert!(text.starts_with("Channel 0: "));
        assert!(text.contains("\nChannel 1: "));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn rejects_empty_output_path() {
        assert_eq!(
            NoiseProf::new(Some(String::new())).unwrap_err(),
            crate::EffectError::InvalidNoiseProfile
        );
    }
}
