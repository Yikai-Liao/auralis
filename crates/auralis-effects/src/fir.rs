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
    values: Vec<f64>,
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
        let values = values.into_iter().collect::<Vec<_>>();
        if values.iter().all(|value| value.is_finite()) {
            Ok(Self { values })
        } else {
            Err(EffectError::InvalidFirCoefficients)
        }
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
        &self.values
    }

    /// Returns the number of coefficients.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns `true` when no coefficients were supplied.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
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

/// Parsed SoX-ng-style FIR coefficient input.
///
/// This type intentionally models only coefficient acquisition for Feature
/// 6.8.1. It is not registered as an executable effect command until the
/// streaming FIR processor is implemented.
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
    use super::{Fir, FirCoefficientSource, FirCoefficients};
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
}
