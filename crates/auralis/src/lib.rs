//! High-level Auralis library API.
//!
//! This crate provides a small facade over the lower-level codec and effect
//! crates. It is intended for applications that want to open supported audio
//! files, build a typed processing chain, and write the result without wiring
//! the codec and effect crates manually.
//!
//! # Examples
//!
//! ```no_run
//! use auralis::AudioFile;
//!
//! AudioFile::open_wav("input.wav")?
//!     .into_pipeline()
//!     .gain_db(-3.0)
//!     .dc_shift(0.125)
//!     .fade_frames(1_000, 2_000)
//!     .reverse()
//!     .pad_frames(0, 48_000)
//!     .write_wav("output.wav")?;
//!
//! # Ok::<(), auralis::Error>(())
//! ```

use std::path::Path;

pub use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
    TimeSeconds,
};
pub use auralis_effects::{
    EffectChain, EffectChainBoundary, EffectChainError, EffectChainParseError, EffectCommand,
    EffectsFileParseError, EffectsFileReadError, parse_effect_chain, parse_effects_file,
    parse_effects_file_str,
};
pub use auralis_simd::BackendKind;

use auralis_effects::{DcShift, Fade, Gain, Pad, Reverse, Trim};
use thiserror::Error;

/// Crate-local result type using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by the high-level Auralis facade.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A shared core value constructor rejected an input.
    #[error(transparent)]
    Core(#[from] auralis_core::AuralisError),

    /// WAV decoding or encoding failed.
    #[error(transparent)]
    Wav(#[from] auralis_wav::WavError),

    /// A typed effect processor rejected its configuration or input buffer.
    #[error(transparent)]
    Effect(#[from] auralis_effects::EffectError),

    /// A sequential effect chain failed while applying one command.
    #[error(transparent)]
    Chain(#[from] auralis_effects::EffectChainError),

    /// Input combination failed before effects were applied.
    #[error(transparent)]
    InputCombine(#[from] InputCombineError),

    /// A seconds-based trim range could not be represented as frames.
    #[error("trim seconds range cannot be represented as frame positions")]
    InvalidTrimSecondsRange,
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Core(left), Self::Core(right)) => left == right,
            (Self::Wav(left), Self::Wav(right)) => left == right,
            (Self::Effect(left), Self::Effect(right)) => left == right,
            (Self::Chain(left), Self::Chain(right)) => left == right,
            (Self::InputCombine(left), Self::InputCombine(right)) => left == right,
            (Self::InvalidTrimSecondsRange, Self::InvalidTrimSecondsRange) => true,
            _ => false,
        }
    }
}

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

    /// The combined planar buffer shape was rejected by the core buffer model.
    #[error(transparent)]
    Core(#[from] auralis_core::AuralisError),

    /// A backend-dispatched mixing kernel rejected the channel slices.
    #[error(transparent)]
    Mix(#[from] auralis_simd::MixError),
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

/// Decoded audio file ready to enter an effect pipeline.
///
/// `AudioFile` currently supports PCM16 WAV input only. Decoding always uses
/// Auralis' internal planar `f32` [`AudioBuffer`] representation.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioFile {
    audio: AudioBuffer,
    requested_backend: BackendKind,
}

impl AudioFile {
    /// Opens a PCM16 WAV file and decodes it into planar `f32` samples.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] when the path cannot be opened, the input is not
    /// a well-formed WAV stream, or the sample format is not supported.
    pub fn open_wav(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_wav_with_backend(path, BackendKind::Scalar)
    }

    /// Opens a PCM16 WAV file using the requested sample-conversion backend.
    ///
    /// The decoded audio is identical to [`Self::open_wav`]. `requested_backend`
    /// controls only backend-aware decode, later backend-aware effect kernels,
    /// and WAV encoding after [`Self::into_pipeline`]. Unsupported SIMD requests
    /// follow Auralis' documented scalar fallback.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] when the path cannot be opened, the input is not
    /// a well-formed WAV stream, or the sample format is not supported.
    pub fn open_wav_with_backend(
        path: impl AsRef<Path>,
        requested_backend: BackendKind,
    ) -> Result<Self> {
        Ok(Self {
            audio: auralis_wav::decode_pcm16_path_with_backend(path, requested_backend)?,
            requested_backend,
        })
    }

    /// Opens multiple PCM16 WAV files and concatenates them in caller order.
    ///
    /// This is the library counterpart to `auralis run --combine concatenate`.
    /// Each input is decoded into planar `f32`, then the buffers are
    /// concatenated before any later pipeline effects are applied. All inputs
    /// must have the same sample rate and channel count; frame lengths may
    /// differ.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] for decode failures or
    /// [`Error::InputCombine`] when the input list is empty or stream metadata
    /// is incompatible.
    pub fn open_wavs_concatenated<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        Self::open_wavs_concatenated_with_backend(paths, BackendKind::Scalar)
    }

    /// Opens multiple PCM16 WAV files and concatenates them with a requested backend.
    ///
    /// `requested_backend` controls decode conversion, later backend-aware
    /// effects, and output encoding after [`Self::into_pipeline`]. The
    /// concatenate combiner itself is a structural copy and does not select a
    /// SIMD kernel.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] for decode failures or
    /// [`Error::InputCombine`] when the input list is empty or stream metadata
    /// is incompatible.
    pub fn open_wavs_concatenated_with_backend<I, P>(
        paths: I,
        requested_backend: BackendKind,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut inputs = Vec::new();
        for path in paths {
            inputs.push(auralis_wav::decode_pcm16_path_with_backend(
                path,
                requested_backend,
            )?);
        }

        Self::from_audio_buffers_concatenated_with_backend(&inputs, requested_backend)
    }

    /// Opens multiple PCM16 WAV files and sequences them in caller order.
    ///
    /// This is the library counterpart to `auralis run --combine sequence`.
    /// Auralis writes one output buffer/file, so sequence boundaries must keep
    /// the same sample rate and channel count. Representable boundaries append
    /// decoded input samples in serial playback order before any later effects
    /// are applied. Frame lengths may differ.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] for decode failures or
    /// [`Error::InputCombine`] when the input list is empty or a sequence
    /// boundary cannot be represented by one output buffer.
    pub fn open_wavs_sequenced<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        Self::open_wavs_sequenced_with_backend(paths, BackendKind::Scalar)
    }

    /// Opens multiple PCM16 WAV files and sequences them with a requested backend.
    ///
    /// `requested_backend` controls decode conversion, later backend-aware
    /// effects, and output encoding after [`Self::into_pipeline`]. Sequencing
    /// itself is a structural copy after boundary validation and does not
    /// select a SIMD kernel.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] for decode failures or
    /// [`Error::InputCombine`] when the input list is empty or a sequence
    /// boundary cannot be represented by one output buffer.
    pub fn open_wavs_sequenced_with_backend<I, P>(
        paths: I,
        requested_backend: BackendKind,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut inputs = Vec::new();
        for path in paths {
            inputs.push(auralis_wav::decode_pcm16_path_with_backend(
                path,
                requested_backend,
            )?);
        }

        Self::from_audio_buffers_sequenced_with_backend(&inputs, requested_backend)
    }

    /// Opens multiple PCM16 WAV files and mixes them into one buffer.
    ///
    /// This is the library counterpart to `auralis run --combine mix`. Each
    /// input is decoded into planar `f32`, scaled by `1 / input_count`, and
    /// summed with corresponding channels before any later effects are applied.
    /// The output length is the longest input; missing tail frames and missing
    /// channels are silence.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] for decode failures or
    /// [`Error::InputCombine`] when the input list is empty, sample rates or
    /// sample formats are incompatible, or the mixed buffer cannot be
    /// represented.
    pub fn open_wavs_mixed<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        Self::open_wavs_mixed_with_backend(paths, BackendKind::Scalar)
    }

    /// Opens multiple PCM16 WAV files and mixes them with a requested backend.
    ///
    /// `requested_backend` controls decode conversion, the scalar/SIMD mix
    /// kernel, later backend-aware effects, and output encoding after
    /// [`Self::into_pipeline`]. SIMD requests follow Auralis' deterministic
    /// scalar fallback rules when SIMD is unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] for decode failures or
    /// [`Error::InputCombine`] when the input list is empty, sample rates or
    /// sample formats are incompatible, or the mixed buffer cannot be
    /// represented.
    pub fn open_wavs_mixed_with_backend<I, P>(
        paths: I,
        requested_backend: BackendKind,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut inputs = Vec::new();
        for path in paths {
            inputs.push(auralis_wav::decode_pcm16_path_with_backend(
                path,
                requested_backend,
            )?);
        }

        Self::from_audio_buffers_mixed_with_backend(&inputs, requested_backend)
    }

    /// Opens multiple PCM16 WAV files and mixes them with equal-power balancing.
    ///
    /// This is the library counterpart to `auralis run --combine mix-power`.
    /// Each input is decoded into planar `f32`, scaled by
    /// `1 / sqrt(input_count)`, and summed with corresponding channels before
    /// any later effects are applied. The output length is the longest input;
    /// missing tail frames and missing channels are silence.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] for decode failures or
    /// [`Error::InputCombine`] when the input list is empty, sample rates or
    /// sample formats are incompatible, or the mixed buffer cannot be
    /// represented.
    pub fn open_wavs_mix_powered<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        Self::open_wavs_mix_powered_with_backend(paths, BackendKind::Scalar)
    }

    /// Opens multiple PCM16 WAV files and mixes them with equal-power balancing and a requested backend.
    ///
    /// `requested_backend` controls decode conversion, the scalar/SIMD mix
    /// kernel, later backend-aware effects, and output encoding after
    /// [`Self::into_pipeline`]. SIMD requests follow Auralis' deterministic
    /// scalar fallback rules when SIMD is unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Wav`] for decode failures or
    /// [`Error::InputCombine`] when the input list is empty, sample rates or
    /// sample formats are incompatible, or the mixed buffer cannot be
    /// represented.
    pub fn open_wavs_mix_powered_with_backend<I, P>(
        paths: I,
        requested_backend: BackendKind,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut inputs = Vec::new();
        for path in paths {
            inputs.push(auralis_wav::decode_pcm16_path_with_backend(
                path,
                requested_backend,
            )?);
        }

        Self::from_audio_buffers_mix_powered_with_backend(&inputs, requested_backend)
    }

    /// Wraps an existing audio buffer in the high-level file type.
    ///
    /// This is primarily useful for tests and applications that decoded audio
    /// through a lower-level crate but still want to use the pipeline builder.
    #[must_use]
    pub const fn from_audio_buffer(audio: AudioBuffer) -> Self {
        Self {
            audio,
            requested_backend: BackendKind::Scalar,
        }
    }

    /// Concatenates existing audio buffers into the high-level file type.
    ///
    /// Inputs are combined before the returned value enters the effect
    /// pipeline. All inputs must share sample rate, channel count, and internal
    /// sample format. Mismatched frame counts are accepted and appended in
    /// caller order.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InputCombine`] when the input list is empty, stream
    /// metadata is incompatible, or the combined buffer shape cannot be
    /// represented.
    pub fn from_audio_buffers_concatenated(inputs: &[AudioBuffer]) -> Result<Self> {
        Self::from_audio_buffers_concatenated_with_backend(inputs, BackendKind::Scalar)
    }

    /// Concatenates existing audio buffers with a requested processing backend.
    ///
    /// The backend is recorded for later pipeline stages. Concatenation itself
    /// is a deterministic structural copy.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InputCombine`] when the input list is empty, stream
    /// metadata is incompatible, or the combined buffer shape cannot be
    /// represented.
    pub fn from_audio_buffers_concatenated_with_backend(
        inputs: &[AudioBuffer],
        requested_backend: BackendKind,
    ) -> Result<Self> {
        Ok(Self {
            audio: concatenate_audio_buffers(inputs)?,
            requested_backend,
        })
    }

    /// Sequences existing audio buffers into the high-level file type.
    ///
    /// Inputs are combined before the returned value enters the effect
    /// pipeline. Auralis currently represents one output buffer, so every
    /// sequence boundary must keep the same sample rate, channel count, and
    /// internal sample format. Mismatched frame counts are accepted and appended
    /// in caller order.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InputCombine`] when the input list is empty, a sequence
    /// boundary cannot be represented, or the combined buffer shape cannot be
    /// represented.
    pub fn from_audio_buffers_sequenced(inputs: &[AudioBuffer]) -> Result<Self> {
        Self::from_audio_buffers_sequenced_with_backend(inputs, BackendKind::Scalar)
    }

    /// Sequences existing audio buffers with a requested processing backend.
    ///
    /// The backend is recorded for later pipeline stages. Sequencing itself is
    /// a deterministic structural copy after boundary validation.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InputCombine`] when the input list is empty, a sequence
    /// boundary cannot be represented, or the combined buffer shape cannot be
    /// represented.
    pub fn from_audio_buffers_sequenced_with_backend(
        inputs: &[AudioBuffer],
        requested_backend: BackendKind,
    ) -> Result<Self> {
        Ok(Self {
            audio: sequence_audio_buffers(inputs)?,
            requested_backend,
        })
    }

    /// Mixes existing audio buffers into the high-level file type.
    ///
    /// Inputs are combined before the returned value enters the effect
    /// pipeline. Each input is scaled by `1 / input_count` and summed into the
    /// corresponding output channel. The output length is the longest input;
    /// missing tail frames and missing channels are silence.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InputCombine`] when the input list is empty, sample
    /// rates or sample formats are incompatible, or the mixed buffer cannot be
    /// represented.
    pub fn from_audio_buffers_mixed(inputs: &[AudioBuffer]) -> Result<Self> {
        Self::from_audio_buffers_mixed_with_backend(inputs, BackendKind::Scalar)
    }

    /// Mixes existing audio buffers with a requested processing backend.
    ///
    /// The backend is recorded for later pipeline stages and selects the
    /// scalar/SIMD mix kernel for this combiner.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InputCombine`] when the input list is empty, sample
    /// rates or sample formats are incompatible, or the mixed buffer cannot be
    /// represented.
    pub fn from_audio_buffers_mixed_with_backend(
        inputs: &[AudioBuffer],
        requested_backend: BackendKind,
    ) -> Result<Self> {
        Ok(Self {
            audio: mix_audio_buffers_with_backend(inputs, requested_backend)?,
            requested_backend,
        })
    }

    /// Mixes existing audio buffers into the high-level file type with equal-power balancing.
    ///
    /// Inputs are combined before the returned value enters the effect
    /// pipeline. Each input is scaled by `1 / sqrt(input_count)` and summed
    /// into the corresponding output channel. The output length is the longest
    /// input; missing tail frames and missing channels are silence.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InputCombine`] when the input list is empty, sample
    /// rates or sample formats are incompatible, or the mixed buffer cannot be
    /// represented.
    pub fn from_audio_buffers_mix_powered(inputs: &[AudioBuffer]) -> Result<Self> {
        Self::from_audio_buffers_mix_powered_with_backend(inputs, BackendKind::Scalar)
    }

    /// Mixes existing audio buffers with equal-power balancing and a requested processing backend.
    ///
    /// The backend is recorded for later pipeline stages and selects the
    /// scalar/SIMD mix kernel for this combiner.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InputCombine`] when the input list is empty, sample
    /// rates or sample formats are incompatible, or the mixed buffer cannot be
    /// represented.
    pub fn from_audio_buffers_mix_powered_with_backend(
        inputs: &[AudioBuffer],
        requested_backend: BackendKind,
    ) -> Result<Self> {
        Ok(Self {
            audio: mix_power_audio_buffers_with_backend(inputs, requested_backend)?,
            requested_backend,
        })
    }

    /// Returns the decoded audio buffer.
    #[must_use]
    pub const fn audio_buffer(&self) -> &AudioBuffer {
        &self.audio
    }

    /// Returns the backend requested for backend-aware processing.
    #[must_use]
    pub const fn requested_backend(&self) -> BackendKind {
        self.requested_backend
    }

    /// Converts the file into a chainable effect pipeline.
    #[must_use]
    pub fn into_pipeline(self) -> Pipeline {
        Pipeline::from_audio_buffer_with_backend(self.audio, self.requested_backend)
    }
}

/// Chainable in-memory audio processing pipeline.
///
/// Effects mutate the internal planar `f32` buffer in order. Methods that take
/// unvalidated user values, such as [`Self::gain_db`], keep the fluent chain
/// shape and defer any validation error until [`Self::write_wav`] or
/// [`Self::into_audio_buffer`] is called. This preserves error propagation
/// without panicking or requiring a `?` after every effect.
#[derive(Debug)]
pub struct Pipeline {
    audio: Result<AudioBuffer>,
    requested_backend: BackendKind,
}

impl Pipeline {
    /// Creates a pipeline from an in-memory audio buffer.
    #[must_use]
    pub fn from_audio_buffer(audio: AudioBuffer) -> Self {
        Self::from_audio_buffer_with_backend(audio, BackendKind::Scalar)
    }

    /// Creates a pipeline from an in-memory audio buffer with a requested backend.
    ///
    /// The backend is used by backend-aware effect kernels and WAV boundary
    /// conversions. Current scalar-only effects keep their scalar behavior until
    /// their own SIMD retrofit features are implemented.
    #[must_use]
    pub fn from_audio_buffer_with_backend(
        audio: AudioBuffer,
        requested_backend: BackendKind,
    ) -> Self {
        Self {
            audio: Ok(audio),
            requested_backend,
        }
    }

    /// Sets the requested backend for later backend-aware pipeline stages.
    ///
    /// Requesting [`BackendKind::Scalar`] forces the scalar reference path.
    /// Requesting [`BackendKind::Simd`] uses SIMD where supported and falls back
    /// to scalar through Auralis backend selection metadata otherwise.
    #[must_use]
    pub const fn with_backend(mut self, requested_backend: BackendKind) -> Self {
        self.requested_backend = requested_backend;
        self
    }

    /// Applies constant gain measured in decibels.
    ///
    /// The gain multiplier is `10^(db / 20)`. Processing is deterministic,
    /// in-place, non-allocating, and uses the requested backend for the
    /// backend-aware `Gain` effect. Scalar remains the default. Output is not
    /// clipped until a boundary writer, such as PCM16 WAV encoding, applies its
    /// documented conversion rules.
    #[must_use]
    pub fn gain_db(mut self, db: f64) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        match Decibels::new(db) {
            Ok(db) => Gain::new(db).process_buffer_with_backend(audio, self.requested_backend),
            Err(error) => self.audio = Err(error.into()),
        }

        self
    }

    /// Applies a constant normalized DC offset.
    ///
    /// `shift` is measured in full-scale sample units, where `0.25` adds one
    /// quarter of full scale and `0.0` is identity. The valid range is
    /// `-2.0..=2.0`, matching SoX-ng's single-argument `dcshift` command.
    /// Processing is deterministic, in-place, non-allocating, and uses the
    /// requested backend for the backend-aware `DcShift` effect. Scalar remains
    /// the default. The effect does not clip; samples outside `[-1.0, 1.0]`
    /// are clipped by boundary writers such as PCM16 WAV encoding.
    #[must_use]
    pub fn dc_shift(mut self, shift: f32) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        match DcShift::new(shift) {
            Ok(dc_shift) => dc_shift.process_buffer_with_backend(audio, self.requested_backend),
            Err(error) => self.audio = Err(error.into()),
        }

        self
    }

    /// Keeps the half-open frame range `start..end` from every channel.
    ///
    /// Frame positions are end-exclusive and measured in audio frames, not
    /// individual samples. The transform preserves channel grouping and returns
    /// an empty buffer when `start == end`. Processing is deterministic and
    /// allocates a new planar buffer for the retained range.
    #[must_use]
    pub fn trim_frames(mut self, start: u64, end: u64) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        match Trim::new(FrameCount::new(start), FrameCount::new(end))
            .and_then(|trim| trim.process_buffer(audio))
        {
            Ok(trimmed) => *audio = trimmed,
            Err(error) => self.audio = Err(error.into()),
        }

        self
    }

    /// Keeps the half-open seconds range `start..end` from every channel.
    ///
    /// Seconds are non-negative finite values. A seconds position is converted
    /// to a frame position by flooring `seconds * sample_rate`; for example,
    /// `0.0000625` at `48_000 Hz` becomes frame `3`. This deterministic rule is
    /// documented so fractional-frame requests never depend on floating-point
    /// rounding mode or CLI formatting. The resulting frame range follows the
    /// same validation and end-exclusive behavior as [`Self::trim_frames`].
    #[must_use]
    pub fn trim_seconds(mut self, start: f64, end: f64) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        let result = TimeSeconds::new(start)
            .map_err(Error::from)
            .and_then(|start| {
                TimeSeconds::new(end)
                    .map_err(Error::from)
                    .map(|end| (start, end))
            })
            .and_then(|(start, end)| {
                let sample_rate = audio.spec().sample_rate().as_u32();
                seconds_to_frame(start, sample_rate)
                    .zip(seconds_to_frame(end, sample_rate))
                    .ok_or(Error::InvalidTrimSecondsRange)
            })
            .and_then(|(start, end)| {
                Trim::new(start, end)
                    .and_then(|trim| trim.process_buffer(audio))
                    .map_err(Error::from)
            });

        match result {
            Ok(trimmed) => *audio = trimmed,
            Err(error) => self.audio = Err(error),
        }

        self
    }

    /// Adds zero-valued frames before and after every channel.
    ///
    /// Padding lengths are measured in audio frames, not individual samples.
    /// The transform preserves channel grouping and returns the original audio
    /// unchanged when both lengths are zero. Processing is deterministic and
    /// allocates a new planar buffer for the padded output.
    #[must_use]
    pub fn pad_frames(mut self, start: u64, end: u64) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        match Pad::new(FrameCount::new(start), FrameCount::new(end)).process_buffer(audio) {
            Ok(padded) => *audio = padded,
            Err(error) => self.audio = Err(error.into()),
        }

        self
    }

    /// Applies linear fade-in and fade-out envelopes measured in frames.
    ///
    /// `fade_in` ramps the start from silence toward unity, and `fade_out`
    /// ramps the end from unity toward silence. A fade length of `4` uses
    /// coefficients `[0.0, 0.25, 0.5, 0.75]` for fade-in and
    /// `[0.75, 0.5, 0.25, 0.0]` for fade-out. If the fade regions overlap,
    /// their coefficients are multiplied. Passing `0` for either side leaves
    /// that side unchanged. Processing is deterministic, in-place,
    /// non-allocating, and uses the requested backend for the backend-aware
    /// `Fade` effect. Scalar remains the default.
    #[must_use]
    pub fn fade_frames(mut self, fade_in: u64, fade_out: u64) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        Fade::new(FrameCount::new(fade_in), FrameCount::new(fade_out))
            .process_buffer_with_backend(audio, self.requested_backend);

        self
    }

    /// Reverses the frame order within each channel.
    ///
    /// This transform is deterministic, in-place, and non-allocating. It is
    /// measured in frames rather than individual interleaved samples, so stereo
    /// and larger channel layouts keep their channel grouping. Applying
    /// `reverse` twice restores the original audio exactly.
    #[must_use]
    pub fn reverse(mut self) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        Reverse::new().process_buffer(audio);

        self
    }

    /// Applies a typed in-memory effect chain in its stored command order.
    ///
    /// This is the command-model counterpart to the fluent methods such as
    /// [`Self::gain_db`] and [`Self::reverse`]. Backend-aware effects inside
    /// the chain use the pipeline's requested backend; structural effects keep
    /// their deterministic scalar behavior. If one command fails, later
    /// commands are skipped and the deferred [`Error::Chain`] identifies the
    /// failing command index, canonical command tokens, argument family, and
    /// typed source error.
    #[must_use]
    pub fn apply_effect_chain(mut self, chain: &EffectChain) -> Self {
        let Ok(audio) = &mut self.audio else {
            return self;
        };

        if let Err(error) = chain.process_buffer_with_backend(audio, self.requested_backend) {
            self.audio = Err(error.into());
        }

        self
    }

    /// Returns the processed audio buffer.
    ///
    /// # Errors
    ///
    /// Returns the first deferred configuration or processing error from the
    /// chain.
    pub fn into_audio_buffer(self) -> Result<AudioBuffer> {
        self.audio
    }

    /// Encodes the processed audio as a PCM16 WAV file.
    ///
    /// Existing files at `path` are overwritten. Encoding clips finite samples
    /// to the PCM16 range as documented by [`auralis_wav::encode_pcm16_path`].
    ///
    /// # Errors
    ///
    /// Returns the first deferred configuration error from the chain, or a WAV
    /// write error if output creation, sample validation, sample writing, or
    /// finalization fails.
    pub fn write_wav(self, path: impl AsRef<Path>) -> Result<()> {
        let audio = self.audio?;
        auralis_wav::encode_pcm16_path_with_backend(path, &audio, self.requested_backend)?;

        Ok(())
    }
}

fn seconds_to_frame(seconds: TimeSeconds, sample_rate: u32) -> Option<FrameCount> {
    let scaled = seconds.as_f64() * f64::from(sample_rate);
    #[allow(
        clippy::cast_precision_loss,
        reason = "This boundary check only needs the representable f64 magnitude of u64::MAX before the explicit saturating cast below."
    )]
    let max_frame = u64::MAX as f64;
    if !scaled.is_finite() || scaled > max_frame {
        return None;
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "TimeSeconds validates finite non-negative input; floor defines fractional-frame behavior before a checked range comparison."
    )]
    let frame = scaled.floor() as u64;

    Some(FrameCount::new(frame))
}

#[cfg(test)]
mod tests {
    use super::{
        AudioFile, BackendKind, EffectChain, EffectCommand, Error, InputCombineError,
        concatenate_audio_buffers, mix_audio_buffers, mix_audio_buffers_with_backend,
        mix_power_audio_buffers, mix_power_audio_buffers_with_backend, sequence_audio_buffers,
    };
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
    };
    use auralis_effects::{DcShift, Fade, Gain, Reverse, Trim};

    #[test]
    fn chain_gain_matches_direct_effect_execution() {
        let source = audio_buffer(vec![0.25, -0.5, 1.0]);
        let mut expected = source.clone();

        Gain::new(Decibels::new(-3.0).unwrap()).process_buffer(&mut expected);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .gain_db(-3.0)
            .into_audio_buffer()
            .unwrap();

        assert_samples_close(actual.as_planar_f32(), expected.as_planar_f32());
    }

    #[test]
    fn chain_gain_matches_under_forced_scalar_and_requested_simd() {
        let source = audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -0.5,
            -0.0,
            0.0,
            0.5,
            0.999_984_74,
            1.0,
        ]);

        let scalar = AudioFile::from_audio_buffer(source.clone())
            .into_pipeline()
            .with_backend(BackendKind::Scalar)
            .gain_db(-3.0)
            .into_audio_buffer()
            .unwrap();
        let simd = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .with_backend(BackendKind::Simd)
            .gain_db(-3.0)
            .into_audio_buffer()
            .unwrap();

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn chain_dc_shift_preserves_stereo_frame_grouping() {
        let source = stereo_audio_buffer(vec![0.75, 1.0, -0.75, -1.0]);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .dc_shift(0.5)
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.as_planar_f32(), &[1.25, 1.5, -0.25, -0.5]);
    }

    #[test]
    fn chain_dc_shift_matches_under_forced_scalar_and_requested_simd() {
        let source = audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -f32::MIN_POSITIVE,
            -f32::from_bits(1),
            -0.0,
            0.0,
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            0.999_984_74,
            1.0,
        ]);

        let scalar = AudioFile::from_audio_buffer(source.clone())
            .into_pipeline()
            .with_backend(BackendKind::Scalar)
            .dc_shift(0.125)
            .into_audio_buffer()
            .unwrap();
        let simd = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .with_backend(BackendKind::Simd)
            .dc_shift(0.125)
            .into_audio_buffer()
            .unwrap();

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn chain_trim_frames_preserves_stereo_frame_grouping() {
        let source = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 0.75, 1.0, -0.25, -0.5, -0.75]);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .trim_frames(1, 3)
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.as_planar_f32(), &[0.25, 0.5, -0.25, -0.5]);
    }

    #[test]
    fn chain_trim_seconds_uses_floor_conversion() {
        let actual = AudioFile::from_audio_buffer(audio_buffer(vec![0.0, 0.25, 0.5, 0.75]))
            .into_pipeline()
            .trim_seconds(1.0 / 48_000.0, 3.9 / 48_000.0)
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.as_planar_f32(), &[0.25, 0.5]);
    }

    #[test]
    fn chain_pad_frames_preserves_stereo_frame_grouping() {
        let source = stereo_audio_buffer(vec![0.25, 0.5, -0.25, -0.5]);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .pad_frames(1, 1)
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(4));
        assert_eq!(
            actual.as_planar_f32(),
            &[0.0, 0.25, 0.5, 0.0, 0.0, -0.25, -0.5, 0.0]
        );
    }

    #[test]
    fn chain_fade_frames_preserves_stereo_frame_grouping() {
        let source = stereo_audio_buffer(vec![1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0]);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .fade_frames(2, 2)
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(4));
        assert_samples_close(
            actual.as_planar_f32(),
            &[0.0, 0.5, 0.5, 0.0, -0.0, -0.5, -0.5, -0.0],
        );
    }

    #[test]
    fn chain_fade_frames_matches_under_forced_scalar_and_requested_simd() {
        let source = stereo_audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -0.5,
            -0.0,
            0.0,
            0.5,
            0.999_984_74,
            1.0,
            1.0,
            0.999_984_74,
            0.5,
            0.0,
            -0.0,
            -0.5,
            -0.999_984_74,
            -1.0,
        ]);

        let scalar = AudioFile::from_audio_buffer(source.clone())
            .into_pipeline()
            .with_backend(BackendKind::Scalar)
            .fade_frames(5, 7)
            .into_audio_buffer()
            .unwrap();
        let simd = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .with_backend(BackendKind::Simd)
            .fade_frames(5, 7)
            .into_audio_buffer()
            .unwrap();

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn chain_reverse_preserves_stereo_frame_grouping() {
        let source = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 1.0, -0.25, -0.5]);

        let actual = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .reverse()
            .into_audio_buffer()
            .unwrap();

        assert_eq!(actual.frames(), FrameCount::new(3));
        assert_eq!(actual.as_planar_f32(), &[0.5, 0.25, 0.0, -0.5, -0.25, 1.0]);
    }

    #[test]
    fn apply_effect_chain_matches_repeated_fluent_pipeline_calls() {
        let source = stereo_audio_buffer(vec![0.25, -0.5, 0.75, 1.0, -0.25, 0.5, -0.75, -1.0]);
        let chain = EffectChain::new(vec![
            EffectCommand::Gain(Gain::new(Decibels::new(-3.0).unwrap())),
            EffectCommand::DcShift(DcShift::new(0.125).unwrap()),
            EffectCommand::Fade(Fade::new(FrameCount::new(2), FrameCount::new(2))),
            EffectCommand::Trim(Trim::new(FrameCount::new(1), FrameCount::new(3)).unwrap()),
            EffectCommand::Reverse(Reverse::new()),
        ]);

        let actual = AudioFile::from_audio_buffer(source.clone())
            .into_pipeline()
            .apply_effect_chain(&chain)
            .into_audio_buffer()
            .unwrap();
        let expected = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .gain_db(-3.0)
            .dc_shift(0.125)
            .fade_frames(2, 2)
            .trim_frames(1, 3)
            .reverse()
            .into_audio_buffer()
            .unwrap();

        assert_samples_close(actual.as_planar_f32(), expected.as_planar_f32());
        assert_eq!(actual.frames(), expected.frames());
        assert_eq!(actual.channels(), expected.channels());
    }

    #[test]
    fn apply_effect_chain_matches_under_forced_scalar_and_requested_simd() {
        let source = stereo_audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -0.5,
            -0.0,
            0.0,
            0.5,
            0.999_984_74,
            1.0,
            1.0,
            0.999_984_74,
            0.5,
            0.0,
            -0.0,
            -0.5,
            -0.999_984_74,
            -1.0,
        ]);
        let chain = EffectChain::new(vec![
            EffectCommand::Gain(Gain::new(Decibels::new(-3.0).unwrap())),
            EffectCommand::DcShift(DcShift::new(0.125).unwrap()),
            EffectCommand::Fade(Fade::new(FrameCount::new(5), FrameCount::new(7))),
            EffectCommand::Reverse(Reverse::new()),
        ]);

        let scalar = AudioFile::from_audio_buffer(source.clone())
            .into_pipeline()
            .with_backend(BackendKind::Scalar)
            .apply_effect_chain(&chain)
            .into_audio_buffer()
            .unwrap();
        let simd = AudioFile::from_audio_buffer(source)
            .into_pipeline()
            .with_backend(BackendKind::Simd)
            .apply_effect_chain(&chain)
            .into_audio_buffer()
            .unwrap();

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn apply_effect_chain_errors_include_failing_command_context() {
        let chain = EffectChain::new(vec![EffectCommand::Trim(
            Trim::new(FrameCount::new(0), FrameCount::new(3)).unwrap(),
        )]);

        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25, -0.5]))
            .into_pipeline()
            .apply_effect_chain(&chain)
            .into_audio_buffer()
            .unwrap_err();

        assert!(matches!(
            error,
            Error::Chain(auralis_effects::EffectChainError::CommandFailed {
                index: 0,
                command: EffectCommand::Trim(_),
                argument: "frame-range",
                source: auralis_effects::EffectError::TrimRangeOutOfBounds,
            })
        ));
        assert_eq!(
            error.to_string(),
            "effect chain command 0 (`trim 0 3`) failed while applying `frame-range`: trim frame range must be within the input duration"
        );
    }

    #[test]
    fn concatenate_audio_buffers_accepts_mismatched_mono_lengths() {
        let first = audio_buffer(vec![0.25, -0.5, 0.75]);
        let second = audio_buffer(vec![1.0]);

        let actual = concatenate_audio_buffers(&[first, second]).unwrap();

        assert_eq!(actual.frames(), FrameCount::new(4));
        assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
        assert_eq!(actual.as_planar_f32(), &[0.25, -0.5, 0.75, 1.0]);
    }

    #[test]
    fn concatenate_audio_buffers_preserves_stereo_channel_grouping() {
        let first = stereo_audio_buffer(vec![1.0, 2.0, -1.0, -2.0]);
        let second = stereo_audio_buffer(vec![3.0, -3.0]);

        let actual = concatenate_audio_buffers(&[first, second]).unwrap();

        assert_eq!(actual.frames(), FrameCount::new(3));
        assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(actual.as_planar_f32(), &[1.0, 2.0, 3.0, -1.0, -2.0, -3.0]);
    }

    #[test]
    fn concatenate_audio_buffers_rejects_mismatched_channel_count() {
        let first = audio_buffer(vec![0.25, -0.5]);
        let second = stereo_audio_buffer(vec![1.0, 2.0, -1.0, -2.0]);

        let error = concatenate_audio_buffers(&[first, second]).unwrap_err();

        assert_eq!(
            error,
            InputCombineError::MismatchedChannelCount {
                input_index: 1,
                expected: ChannelCount::new(1).unwrap(),
                actual: ChannelCount::new(2).unwrap(),
            }
        );
    }

    #[test]
    fn concatenate_audio_buffers_rejects_mismatched_sample_rate() {
        let first = audio_buffer(vec![0.25]);
        let second = audio_buffer_with_spec(vec![-0.25], 44_100, 1, SampleFormat::Float32);

        let error = concatenate_audio_buffers(&[first, second]).unwrap_err();

        assert_eq!(
            error,
            InputCombineError::MismatchedSampleRate {
                input_index: 1,
                expected: SampleRate::new(48_000).unwrap(),
                actual: SampleRate::new(44_100).unwrap(),
            }
        );
    }

    #[test]
    fn concatenate_audio_buffers_rejects_mismatched_sample_format() {
        let first = audio_buffer(vec![0.25]);
        let second = audio_buffer_with_spec(vec![-0.25], 48_000, 1, SampleFormat::Pcm16);

        let error = concatenate_audio_buffers(&[first, second]).unwrap_err();

        assert_eq!(
            error,
            InputCombineError::MismatchedSampleFormat {
                input_index: 1,
                expected: SampleFormat::Float32,
                actual: SampleFormat::Pcm16,
            }
        );
    }

    #[test]
    fn concatenated_audio_enters_effect_pipeline_before_effects() {
        let first = audio_buffer(vec![0.25, -0.5]);
        let second = audio_buffer(vec![0.75]);

        let actual = AudioFile::from_audio_buffers_concatenated(&[first, second])
            .unwrap()
            .into_pipeline()
            .gain_db(6.0)
            .reverse()
            .into_audio_buffer()
            .unwrap();

        let multiplier = 10.0_f32.powf(6.0 / 20.0);
        assert_samples_close(
            actual.as_planar_f32(),
            &[0.75 * multiplier, -0.5 * multiplier, 0.25 * multiplier],
        );
    }

    #[test]
    fn open_wavs_concatenated_round_trips_through_file_boundary() {
        let tempdir = temp_dir();
        fs::create_dir(&tempdir).unwrap();
        let first = tempdir.join("first.wav");
        let second = tempdir.join("second.wav");
        let output = tempdir.join("output.wav");

        auralis_wav::encode_pcm16_path(&first, &audio_buffer(vec![0.25, -0.5])).unwrap();
        auralis_wav::encode_pcm16_path(&second, &audio_buffer(vec![0.75])).unwrap();

        AudioFile::open_wavs_concatenated([&first, &second])
            .unwrap()
            .into_pipeline()
            .write_wav(&output)
            .unwrap();

        let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
        assert_samples_close(decoded.as_planar_f32(), &[0.25, -0.5, 0.75]);
        fs::remove_dir_all(tempdir).unwrap();
    }

    #[test]
    fn sequence_audio_buffers_accepts_mismatched_mono_lengths() {
        let first = audio_buffer(vec![0.25, -0.5]);
        let second = audio_buffer(vec![0.75, 0.0, -0.25]);

        let actual = sequence_audio_buffers(&[first, second]).unwrap();

        assert_eq!(actual.frames(), FrameCount::new(5));
        assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
        assert_eq!(actual.as_planar_f32(), &[0.25, -0.5, 0.75, 0.0, -0.25]);
    }

    #[test]
    fn sequence_audio_buffers_preserves_stereo_channel_grouping() {
        let first = stereo_audio_buffer(vec![1.0, 2.0, -1.0, -2.0]);
        let second = stereo_audio_buffer(vec![3.0, -3.0]);

        let actual = sequence_audio_buffers(&[first, second]).unwrap();

        assert_eq!(actual.frames(), FrameCount::new(3));
        assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(actual.as_planar_f32(), &[1.0, 2.0, 3.0, -1.0, -2.0, -3.0]);
    }

    #[test]
    fn sequence_audio_buffers_rejects_unrepresentable_channel_boundary() {
        let first = audio_buffer(vec![0.25, -0.5]);
        let second = stereo_audio_buffer(vec![1.0, 2.0, -1.0, -2.0]);

        let error = sequence_audio_buffers(&[first, second]).unwrap_err();

        assert_eq!(
            error,
            InputCombineError::SequenceBoundaryChannelCount {
                input_index: 1,
                previous_index: 0,
                expected: ChannelCount::new(1).unwrap(),
                actual: ChannelCount::new(2).unwrap(),
            }
        );
    }

    #[test]
    fn sequence_audio_buffers_rejects_unrepresentable_sample_rate_boundary() {
        let first = audio_buffer(vec![0.25]);
        let second = audio_buffer_with_spec(vec![-0.25], 44_100, 1, SampleFormat::Float32);

        let error = sequence_audio_buffers(&[first, second]).unwrap_err();

        assert_eq!(
            error,
            InputCombineError::SequenceBoundarySampleRate {
                input_index: 1,
                previous_index: 0,
                expected: SampleRate::new(48_000).unwrap(),
                actual: SampleRate::new(44_100).unwrap(),
            }
        );
    }

    #[test]
    fn sequenced_audio_enters_effect_pipeline_before_effects() {
        let first = audio_buffer(vec![0.25, -0.5]);
        let second = audio_buffer(vec![0.75]);

        let actual = AudioFile::from_audio_buffers_sequenced(&[first, second])
            .unwrap()
            .into_pipeline()
            .gain_db(6.0)
            .reverse()
            .into_audio_buffer()
            .unwrap();

        let multiplier = 10.0_f32.powf(6.0 / 20.0);
        assert_samples_close(
            actual.as_planar_f32(),
            &[0.75 * multiplier, -0.5 * multiplier, 0.25 * multiplier],
        );
    }

    #[test]
    fn open_wavs_sequenced_round_trips_through_file_boundary() {
        let tempdir = temp_dir();
        fs::create_dir(&tempdir).unwrap();
        let first = tempdir.join("first.wav");
        let second = tempdir.join("second.wav");
        let output = tempdir.join("output.wav");

        auralis_wav::encode_pcm16_path(&first, &audio_buffer(vec![0.25, -0.5])).unwrap();
        auralis_wav::encode_pcm16_path(&second, &audio_buffer(vec![0.75])).unwrap();

        AudioFile::open_wavs_sequenced([&first, &second])
            .unwrap()
            .into_pipeline()
            .write_wav(&output)
            .unwrap();

        let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
        assert_samples_close(decoded.as_planar_f32(), &[0.25, -0.5, 0.75]);
        fs::remove_dir_all(tempdir).unwrap();
    }

    #[test]
    fn mix_audio_buffers_averages_equal_length_mono_inputs() {
        let first = audio_buffer(vec![1.0, -1.0, 0.5]);
        let second = audio_buffer(vec![0.5, 1.0, -0.5]);

        let actual = mix_audio_buffers(&[first, second]).unwrap();

        assert_eq!(actual.frames(), FrameCount::new(3));
        assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
        assert_sample_bits_eq(actual.as_planar_f32(), &[0.75, 0.0, 0.0]);
    }

    #[test]
    fn mix_audio_buffers_treats_mismatched_lengths_as_trailing_silence() {
        let first = audio_buffer(vec![0.5, -0.5]);
        let second = audio_buffer(vec![1.0, 1.0, 1.0]);

        let actual = mix_audio_buffers(&[first, second]).unwrap();

        assert_eq!(actual.frames(), FrameCount::new(3));
        assert_sample_bits_eq(actual.as_planar_f32(), &[0.75, 0.25, 0.5]);
    }

    #[test]
    fn mix_audio_buffers_preserves_stereo_channel_grouping() {
        let first = stereo_audio_buffer(vec![0.5, -0.5, 1.0, -1.0]);
        let second = stereo_audio_buffer(vec![0.5, 0.5, -0.5, -0.5]);

        let actual = mix_audio_buffers(&[first, second]).unwrap();

        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
        assert_sample_bits_eq(actual.as_planar_f32(), &[0.5, 0.0, 0.25, -0.75]);
    }

    #[test]
    fn mix_audio_buffers_accepts_mismatched_channel_counts_with_silence() {
        let mono = audio_buffer(vec![0.5, -0.5]);
        let stereo = stereo_audio_buffer(vec![1.0, 0.0, -1.0, 0.5]);

        let actual = mix_audio_buffers(&[mono, stereo]).unwrap();

        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
        assert_sample_bits_eq(actual.as_planar_f32(), &[0.75, -0.25, -0.5, 0.25]);
    }

    #[test]
    fn mix_audio_buffers_rejects_mismatched_sample_rate() {
        let first = audio_buffer(vec![0.25]);
        let second = audio_buffer_with_spec(vec![-0.25], 44_100, 1, SampleFormat::Float32);

        let error = mix_audio_buffers(&[first, second]).unwrap_err();

        assert_eq!(
            error,
            InputCombineError::MismatchedSampleRate {
                input_index: 1,
                expected: SampleRate::new(48_000).unwrap(),
                actual: SampleRate::new(44_100).unwrap(),
            }
        );
    }

    #[test]
    fn mix_audio_buffers_rejects_mismatched_sample_format() {
        let first = audio_buffer(vec![0.25]);
        let second = audio_buffer_with_spec(vec![-0.25], 48_000, 1, SampleFormat::Pcm16);

        let error = mix_audio_buffers(&[first, second]).unwrap_err();

        assert_eq!(
            error,
            InputCombineError::MismatchedSampleFormat {
                input_index: 1,
                expected: SampleFormat::Float32,
                actual: SampleFormat::Pcm16,
            }
        );
    }

    #[test]
    fn mix_audio_buffers_matches_under_forced_scalar_and_requested_simd() {
        let first = audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -0.5,
            -0.0,
            0.0,
            0.5,
            0.999_984_74,
            1.0,
        ]);
        let second = audio_buffer(vec![1.0, 0.999_984_74, 0.5, 0.0, -0.0]);

        let scalar =
            mix_audio_buffers_with_backend(&[first.clone(), second.clone()], BackendKind::Scalar)
                .unwrap();
        let simd = mix_audio_buffers_with_backend(&[first, second], BackendKind::Simd).unwrap();

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn mixed_audio_enters_effect_pipeline_before_effects() {
        let first = audio_buffer(vec![0.25, -0.5]);
        let second = audio_buffer(vec![0.75, 0.0, -0.25]);

        let actual = AudioFile::from_audio_buffers_mixed(&[first, second])
            .unwrap()
            .into_pipeline()
            .gain_db(6.0)
            .reverse()
            .into_audio_buffer()
            .unwrap();

        let multiplier = 10.0_f32.powf(6.0 / 20.0);
        assert_samples_close(
            actual.as_planar_f32(),
            &[
                (-0.25 * 0.5) * multiplier,
                (-0.5 * 0.5) * multiplier,
                0.5 * multiplier,
            ],
        );
    }

    #[test]
    fn mixed_audio_is_not_clipped_until_wav_write_boundary() {
        let tempdir = temp_dir();
        fs::create_dir(&tempdir).unwrap();
        let output = tempdir.join("output.wav");
        let first = audio_buffer(vec![2.0]);
        let second = audio_buffer(vec![2.0]);

        let mixed = mix_audio_buffers(&[first, second]).unwrap();

        assert_sample_bits_eq(mixed.as_planar_f32(), &[2.0]);
        AudioFile::from_audio_buffer(mixed)
            .into_pipeline()
            .write_wav(&output)
            .unwrap();
        let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
        assert_samples_close(decoded.as_planar_f32(), &[f32::from(i16::MAX) / 32768.0]);
        fs::remove_dir_all(tempdir).unwrap();
    }

    #[test]
    fn open_wavs_mixed_round_trips_through_file_boundary() {
        let tempdir = temp_dir();
        fs::create_dir(&tempdir).unwrap();
        let first = tempdir.join("first.wav");
        let second = tempdir.join("second.wav");
        let output = tempdir.join("output.wav");

        auralis_wav::encode_pcm16_path(&first, &audio_buffer(vec![0.25, -0.5])).unwrap();
        auralis_wav::encode_pcm16_path(&second, &audio_buffer(vec![0.75, 0.0, -0.25])).unwrap();

        AudioFile::open_wavs_mixed([&first, &second])
            .unwrap()
            .into_pipeline()
            .write_wav(&output)
            .unwrap();

        let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
        assert_samples_close(decoded.as_planar_f32(), &[0.5, -0.25, -0.125]);
        fs::remove_dir_all(tempdir).unwrap();
    }

    #[test]
    fn mix_power_audio_buffers_uses_equal_power_scale_for_equal_length_mono_inputs() {
        let first = audio_buffer(vec![1.0, -1.0, 0.5]);
        let second = audio_buffer(vec![0.5, 1.0, -0.5]);
        let scale = 1.0_f32 / 2.0_f32.sqrt();

        let actual = mix_power_audio_buffers(&[first, second]).unwrap();

        assert_eq!(actual.frames(), FrameCount::new(3));
        assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
        assert_sample_bits_eq(
            actual.as_planar_f32(),
            &[
                1.0 * scale + 0.5 * scale,
                -scale + scale,
                0.5 * scale - 0.5 * scale,
            ],
        );
    }

    #[test]
    fn mix_power_audio_buffers_treats_mismatched_lengths_as_trailing_silence() {
        let first = audio_buffer(vec![0.5, -0.5]);
        let second = audio_buffer(vec![1.0, 1.0, 1.0]);
        let scale = 1.0_f32 / 2.0_f32.sqrt();

        let actual = mix_power_audio_buffers(&[first, second]).unwrap();

        assert_eq!(actual.frames(), FrameCount::new(3));
        assert_sample_bits_eq(
            actual.as_planar_f32(),
            &[
                0.5 * scale + 1.0 * scale,
                -0.5 * scale + 1.0 * scale,
                1.0 * scale,
            ],
        );
    }

    #[test]
    fn mix_power_audio_buffers_accepts_mismatched_channel_counts_with_silence() {
        let mono = audio_buffer(vec![0.5, -0.5]);
        let stereo = stereo_audio_buffer(vec![1.0, 0.0, -1.0, 0.5]);
        let scale = 1.0_f32 / 2.0_f32.sqrt();

        let actual = mix_power_audio_buffers(&[mono, stereo]).unwrap();

        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
        assert_sample_bits_eq(
            actual.as_planar_f32(),
            &[0.5 * scale + 1.0 * scale, -0.5 * scale, -scale, 0.5 * scale],
        );
    }

    #[test]
    fn mix_power_audio_buffers_rejects_mismatched_sample_rate() {
        let first = audio_buffer(vec![0.25]);
        let second = audio_buffer_with_spec(vec![-0.25], 44_100, 1, SampleFormat::Float32);

        let error = mix_power_audio_buffers(&[first, second]).unwrap_err();

        assert_eq!(
            error,
            InputCombineError::MismatchedSampleRate {
                input_index: 1,
                expected: SampleRate::new(48_000).unwrap(),
                actual: SampleRate::new(44_100).unwrap(),
            }
        );
    }

    #[test]
    fn mix_power_audio_buffers_matches_under_forced_scalar_and_requested_simd() {
        let first = audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -0.5,
            -0.0,
            0.0,
            0.5,
            0.999_984_74,
            1.0,
        ]);
        let second = audio_buffer(vec![1.0, 0.999_984_74, 0.5, 0.0, -0.0]);

        let scalar = mix_power_audio_buffers_with_backend(
            &[first.clone(), second.clone()],
            BackendKind::Scalar,
        )
        .unwrap();
        let simd =
            mix_power_audio_buffers_with_backend(&[first, second], BackendKind::Simd).unwrap();

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn mix_powered_audio_enters_effect_pipeline_before_effects() {
        let first = audio_buffer(vec![0.25, -0.5]);
        let second = audio_buffer(vec![0.75, 0.0, -0.25]);
        let scale = 1.0_f32 / 2.0_f32.sqrt();

        let actual = AudioFile::from_audio_buffers_mix_powered(&[first, second])
            .unwrap()
            .into_pipeline()
            .gain_db(6.0)
            .reverse()
            .into_audio_buffer()
            .unwrap();

        let multiplier = 10.0_f32.powf(6.0 / 20.0);
        assert_samples_close(
            actual.as_planar_f32(),
            &[
                (-0.25 * scale) * multiplier,
                (-0.5 * scale) * multiplier,
                (0.25 * scale + 0.75 * scale) * multiplier,
            ],
        );
    }

    #[test]
    fn open_wavs_mix_powered_round_trips_through_file_boundary() {
        let tempdir = temp_dir();
        fs::create_dir(&tempdir).unwrap();
        let first = tempdir.join("first.wav");
        let second = tempdir.join("second.wav");
        let output = tempdir.join("output.wav");

        auralis_wav::encode_pcm16_path(&first, &audio_buffer(vec![0.25, -0.5])).unwrap();
        auralis_wav::encode_pcm16_path(&second, &audio_buffer(vec![0.75, 0.0, -0.25])).unwrap();

        AudioFile::open_wavs_mix_powered([&first, &second])
            .unwrap()
            .into_pipeline()
            .write_wav(&output)
            .unwrap();

        let scale = 1.0_f32 / 2.0_f32.sqrt();
        let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
        assert_samples_close(
            decoded.as_planar_f32(),
            &[scale, -0.5 * scale, -0.25 * scale],
        );
        fs::remove_dir_all(tempdir).unwrap();
    }

    #[test]
    fn invalid_trim_range_propagates_without_panic() {
        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
            .into_pipeline()
            .trim_frames(2, 1)
            .into_audio_buffer()
            .unwrap_err();

        assert_eq!(
            error,
            Error::Effect(auralis_effects::EffectError::InvalidTrimOrder)
        );
    }

    #[test]
    fn invalid_trim_seconds_propagates_without_panic() {
        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
            .into_pipeline()
            .trim_seconds(f64::NAN, 1.0)
            .into_audio_buffer()
            .unwrap_err();

        assert_eq!(
            error,
            Error::Core(auralis_core::AuralisError::InvalidTimeSeconds)
        );
    }

    #[test]
    fn invalid_gain_propagates_without_panic() {
        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
            .into_pipeline()
            .gain_db(f64::NAN)
            .into_audio_buffer()
            .unwrap_err();

        assert_eq!(
            error,
            Error::Core(auralis_core::AuralisError::InvalidDecibels)
        );
    }

    #[test]
    fn invalid_dc_shift_propagates_without_panic() {
        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
            .into_pipeline()
            .dc_shift(f32::NAN)
            .into_audio_buffer()
            .unwrap_err();

        assert_eq!(
            error,
            Error::Effect(auralis_effects::EffectError::InvalidDcShift)
        );
    }

    #[test]
    fn wav_chain_round_trips_through_file_boundary() {
        let tempdir = temp_dir();
        fs::create_dir(&tempdir).unwrap();
        let input = tempdir.join("input.wav");
        let output = tempdir.join("output.wav");
        let source = audio_buffer(vec![0.25, -0.5, 0.75]);

        auralis_wav::encode_pcm16_path(&input, &source).unwrap();

        AudioFile::open_wav(&input)
            .unwrap()
            .into_pipeline()
            .gain_db(0.0)
            .write_wav(&output)
            .unwrap();

        let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
        assert_samples_close(decoded.as_planar_f32(), source.as_planar_f32());
        fs::remove_dir_all(tempdir).unwrap();
    }

    #[test]
    fn deferred_error_prevents_later_processing() {
        let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
            .into_pipeline()
            .gain_db(f64::INFINITY)
            .gain_db(6.0)
            .into_audio_buffer()
            .unwrap_err();

        assert_eq!(
            error,
            Error::Core(auralis_core::AuralisError::InvalidDecibels)
        );
    }

    fn audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        let frames = samples.len().try_into().unwrap();
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        );

        AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
    }

    fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        assert_eq!(samples.len() % 2, 0);
        let frames = u64::try_from(samples.len() / 2).unwrap();

        audio_buffer_with_shape(samples, frames, 48_000, 2, SampleFormat::Float32)
    }

    fn audio_buffer_with_spec(
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
    ) -> AudioBuffer {
        assert_eq!(samples.len() % usize::from(channels), 0);
        let frames = u64::try_from(samples.len() / usize::from(channels)).unwrap();

        audio_buffer_with_shape(samples, frames, sample_rate, channels, sample_format)
    }

    fn audio_buffer_with_shape(
        samples: Vec<f32>,
        frames: u64,
        sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
    ) -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(sample_rate).unwrap(),
            ChannelCount::new(channels).unwrap(),
            sample_format,
        );

        AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());

        for (actual, expected) in actual.iter().zip(expected) {
            let tolerance = 1.0e-4;
            let difference = (actual - expected).abs();

            assert!(
                difference <= tolerance,
                "expected {actual} to be within {tolerance} of {expected}, difference was {difference}"
            );
        }
    }

    fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());

        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "sample {index} differed: {actual} != {expected}"
            );
        }
    }

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        std::env::temp_dir().join(format!("auralis-chain-api-{nanos}"))
    }
}
