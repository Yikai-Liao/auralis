---
kind: historical-roadmap
status: superseded
superseded_by:
  - ../../DEVELOPMENT.md
  - README.md
---

# 0.x Bootstrap Plan

This file contains the implemented repository-bootstrap features. These entries
are kept for history and acceptance traceability; new work should continue from
the priority order in the root [`DEVELOPMENT.md`](../../DEVELOPMENT.md).
It is not an authoritative planning surface.

## Feature 0.1: workspace skeleton

Status: implemented.

Create:

```text
Cargo.toml
rust-toolchain.toml
crates/auralis-core
crates/auralis
crates/auralis-codec
crates/auralis-wav
crates/auralis-dsp
crates/auralis-effects
crates/auralis-simd
crates/auralis-cli
crates/auralis-testkit
tools/pytest
```

Acceptance tests:

- `cargo metadata` succeeds.
- `cargo test --workspace` succeeds.
- `cargo doc --workspace --no-deps` succeeds.
- Empty `uv` project can run `uv run pytest` with one placeholder test.

## Feature 0.2: core error and type vocabulary

Status: implemented.

Implement in `auralis-core`:

- `AuralisError`
- `Result<T>`
- `SampleRate`
- `ChannelCount`
- `FrameCount`
- `Hertz`
- `Decibels`
- `TimeSeconds`
- `SampleFormat`
- `AudioSpec`

Acceptance tests:

- valid and invalid sample rates;
- valid and invalid channel counts;
- display/debug formatting;
- doc examples compile.

## Feature 0.3: internal audio buffer

Status: implemented.

Implement:

- `AudioBuffer`
- planar `f32` storage
- frame/channel indexing
- safe channel views
- mutable channel views
- zero initialization
- shape validation

Acceptance tests:

- mono buffer layout;
- stereo buffer layout;
- channel view mutation;
- invalid data length rejection;
- zero-length buffer handling;
- doc examples compile.
