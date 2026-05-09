use crate::{
    AudioBuffer, AudioSpec, BackendKind, ChannelCount, FrameCount, SampleFormat, SampleRate,
};
use thiserror::Error;

/// Input-combiner method selected before any effects are applied.
///
/// Implemented methods are SoX-ng-style combiners. Later combine modes remain
/// absent from this enum until their own DEVELOPMENT.md leaf features add tests
/// and documented semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CombineMethod {
    /// Append every input in order, preserving channels and sample values.
    Concatenate,

    /// Play inputs in order, preserving explicit sequence-boundary semantics.
    Sequence,

    /// Mix corresponding channels after SoX-ng-style automatic input balancing.
    Mix,

    /// Mix corresponding channels with SoX-ng-style equal-power input balancing.
    MixPower,

    /// Merge all channels from all inputs into one multichannel output.
    Merge,

    /// Multiply corresponding channels and samples from all inputs.
    Multiply,
}

impl CombineMethod {
    /// Returns the canonical command-line name for this method.
    #[must_use]
    pub const fn as_name(self) -> &'static str {
        match self {
            Self::Concatenate => "concatenate",
            Self::Sequence => "sequence",
            Self::Mix => "mix",
            Self::MixPower => "mix-power",
            Self::Merge => "merge",
            Self::Multiply => "multiply",
        }
    }

    /// Resolves a supported combine-method name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "concatenate" => Some(Self::Concatenate),
            "sequence" => Some(Self::Sequence),
            "mix" => Some(Self::Mix),
            "mix-power" => Some(Self::MixPower),
            "merge" => Some(Self::Merge),
            "multiply" => Some(Self::Multiply),
            _ => None,
        }
    }
}

/// Errors produced while combining multiple decoded inputs.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum InputCombineError {
    /// The caller supplied no input buffers.
    #[error("input combiner requires at least one input")]
    EmptyInputList,

    /// One input's sample rate did not match the first input.
    #[error(
        "input {input_index} sample rate {actual} does not match first input sample rate {expected}"
    )]
    MismatchedSampleRate {
        /// Zero-based input index that failed validation.
        input_index: usize,

        /// Sample rate from the first input.
        expected: SampleRate,

        /// Sample rate from the mismatched input.
        actual: SampleRate,
    },

    /// One input's channel count did not match the first input.
    #[error(
        "input {input_index} channel count {actual} does not match first input channel count {expected}"
    )]
    MismatchedChannelCount {
        /// Zero-based input index that failed validation.
        input_index: usize,

        /// Channel count from the first input.
        expected: ChannelCount,

        /// Channel count from the mismatched input.
        actual: ChannelCount,
    },

    /// One input's internal sample format did not match the first input.
    #[error(
        "input {input_index} sample format {actual} does not match first input sample format {expected}"
    )]
    MismatchedSampleFormat {
        /// Zero-based input index that failed validation.
        input_index: usize,

        /// Sample format from the first input.
        expected: SampleFormat,

        /// Sample format from the mismatched input.
        actual: SampleFormat,
    },

    /// A sequence boundary changed sample rate, which one output buffer cannot represent.
    #[error(
        "sequence boundary before input {input_index} cannot be represented in one output: sample rate {actual} does not match previous input {previous_index} sample rate {expected}"
    )]
    SequenceBoundarySampleRate {
        /// Zero-based input index after the failing boundary.
        input_index: usize,

        /// Zero-based input index before the failing boundary.
        previous_index: usize,

        /// Sample rate before the boundary.
        expected: SampleRate,

        /// Sample rate after the boundary.
        actual: SampleRate,
    },

    /// A sequence boundary changed channel count, which one output buffer cannot represent.
    #[error(
        "sequence boundary before input {input_index} cannot be represented in one output: channel count {actual} does not match previous input {previous_index} channel count {expected}"
    )]
    SequenceBoundaryChannelCount {
        /// Zero-based input index after the failing boundary.
        input_index: usize,

        /// Zero-based input index before the failing boundary.
        previous_index: usize,

        /// Channel count before the boundary.
        expected: ChannelCount,

        /// Channel count after the boundary.
        actual: ChannelCount,
    },

    /// A sequence boundary changed sample format, which one output buffer cannot represent.
    #[error(
        "sequence boundary before input {input_index} cannot be represented in one output: sample format {actual} does not match previous input {previous_index} sample format {expected}"
    )]
    SequenceBoundarySampleFormat {
        /// Zero-based input index after the failing boundary.
        input_index: usize,

        /// Zero-based input index before the failing boundary.
        previous_index: usize,

        /// Sample format before the boundary.
        expected: SampleFormat,

        /// Sample format after the boundary.
        actual: SampleFormat,
    },

    /// The combined frame count cannot be represented by Auralis.
    #[error("combined frame count cannot be represented")]
    FrameCountOverflow,

    /// The combined channel count cannot be represented by Auralis.
    #[error("combined channel count cannot be represented")]
    ChannelCountOverflow,

    /// The combined planar buffer shape was rejected by the core buffer model.
    #[error(transparent)]
    Core(#[from] auralis_core::AuralisError),

    /// A backend-dispatched mixing kernel rejected the channel slices.
    #[error(transparent)]
    Mix(#[from] auralis_simd::MixError),

    /// A backend-dispatched multiply kernel rejected the channel slices.
    #[error(transparent)]
    Multiply(#[from] auralis_simd::MultiplyError),
}

/// Concatenates already-decoded planar audio buffers.
///
/// Inputs are appended in caller order. All inputs must have the same sample
/// rate, channel count, and internal sample format as the first input. Frame
/// lengths may differ; the output frame count is the sum of all input frame
/// counts. Samples stay channel-major, so each output channel contains that
/// channel from input 0, followed by the same channel from input 1, and so on.
///
/// # Errors
///
/// Returns [`InputCombineError::EmptyInputList`] for no inputs, a mismatch
/// variant when an input's stream shape is incompatible with the first input,
/// or an overflow/shape error if the combined buffer cannot be represented.
pub fn concatenate_audio_buffers(
    inputs: &[AudioBuffer],
) -> std::result::Result<AudioBuffer, InputCombineError> {
    let Some(first) = inputs.first() else {
        return Err(InputCombineError::EmptyInputList);
    };

    let spec = first.spec();
    for (input_index, input) in inputs.iter().enumerate() {
        validate_concatenate_input(input_index, spec, input)?;
    }

    append_serial_audio_buffers(inputs)
}

/// Sequences already-decoded planar audio buffers for a single output buffer.
///
/// SoX-ng `sequence` can close and reopen some output devices at input
/// boundaries when stream parameters change. Auralis currently writes one
/// in-memory buffer and one PCM16 WAV output file, so only boundaries that keep
/// the same sample rate, channel count, and sample format can be represented.
/// For representable boundaries, samples are appended in caller order just like
/// serial playback: each output channel contains that channel from input 0,
/// then input 1, and so on. Frame lengths may differ.
///
/// # Errors
///
/// Returns [`InputCombineError::EmptyInputList`] for no inputs, a sequence
/// boundary mismatch when one output buffer cannot represent the transition, or
/// an overflow/shape error if the combined buffer cannot be represented.
pub fn sequence_audio_buffers(
    inputs: &[AudioBuffer],
) -> std::result::Result<AudioBuffer, InputCombineError> {
    let Some(first) = inputs.first() else {
        return Err(InputCombineError::EmptyInputList);
    };

    let mut previous = first.spec();
    for (input_index, input) in inputs.iter().enumerate().skip(1) {
        validate_sequence_boundary(input_index - 1, input_index, previous, input.spec())?;
        previous = input.spec();
    }

    append_serial_audio_buffers(inputs)
}

/// Mixes already-decoded planar audio buffers using SoX-ng `mix` semantics.
///
/// Auralis applies SoX-ng's default automatic input balancing: every input is
/// scaled by `1 / input_count` before corresponding channels are summed. The
/// output frame count is the longest input and the output channel count is the
/// largest input channel count. Missing tail frames and missing channels are
/// treated as silence, so a short input or mono input contributes nothing where
/// it has no sample. Inputs must share sample rate and internal sample format.
///
/// Mixing itself does not clip or normalize beyond the `1 / input_count`
/// balancing factor. If mixed samples are outside `[-1.0, 1.0]`, later boundary
/// writers such as PCM16 WAV encoding apply their documented clipping.
///
/// # Errors
///
/// Returns [`InputCombineError::EmptyInputList`] for no inputs, a mismatch
/// variant when an input's sample rate or sample format is incompatible with
/// the first input, or an overflow/shape/kernel error if the output buffer
/// cannot be represented.
pub fn mix_audio_buffers(
    inputs: &[AudioBuffer],
) -> std::result::Result<AudioBuffer, InputCombineError> {
    mix_audio_buffers_with_backend(inputs, BackendKind::Scalar)
}

/// Mixes already-decoded planar audio buffers with a requested backend.
///
/// `requested_backend` selects the scalar or SIMD mixing kernel through the
/// same deterministic backend fallback rules used by backend-aware effects.
/// Numerical behavior matches [`mix_audio_buffers`].
///
/// # Errors
///
/// Returns the same errors as [`mix_audio_buffers`].
pub fn mix_audio_buffers_with_backend(
    inputs: &[AudioBuffer],
    requested_backend: BackendKind,
) -> std::result::Result<AudioBuffer, InputCombineError> {
    parallel_mix_audio_buffers_with_backend(inputs, requested_backend, mix_balance_scale)
}

/// Mixes already-decoded planar audio buffers using SoX-ng `mix-power` semantics.
///
/// Auralis applies SoX-ng's default equal-power balancing: every input is
/// scaled by `1 / sqrt(input_count)` before corresponding channels are summed.
/// The output frame count is the longest input and the output channel count is
/// the largest input channel count. Missing tail frames and missing channels
/// are treated as silence. Inputs must share sample rate and internal sample
/// format.
///
/// Mixing itself does not clip or normalize beyond the equal-power balancing
/// factor. If mixed samples are outside `[-1.0, 1.0]`, later boundary writers
/// such as PCM16 WAV encoding apply their documented clipping.
///
/// # Errors
///
/// Returns [`InputCombineError::EmptyInputList`] for no inputs, a mismatch
/// variant when an input's sample rate or sample format is incompatible with
/// the first input, or an overflow/shape/kernel error if the output buffer
/// cannot be represented.
pub fn mix_power_audio_buffers(
    inputs: &[AudioBuffer],
) -> std::result::Result<AudioBuffer, InputCombineError> {
    mix_power_audio_buffers_with_backend(inputs, BackendKind::Scalar)
}

/// Mixes already-decoded planar audio buffers with equal-power balancing and a requested backend.
///
/// `requested_backend` selects the scalar or SIMD mixing kernel through the
/// same deterministic backend fallback rules used by backend-aware effects.
/// Numerical behavior matches [`mix_power_audio_buffers`].
///
/// # Errors
///
/// Returns the same errors as [`mix_power_audio_buffers`].
pub fn mix_power_audio_buffers_with_backend(
    inputs: &[AudioBuffer],
    requested_backend: BackendKind,
) -> std::result::Result<AudioBuffer, InputCombineError> {
    parallel_mix_audio_buffers_with_backend(inputs, requested_backend, mix_power_balance_scale)
}

/// Merges already-decoded planar audio buffers using SoX-ng `merge` semantics.
///
/// The output frame count is the longest input and the output channel count is
/// the sum of all input channel counts. Output channels are ordered by input:
/// every channel from input 0, then every channel from input 1, and so on.
/// Missing tail frames from shorter inputs are silence. Inputs must share
/// sample rate and internal sample format; channel counts may differ.
///
/// Merge is a deterministic structural copy and does not select a SIMD kernel.
///
/// # Errors
///
/// Returns [`InputCombineError::EmptyInputList`] for no inputs, a mismatch
/// variant when an input's sample rate or sample format is incompatible with
/// the first input, or an overflow/shape error if the merged buffer cannot be
/// represented.
pub fn merge_audio_buffers(
    inputs: &[AudioBuffer],
) -> std::result::Result<AudioBuffer, InputCombineError> {
    let Some(first) = inputs.first() else {
        return Err(InputCombineError::EmptyInputList);
    };

    let spec = first.spec();
    let mut max_frames = first.frames();
    let mut total_channels = 0_usize;
    for (input_index, input) in inputs.iter().enumerate() {
        validate_merge_input(input_index, spec, input)?;
        if input.frames() > max_frames {
            max_frames = input.frames();
        }
        total_channels = total_channels
            .checked_add(input.channels().as_usize())
            .ok_or(InputCombineError::ChannelCountOverflow)?;
    }

    let output_channels = u16::try_from(total_channels)
        .map_err(|_| InputCombineError::ChannelCountOverflow)
        .and_then(|channels| ChannelCount::new(channels).map_err(InputCombineError::from))?;
    let output_spec = AudioSpec::new(spec.sample_rate(), output_channels, spec.sample_format());
    let mut output = AudioBuffer::zeroed(output_spec, max_frames)?;

    let mut output_channel_index = 0_usize;
    for input in inputs {
        for input_channel_index in 0..input.channels().as_usize() {
            let source = input
                .channel(input_channel_index)
                .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
            let output_channel = output
                .channel_mut(output_channel_index)
                .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
            output_channel[..source.len()].copy_from_slice(source);
            output_channel_index += 1;
        }
    }

    Ok(output)
}

/// Multiplies already-decoded planar audio buffers using SoX-ng `multiply` semantics.
///
/// The output frame count is the longest input and the output channel count is
/// the largest input channel count. Each output sample is the product of all
/// corresponding input samples for that channel and frame. Missing tail frames
/// and missing channels are treated as silence, so any missing contribution
/// makes that output sample `0.0`. A single input is an identity copy. Inputs
/// must share sample rate and internal sample format; channel counts may
/// differ.
///
/// Multiplication itself does not clip or normalize. If multiplied samples are
/// outside `[-1.0, 1.0]`, later boundary writers such as PCM16 WAV encoding
/// apply their documented clipping.
///
/// # Errors
///
/// Returns [`InputCombineError::EmptyInputList`] for no inputs, a mismatch
/// variant when an input's sample rate or sample format is incompatible with
/// the first input, or an overflow/shape/kernel error if the output buffer
/// cannot be represented.
pub fn multiply_audio_buffers(
    inputs: &[AudioBuffer],
) -> std::result::Result<AudioBuffer, InputCombineError> {
    multiply_audio_buffers_with_backend(inputs, BackendKind::Scalar)
}

/// Multiplies already-decoded planar audio buffers with a requested backend.
///
/// `requested_backend` selects the scalar or SIMD multiply kernel through the
/// same deterministic backend fallback rules used by backend-aware effects.
/// Numerical behavior matches [`multiply_audio_buffers`].
///
/// # Errors
///
/// Returns the same errors as [`multiply_audio_buffers`].
pub fn multiply_audio_buffers_with_backend(
    inputs: &[AudioBuffer],
    requested_backend: BackendKind,
) -> std::result::Result<AudioBuffer, InputCombineError> {
    let Some(first) = inputs.first() else {
        return Err(InputCombineError::EmptyInputList);
    };

    let spec = first.spec();
    let mut max_frames = first.frames();
    let mut max_channels = spec.channels();
    for (input_index, input) in inputs.iter().enumerate() {
        validate_mix_input(input_index, spec, input)?;
        if input.frames() > max_frames {
            max_frames = input.frames();
        }
        if input.channels() > max_channels {
            max_channels = input.channels();
        }
    }

    let output_spec = AudioSpec::new(spec.sample_rate(), max_channels, spec.sample_format());
    let mut output = AudioBuffer::zeroed(output_spec, max_frames)?;
    let selection = auralis_simd::select_backend(requested_backend);

    for channel_index in 0..max_channels.as_usize() {
        let mut channel_inputs = Vec::with_capacity(inputs.len());
        for input in inputs {
            if channel_index < input.channels().as_usize() {
                channel_inputs.push(
                    input
                        .channel(channel_index)
                        .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?,
                );
            } else {
                channel_inputs.push(&[]);
            }
        }

        let output_channel = output
            .channel_mut(channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        auralis_simd::multiply_f32_with_backend(selection, &channel_inputs, output_channel)?;
    }

    Ok(output)
}

fn parallel_mix_audio_buffers_with_backend(
    inputs: &[AudioBuffer],
    requested_backend: BackendKind,
    scale_for_input_count: fn(usize) -> f32,
) -> std::result::Result<AudioBuffer, InputCombineError> {
    let Some(first) = inputs.first() else {
        return Err(InputCombineError::EmptyInputList);
    };

    let spec = first.spec();
    let mut max_frames = first.frames();
    let mut max_channels = spec.channels();
    for (input_index, input) in inputs.iter().enumerate() {
        validate_mix_input(input_index, spec, input)?;
        if input.frames() > max_frames {
            max_frames = input.frames();
        }
        if input.channels() > max_channels {
            max_channels = input.channels();
        }
    }

    let output_spec = AudioSpec::new(spec.sample_rate(), max_channels, spec.sample_format());
    let mut output = AudioBuffer::zeroed(output_spec, max_frames)?;
    let selection = auralis_simd::select_backend(requested_backend);
    let scale = scale_for_input_count(inputs.len());

    for channel_index in 0..max_channels.as_usize() {
        let mut channel_inputs = Vec::with_capacity(inputs.len());
        for input in inputs {
            if channel_index < input.channels().as_usize() {
                channel_inputs.push(
                    input
                        .channel(channel_index)
                        .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?,
                );
            }
        }

        let output_channel = output
            .channel_mut(channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        auralis_simd::mix_f32_with_backend(selection, &channel_inputs, output_channel, scale)?;
    }

    Ok(output)
}

fn append_serial_audio_buffers(
    inputs: &[AudioBuffer],
) -> std::result::Result<AudioBuffer, InputCombineError> {
    let Some(first) = inputs.first() else {
        return Err(InputCombineError::EmptyInputList);
    };

    let spec = first.spec();
    let mut total_frames = 0_u64;
    for input in inputs {
        total_frames = total_frames
            .checked_add(input.frames().as_u64())
            .ok_or(InputCombineError::FrameCountOverflow)?;
    }

    let total_frames = FrameCount::new(total_frames);
    let total_frame_capacity = usize::try_from(total_frames.as_u64())
        .map_err(|_| InputCombineError::FrameCountOverflow)?;
    let sample_capacity = spec
        .channels()
        .as_usize()
        .checked_mul(total_frame_capacity)
        .ok_or(InputCombineError::FrameCountOverflow)?;
    let mut data = Vec::with_capacity(sample_capacity);

    for channel_index in 0..spec.channels().as_usize() {
        for input in inputs {
            data.extend_from_slice(
                input
                    .channel(channel_index)
                    .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?,
            );
        }
    }

    AudioBuffer::from_planar_f32(spec, total_frames, data).map_err(InputCombineError::from)
}

fn validate_concatenate_input(
    input_index: usize,
    expected: AudioSpec,
    input: &AudioBuffer,
) -> std::result::Result<(), InputCombineError> {
    let actual = input.spec();

    if actual.sample_rate() != expected.sample_rate() {
        return Err(InputCombineError::MismatchedSampleRate {
            input_index,
            expected: expected.sample_rate(),
            actual: actual.sample_rate(),
        });
    }
    if actual.channels() != expected.channels() {
        return Err(InputCombineError::MismatchedChannelCount {
            input_index,
            expected: expected.channels(),
            actual: actual.channels(),
        });
    }
    if actual.sample_format() != expected.sample_format() {
        return Err(InputCombineError::MismatchedSampleFormat {
            input_index,
            expected: expected.sample_format(),
            actual: actual.sample_format(),
        });
    }

    Ok(())
}

fn validate_mix_input(
    input_index: usize,
    expected: AudioSpec,
    input: &AudioBuffer,
) -> std::result::Result<(), InputCombineError> {
    let actual = input.spec();

    if actual.sample_rate() != expected.sample_rate() {
        return Err(InputCombineError::MismatchedSampleRate {
            input_index,
            expected: expected.sample_rate(),
            actual: actual.sample_rate(),
        });
    }
    if actual.sample_format() != expected.sample_format() {
        return Err(InputCombineError::MismatchedSampleFormat {
            input_index,
            expected: expected.sample_format(),
            actual: actual.sample_format(),
        });
    }

    Ok(())
}

fn validate_merge_input(
    input_index: usize,
    expected: AudioSpec,
    input: &AudioBuffer,
) -> std::result::Result<(), InputCombineError> {
    let actual = input.spec();

    if actual.sample_rate() != expected.sample_rate() {
        return Err(InputCombineError::MismatchedSampleRate {
            input_index,
            expected: expected.sample_rate(),
            actual: actual.sample_rate(),
        });
    }
    if actual.sample_format() != expected.sample_format() {
        return Err(InputCombineError::MismatchedSampleFormat {
            input_index,
            expected: expected.sample_format(),
            actual: actual.sample_format(),
        });
    }

    Ok(())
}

fn mix_balance_scale(input_count: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "Mix balancing is an f32 sample operation; huge input counts cannot be represented as decoded in-memory buffers in practice."
    )]
    {
        1.0 / input_count as f32
    }
}

fn mix_power_balance_scale(input_count: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "Mix-power balancing is an f32 sample operation; huge input counts cannot be represented as decoded in-memory buffers in practice."
    )]
    {
        1.0 / (input_count as f32).sqrt()
    }
}

fn validate_sequence_boundary(
    previous_index: usize,
    input_index: usize,
    expected: AudioSpec,
    actual: AudioSpec,
) -> std::result::Result<(), InputCombineError> {
    if actual.sample_rate() != expected.sample_rate() {
        return Err(InputCombineError::SequenceBoundarySampleRate {
            input_index,
            previous_index,
            expected: expected.sample_rate(),
            actual: actual.sample_rate(),
        });
    }
    if actual.channels() != expected.channels() {
        return Err(InputCombineError::SequenceBoundaryChannelCount {
            input_index,
            previous_index,
            expected: expected.channels(),
            actual: actual.channels(),
        });
    }
    if actual.sample_format() != expected.sample_format() {
        return Err(InputCombineError::SequenceBoundarySampleFormat {
            input_index,
            previous_index,
            expected: expected.sample_format(),
            actual: actual.sample_format(),
        });
    }

    Ok(())
}
