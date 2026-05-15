use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    path::{Path, PathBuf},
};

use crate::effect_tokens;
use serde::{Deserialize, Serialize};

const LOCK_VERSION: &str = "auralis.lock/v1";

pub fn check_graph_spec(path: &Path) -> Result<CheckedGraphSpec, GraphSpecError> {
    let source = fs::read_to_string(path).map_err(|error| GraphSpecError::Read {
        path: path.to_path_buf(),
        error,
    })?;
    let spec = parse_graph_spec(&source)?;

    spec.validate()
}

pub fn format_graph_spec(path: &Path) -> Result<String, GraphSpecError> {
    let source = fs::read_to_string(path).map_err(|error| GraphSpecError::Read {
        path: path.to_path_buf(),
        error,
    })?;
    format_graph_spec_source(&source)
}

pub fn format_graph_spec_source(source: &str) -> Result<String, GraphSpecError> {
    let spec = parse_graph_spec(source)?;
    spec.clone().validate()?;
    toml::to_string_pretty(&spec).map_err(GraphSpecError::TomlSerialize)
}

pub fn sync_graph_lock(path: &Path) -> Result<(), GraphSpecError> {
    let graph_lock = build_graph_lock(path)?;
    let lock_path = graph_lock_path(path);
    let lock_toml = toml::to_string_pretty(&graph_lock).map_err(GraphSpecError::LockSerialize)?;
    fs::write(&lock_path, lock_toml).map_err(|error| GraphSpecError::LockWrite {
        path: lock_path,
        error,
    })
}

pub fn verify_graph_lock(path: &Path) -> Result<(), GraphSpecError> {
    let expected = build_graph_lock(path)?;
    let lock_path = graph_lock_path(path);
    let source = match fs::read_to_string(&lock_path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(GraphSpecError::MissingLock {
                path: lock_path,
                spec_path: path.to_path_buf(),
            });
        }
        Err(error) => {
            return Err(GraphSpecError::LockRead {
                path: lock_path,
                error,
            });
        }
    };
    let actual = toml::from_str::<GraphLock>(&source).map_err(GraphSpecError::LockToml)?;
    if actual != expected {
        return Err(GraphSpecError::LockMismatch {
            path: lock_path,
            spec_path: path.to_path_buf(),
        });
    }
    Ok(())
}

pub fn validate_convert_graph_export(
    no_auto_channels: bool,
    no_auto_rate: bool,
    guard: bool,
    container: Option<crate::executor::OutputContainer>,
    sample: Option<auralis::WavSampleFormat>,
) -> Result<(), GraphSpecError> {
    if no_auto_channels || no_auto_rate || guard || container.is_some() || sample.is_some() {
        return Err(GraphSpecError::UnsupportedConvertExportPolicy);
    }
    Ok(())
}

pub fn write_convert_graph_spec(
    path: &Path,
    input: &Path,
    output: &Path,
    output_channels: Option<auralis::ChannelCount>,
    output_sample_rate: Option<auralis::SampleRate>,
    norm: Option<f64>,
) -> Result<(), GraphSpecError> {
    let mut steps = Vec::new();
    if let Some(channels) = output_channels {
        steps.push(ChainStepSpec::with_param(
            "channels",
            "count",
            channels.as_u16().to_string(),
        ));
    }
    if let Some(rate) = output_sample_rate {
        steps.push(ChainStepSpec::with_param(
            "rate",
            "frequency",
            rate.as_u32().to_string(),
        ));
    }
    if let Some(target) = norm {
        steps.push(ChainStepSpec::with_param(
            "norm.peak",
            "target",
            format!("{target}dBFS"),
        ));
    }

    let sink_input = if steps.is_empty() {
        "input.audio"
    } else {
        "convert.audio"
    };
    let spec = GraphSpec {
        version: "auralis.graph/v1".to_owned(),
        name: Some("convert".to_owned()),
        sources: vec![SourceSpec {
            id: "input".to_owned(),
            path: input.to_path_buf(),
        }],
        chains: if steps.is_empty() {
            Vec::new()
        } else {
            vec![ChainSpec {
                id: "convert".to_owned(),
                input: "input.audio".to_owned(),
                steps,
            }]
        },
        nodes: Vec::new(),
        sinks: vec![SinkSpec {
            id: "output".to_owned(),
            input: sink_input.to_owned(),
            path: output.to_path_buf(),
        }],
    };
    let source = toml::to_string_pretty(&spec).map_err(GraphSpecError::TomlSerialize)?;
    fs::write(path, source).map_err(|error| GraphSpecError::Write {
        path: path.to_path_buf(),
        error,
    })
}

fn parse_graph_spec(source: &str) -> Result<GraphSpec, GraphSpecError> {
    toml::from_str::<GraphSpec>(source).map_err(GraphSpecError::Toml)
}

fn build_graph_lock(path: &Path) -> Result<GraphLock, GraphSpecError> {
    let source = fs::read_to_string(path).map_err(|error| GraphSpecError::Read {
        path: path.to_path_buf(),
        error,
    })?;
    let spec = parse_graph_spec(&source)?;
    let checked = spec.clone().validate()?;
    let formatted = toml::to_string_pretty(&spec).map_err(GraphSpecError::TomlSerialize)?;

    Ok(GraphLock {
        version: LOCK_VERSION.to_owned(),
        spec_version: spec.version,
        auralis_version: env!("CARGO_PKG_VERSION").to_owned(),
        backend: "scalar".to_owned(),
        semantic_hash: stable_hash_hex(&formatted),
        operations: checked
            .chains
            .iter()
            .flat_map(|chain| {
                chain
                    .step_ids
                    .iter()
                    .zip(chain.step_labels.iter())
                    .map(|(id, label)| LockedOperation {
                        id: id.clone(),
                        op: label.clone(),
                    })
            })
            .collect(),
        outputs: checked
            .sinks
            .iter()
            .map(|sink| LockedOutput {
                id: sink.id.clone(),
                input: sink.input.clone(),
                path: sink.path.display().to_string(),
            })
            .collect(),
    })
}

fn graph_lock_path(path: &Path) -> PathBuf {
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join("Auralis.lock")
}

fn stable_hash_hex(source: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct GraphLock {
    version: String,
    spec_version: String,
    auralis_version: String,
    backend: String,
    semantic_hash: String,
    operations: Vec<LockedOperation>,
    outputs: Vec<LockedOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct LockedOperation {
    id: String,
    op: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct LockedOutput {
    id: String,
    input: String,
    path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedGraphSpec {
    pub name: Option<String>,
    pub source_count: usize,
    pub chain_count: usize,
    pub node_count: usize,
    pub sink_count: usize,
    pub sources: Vec<CheckedSource>,
    pub chains: Vec<CheckedChain>,
    pub nodes: Vec<CheckedNode>,
    pub sinks: Vec<CheckedSink>,
    pub expanded_step_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedSource {
    pub id: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedChain {
    pub id: String,
    pub input: String,
    pub step_ids: Vec<String>,
    pub step_labels: Vec<String>,
    pub effect_tokens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedNode {
    pub id: String,
    pub op: Option<String>,
    pub inputs: Vec<String>,
    pub input_gains: Vec<Option<String>>,
    pub params: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedSink {
    pub id: String,
    pub input: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GraphSpec {
    version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sources: Vec<SourceSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    chains: Vec<ChainSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    nodes: Vec<NodeSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sinks: Vec<SinkSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SourceSpec {
    id: String,
    path: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ChainSpec {
    id: String,
    input: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    steps: Vec<ChainStepSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ChainStepSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    op: String,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    params: BTreeMap<String, toml::Value>,
}

impl ChainStepSpec {
    fn with_param(op: &str, param: &str, value: String) -> Self {
        let mut params = BTreeMap::new();
        params.insert(param.to_owned(), toml::Value::String(value));
        Self {
            id: None,
            op: op.to_owned(),
            params,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NodeSpec {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    op: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    input: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    inputs: Vec<NodeInputSpec>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    params: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NodeInputSpec {
    from: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gain: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SinkSpec {
    id: String,
    input: String,
    path: PathBuf,
}

impl GraphSpec {
    fn validate(self) -> Result<CheckedGraphSpec, GraphSpecError> {
        if self.version != "auralis.graph/v1" {
            return Err(GraphSpecError::UnsupportedVersion {
                version: self.version,
            });
        }

        let mut seen_ids = BTreeMap::new();
        for source in &self.sources {
            ensure_non_empty_id(&source.id, "source")?;
            register_id(&mut seen_ids, &source.id, "source")?;
            if source.path.as_os_str().is_empty() {
                return Err(GraphSpecError::EmptyPath {
                    kind: "source",
                    id: source.id.clone(),
                });
            }
        }
        for chain in &self.chains {
            ensure_non_empty_id(&chain.id, "chain")?;
            register_id(&mut seen_ids, &chain.id, "chain")?;
        }
        for node in &self.nodes {
            ensure_non_empty_id(&node.id, "node")?;
            register_id(&mut seen_ids, &node.id, "node")?;
        }
        for sink in &self.sinks {
            ensure_non_empty_id(&sink.id, "sink")?;
            register_id(&mut seen_ids, &sink.id, "sink")?;
            if sink.path.as_os_str().is_empty() {
                return Err(GraphSpecError::EmptyPath {
                    kind: "sink",
                    id: sink.id.clone(),
                });
            }
        }

        let available_ports = self
            .sources
            .iter()
            .map(|source| source.id.clone())
            .chain(self.chains.iter().map(|chain| chain.id.clone()))
            .chain(self.nodes.iter().map(|node| node.id.clone()))
            .map(|id| format!("{id}.audio"))
            .collect::<BTreeSet<_>>();

        let (checked_chains, expanded_step_ids) = validate_chains(&self.chains, &available_ports)?;

        for node in &self.nodes {
            match (&node.input, node.inputs.is_empty()) {
                (Some(_), false) => {
                    return Err(GraphSpecError::AmbiguousNodeInputs {
                        id: node.id.clone(),
                    });
                }
                (None, true) => {
                    return Err(GraphSpecError::MissingNodeInput {
                        id: node.id.clone(),
                    });
                }
                (Some(input), true) => {
                    ensure_known_port(input, "node", &node.id, &available_ports)?;
                }
                (None, false) => {
                    for input in &node.inputs {
                        ensure_known_port(&input.from, "node", &node.id, &available_ports)?;
                    }
                }
            }
        }

        for sink in &self.sinks {
            ensure_known_port(&sink.input, "sink", &sink.id, &available_ports)?;
        }

        Ok(CheckedGraphSpec {
            name: self.name,
            source_count: self.sources.len(),
            chain_count: self.chains.len(),
            node_count: self.nodes.len(),
            sink_count: self.sinks.len(),
            sources: self
                .sources
                .into_iter()
                .map(|source| CheckedSource {
                    id: source.id,
                    path: source.path,
                })
                .collect(),
            chains: checked_chains,
            nodes: self
                .nodes
                .into_iter()
                .map(|node| {
                    let NodeSpec {
                        id,
                        op,
                        input,
                        inputs,
                        params,
                    } = node;
                    let input_gains = inputs.iter().map(|input| input.gain.clone()).collect();
                    let inputs = input
                        .into_iter()
                        .chain(inputs.into_iter().map(|input| input.from))
                        .collect();

                    CheckedNode {
                        id,
                        op,
                        inputs,
                        input_gains,
                        params,
                    }
                })
                .collect(),
            sinks: self
                .sinks
                .into_iter()
                .map(|sink| CheckedSink {
                    id: sink.id,
                    input: sink.input,
                    path: sink.path,
                })
                .collect(),
            expanded_step_ids,
        })
    }
}

fn validate_chains(
    chains: &[ChainSpec],
    available_ports: &BTreeSet<String>,
) -> Result<(Vec<CheckedChain>, Vec<String>), GraphSpecError> {
    let mut checked_chains = Vec::new();
    let mut expanded_step_ids = Vec::new();

    for chain in chains {
        ensure_known_port(&chain.input, "chain", &chain.id, available_ports)?;
        if chain.steps.is_empty() {
            return Err(GraphSpecError::EmptyChain {
                id: chain.id.clone(),
            });
        }

        let mut seen_step_ids = BTreeSet::new();
        let mut step_ids = Vec::new();
        let mut step_labels = Vec::new();
        let mut effect_tokens = Vec::new();
        for (index, step) in chain.steps.iter().enumerate() {
            if step.op.trim().is_empty() {
                return Err(GraphSpecError::EmptyStepOp {
                    chain_id: chain.id.clone(),
                    index,
                });
            }

            let expanded_id = step
                .id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map_or_else(
                    || format!("{}/{:02}-{}", chain.id, index + 1, step.op),
                    str::to_owned,
                );
            if !seen_step_ids.insert(expanded_id.clone()) {
                return Err(GraphSpecError::DuplicateChainStepId {
                    chain_id: chain.id.clone(),
                    step_id: expanded_id,
                });
            }
            let step_effect_tokens = chain_step_effect_tokens(chain, index, step)?;
            step_labels.push(step_effect_tokens.join(" "));
            effect_tokens.extend(step_effect_tokens);
            expanded_step_ids.push(expanded_id.clone());
            step_ids.push(expanded_id);
        }
        checked_chains.push(CheckedChain {
            id: chain.id.clone(),
            input: chain.input.clone(),
            step_ids,
            step_labels,
            effect_tokens,
        });
    }

    Ok((checked_chains, expanded_step_ids))
}

fn chain_step_effect_tokens(
    chain: &ChainSpec,
    index: usize,
    step: &ChainStepSpec,
) -> Result<Vec<String>, GraphSpecError> {
    effect_tokens::lower_graph_effect_tokens(&step.op, &step.params).map_err(|error| {
        GraphSpecError::InvalidStepParam {
            chain_id: chain.id.clone(),
            index,
            op: step.op.clone(),
            param: error.param(),
        }
    })
}

fn ensure_non_empty_id(id: &str, kind: &'static str) -> Result<(), GraphSpecError> {
    if id.trim().is_empty() {
        return Err(GraphSpecError::EmptyId { kind });
    }

    Ok(())
}

fn register_id(
    seen_ids: &mut BTreeMap<String, &'static str>,
    id: &str,
    kind: &'static str,
) -> Result<(), GraphSpecError> {
    if let Some(previous_kind) = seen_ids.insert(id.to_owned(), kind) {
        return Err(GraphSpecError::DuplicateId {
            id: id.to_owned(),
            first_kind: previous_kind,
            second_kind: kind,
        });
    }

    Ok(())
}

fn ensure_known_port(
    port: &str,
    kind: &'static str,
    id: &str,
    available_ports: &BTreeSet<String>,
) -> Result<(), GraphSpecError> {
    if !available_ports.contains(port) {
        return Err(GraphSpecError::UnknownInputPort {
            kind,
            id: id.to_owned(),
            port: port.to_owned(),
            available_ports: available_ports.iter().cloned().collect(),
        });
    }

    Ok(())
}

#[derive(Debug)]
pub enum GraphSpecError {
    Read {
        path: PathBuf,
        error: std::io::Error,
    },
    Write {
        path: PathBuf,
        error: std::io::Error,
    },
    Toml(toml::de::Error),
    TomlSerialize(toml::ser::Error),
    LockRead {
        path: PathBuf,
        error: std::io::Error,
    },
    LockWrite {
        path: PathBuf,
        error: std::io::Error,
    },
    LockToml(toml::de::Error),
    LockSerialize(toml::ser::Error),
    MissingLock {
        path: PathBuf,
        spec_path: PathBuf,
    },
    LockMismatch {
        path: PathBuf,
        spec_path: PathBuf,
    },
    UnsupportedVersion {
        version: String,
    },
    EmptyId {
        kind: &'static str,
    },
    DuplicateId {
        id: String,
        first_kind: &'static str,
        second_kind: &'static str,
    },
    EmptyPath {
        kind: &'static str,
        id: String,
    },
    EmptyChain {
        id: String,
    },
    EmptyStepOp {
        chain_id: String,
        index: usize,
    },
    DuplicateChainStepId {
        chain_id: String,
        step_id: String,
    },
    InvalidStepParam {
        chain_id: String,
        index: usize,
        op: String,
        param: &'static str,
    },
    AmbiguousNodeInputs {
        id: String,
    },
    MissingNodeInput {
        id: String,
    },
    UnknownInputPort {
        kind: &'static str,
        id: String,
        port: String,
        available_ports: Vec<String>,
    },
    UnsupportedConvertExportPolicy,
}

impl GraphSpecError {
    fn fmt_file_error(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, error } => {
                write!(
                    formatter,
                    "failed to read graph spec {}: {error}",
                    path.display()
                )
            }
            Self::Write { path, error } => {
                write!(
                    formatter,
                    "failed to write graph spec {}: {error}",
                    path.display()
                )
            }
            Self::Toml(error) => write!(formatter, "invalid graph spec TOML: {error}"),
            Self::TomlSerialize(error) => {
                write!(formatter, "failed to serialize graph spec TOML: {error}")
            }
            Self::LockRead { path, error } => {
                write!(
                    formatter,
                    "failed to read graph lock {}: {error}",
                    path.display()
                )
            }
            Self::LockWrite { path, error } => {
                write!(
                    formatter,
                    "failed to write graph lock {}: {error}",
                    path.display()
                )
            }
            Self::LockToml(error) => write!(formatter, "invalid graph lock TOML: {error}"),
            Self::LockSerialize(error) => {
                write!(formatter, "failed to serialize graph lock TOML: {error}")
            }
            Self::MissingLock { path, spec_path } => write!(
                formatter,
                "missing graph lock {}; run `auralis check {}` to create it",
                path.display(),
                spec_path.display()
            ),
            Self::LockMismatch { path, spec_path } => write!(
                formatter,
                "graph lock {} is stale; run `auralis check {}` to refresh it",
                path.display(),
                spec_path.display()
            ),
            Self::UnsupportedVersion { version } => write!(
                formatter,
                "unsupported graph spec version `{version}`; expected `auralis.graph/v1`"
            ),
            _ => unreachable!("validation errors are formatted separately"),
        }
    }

    fn fmt_validation_error(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId { kind } => write!(formatter, "{kind} id cannot be empty"),
            Self::DuplicateId {
                id,
                first_kind,
                second_kind,
            } => write!(
                formatter,
                "duplicate graph id `{id}` is used by both a {first_kind} and a {second_kind}"
            ),
            Self::EmptyPath { kind, id } => {
                write!(formatter, "{kind} `{id}` requires a non-empty path")
            }
            Self::EmptyChain { id } => {
                write!(formatter, "chain `{id}` requires at least one step")
            }
            Self::EmptyStepOp { chain_id, index } => write!(
                formatter,
                "chain `{chain_id}` step {} requires a non-empty `op`",
                index + 1
            ),
            Self::DuplicateChainStepId { chain_id, step_id } => write!(
                formatter,
                "chain `{chain_id}` expands to duplicate step id `{step_id}`"
            ),
            Self::InvalidStepParam {
                chain_id,
                index,
                op,
                param,
            } => write!(
                formatter,
                "chain `{chain_id}` step {} `{op}` requires `{param}` to be a string or number",
                index + 1
            ),
            Self::AmbiguousNodeInputs { id } => write!(
                formatter,
                "node `{id}` cannot set both `input` and `inputs`"
            ),
            Self::MissingNodeInput { id } => {
                write!(formatter, "node `{id}` requires `input` or `inputs`")
            }
            Self::UnknownInputPort {
                kind,
                id,
                port,
                available_ports,
            } => write!(
                formatter,
                "{kind} `{id}` references unknown input port `{port}`; available ports: {}",
                available_ports.join(", ")
            ),
            Self::UnsupportedConvertExportPolicy => formatter.write_str(
                "convert --export currently supports path-inferred output formats and graph-representable --channels, --rate, and --norm policies",
            ),
            _ => unreachable!("file errors are formatted separately"),
        }
    }
}

impl fmt::Display for GraphSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { .. }
            | Self::Write { .. }
            | Self::Toml(_)
            | Self::TomlSerialize(_)
            | Self::LockRead { .. }
            | Self::LockWrite { .. }
            | Self::LockToml(_)
            | Self::LockSerialize(_)
            | Self::MissingLock { .. }
            | Self::LockMismatch { .. }
            | Self::UnsupportedVersion { .. } => self.fmt_file_error(formatter),
            _ => self.fmt_validation_error(formatter),
        }
    }
}

impl std::error::Error for GraphSpecError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_stable_step_ids_for_implicit_chain_steps() {
        let spec = GraphSpec {
            version: "auralis.graph/v1".to_owned(),
            name: None,
            sources: vec![SourceSpec {
                id: "voice".to_owned(),
                path: PathBuf::from("input/voice.wav"),
            }],
            chains: vec![ChainSpec {
                id: "voice_clean".to_owned(),
                input: "voice.audio".to_owned(),
                steps: vec![
                    ChainStepSpec {
                        id: None,
                        op: "trim".to_owned(),
                        params: BTreeMap::new(),
                    },
                    ChainStepSpec {
                        id: None,
                        op: "filter.highpass".to_owned(),
                        params: BTreeMap::new(),
                    },
                ],
            }],
            nodes: Vec::new(),
            sinks: vec![SinkSpec {
                id: "wav".to_owned(),
                input: "voice_clean.audio".to_owned(),
                path: PathBuf::from("build/out.wav"),
            }],
        };

        let checked = spec.validate().unwrap();

        assert_eq!(
            checked.expanded_step_ids,
            vec![
                "voice_clean/01-trim".to_owned(),
                "voice_clean/02-filter.highpass".to_owned(),
            ]
        );
    }

    #[test]
    fn formats_graph_spec_into_stable_pretty_toml() {
        let formatted = format_graph_spec_source(
            r#"version = "auralis.graph/v1"
name = "episode"

[[sources]]
id = "voice"
path = "input/voice.wav"

[[chains]]
id = "voice_clean"
input = "voice.audio"
steps = [{ op = "trim", range = "10s..30s" }]

[[sinks]]
id = "wav"
input = "voice_clean.audio"
path = "build/out.wav"
"#,
        )
        .unwrap();

        assert!(formatted.contains("version = \"auralis.graph/v1\""));
        assert!(formatted.contains("name = \"episode\""));
        assert!(formatted.contains("[[chains.steps]]"));
        assert!(formatted.contains("range = \"10s..30s\""));
    }
}
