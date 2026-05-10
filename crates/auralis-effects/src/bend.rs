use auralis_core::{AudioBuffer, AudioSpec, FrameCount};
use rustfft::{FftPlanner, num_complex::Complex};

use crate::{EffectError, Result};

const DEFAULT_FRAME_RATE: u32 = 25;
const DEFAULT_OVERSAMPLE: u32 = 16;
const MIN_FRAME_RATE: u32 = 10;
const MAX_FRAME_RATE: u32 = 80;
const MIN_OVERSAMPLE: u32 = 4;
const MAX_OVERSAMPLE: u32 = 32;
const MAX_PITCH_FACTOR: f64 = 10.0;
const MIN_PITCH_FACTOR: f64 = 0.01;

/// Anchor for a SoX-ng-style `bend` position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BendAnchor {
    /// Position is measured from the start of input.
    Start,
    /// Position is measured from the previously resolved bend position.
    Previous,
}

/// Amount for a SoX-ng-style `bend` position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BendAmount {
    /// Direct frame count, matching SoX-ng's `s` suffix.
    Frames(FrameCount),
    /// Seconds resolved using the input sample rate.
    Seconds(f64),
}

/// One unresolved `bend` start or end position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BendPosition {
    anchor: BendAnchor,
    amount: BendAmount,
}

impl BendPosition {
    /// Creates an absolute frame position.
    #[must_use]
    pub const fn frames(frames: FrameCount) -> Self {
        Self {
            anchor: BendAnchor::Start,
            amount: BendAmount::Frames(frames),
        }
    }

    /// Creates an absolute seconds-based position.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBend`] when `seconds` is not finite or is
    /// negative.
    pub fn seconds(seconds: f64) -> Result<Self> {
        validate_seconds(seconds)?;
        Ok(Self {
            anchor: BendAnchor::Start,
            amount: BendAmount::Seconds(seconds),
        })
    }

    /// Creates a position with an explicit anchor and amount.
    #[must_use]
    pub const fn new(anchor: BendAnchor, amount: BendAmount) -> Self {
        Self { anchor, amount }
    }

    /// Returns the position anchor.
    #[must_use]
    pub const fn anchor(self) -> BendAnchor {
        self.anchor
    }

    /// Returns the unresolved position amount.
    #[must_use]
    pub const fn amount(self) -> BendAmount {
        self.amount
    }

    fn resolved(self, sample_rate_hz: u32, previous: FrameCount) -> Result<FrameCount> {
        let offset = self.amount.resolved_frames(sample_rate_hz)?;
        match self.anchor {
            BendAnchor::Start => Ok(offset),
            BendAnchor::Previous => previous
                .as_u64()
                .checked_add(offset.as_u64())
                .map(FrameCount::new)
                .ok_or(EffectError::BendLengthOverflow),
        }
    }
}

impl BendAmount {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "seconds are validated non-negative and finite before SoX-ng-style rounding"
    )]
    fn resolved_frames(self, sample_rate_hz: u32) -> Result<FrameCount> {
        match self {
            Self::Frames(frames) => Ok(frames),
            Self::Seconds(seconds) => {
                validate_seconds(seconds)?;
                let frames = seconds.mul_add(f64::from(sample_rate_hz), 0.5).floor();
                if frames > u64::MAX as f64 {
                    return Err(EffectError::BendLengthOverflow);
                }
                Ok(FrameCount::new(frames as u64))
            }
        }
    }
}

/// One pitch-bend segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BendSegment {
    /// Bend start position.
    pub start: BendPosition,
    /// Pitch shift accumulated over the segment, in cents.
    pub cents: f64,
    /// Bend end position.
    pub end: BendPosition,
}

impl BendSegment {
    /// Creates one `start,cents,end` bend segment.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBend`] when the cents value is not finite
    /// or maps outside SoX-ng's supported pitch-shift factor range.
    pub fn new(start: BendPosition, cents: f64, end: BendPosition) -> Result<Self> {
        validate_cents(cents)?;
        Ok(Self { start, cents, end })
    }
}

/// SoX-ng-style phase-vocoder pitch bend.
///
/// `Bend` applies one or more linear pitch bends over frame positions while
/// preserving input duration and sample-rate metadata. Processing is
/// channel-local and scalar because the SoX-ng algorithm is a stateful
/// short-time Fourier transform.
#[derive(Debug, Clone, PartialEq)]
pub struct Bend {
    /// Analysis frame rate in frames per second.
    pub frame_rate: u32,
    /// STFT oversampling ratio.
    pub oversample: u32,
    segments: Vec<BendSegment>,
}

impl Bend {
    /// Creates a bend processor with SoX-ng defaults.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBend`] when no segment is supplied or any
    /// segment is invalid.
    pub fn new<I>(segments: I) -> Result<Self>
    where
        I: IntoIterator<Item = BendSegment>,
    {
        Self::with_options(DEFAULT_FRAME_RATE, DEFAULT_OVERSAMPLE, segments)
    }

    /// Creates a bend processor with explicit `-f` and `-o` options.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBend`] when options are outside SoX-ng's
    /// accepted ranges or when no segment is supplied.
    pub fn with_options<I>(frame_rate: u32, oversample: u32, segments: I) -> Result<Self>
    where
        I: IntoIterator<Item = BendSegment>,
    {
        let segments = segments.into_iter().collect::<Vec<_>>();
        if !(MIN_FRAME_RATE..=MAX_FRAME_RATE).contains(&frame_rate)
            || !(MIN_OVERSAMPLE..=MAX_OVERSAMPLE).contains(&oversample)
            || segments.is_empty()
        {
            return Err(EffectError::InvalidBend);
        }
        for segment in &segments {
            validate_cents(segment.cents)?;
        }

        Ok(Self {
            frame_rate,
            oversample,
            segments,
        })
    }

    /// Returns the configured bend segments.
    #[must_use]
    pub fn segments(&self) -> &[BendSegment] {
        &self.segments
    }

    /// Applies the pitch bend to every channel independently.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBend`] when resolved positions are out of
    /// order, or [`EffectError::BendLengthOverflow`] when the derived STFT state
    /// or output shape cannot be represented.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let resolved = self.resolved_segments(audio)?;
        if resolved.iter().all(|segment| segment.duration == 0) {
            return Ok(audio.clone());
        }

        let mut output = Vec::with_capacity(audio.as_planar_f32().len());
        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::BendLengthOverflow)?;
            output.extend(
                BendState::new(
                    audio.spec().sample_rate().as_u32(),
                    self.frame_rate,
                    self.oversample,
                    &resolved,
                )?
                .process_channel(channel),
            );
        }

        let spec = AudioSpec::new(
            audio.spec().sample_rate(),
            audio.channels(),
            audio.spec().sample_format(),
        );
        Ok(AudioBuffer::from_planar_f32(spec, audio.frames(), output)?)
    }

    fn resolved_segments(&self, audio: &AudioBuffer) -> Result<Vec<ResolvedBendSegment>> {
        let sample_rate = audio.spec().sample_rate().as_u32();
        let input_frames = audio.frames();
        let mut previous = FrameCount::new(0);
        let mut previous_start = FrameCount::new(0);
        let mut resolved = Vec::with_capacity(self.segments.len());

        for (index, segment) in self.segments.iter().enumerate() {
            let start = segment.start.resolved(sample_rate, previous)?;
            previous = start;
            let end = segment.end.resolved(sample_rate, previous)?;
            previous = end;
            if end < start || start > input_frames || (index > 0 && start < previous_start) {
                return Err(EffectError::InvalidBend);
            }
            previous_start = start;
            resolved.push(ResolvedBendSegment {
                start: usize::try_from(start.as_u64()).map_err(|_| EffectError::InvalidBend)?,
                cents: segment.cents,
                duration: usize::try_from(end.as_u64() - start.as_u64())
                    .map_err(|_| EffectError::InvalidBend)?,
            });
        }

        Ok(resolved)
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedBendSegment {
    start: usize,
    cents: f64,
    duration: usize,
}

#[derive(Debug, Clone)]
struct BendState {
    segments: Vec<ResolvedBendSegment>,
    fft_size: usize,
    oversample: usize,
    sample_rate: f32,
    input_fifo: Vec<f32>,
    output_fifo: Vec<f32>,
    output_accum: Vec<f32>,
    last_phase: Vec<f32>,
    sum_phase: Vec<f32>,
    ana_freq: Vec<f32>,
    ana_magn: Vec<f32>,
    syn_freq: Vec<f32>,
    syn_magn: Vec<f32>,
    window: Vec<f32>,
    rover: usize,
    input_position: usize,
    segment_index: usize,
    shift: f32,
}

impl BendState {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "SoX-ng derives f32 STFT state from validated integer rates"
    )]
    fn new(
        sample_rate: u32,
        frame_rate: u32,
        oversample: u32,
        segments: &[ResolvedBendSegment],
    ) -> Result<Self> {
        let mut n = ((f64::from(sample_rate) / f64::from(frame_rate)) + 0.5) as usize;
        let mut fft_size = 2_usize;
        while n > 2 {
            fft_size = fft_size
                .checked_mul(2)
                .ok_or(EffectError::BendLengthOverflow)?;
            n >>= 1;
        }
        let oversample = usize::try_from(oversample).map_err(|_| EffectError::InvalidBend)?;
        if fft_size == 0 || oversample == 0 || fft_size < oversample {
            return Err(EffectError::BendLengthOverflow);
        }
        let step = fft_size / oversample;
        if step == 0 || fft_size < step {
            return Err(EffectError::BendLengthOverflow);
        }
        let latency = fft_size - step;

        Ok(Self {
            segments: segments.to_vec(),
            fft_size,
            oversample,
            sample_rate: sample_rate as f32,
            input_fifo: vec![0.0; fft_size],
            output_fifo: vec![0.0; fft_size],
            output_accum: vec![0.0; 2 * fft_size],
            last_phase: vec![0.0; fft_size / 2 + 1],
            sum_phase: vec![0.0; fft_size / 2 + 1],
            ana_freq: vec![0.0; fft_size],
            ana_magn: vec![0.0; fft_size],
            syn_freq: vec![0.0; fft_size],
            syn_magn: vec![0.0; fft_size],
            window: hann_window(fft_size),
            rover: latency,
            input_position: 0,
            segment_index: 0,
            shift: 1.0,
        })
    }

    fn process_channel(mut self, input: &[f32]) -> Vec<f32> {
        let mut planner = FftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(self.fft_size);
        let inverse = planner.plan_fft_inverse(self.fft_size);
        let mut spectrum = vec![Complex::new(0.0, 0.0); self.fft_size];
        let mut output = Vec::with_capacity(input.len());
        let latency = self.latency();

        for &sample in input {
            self.input_position += 1;
            self.input_fifo[self.rover] = sample;
            output.push(self.output_fifo[self.rover - latency].clamp(-1.0, 1.0));
            self.rover += 1;

            if self.rover >= self.fft_size {
                self.process_frame(&forward, &inverse, &mut spectrum);
            }
        }

        output
    }

    fn process_frame(
        &mut self,
        forward: &std::sync::Arc<dyn rustfft::Fft<f32>>,
        inverse: &std::sync::Arc<dyn rustfft::Fft<f32>>,
        spectrum: &mut [Complex<f32>],
    ) {
        let pitch_shift = self.current_pitch_shift();
        self.rover = self.latency();
        self.analyze_frame(forward, spectrum);
        self.shift_bins(pitch_shift);
        self.synthesize_frame(inverse, spectrum);
        self.advance_fifo();
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "SoX-ng pitch-bend progress and factors are computed in floating point"
    )]
    fn current_pitch_shift(&mut self) -> f32 {
        let mut pitch_shift = self.shift;
        while let Some(segment) = self.segments.get(self.segment_index).copied() {
            if self.input_position < segment.start + segment.duration {
                break;
            }
            self.shift *= cents_factor(segment.cents) as f32;
            pitch_shift = self.shift;
            self.segment_index += 1;
        }

        if let Some(segment) = self.segments.get(self.segment_index)
            && segment.duration != 0
            && self.input_position >= segment.start
        {
            let progress = (self.input_position - segment.start) as f64 / segment.duration as f64;
            pitch_shift = self.shift * cents_factor(progress * segment.cents) as f32;
        }

        pitch_shift
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "phase-vocoder math mirrors SoX-ng's f32 analysis path"
    )]
    fn analyze_frame(
        &mut self,
        forward: &std::sync::Arc<dyn rustfft::Fft<f32>>,
        spectrum: &mut [Complex<f32>],
    ) {
        let half = self.fft_size / 2;
        let step = self.step_size();
        let freq_per_bin = self.sample_rate / self.fft_size as f32;
        let expected = 2.0 * std::f32::consts::PI * step as f32 / self.fft_size as f32;

        for (index, bin) in spectrum.iter_mut().enumerate() {
            *bin = Complex::new(self.input_fifo[index] * self.window[index], 0.0);
        }
        forward.process(spectrum);

        for (bin, value) in spectrum.iter().enumerate().take(half + 1) {
            let real = value.re;
            let imag = -value.im;
            let magn = 2.0 * real.hypot(imag);
            let phase = imag.atan2(real);
            let mut tmp = phase - self.last_phase[bin];
            self.last_phase[bin] = phase;
            tmp -= bin as f32 * expected;

            let mut qpd = (tmp / std::f32::consts::PI) as i32;
            if qpd >= 0 {
                qpd += qpd & 1;
            } else {
                qpd -= qpd & 1;
            }
            tmp -= std::f32::consts::PI * qpd as f32;
            tmp = self.oversample as f32 * tmp / (2.0 * std::f32::consts::PI);
            tmp = (bin as f32 + tmp) * freq_per_bin;
            self.ana_magn[bin] = magn;
            self.ana_freq[bin] = tmp;
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "SoX-ng truncates shifted bin indexes after validating pitch factors"
    )]
    fn shift_bins(&mut self, pitch_shift: f32) {
        let half = self.fft_size / 2;
        self.syn_magn.fill(0.0);
        self.syn_freq.fill(0.0);
        for bin in 0..=half {
            let target = (bin as f32 * pitch_shift) as usize;
            if target <= half {
                self.syn_magn[target] += self.ana_magn[bin];
                self.syn_freq[target] = self.ana_freq[bin] * pitch_shift;
            }
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "phase-vocoder math mirrors SoX-ng's f32 synthesis path"
    )]
    fn synthesize_frame(
        &mut self,
        inverse: &std::sync::Arc<dyn rustfft::Fft<f32>>,
        spectrum: &mut [Complex<f32>],
    ) {
        let half = self.fft_size / 2;
        let step = self.step_size();
        let freq_per_bin = self.sample_rate / self.fft_size as f32;
        let expected = 2.0 * std::f32::consts::PI * step as f32 / self.fft_size as f32;

        spectrum.fill(Complex::new(0.0, 0.0));
        for (bin, slot) in spectrum.iter_mut().enumerate().take(half + 1) {
            let magn = self.syn_magn[bin];
            let mut tmp = self.syn_freq[bin] - bin as f32 * freq_per_bin;
            tmp /= freq_per_bin;
            tmp = 2.0 * std::f32::consts::PI * tmp / self.oversample as f32;
            tmp += bin as f32 * expected;
            self.sum_phase[bin] += tmp;
            let phase = self.sum_phase[bin];
            *slot = Complex::new(magn * phase.cos(), -magn * phase.sin());
        }

        inverse.process(spectrum);
        for (index, value) in spectrum.iter().enumerate().take(self.fft_size) {
            self.output_accum[index] +=
                2.0 * self.window[index] * value.re / (half as f32 * self.oversample as f32);
        }
    }

    fn advance_fifo(&mut self) {
        let step = self.step_size();
        self.output_fifo[..step].copy_from_slice(&self.output_accum[..step]);
        self.output_accum.copy_within(step..step + self.fft_size, 0);
        self.output_accum[self.fft_size..self.fft_size + step].fill(0.0);
        self.input_fifo.copy_within(step..self.fft_size, 0);
    }

    fn step_size(&self) -> usize {
        self.fft_size / self.oversample
    }

    fn latency(&self) -> usize {
        self.fft_size - self.step_size()
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "SoX-ng builds a single-precision Hann window from integer indexes"
)]
fn hann_window(fft_size: usize) -> Vec<f32> {
    (0..fft_size)
        .map(|index| {
            -0.5 * (2.0 * std::f32::consts::PI * index as f32 / fft_size as f32).cos() + 0.5
        })
        .collect()
}

fn cents_factor(cents: f64) -> f64 {
    2.0_f64.powf(cents / 1200.0)
}

fn validate_cents(cents: f64) -> Result<()> {
    let factor = cents_factor(cents);
    if cents.is_finite()
        && factor.is_finite()
        && (MIN_PITCH_FACTOR..=MAX_PITCH_FACTOR).contains(&factor)
    {
        Ok(())
    } else {
        Err(EffectError::InvalidBend)
    }
}

fn validate_seconds(seconds: f64) -> Result<()> {
    if seconds.is_finite() && seconds >= 0.0 {
        Ok(())
    } else {
        Err(EffectError::InvalidBend)
    }
}

#[cfg(test)]
mod tests {
    use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};

    use super::{Bend, BendAmount, BendAnchor, BendPosition, BendSegment};
    use crate::EffectError;

    #[test]
    fn zero_duration_segments_are_identity() {
        let audio = audio_buffer((0_u16..64).map(|value| f32::from(value) / 64.0).collect());
        let segment = BendSegment::new(
            BendPosition::frames(FrameCount::new(0)),
            0.0,
            BendPosition::frames(FrameCount::new(0)),
        )
        .unwrap();

        let bent = Bend::new([segment])
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(bent, audio);
    }

    #[test]
    fn nonzero_segment_preserves_shape_and_produces_finite_samples() {
        let audio = audio_buffer(
            (0_u16..4096)
                .map(|value| f32::from(value % 128) / 128.0)
                .collect(),
        );
        let segment = BendSegment::new(
            BendPosition::frames(FrameCount::new(0)),
            100.0,
            BendPosition::frames(FrameCount::new(2048)),
        )
        .unwrap();

        let bent = Bend::new([segment])
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(bent.spec(), audio.spec());
        assert_eq!(bent.frames(), audio.frames());
        assert!(bent.as_planar_f32().iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn validates_options_positions_and_cents() {
        let start = BendPosition::frames(FrameCount::new(10));
        let end = BendPosition::frames(FrameCount::new(5));
        let segment = BendSegment::new(start, 0.0, end).unwrap();
        let error = Bend::new([segment])
            .unwrap()
            .process_buffer(&audio_buffer(vec![0.0; 16]))
            .unwrap_err();
        assert_eq!(error, EffectError::InvalidBend);

        assert_eq!(
            BendSegment::new(start, 5000.0, end).unwrap_err(),
            EffectError::InvalidBend
        );
        assert_eq!(
            Bend::with_options(9, 16, [segment]).unwrap_err(),
            EffectError::InvalidBend
        );
        assert_eq!(
            Bend::with_options(25, 33, [segment]).unwrap_err(),
            EffectError::InvalidBend
        );
        assert!(BendPosition::seconds(-1.0).is_err());
        assert_eq!(
            BendPosition::new(BendAnchor::Previous, BendAmount::Frames(FrameCount::new(2)))
                .anchor(),
            BendAnchor::Previous
        );
    }

    fn audio_buffer(samples: Vec<f32>) -> auralis_core::AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
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
