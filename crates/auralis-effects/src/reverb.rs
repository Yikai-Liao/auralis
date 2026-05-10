use auralis_core::{AudioBuffer, AudioSpec, ChannelCount};

use crate::{EffectError, Result};

const DEFAULT_REVERBERANCE_PERCENT: f64 = 50.0;
const DEFAULT_HF_DAMPING_PERCENT: f64 = 50.0;
const DEFAULT_ROOM_SCALE_PERCENT: f64 = 100.0;
const DEFAULT_STEREO_DEPTH_PERCENT: f64 = 100.0;
const DEFAULT_PRE_DELAY_MS: f64 = 0.0;
const DEFAULT_WET_GAIN_DB: f64 = 0.0;
const COMB_LENGTHS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_LENGTHS: [usize; 4] = [225, 341, 441, 556];
const STEREO_ADJUST: f64 = 12.0;

/// SoX-ng-style stereo reverberation.
///
/// `Reverb` implements SoX-ng's Freeverb-derived `reverb` command surface. It
/// is length preserving: the wet path is delayed internally, but the delayed
/// tail is not drained after the input ends. Mono input remains mono in the
/// current command-line-compatible path.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Reverb;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(44_100)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(spec, FrameCount::new(2), vec![1.0, 0.0])?;
/// let processed = Reverb::with_defaults()?.process_buffer(&audio)?;
///
/// assert_eq!(processed.channels().as_u16(), 1);
/// assert_eq!(processed.frames(), audio.frames());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reverb {
    wet_only: bool,
    reverberance_percent: f64,
    hf_damping_percent: f64,
    room_scale_percent: f64,
    stereo_depth_percent: f64,
    pre_delay_ms: f64,
    wet_gain_db: f64,
}

impl Reverb {
    /// Creates a reverb from SoX-ng command parameters.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidReverb`] when percent values are not finite
    /// or outside `0..=100`, pre-delay is outside `0..=500` milliseconds, or
    /// wet gain is outside `-10..=10` dB.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        wet_only: bool,
        reverberance_percent: f64,
        hf_damping_percent: f64,
        room_scale_percent: f64,
        stereo_depth_percent: f64,
        pre_delay_ms: f64,
        wet_gain_db: f64,
    ) -> Result<Self> {
        if !valid_percent(reverberance_percent)
            || !valid_percent(hf_damping_percent)
            || !valid_percent(room_scale_percent)
            || !valid_percent(stereo_depth_percent)
            || !pre_delay_ms.is_finite()
            || !(0.0..=500.0).contains(&pre_delay_ms)
            || !wet_gain_db.is_finite()
            || !(-10.0..=10.0).contains(&wet_gain_db)
        {
            return Err(EffectError::InvalidReverb);
        }

        Ok(Self {
            wet_only,
            reverberance_percent,
            hf_damping_percent,
            room_scale_percent,
            stereo_depth_percent,
            pre_delay_ms,
            wet_gain_db,
        })
    }

    /// Creates reverb with SoX-ng defaults.
    ///
    /// # Errors
    ///
    /// This constructor currently cannot fail because all defaults are valid.
    pub fn with_defaults() -> Result<Self> {
        Self::new(
            false,
            DEFAULT_REVERBERANCE_PERCENT,
            DEFAULT_HF_DAMPING_PERCENT,
            DEFAULT_ROOM_SCALE_PERCENT,
            DEFAULT_STEREO_DEPTH_PERCENT,
            DEFAULT_PRE_DELAY_MS,
            DEFAULT_WET_GAIN_DB,
        )
    }

    /// Returns whether dry input is suppressed.
    #[must_use]
    pub const fn wet_only(self) -> bool {
        self.wet_only
    }

    /// Returns reverberance in percent.
    #[must_use]
    pub const fn reverberance_percent(self) -> f64 {
        self.reverberance_percent
    }

    /// Returns high-frequency damping in percent.
    #[must_use]
    pub const fn hf_damping_percent(self) -> f64 {
        self.hf_damping_percent
    }

    /// Returns room scale in percent.
    #[must_use]
    pub const fn room_scale_percent(self) -> f64 {
        self.room_scale_percent
    }

    /// Returns stereo depth in percent.
    #[must_use]
    pub const fn stereo_depth_percent(self) -> f64 {
        self.stereo_depth_percent
    }

    /// Returns pre-delay in milliseconds.
    #[must_use]
    pub const fn pre_delay_ms(self) -> f64 {
        self.pre_delay_ms
    }

    /// Returns wet gain in dB.
    #[must_use]
    pub const fn wet_gain_db(self) -> f64 {
        self.wet_gain_db
    }

    /// Applies reverb and returns a length-preserving output buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::ReverbLengthOverflow`] when a delay or output
    /// shape cannot be represented.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let input_channels = audio.channels().as_usize();
        let stereo_depth = if input_channels > 2 {
            0.0
        } else {
            self.stereo_depth_percent
        };
        let output_channels = input_channels;
        let processing_channels = if input_channels == 2 && stereo_depth > 0.0 {
            2
        } else {
            1
        };
        let resolved = self.resolve(audio.spec().sample_rate().as_u32(), stereo_depth);

        let frames = usize::try_from(audio.frames().as_u64())
            .map_err(|_| EffectError::ReverbLengthOverflow)?;
        let mut output = vec![
            0.0;
            output_channels
                .checked_mul(frames)
                .ok_or(EffectError::ReverbLengthOverflow)?
        ];

        if processing_channels == 2 {
            let left = audio.channel(0).ok_or(EffectError::ReverbLengthOverflow)?;
            let right = audio.channel(1).ok_or(EffectError::ReverbLengthOverflow)?;
            let left_wet = process_wet(left, resolved);
            let right_wet = process_wet(right, resolved);
            for frame in 0..frames {
                for wet_channel in 0..2 {
                    let dry = if self.wet_only {
                        0.0
                    } else {
                        f64::from(audio.sample(wet_channel, frame).unwrap_or(0.0))
                    };
                    let sample =
                        dry + 0.5 * (left_wet[wet_channel][frame] + right_wet[wet_channel][frame]);
                    output[wet_channel * frames + frame] = f32_from_f64(sample);
                }
            }
        } else {
            for input_channel in 0..input_channels {
                let source = audio
                    .channel(input_channel)
                    .ok_or(EffectError::ReverbLengthOverflow)?;
                let wet = process_wet(source, resolved);
                for frame in 0..frames {
                    let dry = if self.wet_only {
                        0.0
                    } else {
                        f64::from(source[frame])
                    };
                    let wet_sample = if input_channels == 1 && stereo_depth > 0.0 {
                        0.5 * (wet[0][frame] + wet[1][frame])
                    } else {
                        wet[0][frame]
                    };
                    output[input_channel * frames + frame] = f32_from_f64(dry + wet_sample);
                }
            }
        }

        let spec = AudioSpec::new(
            audio.spec().sample_rate(),
            ChannelCount::new(
                u16::try_from(output_channels).map_err(|_| EffectError::ReverbLengthOverflow)?,
            )
            .map_err(EffectError::Core)?,
            audio.spec().sample_format(),
        );
        Ok(AudioBuffer::from_planar_f32(spec, audio.frames(), output)?)
    }

    fn resolve(self, sample_rate_hz: u32, stereo_depth_percent: f64) -> ResolvedReverb {
        let sample_rate = f64::from(sample_rate_hz);
        let delay = (self.pre_delay_ms / 1000.0 * sample_rate + 0.5).floor();
        let scale = self.room_scale_percent / 100.0 * 0.9 + 0.1;
        let depth = stereo_depth_percent / 100.0;
        let a = -1.0 / (1.0_f64 - 0.3).ln();
        let b = 100.0 / ((1.0_f64 - 0.98).ln() * a + 1.0);
        let feedback = 1.0 - ((self.reverberance_percent - b) / (a * b)).exp();

        ResolvedReverb {
            feedback,
            hf_damping: self.hf_damping_percent / 100.0 * 0.3 + 0.2,
            wet_gain: 10.0_f64.powf(self.wet_gain_db / 20.0) * 0.015,
            pre_delay_frames: rounded_nonnegative_to_usize(delay),
            sample_rate,
            scale,
            depth,
            wet_channels: if depth > 0.0 { 2 } else { 1 },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedReverb {
    feedback: f64,
    hf_damping: f64,
    wet_gain: f64,
    pre_delay_frames: usize,
    sample_rate: f64,
    scale: f64,
    depth: f64,
    wet_channels: usize,
}

#[derive(Clone)]
struct DelayFilter {
    buffer: Vec<f64>,
    position: usize,
    store: f64,
}

impl DelayFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size.max(1)],
            position: 0,
            store: 0.0,
        }
    }

    fn comb(&mut self, input: f64, feedback: f64, hf_damping: f64) -> f64 {
        let output = self.buffer[self.position];
        self.store = output + (self.store - output) * hf_damping;
        self.buffer[self.position] = input + self.store * feedback;
        self.advance();
        output
    }

    fn allpass(&mut self, input: f64) -> f64 {
        let output = self.buffer[self.position];
        self.buffer[self.position] = input + output * 0.5;
        self.advance();
        output - input
    }

    fn advance(&mut self) {
        if self.position == 0 {
            self.position = self.buffer.len() - 1;
        } else {
            self.position -= 1;
        }
    }
}

struct FilterArray {
    combs: Vec<DelayFilter>,
    allpasses: Vec<DelayFilter>,
}

impl FilterArray {
    fn new(resolved: ResolvedReverb, wet_index: usize) -> Self {
        let rate_scale = resolved.sample_rate / 44_100.0;
        let mut offset =
            resolved.depth * f64::from(u32::try_from(wet_index).expect("wet index is at most one"));
        let combs = COMB_LENGTHS
            .iter()
            .map(|&length| {
                let delay = f64::from(u32::try_from(length).expect("comb length fits u32"))
                    + STEREO_ADJUST * offset;
                let size = rounded_nonnegative_to_usize(
                    (resolved.scale * rate_scale * delay + 0.5).floor(),
                );
                offset = -offset;
                DelayFilter::new(size)
            })
            .collect();
        let allpasses = ALLPASS_LENGTHS
            .iter()
            .map(|&length| {
                let delay = f64::from(u32::try_from(length).expect("all-pass length fits u32"))
                    + STEREO_ADJUST * offset;
                let size = rounded_nonnegative_to_usize((rate_scale * delay + 0.5).floor());
                offset = -offset;
                DelayFilter::new(size)
            })
            .collect();

        Self { combs, allpasses }
    }

    fn process(&mut self, input: f64, resolved: ResolvedReverb) -> f64 {
        let mut output = 0.0;
        for comb in self.combs.iter_mut().rev() {
            output += comb.comb(input, resolved.feedback, resolved.hf_damping);
        }
        for allpass in self.allpasses.iter_mut().rev() {
            output = allpass.allpass(output);
        }
        output * resolved.wet_gain
    }
}

fn process_wet(source: &[f32], resolved: ResolvedReverb) -> [Vec<f64>; 2] {
    let mut delayed_input = vec![0.0; resolved.pre_delay_frames];
    delayed_input.extend(source.iter().map(|&sample| f64::from(sample)));
    let mut wet = [vec![0.0; source.len()], vec![0.0; source.len()]];

    for (wet_index, output) in wet.iter_mut().enumerate().take(resolved.wet_channels) {
        let mut filters = FilterArray::new(resolved, wet_index);
        for (frame, sample) in output.iter_mut().enumerate() {
            *sample = filters.process(delayed_input[frame], resolved);
        }
    }

    wet
}

fn valid_percent(value: f64) -> bool {
    value.is_finite() && (0.0..=100.0).contains(&value)
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "reverb intentionally stores SoX-ng-style float samples in Auralis f32 buffers"
)]
fn f32_from_f64(value: f64) -> f32 {
    value as f32
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "reverb delays are finite and non-negative after command validation"
)]
fn rounded_nonnegative_to_usize(value: f64) -> usize {
    value as usize
}

#[cfg(test)]
mod tests {
    use super::Reverb;

    #[test]
    fn rejects_out_of_range_parameters() {
        assert_eq!(
            Reverb::new(false, 101.0, 50.0, 100.0, 100.0, 0.0, 0.0).unwrap_err(),
            crate::EffectError::InvalidReverb
        );
        assert_eq!(
            Reverb::new(false, 50.0, 50.0, 100.0, 100.0, 501.0, 0.0).unwrap_err(),
            crate::EffectError::InvalidReverb
        );
        assert_eq!(
            Reverb::new(false, 50.0, 50.0, 100.0, 100.0, 0.0, 11.0).unwrap_err(),
            crate::EffectError::InvalidReverb
        );
    }
}
