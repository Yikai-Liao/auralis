use auralis_core::AudioBuffer;

use crate::{EffectError, Result};

const MIN_EFFECTIVE_SOFT_KNEE_DB: f64 = 0.01;
const DB_TO_NATURAL_LOG: f64 = std::f64::consts::LN_10 / 20.0;
pub(crate) const SOX_SAMPLE_MIN_DB: f64 = -186.638_597_306_397_2;

/// Attack and decay times for one SoX-ng `compand` channel group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompandAttackDecay {
    /// Attack time in seconds.
    pub attack_seconds: f64,
    /// Decay time in seconds.
    pub decay_seconds: f64,
}

impl CompandAttackDecay {
    /// Creates one attack/decay timing pair.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidCompand`] when either value is not finite
    /// or is negative.
    pub fn new(attack_seconds: f64, decay_seconds: f64) -> Result<Self> {
        if !is_non_negative_finite(attack_seconds) || !is_non_negative_finite(decay_seconds) {
            return Err(EffectError::InvalidCompand);
        }

        Ok(Self {
            attack_seconds,
            decay_seconds,
        })
    }
}

/// One point in a SoX-ng `compand` transfer curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompandTransferPoint {
    /// Input level in dB relative to full scale. SoX-ng requires this to be
    /// less than or equal to 0 dB.
    pub input_db: f64,
    /// Output level in dB relative to full scale. A missing value is only valid
    /// for the first parsed SoX-ng point and means "use 0 dB gain at this
    /// point" in the transfer-function table.
    pub output_db: Option<f64>,
}

impl CompandTransferPoint {
    /// Creates an explicit input/output transfer point.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidCompand`] when either value is not finite
    /// or is above 0 dBFS.
    pub fn new(input_db: f64, output_db: f64) -> Result<Self> {
        validate_transfer_db(input_db)?;
        validate_transfer_db(output_db)?;
        Ok(Self {
            input_db,
            output_db: Some(output_db),
        })
    }

    /// Creates the first SoX-ng input-only transfer point.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidCompand`] when `input_db` is not finite or
    /// is above 0 dBFS.
    pub fn input_only(input_db: f64) -> Result<Self> {
        validate_transfer_db(input_db)?;
        Ok(Self {
            input_db,
            output_db: None,
        })
    }
}

/// Parsed SoX-ng `compand` transfer function.
#[derive(Debug, Clone, PartialEq)]
pub struct CompandTransfer {
    soft_knee_db: Option<f64>,
    points: Vec<CompandTransferPoint>,
    segments: Vec<TransferSegment>,
    in_min_linear: f64,
    out_min_linear: f64,
}

impl CompandTransfer {
    /// Creates a transfer curve from SoX-ng-style dB points.
    ///
    /// `soft_knee_db` preserves whether the command used an explicit
    /// `soft-knee-dB:` prefix. The numerical transfer follows SoX-ng by using a
    /// minimum effective knee of 0.01 dB.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidCompand`] when there are no points, point
    /// levels are invalid, point inputs are not strictly increasing, or the
    /// soft knee is not finite.
    pub fn new<I>(points: I, soft_knee_db: Option<f64>) -> Result<Self>
    where
        I: IntoIterator<Item = CompandTransferPoint>,
    {
        Self::with_post_gain(points, soft_knee_db, 0.0)
    }

    fn with_post_gain<I>(points: I, soft_knee_db: Option<f64>, gain_db: f64) -> Result<Self>
    where
        I: IntoIterator<Item = CompandTransferPoint>,
    {
        if soft_knee_db.is_some_and(|value| !value.is_finite()) || !gain_db.is_finite() {
            return Err(EffectError::InvalidCompand);
        }

        let points = points.into_iter().collect::<Vec<_>>();
        validate_points(&points)?;

        let effective_knee_db = soft_knee_db.unwrap_or(0.0).max(MIN_EFFECTIVE_SOFT_KNEE_DB);
        let mut raw_points = raw_transfer_points(&points, effective_knee_db);
        join_colinear_points(&mut raw_points);
        let (segments, in_min_linear, out_min_linear) =
            prepare_segments(&raw_points, gain_db, effective_knee_db);

        Ok(Self {
            soft_knee_db,
            points,
            segments,
            in_min_linear,
            out_min_linear,
        })
    }

    /// Parses a SoX-ng transfer string:
    /// `[soft-knee-dB:]in-dB1[,out-dB1]{,in-dB2,out-dB2}`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidCompand`] when the transfer string cannot
    /// be parsed as a finite SoX-ng transfer function.
    pub fn parse_sox_transfer(value: &str) -> Result<Self> {
        let (soft_knee_db, points) = parse_transfer_points(value)?;
        Self::new(points, soft_knee_db)
    }

    /// Returns the explicit soft-knee prefix in dB, if one was supplied.
    #[must_use]
    pub const fn soft_knee_db(&self) -> Option<f64> {
        self.soft_knee_db
    }

    /// Returns the parsed transfer points.
    #[must_use]
    pub fn points(&self) -> &[CompandTransferPoint] {
        &self.points
    }

    /// Returns the linear gain produced for an input level in `0..=1`.
    #[must_use]
    pub fn linear_gain_for_level(&self, input_linear: f64) -> f64 {
        if input_linear <= self.in_min_linear {
            return self.out_min_linear;
        }

        let input_log = input_linear.clamp(f64::MIN_POSITIVE, 1.0).ln();
        let mut index = 1;
        while index + 1 < self.segments.len() && input_log > self.segments[index + 1].x {
            index += 1;
        }
        let segment = self.segments[index];
        let offset = input_log - segment.x;
        (segment.y + offset * ((segment.a * offset) + segment.b)).exp()
    }

    fn is_unity_gain(&self) -> bool {
        is_exact_one(self.out_min_linear)
            && self.segments.iter().all(|segment| {
                is_exact_zero(segment.y) && is_exact_zero(segment.a) && is_exact_zero(segment.b)
            })
    }

    /// Returns the output level in dBFS for an input level in dBFS.
    #[must_use]
    pub fn output_db_for_input_db(&self, input_db: f64) -> f64 {
        let input_linear = 10.0_f64.powf(input_db / 20.0);
        input_db + linear_to_db(self.linear_gain_for_level(input_linear))
    }
}

/// SoX-ng-style dynamic-range compander.
///
/// `Compand` tracks an envelope follower per channel, or one shared follower
/// for all channels when the command supplied only one attack/decay pair. The
/// current envelope is evaluated through the configured dB transfer function
/// and multiplied into either the current sample or a delayed look-ahead sample.
/// Processing is length-preserving: delay changes which source sample receives
/// the current gain, then drains the buffered input without extending duration.
///
/// # Errors
///
/// [`Self::process_buffer`] returns [`EffectError::InvalidCompand`] when the
/// command supplied multiple attack/decay pairs that do not match the input
/// channel count, or when delay resolution cannot be represented.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Compand;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(2),
///     vec![0.25, -0.25],
/// )?;
/// let compand = Compand::parse_sox_args(&["0,0", "-60,-60,0,0"])?;
///
/// compand.process_buffer(&mut audio)?;
///
/// assert_eq!(audio.as_planar_f32(), &[0.25, -0.25]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Compand {
    attack_decay: Vec<CompandAttackDecay>,
    transfer: CompandTransfer,
    /// Post-processing gain in dB.
    pub gain_db: f64,
    /// Initial volume estimate in dBFS.
    pub initial_volume_db: f64,
    /// Look-ahead delay in seconds.
    pub delay_seconds: f64,
}

impl Compand {
    /// Creates a typed `compand` configuration.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidCompand`] when no timing pairs are
    /// supplied or when gain, initial volume, or delay values are invalid.
    pub fn new<I>(
        attack_decay: I,
        transfer: CompandTransfer,
        gain_db: f64,
        initial_volume_db: f64,
        delay_seconds: f64,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = CompandAttackDecay>,
    {
        let attack_decay = attack_decay.into_iter().collect::<Vec<_>>();
        if attack_decay.is_empty()
            || !gain_db.is_finite()
            || !initial_volume_db.is_finite()
            || initial_volume_db > 0.0
            || !is_non_negative_finite(delay_seconds)
        {
            return Err(EffectError::InvalidCompand);
        }

        let CompandTransfer {
            soft_knee_db,
            points,
            ..
        } = transfer;
        let transfer =
            CompandTransfer::with_post_gain(points.iter().copied(), soft_knee_db, gain_db)?;

        Ok(Self {
            attack_decay,
            transfer,
            gain_db,
            initial_volume_db,
            delay_seconds,
        })
    }

    /// Parses SoX-ng `compand` arguments without enabling audio processing.
    ///
    /// The accepted form is
    /// `attack,decay{,attack,decay} [soft-knee-dB:]in-dB1[,out-dB1]{,in-dB2,out-dB2} [gain [initial-volume-dB [delay]]]`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidCompand`] when the command shape or any
    /// parameter is invalid.
    pub fn parse_sox_args(args: &[&str]) -> Result<Self> {
        if !(2..=5).contains(&args.len()) {
            return Err(EffectError::InvalidCompand);
        }

        let attack_decay = parse_attack_decay(args[0])?;
        let transfer = CompandTransfer::parse_sox_transfer(args[1])?;
        let gain_db = args.get(2).map_or(Ok(0.0), |value| parse_f64(value))?;
        let initial_volume_db = args.get(3).map_or(Ok(0.0), |value| parse_f64(value))?;
        let delay_seconds = args.get(4).map_or(Ok(0.0), |value| parse_f64(value))?;

        Self::new(
            attack_decay,
            transfer,
            gain_db,
            initial_volume_db,
            delay_seconds,
        )
    }

    /// Returns the configured attack/decay timing pairs.
    #[must_use]
    pub fn attack_decay(&self) -> &[CompandAttackDecay] {
        &self.attack_decay
    }

    /// Returns the transfer function, including post-gain.
    #[must_use]
    pub const fn transfer(&self) -> &CompandTransfer {
        &self.transfer
    }

    pub(crate) fn is_passthrough(&self) -> bool {
        self.delay_seconds == 0.0 && self.transfer.is_unity_gain()
    }

    /// Applies the compander to an audio buffer in place.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidCompand`] when the configured
    /// attack/decay pair count is neither one nor the input channel count, or
    /// when delay resolution cannot fit the current platform.
    pub fn process_buffer(&self, audio: &mut AudioBuffer) -> Result<()> {
        let channels = audio.channels().as_usize();
        if self.attack_decay.len() != 1 && self.attack_decay.len() != channels {
            return Err(EffectError::InvalidCompand);
        }

        let frames =
            usize::try_from(audio.frames().as_u64()).map_err(|_| EffectError::InvalidCompand)?;
        let delay_samples = self.delay_samples(audio)?;
        let coefficients = self.attack_decay_coefficients(audio.spec().sample_rate().as_u32());
        let initial_volume = db_to_linear(self.initial_volume_db);
        let mut state = CompandState::new(
            self.attack_decay.len(),
            coefficients,
            initial_volume,
            delay_samples,
        );
        let output = process_planar(audio.as_planar_f32(), frames, channels, self, &mut state);

        *audio = AudioBuffer::from_planar_f32(audio.spec(), audio.frames(), output)?;
        Ok(())
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "the comparison only rejects values outside the representable delay range"
    )]
    fn delay_samples(&self, audio: &AudioBuffer) -> Result<usize> {
        let sample_count = self.delay_seconds
            * f64::from(audio.spec().sample_rate().as_u32())
            * f64::from(audio.channels().as_u16());
        if !sample_count.is_finite() || sample_count > usize::MAX as f64 {
            return Err(EffectError::InvalidCompand);
        }

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "delay_seconds was validated finite and non-negative"
        )]
        Ok(sample_count as usize)
    }

    fn attack_decay_coefficients(&self, sample_rate_hz: u32) -> Vec<CompandEnvelopeCoefficients> {
        self.attack_decay
            .iter()
            .copied()
            .map(|pair| CompandEnvelopeCoefficients {
                attack: envelope_coefficient(pair.attack_seconds, sample_rate_hz),
                decay: envelope_coefficient(pair.decay_seconds, sample_rate_hz),
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct CompandState {
    volumes: Vec<f64>,
    coefficients: Vec<CompandEnvelopeCoefficients>,
    delay_buffer: Vec<f32>,
    delay_index: usize,
    delay_count: usize,
    delay_was_full: bool,
}

impl CompandState {
    fn new(
        channel_groups: usize,
        coefficients: Vec<CompandEnvelopeCoefficients>,
        initial_volume: f64,
        delay_samples: usize,
    ) -> Self {
        Self {
            volumes: vec![initial_volume; channel_groups],
            coefficients,
            delay_buffer: vec![0.0; delay_samples],
            delay_index: 0,
            delay_count: 0,
            delay_was_full: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CompandEnvelopeCoefficients {
    attack: f64,
    decay: f64,
}

fn process_planar(
    input: &[f32],
    frames: usize,
    channels: usize,
    compand: &Compand,
    state: &mut CompandState,
) -> Vec<f32> {
    let mut output = PlanarOutput::new(frames, channels);

    for frame in 0..frames {
        update_frame_volumes(input, frame, frames, channels, state);
        for channel in 0..channels {
            let source = input[channel * frames + frame];
            process_interleaved_sample(source, channel, compand, state, &mut output);
        }
    }

    drain_delay(compand, state, &mut output);
    output.into_inner()
}

fn update_frame_volumes(
    input: &[f32],
    frame: usize,
    frames: usize,
    channels: usize,
    state: &mut CompandState,
) {
    if state.volumes.len() == 1 && channels > 1 {
        let level = (0..channels)
            .map(|channel| f64::from(input[channel * frames + frame]).abs())
            .fold(0.0_f64, f64::max);
        update_volume(state, 0, level);
        return;
    }

    for channel in 0..channels {
        let level = f64::from(input[channel * frames + frame]).abs();
        update_volume(state, channel, level);
    }
}

fn update_volume(state: &mut CompandState, channel_group: usize, level: f64) {
    let volume = &mut state.volumes[channel_group];
    let coefficient = state.coefficients[channel_group];
    let delta = level - *volume;
    let smoothing = if delta > 0.0 {
        coefficient.attack
    } else {
        coefficient.decay
    };
    *volume += delta * smoothing;
}

fn process_interleaved_sample(
    source: f32,
    channel: usize,
    compand: &Compand,
    state: &mut CompandState,
    output: &mut PlanarOutput,
) {
    let channel_group = if state.volumes.len() > 1 { channel } else { 0 };
    let gain = compand
        .transfer
        .linear_gain_for_level(state.volumes[channel_group]);
    if state.delay_buffer.is_empty() {
        output.push(apply_compand_gain(source, gain));
        return;
    }

    if state.delay_count >= state.delay_buffer.len() {
        state.delay_was_full = true;
        output.push(apply_compand_gain(
            state.delay_buffer[state.delay_index],
            gain,
        ));
    } else {
        state.delay_count += 1;
    }
    state.delay_buffer[state.delay_index] = source;
    state.delay_index = (state.delay_index + 1) % state.delay_buffer.len();
}

fn drain_delay(compand: &Compand, state: &mut CompandState, output: &mut PlanarOutput) {
    if state.delay_buffer.is_empty() {
        return;
    }
    if !state.delay_was_full {
        state.delay_index = 0;
    }
    state.delay_was_full = true;

    while state.delay_count > 0 {
        let channel_group = if state.volumes.len() > 1 {
            output.interleaved_len() % state.volumes.len()
        } else {
            0
        };
        let gain = compand
            .transfer
            .linear_gain_for_level(state.volumes[channel_group]);
        output.push(apply_compand_gain(
            state.delay_buffer[state.delay_index],
            gain,
        ));
        state.delay_index = (state.delay_index + 1) % state.delay_buffer.len();
        state.delay_count -= 1;
    }
}

fn apply_compand_gain(sample: f32, gain: f64) -> f32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "Auralis' effect buffer format is f32 and output is clipped to normalized full scale"
    )]
    let output = (f64::from(sample) * gain).clamp(-1.0, 1.0) as f32;
    output
}

struct PlanarOutput {
    data: Vec<f32>,
    frames: usize,
    channels: usize,
    frame: usize,
    channel: usize,
    interleaved_len: usize,
}

impl PlanarOutput {
    fn new(frames: usize, channels: usize) -> Self {
        Self {
            data: vec![0.0; frames * channels],
            frames,
            channels,
            frame: 0,
            channel: 0,
            interleaved_len: 0,
        }
    }

    fn push(&mut self, sample: f32) {
        if self.frame < self.frames {
            self.data[self.channel * self.frames + self.frame] = sample;
        }
        self.interleaved_len += 1;
        self.channel += 1;
        if self.channel == self.channels {
            self.channel = 0;
            self.frame += 1;
        }
    }

    const fn interleaved_len(&self) -> usize {
        self.interleaved_len
    }

    fn into_inner(self) -> Vec<f32> {
        self.data
    }
}

fn envelope_coefficient(seconds: f64, sample_rate_hz: u32) -> f64 {
    if seconds > 1.0 / f64::from(sample_rate_hz) {
        1.0 - (-1.0 / (f64::from(sample_rate_hz) * seconds)).exp()
    } else {
        1.0
    }
}

fn db_to_linear(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RawTransferPoint {
    input_db: f64,
    gain_db: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct TransferSegment {
    x: f64,
    y: f64,
    a: f64,
    b: f64,
}

fn parse_attack_decay(value: &str) -> Result<Vec<CompandAttackDecay>> {
    let values = value.split(',').collect::<Vec<_>>();
    if values.is_empty() || values.len() % 2 != 0 || values.iter().any(|part| part.is_empty()) {
        return Err(EffectError::InvalidCompand);
    }

    values
        .chunks_exact(2)
        .map(|chunk| CompandAttackDecay::new(parse_f64(chunk[0])?, parse_f64(chunk[1])?))
        .collect()
}

fn parse_transfer_points(value: &str) -> Result<(Option<f64>, Vec<CompandTransferPoint>)> {
    let (soft_knee_db, points) = if let Some((knee, points)) = value.split_once(':') {
        (Some(parse_f64(knee)?), points)
    } else {
        (None, value)
    };
    let tokens = points.split(',').collect::<Vec<_>>();
    if tokens.is_empty() || tokens.iter().any(|part| part.is_empty()) {
        return Err(EffectError::InvalidCompand);
    }

    let first_has_output = tokens.len() % 2 == 0;
    let mut parsed = Vec::new();
    let mut index = 0;
    if !first_has_output {
        parsed.push(CompandTransferPoint::input_only(parse_transfer_value(
            tokens[0],
        )?)?);
        index = 1;
    }

    while index < tokens.len() {
        if index + 1 >= tokens.len() {
            return Err(EffectError::InvalidCompand);
        }
        parsed.push(CompandTransferPoint::new(
            parse_transfer_value(tokens[index])?,
            parse_transfer_value(tokens[index + 1])?,
        )?);
        index += 2;
    }

    Ok((soft_knee_db, parsed))
}

fn parse_transfer_value(value: &str) -> Result<f64> {
    if value == "-inf" {
        return Ok(SOX_SAMPLE_MIN_DB);
    }
    let parsed = parse_f64(value)?;
    validate_transfer_db(parsed)?;
    Ok(parsed)
}

fn parse_f64(value: &str) -> Result<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|parsed| parsed.is_finite())
        .ok_or(EffectError::InvalidCompand)
}

fn validate_transfer_db(value: f64) -> Result<()> {
    if value.is_finite() && value <= 0.0 {
        Ok(())
    } else {
        Err(EffectError::InvalidCompand)
    }
}

fn validate_points(points: &[CompandTransferPoint]) -> Result<()> {
    if points.is_empty() {
        return Err(EffectError::InvalidCompand);
    }

    for (index, point) in points.iter().enumerate() {
        if index != 0 && point.output_db.is_none() {
            return Err(EffectError::InvalidCompand);
        }
        validate_transfer_db(point.input_db)?;
        if let Some(output_db) = point.output_db {
            validate_transfer_db(output_db)?;
        }
        if index > 0 && points[index - 1].input_db >= point.input_db {
            return Err(EffectError::InvalidCompand);
        }
    }

    Ok(())
}

fn raw_transfer_points(
    points: &[CompandTransferPoint],
    effective_knee_db: f64,
) -> Vec<RawTransferPoint> {
    let mut raw = Vec::with_capacity(points.len() + 2);
    let first = points[0];
    let first_gain_db = first
        .output_db
        .map_or(0.0, |output_db| output_db - first.input_db);
    raw.push(RawTransferPoint {
        input_db: first.input_db - (2.0 * effective_knee_db),
        gain_db: first_gain_db,
    });

    raw.extend(points.iter().map(|point| {
        RawTransferPoint {
            input_db: point.input_db,
            gain_db: point
                .output_db
                .map_or(0.0, |output_db| output_db - point.input_db),
        }
    }));

    if raw.last().is_none_or(|point| point.input_db != 0.0) {
        raw.push(RawTransferPoint {
            input_db: 0.0,
            gain_db: 0.0,
        });
    }

    raw
}

fn join_colinear_points(points: &mut Vec<RawTransferPoint>) {
    let mut index = 2;
    while index < points.len() {
        let previous_slope = (points[index - 1].gain_db - points[index - 2].gain_db)
            * (points[index].input_db - points[index - 1].input_db);
        let next_slope = (points[index].gain_db - points[index - 1].gain_db)
            * (points[index - 1].input_db - points[index - 2].input_db);
        if (previous_slope - next_slope).abs() == 0.0 {
            points.remove(index - 1);
            index = index.saturating_sub(1).max(2);
        } else {
            index += 1;
        }
    }
}

fn prepare_segments(
    raw_points: &[RawTransferPoint],
    gain_db: f64,
    effective_knee_db: f64,
) -> (Vec<TransferSegment>, f64, f64) {
    let mut segments = vec![TransferSegment::default(); raw_points.len() * 2];
    for (index, point) in raw_points.iter().enumerate() {
        segments[2 * index] = TransferSegment {
            x: point.input_db * DB_TO_NATURAL_LOG,
            y: (point.gain_db + gain_db) * DB_TO_NATURAL_LOG,
            a: 0.0,
            b: 0.0,
        };
    }

    let radius = effective_knee_db * DB_TO_NATURAL_LOG;
    for raw_index in 1..raw_points.len() {
        let line2_index = 2 * raw_index;
        if segments[line2_index].x == 0.0 {
            break;
        }

        let line1_index = line2_index - 2;
        let curve_index = line2_index - 1;
        let line3_index = line2_index + 2;

        segments[line1_index].a = 0.0;
        segments[line1_index].b = (segments[line2_index].y - segments[line1_index].y)
            / (segments[line2_index].x - segments[line1_index].x);

        segments[line2_index].a = 0.0;
        segments[line2_index].b = (segments[line3_index].y - segments[line2_index].y)
            / (segments[line3_index].x - segments[line2_index].x);

        let theta = (segments[line2_index].y - segments[line1_index].y)
            .atan2(segments[line2_index].x - segments[line1_index].x);
        let len = ((segments[line2_index].x - segments[line1_index].x).powi(2)
            + (segments[line2_index].y - segments[line1_index].y).powi(2))
        .sqrt();
        let r = radius.min(len);
        segments[curve_index].x = segments[line2_index].x - (r * theta.cos());
        segments[curve_index].y = segments[line2_index].y - (r * theta.sin());

        let theta = (segments[line3_index].y - segments[line2_index].y)
            .atan2(segments[line3_index].x - segments[line2_index].x);
        let len = ((segments[line3_index].x - segments[line2_index].x).powi(2)
            + (segments[line3_index].y - segments[line2_index].y).powi(2))
        .sqrt();
        let r = radius.min(len / 2.0);
        let x = segments[line2_index].x + (r * theta.cos());
        let y = segments[line2_index].y + (r * theta.sin());

        let center_x = (segments[curve_index].x + segments[line2_index].x + x) / 3.0;
        let center_y = (segments[curve_index].y + segments[line2_index].y + y) / 3.0;

        segments[line2_index].x = x;
        segments[line2_index].y = y;

        let in1 = center_x - segments[curve_index].x;
        let out1 = center_y - segments[curve_index].y;
        let in2 = segments[line2_index].x - segments[curve_index].x;
        let out2 = segments[line2_index].y - segments[curve_index].y;
        segments[curve_index].a = ((out2 / in2) - (out1 / in1)) / (in2 - in1);
        segments[curve_index].b = (out1 / in1) - (segments[curve_index].a * in1);
    }

    let final_index = raw_points
        .iter()
        .position(|point| point.input_db == 0.0)
        .unwrap_or(raw_points.len() - 1);
    let final_curve_index = (2 * final_index).saturating_sub(1);
    segments[final_curve_index].x = 0.0;
    segments[final_curve_index].y = segments[2 * final_index].y;

    (segments.clone(), segments[1].x.exp(), segments[1].y.exp())
}

fn is_non_negative_finite(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn is_exact_one(value: f64) -> bool {
    value.to_bits() == 1.0_f64.to_bits()
}

fn is_exact_zero(value: f64) -> bool {
    value.to_bits().trailing_zeros() >= 63
}

fn linear_to_db(value: f64) -> f64 {
    20.0 * value.log10()
}

#[cfg(test)]
mod tests {
    use super::{Compand, CompandAttackDecay, CompandTransfer, CompandTransferPoint};
    use crate::EffectError;

    #[test]
    fn parses_sox_style_compand_arguments() {
        let compand = Compand::parse_sox_args(&[
            "0.3,1,0.05,0.2",
            "6:-70,-60,-20,-30,0,-5",
            "-3",
            "-90",
            "0.2",
        ])
        .unwrap();

        assert_eq!(
            compand.attack_decay(),
            [
                CompandAttackDecay::new(0.3, 1.0).unwrap(),
                CompandAttackDecay::new(0.05, 0.2).unwrap(),
            ]
        );
        assert_eq!(compand.transfer().soft_knee_db(), Some(6.0));
        assert_close(compand.gain_db, -3.0, f64::EPSILON);
        assert_close(compand.initial_volume_db, -90.0, f64::EPSILON);
        assert_close(compand.delay_seconds, 0.2, f64::EPSILON);
    }

    #[test]
    fn transfer_curve_interpolates_in_decibel_gain_space() {
        let transfer = CompandTransfer::new(
            [
                CompandTransferPoint::new(-60.0, -60.0).unwrap(),
                CompandTransferPoint::new(-20.0, -30.0).unwrap(),
                CompandTransferPoint::new(0.0, -25.0).unwrap(),
            ],
            None,
        )
        .unwrap();

        assert_close(transfer.output_db_for_input_db(-40.0), -45.0, 0.001);
    }

    #[test]
    fn transfer_supports_first_input_only_point_and_minus_inf() {
        let transfer = CompandTransfer::parse_sox_transfer("-inf,-90,-20,-20,0,0").unwrap();

        assert!(transfer.points()[0].input_db < -180.0);
        assert_close(transfer.output_db_for_input_db(-20.0), -20.0, 0.002);
    }

    #[test]
    fn post_gain_is_applied_to_transfer_output() {
        let compand = Compand::parse_sox_args(&["0,0", "-60,-60,0,0", "-6"]).unwrap();

        assert_close(
            compand.transfer().output_db_for_input_db(-20.0),
            -26.0,
            0.001,
        );
    }

    #[test]
    fn detects_zero_delay_unity_gain_passthrough() {
        let compand = Compand::parse_sox_args(&["0,0", "-60,-60,0,0"]).unwrap();
        let gained = Compand::parse_sox_args(&["0,0", "-60,-60,0,0", "-6"]).unwrap();
        let delayed = Compand::parse_sox_args(&["0,0", "-60,-60,0,0", "0", "0", "0.1"]).unwrap();

        assert!(compand.is_passthrough());
        assert!(!gained.is_passthrough());
        assert!(!delayed.is_passthrough());
    }

    #[test]
    fn rejects_invalid_command_shapes_and_values() {
        assert_eq!(
            Compand::parse_sox_args(&["0,1"]).unwrap_err(),
            EffectError::InvalidCompand
        );
        assert_eq!(
            Compand::parse_sox_args(&["0,1,2", "-60,-60"]).unwrap_err(),
            EffectError::InvalidCompand
        );
        assert_eq!(
            Compand::parse_sox_args(&["0,1", "-20,-20,-30,-30"]).unwrap_err(),
            EffectError::InvalidCompand
        );
        assert_eq!(
            Compand::parse_sox_args(&["0,1", "-60,-60", "0", "1"]).unwrap_err(),
            EffectError::InvalidCompand
        );
        assert_eq!(
            Compand::parse_sox_args(&["0,1", "-60,-60", "0", "0", "-1"]).unwrap_err(),
            EffectError::InvalidCompand
        );
    }

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {actual} to be within {tolerance} of {expected}",
        );
    }
}
