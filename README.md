# Auralis

> A deterministic, testable, batch-oriented audio DSP engine for Rust and the command line.

Auralis is a from-scratch Rust audio processing project. It is not a literal
SoX clone, but it uses SoX-ng as the behavioral oracle for comparable command
line effects while rebuilding the core around deterministic behavior, typed
Rust APIs, explicit test contracts, and maintainable modules.

Initial scope is deliberately narrow: **PCM16 WAV only**. Additional formats are
planned only after effect and pipeline behavior are broad and stable, and new
codec backends must stay pure Rust unless a later development-plan change says
otherwise.

## Current Status

Auralis is pre-alpha. The repository already contains:

- a Rust workspace with `auralis`, `auralis-core`, `auralis-wav`,
  `auralis-dsp`, `auralis-effects`, `auralis-simd`, `auralis-testkit`, and
  `auralis-cli` crates;
- PCM16 WAV decode/encode and `auralis inspect`;
- `auralis run` with positional SoX-ng-style effect chains, effects files,
  input combiners, output channel/rate/level/dither policies, and many typed
  effects, with specialized/native-backed/format-boundary effects classified
  before inclusion;
- a deterministic Rust testkit, SoX-ng golden manifests with complex
  chain/boundary cases, L0-L7 layered coverage metadata, fuzz/parser seeds,
  optional Python helpers for cross-tool golden execution and reporting, and a
  7.x primitive ownership audit with reusable biquad, FIR, and deterministic
  dither/noise primitive extractions into `auralis-dsp`;
- an 8.0 codec-backend policy that keeps WAV and future format work behind
  Auralis-owned adapters, classifies AIFF/AIFC as optional pure Rust adapter
  work, FLAC as experimental pure Rust adapter work, and native or `ffmpeg`
  codec backends as not planned under the current roadmap.

Detailed status is kept in [doc/status.md](doc/status.md). Development planning
lives in [DEVELOPMENT.md](DEVELOPMENT.md) and [doc/development/](doc/development/).
Effect-by-effect status intentionally lives outside this README.

## Documentation Map

- [doc/architecture.md](doc/architecture.md): architecture, crate boundaries,
  API rules, dependency policy, and future Python-package constraints.
- [doc/testing.md](doc/testing.md): testing philosophy, L0-L7 layers,
  SoX-ng golden rules, metrics, fuzzing, sanitizers, and coverage reporting.
- [doc/development-commands.md](doc/development-commands.md): local commands
  for formatting, tests, uv/pytest, and milestone validation.
- [doc/development/06-effect-coverage.md](doc/development/06-effect-coverage.md):
  the 6.x effect-coverage index; detailed milestones are split under
  `doc/development/06-effects/`.

## Local Layout

The nearby `sox_ng` checkout is used only as a reference implementation for
golden tests. It is not vendored into Auralis and must not shape public APIs.

```text
~/code/sox-rs/
├── auralis/     # this repository
└── sox_ng/      # reference implementation, globally installed as sox_ng
```

Set `AURALIS_SOX_NG_BIN` when the reference binary is not discoverable from
`PATH`:

```bash
export AURALIS_SOX_NG_BIN=/usr/local/bin/sox_ng
```

## Quick Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
cd tools/pytest && uv run pytest
```

SoX-ng golden jobs must run with a real `sox_ng` binary. Local exploratory runs
may skip when it is absent, but release and gnhf validation must treat a missing
oracle as a failure.

## Non-goals for the initial phase

- Full SoX feature parity.
- Real-time audio I/O.
- GPU acceleration.
- Full codec support.
- Python package release.
- Perfect byte-for-byte WAV reproduction across arbitrary metadata.
- SIMD before scalar correctness.

---

## License

See `LICENSE`.
