# Auralis

> A deterministic, testable, batch-oriented audio DSP engine for Rust and the command line.

Auralis is a from-scratch Rust audio processing project. It is not a literal
SoX clone, but it uses SoX-ng as the behavioral oracle for comparable command
line effects while rebuilding the core around deterministic behavior, typed
Rust APIs, explicit test contracts, and maintainable modules.

Initial scope is deliberately narrow: **WAV first**, with PCM8, PCM16, PCM24,
PCM32, float32, float64, u-law, A-law, and RIFX currently implemented. Raw PCM,
AIFF/AIFC, FLAC, and AU/SND decode/export are growing behind Auralis-owned pure Rust adapters.
Additional formats are planned only after effect and pipeline behavior are broad
and stable, and new codec backends must stay pure Rust unless a later
development-plan change says otherwise.

## Current Status

Auralis is pre-alpha. The repository already contains:

- a Rust workspace with `auralis`, `auralis-aiff`, `auralis-au`, `auralis-flac`,
  `auralis-core`, `auralis-wav`, `auralis-raw`, `auralis-dsp`,
  `auralis-effects`, `auralis-simd`, `auralis-testkit`, and `auralis-cli`
  crates;
- PCM8/PCM16/PCM24/PCM32/float32/float64/u-law/A-law WAV decode plus RIFX
  container decode, PCM16 legacy output, and
  PCM8/PCM16/PCM24/PCM32/float32/float64/u-law/A-law RIFF or RIFX output through
  the newer `OutputFormat::Wav(WavEncodeOptions)` boundary, alongside
  `auralis inspect`;
- `auralis render` for WAV processing with typed `--fx`/`--chain` effect
  inputs, effects files, input combiners, output channel/rate/level/dither
  policies, and many typed effects, with
  specialized/native-backed/format-boundary effects classified before
  inclusion;
- a deterministic Rust testkit, SoX-ng golden manifests with complex
  chain/boundary cases, L0-L7 layered coverage metadata, fuzz/parser seeds,
  optional Python helpers for cross-tool golden execution and reporting, and a
  7.x primitive ownership audit with reusable biquad, FIR, and deterministic
  dither/noise primitive extractions into `auralis-dsp`;
- an 8.0 codec-backend policy that keeps WAV and future format work behind
  Auralis-owned adapters, classifies AIFF/AIFC as optional pure Rust adapter
  work, FLAC as experimental pure Rust adapter work, closes external `ffmpeg`
  backends and native codec wrappers as not planned under the current roadmap;
- an Auralis-owned output-format boundary with `OutputFormat`,
  per-format encode option types, `AudioEncoder`/`EncodeSummary`, a WAV encoder
  adapter, raw signed/unsigned integer and float PCM export with explicit
  raw byte-order, bit-order, and nibble-order options through an
  Auralis-owned adapter, plain AIFF signed-integer PCM plus AIFC little-endian
  integer, float32/float64, and G.711 u-law/A-law decode/export through the
  pure Rust `aifc` adapter, FLAC decode/export through pure Rust
  `claxon`/`flacenc` adapters, and AU/SND PCM, float, and G.711 decode/export
  through an Auralis-owned container adapter;
- Python packaging is blocked until the Rust API, effect pipeline behavior,
  error model, buffer model, and binding documentation are stable enough for a
  public package contract.

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
