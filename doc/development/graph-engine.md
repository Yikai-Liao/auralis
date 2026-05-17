---
kind: development-plan
status: active
---

# Graph Execution Engine

This file is the authoritative plan for the shared Auralis graph engine design.
The engine is a Rust library boundary used by CLI lowering, Rust helpers, TOML
graph specs, and future bindings.

## Core Pipeline

```text
GraphRequest -> ValidGraph -> ExecutionPlan -> whole-buffer render
```

Definitions:

| Term | Meaning |
| --- | --- |
| `GraphRequest` | frontend-neutral request built from CLI, TOML, Rust helpers, or future bindings |
| `ValidGraph` | request after op lookup, parameter validation, port checks, and DAG validation |
| `ExecutionPlan` | deterministic offline schedule, output actions, diagnostics, memory release points, and cache decisions |

The graph engine must not parse CLI strings, TOML syntax details, or Python call
shapes. Frontends lower those forms into `GraphRequest`.

## Scope

- Create `auralis-graph` for graph request validation, planning, and execution.
- Use `petgraph` internally for graph storage, traversal, topological ordering,
  cycle detection, and graph-debug output.
- Use `rayon` internally where deterministic offline parallelism is safe.
- Treat project graphs as finite audio DAGs.
- Keep feedback, delay-line state, recursive filters, and reverberation
  feedback internal to op implementations.
- Release temporary audio buffers promptly after their last planned use.

## Non-Goals

- No streaming graph execution.
- No chunked graph scheduler.
- No realtime graph API.
- No graph-level cycles in the initial project graph model.
- No CLI string parsing inside `auralis-graph`.
- No public dependency on `petgraph` or `rayon` types.

## Whole-Buffer Execution Model

Auralis graph execution renders complete audio artifacts:

```text
read complete source buffer
  -> evaluate complete op inputs
  -> produce complete op output buffers
  -> write complete sink artifacts
```

Whole-buffer execution is a product boundary, not a temporary limitation. It
keeps graph validation, cacheability, memory accounting, replay, and future Rust
API semantics simple and deterministic.

Allowed parallelism:

- independent DAG branches;
- independent sink branches after a fanout point;
- channel-level work where numerical ordering is defined;
- buffer-internal data-parallel kernels;
- fixed-order reductions.

Do not introduce cross-node streaming state, backpressure, realtime deadlines,
or chunk lifecycle contracts at the graph level.

## Request And Validation

`GraphRequest` contains:

- sources;
- node requests;
- input bindings;
- edge-local parameters such as per-input mix gain;
- sinks;
- selected target, if any;
- frontend source spans for diagnostics, when available.

Validation checks at least:

- duplicate IDs;
- unknown ops and aliases;
- unknown or missing parameters;
- invalid parameter values;
- unknown ports;
- arity and type mismatches;
- cycles;
- missing selected targets;
- unreachable nodes where the selected target makes them suspicious;
- sink overwrite and format-policy errors that can be known before execution.

Validation failures produce diagnostics and do not produce `ValidGraph` or an
`ExecutionPlan`.

## Parameter Boundary

Frontends may supply structured values such as strings, numbers, booleans,
arrays, objects, or already-typed domain values. Graph validation converts those
values into typed op parameters using op metadata from `auralis-op`.

`eval` receives prepared typed operations and resolved whole-buffer inputs. It
must not parse raw strings, TOML values, or CLI parameter maps.

## Execution Plan Contract

An `ExecutionPlan` should expose:

- source reads;
- node evaluation order;
- sink writes;
- fanout points;
- whole-buffer materialization points;
- cache read/write decisions;
- release-after points for temporary buffers;
- deterministic backend choices;
- diagnostics and warnings;
- estimated peak memory where the estimate is meaningful.

Plan JSON is a public machine contract. It should be versioned, stable enough
for snapshots, and independent of CLI-only structs.

Sketch:

```rust
pub struct ExecutionPlan {
    pub version: PlanVersion,
    pub mode: ExecutionMode, // offline whole-buffer
    pub steps: Vec<PlanStep>,
    pub diagnostics: Vec<Diagnostic>,
}

pub enum PlanStep {
    ReadSource { id: SourceId, release_after: Vec<BufferId> },
    EvaluateNode { id: NodeId, op: OpName, inputs: Vec<PortRef>, outputs: Vec<PortRef> },
    WriteSink { id: SinkId, input: PortRef, release_after: Vec<BufferId> },
    CacheRead { key: CacheKey, output: PortRef },
    CacheWrite { key: CacheKey, input: PortRef },
}
```

## Cache Policy

Cache policy is part of planning, not an ad hoc CLI behavior.

Candidate modes:

| Mode | Behavior |
| --- | --- |
| `off` | do not read or write persistent cache |
| `smart` | cache expensive, fanout, or selected boundary nodes |
| `full` | cache every deterministic cache-worthy node |
| `locked` | require cache and plan metadata to match lockfile decisions |

Cache keys should include input content, op name, op parameters, implementation
version, backend choice, sample format, sample rate, channel layout, and
deterministic seed where relevant.

## Implementation Phases

1. Move graph request, validation, planning, and execution code out of
   `auralis-cli` into `auralis-graph`.
2. Introduce `GraphRequest`, `ValidGraph`, and `ExecutionPlan` as the library
   boundary.
3. Use `auralis-op` descriptors for op lookup, parameter validation, and
   schema-driven diagnostics.
4. Port current graph-spec `run`, `graph`, `explain`, and planning behavior to
   the new crate without changing user-facing behavior.
5. Add deterministic branch-level and buffer-level parallelism only after the
   sequential whole-buffer plan is correct and covered.
