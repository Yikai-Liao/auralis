//! Golden-test manifest parsing and deterministic command rendering.
//!
//! Golden manifests are small TOML documents that describe one or more
//! reference comparisons. Each case lives under the `id` table; the table key is
//! the stable case identifier used in test reports and failure artifacts.
//! Command vectors are rendered separately from command-line displays: vectors
//! remain suitable for `std::process::Command`, while display helpers apply
//! deterministic quoting and escaping for human-readable failure reports.
//!

use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::corpus::is_known_corpus_id;

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
    /// `snr_db`. Cases may also define `output_channels`,
    /// `output_sample_rate`, `sox_ng_auto_rate = true`, and
    /// `sox_ng_auto_channels = true` to record SoX-ng output-channel options
    /// and output-rate options that auto-insert its `channels` and `rate`
    /// effects.
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
    inputs: Vec<PathBuf>,
    corpus_ids: Vec<String>,
    combine: Option<String>,
    output_channels: Option<u16>,
    output_sample_rate: Option<u32>,
    sox_ng_auto_channels: bool,
    sox_ng_auto_rate: bool,
    auralis: Vec<String>,
    sox_ng: Vec<String>,
    tolerance: GoldenTolerance,
}

impl GoldenCase {
    /// Returns the first input fixture path recorded by the manifest.
    ///
    /// The path is usually relative to the test corpus root. Callers choose
    /// whether to join it to a temporary fixture directory or use it directly.
    #[must_use]
    pub fn input(&self) -> &Path {
        &self.inputs[0]
    }

    /// Returns all input fixture paths recorded by the manifest.
    ///
    /// Single-input cases use the legacy `input = "..."` TOML field and
    /// multi-input combiner cases use `inputs = ["...", "..."]`. This accessor
    /// normalizes both forms into a deterministic non-empty list.
    #[must_use]
    pub fn inputs(&self) -> &[PathBuf] {
        &self.inputs
    }

    /// Returns the first stable corpus identifier recorded by the manifest, if
    /// present.
    ///
    /// Single-input cases use `corpus_id = "..."`; multi-input cases use
    /// `corpus_ids = ["...", "..."]`. The manifest still records file names
    /// separately so command rendering and fixture paths stay stable.
    #[must_use]
    pub fn corpus_id(&self) -> Option<&str> {
        self.corpus_ids.first().map(String::as_str)
    }

    /// Returns all stable corpus identifiers recorded by the manifest.
    #[must_use]
    pub fn corpus_ids(&self) -> &[String] {
        &self.corpus_ids
    }

    /// Returns the requested multi-input combiner method, if explicitly set.
    ///
    /// Multi-input manifests that omit this field retain the historical
    /// `concatenate` rendering default.
    #[must_use]
    pub fn combine_method(&self) -> Option<&str> {
        self.combine.as_deref()
    }

    /// Returns the requested output channel count, if the manifest sets one.
    ///
    /// Single-output cases use this to render Auralis `--channels <N>` and
    /// SoX-ng `--channels <N>` output options. When
    /// [`Self::sox_ng_auto_channels_inserted`] is true, the manifest is
    /// explicitly recording a SoX-ng automatic `channels` effect.
    #[must_use]
    pub const fn output_channels(&self) -> Option<u16> {
        self.output_channels
    }

    /// Returns the requested output sample rate, if the manifest sets one.
    ///
    /// Single-output cases use this to render Auralis `--rate <N>` and SoX-ng
    /// `--rate <N>` output options. When
    /// [`Self::sox_ng_auto_rate_inserted`] is true, the manifest is explicitly
    /// recording a SoX-ng automatic `rate` effect.
    #[must_use]
    pub const fn output_sample_rate(&self) -> Option<u32> {
        self.output_sample_rate
    }

    /// Returns whether the SoX-ng command relies on auto-inserted `channels`.
    #[must_use]
    pub const fn sox_ng_auto_channels_inserted(&self) -> bool {
        self.sox_ng_auto_channels
    }

    /// Returns whether the SoX-ng command relies on auto-inserted `rate`.
    #[must_use]
    pub const fn sox_ng_auto_rate_inserted(&self) -> bool {
        self.sox_ng_auto_rate
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
        self.render_auralis_command_with_inputs(executable, [input_path], output_path)
    }

    /// Renders a deterministic Auralis command vector for one or more inputs.
    ///
    /// Multi-input cases render through Auralis' current combine CLI shape:
    /// the first input remains positional, later inputs are passed as repeated
    /// `--input <FILE>` options, and `--combine <METHOD>` is emitted before
    /// recorded effect arguments. Cases that omit `combine` default to
    /// `concatenate` for backwards compatibility.
    #[must_use]
    pub fn render_auralis_command_with_inputs<I, P>(
        &self,
        executable: impl AsRef<str>,
        input_paths: I,
        output_path: impl AsRef<Path>,
    ) -> Vec<String>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let input_paths = input_paths
            .into_iter()
            .map(|path| path_to_command_arg(path.as_ref()))
            .collect::<Vec<_>>();
        let mut command = vec![executable.as_ref().to_owned(), "run".to_owned()];
        if let Some(first_input) = input_paths.first() {
            command.push(first_input.clone());
        }
        command.push(path_to_command_arg(output_path.as_ref()));
        if input_paths.len() > 1 {
            command.push("--combine".to_owned());
            command.push(self.rendered_combine_method().to_owned());
            for input in input_paths.iter().skip(1) {
                command.push("--input".to_owned());
                command.push(input.clone());
            }
        }
        if let Some(output_channels) = self.output_channels {
            command.push("--channels".to_owned());
            command.push(output_channels.to_string());
        }
        if let Some(output_sample_rate) = self.output_sample_rate {
            command.push("--rate".to_owned());
            command.push(output_sample_rate.to_string());
        }
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

    /// Renders a deterministic display form of a multi-input Auralis command.
    ///
    /// This is intended for failure reports and logs and follows the same
    /// stable quoting rules as [`Self::render_auralis_command_line`].
    #[must_use]
    pub fn render_auralis_command_line_with_inputs<I, P>(
        &self,
        executable: impl AsRef<str>,
        input_paths: I,
        output_path: impl AsRef<Path>,
    ) -> String
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        render_command_line(self.render_auralis_command_with_inputs(
            executable,
            input_paths,
            output_path,
        ))
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
        self.render_sox_ng_command_with_inputs(executable, [input_path], output_path)
    }

    /// Renders a deterministic SoX-ng command vector for one or more inputs.
    ///
    /// Multi-input cases include `--combine <METHOD>` before the input paths.
    /// Cases that omit `combine` default to `concatenate`. The `-R` and `-D`
    /// flags are always included to match Auralis' repeatable golden-test
    /// policy.
    #[must_use]
    pub fn render_sox_ng_command_with_inputs<I, P>(
        &self,
        executable: impl AsRef<str>,
        input_paths: I,
        output_path: impl AsRef<Path>,
    ) -> Vec<String>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let input_paths = input_paths
            .into_iter()
            .map(|path| path_to_command_arg(path.as_ref()))
            .collect::<Vec<_>>();
        let mut command = vec![
            executable.as_ref().to_owned(),
            "-R".to_owned(),
            "-D".to_owned(),
        ];
        if input_paths.len() > 1 {
            command.push("--combine".to_owned());
            command.push(self.rendered_combine_method().to_owned());
        }
        command.extend(input_paths);
        if let Some(output_channels) = self.output_channels {
            command.push("--channels".to_owned());
            command.push(output_channels.to_string());
        }
        if let Some(output_sample_rate) = self.output_sample_rate {
            command.push("--rate".to_owned());
            command.push(output_sample_rate.to_string());
        }
        command.push(path_to_command_arg(output_path.as_ref()));
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

    /// Renders a deterministic display form of a multi-input SoX-ng command.
    ///
    /// This is intended for failure reports and logs and follows the same
    /// repeatability and quoting rules as [`Self::render_sox_ng_command_line`].
    #[must_use]
    pub fn render_sox_ng_command_line_with_inputs<I, P>(
        &self,
        executable: impl AsRef<str>,
        input_paths: I,
        output_path: impl AsRef<Path>,
    ) -> String
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        render_command_line(self.render_sox_ng_command_with_inputs(
            executable,
            input_paths,
            output_path,
        ))
    }

    fn from_raw(id: &str, raw: RawGoldenCase) -> Result<Self> {
        let inputs = validate_inputs(id, raw.input, raw.inputs)?;
        let corpus_ids = validate_corpus_ids(id, inputs.len(), raw.corpus_id, raw.corpus_ids)?;
        validate_combine_method(id, raw.combine.as_deref())?;
        validate_output_channels(id, raw.output_channels)?;
        validate_output_sample_rate(id, raw.output_sample_rate)?;
        validate_sox_ng_auto_channels(id, raw.sox_ng_auto_channels, raw.output_channels)?;
        validate_sox_ng_auto_rate(id, raw.sox_ng_auto_rate, raw.output_sample_rate)?;

        validate_args(id, GoldenCommand::Auralis, &raw.auralis)?;
        validate_args(id, GoldenCommand::SoxNg, &raw.sox_ng)?;
        validate_tolerance(id, GoldenMetric::MaxAbs, raw.max_abs)?;
        validate_tolerance(id, GoldenMetric::Rms, raw.rms)?;
        validate_tolerance(id, GoldenMetric::SnrDb, raw.snr_db)?;

        Ok(Self {
            inputs,
            corpus_ids,
            combine: raw.combine,
            output_channels: raw.output_channels,
            output_sample_rate: raw.output_sample_rate,
            sox_ng_auto_channels: raw.sox_ng_auto_channels,
            sox_ng_auto_rate: raw.sox_ng_auto_rate,
            auralis: raw.auralis,
            sox_ng: raw.sox_ng,
            tolerance: GoldenTolerance {
                max_abs: raw.max_abs,
                rms: raw.rms,
                snr_db: raw.snr_db,
            },
        })
    }

    fn rendered_combine_method(&self) -> &str {
        self.combine.as_deref().unwrap_or("concatenate")
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

    /// A case did not define either `input` or `inputs`.
    MissingInput {
        /// Case identifier missing input fields.
        id: String,
    },

    /// A case defined both `input` and `inputs`.
    AmbiguousInput {
        /// Case identifier containing both input fields.
        id: String,
    },

    /// A case defined both `corpus_id` and `corpus_ids`.
    AmbiguousCorpusId {
        /// Case identifier containing both corpus identifier fields.
        id: String,
    },

    /// A case defined an empty or unknown corpus identifier.
    InvalidCorpusId {
        /// Case identifier containing the invalid corpus identifier.
        id: String,

        /// Rejected corpus identifier.
        corpus_id: String,
    },

    /// A case recorded a corpus identifier count that does not match inputs.
    CorpusIdInputCountMismatch {
        /// Case identifier containing mismatched corpus IDs.
        id: String,

        /// Number of manifest inputs.
        inputs: usize,

        /// Number of manifest corpus IDs.
        corpus_ids: usize,
    },

    /// A case requested an unknown multi-input combiner method.
    InvalidCombineMethod {
        /// Case identifier containing the invalid combiner method.
        id: String,

        /// Rejected combiner method.
        combine: String,
    },

    /// A case requested an invalid output channel count.
    InvalidOutputChannels {
        /// Case identifier containing the invalid output channel count.
        id: String,

        /// Rejected output channel count.
        channels: u16,
    },

    /// A case requested an invalid output sample rate.
    InvalidOutputSampleRate {
        /// Case identifier containing the invalid output sample rate.
        id: String,

        /// Rejected output sample rate.
        sample_rate: u32,
    },

    /// A case recorded SoX-ng automatic channels without an output channel target.
    AutoChannelsWithoutOutputChannels {
        /// Case identifier missing `output_channels`.
        id: String,
    },

    /// A case recorded SoX-ng automatic rate without an output sample-rate target.
    AutoRateWithoutOutputSampleRate {
        /// Case identifier missing `output_sample_rate`.
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
            Self::MissingInput { id } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` must define `input` or `inputs`"
                )
            }
            Self::AmbiguousInput { id } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` must not define both `input` and `inputs`"
                )
            }
            Self::AmbiguousCorpusId { id } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` must not define both `corpus_id` and `corpus_ids`"
                )
            }
            Self::InvalidCorpusId { id, corpus_id } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` has invalid corpus id `{corpus_id}`"
                )
            }
            Self::CorpusIdInputCountMismatch {
                id,
                inputs,
                corpus_ids,
            } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` has {inputs} inputs but {corpus_ids} corpus ids"
                )
            }
            Self::InvalidCombineMethod { id, combine } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` has invalid combine method `{combine}`"
                )
            }
            Self::InvalidOutputChannels { id, channels } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` has invalid output channel count `{channels}`"
                )
            }
            Self::InvalidOutputSampleRate { id, sample_rate } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` has invalid output sample rate `{sample_rate}`"
                )
            }
            Self::AutoChannelsWithoutOutputChannels { id } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` records SoX-ng automatic channels without `output_channels`"
                )
            }
            Self::AutoRateWithoutOutputSampleRate { id } => {
                write!(
                    formatter,
                    "golden manifest case `{id}` records SoX-ng automatic rate without `output_sample_rate`"
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
            | Self::MissingInput { .. }
            | Self::AmbiguousInput { .. }
            | Self::AmbiguousCorpusId { .. }
            | Self::InvalidCorpusId { .. }
            | Self::CorpusIdInputCountMismatch { .. }
            | Self::InvalidCombineMethod { .. }
            | Self::InvalidOutputChannels { .. }
            | Self::InvalidOutputSampleRate { .. }
            | Self::AutoChannelsWithoutOutputChannels { .. }
            | Self::AutoRateWithoutOutputSampleRate { .. }
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
    input: Option<String>,
    inputs: Option<Vec<String>>,
    corpus_id: Option<String>,
    corpus_ids: Option<Vec<String>>,
    combine: Option<String>,
    output_channels: Option<u16>,
    output_sample_rate: Option<u32>,
    #[serde(default)]
    sox_ng_auto_channels: bool,
    #[serde(default)]
    sox_ng_auto_rate: bool,
    auralis: Vec<String>,
    sox_ng: Vec<String>,
    max_abs: f64,
    rms: f64,
    snr_db: f64,
}

const SUPPORTED_COMBINE_METHODS: &[&str] = &[
    "concatenate",
    "sequence",
    "mix",
    "mix-power",
    "merge",
    "multiply",
];

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

fn validate_inputs(
    id: &str,
    input: Option<String>,
    inputs: Option<Vec<String>>,
) -> Result<Vec<PathBuf>> {
    match (input, inputs) {
        (Some(_), Some(_)) => Err(GoldenManifestError::AmbiguousInput { id: id.to_owned() }),
        (None, None) => Err(GoldenManifestError::MissingInput { id: id.to_owned() }),
        (Some(input), None) => {
            if input.is_empty() {
                Err(GoldenManifestError::EmptyInput { id: id.to_owned() })
            } else {
                Ok(vec![input.into()])
            }
        }
        (None, Some(inputs)) => {
            if inputs.is_empty() || inputs.iter().any(String::is_empty) {
                Err(GoldenManifestError::EmptyInput { id: id.to_owned() })
            } else {
                Ok(inputs.into_iter().map(PathBuf::from).collect())
            }
        }
    }
}

fn validate_corpus_ids(
    id: &str,
    input_count: usize,
    corpus_id: Option<String>,
    corpus_ids: Option<Vec<String>>,
) -> Result<Vec<String>> {
    let corpus_ids = match (corpus_id, corpus_ids) {
        (Some(_), Some(_)) => {
            return Err(GoldenManifestError::AmbiguousCorpusId { id: id.to_owned() });
        }
        (None, None) => return Ok(Vec::new()),
        (Some(corpus_id), None) => vec![corpus_id],
        (None, Some(corpus_ids)) => corpus_ids,
    };

    if corpus_ids.len() != input_count {
        return Err(GoldenManifestError::CorpusIdInputCountMismatch {
            id: id.to_owned(),
            inputs: input_count,
            corpus_ids: corpus_ids.len(),
        });
    }
    if let Some(corpus_id) = corpus_ids
        .iter()
        .find(|corpus_id| corpus_id.is_empty() || !is_known_corpus_id(corpus_id))
    {
        return Err(GoldenManifestError::InvalidCorpusId {
            id: id.to_owned(),
            corpus_id: corpus_id.clone(),
        });
    }

    Ok(corpus_ids)
}

fn validate_combine_method(id: &str, combine: Option<&str>) -> Result<()> {
    let Some(combine) = combine else {
        return Ok(());
    };

    if SUPPORTED_COMBINE_METHODS.contains(&combine) {
        Ok(())
    } else {
        Err(GoldenManifestError::InvalidCombineMethod {
            id: id.to_owned(),
            combine: combine.to_owned(),
        })
    }
}

fn validate_output_channels(id: &str, output_channels: Option<u16>) -> Result<()> {
    match output_channels {
        Some(0) => Err(GoldenManifestError::InvalidOutputChannels {
            id: id.to_owned(),
            channels: 0,
        }),
        Some(_) | None => Ok(()),
    }
}

fn validate_output_sample_rate(id: &str, output_sample_rate: Option<u32>) -> Result<()> {
    match output_sample_rate {
        Some(0) => Err(GoldenManifestError::InvalidOutputSampleRate {
            id: id.to_owned(),
            sample_rate: 0,
        }),
        Some(_) | None => Ok(()),
    }
}

fn validate_sox_ng_auto_channels(
    id: &str,
    sox_ng_auto_channels: bool,
    output_channels: Option<u16>,
) -> Result<()> {
    if sox_ng_auto_channels && output_channels.is_none() {
        Err(GoldenManifestError::AutoChannelsWithoutOutputChannels { id: id.to_owned() })
    } else {
        Ok(())
    }
}

fn validate_sox_ng_auto_rate(
    id: &str,
    sox_ng_auto_rate: bool,
    output_sample_rate: Option<u32>,
) -> Result<()> {
    if sox_ng_auto_rate && output_sample_rate.is_none() {
        Err(GoldenManifestError::AutoRateWithoutOutputSampleRate { id: id.to_owned() })
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
