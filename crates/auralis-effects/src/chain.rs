//! In-memory sequential effect chains.
//!
//! A chain is an ordered list of parsed, typed [`EffectCommand`] values. It
//! can also preserve explicit SoX-ng-style `:` chain boundaries. Current
//! Auralis processing executes the commands sequentially in memory; boundaries
//! are retained for diagnostics, deterministic rendering, and later
//! multi-chain semantics.
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

use crate::{
    EffectCommand, EffectCommandParseError, EffectError, EffectNameError, EffectRegistry,
    chain_dispatch::{apply_command, command_end},
    chain_gain::GainHeadroomState,
    parse_effect_command,
};

pub(crate) const CHAIN_BOUNDARY_TOKEN: &str = ":";

const UNSUPPORTED_BOUNDARY_CONTROLS: &[&str] = &["newfile", "restart"];

/// Crate-local result type for effect-chain processing.
pub type ChainResult<T> = std::result::Result<T, EffectChainError>;

/// Crate-local result type for tokenized effect-chain parsing.
pub type ChainParseResult<T> = std::result::Result<T, EffectChainParseError>;

/// An ordered in-memory sequence of typed effect commands.
///
/// `EffectChain` owns validated command values and applies them exactly in the
/// order supplied by the caller. It does not hide defaults; callers can use
/// [`parse_effect_chain`] before constructing the chain when they start from a
/// flat SoX-ng-style token stream.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectChain {
    commands: Vec<EffectCommand>,
    boundaries: Vec<EffectChainBoundary>,
}

/// A preserved explicit boundary between effect-chain segments.
///
/// A boundary's `before_command` value is the index of the command that starts
/// the next segment. For example, parsing `gain -3 : reverse` produces one
/// boundary with `before_command == 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectChainBoundary {
    before_command: usize,
}

impl EffectChainBoundary {
    /// Creates a boundary before the command at `before_command`.
    ///
    /// The parser only emits boundaries between existing commands, never before
    /// the first command or after the final command.
    #[must_use]
    pub const fn new(before_command: usize) -> Self {
        Self { before_command }
    }

    /// Returns the command index that starts the segment after this boundary.
    #[must_use]
    pub const fn before_command(self) -> usize {
        self.before_command
    }
}

impl EffectChain {
    /// Creates a chain from commands in execution order.
    #[must_use]
    pub fn new(commands: Vec<EffectCommand>) -> Self {
        Self {
            commands,
            boundaries: Vec::new(),
        }
    }

    /// Creates an empty chain.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            commands: Vec::new(),
            boundaries: Vec::new(),
        }
    }

    /// Returns the commands in execution order.
    #[must_use]
    pub fn commands(&self) -> &[EffectCommand] {
        &self.commands
    }

    /// Returns explicit chain boundaries in parse/render order.
    ///
    /// Boundaries are represented by the command index that starts the next
    /// segment. They are preserved for deterministic diagnostics and rendering;
    /// current processing still applies all commands sequentially.
    #[must_use]
    pub fn boundaries(&self) -> &[EffectChainBoundary] {
        &self.boundaries
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

    /// Returns true when the parsed chain contained one or more explicit `:`
    /// boundaries.
    #[must_use]
    pub fn has_boundaries(&self) -> bool {
        !self.boundaries.is_empty()
    }

    /// Renders this chain as canonical SoX-ng-style tokens.
    ///
    /// Effect commands are rendered through [`EffectCommand::render_tokens`].
    /// Preserved boundaries render as a single `:` token between command
    /// segments. The returned vector is process-safe and intentionally
    /// unquoted.
    #[must_use]
    pub fn render_tokens(&self) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut boundary_index = 0;

        for (command_index, command) in self.commands.iter().enumerate() {
            while self
                .boundaries
                .get(boundary_index)
                .is_some_and(|boundary| boundary.before_command == command_index)
            {
                tokens.push(CHAIN_BOUNDARY_TOKEN.to_owned());
                boundary_index += 1;
            }
            tokens.extend(command.render_tokens());
        }

        tokens
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
        let mut gain_headroom = GainHeadroomState::default();
        for (index, command) in self.commands.iter().enumerate() {
            apply_command(command, audio, requested_backend, &mut gain_headroom).map_err(
                |(argument, source)| EffectChainError::CommandFailed {
                    index,
                    command: command.clone(),
                    argument,
                    source,
                },
            )?;
        }

        Ok(())
    }

    pub(crate) fn from_parts(
        commands: Vec<EffectCommand>,
        boundaries: Vec<EffectChainBoundary>,
    ) -> Self {
        Self {
            commands,
            boundaries,
        }
    }
}

/// Parses a flat sequence of SoX-ng-style effect tokens into an [`EffectChain`].
///
/// The token stream is segmented by effect names and aliases, so
/// `["gain", "-3", "reverse"]` becomes two commands while
/// `["gain", "-n"]` remains a single failing command that reports the
/// unsupported gain option. Explicit `:` boundaries are preserved for
/// deterministic rendering. The resulting chain stores typed commands and
/// applies them in the same order as the input tokens.
///
/// # Errors
///
/// Returns [`EffectChainParseError::CommandParseFailed`] when an effect name is
/// unknown or unsupported, an argument is missing or invalid, or a command uses
/// an option outside the currently implemented Auralis subset. Returns
/// [`EffectChainParseError::EmptyBoundaryChain`] for leading, repeated, or
/// trailing `:` boundaries, and
/// [`EffectChainParseError::UnsupportedBoundaryControl`] for `newfile` and
/// `restart` until those SoX-ng semantics are implemented.
///
/// # Examples
///
/// ```
/// use auralis_effects::parse_effect_chain;
///
/// let chain = parse_effect_chain(&["gain", "-3", "reverse"])?;
///
/// assert_eq!(chain.len(), 2);
/// assert_eq!(chain.commands()[0].render_tokens(), ["gain", "-3"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_effect_chain(tokens: &[&str]) -> ChainParseResult<EffectChain> {
    let mut commands = Vec::new();
    let mut boundaries = Vec::new();
    let mut offset = 0;
    let mut pending_boundary_token = None;

    while offset < tokens.len() {
        let name = tokens[offset];
        if is_chain_boundary_token(name) {
            if commands.is_empty() || pending_boundary_token.is_some() {
                return Err(EffectChainParseError::EmptyBoundaryChain {
                    token_index: offset,
                });
            }
            boundaries.push(EffectChainBoundary::new(commands.len()));
            pending_boundary_token = Some(offset);
            offset += 1;
            continue;
        }
        if is_unsupported_boundary_control(name) {
            return Err(EffectChainParseError::UnsupportedBoundaryControl {
                token_index: offset,
                control: name.to_owned(),
            });
        }

        let index = commands.len();

        let descriptor = EffectRegistry::resolve(name).map_err(|source| {
            EffectChainParseError::CommandParseFailed {
                index,
                command: name.to_owned(),
                source: source.into(),
            }
        })?;
        let end = command_end(descriptor.kind(), tokens, offset);
        let command_tokens = &tokens[offset..end];
        let command = parse_effect_command(command_tokens).map_err(|source| {
            EffectChainParseError::CommandParseFailed {
                index,
                command: command_tokens.join(" "),
                source,
            }
        })?;

        commands.push(command);
        pending_boundary_token = None;
        offset = end;
    }

    if let Some(token_index) = pending_boundary_token {
        return Err(EffectChainParseError::EmptyBoundaryChain { token_index });
    }

    Ok(EffectChain::from_parts(commands, boundaries))
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

/// Errors produced while parsing a tokenized [`EffectChain`].
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum EffectChainParseError {
    /// One command in the chain could not be parsed into a typed effect command.
    #[error("effect chain command {index} (`{command}`) failed to parse: {source}")]
    CommandParseFailed {
        /// Zero-based command index.
        index: usize,

        /// Raw command tokens that were being parsed.
        command: String,

        /// Typed command parser source error.
        #[source]
        source: EffectCommandParseError,
    },

    /// A `:` boundary would create an empty chain segment.
    #[error("effect chain boundary at token {token_index} creates an empty chain segment")]
    EmptyBoundaryChain {
        /// Zero-based token index of the invalid boundary.
        token_index: usize,
    },

    /// A SoX-ng boundary control token is known but not implemented yet.
    #[error(
        "effect chain token {token_index} (`{control}`) uses unsupported boundary control; `newfile` and `restart` semantics are not implemented"
    )]
    UnsupportedBoundaryControl {
        /// Zero-based token index of the unsupported control.
        token_index: usize,

        /// Unsupported boundary control token.
        control: String,
    },
}

pub(crate) fn is_chain_boundary_token(token: &str) -> bool {
    token == CHAIN_BOUNDARY_TOKEN
}

pub(crate) fn is_unsupported_boundary_control(token: &str) -> bool {
    UNSUPPORTED_BOUNDARY_CONTROLS.contains(&token)
}

pub(crate) fn is_effect_boundary(token: &str) -> bool {
    match EffectRegistry::resolve(token) {
        Ok(_) | Err(EffectNameError::UnsupportedSoxNgEffect { .. }) => true,
        Err(EffectNameError::EmptyName | EffectNameError::UnknownEffect { .. }) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EffectChain, EffectChainBoundary, EffectChainError, EffectChainParseError,
        parse_effect_chain,
    };
    use crate::{
        BiquadState, DcShift, EffectCommand, EffectError, Fade, Gain, Pad, Reverse, SoftVol, Trim,
    };
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
    fn parses_flat_tokens_into_user_ordered_effect_chain() {
        let chain = parse_effect_chain(&[
            "gain", "-3", "dcshift", "0.125", "fade", "t", "2", "reverse",
        ])
        .unwrap();

        let rendered: Vec<Vec<String>> = chain
            .commands()
            .iter()
            .map(EffectCommand::render_tokens)
            .collect();
        assert_eq!(
            rendered,
            vec![
                vec!["gain", "-3"],
                vec!["dcshift", "0.125"],
                vec!["fade", "t", "2"],
                vec!["reverse"],
            ]
        );
    }

    #[test]
    fn chain_token_parser_preserves_boundaries_for_deterministic_rendering() {
        let chain =
            parse_effect_chain(&["gain", "-3", ":", "dcshift", "0.125", "reverse"]).unwrap();

        assert!(chain.has_boundaries());
        assert_eq!(chain.boundaries(), &[EffectChainBoundary::new(1)]);
        assert_eq!(chain.boundaries()[0].before_command(), 1);
        assert_eq!(
            chain.render_tokens(),
            ["gain", "-3", ":", "dcshift", "0.125", "reverse"]
        );
    }

    #[test]
    fn boundary_chain_processes_like_equivalent_sequential_chain_for_now() {
        let source = audio_buffer(vec![0.25, -0.5, 1.0]);
        let mut boundary = source.clone();
        let mut flat = source;
        let boundary_chain =
            parse_effect_chain(&["gain", "-3", ":", "dcshift", "0.125", "reverse"]).unwrap();
        let flat_chain =
            parse_effect_chain(&["gain", "-3", "dcshift", "0.125", "reverse"]).unwrap();

        boundary_chain.process_buffer(&mut boundary).unwrap();
        flat_chain.process_buffer(&mut flat).unwrap();

        assert_eq!(boundary_chain.render_tokens()[2], ":");
        assert_samples_close(boundary.as_planar_f32(), flat.as_planar_f32());
    }

    #[test]
    fn chain_token_parser_preserves_defaults_and_alias_boundaries() {
        let chain = parse_effect_chain(&["gain", "dc-shift", "-0.25", "pad", "reverse"]).unwrap();

        let rendered: Vec<Vec<String>> = chain
            .commands()
            .iter()
            .map(EffectCommand::render_tokens)
            .collect();
        assert_eq!(
            rendered,
            vec![
                vec!["gain", "0"],
                vec!["dcshift", "-0.25"],
                vec!["pad", "0", "0"],
                vec!["reverse"],
            ]
        );
    }

    #[test]
    fn chain_token_parser_handles_softvol_defaults_and_arguments() {
        let chain = parse_effect_chain(&["softvol", "2", "10", "0.1", "reverse"]).unwrap();

        assert_eq!(
            chain.commands()[0],
            EffectCommand::SoftVol(SoftVol::new(2.0, 10.0, 0.1).unwrap())
        );
        assert_eq!(
            chain.render_tokens(),
            ["softvol", "2", "10", "0.1", "reverse"]
        );
    }

    #[test]
    fn chain_token_parser_reports_failing_command_and_argument() {
        let error = parse_effect_chain(&["gain", "-3", "trim", "reverse"]).unwrap_err();

        assert_eq!(
            error,
            EffectChainParseError::CommandParseFailed {
                index: 1,
                command: "trim".to_owned(),
                source: crate::EffectCommandParseError::MissingArgument {
                    effect: "trim",
                    argument: "position",
                },
            }
        );
        assert_eq!(
            error.to_string(),
            "effect chain command 1 (`trim`) failed to parse: effect `trim` requires argument `position`"
        );
    }

    #[test]
    fn chain_token_parser_rejects_empty_boundary_segments() {
        let leading = parse_effect_chain(&[":", "gain", "-3"]).unwrap_err();
        let repeated = parse_effect_chain(&["gain", "-3", ":", ":", "reverse"]).unwrap_err();
        let trailing = parse_effect_chain(&["gain", "-3", ":"]).unwrap_err();

        assert_eq!(
            leading,
            EffectChainParseError::EmptyBoundaryChain { token_index: 0 }
        );
        assert_eq!(
            repeated,
            EffectChainParseError::EmptyBoundaryChain { token_index: 3 }
        );
        assert_eq!(
            trailing,
            EffectChainParseError::EmptyBoundaryChain { token_index: 2 }
        );
        assert_eq!(
            trailing.to_string(),
            "effect chain boundary at token 2 creates an empty chain segment"
        );
    }

    #[test]
    fn unsupported_newfile_and_restart_report_stable_boundary_diagnostics() {
        let newfile = parse_effect_chain(&["gain", "-3", ":", "newfile"]).unwrap_err();
        let restart = parse_effect_chain(&["gain", "-3", "restart"]).unwrap_err();

        assert_eq!(
            newfile,
            EffectChainParseError::UnsupportedBoundaryControl {
                token_index: 3,
                control: "newfile".to_owned(),
            }
        );
        assert_eq!(
            restart,
            EffectChainParseError::UnsupportedBoundaryControl {
                token_index: 2,
                control: "restart".to_owned(),
            }
        );
        assert_eq!(
            newfile.to_string(),
            "effect chain token 3 (`newfile`) uses unsupported boundary control; `newfile` and `restart` semantics are not implemented"
        );
    }

    #[test]
    fn chain_token_parser_keeps_option_like_values_with_current_command() {
        let error = parse_effect_chain(&["gain", "-q", "reverse"]).unwrap_err();

        assert_eq!(
            error,
            EffectChainParseError::CommandParseFailed {
                index: 0,
                command: "gain -q".to_owned(),
                source: crate::EffectCommandParseError::UnsupportedOption {
                    effect: "gain",
                    option: "-q".to_owned(),
                },
            }
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
        for command in chain.commands() {
            match command {
                EffectCommand::Biquad(biquad) => {
                    let mut state = BiquadState::new(biquad.coefficients());
                    for chunk in chunks_mut(samples, chunk_sizes) {
                        state.process_mono_samples(chunk);
                    }
                }
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
                EffectCommand::Vol(vol) => {
                    for chunk in chunks_mut(samples, chunk_sizes) {
                        vol.process_samples(chunk);
                    }
                }
                EffectCommand::AllPass(_)
                | EffectCommand::Band(_)
                | EffectCommand::BandPass(_)
                | EffectCommand::BandReject(_)
                | EffectCommand::Centercut(_)
                | EffectCommand::Channels(_)
                | EffectCommand::Contrast(_)
                | EffectCommand::Norm(_)
                | EffectCommand::Oops(_)
                | EffectCommand::Overdrive(_)
                | EffectCommand::Pad(_)
                | EffectCommand::Repeat(_)
                | EffectCommand::Remix(_)
                | EffectCommand::Reverse(_)
                | EffectCommand::Saturation(_)
                | EffectCommand::SoftVol(_)
                | EffectCommand::Swap(_)
                | EffectCommand::Tremolo(_)
                | EffectCommand::Trim(_) => {
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
