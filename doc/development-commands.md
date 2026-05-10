# Auralis Development Commands

## Development commands

Rust:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
cargo bench
```

Python test environment must use `uv`:

```bash
cd tools/pytest
uv sync
uv run pytest
```

Do not use ad-hoc system `pip` installs for project tests.

Python is an auxiliary test layer, not the default place for new behavior
coverage. Prefer Rust `auralis-testkit` integration tests for durable SoX-ng
golden and complex pipeline cases when practical, then keep Python for
cross-tool execution, numerical checks, and failure artifacts. Python must not
be the only critical behavior check unless the feature records why a Rust test
would be impractical.

Release and gnhf golden validation must run with a real SoX-ng oracle:

```bash
export AURALIS_SOX_NG_BIN=${AURALIS_SOX_NG_BIN:-/usr/local/bin/sox_ng}
cargo test --workspace --all-features golden
cd tools/pytest
uv run pytest -m golden
```

The Rust `golden` test filter and pytest `-m golden` gate fail immediately when
the oracle is missing. Set `AURALIS_REQUIRE_SOX_NG=1` to force the same Rust
oracle check during a broader test invocation.

Pipeline-sensitive milestones should grow complex pipeline golden manifests in
Rust/testkit first. Coverage report artifacts are expected for later milestones
that touch shared behavior, parser surfaces, or DSP modules; record the summary
or LCOV path in validation notes when that gate applies.

---

## First milestone

The first meaningful milestone is not “many effects”. It is a tested pipeline.

Milestone 1 acceptance:

- Rust workspace exists.
- WAV read/write works for PCM16 mono/stereo.
- Internal planar `f32` representation exists.
- CLI can inspect WAV files.
- CLI can copy WAV input to WAV output through the internal pipeline.
- CLI can apply `gain`.
- Library supports equivalent chainable calls.
- Golden tests compare `gain` against `sox_ng`.
- Analytical tests verify `gain` math.
- Chunk invariance passes for `gain`.
- Documentation examples compile.
- Python pytest harness runs under `uv`.

---
