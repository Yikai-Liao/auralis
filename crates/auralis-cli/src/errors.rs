use std::path::PathBuf;

use auralis_wav::WavError;

use crate::{PathRole, spec};

#[derive(Debug)]
pub(crate) enum CliError {
    Auralis(auralis::Error),
    ChainParse(auralis::EffectChainParseError),
    EffectName(auralis::EffectNameError),
    EffectsFile(auralis::EffectsFileReadError),
    GraphSpec(spec::GraphSpecError),
    Json(serde_json::Error),
    Io(std::io::Error),
    Wav(WavError),
    EmptyEffectSpec,
    InvalidEffectSpec {
        spec: String,
    },
    MissingEffectSpec,
    MixedCheckInputs,
    MixedEffectInputs,
    MixedGuardAndNorm,
    DitherSeedWithoutDither,
    CacheClearNeedsConfirmation,
    NoAutoChannelsWithoutOutputChannels,
    NoAutoRateWithoutOutputRate,
    LockedRequiresSpec,
    GraphSpecNeedsFormatting {
        path: PathBuf,
    },
    UnknownManTopic {
        topic: String,
    },
    UnknownExplainTarget {
        target: String,
    },
    UnsupportedGraphRunShape,
    UnsupportedGraphNodeInputs {
        node: String,
        inputs: Vec<String>,
    },
    UnsupportedGraphNodeOp {
        op: String,
        reason: String,
    },
    #[allow(dead_code)]
    UnsupportedGraphChainInput {
        chain: String,
        input: String,
    },
    UnsupportedGraphSink {
        sink: String,
        input: String,
    },
    UnsupportedConvertInputFormat {
        path: PathBuf,
    },
    UnsupportedConvertOutputFormat {
        path: PathBuf,
    },
    UnsupportedFormat {
        path: PathBuf,
        role: PathRole,
    },
    WavSampleFormatRequiresWavOutput,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auralis(error) => write!(formatter, "{error}"),
            Self::ChainParse(error) => write!(formatter, "{error}"),
            Self::EffectName(error) => write!(formatter, "{error}"),
            Self::EffectsFile(error) => write!(formatter, "{error}"),
            Self::GraphSpec(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Wav(error) => write!(formatter, "{error}"),
            Self::EmptyEffectSpec => formatter.write_str("effect input requires a non-empty effect"),
            Self::InvalidEffectSpec { spec } => write!(
                formatter,
                "effect string `{spec}` contains unmatched shell quoting"
            ),
            Self::MissingEffectSpec => {
                formatter.write_str("one of SPEC, --fx, --chain, or --effects-file is required")
            }
            Self::MixedCheckInputs => {
                formatter.write_str("check accepts either SPEC or one effect input mode")
            }
            Self::MixedEffectInputs => {
                formatter.write_str("--fx, --chain, and --effects-file are mutually exclusive")
            }
            Self::MixedGuardAndNorm => formatter.write_str("--guard cannot be combined with --norm"),
            Self::DitherSeedWithoutDither => formatter.write_str("--dither-seed requires --dither"),
            Self::CacheClearNeedsConfirmation => {
                formatter.write_str("cache clear requires --yes to remove files")
            }
            Self::NoAutoChannelsWithoutOutputChannels => {
                formatter.write_str("--no-auto-channels requires --channels")
            }
            Self::NoAutoRateWithoutOutputRate => {
                formatter.write_str("--no-auto-rate requires --rate")
            }
            Self::LockedRequiresSpec => formatter.write_str("--locked requires a graph spec input"),
            Self::GraphSpecNeedsFormatting { path } => write!(
                formatter,
                "graph spec {} is not formatted; run `auralis fmt {}`",
                path.display(),
                path.display()
            ),
            Self::UnknownManTopic { topic } => {
                write!(formatter, "no built-in manual page for `{topic}`")
            }
            Self::UnknownExplainTarget { target } => write!(
                formatter,
                "spec does not define a source, chain, node, or sink named `{target}`"
            ),
            Self::UnsupportedGraphRunShape => formatter.write_str(
                "run currently supports source-to-chain-to-sink graph specs only; use `plan` to inspect unsupported nodes",
            ),
            Self::UnsupportedGraphNodeInputs { node, inputs } => write!(
                formatter,
                "node `{node}` cannot be run with inputs [{}]; only single-input passthrough nodes are supported",
                inputs.join(", ")
            ),
            Self::UnsupportedGraphNodeOp { op, reason } => write!(
                formatter,
                "node op `{op}` is not supported by the current graph runner: {reason}"
            ),
            Self::UnsupportedGraphChainInput { chain, input } => write!(
                formatter,
                "chain `{chain}` cannot be run from unsupported input `{input}`"
            ),
            Self::UnsupportedGraphSink { sink, input } => write!(
                formatter,
                "sink `{sink}` cannot be run from unsupported input `{input}`"
            ),
            Self::UnsupportedConvertInputFormat { path } => write!(
                formatter,
                "unsupported convert input format for {}; supported inputs are wav, flac, au, and snd",
                path.display()
            ),
            Self::UnsupportedConvertOutputFormat { path } => write!(
                formatter,
                "unsupported convert output format for {}; supported outputs are wav, flac, aiff, aif, aifc, au, and snd",
                path.display()
            ),
            Self::UnsupportedFormat { path, role } => {
                write!(
                    formatter,
                    "unsupported {role} format for {}; only PCM16 WAV is supported",
                    path.display()
                )
            }
            Self::WavSampleFormatRequiresWavOutput => {
                formatter.write_str("--sample is supported only for WAV output")
            }
        }
    }
}

impl From<auralis::Error> for CliError {
    fn from(error: auralis::Error) -> Self {
        Self::Auralis(error)
    }
}

impl From<auralis::EffectChainParseError> for CliError {
    fn from(error: auralis::EffectChainParseError) -> Self {
        Self::ChainParse(error)
    }
}

impl From<auralis::EffectNameError> for CliError {
    fn from(error: auralis::EffectNameError) -> Self {
        Self::EffectName(error)
    }
}

impl From<auralis::EffectsFileReadError> for CliError {
    fn from(error: auralis::EffectsFileReadError) -> Self {
        Self::EffectsFile(error)
    }
}

impl From<spec::GraphSpecError> for CliError {
    fn from(error: spec::GraphSpecError) -> Self {
        Self::GraphSpec(error)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<WavError> for CliError {
    fn from(error: WavError) -> Self {
        Self::Wav(error)
    }
}
