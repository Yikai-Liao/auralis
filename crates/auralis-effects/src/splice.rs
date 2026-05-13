use auralis_core::{AudioBuffer, AudioSpec, FrameCount};

use crate::{EffectError, Result};

const DEFAULT_EXCESS_SECONDS: f64 = 0.005;

/// SoX-ng `splice` cross-fade family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceFade {
    /// Half-sine equal-gain fade, SoX-ng's default `-h` mode.
    HalfSine,
    /// Triangular linear equal-gain fade.
    Triangular,
    /// Quarter-sine equal-power fade.
    QuarterSine,
}

/// A frame or seconds amount used by one `splice` point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpliceAmount {
    /// Direct frame count, matching SoX-ng's `s` suffix.
    Frames(FrameCount),
    /// Seconds resolved using the input sample rate.
    Seconds(f64),
}

/// One SoX-ng-style `splice` position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplicePosition {
    amount: SpliceAmount,
}

impl SplicePosition {
    /// Creates a frame-based splice position.
    #[must_use]
    pub const fn frames(frames: FrameCount) -> Self {
        Self {
            amount: SpliceAmount::Frames(frames),
        }
    }

    /// Creates a seconds-based splice position.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSplice`] when `seconds` is not finite or
    /// is negative.
    pub fn seconds(seconds: f64) -> Result<Self> {
        validate_seconds(seconds)?;
        Ok(Self {
            amount: SpliceAmount::Seconds(seconds),
        })
    }

    /// Returns the unresolved amount.
    #[must_use]
    pub const fn amount(self) -> SpliceAmount {
        self.amount
    }

    fn resolved(self, sample_rate_hz: u32) -> Result<FrameCount> {
        self.amount.resolved_frames(sample_rate_hz)
    }
}

impl SpliceAmount {
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
                    return Err(EffectError::SpliceLengthOverflow);
                }
                Ok(FrameCount::new(frames as u64))
            }
        }
    }
}

/// One unresolved `splice` point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplicePoint {
    /// Position of the splice, measured in the original input timeline.
    pub position: SplicePosition,
    /// Excess audio on both sides of the splice.
    pub excess: Option<SpliceAmount>,
    /// Search leeway before the second part.
    pub leeway: Option<SpliceAmount>,
}

impl SplicePoint {
    /// Creates one splice point.
    #[must_use]
    pub const fn new(
        position: SplicePosition,
        excess: Option<SpliceAmount>,
        leeway: Option<SpliceAmount>,
    ) -> Self {
        Self {
            position,
            excess,
            leeway,
        }
    }
}

/// SoX-ng-style audio splicing with cross-faded joins.
///
/// `Splice` removes excess audio around one or more original-input positions
/// and replaces the join with a short cross-fade. Processing is whole-buffer
/// and scalar because SoX-ng chooses each splice offset by searching for the
/// least-different overlap window across all channels.
#[derive(Debug, Clone, PartialEq)]
pub struct Splice {
    /// Cross-fade shape.
    pub fade: SpliceFade,
    points: Vec<SplicePoint>,
}

impl Splice {
    /// Creates a `splice` processor with the default half-sine fade.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSplice`] when no points are supplied.
    pub fn new<I>(points: I) -> Result<Self>
    where
        I: IntoIterator<Item = SplicePoint>,
    {
        Self::with_fade(SpliceFade::HalfSine, points)
    }

    /// Creates a `splice` processor with an explicit fade family.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSplice`] when no points are supplied.
    pub fn with_fade<I>(fade: SpliceFade, points: I) -> Result<Self>
    where
        I: IntoIterator<Item = SplicePoint>,
    {
        let points = points.into_iter().collect::<Vec<_>>();
        if points.is_empty() {
            return Err(EffectError::InvalidSplice);
        }
        Ok(Self { fade, points })
    }

    /// Returns the configured splice points.
    #[must_use]
    pub fn points(&self) -> &[SplicePoint] {
        &self.points
    }

    /// Applies all splice points to the decoded input buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSplice`] when resolved splice points are
    /// not strictly increasing or when the requested excess is longer than the
    /// splice position. Returns [`EffectError::SpliceLengthOverflow`] when the
    /// derived buffer geometry cannot be represented.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let resolved = self.resolved_points(audio.spec().sample_rate().as_u32())?;
        let frames = usize::try_from(audio.frames().as_u64())
            .map_err(|_| EffectError::SpliceLengthOverflow)?;
        let channels = audio.channels().as_usize();
        let plan = splice_plan(audio, resolved, frames)?;
        let mut output = Vec::with_capacity(audio.as_planar_f32().len());
        let mut output_frames = None;

        for channel_index in 0..channels {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::SpliceLengthOverflow)?;
            let channel_start_len = output.len();
            flush_channel(channel, &plan, self.fade, frames, &mut output)?;
            let channel_frames = output.len() - channel_start_len;
            output_frames.get_or_insert(channel_frames);
            debug_assert_eq!(output_frames, Some(channel_frames));
        }

        let output_frames = output_frames
            .unwrap_or(0)
            .try_into()
            .map(FrameCount::new)
            .map_err(|_| EffectError::SpliceLengthOverflow)?;
        AudioBuffer::from_planar_f32(
            AudioSpec::new(
                audio.spec().sample_rate(),
                audio.channels(),
                audio.spec().sample_format(),
            ),
            output_frames,
            output,
        )
        .map_err(|_| EffectError::SpliceLengthOverflow)
    }

    fn resolved_points(&self, sample_rate_hz: u32) -> Result<Vec<ResolvedSplicePoint>> {
        let mut resolved = Vec::with_capacity(self.points.len());
        let mut previous_position = 0_u64;

        for point in &self.points {
            let position = point.position.resolved(sample_rate_hz)?.as_u64();
            if !resolved.is_empty() && position <= previous_position {
                return Err(EffectError::InvalidSplice);
            }
            previous_position = position;

            let default_excess = SpliceAmount::Seconds(DEFAULT_EXCESS_SECONDS)
                .resolved_frames(sample_rate_hz)?
                .as_u64();
            let mut overlap = match point.excess {
                Some(excess) => excess
                    .resolved_frames(sample_rate_hz)?
                    .as_u64()
                    .checked_mul(2)
                    .ok_or(EffectError::SpliceLengthOverflow)?,
                None => default_excess
                    .checked_mul(2)
                    .ok_or(EffectError::SpliceLengthOverflow)?,
            };
            let mut search = if self.fade == SpliceFade::QuarterSine {
                0
            } else {
                default_excess
                    .checked_mul(2)
                    .ok_or(EffectError::SpliceLengthOverflow)?
            };
            if let Some(leeway) = point.leeway {
                search = leeway
                    .resolved_frames(sample_rate_hz)?
                    .as_u64()
                    .checked_mul(2)
                    .ok_or(EffectError::SpliceLengthOverflow)?;
            }

            overlap = overlap
                .checked_add(4)
                .ok_or(EffectError::SpliceLengthOverflow)?
                .max(16)
                & !7;
            if position < overlap {
                return Err(EffectError::InvalidSplice);
            }

            resolved.push(ResolvedSplicePoint {
                start: usize::try_from(position - overlap)
                    .map_err(|_| EffectError::SpliceLengthOverflow)?,
                overlap: usize::try_from(overlap).map_err(|_| EffectError::SpliceLengthOverflow)?,
                search: usize::try_from(search).map_err(|_| EffectError::SpliceLengthOverflow)?,
            });
        }

        Ok(resolved)
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedSplicePoint {
    start: usize,
    overlap: usize,
    search: usize,
}

#[derive(Debug, Clone, Copy)]
struct SpliceAction {
    point: ResolvedSplicePoint,
    offset: usize,
    buffer_end: usize,
}

struct SplicePlan {
    actions: Vec<SpliceAction>,
    tail_start: usize,
}

fn splice_plan(
    audio: &AudioBuffer,
    resolved: Vec<ResolvedSplicePoint>,
    frames: usize,
) -> Result<SplicePlan> {
    let mut actions = Vec::with_capacity(resolved.len());
    let mut cursor = 0_usize;

    for point in resolved {
        if point.start >= frames {
            break;
        }
        if point.start < cursor {
            continue;
        }

        let Some(buffer_end) = point
            .start
            .checked_add(
                point
                    .overlap
                    .checked_mul(2)
                    .ok_or(EffectError::SpliceLengthOverflow)?,
            )
            .and_then(|end| end.checked_add(point.search))
        else {
            return Err(EffectError::SpliceLengthOverflow);
        };

        if buffer_end > frames {
            cursor = point.start;
            break;
        }

        let offset = if point.search == 0 {
            0
        } else {
            best_overlap_position(audio, point.start, point.overlap, point.search)
        };
        actions.push(SpliceAction {
            point,
            offset,
            buffer_end,
        });
        cursor = buffer_end;
    }

    Ok(SplicePlan {
        actions,
        tail_start: cursor,
    })
}

fn flush_channel(
    channel: &[f32],
    plan: &SplicePlan,
    fade: SpliceFade,
    frames: usize,
    output: &mut Vec<f32>,
) -> Result<()> {
    let mut cursor = 0_usize;
    for action in &plan.actions {
        output.extend_from_slice(&channel[cursor..action.point.start]);
        flush_channel_splice(channel, output, fade, action.point, action.offset)?;
        cursor = action.buffer_end;
    }
    debug_assert!(plan.tail_start >= cursor);
    output.extend_from_slice(&channel[plan.tail_start..frames]);
    Ok(())
}

fn flush_channel_splice(
    channel: &[f32],
    output: &mut Vec<f32>,
    fade: SpliceFade,
    point: ResolvedSplicePoint,
    offset: usize,
) -> Result<()> {
    let flush_start = point
        .overlap
        .checked_add(offset)
        .ok_or(EffectError::SpliceLengthOverflow)?;
    let crossfade_end = flush_start
        .checked_add(point.overlap)
        .ok_or(EffectError::SpliceLengthOverflow)?;
    let buffer_end = point
        .overlap
        .checked_mul(2)
        .and_then(|end| end.checked_add(point.search))
        .ok_or(EffectError::SpliceLengthOverflow)?;

    for local_frame in flush_start..buffer_end {
        let source_frame = point
            .start
            .checked_add(local_frame)
            .ok_or(EffectError::SpliceLengthOverflow)?;
        let sample = if local_frame < crossfade_end {
            let overlap_index = local_frame - flush_start;
            let in1 = channel[point.start + overlap_index];
            let in2 = channel[source_frame];
            crossfade_sample(fade, overlap_index, point.overlap, in1, in2)
        } else {
            channel[source_frame]
        };
        output.push(sample);
    }

    Ok(())
}

fn best_overlap_position(
    audio: &AudioBuffer,
    start_frame: usize,
    overlap: usize,
    search: usize,
) -> usize {
    let mut best_offset = 0;
    let mut least_difference = difference(audio, start_frame, start_frame + overlap, overlap);

    for offset in 1..search {
        let diff = difference(audio, start_frame, start_frame + overlap + offset, overlap);
        if diff < least_difference {
            least_difference = diff;
            best_offset = offset;
        }
    }

    best_offset
}

fn difference(audio: &AudioBuffer, left: usize, right: usize, overlap: usize) -> f64 {
    let mut diff = 0.0;
    for channel_index in 0..audio.channels().as_usize() {
        let channel = audio
            .channel(channel_index)
            .expect("channel index is within input channel count");
        for index in 0..overlap {
            let delta = f64::from(channel[left + index]) - f64::from(channel[right + index]);
            diff += delta * delta;
        }
    }
    diff
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "cross-fade math is bounded to normalized f32 audio before conversion"
)]
fn crossfade_sample(fade: SpliceFade, index: usize, overlap: usize, in1: f32, in2: f32) -> f32 {
    let phase = index as f64 / overlap as f64;
    let (fade_in, fade_out) = match fade {
        SpliceFade::QuarterSine => {
            let angle = phase * std::f64::consts::FRAC_PI_2;
            (angle.sin(), angle.cos())
        }
        SpliceFade::HalfSine => {
            let fade_in = 0.5 - 0.5 * (phase * std::f64::consts::PI).cos();
            (fade_in, 1.0 - fade_in)
        }
        SpliceFade::Triangular => (phase, 1.0 - phase),
    };
    (f64::from(in1).mul_add(fade_out, f64::from(in2) * fade_in)).clamp(-1.0, 1.0) as f32
}

fn validate_seconds(seconds: f64) -> Result<()> {
    if seconds.is_finite() && seconds >= 0.0 {
        Ok(())
    } else {
        Err(EffectError::InvalidSplice)
    }
}

#[cfg(test)]
mod tests {
    use auralis_core::{AudioSpec, ChannelCount, SampleFormat, SampleRate};

    use super::{Splice, SpliceAmount, SpliceFade, SplicePoint, SplicePosition};
    use crate::EffectError;

    #[test]
    fn splice_removes_overlap_and_crossfades_the_join() {
        let audio = mono_audio((0_u16..64).map(|value| f32::from(value) / 64.0).collect());
        let spliced = Splice::with_fade(
            SpliceFade::Triangular,
            [SplicePoint::new(
                SplicePosition::frames(FrameCount::new(32)),
                Some(SpliceAmount::Frames(FrameCount::new(4))),
                Some(SpliceAmount::Frames(FrameCount::new(0))),
            )],
        )
        .unwrap()
        .process_buffer(&audio)
        .unwrap();

        assert_eq!(spliced.frames().as_u64(), 48);
        assert_eq!(&spliced.as_planar_f32()[..16], &audio.as_planar_f32()[..16]);
        assert_eq!(
            spliced.as_planar_f32()[16].to_bits(),
            audio.as_planar_f32()[16].to_bits()
        );
        assert!(spliced.as_planar_f32()[24] > audio.as_planar_f32()[24]);
    }

    #[test]
    fn quarter_sine_uses_zero_default_search() {
        let audio = mono_audio(
            (0_u16..2048)
                .map(|value| f32::from(value % 128) / 128.0)
                .collect(),
        );
        let default_search = Splice::with_fade(
            SpliceFade::QuarterSine,
            [SplicePoint::new(
                SplicePosition::frames(FrameCount::new(1000)),
                None,
                None,
            )],
        )
        .unwrap()
        .process_buffer(&audio)
        .unwrap();

        assert_eq!(default_search.frames().as_u64(), 1568);
    }

    #[test]
    fn rejects_empty_or_unordered_splices() {
        assert_eq!(Splice::new([]).unwrap_err(), EffectError::InvalidSplice);

        let audio = mono_audio(vec![0.0; 128]);
        assert_eq!(
            Splice::new([
                SplicePoint::new(SplicePosition::frames(FrameCount::new(48)), None, None),
                SplicePoint::new(SplicePosition::frames(FrameCount::new(48)), None, None),
            ])
            .unwrap()
            .process_buffer(&audio)
            .unwrap_err(),
            EffectError::InvalidSplice
        );
    }

    fn mono_audio(samples: Vec<f32>) -> AudioBuffer {
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

    use auralis_core::{AudioBuffer, FrameCount};
}
