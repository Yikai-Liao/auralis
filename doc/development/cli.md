---
kind: development-plan
status: active
---

# CLI Development Plan

This file is the authoritative plan for the lightweight CLI design. It
describes the target shape, not the current implementation snapshot.

## Current Migration Debt

`crates/auralis-cli` still has many top-level recipe commands, per-command
argument structs, and a mirrored `plan` command path. Those are useful shipped
capabilities and migration references, but they are not the long-term CLI
architecture.

New CLI work should move toward one lowering path:

```text
CLI argv -> frontend request -> GraphRequest -> ValidGraph -> ExecutionPlan
```

CLI code must not call DSP internals directly for new graph-backed behavior.
Graph code must not parse CLI strings.

## Target Interaction Layers

| Layer | User intent | Target entry point | Lowering |
| --- | --- | --- | --- |
| Recipe | common result | `convert`, focused aliases where kept | `GraphRequest` |
| Linear chain | ordered transforms | `render --fx`, optional `--chain` | `GraphRequest` |
| Graph spec | named dependency graph | `run Auralis.toml` | `GraphRequest` |

The layers are user-facing conveniences only. They must converge before graph
validation so validation, planning, diagnostics, execution, and future Rust API
helpers share behavior.

## Target Command Surface

Primary long-term commands:

| Command | Purpose |
| --- | --- |
| `convert` | format and boundary conversion for one input |
| `render` | ordered DSP over one primary stream plus explicit input policy |
| `pipe` | compact exploratory expression, optional until parser value is clear |
| `run` | execute a graph spec or selected graph target |
| `graph` | emit graph views such as mermaid, dot, svg, or JSON |
| `explain` | explain why a node or target participates in a plan |
| `ops` | list ops and emit schema metadata |
| `fmt` | format graph specs |
| `init` | create a project graph scaffold |
| `cache` | inspect and manage graph cache state |
| `inspect` | inspect audio inputs and artifacts |

Validation and planning are execution modes, not separate mirrored command
families:

```bash
auralis convert input.wav -o output.flac --plan --format json
auralis render input.wav -o output.wav --fx gain,by=-3dB --check
auralis run Auralis.toml#master --plan --format json
```

`--check` and `--plan` are mutually exclusive. The mode is exactly one of
execute, check, or plan.

## Effect Syntax

The target `--fx` syntax is one comma-separated op expression per flag:

```bash
auralis render input.wav -o output.wav \
  --fx trim,range=10s..30s \
  --fx gain,by=-3dB \
  --fx fade,out=500ms,curve=linear
```

Rules:

- the first segment is the op name;
- remaining segments are `key=value` named parameters;
- positional effect parameters are not part of the target model;
- CLI lowering preserves values such as `"-3dB"` and `"500ms"` as parameter
  values, then graph validation performs typed parsing;
- output policy such as sample rate, channel count, sample format, and
  overwrite behavior is separate from ordered DSP effects.

An optional `--chain` form may exist for hand-written exploration, but it must
lower into the same ordered op list:

```bash
auralis render input.wav -o output.wav \
  --chain 'trim,range=10s..30s | gain,by=-3dB | fade,out=500ms'
```

Do not add new SoX-style positional effect syntax.

## CLI Lowering Contract

Each executable command builds a `GraphRequest`:

| Command | Lowering responsibility |
| --- | --- |
| `convert` | source, output-boundary sink, no DSP nodes unless policy requires one |
| `render` | source, ordered op nodes, output sink, output policy |
| `run` | parsed graph spec lowered to `GraphRequest` |
| `pipe` | parsed expression lowered to `GraphRequest` if the command ships |

The CLI owns:

- shell argument parsing;
- command-level defaults;
- source-span preservation for diagnostics;
- rendering human and JSON output;
- exit codes.

The graph engine owns:

- op lookup;
- parameter validation;
- graph validation;
- plan construction;
- graph execution.

## Output Contracts

Human output should be concise. Machine-readable output must be stable enough
for tests, CI, and downstream tooling.

Required JSON surfaces:

```bash
auralis convert input.wav -o output.flac --plan --format json
auralis render input.wav -o output.wav --fx gain,by=-3dB --plan --format json
auralis run Auralis.toml --plan --format json
auralis graph Auralis.toml --format json
auralis explain Auralis.toml#master node_id --json
auralis ops gain --schema json
```

Plan JSON must come from `ExecutionPlan`, not from CLI-only structures.

## Migration Steps

1. Add shared execution-mode flags to executable command args.
2. Route `convert`, `render`, and graph-spec `run` through shared
   `GraphRequest` lowering.
3. Move current mirrored `plan` command behavior behind `--plan` on the real
   execution commands.
4. Stop adding per-effect top-level commands for new work; use registry-backed
   `render --fx` and graph specs instead.
5. Keep existing recipe aliases only as explicit compatibility or convenience
   surfaces until a narrower deprecation plan exists.
6. Snapshot-test human output and schema-test JSON output.

## Non-Goals

- No graph parsing of CLI strings.
- No duplicated plan-mode parser.
- No new SoX-style positional effect parameter surface.
- No direct CLI dependency from effect implementations.
- No Python-driven CLI design until `doc/development/python.md` moves out of
  `not-planned`.
