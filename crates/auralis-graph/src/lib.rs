//! Surface-neutral graph request validation for Auralis.
//!
//! This crate owns the shared graph boundary between frontends and execution.
//! CLI strings, TOML syntax, codec I/O, DSP kernels, and concrete effect
//! implementations remain outside this crate.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Path, PathBuf},
};

use auralis_op::{OpCatalog, OpDescriptor, ParamKind, PortKind};
use thiserror::Error;

/// Crate-local result type using [`GraphValidationError`].
pub type Result<T> = std::result::Result<T, GraphValidationError>;

/// Frontend-neutral request for a finite whole-buffer audio graph.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphRequest {
    /// Optional human-readable graph name.
    pub name: Option<String>,
    /// Complete input artifacts.
    pub sources: Vec<SourceRequest>,
    /// Operation nodes.
    pub nodes: Vec<NodeRequest>,
    /// Output artifacts.
    pub sinks: Vec<SinkRequest>,
    /// Optional selected target by graph item ID.
    pub target: Option<String>,
}

impl GraphRequest {
    /// Validates this request against an operation catalog.
    ///
    /// # Errors
    ///
    /// Returns [`GraphValidationError`] when IDs, ports, operation names,
    /// parameters, or DAG structure violate the graph contract.
    pub fn validate(&self, catalog: &OpCatalog) -> Result<ValidGraph> {
        validate_request(self, catalog)
    }
}

/// Source artifact request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRequest {
    /// Stable graph item ID.
    pub id: String,
    /// Filesystem path to decode before graph execution.
    pub path: PathBuf,
}

/// Operation node request.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeRequest {
    /// Stable graph item ID.
    pub id: String,
    /// Canonical operation name or alias.
    pub op: String,
    /// Ordered input bindings.
    pub inputs: Vec<InputBinding>,
    /// Named operation parameters.
    pub params: BTreeMap<String, ParamValue>,
}

/// Sink artifact request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkRequest {
    /// Stable graph item ID.
    pub id: String,
    /// Port to write.
    pub input: PortRef,
    /// Filesystem path to encode after graph execution.
    pub path: PathBuf,
}

/// Edge-local input binding.
#[derive(Debug, Clone, PartialEq)]
pub struct InputBinding {
    /// Upstream output port.
    pub port: PortRef,
    /// Optional edge gain or equivalent frontend-lowered edge parameter.
    pub gain: Option<ParamValue>,
}

/// Reference to an output port on a source or node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortRef {
    /// Source or node ID that owns the port.
    pub owner: String,
    /// Output port name.
    pub port: String,
}

impl fmt::Display for PortRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.owner, self.port)
    }
}

/// Frontend-neutral parameter value.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ParamValue {
    /// String or unit-bearing value.
    String(String),
    /// Floating-point number.
    Number(f64),
    /// Integer number.
    Integer(i64),
    /// Boolean flag.
    Bool(bool),
}

impl ParamValue {
    #[must_use]
    fn kind(&self) -> ParamKind {
        match self {
            Self::String(_) => ParamKind::String,
            Self::Number(_) => ParamKind::Number,
            Self::Integer(_) => ParamKind::Integer,
            Self::Bool(_) => ParamKind::Bool,
        }
    }
}

/// Validated graph request ready for planning.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidGraph {
    /// Optional human-readable graph name.
    pub name: Option<String>,
    /// Complete input artifacts.
    pub sources: Vec<SourceRequest>,
    /// Validated operation nodes.
    pub nodes: Vec<ValidNode>,
    /// Output artifacts.
    pub sinks: Vec<SinkRequest>,
    /// Optional selected target by graph item ID.
    pub target: Option<String>,
}

/// Validated operation node with canonical operation name.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidNode {
    /// Stable graph item ID.
    pub id: String,
    /// Canonical operation name after alias resolution.
    pub op: String,
    /// Ordered input bindings.
    pub inputs: Vec<InputBinding>,
    /// Named operation parameters.
    pub params: BTreeMap<String, ParamValue>,
}

/// Graph request validation failure.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum GraphValidationError {
    /// A graph item has an empty ID.
    #[error("{kind} id cannot be empty")]
    EmptyId {
        /// Graph item kind.
        kind: &'static str,
    },
    /// Two graph items share the same ID.
    #[error("duplicate graph id `{id}` for {first_kind} and {second_kind}")]
    DuplicateId {
        /// Duplicated ID.
        id: String,
        /// First graph item kind.
        first_kind: &'static str,
        /// Second graph item kind.
        second_kind: &'static str,
    },
    /// A source or sink path is empty.
    #[error("{kind} `{id}` path cannot be empty")]
    EmptyPath {
        /// Graph item kind.
        kind: &'static str,
        /// Graph item ID.
        id: String,
    },
    /// An operation name is not registered.
    #[error("node `{id}` references unknown op `{op}`")]
    UnknownOp {
        /// Node ID.
        id: String,
        /// Unknown operation name.
        op: String,
    },
    /// A node is missing a required parameter.
    #[error("node `{id}` op `{op}` is missing required parameter `{param}`")]
    MissingParam {
        /// Node ID.
        id: String,
        /// Canonical operation name.
        op: String,
        /// Missing parameter name.
        param: String,
    },
    /// A node provides a parameter not declared by the operation.
    #[error("node `{id}` op `{op}` has unknown parameter `{param}`")]
    UnknownParam {
        /// Node ID.
        id: String,
        /// Canonical operation name.
        op: String,
        /// Unknown parameter name.
        param: String,
    },
    /// A node provides a parameter value with the wrong kind.
    #[error("node `{id}` op `{op}` parameter `{param}` expected {expected:?}, got {actual:?}")]
    InvalidParamKind {
        /// Node ID.
        id: String,
        /// Canonical operation name.
        op: String,
        /// Parameter name.
        param: String,
        /// Expected parameter kind.
        expected: ParamKind,
        /// Actual parameter kind.
        actual: ParamKind,
    },
    /// A node has the wrong number of input bindings for its operation.
    #[error("node `{id}` op `{op}` expected {expected}, got {actual} input bindings")]
    InvalidInputCount {
        /// Node ID.
        id: String,
        /// Canonical operation name.
        op: String,
        /// Human-readable expected count.
        expected: String,
        /// Actual input count.
        actual: usize,
    },
    /// A graph item references an unknown input port.
    #[error("{kind} `{id}` references unknown input port `{port}`")]
    UnknownInputPort {
        /// Graph item kind.
        kind: &'static str,
        /// Graph item ID.
        id: String,
        /// Unknown port.
        port: PortRef,
    },
    /// A graph item binds a port with the wrong value kind.
    #[error("{kind} `{id}` input `{port}` expected {expected:?}, got {actual:?}")]
    PortKindMismatch {
        /// Graph item kind.
        kind: &'static str,
        /// Graph item ID.
        id: String,
        /// Input port.
        port: PortRef,
        /// Expected port kind.
        expected: PortKind,
        /// Actual port kind.
        actual: PortKind,
    },
    /// Graph node dependencies contain a cycle.
    #[error("graph contains a cycle through node `{id}`")]
    Cycle {
        /// Node ID where a cycle was detected.
        id: String,
    },
    /// Selected target ID does not exist.
    #[error("unknown graph target `{target}`")]
    UnknownTarget {
        /// Unknown target ID.
        target: String,
    },
}

fn validate_request(request: &GraphRequest, catalog: &OpCatalog) -> Result<ValidGraph> {
    let mut ids = BTreeMap::new();
    let mut ports = BTreeMap::new();
    let mut descriptors = BTreeMap::new();

    for source in &request.sources {
        validate_id_and_path(&source.id, &source.path, "source", &mut ids)?;
        ports.insert(audio_port(&source.id), PortKind::Audio);
    }

    for node in &request.nodes {
        validate_id(&node.id, "node", &mut ids)?;
        let descriptor =
            catalog
                .resolve(&node.op)
                .ok_or_else(|| GraphValidationError::UnknownOp {
                    id: node.id.clone(),
                    op: node.op.clone(),
                })?;
        for output in descriptor.outputs {
            ports.insert(
                PortRef {
                    owner: node.id.clone(),
                    port: output.name.to_owned(),
                },
                output.kind,
            );
        }
        descriptors.insert(node.id.as_str(), descriptor);
    }

    for sink in &request.sinks {
        validate_id_and_path(&sink.id, &sink.path, "sink", &mut ids)?;
    }

    if let Some(target) = &request.target
        && !ids.contains_key(target)
    {
        return Err(GraphValidationError::UnknownTarget {
            target: target.clone(),
        });
    }

    let mut valid_nodes = Vec::new();
    let mut dependencies = BTreeMap::<String, BTreeSet<String>>::new();
    for node in &request.nodes {
        let descriptor = descriptors[node.id.as_str()];
        validate_node_inputs(node, descriptor, &ports, &mut dependencies)?;
        validate_node_params(node, descriptor)?;
        valid_nodes.push(ValidNode {
            id: node.id.clone(),
            op: descriptor.name.to_owned(),
            inputs: node.inputs.clone(),
            params: node.params.clone(),
        });
    }

    for sink in &request.sinks {
        let actual =
            ports
                .get(&sink.input)
                .ok_or_else(|| GraphValidationError::UnknownInputPort {
                    kind: "sink",
                    id: sink.id.clone(),
                    port: sink.input.clone(),
                })?;
        if *actual != PortKind::Audio {
            return Err(GraphValidationError::PortKindMismatch {
                kind: "sink",
                id: sink.id.clone(),
                port: sink.input.clone(),
                expected: PortKind::Audio,
                actual: *actual,
            });
        }
    }

    reject_cycles(&dependencies)?;

    Ok(ValidGraph {
        name: request.name.clone(),
        sources: request.sources.clone(),
        nodes: valid_nodes,
        sinks: request.sinks.clone(),
        target: request.target.clone(),
    })
}

fn validate_id_and_path(
    id: &str,
    path: &Path,
    kind: &'static str,
    ids: &mut BTreeMap<String, &'static str>,
) -> Result<()> {
    validate_id(id, kind, ids)?;
    if path.as_os_str().is_empty() {
        return Err(GraphValidationError::EmptyPath {
            kind,
            id: id.to_owned(),
        });
    }
    Ok(())
}

fn validate_id(
    id: &str,
    kind: &'static str,
    ids: &mut BTreeMap<String, &'static str>,
) -> Result<()> {
    if id.trim().is_empty() {
        return Err(GraphValidationError::EmptyId { kind });
    }
    if let Some(first_kind) = ids.insert(id.to_owned(), kind) {
        return Err(GraphValidationError::DuplicateId {
            id: id.to_owned(),
            first_kind,
            second_kind: kind,
        });
    }
    Ok(())
}

fn validate_node_inputs(
    node: &NodeRequest,
    descriptor: OpDescriptor,
    ports: &BTreeMap<PortRef, PortKind>,
    dependencies: &mut BTreeMap<String, BTreeSet<String>>,
) -> Result<()> {
    validate_input_count(node, descriptor)?;
    for (index, input) in node.inputs.iter().enumerate() {
        let actual =
            ports
                .get(&input.port)
                .ok_or_else(|| GraphValidationError::UnknownInputPort {
                    kind: "node",
                    id: node.id.clone(),
                    port: input.port.clone(),
                })?;
        let expected = input_kind(descriptor, index).unwrap_or(PortKind::Audio);
        if *actual != expected {
            return Err(GraphValidationError::PortKindMismatch {
                kind: "node",
                id: node.id.clone(),
                port: input.port.clone(),
                expected,
                actual: *actual,
            });
        }
        if ports.contains_key(&audio_port(&input.port.owner)) {
            dependencies
                .entry(node.id.clone())
                .or_default()
                .insert(input.port.owner.clone());
        }
    }
    Ok(())
}

fn validate_input_count(node: &NodeRequest, descriptor: OpDescriptor) -> Result<()> {
    let required = descriptor.inputs.len();
    let has_variadic = descriptor.inputs.iter().any(|input| input.variadic);
    let actual = node.inputs.len();
    if has_variadic {
        if actual >= required {
            return Ok(());
        }
        return Err(GraphValidationError::InvalidInputCount {
            id: node.id.clone(),
            op: descriptor.name.to_owned(),
            expected: format!("at least {required}"),
            actual,
        });
    }
    if actual == descriptor.inputs.len() {
        return Ok(());
    }
    Err(GraphValidationError::InvalidInputCount {
        id: node.id.clone(),
        op: descriptor.name.to_owned(),
        expected: descriptor.inputs.len().to_string(),
        actual,
    })
}

fn validate_node_params(node: &NodeRequest, descriptor: OpDescriptor) -> Result<()> {
    let params = descriptor
        .params
        .iter()
        .map(|param| (param.name, param))
        .collect::<BTreeMap<_, _>>();
    for param in descriptor.params.iter().filter(|param| param.required) {
        if !node.params.contains_key(param.name) {
            return Err(GraphValidationError::MissingParam {
                id: node.id.clone(),
                op: descriptor.name.to_owned(),
                param: param.name.to_owned(),
            });
        }
    }
    for (name, value) in &node.params {
        let descriptor_param =
            params
                .get(name.as_str())
                .ok_or_else(|| GraphValidationError::UnknownParam {
                    id: node.id.clone(),
                    op: descriptor.name.to_owned(),
                    param: name.clone(),
                })?;
        if !param_kind_matches(descriptor_param.kind, value.kind()) {
            return Err(GraphValidationError::InvalidParamKind {
                id: node.id.clone(),
                op: descriptor.name.to_owned(),
                param: name.clone(),
                expected: descriptor_param.kind,
                actual: value.kind(),
            });
        }
    }
    Ok(())
}

fn input_kind(descriptor: OpDescriptor, index: usize) -> Option<PortKind> {
    descriptor
        .inputs
        .get(index)
        .or_else(|| descriptor.inputs.iter().find(|input| input.variadic))
        .map(|input| input.kind)
}

fn param_kind_matches(expected: ParamKind, actual: ParamKind) -> bool {
    expected == actual || (expected == ParamKind::Number && actual == ParamKind::Integer)
}

fn reject_cycles(dependencies: &BTreeMap<String, BTreeSet<String>>) -> Result<()> {
    let mut states = BTreeMap::new();
    for node in dependencies.keys() {
        visit_node(node, dependencies, &mut states)?;
    }
    Ok(())
}

fn visit_node(
    node: &str,
    dependencies: &BTreeMap<String, BTreeSet<String>>,
    states: &mut BTreeMap<String, VisitState>,
) -> Result<()> {
    match states.get(node) {
        Some(VisitState::Visiting) => {
            return Err(GraphValidationError::Cycle {
                id: node.to_owned(),
            });
        }
        Some(VisitState::Done) => return Ok(()),
        None => {}
    }
    states.insert(node.to_owned(), VisitState::Visiting);
    if let Some(upstream) = dependencies.get(node) {
        for dependency in upstream {
            visit_node(dependency, dependencies, states)?;
        }
    }
    states.insert(node.to_owned(), VisitState::Done);
    Ok(())
}

fn audio_port(owner: &str) -> PortRef {
    PortRef {
        owner: owner.to_owned(),
        port: "audio".to_owned(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Done,
}

#[cfg(test)]
mod tests {
    use super::{
        GraphRequest, GraphValidationError, InputBinding, NodeRequest, ParamValue, PortRef,
        SinkRequest, SourceRequest,
    };
    use auralis_op::{
        OpCapabilities, OpCatalog, OpCategory, OpDescriptor, ParamDescriptor, ParamKind,
        PortDescriptor, PortKind,
    };
    use std::{collections::BTreeMap, path::PathBuf};

    const AUDIO_IN: &[PortDescriptor] = &[PortDescriptor {
        name: "in",
        kind: PortKind::Audio,
        variadic: false,
        summary: "input audio",
    }];
    const AUDIO_OUT: &[PortDescriptor] = &[PortDescriptor {
        name: "audio",
        kind: PortKind::Audio,
        variadic: false,
        summary: "output audio",
    }];
    const MIX_IN: &[PortDescriptor] = &[PortDescriptor {
        name: "inputs",
        kind: PortKind::Audio,
        variadic: true,
        summary: "input audio",
    }];
    const GAIN_PARAMS: &[ParamDescriptor] = &[ParamDescriptor {
        name: "by",
        kind: ParamKind::String,
        required: true,
        summary: "gain amount",
    }];

    fn catalog() -> OpCatalog {
        OpCatalog::new([
            OpDescriptor {
                name: "gain",
                aliases: &["vol"],
                category: OpCategory::AudioTransform,
                inputs: AUDIO_IN,
                outputs: AUDIO_OUT,
                params: GAIN_PARAMS,
                capabilities: OpCapabilities {
                    deterministic: true,
                    whole_buffer: true,
                },
                examples: &[],
            },
            OpDescriptor {
                name: "mix.sum",
                aliases: &[],
                category: OpCategory::Mixer,
                inputs: MIX_IN,
                outputs: AUDIO_OUT,
                params: &[],
                capabilities: OpCapabilities {
                    deterministic: true,
                    whole_buffer: true,
                },
                examples: &[],
            },
        ])
        .unwrap()
    }

    fn source(id: &str) -> SourceRequest {
        SourceRequest {
            id: id.to_owned(),
            path: PathBuf::from(format!("{id}.wav")),
        }
    }

    fn sink(id: &str, owner: &str) -> SinkRequest {
        SinkRequest {
            id: id.to_owned(),
            input: port(owner),
            path: PathBuf::from(format!("{id}.wav")),
        }
    }

    fn port(owner: &str) -> PortRef {
        PortRef {
            owner: owner.to_owned(),
            port: "audio".to_owned(),
        }
    }

    fn input(owner: &str) -> InputBinding {
        InputBinding {
            port: port(owner),
            gain: None,
        }
    }

    fn gain_node(id: &str, owner: &str) -> NodeRequest {
        NodeRequest {
            id: id.to_owned(),
            op: "vol".to_owned(),
            inputs: vec![input(owner)],
            params: BTreeMap::from([("by".to_owned(), ParamValue::String("-3dB".to_owned()))]),
        }
    }

    #[test]
    fn validates_graph_and_canonicalizes_op_aliases() {
        let request = GraphRequest {
            name: Some("voice cleanup".to_owned()),
            sources: vec![source("input")],
            nodes: vec![gain_node("clean", "input")],
            sinks: vec![sink("output", "clean")],
            target: Some("output".to_owned()),
        };

        let valid = request.validate(&catalog()).unwrap();

        assert_eq!(valid.name.as_deref(), Some("voice cleanup"));
        assert_eq!(valid.nodes[0].op, "gain");
        assert_eq!(valid.target.as_deref(), Some("output"));
    }

    #[test]
    fn rejects_unknown_ops() {
        let mut request = valid_request();
        request.nodes[0].op = "gian".to_owned();

        let error = request.validate(&catalog()).unwrap_err();

        assert_eq!(
            error,
            GraphValidationError::UnknownOp {
                id: "clean".to_owned(),
                op: "gian".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_missing_required_params() {
        let mut request = valid_request();
        request.nodes[0].params.clear();

        let error = request.validate(&catalog()).unwrap_err();

        assert_eq!(
            error,
            GraphValidationError::MissingParam {
                id: "clean".to_owned(),
                op: "gain".to_owned(),
                param: "by".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_unknown_params() {
        let mut request = valid_request();
        request.nodes[0]
            .params
            .insert("db".to_owned(), ParamValue::String("-3dB".to_owned()));

        let error = request.validate(&catalog()).unwrap_err();

        assert_eq!(
            error,
            GraphValidationError::UnknownParam {
                id: "clean".to_owned(),
                op: "gain".to_owned(),
                param: "db".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_unknown_input_ports() {
        let mut request = valid_request();
        request.nodes[0].inputs[0].port.owner = "missing".to_owned();

        let error = request.validate(&catalog()).unwrap_err();

        assert_eq!(
            error,
            GraphValidationError::UnknownInputPort {
                kind: "node",
                id: "clean".to_owned(),
                port: port("missing"),
            }
        );
    }

    #[test]
    fn rejects_node_cycles() {
        let request = GraphRequest {
            name: None,
            sources: vec![],
            nodes: vec![gain_node("a", "b"), gain_node("b", "a")],
            sinks: vec![sink("out", "a")],
            target: None,
        };

        let error = request.validate(&catalog()).unwrap_err();

        assert!(matches!(error, GraphValidationError::Cycle { .. }));
    }

    #[test]
    fn accepts_variadic_inputs() {
        let request = GraphRequest {
            name: None,
            sources: vec![source("left"), source("right")],
            nodes: vec![NodeRequest {
                id: "mix".to_owned(),
                op: "mix.sum".to_owned(),
                inputs: vec![input("left"), input("right")],
                params: BTreeMap::new(),
            }],
            sinks: vec![sink("out", "mix")],
            target: None,
        };

        let valid = request.validate(&catalog()).unwrap();

        assert_eq!(valid.nodes[0].inputs.len(), 2);
    }

    #[test]
    fn rejects_empty_variadic_inputs() {
        let request = GraphRequest {
            name: None,
            sources: vec![source("left")],
            nodes: vec![NodeRequest {
                id: "mix".to_owned(),
                op: "mix.sum".to_owned(),
                inputs: vec![],
                params: BTreeMap::new(),
            }],
            sinks: vec![sink("out", "mix")],
            target: None,
        };

        let error = request.validate(&catalog()).unwrap_err();

        assert_eq!(
            error,
            GraphValidationError::InvalidInputCount {
                id: "mix".to_owned(),
                op: "mix.sum".to_owned(),
                expected: "at least 1".to_owned(),
                actual: 0,
            }
        );
    }

    fn valid_request() -> GraphRequest {
        GraphRequest {
            name: None,
            sources: vec![source("input")],
            nodes: vec![gain_node("clean", "input")],
            sinks: vec![sink("output", "clean")],
            target: None,
        }
    }
}
