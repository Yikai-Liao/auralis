use std::{fs, path::Path};

use auralis_core::{AudioBuffer, SampleRate};

use crate::{EffectError, Fir, FirCoefficients, Result, fir::FirBackend};

const FIRFIT_TAP_COUNT: usize = 2047;
const FIRFIT_CENTER_TAP: usize = FIRFIT_TAP_COUNT / 2;
const FIRFIT_RESPONSE_STEPS: usize = 4096;

/// A frequency/gain knot for SoX-ng-style `firfit` response fitting.
///
/// Frequencies are measured in hertz and gains are measured in decibels.
/// Knots must be supplied in strictly increasing frequency order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirFitKnot {
    frequency_hz: f64,
    gain_db: f64,
}

impl FirFitKnot {
    /// Creates one validated `firfit` knot.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirFit`] when the frequency is negative
    /// or either value is NaN or infinite.
    pub fn new(frequency_hz: f64, gain_db: f64) -> Result<Self> {
        if frequency_hz.is_finite() && frequency_hz >= 0.0 && gain_db.is_finite() {
            Ok(Self {
                frequency_hz,
                gain_db,
            })
        } else {
            Err(EffectError::InvalidFirFit)
        }
    }

    /// Returns the knot frequency in hertz.
    #[must_use]
    pub const fn frequency_hz(self) -> f64 {
        self.frequency_hz
    }

    /// Returns the knot gain in decibels.
    #[must_use]
    pub const fn gain_db(self) -> f64 {
        self.gain_db
    }
}

/// The source of knots for a SoX-ng-style `firfit` command.
#[derive(Debug, Clone, PartialEq)]
pub enum FirFitKnotSource {
    /// Read knots from standard input (`firfit` or `firfit -`).
    Stdin,
    /// Read knots from a named knot file.
    File(String),
    /// Use knots supplied directly on the command line.
    Inline(Vec<FirFitKnot>),
}

/// Parsed SoX-ng-style FIR response fitting effect.
///
/// `firfit [knots-file | <freq gain>]` builds a deterministic 2047-tap scalar
/// FIR filter from frequency/gain knots and then applies it with the same
/// length-preserving FIR executor used by [`Fir`]. A flat response is emitted
/// as an exact centered impulse, while non-flat responses use a deterministic
/// log-frequency interpolation scaffold suitable for the current scalar
/// reference implementation.
#[derive(Debug, Clone, PartialEq)]
pub struct FirFit {
    source: FirFitKnotSource,
}

impl FirFit {
    /// Creates a `firfit` input that reads knots from standard input.
    #[must_use]
    pub const fn stdin() -> Self {
        Self {
            source: FirFitKnotSource::Stdin,
        }
    }

    /// Creates a `firfit` input that reads knots from a file path.
    ///
    /// `None` and `Some("-")` represent standard input.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirFit`] when the path is empty.
    pub fn from_path(path: Option<String>) -> Result<Self> {
        match path {
            None => Ok(Self::stdin()),
            Some(path) if path == "-" => Ok(Self::stdin()),
            Some(path) if path.is_empty() => Err(EffectError::InvalidFirFit),
            Some(path) => Ok(Self {
                source: FirFitKnotSource::File(path),
            }),
        }
    }

    /// Creates a `firfit` input with inline knots.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirFit`] unless at least one strictly
    /// increasing knot is supplied.
    pub fn from_knots<I>(knots: I) -> Result<Self>
    where
        I: IntoIterator<Item = FirFitKnot>,
    {
        let knots = knots.into_iter().collect::<Vec<_>>();
        validate_knots(&knots)?;
        Ok(Self {
            source: FirFitKnotSource::Inline(knots),
        })
    }

    /// Parses SoX-ng-style `firfit` arguments.
    ///
    /// No arguments and `-` read knots from standard input. One argument is a
    /// knot-file path. Two or more arguments are parsed as inline
    /// frequency/gain pairs.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirFit`] when inline arguments are not
    /// complete finite frequency/gain pairs or frequencies are not strictly
    /// increasing.
    pub fn parse_sox_args(args: &[&str]) -> Result<Self> {
        match args {
            [] => Ok(Self::stdin()),
            [path] => Self::from_path(Some((*path).to_owned())),
            values if values.len() % 2 == 0 => {
                let mut knots = Vec::with_capacity(values.len() / 2);
                for pair in values.chunks_exact(2) {
                    let frequency_hz = parse_frequency_hz(pair[0])?;
                    let gain_db = parse_gain_db(pair[1])?;
                    knots.push(FirFitKnot::new(frequency_hz, gain_db)?);
                }
                Self::from_knots(knots)
            }
            _ => Err(EffectError::InvalidFirFit),
        }
    }

    /// Parses SoX-ng-style knot text.
    ///
    /// Whitespace separates values and `#` comments run to the end of the
    /// line. Multiple frequency/gain pairs may appear on one line.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirFit`] when the text does not contain
    /// complete strictly increasing finite frequency/gain pairs.
    pub fn parse_knot_text(text: &str) -> Result<Vec<FirFitKnot>> {
        let mut tokens = Vec::new();
        for line in text.lines() {
            let data = line.split_once('#').map_or(line, |(data, _)| data);
            tokens.extend(data.split_whitespace());
        }
        if tokens.len() % 2 != 0 {
            return Err(EffectError::InvalidFirFit);
        }

        let mut knots = Vec::with_capacity(tokens.len() / 2);
        for pair in tokens.chunks_exact(2) {
            knots.push(FirFitKnot::new(
                parse_frequency_hz(pair[0])?,
                parse_gain_db(pair[1])?,
            )?);
        }
        validate_knots(&knots)?;
        Ok(knots)
    }

    /// Returns the knot source.
    #[must_use]
    pub const fn source(&self) -> &FirFitKnotSource {
        &self.source
    }

    /// Renders canonical SoX-ng-style command tokens.
    #[must_use]
    pub fn render_tokens(&self) -> Vec<String> {
        match &self.source {
            FirFitKnotSource::Stdin => vec!["firfit".to_owned(), "-".to_owned()],
            FirFitKnotSource::File(path) => vec!["firfit".to_owned(), path.clone()],
            FirFitKnotSource::Inline(knots) => {
                let mut tokens = Vec::with_capacity(1 + knots.len() * 2);
                tokens.push("firfit".to_owned());
                for knot in knots {
                    tokens.push(render_f64(knot.frequency_hz()));
                    tokens.push(render_f64(knot.gain_db()));
                }
                tokens
            }
        }
    }

    /// Designs scalar FIR coefficients for a concrete input sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirFit`] when command-style knot loading
    /// fails or when the knot source is stdin.
    pub fn coefficients_for_sample_rate(&self, sample_rate: SampleRate) -> Result<FirCoefficients> {
        let knots = self.resolved_knots()?;
        design_coefficients(&knots, sample_rate)
    }

    /// Applies the designed FIR response to a decoded audio buffer.
    ///
    /// Command-style stdin is rejected at processing time because in-memory
    /// library execution cannot safely read interactive stdin.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFirFit`] when knot loading or FIR design
    /// fails, or a FIR execution error if the output shape cannot be
    /// represented.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let coefficients = self.coefficients_for_sample_rate(audio.spec().sample_rate())?;
        let backend = if is_centered_impulse(coefficients.as_slice()) {
            FirBackend::Direct
        } else {
            FirBackend::Dft
        };
        Fir::from_coefficients(coefficients).process_buffer_with_backend(audio, backend)
    }

    fn resolved_knots(&self) -> Result<Vec<FirFitKnot>> {
        match &self.source {
            FirFitKnotSource::Inline(knots) => Ok(knots.clone()),
            FirFitKnotSource::File(path) => {
                let text =
                    fs::read_to_string(Path::new(path)).map_err(|_| EffectError::InvalidFirFit)?;
                Self::parse_knot_text(&text)
            }
            FirFitKnotSource::Stdin => Err(EffectError::InvalidFirFit),
        }
    }
}

fn validate_knots(knots: &[FirFitKnot]) -> Result<()> {
    if knots.is_empty() {
        return Err(EffectError::InvalidFirFit);
    }

    for pair in knots.windows(2) {
        if pair[1].frequency_hz() <= pair[0].frequency_hz() {
            return Err(EffectError::InvalidFirFit);
        }
    }

    Ok(())
}

#[allow(
    clippy::cast_precision_loss,
    reason = "firfit response-grid indices are bounded constants and require f64 DSP math"
)]
fn design_coefficients(knots: &[FirFitKnot], sample_rate: SampleRate) -> Result<FirCoefficients> {
    validate_knots(knots)?;
    if let Some(gain_db) = flat_gain_db(knots) {
        return FirCoefficients::new(centered_impulse(db_to_linear(gain_db)));
    }

    let nyquist = f64::from(sample_rate.as_u32()) * 0.5;
    let mut response = Vec::with_capacity(FIRFIT_RESPONSE_STEPS + 1);
    for index in 0..=FIRFIT_RESPONSE_STEPS {
        let frequency = nyquist * index as f64 / FIRFIT_RESPONSE_STEPS as f64;
        response.push(db_to_linear(interpolated_gain_db(knots, frequency)));
    }

    let mut coefficients = Vec::with_capacity(FIRFIT_TAP_COUNT);
    for tap in 0..FIRFIT_TAP_COUNT {
        let offset = isize::try_from(tap).expect("tap count fits isize")
            - isize::try_from(FIRFIT_CENTER_TAP).expect("tap count fits isize");
        let mut sum = 0.0;
        for (index, amplitude) in response.iter().copied().enumerate() {
            let weight = if index == 0 || index == FIRFIT_RESPONSE_STEPS {
                0.5
            } else {
                1.0
            };
            let phase =
                std::f64::consts::PI * index as f64 * offset as f64 / FIRFIT_RESPONSE_STEPS as f64;
            sum += weight * amplitude * phase.cos();
        }
        coefficients.push(sum / FIRFIT_RESPONSE_STEPS as f64 * blackman_nuttall(tap));
    }

    FirCoefficients::new(coefficients)
}

fn flat_gain_db(knots: &[FirFitKnot]) -> Option<f64> {
    let first = knots.first()?.gain_db();
    if knots
        .iter()
        .all(|knot| (knot.gain_db() - first).abs() <= f64::EPSILON)
    {
        Some(first)
    } else {
        None
    }
}

fn centered_impulse(multiplier: f64) -> Vec<f64> {
    let mut coefficients = vec![0.0; FIRFIT_TAP_COUNT];
    coefficients[FIRFIT_CENTER_TAP] = multiplier;
    coefficients
}

fn is_centered_impulse(coefficients: &[f64]) -> bool {
    coefficients.len() == FIRFIT_TAP_COUNT
        && coefficients
            .iter()
            .enumerate()
            .all(|(index, coefficient)| index == FIRFIT_CENTER_TAP || *coefficient == 0.0)
}

fn interpolated_gain_db(knots: &[FirFitKnot], frequency_hz: f64) -> f64 {
    let frequency = frequency_hz.max(1.0);
    let first = knots[0];
    if frequency <= first.frequency_hz().max(1.0) {
        return first.gain_db();
    }

    for pair in knots.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let left_frequency = left.frequency_hz().max(1.0);
        let right_frequency = right.frequency_hz().max(1.0);
        if frequency <= right_frequency {
            let span = right_frequency.ln() - left_frequency.ln();
            if span <= f64::EPSILON {
                return right.gain_db();
            }
            let t = (frequency.ln() - left_frequency.ln()) / span;
            return left.gain_db() + t * (right.gain_db() - left.gain_db());
        }
    }

    knots
        .last()
        .expect("validated knots are non-empty")
        .gain_db()
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Blackman-Nuttall window positions are bounded by the fixed FIR tap count"
)]
fn blackman_nuttall(tap: usize) -> f64 {
    let position = tap as f64 / (FIRFIT_TAP_COUNT - 1) as f64;
    0.363_581_9 - 0.489_177_5 * (2.0 * std::f64::consts::PI * position).cos()
        + 0.136_599_5 * (4.0 * std::f64::consts::PI * position).cos()
        - 0.010_641_1 * (6.0 * std::f64::consts::PI * position).cos()
}

fn parse_frequency_hz(token: &str) -> Result<f64> {
    let (number, multiplier) = match token.as_bytes().last().copied() {
        Some(b'k' | b'K') => (&token[..token.len() - 1], 1_000.0),
        _ => (token, 1.0),
    };
    let frequency = number
        .parse::<f64>()
        .map_err(|_| EffectError::InvalidFirFit)?
        * multiplier;
    if frequency.is_finite() && frequency >= 0.0 {
        Ok(frequency)
    } else {
        Err(EffectError::InvalidFirFit)
    }
}

fn parse_gain_db(token: &str) -> Result<f64> {
    let gain = token
        .parse::<f64>()
        .map_err(|_| EffectError::InvalidFirFit)?;
    if gain.is_finite() {
        Ok(gain)
    } else {
        Err(EffectError::InvalidFirFit)
    }
}

fn db_to_linear(gain_db: f64) -> f64 {
    10.0_f64.powf(gain_db / 20.0)
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
    use super::{FirFit, FirFitKnot, FirFitKnotSource};
    use crate::EffectError;

    #[test]
    fn parses_knot_text_with_comments_and_multiple_pairs_per_line() {
        let knots =
            FirFit::parse_knot_text(" # telephone\n300 -100 3k 0\n10k -6 # tail\n").unwrap();

        assert_eq!(
            knots,
            [
                FirFitKnot::new(300.0, -100.0).unwrap(),
                FirFitKnot::new(3000.0, 0.0).unwrap(),
                FirFitKnot::new(10_000.0, -6.0).unwrap(),
            ]
        );
    }

    #[test]
    fn parses_sox_argument_shapes() {
        assert_eq!(FirFit::parse_sox_args(&[]).unwrap(), FirFit::stdin());
        assert_eq!(
            FirFit::parse_sox_args(&["-"]).unwrap().source(),
            &FirFitKnotSource::Stdin
        );
        assert_eq!(
            FirFit::parse_sox_args(&["knots.txt"]).unwrap().source(),
            &FirFitKnotSource::File("knots.txt".to_owned())
        );
        assert_eq!(
            FirFit::parse_sox_args(&["20", "0", "10k", "-3"])
                .unwrap()
                .render_tokens(),
            ["firfit", "20", "0", "10000", "-3"]
        );
    }

    #[test]
    fn rejects_malformed_or_unordered_knots() {
        assert_eq!(
            FirFit::parse_sox_args(&["20", "0", "10k"]).unwrap_err(),
            EffectError::InvalidFirFit
        );
        assert_eq!(
            FirFit::parse_sox_args(&["20", "0", "10", "-3"]).unwrap_err(),
            EffectError::InvalidFirFit
        );
        assert_eq!(
            FirFit::parse_sox_args(&["20", "nan"]).unwrap_err(),
            EffectError::InvalidFirFit
        );
    }

    #[test]
    fn flat_response_designs_centered_impulse() {
        let firfit = FirFit::parse_sox_args(&["20", "-6", "10k", "-6"]).unwrap();

        let coefficients = firfit
            .coefficients_for_sample_rate(auralis_core::SampleRate::new(48_000).unwrap())
            .unwrap();

        assert_eq!(coefficients.len(), 2047);
        assert!(coefficients.as_slice()[1023] > 0.501);
        assert!(coefficients.as_slice()[1023] < 0.502);
        assert_eq!(coefficients.as_slice()[0].to_bits(), 0.0_f64.to_bits());
    }
}
