use std::{fs, path::Path};

use auralis_core::{AudioBuffer, FrameCount};
use rustfft::{FftPlanner, num_complex::Complex};

use crate::{
    EffectError, NOISE_PROFILE_FREQ_COUNT, NOISE_PROFILE_WINDOW_SIZE, NoiseProfile, Result,
};

const HALF_WINDOW_SIZE: usize = NOISE_PROFILE_WINDOW_SIZE / 2;
const WINDOW_SIZE_F32: f32 = 2048.0;

/// SoX-ng-style spectral noise reduction.
///
/// `noisered [profile-file(-) [amount]]` consumes a `noiseprof` text profile
/// and applies scalar FFT-domain gating with 50% overlap windows. The typed
/// API can hold a parsed [`NoiseProfile`] directly via [`Self::from_profile`];
/// command-style values retain the profile path and load it when processing.
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseRed {
    profile_path: Option<String>,
    profile: Option<NoiseProfile>,
    amount: f64,
}

impl NoiseRed {
    /// SoX-ng default reduction amount.
    pub const DEFAULT_AMOUNT: f64 = 0.5;

    /// Creates a processor from an already parsed noise profile.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidNoiseReduction`] when `amount` is not in
    /// SoX-ng's supported `0..=1` range.
    pub fn from_profile(profile: NoiseProfile, amount: f64) -> Result<Self> {
        validate_amount(amount)?;
        Ok(Self {
            profile_path: None,
            profile: Some(profile),
            amount,
        })
    }

    /// Creates a command-style processor that loads a profile path at runtime.
    ///
    /// `None` represents SoX-ng's `-` profile source. Library chain execution
    /// cannot read stdin, so processing such a value returns
    /// [`EffectError::InvalidNoiseReduction`]; CLI callers should pass an
    /// explicit profile path.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidNoiseReduction`] when `amount` is outside
    /// `0..=1` or when the path is empty.
    pub fn from_profile_path(profile_path: Option<String>, amount: f64) -> Result<Self> {
        validate_amount(amount)?;
        if profile_path.as_deref().is_some_and(str::is_empty) {
            return Err(EffectError::InvalidNoiseReduction);
        }

        Ok(Self {
            profile_path,
            profile: None,
            amount,
        })
    }

    /// Returns the configured profile path, or `None` for SoX-ng's `-`.
    #[must_use]
    pub fn profile_path(&self) -> Option<&str> {
        self.profile_path.as_deref()
    }

    /// Returns the reduction amount in SoX-ng's `0..=1` range.
    #[must_use]
    pub const fn amount(&self) -> f64 {
        self.amount
    }

    /// Applies spectral noise reduction to a decoded audio buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidNoiseReduction`] when the profile cannot
    /// be loaded, parsed, or matched to the input channel count.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let profile = self.resolved_profile()?;
        if profile.channels().len() != audio.channels().as_usize() {
            return Err(EffectError::InvalidNoiseReduction);
        }

        let mut planar = Vec::new();
        let mut output_frames = None;
        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::InvalidNoiseReduction)?;
            let reduced = reduce_channel(channel, &profile.channels()[channel_index], self.amount)?;
            let frame_count = reduced.len();
            if output_frames.is_some_and(|frames| frames != frame_count) {
                return Err(EffectError::InvalidNoiseReduction);
            }
            output_frames = Some(frame_count);
            planar.extend(reduced);
        }

        let frames = FrameCount::new(
            u64::try_from(output_frames.unwrap_or(0))
                .map_err(|_| EffectError::NoiseReductionLengthOverflow)?,
        );
        AudioBuffer::from_planar_f32(audio.spec(), frames, planar)
            .map_err(|_| EffectError::NoiseReductionLengthOverflow)
    }

    fn resolved_profile(&self) -> Result<NoiseProfile> {
        if let Some(profile) = &self.profile {
            return Ok(profile.clone());
        }

        let path = self
            .profile_path
            .as_deref()
            .ok_or(EffectError::InvalidNoiseReduction)?;
        let text =
            fs::read_to_string(Path::new(path)).map_err(|_| EffectError::InvalidNoiseReduction)?;
        NoiseProfile::parse_text(&text)
    }
}

fn validate_amount(amount: f64) -> Result<()> {
    if amount.is_finite() && (0.0..=1.0).contains(&amount) {
        Ok(())
    } else {
        Err(EffectError::InvalidNoiseReduction)
    }
}

fn reduce_channel(samples: &[f32], noisegate: &[f64], amount: f64) -> Result<Vec<f32>> {
    if noisegate.len() != NOISE_PROFILE_FREQ_COUNT {
        return Err(EffectError::InvalidNoiseReduction);
    }

    let mut state = ChannelState::new(noisegate, amount);
    let mut consumed = 0;
    let mut output = Vec::new();

    while consumed < samples.len() {
        let old_bufdata = state.bufdata;
        let ncopy = (samples.len() - consumed).min(NOISE_PROFILE_WINDOW_SIZE - state.bufdata);
        state.window[old_bufdata..old_bufdata + ncopy]
            .copy_from_slice(&samples[consumed..consumed + ncopy]);
        consumed += ncopy;
        state.bufdata += ncopy;

        if state.bufdata == NOISE_PROFILE_WINDOW_SIZE {
            state.process_window(NOISE_PROFILE_WINDOW_SIZE, &mut output);
            state.bufdata = HALF_WINDOW_SIZE;
        }
    }

    if state.bufdata > 0 {
        state.window[state.bufdata..].fill(0.0);
        state.process_window(state.bufdata, &mut output);
    }

    Ok(output)
}

struct ChannelState<'profile> {
    window: Vec<f32>,
    last_window: Option<Vec<f32>>,
    smoothing: Vec<f32>,
    noisegate: &'profile [f64],
    amount: f64,
    bufdata: usize,
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    ifft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    spectrum: Vec<Complex<f32>>,
}

impl<'profile> ChannelState<'profile> {
    fn new(noisegate: &'profile [f64], amount: f64) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(NOISE_PROFILE_WINDOW_SIZE);
        let ifft = planner.plan_fft_inverse(NOISE_PROFILE_WINDOW_SIZE);
        Self {
            window: vec![0.0; NOISE_PROFILE_WINDOW_SIZE],
            last_window: None,
            smoothing: vec![0.0; NOISE_PROFILE_FREQ_COUNT],
            noisegate,
            amount,
            bufdata: 0,
            fft,
            ifft,
            spectrum: vec![Complex::new(0.0, 0.0); NOISE_PROFILE_WINDOW_SIZE],
        }
    }

    fn process_window(&mut self, len: usize, output: &mut Vec<f32>) {
        let use_len = len.min(NOISE_PROFILE_WINDOW_SIZE) - len.min(HALF_WINDOW_SIZE);
        let mut next_window = vec![0.0; NOISE_PROFILE_WINDOW_SIZE];
        next_window[..HALF_WINDOW_SIZE]
            .copy_from_slice(&self.window[HALF_WINDOW_SIZE..NOISE_PROFILE_WINDOW_SIZE]);

        self.reduce_noise();

        if let Some(last_window) = &self.last_window {
            output.extend(
                self.window[..use_len]
                    .iter()
                    .zip(&last_window[HALF_WINDOW_SIZE..HALF_WINDOW_SIZE + use_len])
                    .map(|(current, previous)| current + previous),
            );
        } else {
            output.extend_from_slice(&self.window[..use_len]);
        }

        self.last_window = Some(std::mem::replace(&mut self.window, next_window));
    }

    fn reduce_noise(&mut self) {
        for (bin, sample) in self.spectrum.iter_mut().zip(&self.window) {
            *bin = Complex::new(*sample, 0.0);
        }
        self.fft.process(&mut self.spectrum);

        let power = hann_power_spectrum(&self.window);
        for (index, power) in power.into_iter().enumerate() {
            let target = if power > 0.0
                && f64::from(power.ln()) < self.noisegate[index] + self.amount * 8.0
            {
                0.0
            } else {
                1.0
            };
            self.smoothing[index] = target * 0.5 + self.smoothing[index] * 0.5;
        }
        suppress_isolated_bins(&mut self.smoothing);

        self.spectrum[0] *= self.smoothing[0];
        self.spectrum[HALF_WINDOW_SIZE] *= self.smoothing[HALF_WINDOW_SIZE];
        for index in 1..HALF_WINDOW_SIZE {
            let smooth = self.smoothing[index];
            self.spectrum[index] *= smooth;
            self.spectrum[NOISE_PROFILE_WINDOW_SIZE - index] *= smooth;
        }

        self.ifft.process(&mut self.spectrum);
        for (index, sample) in self.window.iter_mut().enumerate() {
            *sample = self.spectrum[index].re / WINDOW_SIZE_F32 * hann_coefficient(index);
        }
    }
}

fn hann_power_spectrum(window: &[f32]) -> Vec<f32> {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(NOISE_PROFILE_WINDOW_SIZE);
    let mut spectrum = vec![Complex::new(0.0, 0.0); NOISE_PROFILE_WINDOW_SIZE];

    for (index, bin) in spectrum.iter_mut().enumerate() {
        *bin = Complex::new(window[index] * hann_coefficient(index), 0.0);
    }
    fft.process(&mut spectrum);

    spectrum
        .iter()
        .take(NOISE_PROFILE_FREQ_COUNT)
        .map(Complex::norm_sqr)
        .collect()
}

fn hann_coefficient(index: usize) -> f32 {
    let index = f32::from(u16::try_from(index).expect("noise reduction window index fits in u16"));
    0.5 - 0.5 * (std::f32::consts::TAU * index / WINDOW_SIZE_F32).cos()
}

fn suppress_isolated_bins(smoothing: &mut [f32]) {
    for index in 2..NOISE_PROFILE_FREQ_COUNT - 2 {
        if (0.5..=0.55).contains(&smoothing[index])
            && smoothing[index - 1] < 0.1
            && smoothing[index - 2] < 0.1
            && smoothing[index + 1] < 0.1
            && smoothing[index + 2] < 0.1
        {
            smoothing[index] = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    use super::NoiseRed;
    use crate::{NOISE_PROFILE_FREQ_COUNT, NoiseProfile};

    #[test]
    fn parses_noiseprof_text() {
        let text = format!(
            "Channel 0: {}\n",
            vec!["0.000000"; NOISE_PROFILE_FREQ_COUNT].join(", ")
        );
        let profile = NoiseProfile::parse_text(&text).unwrap();

        assert_eq!(profile.channels().len(), 1);
        assert_eq!(profile.channels()[0].len(), NOISE_PROFILE_FREQ_COUNT);
    }

    #[test]
    fn rejects_invalid_amount() {
        let profile = NoiseProfile::new(vec![vec![0.0; NOISE_PROFILE_FREQ_COUNT]]).unwrap();

        assert!(NoiseRed::from_profile(profile, 1.5).is_err());
    }

    #[test]
    fn noisered_reduces_full_windows_and_shortens_by_half_window() {
        let profile = NoiseProfile::new(vec![vec![0.0; NOISE_PROFILE_FREQ_COUNT]]).unwrap();
        let processor = NoiseRed::from_profile(profile, 0.5).unwrap();
        let audio = mono_audio_buffer(vec![0.0; 4096]);

        let reduced = processor.process_buffer(&audio).unwrap();

        assert_eq!(reduced.frames(), FrameCount::new(3072));
        assert!(reduced.as_planar_f32().iter().all(|sample| *sample == 0.0));
    }

    fn mono_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        );
        AudioBuffer::from_planar_f32(
            spec,
            FrameCount::new(u64::try_from(samples.len()).unwrap()),
            samples,
        )
        .unwrap()
    }
}
