use std::{collections::BTreeMap, ffi::OsStr, path::Path};

use serde::Serialize;

use crate::{CliError, graph_runtime, spec};

pub(super) fn plan_graph_spec(spec: &Path, json: bool, locked: bool) -> Result<(), CliError> {
    if locked {
        spec::verify_graph_lock(spec)?;
    }
    let checked = spec::check_graph_spec(spec)?;
    let plan = build_plan(&checked);
    let pipeline_name = checked
        .name
        .as_deref()
        .or_else(|| spec.file_stem().and_then(OsStr::to_str))
        .unwrap_or("Auralis.toml");
    if json {
        return print_json_plan(spec, pipeline_name, &checked, &plan);
    }

    println!("Pipeline: {pipeline_name}");
    println!("Spec: {}", spec.display());
    println!();
    println!("Inputs:");
    for source in &checked.sources {
        println!("  {}  {}", source.id, source.path.display());
    }
    println!();
    println!("Outputs:");
    for sink in &checked.sinks {
        println!("  {}  {}", sink.id, sink.path.display());
    }
    println!();
    println!("Graph:");
    println!("  sources: {}", checked.source_count);
    println!("  chains: {}", checked.chain_count);
    println!("  nodes: {}", checked.node_count);
    println!("  sinks: {}", checked.sink_count);
    println!("  expanded steps: {}", checked.expanded_step_ids.len());
    println!("  streaming segments: {}", plan.streaming_segments);
    println!("  whole-buffer barriers: {}", plan.whole_buffer_barriers);
    println!("  fanout points: {}", plan.fanout_points.len());
    println!();
    println!("Execution:");
    for source in &checked.sources {
        println!("  read {}", source.id);
    }
    for chain in &checked.chains {
        println!("  chain {} <- {}", chain.id, chain.input);
        for step_id in &chain.step_ids {
            println!("    step {step_id}");
        }
    }
    for node in &checked.nodes {
        println!("  node {} ({})", node.id, node_display_label(node));
    }
    for sink in &checked.sinks {
        println!("  write {} <- {}", sink.id, sink.input);
    }
    if !plan.segments.is_empty() {
        println!();
        println!("Segments:");
        for segment in &plan.segments {
            println!("  {}  {}", segment.id, segment.summary);
            println!("      mode: {}", segment.mode);
            if let Some(reason) = &segment.reason {
                println!("      reason: {reason}");
            }
        }
    }
    if !plan.fanout_points.is_empty() {
        println!();
        println!("Fanout:");
        for fanout in &plan.fanout_points {
            println!("  {} -> {}", fanout.port, fanout.consumers.join(", "));
        }
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct JsonPlan {
    pipeline: String,
    spec: String,
    inputs: Vec<JsonPlanIo>,
    outputs: Vec<JsonPlanIo>,
    graph: JsonPlanGraph,
    execution: Vec<JsonPlanStep>,
    segments: Vec<JsonPlanSegment>,
    fanout_points: Vec<JsonPlanFanout>,
}

#[derive(Debug, Serialize)]
struct JsonPlanIo {
    id: String,
    path: String,
}

#[derive(Debug, Serialize)]
struct JsonPlanGraph {
    sources: usize,
    chains: usize,
    nodes: usize,
    sinks: usize,
    expanded_steps: usize,
    streaming_segments: usize,
    whole_buffer_barriers: usize,
    fanout_points: usize,
}

#[derive(Debug, Serialize)]
#[serde(tag = "action")]
enum JsonPlanStep {
    #[serde(rename = "read")]
    Read { id: String },
    #[serde(rename = "chain")]
    Chain {
        id: String,
        input: String,
        steps: Vec<String>,
    },
    #[serde(rename = "node")]
    Node {
        id: String,
        inputs: Vec<String>,
        label: String,
    },
    #[serde(rename = "write")]
    Write { id: String, input: String },
}

#[derive(Debug, Serialize)]
struct JsonPlanSegment {
    id: String,
    mode: String,
    summary: String,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct JsonPlanFanout {
    port: String,
    consumers: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum PlanStepMode {
    Streaming,
    WholeBufferBarrier,
    AnalysisPass,
}

impl PlanStepMode {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Streaming => "streaming",
            Self::WholeBufferBarrier => "whole-buffer barrier",
            Self::AnalysisPass => "analysis pass",
        }
    }
}

#[derive(Debug)]
struct PlanSegment {
    id: String,
    mode: &'static str,
    summary: String,
    reason: Option<String>,
}

#[derive(Debug)]
struct FanoutPoint {
    port: String,
    consumers: Vec<String>,
}

#[derive(Debug)]
struct GraphPlan {
    streaming_segments: usize,
    whole_buffer_barriers: usize,
    segments: Vec<PlanSegment>,
    fanout_points: Vec<FanoutPoint>,
}

fn print_json_plan(
    spec: &Path,
    pipeline_name: &str,
    checked: &spec::CheckedGraphSpec,
    plan: &GraphPlan,
) -> Result<(), CliError> {
    let mut execution = Vec::new();
    for source in &checked.sources {
        execution.push(JsonPlanStep::Read {
            id: source.id.clone(),
        });
    }
    for chain in &checked.chains {
        execution.push(JsonPlanStep::Chain {
            id: chain.id.clone(),
            input: chain.input.clone(),
            steps: chain.step_ids.clone(),
        });
    }
    for node in &checked.nodes {
        execution.push(JsonPlanStep::Node {
            id: node.id.clone(),
            inputs: node.inputs.clone(),
            label: node_display_label(node),
        });
    }
    for sink in &checked.sinks {
        execution.push(JsonPlanStep::Write {
            id: sink.id.clone(),
            input: sink.input.clone(),
        });
    }

    let plan = JsonPlan {
        pipeline: pipeline_name.to_owned(),
        spec: spec.display().to_string(),
        inputs: checked
            .sources
            .iter()
            .map(|source| JsonPlanIo {
                id: source.id.clone(),
                path: source.path.display().to_string(),
            })
            .collect(),
        outputs: checked
            .sinks
            .iter()
            .map(|sink| JsonPlanIo {
                id: sink.id.clone(),
                path: sink.path.display().to_string(),
            })
            .collect(),
        graph: JsonPlanGraph {
            sources: checked.source_count,
            chains: checked.chain_count,
            nodes: checked.node_count,
            sinks: checked.sink_count,
            expanded_steps: checked.expanded_step_ids.len(),
            streaming_segments: plan.streaming_segments,
            whole_buffer_barriers: plan.whole_buffer_barriers,
            fanout_points: plan.fanout_points.len(),
        },
        execution,
        segments: plan
            .segments
            .iter()
            .map(|segment| JsonPlanSegment {
                id: segment.id.clone(),
                mode: segment.mode.to_owned(),
                summary: segment.summary.clone(),
                reason: segment.reason.clone(),
            })
            .collect(),
        fanout_points: plan
            .fanout_points
            .iter()
            .map(|fanout| JsonPlanFanout {
                port: fanout.port.clone(),
                consumers: fanout.consumers.clone(),
            })
            .collect(),
    };

    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

fn build_plan(checked: &spec::CheckedGraphSpec) -> GraphPlan {
    let mut segments = Vec::new();
    let mut streaming_segments = 0;
    let mut whole_buffer_barriers = 0;

    for chain in &checked.chains {
        let upstream = chain
            .input
            .strip_suffix(".audio")
            .unwrap_or(chain.input.as_str());
        let mut stream_steps = vec![format!("{upstream}.read")];
        for (step_id, label) in chain.step_ids.iter().zip(chain.step_labels.iter()) {
            match classify_plan_step(label) {
                (PlanStepMode::Streaming, _) => stream_steps.push(step_id.clone()),
                (mode, reason) => {
                    if stream_steps.len() > 1 {
                        streaming_segments += 1;
                        segments.push(PlanSegment {
                            id: format!("S{streaming_segments}"),
                            mode: PlanStepMode::Streaming.label(),
                            summary: stream_steps.join(" -> "),
                            reason: None,
                        });
                        stream_steps = Vec::new();
                    }
                    whole_buffer_barriers += 1;
                    segments.push(PlanSegment {
                        id: format!("B{whole_buffer_barriers}"),
                        mode: mode.label(),
                        summary: step_id.clone(),
                        reason: Some(reason.to_owned()),
                    });
                }
            }
        }
        if stream_steps.len() > 1 {
            streaming_segments += 1;
            segments.push(PlanSegment {
                id: format!("S{streaming_segments}"),
                mode: PlanStepMode::Streaming.label(),
                summary: stream_steps.join(" -> "),
                reason: None,
            });
        }
    }

    GraphPlan {
        streaming_segments,
        whole_buffer_barriers,
        segments,
        fanout_points: fanout_points(checked),
    }
}

pub(super) fn classify_plan_step(label: &str) -> (PlanStepMode, &'static str) {
    let op = label.split_whitespace().next().unwrap_or(label);
    match op {
        "reverse" => (
            PlanStepMode::WholeBufferBarrier,
            "reverse requires a full-buffer materialization",
        ),
        "norm" => (
            PlanStepMode::AnalysisPass,
            "norm.peak scans the whole stream before applying gain",
        ),
        _ => (PlanStepMode::Streaming, ""),
    }
}

fn fanout_points(checked: &spec::CheckedGraphSpec) -> Vec<FanoutPoint> {
    let mut consumers = BTreeMap::<String, Vec<String>>::new();
    for chain in &checked.chains {
        consumers
            .entry(chain.input.clone())
            .or_default()
            .push(format!("chain {}", chain.id));
    }
    for node in &checked.nodes {
        for input in &node.inputs {
            consumers
                .entry(input.clone())
                .or_default()
                .push(format!("node {}", node.id));
        }
    }
    for sink in &checked.sinks {
        consumers
            .entry(sink.input.clone())
            .or_default()
            .push(format!("sink {}", sink.id));
    }

    consumers
        .into_iter()
        .filter_map(|(port, consumers)| {
            (consumers.len() > 1).then_some(FanoutPoint { port, consumers })
        })
        .collect()
}

pub(super) fn node_display_label(node: &spec::CheckedNode) -> String {
    match node.op.as_deref() {
        None => "passthrough".to_owned(),
        Some("mix.sum") => "mix.sum".to_owned(),
        Some(op) => graph_runtime::node_effect_tokens(op, &node.params).map_or_else(
            |_| op.to_owned(),
            |tokens| {
                if tokens.is_empty() {
                    op.to_owned()
                } else {
                    tokens.join(" ")
                }
            },
        ),
    }
}

pub(super) fn classify_node_plan_mode(node: &spec::CheckedNode) -> (PlanStepMode, &'static str) {
    match node.op.as_deref() {
        None => (PlanStepMode::Streaming, "passthrough node"),
        Some("mix.sum") => (PlanStepMode::Streaming, "multi-input streaming mix"),
        Some("norm.peak") => (
            PlanStepMode::AnalysisPass,
            "norm.peak scans the whole stream before applying gain",
        ),
        Some("reverse") => (
            PlanStepMode::WholeBufferBarrier,
            "reverse requires a full-buffer materialization",
        ),
        Some(_) => (PlanStepMode::Streaming, ""),
    }
}
