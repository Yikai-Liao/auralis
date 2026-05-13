use std::path::Path;

use crate::{
    AudioBuffer, BackendKind, Pipeline, Result, concatenate_audio_buffers, merge_audio_buffers,
    mix_audio_buffers_with_backend, mix_power_audio_buffers_with_backend,
    multiply_audio_buffers_with_backend, sequence_audio_buffers,
};

/// Decoded audio file ready to enter an effect pipeline.
///
/// `AudioFile` supports the checked-in WAV family, FLAC, and AU/SND decode paths.
/// Decoding always uses Auralis' internal planar `f32` [`AudioBuffer`]
/// representation.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioFile {
    audio: AudioBuffer,
    requested_backend: BackendKind,
}

impl AudioFile {
    /// Opens a supported linear PCM WAV file and decodes it into planar `f32`
    /// samples.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] when the path cannot be opened, the input is not
    /// a well-formed WAV stream, or the sample format is not supported.
    pub fn open_wav(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_wav_with_backend(path, BackendKind::Scalar)
    }

    /// Opens a supported linear PCM WAV file using the requested
    /// sample-conversion backend.
    ///
    /// The decoded audio is identical to [`Self::open_wav`]. `requested_backend`
    /// controls only backend-aware decode, later backend-aware effect kernels,
    /// and WAV encoding after [`Self::into_pipeline`]. Unsupported SIMD requests
    /// follow Auralis' documented scalar fallback.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] when the path cannot be opened, the input is not
    /// a well-formed WAV stream, or the sample format is not supported.
    pub fn open_wav_with_backend(
        path: impl AsRef<Path>,
        requested_backend: BackendKind,
    ) -> Result<Self> {
        Ok(Self {
            audio: auralis_wav::decode_wav_path_with_backend(path, requested_backend)?,
            requested_backend,
        })
    }

    /// Opens a supported FLAC file and decodes it into planar `f32` samples.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Flac`] when the path cannot be opened, the input
    /// is not a well-formed FLAC stream, or the sample format is outside the
    /// supported integer range.
    pub fn open_flac(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            audio: auralis_flac::decode_flac_path(path)?,
            requested_backend: BackendKind::Scalar,
        })
    }

    /// Opens a supported AU/SND file and decodes it into planar `f32` samples.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Au`] when the path cannot be opened, the input
    /// is not a well-formed AU/SND stream, or the sample encoding is outside
    /// the supported PCM, float, and G.711 set.
    pub fn open_au(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            audio: auralis_au::decode_au_path(path)?,
            requested_backend: BackendKind::Scalar,
        })
    }

    /// Opens multiple supported linear PCM WAV files and concatenates them in
    /// caller order.
    ///
    /// This is the library counterpart to `auralis run --combine concatenate`.
    /// Each input is decoded into planar `f32`, then the buffers are
    /// concatenated before any later pipeline effects are applied. All inputs
    /// must have the same sample rate and channel count; frame lengths may
    /// differ.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty or stream metadata
    /// is incompatible.
    pub fn open_wavs_concatenated<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        Self::open_wavs_concatenated_with_backend(paths, BackendKind::Scalar)
    }

    /// Opens multiple supported linear PCM WAV files and concatenates them with
    /// a requested backend.
    ///
    /// `requested_backend` controls decode conversion, later backend-aware
    /// effects, and output encoding after [`Self::into_pipeline`]. The
    /// concatenate combiner itself is a structural copy and does not select a
    /// SIMD kernel.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty or stream metadata
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
            inputs.push(auralis_wav::decode_wav_path_with_backend(
                path,
                requested_backend,
            )?);
        }

        Self::from_audio_buffers_concatenated_with_backend(&inputs, requested_backend)
    }

    /// Opens multiple supported linear PCM WAV files and sequences them in
    /// caller order.
    ///
    /// This is the library counterpart to `auralis run --combine sequence`.
    /// Auralis writes one output buffer/file, so sequence boundaries must keep
    /// the same sample rate and channel count. Representable boundaries append
    /// decoded input samples in serial playback order before any later effects
    /// are applied. Frame lengths may differ.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty or a sequence
    /// boundary cannot be represented by one output buffer.
    pub fn open_wavs_sequenced<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        Self::open_wavs_sequenced_with_backend(paths, BackendKind::Scalar)
    }

    /// Opens multiple supported linear PCM WAV files and sequences them with a
    /// requested backend.
    ///
    /// `requested_backend` controls decode conversion, later backend-aware
    /// effects, and output encoding after [`Self::into_pipeline`]. Sequencing
    /// itself is a structural copy after boundary validation and does not
    /// select a SIMD kernel.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty or a sequence
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
            inputs.push(auralis_wav::decode_wav_path_with_backend(
                path,
                requested_backend,
            )?);
        }

        Self::from_audio_buffers_sequenced_with_backend(&inputs, requested_backend)
    }

    /// Opens multiple supported linear PCM WAV files and mixes them into one
    /// buffer.
    ///
    /// This is the library counterpart to `auralis run --combine mix`. Each
    /// input is decoded into planar `f32`, scaled by `1 / input_count`, and
    /// summed with corresponding channels before any later effects are applied.
    /// The output length is the longest input; missing tail frames and missing
    /// channels are silence.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty, sample rates or
    /// sample formats are incompatible, or the mixed buffer cannot be
    /// represented.
    pub fn open_wavs_mixed<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        Self::open_wavs_mixed_with_backend(paths, BackendKind::Scalar)
    }

    /// Opens multiple supported linear PCM WAV files and mixes them with a
    /// requested backend.
    ///
    /// `requested_backend` controls decode conversion, the scalar/SIMD mix
    /// kernel, later backend-aware effects, and output encoding after
    /// [`Self::into_pipeline`]. SIMD requests follow Auralis' deterministic
    /// scalar fallback rules when SIMD is unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty, sample rates or
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
            inputs.push(auralis_wav::decode_wav_path_with_backend(
                path,
                requested_backend,
            )?);
        }

        Self::from_audio_buffers_mixed_with_backend(&inputs, requested_backend)
    }

    /// Opens multiple supported linear PCM WAV files and mixes them with
    /// equal-power balancing.
    ///
    /// This is the library counterpart to `auralis run --combine mix-power`.
    /// Each input is decoded into planar `f32`, scaled by
    /// `1 / sqrt(input_count)`, and summed with corresponding channels before
    /// any later effects are applied. The output length is the longest input;
    /// missing tail frames and missing channels are silence.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty, sample rates or
    /// sample formats are incompatible, or the mixed buffer cannot be
    /// represented.
    pub fn open_wavs_mix_powered<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        Self::open_wavs_mix_powered_with_backend(paths, BackendKind::Scalar)
    }

    /// Opens multiple supported linear PCM WAV files and mixes them with
    /// equal-power balancing and a requested backend.
    ///
    /// `requested_backend` controls decode conversion, the scalar/SIMD mix
    /// kernel, later backend-aware effects, and output encoding after
    /// [`Self::into_pipeline`]. SIMD requests follow Auralis' deterministic
    /// scalar fallback rules when SIMD is unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty, sample rates or
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
            inputs.push(auralis_wav::decode_wav_path_with_backend(
                path,
                requested_backend,
            )?);
        }

        Self::from_audio_buffers_mix_powered_with_backend(&inputs, requested_backend)
    }

    /// Opens multiple supported linear PCM WAV files and merges all input
    /// channels.
    ///
    /// This is the library counterpart to `auralis run --combine merge`. Each
    /// input is decoded into planar `f32`; output channels contain all channels
    /// from the first input, then all channels from each later input. The output
    /// length is the longest input; missing tail frames are silence.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty, sample rates or
    /// sample formats are incompatible, or the merged buffer cannot be
    /// represented.
    pub fn open_wavs_merged<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        Self::open_wavs_merged_with_backend(paths, BackendKind::Scalar)
    }

    /// Opens multiple supported linear PCM WAV files and merges all input
    /// channels with a requested backend.
    ///
    /// `requested_backend` controls decode conversion, later backend-aware
    /// effects, and output encoding after [`Self::into_pipeline`]. Merge itself
    /// is a structural copy after validation and does not select a SIMD kernel.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty, sample rates or
    /// sample formats are incompatible, or the merged buffer cannot be
    /// represented.
    pub fn open_wavs_merged_with_backend<I, P>(
        paths: I,
        requested_backend: BackendKind,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut inputs = Vec::new();
        for path in paths {
            inputs.push(auralis_wav::decode_wav_path_with_backend(
                path,
                requested_backend,
            )?);
        }

        Self::from_audio_buffers_merged_with_backend(&inputs, requested_backend)
    }

    /// Opens multiple supported linear PCM WAV files and multiplies
    /// corresponding samples.
    ///
    /// This is the library counterpart to `auralis run --combine multiply`.
    /// Each input is decoded into planar `f32`; output samples are the product
    /// of corresponding input channels and frames. The output length is the
    /// longest input and the output channel count is the largest input channel
    /// count. Missing tail frames and missing channels are silence.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty, sample rates or
    /// sample formats are incompatible, or the multiplied buffer cannot be
    /// represented.
    pub fn open_wavs_multiplied<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        Self::open_wavs_multiplied_with_backend(paths, BackendKind::Scalar)
    }

    /// Opens multiple supported linear PCM WAV files and multiplies
    /// corresponding samples with a requested backend.
    ///
    /// `requested_backend` controls decode conversion, the scalar/SIMD multiply
    /// kernel, later backend-aware effects, and output encoding after
    /// [`Self::into_pipeline`]. SIMD requests follow Auralis' deterministic
    /// scalar fallback rules when SIMD is unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Wav`] for decode failures or
    /// [`crate::Error::InputCombine`] when the input list is empty, sample rates or
    /// sample formats are incompatible, or the multiplied buffer cannot be
    /// represented.
    pub fn open_wavs_multiplied_with_backend<I, P>(
        paths: I,
        requested_backend: BackendKind,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut inputs = Vec::new();
        for path in paths {
            inputs.push(auralis_wav::decode_wav_path_with_backend(
                path,
                requested_backend,
            )?);
        }

        Self::from_audio_buffers_multiplied_with_backend(&inputs, requested_backend)
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
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, stream
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
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, stream
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
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, a sequence
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
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, a sequence
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
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, sample
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
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, sample
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
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, sample
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
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, sample
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

    /// Merges existing audio buffers into the high-level file type.
    ///
    /// Inputs are combined before the returned value enters the effect
    /// pipeline. Output channels contain every channel from each input in
    /// caller order. The output length is the longest input; shorter inputs are
    /// padded with silent tail frames.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, sample
    /// rates or sample formats are incompatible, or the merged buffer cannot be
    /// represented.
    pub fn from_audio_buffers_merged(inputs: &[AudioBuffer]) -> Result<Self> {
        Self::from_audio_buffers_merged_with_backend(inputs, BackendKind::Scalar)
    }

    /// Merges existing audio buffers with a requested processing backend.
    ///
    /// The backend is recorded for later pipeline stages. Merge itself is a
    /// deterministic structural copy and does not select a SIMD kernel.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, sample
    /// rates or sample formats are incompatible, or the merged buffer cannot be
    /// represented.
    pub fn from_audio_buffers_merged_with_backend(
        inputs: &[AudioBuffer],
        requested_backend: BackendKind,
    ) -> Result<Self> {
        Ok(Self {
            audio: merge_audio_buffers(inputs)?,
            requested_backend,
        })
    }

    /// Multiplies existing audio buffers into the high-level file type.
    ///
    /// Inputs are combined before the returned value enters the effect
    /// pipeline. Output samples are the product of corresponding input channels
    /// and frames. The output length is the longest input and the output
    /// channel count is the largest input channel count. Missing tail frames
    /// and missing channels are silence.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, sample
    /// rates or sample formats are incompatible, or the multiplied buffer
    /// cannot be represented.
    pub fn from_audio_buffers_multiplied(inputs: &[AudioBuffer]) -> Result<Self> {
        Self::from_audio_buffers_multiplied_with_backend(inputs, BackendKind::Scalar)
    }

    /// Multiplies existing audio buffers with a requested processing backend.
    ///
    /// The backend is recorded for later pipeline stages and selects the
    /// scalar/SIMD multiply kernel for this combiner.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::InputCombine`] when the input list is empty, sample
    /// rates or sample formats are incompatible, or the multiplied buffer
    /// cannot be represented.
    pub fn from_audio_buffers_multiplied_with_backend(
        inputs: &[AudioBuffer],
        requested_backend: BackendKind,
    ) -> Result<Self> {
        Ok(Self {
            audio: multiply_audio_buffers_with_backend(inputs, requested_backend)?,
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
