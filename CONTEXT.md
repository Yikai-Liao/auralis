# Auralis Context

Auralis is an audio DSP foundation library and command-line tool. This glossary fixes project language for user intent, audio processing graphs, execution planning, and compatibility boundaries.

## Language

**Graph Execution Engine**:
The shared audio-processing engine that turns user intent into an executable graph plan.
_Avoid_: inference engine, CLI engine, command runner

**Audio DAG**:
A finite directed acyclic graph of audio sources, processing nodes, and sinks.
_Avoid_: feedback graph, realtime graph

**GraphRequest**:
A surface-neutral request to build an audio graph from sources, node requests, input bindings, and sinks.
_Avoid_: CLI spec, TOML spec, Python graph

**ValidGraph**:
A checked **Audio DAG** whose node IDs, ports, op names, parameters, and sink targets have been resolved.
_Avoid_: parsed spec, unchecked graph

**ExecutionPlan**:
The ordered whole-buffer work plan derived from a **ValidGraph**.
_Avoid_: dry run, command list

**ValidationDiagnostics**:
Compiler-style messages produced when a **GraphRequest** cannot become a **ValidGraph**.
_Avoid_: runtime errors, plan output

**Offline Rendering**:
Processing finite audio artifacts as complete jobs with reproducible outputs.
_Avoid_: streaming, chunked processing, realtime processing

## Relationships

- A **Graph Execution Engine** serves CLI commands, Rust library calls, and future language bindings through one conceptual execution model.
- CLI, TOML, Rust helpers, and future bindings produce a **GraphRequest**.
- A **GraphRequest** validates into a **ValidGraph** or produces **ValidationDiagnostics**.
- A **ValidGraph** plans into an **ExecutionPlan**.
- A **Graph Execution Engine** executes an **Audio DAG**; feedback inside an effect is not modeled as a graph edge.
- Auralis uses **Offline Rendering** for graph execution.

## Example Dialogue

> **Dev:** "Should `plan` have its own engine?"
> **Domain expert:** "No. `plan` should inspect the same **Graph Execution Engine** that `run` and library calls use."

> **Dev:** "Does a reverb feedback loop make the project graph cyclic?"
> **Domain expert:** "No. That feedback is internal to one effect; the project-level model remains an **Audio DAG**."

> **Dev:** "Should the graph executor plan chunked streaming?"
> **Domain expert:** "No. Auralis targets **Offline Rendering** over finite audio artifacts."

> **Dev:** "Should CLI pass `--fx` strings into graph?"
> **Domain expert:** "No. CLI lowers user input into a **GraphRequest** before graph validation."

## Flagged Ambiguities

- "inference engine" was used to mean audio DSP execution, not machine-learning inference. Resolved: use **Graph Execution Engine**.
