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
    pub name: Option<String>,
    pub source_count: usize,
    pub chain_count: usize,
    pub node_count: usize,
    pub sink_count: usize,
    pub sources: Vec<CheckedSource>,
    pub chains: Vec<CheckedChain>,
    pub nodes: Vec<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedSink {
    pub id: String,
    pub input: String,
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct GraphSpec {
    version: String,
    #[serde(default)]
    name: Option<String>,
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
    #[serde(default, flatten)]
    params: BTreeMap<String, toml::Value>,
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
            nodes: self.nodes.into_iter().map(|node| node.id).collect(),
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
    match step.op.as_str() {
        "dcshift" => {
            let shift = param_as_string(step.params.get("shift").ok_or_else(|| {
                GraphSpecError::InvalidStepParam {
                    chain_id: chain.id.clone(),
                    index,
                    op: step.op.clone(),
                    param: "shift",
                }
            })?)
            .ok_or_else(|| GraphSpecError::InvalidStepParam {
                chain_id: chain.id.clone(),
                index,
                op: step.op.clone(),
                param: "shift",
            })?;
            Ok(vec![step.op.clone(), shift])
        }
        "gain" => {
            let Some(by) = step.params.get("by") else {
                return Ok(vec![step.op.clone()]);
            };
            let by = param_as_string(by).ok_or_else(|| GraphSpecError::InvalidStepParam {
                chain_id: chain.id.clone(),
                index,
                op: step.op.clone(),
                param: "by",
            })?;
            Ok(vec![step.op.clone(), by])
        }
        "fade" => {
            let Some(fade_in) = step.params.get("fade_in") else {
                return Ok(vec![step.op.clone()]);
            };
            let fade_in =
                param_as_string(fade_in).ok_or_else(|| GraphSpecError::InvalidStepParam {
                    chain_id: chain.id.clone(),
                    index,
                    op: step.op.clone(),
                    param: "fade_in",
                })?;
            let curve = step
                .params
                .get("curve")
                .map(|value| {
                    param_as_string(value).ok_or_else(|| GraphSpecError::InvalidStepParam {
                        chain_id: chain.id.clone(),
                        index,
                        op: step.op.clone(),
                        param: "curve",
                    })
                })
                .transpose()?
                .map_or_else(|| "l".to_owned(), fade_curve_token);
            let Some(fade_out) = step.params.get("fade_out") else {
                return Ok(vec![step.op.clone(), curve, fade_in]);
            };
            let fade_out =
                param_as_string(fade_out).ok_or_else(|| GraphSpecError::InvalidStepParam {
                    chain_id: chain.id.clone(),
                    index,
                    op: step.op.clone(),
                    param: "fade_out",
                })?;
            Ok(vec![
                step.op.clone(),
                curve,
                fade_in,
                "0".to_owned(),
                fade_out,
            ])
        }
        "filter.highpass" => highpass_effect_tokens(chain, index, step),
        "norm.peak" => norm_peak_effect_tokens(chain, index, step),
        "trim" => {
            let Some(range) = step.params.get("range") else {
                return Ok(vec![step.op.clone()]);
            };
            let range = param_as_string(range).ok_or_else(|| GraphSpecError::InvalidStepParam {
                chain_id: chain.id.clone(),
                index,
                op: step.op.clone(),
                param: "range",
            })?;
            let Some((start, end)) = range.split_once("..") else {
                return Err(GraphSpecError::InvalidStepParam {
                    chain_id: chain.id.clone(),
                    index,
                    op: step.op.clone(),
                    param: "range",
                });
            };
            Ok(vec![step.op.clone(), start.to_owned(), format!("={end}")])
        }
        _ => Ok(vec![step.op.clone()]),
    }
}

fn highpass_effect_tokens(
    chain: &ChainSpec,
    index: usize,
    step: &ChainStepSpec,
) -> Result<Vec<String>, GraphSpecError> {
    let Some(cutoff) = step.params.get("cutoff") else {
        return Ok(vec![step.op.clone()]);
    };
    let cutoff = param_as_frequency_hz(cutoff).ok_or_else(|| GraphSpecError::InvalidStepParam {
        chain_id: chain.id.clone(),
        index,
        op: step.op.clone(),
        param: "cutoff",
    })?;
    let mut tokens = vec!["highpass".to_owned(), cutoff];
    if let Some(q) = step.params.get("q") {
        let q = param_as_string(q).ok_or_else(|| GraphSpecError::InvalidStepParam {
            chain_id: chain.id.clone(),
            index,
            op: step.op.clone(),
            param: "q",
        })?;
        tokens.push(format!("{q}q"));
    }
    Ok(tokens)
}

fn norm_peak_effect_tokens(
    chain: &ChainSpec,
    index: usize,
    step: &ChainStepSpec,
) -> Result<Vec<String>, GraphSpecError> {
    let Some(target) = step.params.get("target") else {
        return Ok(vec!["norm".to_owned()]);
    };
    let target = param_as_dbfs(target).ok_or_else(|| GraphSpecError::InvalidStepParam {
        chain_id: chain.id.clone(),
        index,
        op: step.op.clone(),
        param: "target",
    })?;
    Ok(vec!["norm".to_owned(), target])
}

fn param_as_frequency_hz(value: &toml::Value) -> Option<String> {
    let value = param_as_string(value)?;
    let value = value
        .strip_suffix("Hz")
        .or_else(|| value.strip_suffix("hz"))
        .unwrap_or(&value);
    Some(value.to_owned())
}

fn param_as_dbfs(value: &toml::Value) -> Option<String> {
    let value = param_as_string(value)?;
    let value = value
        .strip_suffix("dBFS")
        .or_else(|| value.strip_suffix("dbfs"))
        .or_else(|| value.strip_suffix("dB"))
        .or_else(|| value.strip_suffix("db"))
        .unwrap_or(&value);
    Some(value.to_owned())
}

fn fade_curve_token(value: String) -> String {
    match value.as_str() {
        "linear" => "t".to_owned(),
        "logarithmic" => "l".to_owned(),
        "quarter-sine" => "q".to_owned(),
        "half-sine" => "h".to_owned(),
        "inverted-parabola" => "p".to_owned(),
        _ => value,
    }
}

fn param_as_string(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::String(value) => Some(value.clone()),
        toml::Value::Integer(value) => Some(value.to_string()),
        toml::Value::Float(value) => Some(value.to_string()),
        _ => None,
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
}
