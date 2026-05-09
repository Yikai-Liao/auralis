//! Golden-test manifest parsing and deterministic command rendering.
//!
//! Golden manifests are small TOML documents that describe one or more
//! reference comparisons. Each case lives under the `id` table; the table key is
//! the stable case identifier used in test reports and failure artifacts.
//! Command vectors are rendered separately from command-line displays: vectors
//! remain suitable for `std::process::Command`, while display helpers apply
//! deterministic quoting and escaping for human-readable failure reports.
//!
//! # Examples
//!
//! ```
//! use auralis_testkit::golden::GoldenManifest;
//!
//! let manifest = GoldenManifest::parse_toml(
//!     r#"
//!     [id.gain_minus_3_mono]
//!     input = "sine_48k_mono.wav"
//!     auralis = ["--gain-db", "-3"]
//!     sox_ng = ["gain", "-3"]
//!     max_abs = 1e-4
//!     rms = 1e-6
//!     snr_db = 90.0
//!     "#,
//! )?;
//!
//! let (_, case) = manifest.iter().next().expect("one case");
//! assert_eq!(
//!     case.render_sox_ng_command("sox_ng", case.input(), "out.wav"),
//!     ["sox_ng", "-R", "-D", "sine_48k_mono.wav", "out.wav", "gain", "-3"],
//! );
//! assert_eq!(
//!     case.render_sox_ng_command_line("sox_ng", "input file.wav", "out.wav"),
//!     "sox_ng -R -D \"input file.wav\" out.wav gain -3",
//! );
//! # Ok::<(), auralis_testkit::golden::GoldenManifestError>(())
//! ```

use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

use serde::Deserialize;

/// Result type for golden manifest parsing and validation.
pub type Result<T> = std::result::Result<T, GoldenManifestError>;

/// A parsed golden-test manifest.
///
/// Cases are stored in a [`BTreeMap`] so iteration and report rendering are
/// deterministic across platforms and TOML parser internals.
#[derive(Debug, Clone, PartialEq)]
pub struct GoldenManifest {
    cases: BTreeMap<String, GoldenCase>,
}

impl GoldenManifest {
    /// Parses and validates a TOML golden-test manifest.
    ///
    /// The accepted format contains one or more `[id.<case>]` tables. Each case
    /// table must define `input`, `auralis`, `sox_ng`, `max_abs`, `rms`, and
    /// `snr_db`.
    ///
    /// # Errors
    ///
    /// Returns [`GoldenManifestError::Toml`] when the TOML is syntactically
    /// invalid or misses required fields. Returns a validation variant when a
    /// case identifier, input path, command argument, or tolerance value is not
    /// usable for deterministic golden tests.
    pub fn parse_toml(source: &str) -> Result<Self> {
        let raw = toml::from_str::<RawManifest>(source).map_err(GoldenManifestError::Toml)?;
        Self::from_raw(raw)
    }

    /// Returns all manifest cases keyed by stable case identifier.
    #[must_use]
    pub const fn cases(&self) -> &BTreeMap<String, GoldenCase> {
        &self.cases
    }

    /// Returns a single case by identifier.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&GoldenCase> {
        self.cases.get(id)
    }

    /// Iterates over manifest cases in deterministic identifier order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &GoldenCase)> {
        self.cases.iter().map(|(id, case)| (id.as_str(), case))
    }

    fn from_raw(raw: RawManifest) -> Result<Self> {
        if raw.id.is_empty() {
            return Err(GoldenManifestError::EmptyManifest);
        }

        let mut cases = BTreeMap::new();
        for (id, raw_case) in raw.id {
            if !is_valid_case_id(&id) {
                return Err(GoldenManifestError::InvalidCaseId { id });
            }
            cases.insert(id.clone(), GoldenCase::from_raw(&id, raw_case)?);
        }

        Ok(Self { cases })
    }
}

/// One golden comparison case from a manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct GoldenCase {
    input: PathBuf,
    auralis: Vec<String>,
    sox_ng: Vec<String>,
    tolerance: GoldenTolerance,
}

impl GoldenCase {
    /// Returns the input fixture path recorded by the manifest.
    ///
    /// The path is usually relative to the test corpus root. Callers choose
    /// whether to join it to a temporary fixture directory or use it directly.
    #[must_use]
    pub fn input(&self) -> &Path {
        &self.input
    }

    /// Returns the Auralis argument fragment recorded by the manifest.
    ///
    /// Arguments are appended after `auralis run <input> <output>` by
    /// [`Self::render_auralis_command`].
    #[must_use]
    pub fn auralis_args(&self) -> &[String] {
        &self.auralis
    }

    /// Returns the SoX-ng argument fragment recorded by the manifest.
    ///
    /// Arguments are appended after `sox_ng -R -D <input> <output>` by
    /// [`Self::render_sox_ng_command`].
    #[must_use]
    pub fn sox_ng_args(&self) -> &[String] {
        &self.sox_ng
    }

    /// Returns the metric thresholds for this comparison.
    #[must_use]
    pub const fn tolerance(&self) -> GoldenTolerance {
        self.tolerance
    }

    /// Renders a deterministic Auralis command vector.
    ///
    /// The returned vector is suitable for `std::process::Command` construction
    /// and stable failure reporting. It intentionally does not shell-quote
    /// arguments; display layers should quote only when rendering for humans.
    #[must_use]
    pub fn render_auralis_command(
        &self,
        executable: impl AsRef<str>,
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
    ) -> Vec<String> {
        let mut command = vec![
            executable.as_ref().to_owned(),
            "run".to_owned(),
            path_to_command_arg(input_path.as_ref()),
            path_to_command_arg(output_path.as_ref()),
        ];
        command.extend(self.auralis.iter().cloned());
        command
    }

    /// Renders a deterministic display form of the Auralis command.
    ///
    /// This is intended for failure reports and logs. It uses the same command
    /// vector as [`Self::render_auralis_command`] and then applies
    /// [`render_command_line`] so whitespace, quotes, backslashes, and control
    /// characters are escaped consistently across runs.
    #[must_use]
    pub fn render_auralis_command_line(
        &self,
        executable: impl AsRef<str>,
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
    ) -> String {
        render_command_line(self.render_auralis_command(executable, input_path, output_path))
    }

    /// Renders a deterministic SoX-ng command vector.
    ///
    /// The `-R` and `-D` flags are always included to match Auralis' repeatable
    /// golden-test policy: repeatable random state and disabled automatic
    /// dithering.
    #[must_use]
    pub fn render_sox_ng_command(
        &self,
        executable: impl AsRef<str>,
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
    ) -> Vec<String> {
        let mut command = vec![
            executable.as_ref().to_owned(),
            "-R".to_owned(),
            "-D".to_owned(),
            path_to_command_arg(input_path.as_ref()),
            path_to_command_arg(output_path.as_ref()),
        ];
        command.extend(self.sox_ng.iter().cloned());
        command
    }

    /// Renders a deterministic display form of the SoX-ng command.
    ///
    /// This is intended for failure reports and logs. It includes the same
    /// repeatability flags as [`Self::render_sox_ng_command`] and applies
    /// [`render_command_line`] for stable quoting and escaping.
    #[must_use]
    pub fn render_sox_ng_command_line(
        &self,
        executable: impl AsRef<str>,
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
    ) -> String {
        render_command_line(self.render_sox_ng_command(executable, input_path, output_path))
    }

    fn from_raw(id: &str, raw: RawGoldenCase) -> Result<Self> {
        if raw.input.is_empty() {
            return Err(GoldenManifestError::EmptyInput { id: id.to_owned() });
        }

        validate_args(id, GoldenCommand::Auralis, &raw.auralis)?;
        validate_args(id, GoldenCommand::SoxNg, &raw.sox_ng)?;
        validate_tolerance(id, GoldenMetric::MaxAbs, raw.max_abs)?;
        validate_tolerance(id, GoldenMetric::Rms, raw.rms)?;
        validate_tolerance(id, GoldenMetric::SnrDb, raw.snr_db)?;

        Ok(Self {
            input: raw.input.into(),
            auralis: raw.auralis,
            sox_ng: raw.sox_ng,
            tolerance: GoldenTolerance {
                max_abs: raw.max_abs,
                rms: raw.rms,
                snr_db: raw.snr_db,
            },
        })
    }
}

/// Renders a deterministic single-line display for a command vector.
///
/// Arguments that are plain ASCII command tokens are left unquoted. Arguments
/// containing whitespace, quotes, backslashes, or control characters are
/// double-quoted with stable backslash escaping. The returned string is meant
/// for diagnostics and failure artifacts; callers should keep using the vector
/// form when spawning a process.
#[must_use]
pub fn render_command_line<I, S>(command: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut rendered = String::new();
    for (index, argument) in command.into_iter().enumerate() {
        if index > 0 {
            rendered.push(' ');
        }
        rendered.push_str(&quote_command_arg(argument.as_ref()));
    }

    rendered
}

/// Quotes one command argument for deterministic human-readable reports.
///
/// Safe ASCII command tokens are returned unchanged. Other arguments are
/// double-quoted and escaped with deterministic sequences for `"`, `\`,
/// newline, carriage return, tab, and other ASCII control bytes.
#[must_use]
pub fn quote_command_arg(argument: &str) -> String {
    if !argument.is_empty() && argument.bytes().all(is_unquoted_command_byte) {
        return argument.to_owned();
    }

    let mut quoted = String::with_capacity(argument.len() + 2);
    quoted.push('"');
    for character in argument.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            control if control.is_ascii_control() => {
                use std::fmt::Write as _;

                write!(quoted, "\\x{:02x}", control as u32)
                    .expect("writing to an in-memory String cannot fail");
            }
            other => quoted.push(other),
        }
    }
    quoted.push('"');

    quoted
}

/// Metric thresholds used to decide whether a golden comparison passes.
///
/// `max_abs` and `rms` are upper bounds on full-scale sample error. `snr_db` is
/// the minimum acceptable signal-to-noise ratio in decibels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoldenTolerance {
    /// Maximum permitted absolute sample error.
    pub max_abs: f64,

    /// Maximum permitted root-mean-square sample error.
    pub rms: f64,

    /// Minimum permitted signal-to-noise ratio in decibels.
    pub snr_db: f64,
}

/// Errors produced while parsing or validating a golden manifest.
#[derive(Debug)]
#[non_exhaustive]
pub enum GoldenManifestError {
    /// The TOML parser rejected the document or a required field was missing.
    Toml(toml::de::Error),

    /// The manifest contained no `[id.<case>]` tables.
    EmptyManifest,

    /// A case identifier was empty or contained unsupported characters.
    InvalidCaseId {
        /// Rejected case identifier.
        id: String,
    },

    /// A case had an empty input path.
    EmptyInput {
        /// Case identifier containing the invalid input path.
        id: String,
    },

    /// A command argument was an empty string.
    EmptyArgument {
        /// Case identifier containing the invalid argument.
        id: String,

        /// Command field containing the invalid argument.
        command: GoldenCommand,
    },

    /// A metric threshold was negative, NaN, or infinite.
    InvalidTolerance {
        /// Case identifier containing the invalid threshold.
        id: String,

        /// Metric field containing the invalid threshold.
        metric: GoldenMetric,

        /// Rejected value.
        value: f64,
    },
}

impl fmt::Display for GoldenManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(error) => write!(formatter, "invalid golden manifest TOML: {error}"),
            Self::EmptyManifest => {
                formatter.write_str("golden manifest must contain at least one case")
            }
            Self::InvalidCaseId { id } => {
                write!(formatter, "invalid golden manifest case id `{id}`")
            }
            Self::EmptyInput { id } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` must define a non-empty input"
                )
            }
            Self::EmptyArgument { id, command } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` contains an empty {command} argument"
                )
            }
            Self::InvalidTolerance { id, metric, value } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` has invalid {metric} tolerance `{value}`"
                )
            }
        }
    }
}

impl Error for GoldenManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Toml(error) => Some(error),
            Self::EmptyManifest
            | Self::InvalidCaseId { .. }
            | Self::EmptyInput { .. }
            | Self::EmptyArgument { .. }
            | Self::InvalidTolerance { .. } => None,
        }
    }
}

/// Manifest command field names used in validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoldenCommand {
    /// Auralis command argument fragment.
    Auralis,

    /// SoX-ng command argument fragment.
    SoxNg,
}

impl fmt::Display for GoldenCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auralis => formatter.write_str("auralis"),
            Self::SoxNg => formatter.write_str("sox_ng"),
        }
    }
}

/// Manifest metric field names used in validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoldenMetric {
    /// `max_abs` tolerance.
    MaxAbs,

    /// `rms` tolerance.
    Rms,

    /// `snr_db` tolerance.
    SnrDb,
}

impl fmt::Display for GoldenMetric {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxAbs => formatter.write_str("max_abs"),
            Self::Rms => formatter.write_str("rms"),
            Self::SnrDb => formatter.write_str("snr_db"),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    id: BTreeMap<String, RawGoldenCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGoldenCase {
    input: String,
    auralis: Vec<String>,
    sox_ng: Vec<String>,
    max_abs: f64,
    rms: f64,
    snr_db: f64,
}

fn is_valid_case_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn validate_args(id: &str, command: GoldenCommand, args: &[String]) -> Result<()> {
    if args.iter().any(String::is_empty) {
        Err(GoldenManifestError::EmptyArgument {
            id: id.to_owned(),
            command,
        })
    } else {
        Ok(())
    }
}

fn validate_tolerance(id: &str, metric: GoldenMetric, value: f64) -> Result<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(GoldenManifestError::InvalidTolerance {
            id: id.to_owned(),
            metric,
            value,
        })
    }
}

fn path_to_command_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn is_unquoted_command_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'@' | b'%' | b'_' | b'+' | b'=' | b':' | b',' | b'.' | b'/' | b'-'
        )
}

#[cfg(test)]
mod tests {
    use super::{
        GoldenCommand, GoldenManifest, GoldenManifestError, GoldenMetric, quote_command_arg,
        render_command_line,
    };

    const VALID_MANIFEST: &str = r#"
        [id.fade_out_stereo]
        input = "stereo/step.wav"
        auralis = ["--fade-out-frame", "4"]
        sox_ng = ["fade", "0", "0", "4s"]
        max_abs = 0.0001
        rms = 0.000001
        snr_db = 90.0

        [id.gain_minus_3_mono]
        input = "mono/sine.wav"
        auralis = ["--gain-db", "-3"]
        sox_ng = ["gain", "-3"]
        max_abs = 0.0001
        rms = 0.000001
        snr_db = 90.0
    "#;

    #[test]
    fn manifest_parse_keeps_cases_in_deterministic_id_order() {
        let manifest = GoldenManifest::parse_toml(VALID_MANIFEST).unwrap();

        let ids = manifest.iter().map(|(id, _)| id).collect::<Vec<_>>();

        assert_eq!(ids, ["fade_out_stereo", "gain_minus_3_mono"]);
        assert_eq!(manifest.cases().len(), 2);
    }

    #[test]
    fn manifest_parse_preserves_case_fields() {
        let manifest = GoldenManifest::parse_toml(VALID_MANIFEST).unwrap();
        let case = manifest.get("gain_minus_3_mono").unwrap();

        assert_eq!(case.input().to_string_lossy(), "mono/sine.wav");
        assert_eq!(case.auralis_args(), ["--gain-db", "-3"]);
        assert_eq!(case.sox_ng_args(), ["gain", "-3"]);
        assert_float_eq(case.tolerance().max_abs, 0.000_1);
        assert_float_eq(case.tolerance().rms, 0.000_001);
        assert_float_eq(case.tolerance().snr_db, 90.0);
    }

    #[test]
    fn invalid_manifest_is_rejected_for_missing_fields() {
        let error = GoldenManifest::parse_toml(
            r#"
            [id.missing_sox_command]
            input = "mono/sine.wav"
            auralis = ["--gain-db", "-3"]
            max_abs = 0.0001
            rms = 0.000001
            snr_db = 90.0
            "#,
        )
        .unwrap_err();

        assert!(matches!(error, GoldenManifestError::Toml(_)));
    }

    #[test]
    fn invalid_manifest_is_rejected_for_bad_case_id() {
        let error = GoldenManifest::parse_toml(
            r#"
            [id."gain minus 3"]
            input = "mono/sine.wav"
            auralis = ["--gain-db", "-3"]
            sox_ng = ["gain", "-3"]
            max_abs = 0.0001
            rms = 0.000001
            snr_db = 90.0
            "#,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            GoldenManifestError::InvalidCaseId { id } if id == "gain minus 3"
        ));
    }

    #[test]
    fn invalid_manifest_is_rejected_for_empty_argument() {
        let error = GoldenManifest::parse_toml(
            r#"
            [id.gain_minus_3]
            input = "mono/sine.wav"
            auralis = ["--gain-db", ""]
            sox_ng = ["gain", "-3"]
            max_abs = 0.0001
            rms = 0.000001
            snr_db = 90.0
            "#,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            GoldenManifestError::EmptyArgument {
                id,
                command: GoldenCommand::Auralis,
            } if id == "gain_minus_3"
        ));
    }

    #[test]
    fn invalid_manifest_is_rejected_for_negative_tolerance() {
        let error = GoldenManifest::parse_toml(
            r#"
            [id.gain_minus_3]
            input = "mono/sine.wav"
            auralis = ["--gain-db", "-3"]
            sox_ng = ["gain", "-3"]
            max_abs = -0.0001
            rms = 0.000001
            snr_db = 90.0
            "#,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            GoldenManifestError::InvalidTolerance {
                id,
                metric: GoldenMetric::MaxAbs,
                ..
            } if id == "gain_minus_3"
        ));
    }

    #[test]
    fn command_rendering_is_deterministic() {
        let manifest = GoldenManifest::parse_toml(VALID_MANIFEST).unwrap();
        let case = manifest.get("gain_minus_3_mono").unwrap();

        assert_eq!(
            case.render_auralis_command("auralis", "/tmp/in.wav", "/tmp/out.wav"),
            [
                "auralis",
                "run",
                "/tmp/in.wav",
                "/tmp/out.wav",
                "--gain-db",
                "-3"
            ],
        );
        assert_eq!(
            case.render_sox_ng_command("sox_ng", "/tmp/in.wav", "/tmp/out.wav"),
            [
                "sox_ng",
                "-R",
                "-D",
                "/tmp/in.wav",
                "/tmp/out.wav",
                "gain",
                "-3"
            ],
        );
    }

    #[test]
    fn command_line_rendering_quotes_and_escapes_deterministically() {
        let command = [
            "auralis",
            "run",
            "input file.wav",
            "quote\"and\\slash",
            "line\nbreak",
            "tab\tvalue",
            "",
        ];

        assert_eq!(
            render_command_line(command),
            "auralis run \"input file.wav\" \"quote\\\"and\\\\slash\" \"line\\nbreak\" \"tab\\tvalue\" \"\""
        );
        assert_eq!(quote_command_arg("safe/path-1.wav"), "safe/path-1.wav");
        assert_eq!(quote_command_arg("needs space"), "\"needs space\"");
    }

    #[test]
    fn golden_manifest_command_line_rendering_is_stable_across_runs() {
        let manifest = GoldenManifest::parse_toml(
            r#"
            [id.quoted_paths]
            input = "fixtures/input file.wav"
            auralis = ["--gain-db", "-3"]
            sox_ng = ["gain", "-3"]
            max_abs = 0.0001
            rms = 0.000001
            snr_db = 90.0
            "#,
        )
        .unwrap();
        let case = manifest.get("quoted_paths").unwrap();

        let first = case.render_auralis_command_line(
            "auralis",
            "fixtures/input file.wav",
            "tmp/output file.wav",
        );
        let second = case.render_auralis_command_line(
            "auralis",
            "fixtures/input file.wav",
            "tmp/output file.wav",
        );

        assert_eq!(first, second);
        assert_eq!(
            first,
            "auralis run \"fixtures/input file.wav\" \"tmp/output file.wav\" --gain-db -3"
        );
        assert_eq!(
            case.render_sox_ng_command_line("sox_ng", "fixtures/input file.wav", "tmp/out.wav"),
            "sox_ng -R -D \"fixtures/input file.wav\" tmp/out.wav gain -3"
        );
    }

    fn assert_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }
}
