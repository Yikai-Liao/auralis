use std::{fs::File, path::Path};

use auralis_codec::{
    AudioEncoder, CodecKind, EncodeSummary, OutputFormat, UnsupportedEncoder, WavContainer,
    WavEncodeOptions, WavSampleFormat,
};
use auralis_effects::{DcShift, Fade, Gain, Pad, Reverse, Trim};

use crate::channel_policy::apply_channel_conversion_policy_with_backend;
use crate::dither_policy::apply_output_dither_policy;
use crate::level_policy::apply_output_level_policy_with_backend;
use crate::rate_policy::apply_sample_rate_conversion_policy;
use crate::{
    AudioBuffer, BackendKind, ChannelConversionPolicy, ChannelCount, Decibels, EffectChain, Error,
    FrameCount, OutputDitherConfig, OutputDitherPolicy, OutputLevelPolicy, Result, SampleRate,
    SampleRateConversionPolicy, TimeSeconds,
};

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
    sample_rate_conversion_policy: SampleRateConversionPolicy,
    channel_conversion_policy: ChannelConversionPolicy,
    output_level_policy: OutputLevelPolicy,
    output_dither_policy: OutputDitherPolicy,
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
            sample_rate_conversion_policy: SampleRateConversionPolicy::Preserve,
            channel_conversion_policy: ChannelConversionPolicy::Preserve,
            output_level_policy: OutputLevelPolicy::Preserve,
            output_dither_policy: OutputDitherPolicy::Disabled,
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

    /// Sets the explicit output sample-rate conversion policy for later writes.
    ///
    /// The default policy is [`SampleRateConversionPolicy::Preserve`], which
    /// means `write_wav` uses the pipeline's current sample rate. Use
    /// [`SampleRateConversionPolicy::Automatic`] to request deterministic
    /// output-boundary resampling, or [`SampleRateConversionPolicy::Require`]
    /// to verify a target sample rate while failing instead of converting when
    /// it differs.
    #[must_use]
    pub const fn with_sample_rate_conversion_policy(
        mut self,
        policy: SampleRateConversionPolicy,
    ) -> Self {
        self.sample_rate_conversion_policy = policy;
        self
    }

    /// Requests deterministic automatic sample-rate conversion before writing.
    ///
    /// This is a convenience wrapper around
    /// [`Self::with_sample_rate_conversion_policy`] using
    /// [`SampleRateConversionPolicy::Automatic`].
    #[must_use]
    pub const fn with_output_sample_rate(mut self, sample_rate: SampleRate) -> Self {
        self.sample_rate_conversion_policy = SampleRateConversionPolicy::Automatic(sample_rate);
        self
    }

    /// Sets the explicit output channel-conversion policy for later writes.
    ///
    /// The default policy is [`ChannelConversionPolicy::Preserve`], which means
    /// `write_wav` uses the pipeline's current channel count. Use
    /// [`ChannelConversionPolicy::Automatic`] to request SoX-ng-style automatic
    /// `channels` conversion at the output boundary, or
    /// [`ChannelConversionPolicy::Require`] to verify a target channel count
    /// while failing instead of converting when it differs.
    #[must_use]
    pub const fn with_channel_conversion_policy(mut self, policy: ChannelConversionPolicy) -> Self {
        self.channel_conversion_policy = policy;
        self
    }

    /// Requests SoX-ng-style automatic channel conversion before writing.
    ///
    /// This is a convenience wrapper around
    /// [`Self::with_channel_conversion_policy`] using
    /// [`ChannelConversionPolicy::Automatic`].
    #[must_use]
    pub const fn with_output_channels(mut self, channels: ChannelCount) -> Self {
        self.channel_conversion_policy = ChannelConversionPolicy::Automatic(channels);
        self
    }

    /// Sets the explicit output level policy for later writes.
    ///
    /// The default policy is [`OutputLevelPolicy::Preserve`], which means
    /// `write_wav` leaves current sample levels unchanged and relies on the
    /// PCM16 encoder's documented clipping. Use [`OutputLevelPolicy::Guard`]
    /// to attenuate only when the final buffer exceeds full scale, or
    /// [`OutputLevelPolicy::Normalize`] to scale non-silent output to a target
    /// peak level.
    #[must_use]
    pub const fn with_output_level_policy(mut self, policy: OutputLevelPolicy) -> Self {
        self.output_level_policy = policy;
        self
    }

    /// Requests full-scale clipping guard before writing.
    ///
    /// This is a convenience wrapper around [`Self::with_output_level_policy`]
    /// using [`OutputLevelPolicy::Guard`].
    #[must_use]
    pub const fn with_output_guard(mut self) -> Self {
        self.output_level_policy = OutputLevelPolicy::Guard;
        self
    }

    /// Requests peak normalization before writing.
    ///
    /// This is a convenience wrapper around [`Self::with_output_level_policy`]
    /// using [`OutputLevelPolicy::Normalize`].
    #[must_use]
    pub const fn with_output_normalization(mut self, target: Decibels) -> Self {
        self.output_level_policy = OutputLevelPolicy::Normalize(target);
        self
    }

    /// Sets the explicit output dither policy for later writes.
    ///
    /// The default policy is [`OutputDitherPolicy::Disabled`], which means
    /// `write_wav` performs PCM16 quantization without added dither. Use
    /// [`OutputDitherPolicy::Automatic`] to apply deterministic TPDF dither
    /// after output rate, channel, and level policies and before PCM16
    /// encoding.
    #[must_use]
    pub const fn with_output_dither_policy(mut self, policy: OutputDitherPolicy) -> Self {
        self.output_dither_policy = policy;
        self
    }

    /// Requests deterministic TPDF dither before PCM16 writing.
    ///
    /// This is a convenience wrapper around
    /// [`Self::with_output_dither_policy`] using
    /// [`OutputDitherPolicy::Automatic`].
    #[must_use]
    pub const fn with_output_dither(mut self) -> Self {
        self.output_dither_policy = OutputDitherPolicy::Automatic(OutputDitherConfig::new());
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

    fn finalize_output_audio(self) -> Result<AudioBuffer> {
        let audio = self.audio?;
        let audio = apply_sample_rate_conversion_policy(audio, self.sample_rate_conversion_policy)?;
        let audio = apply_channel_conversion_policy_with_backend(
            audio,
            self.channel_conversion_policy,
            self.requested_backend,
        )?;
        let audio = apply_output_level_policy_with_backend(
            audio,
            self.output_level_policy,
            self.requested_backend,
        )?;

        Ok(apply_output_dither_policy(
            audio,
            self.output_dither_policy,
        )?)
    }

    /// Encodes the processed audio using an explicit output format model.
    ///
    /// WAV currently remains the only implemented encoder. Other planned
    /// formats return typed [`crate::Error::Codec`] unsupported-format
    /// diagnostics until their own roadmap leaves land.
    ///
    /// # Errors
    ///
    /// Returns the first deferred configuration error from the chain, any
    /// output-boundary policy error, or a typed codec dispatch/encode error.
    pub fn write(self, path: impl AsRef<Path>, format: OutputFormat) -> Result<EncodeSummary> {
        let kind = format.codec_kind();
        match format {
            OutputFormat::Wav(options) => self.write_wav_with_options(path, options),
            OutputFormat::RawPcm(_) => self.write_unsupported(path, CodecKind::RawPcm),
            OutputFormat::Aiff(_) => self.write_unsupported(path, CodecKind::Aiff),
            OutputFormat::Flac(_) => self.write_unsupported(path, CodecKind::Flac),
            _ => self.write_unsupported(path, kind),
        }
    }

    fn write_wav_with_options(
        self,
        path: impl AsRef<Path>,
        options: WavEncodeOptions,
    ) -> Result<EncodeSummary> {
        let path = path.as_ref();
        let requested_backend = self.requested_backend;
        let audio = self.finalize_output_audio()?;
        let mut output =
            File::create(path).map_err(|error| auralis_codec::CodecError::EncodeFailed {
                kind: CodecKind::Wav,
                message: error.to_string(),
            })?;
        if options.container() == WavContainer::Rifx {
            let encoder = auralis_wav::RifxWavEncoder::new(options, requested_backend);
            return Ok(encoder.encode(&audio, &mut output)?);
        }
        match options.sample_format() {
            WavSampleFormat::Pcm8 => {
                let encoder = auralis_wav::Pcm8WavEncoder::new(options, requested_backend);
                Ok(encoder.encode(&audio, &mut output)?)
            }
            WavSampleFormat::Pcm16 => {
                let encoder = auralis_wav::Pcm16WavEncoder::new(options, requested_backend);
                Ok(encoder.encode(&audio, &mut output)?)
            }
            WavSampleFormat::Pcm24 => {
                let encoder = auralis_wav::Pcm24WavEncoder::new(options, requested_backend);
                Ok(encoder.encode(&audio, &mut output)?)
            }
            WavSampleFormat::Pcm32 => {
                let encoder = auralis_wav::Pcm32WavEncoder::new(options, requested_backend);
                Ok(encoder.encode(&audio, &mut output)?)
            }
            WavSampleFormat::Float32 => {
                let encoder = auralis_wav::Float32WavEncoder::new(options, requested_backend);
                Ok(encoder.encode(&audio, &mut output)?)
            }
            WavSampleFormat::Float64 => {
                let encoder = auralis_wav::Float64WavEncoder::new(options, requested_backend);
                Ok(encoder.encode(&audio, &mut output)?)
            }
            WavSampleFormat::ULaw => {
                let encoder = auralis_wav::ULawWavEncoder::new(options, requested_backend);
                Ok(encoder.encode(&audio, &mut output)?)
            }
            WavSampleFormat::ALaw => {
                let encoder = auralis_wav::ALawWavEncoder::new(options, requested_backend);
                Ok(encoder.encode(&audio, &mut output)?)
            }
            _ => {
                let encoder = UnsupportedEncoder::new(CodecKind::Wav);
                Ok(encoder.encode(&audio, &mut output)?)
            }
        }
    }

    fn write_unsupported(self, _path: impl AsRef<Path>, kind: CodecKind) -> Result<EncodeSummary> {
        let audio = self.finalize_output_audio()?;
        let encoder = UnsupportedEncoder::new(kind);
        let mut output = std::io::Cursor::new(Vec::new());

        Ok(encoder.encode(&audio, &mut output)?)
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
    /// finalization fails. Explicit output sample-rate and channel-conversion
    /// policies plus explicit output level policy are applied only here and can
    /// also return typed policy errors.
    pub fn write_wav(self, path: impl AsRef<Path>) -> Result<()> {
        let requested_backend = self.requested_backend;
        let audio = self.finalize_output_audio()?;
        auralis_wav::encode_pcm16_path_with_backend(path, &audio, requested_backend)?;

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
