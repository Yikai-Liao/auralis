use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

pub fn check_graph_spec(path: &Path) -> Result<CheckedGraphSpec, GraphSpecError> {
    let source = fs::read_to_string(path).map_err(|error| GraphSpecError::Read {
        path: path.to_path_buf(),
        error,
    })?;
    let spec = toml::from_str::<GraphSpec>(&source).map_err(GraphSpecError::Toml)?;

    spec.validate()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedGraphSpec {
    pub source_count: usize,
    pub chain_count: usize,
    pub node_count: usize,
    pub sink_count: usize,
    pub expanded_step_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GraphSpec {
    version: String,
    #[serde(default)]
    sources: Vec<SourceSpec>,
    #[serde(default)]
    chains: Vec<ChainSpec>,
    #[serde(default)]
    nodes: Vec<NodeSpec>,
    #[serde(default)]
    sinks: Vec<SinkSpec>,
}

#[derive(Debug, Deserialize)]
struct SourceSpec {
    id: String,
    path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ChainSpec {
    id: String,
    input: String,
    #[serde(default)]
    steps: Vec<ChainStepSpec>,
}

#[derive(Debug, Deserialize)]
struct ChainStepSpec {
    id: Option<String>,
    op: String,
}

#[derive(Debug, Deserialize)]
struct NodeSpec {
    id: String,
    #[serde(default)]
    input: Option<String>,
    #[serde(default)]
    inputs: Vec<NodeInputSpec>,
}

#[derive(Debug, Deserialize)]
struct NodeInputSpec {
    from: String,
}

#[derive(Debug, Deserialize)]
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

        let mut expanded_step_ids = Vec::new();
        for chain in &self.chains {
            ensure_known_port(&chain.input, "chain", &chain.id, &available_ports)?;
            if chain.steps.is_empty() {
                return Err(GraphSpecError::EmptyChain {
                    id: chain.id.clone(),
                });
            }

            let mut seen_step_ids = BTreeSet::new();
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
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| format!("{}/{:02}-{}", chain.id, index + 1, step.op));
                if !seen_step_ids.insert(expanded_id.clone()) {
                    return Err(GraphSpecError::DuplicateChainStepId {
                        chain_id: chain.id.clone(),
                        step_id: expanded_id,
                    });
                }
                expanded_step_ids.push(expanded_id);
            }
        }

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
            source_count: self.sources.len(),
            chain_count: self.chains.len(),
            node_count: self.nodes.len(),
            sink_count: self.sinks.len(),
            expanded_step_ids,
        })
    }
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
    Toml(toml::de::Error),
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
}

impl fmt::Display for GraphSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, error } => {
                write!(
                    formatter,
                    "failed to read graph spec {}: {error}",
                    path.display()
                )
            }
            Self::Toml(error) => write!(formatter, "invalid graph spec TOML: {error}"),
            Self::UnsupportedVersion { version } => write!(
                formatter,
                "unsupported graph spec version `{version}`; expected `auralis.graph/v1`"
            ),
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
                    },
                    ChainStepSpec {
                        id: None,
                        op: "filter.highpass".to_owned(),
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
}
