use auralis_core::{AudioBuffer, AudioSpec, FrameCount};

use crate::{EffectError, Result};

const DEFAULT_SEGMENT_MS: f64 = 82.0;
const DEFAULT_OVERLAP_DIVISOR: f64 = 6.833;
const DEFAULT_SEARCH_DIVISOR: f64 = 5.587;

/// SoX-ng-style tempo adjustment that preserves pitch.
///
/// `Tempo` changes decoded duration by overlap-adding similar windows while
/// keeping the input sample rate unchanged. A factor greater than `1` speeds
/// audio up and reduces frame count; a factor below `1` slows audio down and
/// increases frame count. Feature 6.6.9 implements the default SoX-ng profile;
/// tuning flags and explicit segment/search/overlap options are reserved for
/// the following roadmap feature.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Tempo;
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
/// let faster = Tempo::new(1.25)?.process_buffer(&audio)?;
///
/// assert_eq!(faster.spec().sample_rate(), audio.spec().sample_rate());
/// assert!(faster.frames().as_u64() < audio.frames().as_u64());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidTempoFactor`] for non-finite
/// factors or values outside SoX-ng's `0.1..=100` range.
/// [`Self::process_buffer`] returns [`EffectError::TempoLengthOverflow`] when
/// the derived state or output shape cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tempo {
    /// Ratio of new tempo to old tempo.
    pub factor: f64,
}

impl Tempo {
    /// Creates a default-profile `tempo factor` processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidTempoFactor`] when `factor` is not finite
    /// or is outside SoX-ng's supported `0.1..=100` range.
    pub fn new(factor: f64) -> Result<Self> {
        if !factor.is_finite() || !(0.1..=100.0).contains(&factor) {
            return Err(EffectError::InvalidTempoFactor);
        }
        Ok(Self { factor })
    }

    /// Applies default-profile tempo processing.
    ///
    /// Factor `1` is a null effect and returns an identity copy.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::TempoLengthOverflow`] when the derived window
    /// sizes or output buffer shape cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        if self.factor.to_bits() == 1.0_f64.to_bits() {
            return Ok(audio.clone());
        }

        let state = TempoState::new(
            audio.spec().sample_rate().as_u32(),
            audio.channels().as_usize(),
            self.factor,
        )?;
        let interleaved = interleave(audio)?;
        let output = state.process(&interleaved, audio.frames())?;
        let output_frames = FrameCount::new(
            u64::try_from(output.len() / audio.channels().as_usize())
                .map_err(|_| EffectError::TempoLengthOverflow)?,
        );
        let planar = deinterleave(&output, audio.channels().as_usize(), output_frames)?;
        let spec = AudioSpec::new(
            audio.spec().sample_rate(),
            audio.channels(),
            audio.spec().sample_format(),
        );

        Ok(AudioBuffer::from_planar_f32(spec, output_frames, planar)?)
    }
}

#[derive(Debug, Clone)]
struct TempoState {
    channels: usize,
    factor: f64,
    search: usize,
    segment: usize,
    overlap: usize,
    process_size: usize,
}

impl TempoState {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "SoX-ng derives tempo window sizes by rounding validated floating-point sample counts to size_t"
    )]
    fn new(sample_rate: u32, channels: usize, factor: f64) -> Result<Self> {
        let segment = ((f64::from(sample_rate) * DEFAULT_SEGMENT_MS / 1000.0) + 0.5) as usize;
        let search_ms = DEFAULT_SEGMENT_MS / DEFAULT_SEARCH_DIVISOR;
        let search = ((f64::from(sample_rate) * search_ms / 1000.0) + 0.5) as usize;
        let overlap_ms = DEFAULT_SEGMENT_MS / DEFAULT_OVERLAP_DIVISOR;
        let mut overlap = (((f64::from(sample_rate) * overlap_ms / 1000.0) + 4.5) as usize).max(16);
        overlap &= !7;
        if overlap * 2 > segment {
            overlap = overlap
                .checked_sub(8)
                .ok_or(EffectError::TempoLengthOverflow)?;
        }
        let max_skip = (factor * (segment - overlap) as f64).ceil() as usize;
        let process_size = max_skip
            .checked_add(overlap)
            .map(|with_overlap| with_overlap.max(segment))
            .and_then(|size| size.checked_add(search))
            .ok_or(EffectError::TempoLengthOverflow)?;

        if channels == 0 || search == 0 || segment == 0 || overlap == 0 || process_size == 0 {
            return Err(EffectError::TempoLengthOverflow);
        }

        Ok(Self {
            channels,
            factor,
            search,
            segment,
            overlap,
            process_size,
        })
    }

    fn process(&self, input: &[f32], input_frames: FrameCount) -> Result<Vec<f32>> {
        let mut machine = TempoMachine::new(self);
        machine.feed(input)?;
        machine.process()?;
        machine.flush(input_frames)?;
        Ok(machine.output_fifo)
    }

    fn wide_to_flat(&self, wide_index: usize) -> Result<usize> {
        wide_index
            .checked_mul(self.channels)
            .ok_or(EffectError::TempoLengthOverflow)
    }
}

struct TempoMachine<'state> {
    state: &'state TempoState,
    input_fifo: Vec<f32>,
    output_fifo: Vec<f32>,
    overlap_buf: Vec<f32>,
    segments_total: u64,
    skip_total: u64,
}

impl<'state> TempoMachine<'state> {
    fn new(state: &'state TempoState) -> Self {
        let input_fifo = vec![0.0; (state.search / 2) * state.channels];
        Self {
            state,
            input_fifo,
            output_fifo: Vec::new(),
            overlap_buf: vec![0.0; state.overlap * state.channels],
            segments_total: 0,
            skip_total: 0,
        }
    }

    fn feed(&mut self, input: &[f32]) -> Result<()> {
        self.input_fifo
            .try_reserve(input.len())
            .map_err(|_| EffectError::TempoLengthOverflow)?;
        self.input_fifo.extend_from_slice(input);
        Ok(())
    }

    fn flush(&mut self, input_frames: FrameCount) -> Result<()> {
        let target_frames = rounded_output_frames(input_frames, self.state.factor)?;
        let target_samples = self.state.wide_to_flat(target_frames)?;
        let zeros = vec![0.0; 128 * self.state.channels];
        while self.output_fifo.len() < target_samples {
            self.feed(&zeros)?;
            self.process()?;
        }
        self.output_fifo.truncate(target_samples);
        Ok(())
    }

    fn process(&mut self) -> Result<()> {
        while self.input_frames() >= self.state.process_size {
            let offset = if self.segments_total == 0 {
                self.state.search / 2
            } else {
                self.best_overlap_position()?
            };

            if self.segments_total == 0 {
                self.copy_overlap_to_output(offset)?;
            } else {
                self.overlap_to_output(offset)?;
            }
            self.copy_middle_to_output(offset)?;
            self.save_overlap(offset)?;
            self.advance_input()?;
        }

        Ok(())
    }

    fn input_frames(&self) -> usize {
        self.input_fifo.len() / self.state.channels
    }

    fn best_overlap_position(&self) -> Result<usize> {
        let mut best_pos = 0;
        let mut least_diff = self.difference_at(0)?;
        for offset in 1..self.state.search {
            let diff = self.difference_at(offset)?;
            if diff < least_diff {
                least_diff = diff;
                best_pos = offset;
            }
        }
        Ok(best_pos)
    }

    fn difference_at(&self, offset: usize) -> Result<f32> {
        let start = self.state.wide_to_flat(offset)?;
        let length = self
            .state
            .wide_to_flat(self.state.overlap)
            .map_err(|_| EffectError::TempoLengthOverflow)?;
        let input = self
            .input_fifo
            .get(start..start + length)
            .ok_or(EffectError::TempoLengthOverflow)?;
        Ok(input
            .iter()
            .zip(&self.overlap_buf)
            .map(|(left, right)| {
                let delta = left - right;
                delta * delta
            })
            .sum())
    }

    fn copy_overlap_to_output(&mut self, offset: usize) -> Result<()> {
        let start = self.state.wide_to_flat(offset)?;
        let length = self.state.wide_to_flat(self.state.overlap)?;
        let chunk = self
            .input_fifo
            .get(start..start + length)
            .ok_or(EffectError::TempoLengthOverflow)?;
        self.output_fifo.extend(chunk.iter().copied().map(clip));
        Ok(())
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "SoX-ng tempo overlap computes f32 fade coefficients from sample indexes"
    )]
    fn overlap_to_output(&mut self, offset: usize) -> Result<()> {
        let start = self.state.wide_to_flat(offset)?;
        let length = self.state.wide_to_flat(self.state.overlap)?;
        let input = self
            .input_fifo
            .get(start..start + length)
            .ok_or(EffectError::TempoLengthOverflow)?;
        let fade_step = 1.0_f32 / self.state.overlap as f32;
        for frame in 0..self.state.overlap {
            let fade_in = fade_step * frame as f32;
            let fade_out = 1.0 - fade_in;
            for channel in 0..self.state.channels {
                let index = frame * self.state.channels + channel;
                let sample = self.overlap_buf[index] * fade_out + input[index] * fade_in;
                self.output_fifo.push(clip(sample));
            }
        }
        Ok(())
    }

    fn copy_middle_to_output(&mut self, offset: usize) -> Result<()> {
        let middle_start = offset
            .checked_add(self.state.overlap)
            .ok_or(EffectError::TempoLengthOverflow)?;
        let middle_frames = self
            .state
            .segment
            .checked_sub(2 * self.state.overlap)
            .ok_or(EffectError::TempoLengthOverflow)?;
        let start = self.state.wide_to_flat(middle_start)?;
        let length = self.state.wide_to_flat(middle_frames)?;
        let chunk = self
            .input_fifo
            .get(start..start + length)
            .ok_or(EffectError::TempoLengthOverflow)?;
        self.output_fifo.extend(chunk.iter().copied().map(clip));
        Ok(())
    }

    fn save_overlap(&mut self, offset: usize) -> Result<()> {
        let overlap_start = offset
            .checked_add(self.state.segment - self.state.overlap)
            .ok_or(EffectError::TempoLengthOverflow)?;
        let start = self.state.wide_to_flat(overlap_start)?;
        let length = self.state.wide_to_flat(self.state.overlap)?;
        self.overlap_buf.copy_from_slice(
            self.input_fifo
                .get(start..start + length)
                .ok_or(EffectError::TempoLengthOverflow)?,
        );
        Ok(())
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "SoX-ng accumulates rounded tempo skip positions with double arithmetic"
    )]
    fn advance_input(&mut self) -> Result<()> {
        self.segments_total += 1;
        let target_skip_total = (self.state.factor
            * self.segments_total as f64
            * (self.state.segment - self.state.overlap) as f64
            + 0.5) as u64;
        let skip = target_skip_total
            .checked_sub(self.skip_total)
            .ok_or(EffectError::TempoLengthOverflow)?;
        self.skip_total = target_skip_total;
        let skip_samples = self
            .state
            .wide_to_flat(usize::try_from(skip).map_err(|_| EffectError::TempoLengthOverflow)?)?;
        if skip_samples > self.input_fifo.len() {
            return Err(EffectError::TempoLengthOverflow);
        }
        self.input_fifo.drain(..skip_samples);
        Ok(())
    }
}

fn interleave(audio: &AudioBuffer) -> Result<Vec<f32>> {
    let channels = audio.channels().as_usize();
    let frames =
        usize::try_from(audio.frames().as_u64()).map_err(|_| EffectError::TempoLengthOverflow)?;
    let mut interleaved = Vec::with_capacity(audio.as_planar_f32().len());
    for frame in 0..frames {
        for channel in 0..channels {
            let samples = audio
                .channel(channel)
                .ok_or(EffectError::TempoLengthOverflow)?;
            interleaved.push(samples[frame]);
        }
    }
    Ok(interleaved)
}

fn deinterleave(input: &[f32], channels: usize, frames: FrameCount) -> Result<Vec<f32>> {
    let frames_usize =
        usize::try_from(frames.as_u64()).map_err(|_| EffectError::TempoLengthOverflow)?;
    let mut planar = vec![0.0; input.len()];
    for frame in 0..frames_usize {
        for channel in 0..channels {
            planar[channel * frames_usize + frame] = input[frame * channels + channel];
        }
    }
    Ok(planar)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "SoX-ng rounds known input frame counts with double arithmetic"
)]
fn rounded_output_frames(input_frames: FrameCount, factor: f64) -> Result<usize> {
    let rounded = (input_frames.as_u64() as f64 / factor + 0.5) as u64;
    usize::try_from(rounded).map_err(|_| EffectError::TempoLengthOverflow)
}

fn clip(sample: f32) -> f32 {
    sample.clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::Tempo;
    use crate::{
        EffectError,
        test_support::{audio_buffer, stereo_audio_buffer},
    };

    #[test]
    fn factor_one_is_identity_copy() {
        let audio = audio_buffer(vec![-0.5, 0.0, 0.5]);

        let processed = Tempo::new(1.0).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(processed, audio);
    }

    #[test]
    fn factor_above_one_shortens_audio_without_changing_rate() {
        let audio = audio_buffer(
            (0_u16..8192)
                .map(|frame| f32::from(frame % 128) / 128.0)
                .collect(),
        );

        let processed = Tempo::new(1.5).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(processed.frames().as_u64(), 5461);
        assert_eq!(processed.spec().sample_rate(), audio.spec().sample_rate());
        assert!(
            processed
                .as_planar_f32()
                .iter()
                .all(|sample| sample.is_finite())
        );
    }

    #[test]
    fn factor_below_one_lengthens_stereo_audio() {
        let mut samples = (0_u16..8192)
            .map(|frame| f32::from(frame % 128) / 128.0)
            .collect::<Vec<_>>();
        samples.extend((0_u16..8192).map(|frame| -f32::from(frame % 128) / 128.0));
        let audio = stereo_audio_buffer(samples);

        let processed = Tempo::new(0.75).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(processed.channels(), audio.channels());
        assert_eq!(processed.frames().as_u64(), 10923);
    }

    #[test]
    fn rejects_invalid_factor() {
        assert_eq!(
            Tempo::new(f64::NAN).unwrap_err(),
            EffectError::InvalidTempoFactor
        );
        assert_eq!(
            Tempo::new(0.01).unwrap_err(),
            EffectError::InvalidTempoFactor
        );
        assert_eq!(
            Tempo::new(101.0).unwrap_err(),
            EffectError::InvalidTempoFactor
        );
    }
}
