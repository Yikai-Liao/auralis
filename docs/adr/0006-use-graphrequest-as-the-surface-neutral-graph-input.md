# Use GraphRequest as the surface-neutral graph input

Auralis will use `GraphRequest` as the single public request model for building audio graphs from Rust helpers, TOML specs, CLI lowering, and future bindings. `auralis-graph` validates a `GraphRequest` against the op registry into a `ValidGraph`, then derives an `ExecutionPlan`; CLI effect strings, TOML parsing details, and Python call shapes must be lowered before entering graph validation.

`GraphRequest` contains sources, node requests, input bindings, edge parameters, and sinks. The core dynamic API is `add_node(NodeRequest)` / `add_sink(SinkRequest)` because runtime graph shape is not always known at compile time. Typed Rust helpers may exist, but they only construct `GraphRequest` entries.

**Examples**

Rust dynamic API:

```rust
let mut graph = GraphBuilder::new();
let voice = graph.source_file("voice", "input/voice.wav")?;
let music = graph.source_file("music", "input/music.wav")?;

let clean = graph.add_node(
    NodeRequest::new("clean", "gain")
        .input("in", voice.audio())
        .param("by", "-3dB"),
)?;

let bed = graph.add_node(
    NodeRequest::new("bed", "gain")
        .input("in", music.audio())
        .param("by", "-14dB"),
)?;

let mix = graph.add_node(
    NodeRequest::new("mix", "mix.sum")
        .input_with("inputs", clean.audio(), [("gain", "0dB")])
        .input_with("inputs", bed.audio(), [("gain", "0dB")]),
)?;

graph.add_sink(SinkRequest::file("master", mix.audio(), "build/master.wav"))?;
```

Rust typed helpers are sugar over the same request model:

```rust
let clean = graph
    .typed("clean", ops::gain(GainParams { by: db("-3dB")? }))
    .input("in", voice.audio())
    .finish()?;
```

TOML graph specs lower into the same `GraphRequest`:

```toml
version = "auralis.graph/v2"

[[sources]]
id = "voice"
path = "input/voice.wav"

[[sources]]
id = "music"
path = "input/music.wav"

[[nodes]]
id = "clean"
op = "gain"
input = "voice.audio"
by = "-3dB"

[[nodes]]
id = "mix"
op = "mix.sum"
inputs = [
  { port = "inputs", from = "clean.audio", gain = "0dB" },
  { port = "inputs", from = "music.audio", gain = "-14dB" },
]

[[sinks]]
id = "master"
input = "mix.audio"
path = "build/master.wav"
format = { container = "wav", sample = "pcm24", sample_rate = "48k" }
```

CLI lowering must stop at `GraphRequest`; graph never parses `--fx` strings:

```text
auralis render input.wav -o output.wav --fx gain,by=-3dB --plan --format json

CLI parse:
  --fx gain,by=-3dB

CLI lowering:
  NodeRequest { id: "fx_001", op: "gain", input: "input.audio", params: { by: "-3dB" } }

Graph receives:
  GraphRequest { sources, nodes, sinks }
```

Future Python bindings should follow the same model:

```python
g = auralis.Graph()
voice = g.source("voice", path="input/voice.wav")
music = g.source("music", path="input/music.wav")

clean = g.op("clean", "gain", input=voice.audio, by="-3dB")
mix = g.op("mix", "mix.sum", inputs=[
    {"from": clean.audio, "gain": "0dB"},
    {"from": music.audio, "gain": "-14dB"},
])

g.sink("master", mix.audio, path="build/master.wav")
```

Plan JSON is produced from `ExecutionPlan`, not from CLI-specific structures:

```json
{
  "version": "auralis.plan/v1",
  "mode": "offline-whole-buffer",
  "steps": [
    {
      "id": "op:clean",
      "kind": "op",
      "op": "gain",
      "inputs": ["voice.audio"],
      "outputs": ["clean.audio"],
      "release_after": ["voice.audio"]
    },
    {
      "id": "sink:master",
      "kind": "write",
      "input": "mix.audio",
      "release_after": ["mix.audio"]
    }
  ]
}
```

Planning has two phases:

```text
GraphRequest -> ValidationDiagnostics
GraphRequest -> ValidGraph -> ExecutionPlan
```

Validation must behave like a compiler front end. It checks graph shape, IDs,
ports, missing sources/sinks, unknown ops, missing parameters, unknown
parameters, invalid values, arity mismatches, type mismatches, cycles, duplicate
IDs, and unreachable or unused nodes where that matters. If validation fails,
`--plan` emits structured diagnostics and does not produce an execution plan.

Human diagnostics should be source-aware when the request came from TOML or CLI:

```text
error[E0204]: unknown op `gian`
  --> Auralis.toml:12:6
   |
12 | op = "gian"
   |      ^^^^^^ unknown op
   |
help: did you mean `gain`?
```

JSON diagnostics are part of the machine contract:

```json
{
  "version": "auralis.diagnostics/v1",
  "errors": [
    {
      "code": "unknown_op",
      "message": "unknown op `gian`",
      "span": { "file": "Auralis.toml", "line": 12, "column": 6 },
      "suggestions": ["gain"]
    }
  ]
}
```

Diagnostics have severity. `error` diagnostics prevent `ValidGraph` and
`ExecutionPlan` creation; `warning` diagnostics do not. Plan JSON with errors
contains diagnostics only. Plan JSON without errors contains diagnostics,
canonical graph information, and execution plan information.

Examples of errors include unknown ops, unknown parameters, missing required
inputs, invalid ports, invalid ranges, cycles, duplicate IDs, and no selected
target sink. Examples of warnings include unreachable nodes outside selected
targets, deprecated aliases that canonicalize cleanly, allowed overwrites, and
experimental ops.

The spelling and suggestion system should be tested directly and fuzzed. Fuzz
targets should cover malformed op names, near-miss parameter names, duplicate
IDs, invalid port references, random TOML fragments, and random CLI-lowered
`GraphRequest` values. Fuzzing should prove validation returns diagnostics
rather than panicking or accepting malformed graphs.

Parameter and input handling follows one boundary:

```text
NodeRequest { params, inputs } -> PreparedNode -> eval(inputs)
```

`NodeRequest.params` holds node-level configuration, while `InputBinding` holds
audio/report port references plus edge-local parameters such as a per-input mix
gain. Validation and prepare decode all parameter maps into typed params and
typed edge params before execution. `eval` receives prepared operations and
resolved whole-buffer inputs; it must not parse CLI strings, TOML values, or raw
parameter maps.

Typed params own defaults through ordinary Rust `Default` implementations or
explicit default functions. Field types express units, so `Db`, `Percent`, and
`Milliseconds` do not need duplicate `unit` metadata. `#[param(...)]` metadata
is for names, descriptions, aliases, and validation ranges, not for CLI-only
behavior. Auralis op parameters are named parameters across CLI, `GraphRequest`,
TOML, Rust helpers, and future bindings; positional effect parameters are not
part of the target model. Range and semantic parameter validation belongs to
graph validation/prepare so `--plan` can report invalid values before
execution.

Typed value parsing is shared. CLI, TOML, Rust helpers, and future bindings may
produce `ParamValue` values such as strings, numbers, booleans, arrays, or typed
values. `Db`, `Percent`, `Milliseconds`, and similar domain value types own
their parsing from those `ParamValue`s. Graph validation/prepare performs the
final conversion according to the params struct field type, so all frontends get
the same errors and `--plan` can report bad values before execution. `eval`
never sees raw strings or raw parameter maps.

`ParamValue` is an Auralis-owned structured value type, not `toml::Value` or
`serde_json::Value`. This keeps `GraphRequest` independent of any specific file
or wire format:

```rust
pub enum ParamValue {
    String(String),
    Number(f64),
    Integer(i64),
    Bool(bool),
    Db(Db),
    Percent(Percent),
    Milliseconds(Milliseconds),
    Array(Vec<ParamValue>),
    Object(BTreeMap<String, ParamValue>),
}
```

TOML, CLI, Rust helpers, and future bindings convert into `ParamValue` before
graph validation. The graph layer then converts `ParamValue` into typed params.
Typed Rust helpers may pass domain values directly through `From<T>`
implementations, for example `.param("by", Db::new(-3.0)?)`; CLI/TOML may pass
the same value as a string such as `"-3dB"`. Both paths converge during graph
validation/prepare.

The request layer must remain serializable and canonicalizable. `GraphRequest`,
`NodeRequest`, `InputBinding`, `SinkRequest`, and `ParamValue` need stable JSON
or equivalent serialization for plan output, diagnostics, fixtures, replay,
lockfiles, and future cache keys. Prepared runtime objects such as
`PreparedNode`, `PreparedOp`, `AudioBuffer`, and DSP state do not need to be
serializable.

Graph processing canonicalizes before planning:

```text
frontend input -> GraphRequest -> canonical GraphRequest -> ValidGraph -> ExecutionPlan
```

Canonicalization resolves op aliases to canonical op names, parameter aliases to
canonical parameter names, input sugar such as `input = "voice.audio"` into
explicit `InputBinding`s, missing parameter values into typed defaults, and map
ordering into stable output. Plan JSON, lockfiles, future cache keys, and test
fixtures use canonical values. Diagnostics should still retain enough source
span/origin information to point back to the original frontend input.

Machine-readable plan output uses the canonical graph and execution plan, not
the original frontend spelling. Original CLI/TOML/Python input spelling is used
only for diagnostics origin, spans, and suggestions.

Example:

```rust
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize, JsonSchema, OpParams)]
#[serde(default, deny_unknown_fields)]
pub struct GainParams {
    /// Gain amount.
    #[param(description = "Gain amount.")]
    pub by: Db,
}
```

```rust
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, JsonSchema, OpParams)]
#[serde(default, deny_unknown_fields)]
pub struct ReverbParams {
    #[param(description = "Output only the wet reverberated signal.")]
    pub wet_only: bool,

    #[param(range = 0.0..=100.0)]
    pub reverberance: Percent,

    #[param(range = 0.0..=100.0)]
    pub hf_damping: Percent,

    pub pre_delay: Milliseconds,
    pub wet_gain: Db,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            wet_only: false,
            reverberance: Percent::new_unchecked(50.0),
            hf_damping: Percent::new_unchecked(50.0),
            pre_delay: Milliseconds::ZERO,
            wet_gain: Db::ZERO,
        }
    }
}
```

**Considered Options**

- Make generic typed `add_node<T>` the core graph API: rejected because CLI, TOML, and future bindings discover graph structure at runtime.
- Let `auralis-graph` parse CLI effect syntax directly: rejected because graph must remain usable without CLI and must not depend on command-line syntax.
- Use a surface-neutral `GraphRequest`: accepted because it gives every entry point one shared lowering target before validation, planning, JSON plan output, and whole-buffer execution.
