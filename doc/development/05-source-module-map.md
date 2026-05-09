# Feature 5.6.1 Source Module Map

This file records the file-size audit and ownership plan for Milestone 5.6.
The goal is to make the later split features mechanical and reviewable: move
code by ownership boundary, keep public APIs stable, and avoid behavior changes
until the target modules are in place.

## Repeatable Audit Command

Run the line-count audit from the repository root:

```bash
rg --files -g '*.rs' | xargs wc -l | sort -nr
```

For a strict local check candidate before Feature 5.6.6 adds a permanent guard:

```bash
python3 - <<'PY'
from pathlib import Path

limit = 1000
oversized = []

for path in sorted(Path(".").rglob("*.rs")):
    lines = sum(1 for _ in path.open("rb"))
    if lines > limit:
        oversized.append((lines, path))

for lines, path in oversized:
    print(f"{lines:5} {path}")

raise SystemExit(1 if oversized else 0)
PY
```

The current project policy is "no Rust source file above 1,000 lines." The
strict check uses `>` rather than `>=` so a file with exactly 1,000 lines is
still accepted by the policy as written.

## Feature 5.6.1 Baseline Line Counts

| Lines | File | Classification |
|---:|---|---|
| 4,183 | `crates/auralis/src/lib.rs` | Oversized implementation-heavy facade |
| 2,617 | `crates/auralis-cli/tests/inspect.rs` | Oversized integration test file |
| 2,466 | `crates/auralis-simd/src/lib.rs` | Oversized implementation-heavy crate root |
| 1,391 | `crates/auralis-testkit/src/golden.rs` | Oversized golden-manifest module |
| 1,103 | `crates/auralis-effects/src/lib.rs` | Oversized implementation-heavy crate root |
| 1,079 | `crates/auralis-wav/src/lib.rs` | Oversized implementation-heavy crate root |
| 942 | `crates/auralis-effects/src/chain.rs` | Watch list: near limit |
| 753 | `crates/auralis-core/src/lib.rs` | Watch list: central public API |
| 746 | `crates/auralis-effects/src/command.rs` | Watch list: parser growth risk |
| 696 | `crates/auralis-effects/src/effects_file.rs` | Watch list: parser growth risk |
| 693 | `crates/auralis-testkit/src/backend_conformance.rs` | Watch list: reusable test helper growth |
| 633 | `crates/auralis-cli/src/main.rs` | Watch list: CLI surface growth risk |
| 582 | `crates/auralis-effects/src/registry.rs` | Below limit |
| 497 | `crates/auralis-dsp/src/lib.rs` | Below limit |
| 306 | `crates/auralis-codec/src/lib.rs` | Below limit |
| 299 | `crates/auralis-testkit/src/lib.rs` | Below limit |
| 246 | `crates/auralis-testkit/tests/golden_combine.rs` | Below limit |
| 72 | `crates/auralis-testkit/tests/golden_chains.rs` | Below limit |
| 44 | `crates/auralis-testkit/tests/golden_auto_level.rs` | Below limit |
| 39 | `crates/auralis-testkit/tests/golden_auto_rate.rs` | Below limit |
| 39 | `crates/auralis-testkit/tests/golden_auto_channels.rs` | Below limit |
| 33 | `crates/auralis-wav/examples/decode_pcm16.rs` | Below limit |

Oversized files that are not named by the original 5.6.2-5.6.5 split features
must still be addressed before the no-thousand-line-file policy can be
enforced. There is no current generated or vendored-file exemption.

## Target Module Map

### `crates/auralis`

Feature 5.6.2 completed this split. At completion, the largest resulting
`crates/auralis` Rust files were:

| Lines | File |
|---:|---|
| 840 | `crates/auralis/tests/combine.rs` |
| 693 | `crates/auralis/src/combine.rs` |
| 616 | `crates/auralis/src/audio_file.rs` |
| 409 | `crates/auralis/tests/pipeline.rs` |
| 401 | `crates/auralis/src/pipeline.rs` |
| 388 | `crates/auralis/tests/output_policies.rs` |

Target shape:

- `src/lib.rs`: crate docs, public re-exports, module declarations, and the
  `Result` alias.
- `src/errors.rs`: high-level `Error` enum and `PartialEq` implementation.
- `src/audio_file.rs`: `AudioFile`, WAV opening helpers, and constructors from
  existing buffers.
- `src/pipeline.rs`: `Pipeline`, fluent effect application, chain application,
  WAV writing, and `seconds_to_frame`.
- `src/combine.rs`: `CombineMethod`, `InputCombineError`, concatenate,
  sequence, mix, mix-power, merge, multiply, and combiner validation helpers.
- `src/channel_policy.rs`: `ChannelConversionPolicy`,
  `ChannelConversionError`, channel conversion helpers, and backend-aware
  downmixing.
- `src/rate_policy.rs`: `SampleRateConversionPolicy`,
  `SampleRateConversionError`, linear output-boundary resampling, and frame
  count conversion helpers.
- `src/level_policy.rs`: `OutputLevelPolicy`, `OutputLevelError`, guard,
  normalization, peak scanning, and multiplier application.

Test relocation:

- move chain and pipeline tests to `crates/auralis/tests/pipeline.rs`;
- move combiner tests to `crates/auralis/tests/combine.rs`;
- move output channel, rate, and level policy tests to
  `crates/auralis/tests/output_policies.rs`;
- move error-propagation and file-boundary smoke tests to focused integration
  files;
- put shared buffer and temp-path helpers in `crates/auralis/tests/support/`.

Feature 5.6.2 owns this split. It should not change public re-export names or
documented runtime behavior.

### `crates/auralis-simd`

Target shape:

- `src/lib.rs`: crate docs, module declarations, and public re-exports.
- `src/backend.rs`: `BackendKind`, `BackendDescriptor`,
  `BackendFallbackReason`, `BackendSelection`, `Backend`, `ScalarBackend`, and
  `SimdBackend`.
- `src/selection.rs`: `backend_descriptor`, `select_backend`,
  `select_named_backend`, build-feature and target fallback detection.
- `src/convert.rs`: PCM16-to-`f32` and `f32`-to-PCM16 public conversion APIs,
  validation, scalar reference paths, and SIMD selected paths.
- `src/gain.rs`: gain scalar/reference and selected SIMD kernels.
- `src/dcshift.rs`: DC shift scalar/reference and selected SIMD kernels.
- `src/fade.rs`: fade scalar/reference and selected SIMD kernels.
- `src/mix.rs`: mix errors, validation, scalar/reference and selected SIMD
  kernels.
- `src/multiply.rs`: multiply errors, validation, scalar/reference and
  selected SIMD kernels.
- `src/scalar.rs` or per-module private helpers: shared scalar sample math only
  if duplication appears during the split.

Test relocation:

- move backend selection tests to `crates/auralis-simd/tests/backend.rs`;
- move conversion conformance tests to
  `crates/auralis-simd/tests/convert.rs`;
- move gain, dcshift, fade, mix, and multiply conformance tests to focused
  integration files;
- put seeded fixture generators and assertion helpers in
  `crates/auralis-simd/tests/support/`.

Feature 5.6.3 owns this split. `rten-simd` must remain hidden behind
Auralis-owned public APIs.

### `crates/auralis-effects`

Target shape:

- `src/lib.rs`: crate docs, module declarations, and public re-exports.
- `src/error.rs`: `EffectError` and effect-local result alias.
- `src/gain.rs`: `Gain` and gain-specific processing helpers.
- `src/dcshift.rs`: `DcShift` and DC shift-specific processing helpers.
- `src/trim.rs`: `Trim` and range validation.
- `src/pad.rs`: `Pad` and frame padding.
- `src/fade.rs`: `Fade` and segment processing.
- `src/reverse.rs`: `Reverse` and frame-order reversal.
- `src/processor.rs` or `src/types.rs`: shared processing traits or small
  helper types only if the split exposes repeated glue.
- existing `src/command.rs`, `src/chain.rs`, `src/effects_file.rs`, and
  `src/registry.rs` remain separate ownership units.

Test relocation:

- move each effect family's tests beside its module under `src/` using
  `#[cfg(test)] mod tests;`, or into focused integration files when only public
  APIs are needed;
- keep command parser, chain, effects-file, and registry tests with their
  existing modules;
- move shared buffer and assertion helpers to a test-support module if they
  become duplicated.

Feature 5.6.4 owns this split. Parser, registry, and chain public behavior
should remain unchanged except for import paths.

### `crates/auralis-wav`

Target shape:

- `src/lib.rs`: crate docs, module declarations, public re-exports, and the
  result alias.
- `src/error.rs`: `WavError` and `From<WavError> for CodecError`.
- `src/format.rs`: `WavSampleEncoding`, PCM16 format validation,
  `hound::WavSpec` construction, frame-count conversion, and malformed-input
  mapping.
- `src/reader.rs`: `decode_pcm16`, path decode helpers, backend-aware decode,
  and `Pcm16WavReader`.
- `src/writer.rs`: `encode_pcm16`, path encode helpers, backend-aware encode,
  and `Pcm16WavWriter`.
- `src/sample_conversion.rs`: backend-dispatched sample conversion glue and
  error mapping between SIMD and WAV layers.

Test relocation:

- move decode and format-rejection tests to `crates/auralis-wav/tests/decode.rs`;
- move encode and round-trip tests to `crates/auralis-wav/tests/encode.rs`;
- move SoX-ng reference tests to `crates/auralis-wav/tests/golden_reference.rs`;
- put WAV byte construction, temp paths, and PCM helper functions in
  `crates/auralis-wav/tests/support/`.

Feature 5.6.5 owns this split. PCM16 decode/encode behavior and unsupported
format diagnostics must stay stable.

### Additional Oversized Files

The policy applies to all Rust source files, not only crate roots. These files
are outside the original 5.6.2-5.6.5 target list but must be handled before
Feature 5.6.6 can enforce the limit:

- `crates/auralis-cli/tests/inspect.rs`: split by CLI workflow into inspect,
  copy, output policy, combine, effect, chain, effects-file, and error tests;
  move WAV fixture helpers to `crates/auralis-cli/tests/support/`.
- `crates/auralis-testkit/src/golden.rs`: split into manifest parsing, case
  normalization, command rendering, tolerance/error types, and validation
  modules; move current unit tests with their owning modules.

`crates/auralis-effects/src/chain.rs` is below the limit but close enough that
new chain behavior should either add a submodule first or move tests out before
more implementation is added.

## Verification Scope

Feature 5.6.1 intentionally has no runtime behavior changes. Later split
features should verify with the full required command set:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
cd tools/pytest && uv run pytest
```
