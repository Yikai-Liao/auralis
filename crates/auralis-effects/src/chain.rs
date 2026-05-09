//! In-memory sequential effect chains.
//!
//! A chain is an ordered list of parsed, typed [`EffectCommand`] values. It
//! processes an [`AudioBuffer`] in user order and reports processing failures
//! with the command index and canonical command tokens that failed.
//!
//! # Examples
//!
//! ```
//! use auralis_core::{
//!     AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
//! };
//! use auralis_effects::{EffectChain, EffectCommand, Gain, Reverse};
//!
//! let spec = AudioSpec::new(
//!     SampleRate::new(48_000)?,
//!     ChannelCount::new(1)?,
//!     SampleFormat::Float32,
//! );
//! let mut audio = AudioBuffer::from_planar_f32(
//!     spec,
//!     FrameCount::new(3),
//!     vec![0.25, -0.5, 1.0],
//! )?;
//! let chain = EffectChain::new(vec![
//!     EffectCommand::Gain(Gain::new(Decibels::new(6.0)?)),
//!     EffectCommand::Reverse(Reverse::new()),
//! ]);
//!
//! chain.process_buffer(&mut audio)?;
//!
//! assert!(audio.as_planar_f32()[0] > 1.99);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use auralis_core::AudioBuffer;
use auralis_simd::BackendKind;
use thiserror::Error;

use crate::{EffectCommand, EffectError};

/// Crate-local result type for effect-chain processing.
pub type ChainResult<T> = std::result::Result<T, EffectChainError>;

/// An ordered in-memory sequence of typed effect commands.
///
/// `EffectChain` owns validated command values and applies them exactly in the
/// order supplied by the caller. It does not parse CLI syntax and does not hide
/// defaults; callers can use [`crate::parse_effect_command`] before constructing
/// the chain when they start from SoX-ng-style tokens.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectChain {
    commands: Vec<EffectCommand>,
}

impl EffectChain {
    /// Creates a chain from commands in execution order.
    #[must_use]
    pub fn new(commands: Vec<EffectCommand>) -> Self {
        Self { commands }
    }

    /// Creates an empty chain.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Returns the commands in execution order.
    #[must_use]
    pub fn commands(&self) -> &[EffectCommand] {
        &self.commands
    }

    /// Returns the number of commands in the chain.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns true when the chain contains no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Applies every command to `audio` using the scalar backend.
    ///
    /// Commands run sequentially in the order returned by [`Self::commands`].
    /// If a command fails, processing stops and the returned
    /// [`EffectChainError`] identifies the failing command index, command
    /// tokens, affected argument family, and typed source error.
    ///
    /// # Errors
    ///
    /// Returns [`EffectChainError::CommandFailed`] when a command such as
    /// `trim` or `pad` rejects the current buffer shape.
    pub fn process_buffer(&self, audio: &mut AudioBuffer) -> ChainResult<()> {
        self.process_buffer_with_backend(audio, BackendKind::Scalar)
    }

    /// Applies every command to `audio` using the requested backend.
    ///
    /// Backend-aware commands (`gain`, `dcshift`, and linear `fade`) use
    /// Auralis' deterministic backend selection. Other commands are structural
    /// buffer transforms and have no SIMD kernel to select.
    ///
    /// # Errors
    ///
    /// Returns [`EffectChainError::CommandFailed`] when a command rejects the
    /// current buffer shape.
    pub fn process_buffer_with_backend(
        &self,
        audio: &mut AudioBuffer,
        requested_backend: BackendKind,
    ) -> ChainResult<()> {
        for (index, &command) in self.commands.iter().enumerate() {
            apply_command(command, audio, requested_backend).map_err(|(argument, source)| {
                EffectChainError::CommandFailed {
                    index,
                    command,
                    argument,
                    source,
                }
            })?;
        }

        Ok(())
    }
}

/// Errors produced while applying an [`EffectChain`].
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum EffectChainError {
    /// One command in the chain rejected the current buffer.
    #[error(
        "effect chain command {index} (`{command}`) failed while applying `{argument}`: {source}"
    )]
    CommandFailed {
        /// Zero-based command index.
        index: usize,

        /// Canonical typed command that failed.
        command: EffectCommand,

        /// Argument family or option family that failed.
        argument: &'static str,

        /// Typed effect source error.
        #[source]
        source: EffectError,
    },
}

fn apply_command(
    command: EffectCommand,
    audio: &mut AudioBuffer,
    requested_backend: BackendKind,
) -> std::result::Result<(), (&'static str, EffectError)> {
    match command {
        EffectCommand::DcShift(dc_shift) => {
            dc_shift.process_buffer_with_backend(audio, requested_backend);
            Ok(())
        }
        EffectCommand::Fade(fade) => {
            fade.process_buffer_with_backend(audio, requested_backend);
            Ok(())
        }
        EffectCommand::Gain(gain) => {
            gain.process_buffer_with_backend(audio, requested_backend);
            Ok(())
        }
        EffectCommand::Pad(pad) => {
            let padded = pad
                .process_buffer(audio)
                .map_err(|source| ("frame-count", source))?;
            *audio = padded;
            Ok(())
        }
        EffectCommand::Reverse(reverse) => {
            reverse.process_buffer(audio);
            Ok(())
        }
        EffectCommand::Trim(trim) => {
            let trimmed = trim
                .process_buffer(audio)
                .map_err(|source| ("frame-range", source))?;
            *audio = trimmed;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EffectChain, EffectChainError};
    use crate::{DcShift, EffectCommand, EffectError, Fade, Gain, Pad, Reverse, Trim};
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
    };
    use auralis_simd::BackendKind;

    #[test]
    fn multiple_effects_execute_in_user_order() {
        let mut audio = audio_buffer(vec![0.25, -0.5, 1.0]);
        let chain = EffectChain::new(vec![
            EffectCommand::Gain(Gain::new(db(6.0))),
            EffectCommand::DcShift(DcShift::new(-0.25).unwrap()),
            EffectCommand::Reverse(Reverse::new()),
        ]);

        chain.process_buffer(&mut audio).unwrap();

        let multiplier = 10.0_f32.powf(6.0 / 20.0);
        assert_samples_close(
            audio.as_planar_f32(),
            &[
                1.0 * multiplier - 0.25,
                -0.5 * multiplier - 0.25,
                0.25 * multiplier - 0.25,
            ],
        );
    }

    #[test]
    fn chain_output_matches_repeated_direct_library_calls() {
        let source = stereo_audio_buffer(vec![0.25, -0.5, 0.75, 1.0, -0.25, 0.5, -0.75, -1.0]);
        let mut chained = source.clone();
        let mut direct = source;
        let chain = EffectChain::new(vec![
            EffectCommand::Gain(Gain::new(db(-3.0))),
            EffectCommand::DcShift(DcShift::new(0.125).unwrap()),
            EffectCommand::Fade(Fade::new(FrameCount::new(2), FrameCount::new(2))),
            EffectCommand::Trim(Trim::new(FrameCount::new(1), FrameCount::new(3)).unwrap()),
            EffectCommand::Pad(Pad::new(FrameCount::new(1), FrameCount::new(0))),
            EffectCommand::Reverse(Reverse::new()),
        ]);

        chain.process_buffer(&mut chained).unwrap();
        Gain::new(db(-3.0)).process_buffer(&mut direct);
        DcShift::new(0.125).unwrap().process_buffer(&mut direct);
        Fade::new(FrameCount::new(2), FrameCount::new(2)).process_buffer(&mut direct);
        direct = Trim::new(FrameCount::new(1), FrameCount::new(3))
            .unwrap()
            .process_buffer(&direct)
            .unwrap();
        direct = Pad::new(FrameCount::new(1), FrameCount::new(0))
            .process_buffer(&direct)
            .unwrap();
        Reverse::new().process_buffer(&mut direct);

        assert_samples_close(chained.as_planar_f32(), direct.as_planar_f32());
        assert_eq!(chained.frames(), direct.frames());
        assert_eq!(chained.channels(), direct.channels());
    }

    #[test]
    fn processing_failure_reports_index_command_argument_and_source() {
        let mut audio = audio_buffer(vec![0.25, -0.5]);
        let chain = EffectChain::new(vec![
            EffectCommand::Gain(Gain::new(db(0.0))),
            EffectCommand::Trim(Trim::new(FrameCount::new(0), FrameCount::new(3)).unwrap()),
        ]);

        let error = chain.process_buffer(&mut audio).unwrap_err();

        assert_eq!(
            error,
            EffectChainError::CommandFailed {
                index: 1,
                command: EffectCommand::Trim(
                    Trim::new(FrameCount::new(0), FrameCount::new(3)).unwrap()
                ),
                argument: "frame-range",
                source: EffectError::TrimRangeOutOfBounds,
            }
        );
        assert_eq!(
            error.to_string(),
            "effect chain command 1 (`trim 0 3`) failed while applying `frame-range`: trim frame range must be within the input duration"
        );
    }

    #[test]
    fn chunked_chain_matches_whole_chain_for_streaming_safe_commands() {
        let source = vec![-1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0];
        let mut whole = audio_buffer(source.clone());
        let mut chunked = source;
        let chain = EffectChain::new(vec![
            EffectCommand::Gain(Gain::new(db(-6.0))),
            EffectCommand::DcShift(DcShift::new(0.125).unwrap()),
            EffectCommand::Fade(Fade::new(FrameCount::new(4), FrameCount::new(4))),
        ]);

        chain.process_buffer(&mut whole).unwrap();
        apply_streaming_safe_chain_by_chunks(&chain, &mut chunked, 9, &[3, 1, 0, 4, 1]);

        assert_samples_close(whole.as_planar_f32(), &chunked);
    }

    #[test]
    fn full_chain_matches_under_forced_scalar_and_requested_simd() {
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
            EffectCommand::Gain(Gain::new(db(-3.0))),
            EffectCommand::DcShift(DcShift::new(0.125).unwrap()),
            EffectCommand::Fade(Fade::new(FrameCount::new(5), FrameCount::new(7))),
            EffectCommand::Reverse(Reverse::new()),
        ]);
        let mut scalar = source.clone();
        let mut simd = source;

        chain
            .process_buffer_with_backend(&mut scalar, BackendKind::Scalar)
            .unwrap();
        chain
            .process_buffer_with_backend(&mut simd, BackendKind::Simd)
            .unwrap();

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn empty_chain_is_identity() {
        let mut audio = audio_buffer(vec![0.25, -0.5]);
        let expected = audio.clone();
        let chain = EffectChain::empty();

        chain.process_buffer(&mut audio).unwrap();

        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
        assert_eq!(chain.commands(), &[]);
        assert_eq!(audio, expected);
    }

    fn apply_streaming_safe_chain_by_chunks(
        chain: &EffectChain,
        samples: &mut [f32],
        total_frames: u64,
        chunk_sizes: &[usize],
    ) {
        for &command in chain.commands() {
            match command {
                EffectCommand::Gain(gain) => {
                    for chunk in chunks_mut(samples, chunk_sizes) {
                        gain.process_samples(chunk);
                    }
                }
                EffectCommand::DcShift(dc_shift) => {
                    for chunk in chunks_mut(samples, chunk_sizes) {
                        dc_shift.process_samples(chunk);
                    }
                }
                EffectCommand::Fade(fade) => {
                    let mut start = 0_u64;
                    for chunk in chunks_mut(samples, chunk_sizes) {
                        fade.process_channel_segment(chunk, total_frames, FrameCount::new(start));
                        start += u64::try_from(chunk.len()).unwrap();
                    }
                }
                EffectCommand::Pad(_) | EffectCommand::Reverse(_) | EffectCommand::Trim(_) => {
                    panic!("test helper only supports streaming-safe commands")
                }
            }
        }
    }

    fn chunks_mut<'samples>(
        samples: &'samples mut [f32],
        chunk_sizes: &'samples [usize],
    ) -> Vec<&'samples mut [f32]> {
        let mut chunks = Vec::new();
        let mut remaining = samples;
        for &size in chunk_sizes {
            if remaining.is_empty() {
                break;
            }
            let size = size.min(remaining.len());
            let (chunk, rest) = remaining.split_at_mut(size);
            chunks.push(chunk);
            remaining = rest;
        }
        if !remaining.is_empty() {
            chunks.push(remaining);
        }

        chunks
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
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(2).unwrap(),
            SampleFormat::Float32,
        );

        AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
    }

    fn db(value: f64) -> Decibels {
        Decibels::new(value).unwrap()
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());

        for (actual, expected) in actual.iter().zip(expected) {
            let tolerance = 1.0e-5;
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
}
