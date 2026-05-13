use std::{fs, path::Path};

use auralis_core::{AudioBuffer, FrameCount};
use auralis_dsp::{
    DftFir as DspDftFir, FirCoefficients as DspFirCoefficients, FirState as DspFirState,
};

use crate::{EffectError, Result};

/// A validated list of SoX-ng-style FIR coefficients.
///
/// Coefficients are stored in command/file order. Empty coefficient lists are
/// allowed because SoX-ng treats an empty coefficient file as a null `fir`
/// effect; the executable processor is added by a later feature.
///
/// # Examples
///
/// ```
/// use auralis_effects::FirCoefficients;
///
/// let coefficients = FirCoefficients::parse_text("0.5 0.25 # trailing comment\n0.125")?;
/// assert_eq!(coefficients.as_slice(), &[0.5, 0.25, 0.125]);
/// # Ok::<(), auralis_effects::EffectError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FirCoefficients {
    inner: DspFirCoefficients,
}

impl FirCoefficients {
    /// Creates FIR coefficients from already parsed numeric values.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirCoefficients`] when any coefficient is
    /// NaN or infinite.
    pub fn new<I>(values: I) -> Result<Self>
    where
        I: IntoIterator<Item = f64>,
    {
        DspFirCoefficients::new(values)
            .map(Self::from)
            .map_err(EffectError::from)
    }

    /// Parses whitespace-separated coefficients from SoX-ng-style text.
    ///
    /// `#` starts a comment that runs to the end of the line. Comments may
    /// appear on their own line or immediately after a coefficient.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirCoefficients`] when any token is not a
    /// finite floating-point coefficient.
    pub fn parse_text(text: &str) -> Result<Self> {
        let mut values = Vec::new();
        let mut token = String::new();
        let mut in_comment = false;

        for character in text.chars() {
            if in_comment {
                if character == '\n' {
                    in_comment = false;
                }
                continue;
            }

            match character {
                '#' => {
                    push_token(&mut values, &mut token)?;
                    in_comment = true;
                }
                value if value.is_whitespace() => {
                    push_token(&mut values, &mut token)?;
                }
                value => token.push(value),
            }
        }

        push_token(&mut values, &mut token)?;
        Self::new(values)
    }

    /// Returns the coefficients in command/file order.
    #[must_use]
    pub fn as_slice(&self) -> &[f64] {
        self.inner.as_slice()
    }

    /// Returns the number of coefficients.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` when no coefficients were supplied.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn into_dsp(self) -> DspFirCoefficients {
        self.inner
    }
}

impl From<DspFirCoefficients> for FirCoefficients {
    fn from(inner: DspFirCoefficients) -> Self {
        Self { inner }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FirBackend {
    Direct,
    Dft,
}

/// The source of coefficients for a SoX-ng-style `fir` command.
#[derive(Debug, Clone, PartialEq)]
pub enum FirCoefficientSource {
    /// Read coefficients from standard input (`fir` or `fir -`).
    Stdin,
    /// Read coefficients from a named coefficient file.
    File(String),
    /// Use coefficients supplied directly on the command line.
    Inline(FirCoefficients),
}

/// Parsed SoX-ng-style FIR effect.
///
/// `fir [coefs-file | coef <coef>]` loads a validated coefficient list and
/// applies a deterministic scalar finite impulse response filter. Like
/// SoX-ng's DFT FIR effect, output length matches input length and the impulse
/// response is aligned by dropping `(coefficient_count - 1) / 2` leading
/// convolution samples.
#[derive(Debug, Clone, PartialEq)]
pub struct Fir {
    source: FirCoefficientSource,
}

impl Fir {
    /// Creates a `fir` input that reads coefficients from standard input.
    #[must_use]
    pub const fn stdin() -> Self {
        Self {
            source: FirCoefficientSource::Stdin,
        }
    }

    /// Creates a `fir` input that reads coefficients from a file path.
    ///
    /// `None` represents standard input. `Some("-")` is canonicalized to
    /// standard input to match SoX-ng command behavior.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirCoefficients`] when the path is empty.
    pub fn from_path(path: Option<String>) -> Result<Self> {
        match path {
            None => Ok(Self::stdin()),
            Some(path) if path == "-" => Ok(Self::stdin()),
            Some(path) if path.is_empty() => Err(EffectError::InvalidFirCoefficients),
            Some(path) => Ok(Self {
                source: FirCoefficientSource::File(path),
            }),
        }
    }

    /// Creates a `fir` input with inline coefficients.
    #[must_use]
    pub fn from_coefficients(coefficients: FirCoefficients) -> Self {
        Self {
            source: FirCoefficientSource::Inline(coefficients),
        }
    }

    /// Parses SoX-ng-style `fir` arguments.
    ///
    /// SoX-ng treats no arguments as standard input, a single argument as a
    /// coefficient-file path even when it looks numeric, and two or more
    /// arguments as inline numeric coefficients.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirCoefficients`] when inline coefficient
    /// arguments are malformed or non-finite, or when a single file path is
    /// empty.
    pub fn parse_sox_args(args: &[&str]) -> Result<Self> {
        match args {
            [] => Ok(Self::stdin()),
            [path] => Self::from_path(Some((*path).to_owned())),
            coefficients => {
                let values = coefficients
                    .iter()
                    .map(|value| {
                        value
                            .parse::<f64>()
                            .map_err(|_| EffectError::InvalidFirCoefficients)
                    })
                    .collect::<Result<Vec<_>>>()?;
                FirCoefficients::new(values).map(Self::from_coefficients)
            }
        }
    }

    /// Returns the coefficient source.
    #[must_use]
    pub const fn source(&self) -> &FirCoefficientSource {
        &self.source
    }

    /// Renders canonical SoX-ng-style command tokens for this coefficient
    /// input.
    #[must_use]
    pub fn render_tokens(&self) -> Vec<String> {
        match &self.source {
            FirCoefficientSource::Stdin => vec!["fir".to_owned(), "-".to_owned()],
            FirCoefficientSource::File(path) => vec!["fir".to_owned(), path.clone()],
            FirCoefficientSource::Inline(coefficients) => {
                let mut tokens = Vec::with_capacity(coefficients.len() + 1);
                tokens.push("fir".to_owned());
                tokens.extend(coefficients.as_slice().iter().copied().map(render_f64));
                tokens
            }
        }
    }

    /// Applies the resolved FIR coefficients to a decoded audio buffer.
    ///
    /// Empty coefficient lists are null effects and return the input audio
    /// unchanged. Library chain execution cannot read command-style stdin, so
    /// `FirCoefficientSource::Stdin` returns
    /// [`EffectError::InvalidFirCoefficients`]; CLI and library callers should
    /// pass inline coefficients or an explicit coefficient file path.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirCoefficients`] when command-style
    /// coefficient loading fails, or [`EffectError::FirLengthOverflow`] when
    /// the output shape cannot be represented.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        self.process_buffer_with_backend(audio, FirBackend::Direct)
    }

    pub(crate) fn process_buffer_with_backend(
        &self,
        audio: &AudioBuffer,
        backend: FirBackend,
    ) -> Result<AudioBuffer> {
        let coefficients = self.resolved_coefficients()?;
        if coefficients.is_empty() {
            return Ok(audio.clone());
        }

        let frames =
            usize::try_from(audio.frames().as_u64()).map_err(|_| EffectError::FirLengthOverflow)?;
        let mut planar = vec![0.0; audio.as_planar_f32().len()];
        let input = audio.as_planar_f32();
        let dft = match backend {
            FirBackend::Direct => None,
            FirBackend::Dft => Some(DspDftFir::new(coefficients.clone().into_dsp())),
        };

        for channel_index in 0..audio.channels().as_usize() {
            let start = channel_index
                .checked_mul(frames)
                .ok_or(EffectError::FirLengthOverflow)?;
            let end = start
                .checked_add(frames)
                .ok_or(EffectError::FirLengthOverflow)?;
            let channel = input
                .get(start..end)
                .ok_or(EffectError::FirLengthOverflow)?;
            let output = planar
                .get_mut(start..end)
                .ok_or(EffectError::FirLengthOverflow)?;
            match backend {
                FirBackend::Direct => {
                    FirState::new(coefficients.clone()).process_into(channel, output);
                }
                FirBackend::Dft => {
                    if let Some(dft) = &dft {
                        dft.process_into(channel, output);
                    }
                }
            }
        }

        AudioBuffer::from_planar_f32(
            audio.spec(),
            FrameCount::new(audio.frames().as_u64()),
            planar,
        )
        .map_err(|_| EffectError::FirLengthOverflow)
    }

    fn resolved_coefficients(&self) -> Result<FirCoefficients> {
        match &self.source {
            FirCoefficientSource::Inline(coefficients) => Ok(coefficients.clone()),
            FirCoefficientSource::File(path) => {
                let text = fs::read_to_string(Path::new(path))
                    .map_err(|_| EffectError::InvalidFirCoefficients)?;
                FirCoefficients::parse_text(&text)
            }
            FirCoefficientSource::Stdin => Err(EffectError::InvalidFirCoefficients),
        }
    }
}

/// Stateful scalar FIR processor for one mono sample stream.
///
/// This is a compatibility wrapper around the reusable `auralis-dsp` FIR
/// primitive. The state emits samples with SoX-ng-compatible alignment. For
/// coefficient lists longer than two taps, output for the newest input sample
/// is delayed until enough look-ahead is available; callers must invoke
/// [`Self::finish`] once at end-of-stream to flush the final aligned samples.
#[derive(Debug, Clone)]
pub struct FirState {
    inner: DspFirState,
}

impl FirState {
    /// Creates zero-initialized FIR state for one mono stream.
    #[must_use]
    pub fn new(coefficients: FirCoefficients) -> Self {
        Self {
            inner: DspFirState::new(coefficients.into_dsp()),
        }
    }

    /// Processes one chunk of mono samples, appending available output samples.
    pub fn process_mono_samples(&mut self, input: &[f32], output: &mut Vec<f32>) {
        self.inner.process_mono_samples(input, output);
    }

    /// Processes a complete mono stream into an equally sized output slice.
    pub fn process_into(self, input: &[f32], output: &mut [f32]) {
        self.inner.process_into(input, output);
    }

    /// Flushes delayed end-of-stream samples by appending the remaining output.
    ///
    /// This method consumes the state so a stream cannot accidentally be
    /// flushed twice.
    pub fn finish(self, output: &mut Vec<f32>) {
        self.inner.finish(output);
    }
}

fn push_token(values: &mut Vec<f64>, token: &mut String) -> Result<()> {
    if token.is_empty() {
        return Ok(());
    }

    let value = token
        .parse::<f64>()
        .map_err(|_| EffectError::InvalidFirCoefficients)?;
    values.push(value);
    token.clear();
    Ok(())
}

fn render_f64(value: f64) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{Fir, FirCoefficientSource, FirCoefficients, FirState};
    use crate::EffectError;

    #[test]
    fn parses_coefficient_text_with_comments() {
        let coefficients =
            FirCoefficients::parse_text(" # leading comment\n0.5 0.25#tail\n-0.125\n").unwrap();

        assert_eq!(coefficients.as_slice(), &[0.5, 0.25, -0.125]);
    }

    #[test]
    fn preserves_empty_coefficient_files_as_empty_lists() {
        let coefficients = FirCoefficients::parse_text("# no coefficients\n").unwrap();

        assert!(coefficients.is_empty());
    }

    #[test]
    fn rejects_malformed_or_non_finite_coefficients() {
        assert_eq!(
            FirCoefficients::parse_text("0.5 nope").unwrap_err(),
            EffectError::InvalidFirCoefficients
        );
        assert_eq!(
            FirCoefficients::new([0.5, f64::NAN]).unwrap_err(),
            EffectError::InvalidFirCoefficients
        );
        assert_eq!(
            Fir::parse_sox_args(&["0.5", "inf"]).unwrap_err(),
            EffectError::InvalidFirCoefficients
        );
    }

    #[test]
    fn parses_sox_argument_shapes() {
        assert_eq!(Fir::parse_sox_args(&[]).unwrap(), Fir::stdin());
        assert_eq!(
            Fir::parse_sox_args(&["-"]).unwrap().source(),
            &FirCoefficientSource::Stdin
        );
        assert_eq!(
            Fir::parse_sox_args(&["0.5"]).unwrap().source(),
            &FirCoefficientSource::File("0.5".to_owned())
        );
        assert_eq!(
            Fir::parse_sox_args(&["0.5", "0.25"])
                .unwrap()
                .render_tokens(),
            ["fir", "0.5", "0.25"]
        );
    }

    #[test]
    fn rejects_invalid_paths_and_mixed_inline_arguments() {
        assert_eq!(
            Fir::parse_sox_args(&[""]).unwrap_err(),
            EffectError::InvalidFirCoefficients
        );
        assert_eq!(
            Fir::parse_sox_args(&["coeffs.txt", "extra"]).unwrap_err(),
            EffectError::InvalidFirCoefficients
        );
    }

    #[test]
    fn empty_coefficients_are_a_null_effect() {
        let fir = Fir::from_coefficients(FirCoefficients::new([]).unwrap());
        let audio = mono_audio_buffer(vec![0.0, 0.25, -0.5]);

        let filtered = fir.process_buffer(&audio).unwrap();

        assert_eq!(filtered, audio);
    }

    #[test]
    fn filters_mono_samples_with_sox_aligned_impulse_response() {
        let fir = Fir::from_coefficients(FirCoefficients::new([1.0, 2.0, 3.0]).unwrap());
        let audio = mono_audio_buffer(vec![1.0, 0.0, 0.0, 0.0]);

        let filtered = fir.process_buffer(&audio).unwrap();

        assert_eq!(filtered.as_planar_f32(), &[2.0, 3.0, 0.0, 0.0]);
    }

    #[test]
    fn filters_each_channel_independently() {
        let fir = Fir::from_coefficients(FirCoefficients::new([0.5, 0.25]).unwrap());
        let audio = stereo_audio_buffer(vec![1.0, 0.0, 0.0, 0.0, -1.0, 0.0]);

        let filtered = fir.process_buffer(&audio).unwrap();

        assert_eq!(
            filtered.as_planar_f32(),
            &[0.5, 0.25, 0.0, 0.0, -0.5, -0.25]
        );
    }

    #[test]
    fn command_style_stdin_is_rejected_at_processing_time() {
        let audio = mono_audio_buffer(vec![0.0]);

        assert_eq!(
            Fir::stdin().process_buffer(&audio).unwrap_err(),
            EffectError::InvalidFirCoefficients
        );
    }

    #[test]
    fn state_preserves_alignment_across_chunks() {
        let coefficients = FirCoefficients::new([0.25, 0.5, 0.25]).unwrap();
        let fir = Fir::from_coefficients(coefficients.clone());
        let audio = mono_audio_buffer(vec![0.0, 1.0, 0.5, -0.5, 0.0]);
        let whole = fir.process_buffer(&audio).unwrap();
        let mut state = FirState::new(coefficients);
        let mut chunked = Vec::new();

        state.process_mono_samples(&audio.as_planar_f32()[..2], &mut chunked);
        state.process_mono_samples(&audio.as_planar_f32()[2..3], &mut chunked);
        state.process_mono_samples(&audio.as_planar_f32()[3..], &mut chunked);
        state.finish(&mut chunked);

        assert_eq!(whole.as_planar_f32(), chunked.as_slice());
    }

    fn mono_audio_buffer(samples: Vec<f32>) -> auralis_core::AudioBuffer {
        audio_buffer(samples, 1)
    }

    fn stereo_audio_buffer(samples: Vec<f32>) -> auralis_core::AudioBuffer {
        audio_buffer(samples, 2)
    }

    fn audio_buffer(samples: Vec<f32>, channels: u16) -> auralis_core::AudioBuffer {
        let spec = auralis_core::AudioSpec::new(
            auralis_core::SampleRate::new(48_000).unwrap(),
            auralis_core::ChannelCount::new(channels).unwrap(),
            auralis_core::SampleFormat::Float32,
        );
        auralis_core::AudioBuffer::from_planar_f32(
            spec,
            auralis_core::FrameCount::new(
                u64::try_from(samples.len() / usize::from(channels)).unwrap(),
            ),
            samples,
        )
        .unwrap()
    }
}
