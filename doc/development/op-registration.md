---
kind: development-plan
status: active
---

# Op Registration

This file is the migration target for the `auralis-op` contract crate and
distributed operation registration.

## Target Boundary

`auralis-op` owns the shared operator vocabulary:

- canonical op names and aliases;
- parameter descriptors;
- input and output port descriptors;
- capability metadata;
- prepared parameter decoding contracts;
- schema and example metadata;
- registry construction and validation.

It does not own graph requests, graph planning, CLI parsing, DSP kernels, codec
I/O, or execution policy.

## Dependency Direction

Target dependency rules:

- `auralis-effects` depends on `auralis-op` to declare and register concrete
  ops.
- `auralis-graph` depends on `auralis-op` to validate and prepare nodes.
- `auralis-cli` depends on `auralis-op` only for discovery, help, schema, and
  lowering support.
- `auralis-op` must not depend on `auralis-cli`, `auralis-graph`, or concrete
  effect implementation crates.

This prevents effect crates from depending on graph planning and prevents graph
validation from depending directly on every concrete effect module.

## Registration Mechanism

Use `linkme` behind Auralis-owned APIs or macros for compile-time distributed
registration.

Rules:

- all graph ops are registered by default when their crate is linked;
- catalog construction sorts by canonical op name;
- duplicate canonical names are test failures;
- duplicate aliases are test failures;
- aliases must not collide with canonical names;
- CLI availability is controlled by Cargo features and runtime configuration,
  not by per-op declaration forks.

Because linker slices do not prove uniqueness during Rust type checking,
catalog validation must run in ordinary `cargo test`, CI, release, and gnhf
gates.

## Descriptor Shape

Every op descriptor should provide at least:

```rust
pub struct OpDescriptor {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub category: OpCategory,
    pub inputs: &'static [PortDescriptor],
    pub outputs: &'static [PortDescriptor],
    pub params: &'static [ParamDescriptor],
    pub capabilities: OpCapabilities,
    pub examples: &'static [OpExample],
}
```

Parameter descriptors are named. Positional effect parameters are migration
debt, not a target model.

## Named Parameter Contract

The same named parameter model is used by:

- CLI `--fx`;
- `GraphRequest`;
- TOML graph specs;
- Rust helpers;
- future bindings;
- schema generation;
- diagnostics;
- tests.

Example CLI:

```bash
auralis render input.wav -o output.wav --fx gain,by=-3dB
```

Example graph spec:

```toml
[[nodes]]
id = "clean"
op = "gain"
input = "voice.audio"
by = "-3dB"
```

The CLI may carry `"-3dB"` as a raw parameter value. Graph validation and op
preparation convert it into the typed `Db` field used by the concrete op.

## `ops` UX

`auralis ops` is the public registry inspection surface:

```bash
auralis ops
auralis ops gain
auralis ops gain --schema json
```

Human output for one op should include:

- purpose;
- inputs and outputs;
- parameters and accepted units;
- whole-buffer behavior;
- determinism;
- CLI examples;
- TOML examples;
- test/oracle notes when useful.

Machine-readable schema output should be generated from the same descriptor
metadata used by graph validation. It must not be a hand-maintained parallel
schema.

## Required Registry Tests

The registry test suite must check:

- canonical op-name uniqueness;
- alias uniqueness;
- aliases do not collide with canonical names;
- stable sorted catalog output;
- complete input and output signatures;
- complete parameter descriptors;
- default parameter decoding;
- schema generation for plan and ops JSON;
- descriptor examples parse into valid `GraphRequest` fragments.

## Migration Steps

1. Create the `auralis-op` crate with descriptors and registry validation.
2. Hide `linkme` behind project-owned macros or functions.
3. Register a small set of existing effects such as `gain`, `trim`, and
   `fade`.
4. Route `ops` output through the catalog.
5. Route graph validation through the catalog.
6. Migrate remaining effects as their flat effect documents are written.

## Non-Goals

- No large central registration list for every effect.
- No runtime plugin loading.
- No CLI dependency from effect implementations.
- No graph-planner dependency from concrete effect implementations.
- No positional parameters in new op descriptors.
