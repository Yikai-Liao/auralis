use auralis_core::{AudioBuffer, AudioSpec, FrameCount};

use crate::{EffectError, Result};

const DEFAULT_FACTOR: f64 = 1.0;
const DEFAULT_WINDOW_MS: f64 = 20.0;
const DEFAULT_SLOW_SHIFT_RATIO: f64 = 0.8;
const DEFAULT_FAST_SHIFT_RATIO: f64 = 1.0;

/// SoX-ng `stretch` cross-fade family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StretchFade {
    /// Linear equal-gain cross-fade.
    Linear,
    /// Square-root equal-power cross-fade.
    Sqrt,
    /// Half-cosine equal-gain cross-fade.
    HalfCosine,
    /// Quarter-cosine equal-power cross-fade.
    QuarterCosine,
}

/// SoX-ng-style basic time stretcher.
///
/// `Stretch` changes the number of decoded frames by copying overlapping
/// windows and cross-fading their overlap. A factor greater than `1` lengthens
/// audio; a factor less than `1` shortens audio. The implementation mirrors
/// SoX-ng's channel-local flow state and currently uses scalar processing
/// because the algorithm is stateful and window-oriented.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Stretch;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(256),
///     (0_u16..256).map(|frame| f32::from(frame) / 256.0).collect(),
/// )?;
///
/// let stretched = Stretch::new(1.5, 1.0)?.process_buffer(&audio)?;
///
/// assert!(stretched.frames().as_u64() > audio.frames().as_u64());
/// assert_eq!(stretched.spec().sample_rate(), audio.spec().sample_rate());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidStretch`] for non-finite values,
/// window sizes below one millisecond, shift ratios outside `(0, 1]`, or
/// fading ratios outside `0..=0.5`. [`Self::process_buffer`] returns
/// [`EffectError::StretchLengthOverflow`] if the derived state or output shape
/// cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stretch {
    /// Output length factor relative to the input length.
    pub factor: f64,
    /// Cross-fading window size in milliseconds.
    pub window_ms: f64,
    /// Cross-fade coefficient family.
    pub fade: StretchFade,
    /// Shift ratio relative to the window.
    pub shift: f64,
    /// Fading ratio relative to the window.
    pub fading: f64,
}

impl Stretch {
    /// Creates a `stretch` processor with SoX-ng defaults derived from `factor`
    /// and `window_ms`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidStretch`] when either value is invalid.
    pub fn new(factor: f64, window_ms: f64) -> Result<Self> {
        let shift = if factor <= 1.0 {
            DEFAULT_FAST_SHIFT_RATIO
        } else {
            DEFAULT_SLOW_SHIFT_RATIO
        };
        let fading = default_fading(factor, shift);
        Self::with_options(factor, window_ms, StretchFade::Linear, shift, fading)
    }

    /// Creates the SoX-ng default `stretch`, equivalent to `stretch 1 20 l`.
    #[must_use]
    pub const fn default_settings() -> Self {
        Self {
            factor: DEFAULT_FACTOR,
            window_ms: DEFAULT_WINDOW_MS,
            fade: StretchFade::Linear,
            shift: DEFAULT_FAST_SHIFT_RATIO,
            fading: 0.0,
        }
    }

    /// Creates a `stretch` processor from explicit SoX-ng command options.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidStretch`] when any value is outside
    /// SoX-ng's documented range.
    pub fn with_options(
        factor: f64,
        window_ms: f64,
        fade: StretchFade,
        shift: f64,
        fading: f64,
    ) -> Result<Self> {
        if !factor.is_finite()
            || factor < 0.0
            || !window_ms.is_finite()
            || window_ms < 1.0
            || !shift.is_finite()
            || shift <= 0.0
            || shift > 1.0
            || !fading.is_finite()
            || !(0.0..=0.5).contains(&fading)
        {
            return Err(EffectError::InvalidStretch);
        }

        Ok(Self {
            factor,
            window_ms,
            fade,
            shift,
            fading,
        })
    }

    /// Applies channel-local SoX-ng-style stretch processing.
    ///
    /// Factor `1` is a null effect and returns an identity copy.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::StretchLengthOverflow`] when the derived window
    /// shifts are zero or when the output buffer shape cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        if self.factor.to_bits() == DEFAULT_FACTOR.to_bits() {
            return Ok(audio.clone());
        }

        let state = StretchState::new(self, audio.spec().sample_rate().as_u32())?;
        let mut output = Vec::with_capacity(estimated_output_samples(
            audio.frames(),
            audio.channels().as_usize(),
            self.factor,
        )?);
        let mut output_frames = None;

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::StretchLengthOverflow)?;
            let channel_frames = state.process_channel_into(channel, &mut output);
            output_frames.get_or_insert(channel_frames);
            if output_frames != Some(channel_frames) {
                return Err(EffectError::StretchLengthOverflow);
            }
        }

        let output_frames = output_frames
            .unwrap_or(0)
            .try_into()
            .map(FrameCount::new)
            .map_err(|_| EffectError::StretchLengthOverflow)?;
        let spec = AudioSpec::new(
            audio.spec().sample_rate(),
            audio.channels(),
            audio.spec().sample_format(),
        );
        Ok(AudioBuffer::from_planar_f32(spec, output_frames, output)?)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "stretch output capacity is only a best-effort allocation hint derived from validated effect parameters"
)]
fn estimated_output_samples(
    input_frames: FrameCount,
    channels: usize,
    factor: f64,
) -> Result<usize> {
    let frames = ((input_frames.as_u64() as f64) * factor).ceil() as usize;
    frames
        .checked_mul(channels)
        .ok_or(EffectError::StretchLengthOverflow)
}

impl Default for Stretch {
    fn default() -> Self {
        Self::default_settings()
    }
}

#[derive(Debug, Clone)]
struct StretchState {
    segment: usize,
    ishift: usize,
    oshift: usize,
    overlap: usize,
    fade_coefs: Vec<f32>,
}

impl StretchState {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "SoX-ng derives stretch window and shift sizes by truncating validated floating-point values to size_t"
    )]
    fn new(stretch: Stretch, sample_rate: u32) -> Result<Self> {
        let segment = (f64::from(sample_rate) * 0.001 * stretch.window_ms) as usize;
        let (ishift, oshift) = if stretch.factor < 1.0 {
            let ishift = (stretch.shift * segment as f64) as usize;
            let oshift = (stretch.factor * ishift as f64) as usize;
            (ishift, oshift)
        } else {
            let oshift = (stretch.shift * segment as f64) as usize;
            let ishift = (oshift as f64 / stretch.factor) as usize;
            (ishift, oshift)
        };
        let overlap = (stretch.fading * segment as f64) as usize;

        if segment == 0 || ishift == 0 || oshift == 0 || ishift > segment || oshift > segment {
            return Err(EffectError::StretchLengthOverflow);
        }

        Ok(Self {
            segment,
            ishift,
            oshift,
            overlap,
            fade_coefs: fade_coefficients(stretch.fade, overlap),
        })
    }

    fn process_channel_into(&self, input: &[f32], output: &mut Vec<f32>) -> usize {
        let start_len = output.len();
        let mut machine = StretchMachine::new(self, output);
        machine.feed_all(input);
        machine.drain();
        machine.output.len() - start_len
    }
}

struct StretchMachine<'state, 'output> {
    state: &'state StretchState,
    input_state: bool,
    index: usize,
    oindex: usize,
    ibuf: Vec<f32>,
    obuf: Vec<f32>,
    output: &'output mut Vec<f32>,
}

impl<'state, 'output> StretchMachine<'state, 'output> {
    fn new(state: &'state StretchState, output: &'output mut Vec<f32>) -> Self {
        let index = state.segment / 2;
        Self {
            state,
            input_state: true,
            index,
            oindex: index,
            ibuf: vec![0.0; state.segment],
            obuf: vec![0.0; state.segment],
            output,
        }
    }

    fn feed_all(&mut self, input: &[f32]) {
        let mut input_index = 0;

        while input_index < input.len() {
            if self.input_state {
                let to_copy = (input.len() - input_index).min(self.state.segment - self.index);
                self.ibuf[self.index..self.index + to_copy]
                    .copy_from_slice(&input[input_index..input_index + to_copy]);
                input_index += to_copy;
                self.index += to_copy;

                if self.index == self.state.segment {
                    self.combine();
                    self.shift_input();
                    self.input_state = false;
                }
            }

            if !self.input_state {
                self.flush_output_shift();
            }
        }
    }

    fn drain(&mut self) {
        if self.input_state {
            self.ibuf[self.index..].fill(0.0);
            self.combine();
            self.input_state = false;
        }

        while self.oindex < self.index {
            self.push_output_sample(self.obuf[self.oindex]);
            self.oindex += 1;
        }
    }

    fn combine(&mut self) {
        let overlap = self.state.overlap;
        for i in 0..overlap {
            self.obuf[i] += self.state.fade_coefs[overlap - 1 - i] * self.ibuf[i];
        }
        for i in overlap..self.state.segment - overlap {
            self.obuf[i] += self.ibuf[i];
        }
        for i in self.state.segment - overlap..self.state.segment {
            self.obuf[i] += self.state.fade_coefs[i + overlap - self.state.segment] * self.ibuf[i];
        }
    }

    fn shift_input(&mut self) {
        let remaining = self.state.segment - self.state.ishift;
        self.ibuf.copy_within(self.state.ishift.., 0);
        self.index -= self.state.ishift;
        self.ibuf[remaining..].fill(0.0);
    }

    fn flush_output_shift(&mut self) {
        while self.oindex < self.state.oshift {
            self.push_output_sample(self.obuf[self.oindex]);
            self.oindex += 1;
        }

        if self.oindex >= self.state.oshift {
            self.oindex -= self.state.oshift;
            let remaining = self.state.segment - self.state.oshift;
            self.obuf.copy_within(self.state.oshift.., 0);
            self.obuf[remaining..].fill(0.0);
            self.input_state = true;
        }
    }

    fn push_output_sample(&mut self, sample: f32) {
        self.output.push(sample.clamp(-1.0, 1.0));
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "fade tables mirror SoX-ng's index-to-double coefficient formulas before storing f32 coefficients for f32 audio buffers"
)]
fn fade_coefficients(fade: StretchFade, overlap: usize) -> Vec<f32> {
    let mut coefs = vec![0.0; overlap];
    if overlap == 0 {
        return coefs;
    }
    coefs[0] = 1.0;
    if overlap == 1 {
        return coefs;
    }

    let slope = 1.0 / (overlap - 1) as f64;
    for (i, coef) in coefs.iter_mut().enumerate().take(overlap - 1).skip(1) {
        let reverse_fraction = slope * (overlap - i - 1) as f64;
        *coef = match fade {
            StretchFade::Linear => reverse_fraction,
            StretchFade::Sqrt => reverse_fraction.sqrt(),
            StretchFade::HalfCosine => {
                0.5 + (((i as f64) / (overlap - 1) as f64) * std::f64::consts::PI).cos() / 2.0
            }
            StretchFade::QuarterCosine => {
                (((i as f64) / (overlap - 1) as f64) * std::f64::consts::FRAC_PI_2).cos()
            }
        } as f32;
    }

    coefs
}

fn default_fading(factor: f64, shift: f64) -> f64 {
    let fading = if factor < 1.0 {
        1.0 - (factor * shift)
    } else {
        1.0 - shift
    };
    fading.min(0.5)
}

#[cfg(test)]
mod tests {
    use super::{Stretch, StretchFade};
    use crate::{
        EffectError,
        test_support::{audio_buffer, stereo_audio_buffer},
    };
    use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};

    #[test]
    fn factor_one_is_identity_copy() {
        let audio = audio_buffer(vec![-0.5, 0.0, 0.5]);

        let stretched = Stretch::default().process_buffer(&audio).unwrap();

        assert_eq!(stretched, audio);
    }

    #[test]
    fn lengthens_each_channel_with_default_slow_shift() {
        let audio = audio_buffer((0_u16..256).map(|frame| f32::from(frame) / 256.0).collect());
        let stretch = Stretch::with_options(2.0, 1.0, StretchFade::Linear, 1.0, 0.0).unwrap();

        let stretched = stretch.process_buffer(&audio).unwrap();

        assert!(stretched.frames().as_u64() > audio.frames().as_u64());
        assert_eq!(stretched.spec().sample_rate(), audio.spec().sample_rate());
        assert!(
            stretched
                .as_planar_f32()
                .iter()
                .all(|sample| sample.is_finite())
        );
    }

    #[test]
    fn preserves_stereo_planar_channel_shape() {
        let audio = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 0.75, -0.5, -0.25, 0.0, 0.25]);
        let stretch = Stretch::with_options(0.5, 1.0, StretchFade::Sqrt, 1.0, 0.25).unwrap();

        let stretched = stretch.process_buffer(&audio).unwrap();

        assert_eq!(stretched.channels(), audio.channels());
        assert_eq!(
            stretched.as_planar_f32().len(),
            stretched.channels().as_usize() * usize::try_from(stretched.frames().as_u64()).unwrap()
        );
    }

    #[test]
    fn rejects_invalid_values_and_degenerate_state() {
        assert_eq!(
            Stretch::new(f64::NAN, 20.0).unwrap_err(),
            EffectError::InvalidStretch
        );
        assert_eq!(
            Stretch::new(1.0, 0.5).unwrap_err(),
            EffectError::InvalidStretch
        );
        assert_eq!(
            Stretch::with_options(1.5, 1.0, StretchFade::Linear, 0.0, 0.0).unwrap_err(),
            EffectError::InvalidStretch
        );
        assert_eq!(
            Stretch::with_options(1.5, 1.0, StretchFade::Linear, 1.0, 0.75).unwrap_err(),
            EffectError::InvalidStretch
        );

        let spec = AudioSpec::new(
            SampleRate::new(1).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        );
        let audio = auralis_core::AudioBuffer::from_planar_f32(spec, FrameCount::new(1), vec![0.0])
            .unwrap();
        assert_eq!(
            Stretch::new(1.5, 1.0)
                .unwrap()
                .process_buffer(&audio)
                .unwrap_err(),
            EffectError::StretchLengthOverflow
        );
    }
}
