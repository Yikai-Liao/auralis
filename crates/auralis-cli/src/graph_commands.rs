use std::{fmt::Write as _, fs, path::Path};

use serde::Serialize;

use crate::{CliError, GraphFormat, graph_plan, spec};

pub(super) fn graph_spec(
    spec: &Path,
    output: Option<&Path>,
    format: GraphFormat,
) -> Result<(), CliError> {
    let checked = spec::check_graph_spec(spec)?;
    let rendered = render_graph(&checked, format)?;
    if let Some(output) = output {
        fs::write(output, rendered)?;
    } else {
        print!("{rendered}");
    }

    Ok(())
}

pub(super) fn format_graph_spec(spec: &Path, check: bool) -> Result<(), CliError> {
    let formatted = spec::format_graph_spec(spec)?;
    let current = fs::read_to_string(spec).map_err(|error| spec::GraphSpecError::Read {
        path: spec.to_path_buf(),
        error,
    })?;
    if check {
        if current == formatted {
            return Ok(());
        }
        return Err(CliError::GraphSpecNeedsFormatting {
            path: spec.to_path_buf(),
        });
    }

    if current != formatted {
        fs::write(spec, formatted)?;
    }
    Ok(())
}

pub(super) fn explain_graph_target(spec: &Path, target: &str) -> Result<(), CliError> {
    let checked = spec::check_graph_spec(spec)?;

    if let Some(source) = checked.sources.iter().find(|source| source.id == target) {
        println!("Node: {}", source.id);
        println!("Kind: source");
        println!("Path:");
        println!("  {}", source.path.display());
        println!("Output port:");
        println!("  {}.audio", source.id);
        print_downstream(&checked, &format!("{}.audio", source.id));
        return Ok(());
    }

    if let Some(chain) = checked.chains.iter().find(|chain| chain.id == target) {
        println!("Node: {}", chain.id);
        println!("Kind: chain");
        println!("Input:");
        println!("  {}", chain.input);
        println!("Expanded steps:");
        for step_id in &chain.step_ids {
            println!("  {step_id}");
        }
        println!("Execution mode:");
        for (step_id, label) in chain.step_ids.iter().zip(chain.step_labels.iter()) {
            let (mode, reason) = graph_plan::classify_plan_step(label);
            if reason.is_empty() {
                println!("  {step_id:<24} {}", mode.label());
            } else {
                println!("  {step_id:<24} {} ({reason})", mode.label());
            }
        }
        print_downstream(&checked, &format!("{}.audio", chain.id));
        return Ok(());
    }

    if let Some(node) = checked.nodes.iter().find(|node| node.id == target) {
        println!("Node: {}", node.id);
        println!("Kind: node");
        println!("Op:");
        println!("  {}", graph_plan::node_display_label(node));
        println!("Inputs:");
        for input in &node.inputs {
            println!("  {input}");
        }
        println!("Execution mode:");
        let (mode, reason) = graph_plan::classify_node_plan_mode(node);
        if reason.is_empty() {
            println!("  {}", mode.label());
        } else {
            println!("  {} ({reason})", mode.label());
        }
        print_downstream(&checked, &format!("{}.audio", node.id));
        return Ok(());
    }

    if let Some(sink) = checked.sinks.iter().find(|sink| sink.id == target) {
        println!("Node: {}", sink.id);
        println!("Kind: sink");
        println!("Input:");
        println!("  {}", sink.input);
        println!("Path:");
        println!("  {}", sink.path.display());
        println!("Downstream:");
        println!("  none");
        return Ok(());
    }

    Err(CliError::UnknownExplainTarget {
        target: target.to_owned(),
    })
}

fn print_downstream(checked: &spec::CheckedGraphSpec, port: &str) {
    println!("Downstream:");
    let downstream = downstream_consumers(checked, port);
    if downstream.is_empty() {
        println!("  none");
        return;
    }
    for consumer in downstream {
        println!("  {consumer}");
    }
}

fn render_graph(checked: &spec::CheckedGraphSpec, format: GraphFormat) -> Result<String, CliError> {
    match format {
        GraphFormat::Mermaid => Ok(render_mermaid_graph(checked)),
        GraphFormat::Dot => Ok(render_dot_graph(checked)),
        GraphFormat::Svg => Ok(render_svg_graph(checked)),
        GraphFormat::Json => render_json_graph(checked),
    }
}

fn render_mermaid_graph(checked: &spec::CheckedGraphSpec) -> String {
    let mut output = String::new();
    writeln!(&mut output, "flowchart LR").expect("writing to String cannot fail");
    for source in &checked.sources {
        writeln!(
            &mut output,
            "  {}[\"source: {}\"]",
            mermaid_id(&source.id),
            source.path.display()
        )
        .expect("writing to String cannot fail");
    }
    for chain in &checked.chains {
        let mut previous = chain.input.strip_suffix(".audio").unwrap_or(&chain.input);
        for (step_id, label) in chain.step_ids.iter().zip(chain.step_labels.iter()) {
            let node_id = mermaid_id(step_id);
            writeln!(&mut output, "  {node_id}[\"{label}\"]")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "  {} --> {node_id}", mermaid_id(previous))
                .expect("writing to String cannot fail");
            previous = step_id;
        }
    }
    for node in &checked.nodes {
        writeln!(
            &mut output,
            "  {}[\"{}\"]",
            mermaid_id(&node.id),
            graph_plan::node_display_label(node)
        )
        .expect("writing to String cannot fail");
        for input in &node.inputs {
            let upstream = sink_upstream(checked, strip_audio_suffix(input));
            writeln!(
                &mut output,
                "  {} --> {}",
                mermaid_id(upstream),
                mermaid_id(&node.id)
            )
            .expect("writing to String cannot fail");
        }
    }
    for sink in &checked.sinks {
        writeln!(
            &mut output,
            "  {}[\"sink: {}\"]",
            mermaid_id(&sink.id),
            sink.path.display()
        )
        .expect("writing to String cannot fail");
        let input = sink.input.strip_suffix(".audio").unwrap_or(&sink.input);
        let upstream = sink_upstream(checked, input);
        writeln!(
            &mut output,
            "  {} --> {}",
            mermaid_id(upstream),
            mermaid_id(&sink.id)
        )
        .expect("writing to String cannot fail");
    }
    output
}

fn render_dot_graph(checked: &spec::CheckedGraphSpec) -> String {
    let mut output = String::new();
    writeln!(&mut output, "digraph Auralis {{").expect("writing to String cannot fail");
    writeln!(&mut output, "  rankdir=LR;").expect("writing to String cannot fail");
    for source in &checked.sources {
        writeln!(
            &mut output,
            "  {} [label={}];",
            dot_id(&source.id),
            dot_label(&format!("source: {}", source.path.display()))
        )
        .expect("writing to String cannot fail");
    }
    for chain in &checked.chains {
        let mut previous = chain.input.strip_suffix(".audio").unwrap_or(&chain.input);
        for (step_id, label) in chain.step_ids.iter().zip(chain.step_labels.iter()) {
            writeln!(
                &mut output,
                "  {} [label={}];",
                dot_id(step_id),
                dot_label(label)
            )
            .expect("writing to String cannot fail");
            writeln!(
                &mut output,
                "  {} -> {};",
                dot_id(previous),
                dot_id(step_id)
            )
            .expect("writing to String cannot fail");
            previous = step_id;
        }
    }
    for node in &checked.nodes {
        writeln!(
            &mut output,
            "  {} [label={}];",
            dot_id(&node.id),
            dot_label(&graph_plan::node_display_label(node))
        )
        .expect("writing to String cannot fail");
        for input in &node.inputs {
            writeln!(
                &mut output,
                "  {} -> {};",
                dot_id(sink_upstream(checked, strip_audio_suffix(input))),
                dot_id(&node.id)
            )
            .expect("writing to String cannot fail");
        }
    }
    for sink in &checked.sinks {
        writeln!(
            &mut output,
            "  {} [label={}];",
            dot_id(&sink.id),
            dot_label(&format!("sink: {}", sink.path.display()))
        )
        .expect("writing to String cannot fail");
        let input = sink.input.strip_suffix(".audio").unwrap_or(&sink.input);
        writeln!(
            &mut output,
            "  {} -> {};",
            dot_id(sink_upstream(checked, input)),
            dot_id(&sink.id)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(&mut output, "}}").expect("writing to String cannot fail");
    output
}

#[derive(Debug, Serialize)]
struct JsonGraph {
    nodes: Vec<JsonGraphNode>,
    edges: Vec<JsonGraphEdge>,
}

#[derive(Debug, Serialize)]
struct JsonGraphNode {
    id: String,
    kind: &'static str,
    label: String,
}

#[derive(Debug, Serialize)]
struct JsonGraphEdge {
    from: String,
    to: String,
}

fn render_json_graph(checked: &spec::CheckedGraphSpec) -> Result<String, CliError> {
    let graph = build_json_graph(checked);
    Ok(format!("{}\n", serde_json::to_string_pretty(&graph)?))
}

fn build_json_graph(checked: &spec::CheckedGraphSpec) -> JsonGraph {
    let mut graph = JsonGraph {
        nodes: Vec::new(),
        edges: Vec::new(),
    };

    for source in &checked.sources {
        graph.nodes.push(JsonGraphNode {
            id: source.id.clone(),
            kind: "source",
            label: source.path.display().to_string(),
        });
    }

    for chain in &checked.chains {
        let mut previous = chain.input.strip_suffix(".audio").unwrap_or(&chain.input);
        for (step_id, label) in chain.step_ids.iter().zip(chain.step_labels.iter()) {
            graph.nodes.push(JsonGraphNode {
                id: step_id.clone(),
                kind: "step",
                label: label.clone(),
            });
            graph.edges.push(JsonGraphEdge {
                from: previous.to_string(),
                to: step_id.clone(),
            });
            previous = step_id;
        }
    }

    for node in &checked.nodes {
        graph.nodes.push(JsonGraphNode {
            id: node.id.clone(),
            kind: "node",
            label: graph_plan::node_display_label(node),
        });
        for input in &node.inputs {
            graph.edges.push(JsonGraphEdge {
                from: sink_upstream(checked, strip_audio_suffix(input)).to_string(),
                to: node.id.clone(),
            });
        }
    }

    for sink in &checked.sinks {
        graph.nodes.push(JsonGraphNode {
            id: sink.id.clone(),
            kind: "sink",
            label: sink.path.display().to_string(),
        });
        let input = sink.input.strip_suffix(".audio").unwrap_or(&sink.input);
        graph.edges.push(JsonGraphEdge {
            from: sink_upstream(checked, input).to_string(),
            to: sink.id.clone(),
        });
    }

    graph
}

fn render_svg_graph(checked: &spec::CheckedGraphSpec) -> String {
    let graph = build_json_graph(checked);
    let card_width = 200_i32;
    let card_height = 44_i32;
    let gap = 28_i32;
    let margin = 24_i32;
    let width = margin * 2 + card_width;
    let node_count = coordinate_index(graph.nodes.len());
    let height = margin * 2
        + node_count
            .saturating_mul(card_height + gap)
            .saturating_sub(gap);

    let mut output = String::new();
    append_svg_header(&mut output, width, height);

    let positions = svg_node_positions(&graph, margin, card_height, gap);
    append_svg_edges(&mut output, &graph, &positions, card_width, card_height);
    append_svg_nodes(&mut output, &graph, &positions, card_width, card_height);

    writeln!(&mut output, "</svg>").expect("writing to String cannot fail");
    output
}

fn append_svg_header(output: &mut String, width: i32, height: i32) {
    writeln!(
        output,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
    )
    .expect("writing to String cannot fail");
    writeln!(output, "  <style>").expect("writing to String cannot fail");
    writeln!(
        output,
        "    text {{ font-family: monospace; font-size: 12px; fill: #111827; }}"
    )
    .expect("writing to String cannot fail");
    writeln!(output, "    .kind {{ font-size: 10px; fill: #6b7280; }}")
        .expect("writing to String cannot fail");
    writeln!(output, "    .source {{ fill: #dbeafe; stroke: #2563eb; }}")
        .expect("writing to String cannot fail");
    writeln!(output, "    .step {{ fill: #dcfce7; stroke: #16a34a; }}")
        .expect("writing to String cannot fail");
    writeln!(output, "    .node {{ fill: #f3e8ff; stroke: #7c3aed; }}")
        .expect("writing to String cannot fail");
    writeln!(output, "    .sink {{ fill: #fee2e2; stroke: #dc2626; }}")
        .expect("writing to String cannot fail");
    writeln!(
        output,
        "    .edge {{ stroke: #94a3b8; stroke-width: 2; fill: none; }}"
    )
    .expect("writing to String cannot fail");
    writeln!(output, "  </style>").expect("writing to String cannot fail");
    writeln!(output, "  <defs>").expect("writing to String cannot fail");
    writeln!(
        output,
        "    <marker id=\"arrow\" markerWidth=\"10\" markerHeight=\"10\" refX=\"8\" refY=\"3\" orient=\"auto\">"
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "      <path d=\"M0,0 L0,6 L9,3 z\" fill=\"#94a3b8\" />"
    )
    .expect("writing to String cannot fail");
    writeln!(output, "    </marker>").expect("writing to String cannot fail");
    writeln!(output, "  </defs>").expect("writing to String cannot fail");
}

fn svg_node_positions(
    graph: &JsonGraph,
    margin: i32,
    card_height: i32,
    gap: i32,
) -> std::collections::BTreeMap<String, (i32, i32)> {
    let mut positions = std::collections::BTreeMap::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        let x = margin;
        let y = margin + coordinate_index(index).saturating_mul(card_height + gap);
        positions.insert(node.id.clone(), (x, y));
    }
    positions
}

fn append_svg_edges(
    output: &mut String,
    graph: &JsonGraph,
    positions: &std::collections::BTreeMap<String, (i32, i32)>,
    card_width: i32,
    card_height: i32,
) {
    for edge in &graph.edges {
        let Some(&(from_x, from_y)) = positions.get(&edge.from) else {
            continue;
        };
        let Some(&(to_x, to_y)) = positions.get(&edge.to) else {
            continue;
        };
        let x1 = from_x + card_width / 2;
        let y1 = from_y + card_height;
        let x2 = to_x + card_width / 2;
        let y2 = to_y;
        writeln!(
            output,
            "  <path class=\"edge\" marker-end=\"url(#arrow)\" d=\"M{x1} {y1} L{x2} {y2}\" />"
        )
        .expect("writing to String cannot fail");
    }
}

fn append_svg_nodes(
    output: &mut String,
    graph: &JsonGraph,
    positions: &std::collections::BTreeMap<String, (i32, i32)>,
    card_width: i32,
    card_height: i32,
) {
    for node in &graph.nodes {
        let (x, y) = positions[&node.id];
        let label = xml_escape(&node.label);
        let kind = xml_escape(node.kind);
        writeln!(
            output,
            "  <rect class=\"{}\" x=\"{x}\" y=\"{y}\" width=\"{card_width}\" height=\"{card_height}\" rx=\"10\" />",
            node.kind
        )
        .expect("writing to String cannot fail");
        writeln!(
            output,
            "  <text class=\"kind\" x=\"{}\" y=\"{}\">{kind}</text>",
            x + 12,
            y + 16
        )
        .expect("writing to String cannot fail");
        writeln!(
            output,
            "  <text x=\"{}\" y=\"{}\">{label}</text>",
            x + 12,
            y + 31
        )
        .expect("writing to String cannot fail");
    }
}

fn mermaid_id(id: &str) -> String {
    id.chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => character,
            _ => '_',
        })
        .collect()
}

fn strip_audio_suffix(port: &str) -> &str {
    port.strip_suffix(".audio").unwrap_or(port)
}

fn sink_upstream<'a>(checked: &'a spec::CheckedGraphSpec, input: &'a str) -> &'a str {
    checked
        .chains
        .iter()
        .find(|chain| chain.id == input)
        .and_then(|chain| chain.step_ids.last())
        .map_or(input, String::as_str)
}

fn downstream_consumers(checked: &spec::CheckedGraphSpec, port: &str) -> Vec<String> {
    let mut consumers = Vec::new();
    for chain in &checked.chains {
        if chain.input == port {
            consumers.push(chain.id.clone());
        }
    }
    for node in &checked.nodes {
        if node.inputs.iter().any(|input| input == port) {
            consumers.push(node.id.clone());
        }
    }
    for sink in &checked.sinks {
        if sink.input == port {
            consumers.push(sink.id.clone());
        }
    }
    consumers
}

fn dot_id(id: &str) -> String {
    dot_label(id)
}

fn dot_label(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn coordinate_index(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
