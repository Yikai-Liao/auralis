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
/// increases frame count. The tuning profile and optional
/// segment/search/overlap sizes use SoX-ng's millisecond units.
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
/// [`Self::with_tuning`] also returns [`EffectError::InvalidTempoTuning`] for
/// segment/search/overlap values outside SoX-ng's accepted ranges.
/// [`Self::process_buffer`] returns [`EffectError::TempoLengthOverflow`] when
/// the derived state or output shape cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tempo {
    /// Ratio of new tempo to old tempo.
    pub factor: f64,

    /// Whether to use SoX-ng's hierarchical quick overlap search.
    pub quick_search: bool,

    /// Tuning profile used to derive unspecified timing parameters.
    pub profile: TempoProfile,

    /// Explicit segment size in milliseconds.
    pub segment_ms: Option<f64>,

    /// Explicit overlap-search span in milliseconds.
    pub search_ms: Option<f64>,

    /// Explicit overlap size in milliseconds.
    pub overlap_ms: Option<f64>,
}

/// SoX-ng tuning profile for [`Tempo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempoProfile {
    /// Default SoX-ng tempo profile.
    Default,
    /// `tempo -m`, optimized for music.
    Music,
    /// `tempo -s`, optimized for speech.
    Speech,
    /// `tempo -l`, optimized for linear processing with a zero default search span.
    Linear,
}

impl Tempo {
    /// Creates a default-profile `tempo factor` processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidTempoFactor`] when `factor` is not finite
    /// or is outside SoX-ng's supported `0.1..=100` range.
    pub fn new(factor: f64) -> Result<Self> {
        Self::with_tuning(factor, false, TempoProfile::Default, None, None, None)
    }

    /// Creates a tempo processor using one of SoX-ng's named tuning profiles.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidTempoFactor`] when `factor` is not finite
    /// or is outside SoX-ng's supported `0.1..=100` range.
    pub fn with_profile(factor: f64, profile: TempoProfile) -> Result<Self> {
        Self::with_tuning(factor, false, profile, None, None, None)
    }

    /// Creates a tempo processor with explicit SoX-ng tuning options.
    ///
    /// `segment_ms`, `search_ms`, and `overlap_ms` are optional positional
    /// values in milliseconds and use SoX-ng's accepted ranges: segment
    /// `10..=120`, search `0..=30`, and overlap `0..=30`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidTempoFactor`] when `factor` is invalid or
    /// [`EffectError::InvalidTempoTuning`] when a supplied timing value is
    /// outside SoX-ng's supported range.
    pub fn with_tuning(
        factor: f64,
        quick_search: bool,
        profile: TempoProfile,
        segment_ms: Option<f64>,
        search_ms: Option<f64>,
        overlap_ms: Option<f64>,
    ) -> Result<Self> {
        if !factor.is_finite() || !(0.1..=100.0).contains(&factor) {
            return Err(EffectError::InvalidTempoFactor);
        }
        validate_optional_range(segment_ms, 10.0, 120.0)?;
        validate_optional_range(search_ms, 0.0, 30.0)?;
        validate_optional_range(overlap_ms, 0.0, 30.0)?;

        Ok(Self {
            factor,
            quick_search,
            profile,
            segment_ms,
            search_ms,
            overlap_ms,
        })
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
            self.quick_search,
            self.resolved_tuning(),
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

    fn resolved_tuning(self) -> TempoTuning {
        let segment_ms = self
            .segment_ms
            .unwrap_or_else(|| self.profile.default_segment_ms(self.factor));
        let search_ms = self
            .search_ms
            .unwrap_or_else(|| self.profile.default_search_ms(segment_ms));
        let overlap_ms = self
            .overlap_ms
            .unwrap_or_else(|| segment_ms / self.profile.overlap_divisor())
            .min(segment_ms / 2.0);

        TempoTuning {
            segment: segment_ms,
            search: search_ms,
            overlap: overlap_ms,
        }
    }
}

impl TempoProfile {
    pub(crate) fn default_segment_ms(self, factor: f64) -> f64 {
        let (base_ms, exponent) = match self {
            Self::Default => (DEFAULT_SEGMENT_MS, 0.0),
            Self::Music => (82.0, 1.0),
            Self::Speech => (35.0, 0.33),
            Self::Linear => (20.0, 1.0),
        };
        (base_ms / factor.powf(exponent).max(1.0)).max(10.0)
    }

    pub(crate) fn default_search_ms(self, segment_ms: f64) -> f64 {
        match self {
            Self::Linear => 0.0,
            _ => segment_ms / self.search_divisor(),
        }
    }

    fn search_divisor(self) -> f64 {
        match self {
            Self::Default => DEFAULT_SEARCH_DIVISOR,
            Self::Music => 6.0,
            Self::Speech => 2.14,
            Self::Linear => 2.0,
        }
    }

    fn overlap_divisor(self) -> f64 {
        match self {
            Self::Default => DEFAULT_OVERLAP_DIVISOR,
            Self::Music => 7.0,
            Self::Speech => 2.5,
            Self::Linear => 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TempoTuning {
    segment: f64,
    search: f64,
    overlap: f64,
}

#[derive(Debug, Clone)]
struct TempoState {
    channels: usize,
    factor: f64,
    quick_search: bool,
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
    fn new(
        sample_rate: u32,
        channels: usize,
        factor: f64,
        quick_search: bool,
        tuning: TempoTuning,
    ) -> Result<Self> {
        let segment = ((f64::from(sample_rate) * tuning.segment / 1000.0) + 0.5) as usize;
        let search = ((f64::from(sample_rate) * tuning.search / 1000.0) + 0.5) as usize;
        let mut overlap =
            (((f64::from(sample_rate) * tuning.overlap / 1000.0) + 4.5) as usize).max(16);
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

        if channels == 0 || segment == 0 || overlap == 0 || process_size == 0 {
            return Err(EffectError::TempoLengthOverflow);
        }

        Ok(Self {
            channels,
            factor,
            quick_search,
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
    input_start: usize,
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
            input_start: 0,
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
        self.input_len() / self.state.channels
    }

    fn best_overlap_position(&self) -> Result<usize> {
        if self.state.search == 0 {
            return Ok(0);
        }
        if self.state.quick_search {
            return self.quick_best_overlap_position();
        }

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

    fn quick_best_overlap_position(&self) -> Result<usize> {
        let mut prev_best_pos = (self.state.search + 1) >> 1;
        let mut best_pos = prev_best_pos;
        let mut least_diff = self.difference_at(best_pos)?;
        let mut step = 64_usize;

        loop {
            for subtract in [true, false] {
                let mut probe = 1_usize;
                while probe < 4 || step == 64 {
                    let distance = probe
                        .checked_mul(step)
                        .ok_or(EffectError::TempoLengthOverflow)?;
                    let Some(offset) = (if subtract {
                        prev_best_pos.checked_sub(distance)
                    } else {
                        prev_best_pos.checked_add(distance)
                    }) else {
                        break;
                    };
                    if offset >= self.state.search {
                        break;
                    }
                    let diff = self.difference_at(offset)?;
                    if diff < least_diff {
                        least_diff = diff;
                        best_pos = offset;
                    }
                    probe += 1;
                }
            }
            prev_best_pos = best_pos;
            step >>= 2;
            if step == 0 {
                break;
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
        let input_range = self.input_bounds(start, length)?;
        let input = &self.input_fifo[input_range];
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
        let chunk_range = self.input_bounds(start, length)?;
        let chunk = &self.input_fifo[chunk_range];
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
        let input_range = self.input_bounds(start, length)?;
        let input = &self.input_fifo[input_range];
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
        let chunk_range = self.input_bounds(start, length)?;
        let chunk = &self.input_fifo[chunk_range];
        self.output_fifo.extend(chunk.iter().copied().map(clip));
        Ok(())
    }

    fn save_overlap(&mut self, offset: usize) -> Result<()> {
        let overlap_start = offset
            .checked_add(self.state.segment - self.state.overlap)
            .ok_or(EffectError::TempoLengthOverflow)?;
        let start = self.state.wide_to_flat(overlap_start)?;
        let length = self.state.wide_to_flat(self.state.overlap)?;
        let input_range = self.input_bounds(start, length)?;
        let input = &self.input_fifo[input_range];
        self.overlap_buf.copy_from_slice(input);
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
        if skip_samples > self.input_len() {
            return Err(EffectError::TempoLengthOverflow);
        }
        self.input_start += skip_samples;
        self.compact_input_if_needed();
        Ok(())
    }

    fn input_len(&self) -> usize {
        self.input_fifo.len() - self.input_start
    }

    fn input_bounds(&self, start: usize, length: usize) -> Result<std::ops::Range<usize>> {
        let absolute_start = self
            .input_start
            .checked_add(start)
            .ok_or(EffectError::TempoLengthOverflow)?;
        let absolute_end = absolute_start
            .checked_add(length)
            .ok_or(EffectError::TempoLengthOverflow)?;
        if absolute_end <= self.input_fifo.len() {
            Ok(absolute_start..absolute_end)
        } else {
            Err(EffectError::TempoLengthOverflow)
        }
    }

    fn compact_input_if_needed(&mut self) {
        if self.input_start > self.input_fifo.len() / 2 {
            self.input_fifo.drain(..self.input_start);
            self.input_start = 0;
        }
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

fn validate_optional_range(value: Option<f64>, min: f64, max: f64) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_finite() && (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(EffectError::InvalidTempoTuning)
    }
}

#[cfg(test)]
mod tests {
    use super::{Tempo, TempoProfile};
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
    fn tuning_profiles_change_overlap_state_but_preserve_target_length() {
        let audio = audio_buffer(
            (0_u16..8192)
                .map(|frame| f32::from(frame % 128) / 128.0)
                .collect(),
        );

        let music = Tempo::with_profile(1.5, TempoProfile::Music)
            .unwrap()
            .process_buffer(&audio)
            .unwrap();
        let speech = Tempo::with_tuning(1.5, true, TempoProfile::Speech, None, None, None)
            .unwrap()
            .process_buffer(&audio)
            .unwrap();
        let linear = Tempo::with_profile(1.5, TempoProfile::Linear)
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(music.frames().as_u64(), 5461);
        assert_eq!(speech.frames().as_u64(), 5461);
        assert_eq!(linear.frames().as_u64(), 5461);
        assert!(music.as_planar_f32() != speech.as_planar_f32());
    }

    #[test]
    fn explicit_tuning_options_are_validated() {
        assert_eq!(
            Tempo::with_tuning(1.25, false, TempoProfile::Default, Some(9.0), None, None)
                .unwrap_err(),
            EffectError::InvalidTempoTuning
        );
        assert_eq!(
            Tempo::with_tuning(1.25, false, TempoProfile::Default, None, Some(31.0), None)
                .unwrap_err(),
            EffectError::InvalidTempoTuning
        );
        assert_eq!(
            Tempo::with_tuning(
                1.25,
                false,
                TempoProfile::Default,
                None,
                None,
                Some(f64::NAN)
            )
            .unwrap_err(),
            EffectError::InvalidTempoTuning
        );
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
