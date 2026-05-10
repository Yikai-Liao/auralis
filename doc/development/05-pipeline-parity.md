# 5.x Pipeline Parity Milestone

Pipeline behavior is higher priority than additional file formats because many
SoX-ng effects only make sense inside chains.

## Milestone 5.1: effect command model

### Feature 5.1.1: effect registry and name resolution

Status: implemented.

Acceptance tests:

- supported names resolve to typed effect descriptors;
- unknown names fail with suggestions;
- unsupported SoX-ng effects fail with a message that names missing coverage;
- existing typed APIs remain unchanged.

### Feature 5.1.2: command parser for implemented effects

Status: implemented.

Acceptance tests:

- parse implemented effect names and options into typed configs;
- reject unsupported options with a diagnostic that names the effect and option;
- no stringly typed effect config leaks into public library APIs;
- preserve typed API behavior for existing effects.

### Feature 5.1.3: deterministic command rendering

Status: implemented.

Acceptance tests:

- equivalent command values render identically;
- quoting and escaping are deterministic;
- golden manifest command rendering is stable across runs.

## Milestone 5.2: multi-effect chains

### Feature 5.2.1: in-memory sequential chain

Status: implemented.

Acceptance tests:

- multiple effects execute in user-specified order;
- chain output matches repeated direct library calls;
- failures report which effect and option failed;
- chunk invariance holds across the full chain;
- scalar and SIMD backends can be forced for the full chain.

### Feature 5.2.2: CLI sequential chain syntax

Status: implemented.

Acceptance tests:

- CLI order matches user-specified order;
- CLI output matches in-memory chain output;
- invalid chain syntax reports the failing effect and argument;
- existing single-effect CLI behavior remains compatible.

### Feature 5.2.3: SoX-ng golden tests for chains

Status: implemented.

Acceptance tests:

- at least one editing chain, one level chain, and one filter-style chain;
- Auralis and SoX-ng command lines are recorded;
- decoded samples and metadata are compared with documented tolerances.

Implementation notes:

- `tests/golden/chains.toml` includes simple editing, level, and filter-style
  chains plus the first complex multi-input pipeline golden case: `mix`
  combine followed by user `gain` and `reverse` effects.
- The pytest chain runner executes both positional CLI chains and equivalent
  effects-file chains before comparing the positional output with SoX-ng, so
  the complex case covers combine ordering and effects-file equivalence in the
  same L2 gate.

## Milestone 5.3: effects files and chain boundaries

### Feature 5.3.1: effects file parser

Status: implemented.

Acceptance tests:

- read effects from a text file;
- ignore blank lines and documented comments;
- reject malformed files with line and column diagnostics;
- parsed effects match equivalent CLI args.

### Feature 5.3.2: effects file CLI integration

Status: implemented.

Acceptance tests:

- CLI accepts an effects file;
- effects file output matches equivalent CLI args;
- missing files and unreadable files fail clearly;
- golden tests cover the same chain from CLI args and effects file.

### Feature 5.3.3: chain boundary syntax

Status: implemented.

Acceptance tests:

- boundary syntax is accepted in CLI args and effects files;
- empty chains are rejected or documented;
- boundary rendering is deterministic in manifests;
- unsupported `newfile` and `restart` semantics report stable diagnostics until
  their leaf features are implemented.

## Milestone 5.4: input combiners

### Feature 5.4.1: concatenate combiner

Status: implemented.

Acceptance tests:

- mono and stereo inputs;
- mismatched lengths;
- mismatched channel count behavior documented;
- SoX-ng golden comparison;
- full-chain test combining inputs before effects.

### Feature 5.4.2: sequence combiner

Status: implemented.

Acceptance tests:

- mono and stereo inputs;
- sequence boundary behavior matches documented semantics;
- SoX-ng golden comparison;
- full-chain test combining inputs before effects.

### Feature 5.4.3: mix combiner

Status: implemented.

Acceptance tests:

- equal-length and mismatched-length inputs;
- clipping and normalization behavior documented;
- scalar-vs-SIMD tests for mixing kernels;
- SoX-ng golden comparison.

### Feature 5.4.4: mix-power combiner

Status: implemented.

Acceptance tests:

- equal-length and mismatched-length inputs;
- power scaling behavior documented;
- scalar-vs-SIMD tests for mixing kernels;
- SoX-ng golden comparison.

### Feature 5.4.5: merge combiner

Status: implemented.

Acceptance tests:

- mono-to-stereo merge;
- multichannel merge;
- mismatched length behavior documented;
- SoX-ng golden comparison.

### Feature 5.4.6: multiply combiner

Status: implemented.

Acceptance tests:

- mono and stereo inputs;
- zero and one identity cases;
- scalar-vs-SIMD tests for multiply kernels;
- SoX-ng golden comparison.

## Milestone 5.5: automatic pipeline effects

Define explicit equivalents for SoX-ng automatic behavior. Automatic behavior
must remain visible in library policy types and test controls.

### Feature 5.5.1: automatic channel conversion policy

Status: implemented.

Acceptance tests:

- no hidden behavior in library APIs;
- CLI defaults are documented;
- disabling automatic channel conversion is possible in tests;
- SoX-ng comparison tests record when SoX-ng auto-inserted channel conversion.

### Feature 5.5.2: automatic sample-rate conversion policy

Status: implemented.

Acceptance tests:

- no hidden behavior in library APIs;
- CLI defaults are documented;
- disabling automatic rate conversion is possible in tests;
- SoX-ng comparison tests record when SoX-ng auto-inserted `rate`.

### Feature 5.5.3: guard and norm pipeline behavior

Status: implemented.

Acceptance tests:

- guard behavior is explicit in library APIs;
- CLI `--guard` and `--norm` behavior is documented;
- SoX-ng golden tests cover representative clipping cases.

### Feature 5.5.4: automatic dither insertion policy

Status: moved to Feature 6.8.8.

This was previously recorded as blocked by commit `18d002f`. The blocker was a
planning error: automatic dither insertion cannot be implemented or tested
before the `dither` effect exists. Do not implement this as a 5.5 leaf feature.

The replacement feature is
[`Feature 6.8.8`](06-effects/06-8-fir-analysis-generation-and-dither-effects.md#feature-688-automatic-dither-insertion-policy),
after `dither` TPDF behavior exists.

## Milestone 5.6: source modularization debt

Status: planned. This milestone must be completed before adding more effect
surface area.

The current codebase has several implementation-heavy `lib.rs` files. This
violates the project maintainability goal and makes future feature work harder
to review, test, and parallelize.

Target policy:

- no Rust source file above 1,000 lines after the pass;
- `lib.rs` files should be crate docs, module declarations, and re-exports;
- split by functional ownership, not by arbitrary chunks;
- avoid unrelated behavior changes during modularization;
- keep public APIs stable unless the feature explicitly says otherwise.

### Feature 5.6.1: file-size audit and module map

Status: implemented.

Module map:
[`05-source-module-map.md`](05-source-module-map.md).

Produce a checked-in module map for the oversized files and document ownership
boundaries before moving code.

Acceptance tests:

- record current line counts for every Rust source file;
- identify all files at or above 1,000 lines and files likely to cross the limit;
- define target modules for `auralis`, `auralis-simd`, `auralis-effects`, and
  `auralis-wav`;
- define where large test modules should move;
- add or document a repeatable file-size check command for future use;
- no runtime behavior changes.

### Feature 5.6.2: split `crates/auralis/src/lib.rs`

Status: implemented.

Split the high-level facade by functional ownership.

Suggested modules:

- `audio_file`
- `pipeline`
- `combine`
- `output_policy`
- `level_policy`
- `rate_policy`
- `channel_policy`
- `errors`
- focused test modules or integration tests

Acceptance tests:

- no public facade behavior changes;
- existing library and CLI tests pass;
- every resulting Rust source file is below 1,000 lines;
- `lib.rs` becomes crate docs, module declarations, and re-exports;
- moved tests remain focused and discoverable.

Implementation notes:

- `crates/auralis/src/lib.rs` is now a crate-root facade with module
  declarations, public re-exports, and the `Result` alias.
- Runtime behavior moved into `audio_file`, `pipeline`, `combine`,
  `channel_policy`, `rate_policy`, `level_policy`, and `errors` modules.
- The previous monolithic unit-test module moved into focused integration
  tests under `crates/auralis/tests/` with shared fixtures in
  `crates/auralis/tests/support/`.

### Feature 5.6.3: split `crates/auralis-simd/src/lib.rs`

Status: implemented.

Split backend selection, sample conversion, arithmetic kernels, and tests.

Suggested modules:

- `backend`
- `selection`
- `convert`
- `gain`
- `dcshift`
- `fade`
- `mix`
- `multiply`
- `testing` or focused `tests/` files

Acceptance tests:

- scalar and SIMD behavior is unchanged;
- forced backend tests still pass;
- conversion and kernel conformance tests remain easy to find;
- no resulting Rust source file exceeds 1,000 lines;
- `rten-simd` remains hidden behind Auralis-owned APIs.

Implementation notes:

- `crates/auralis-simd/src/lib.rs` is now a crate-root facade with module
  declarations and public re-exports.
- Runtime behavior moved into `backend`, `selection`, `convert`, `gain`,
  `dcshift`, `fade`, `mix`, and `multiply` modules without changing exported
  type or function names.
- The previous monolithic unit-test module moved into focused crate-internal
  test modules, with seeded fixtures and sample assertions in
  `test_support`.
- After the split, the largest `auralis-simd` Rust source file is
  `src/convert.rs` at 353 lines.

### Feature 5.6.4: split `crates/auralis-effects/src/lib.rs`

Status: implemented.

Split effect implementations and tests by effect family.

Suggested modules:

- `gain`
- `dcshift`
- `trim`
- `pad`
- `reverse`
- `fade`
- shared `processor` or `types` module if needed
- focused effect test modules

Acceptance tests:

- public effect types and constructors remain compatible;
- parser, registry, and chain modules continue to compile unchanged unless
  imports require mechanical updates;
- all existing effect and chain tests pass;
- no resulting Rust source file exceeds 1,000 lines.

Implementation notes:

- `crates/auralis-effects/src/lib.rs` is now a crate-root facade with module
  declarations and public re-exports.
- Runtime behavior moved into `error`, `gain`, `dcshift`, `trim`, `pad`,
  `fade`, and `reverse` modules without changing exported type or constructor
  names.
- The previous monolithic unit-test module moved into focused per-effect test
  modules, with shared buffer and assertion helpers in `test_support`.
- After the split, the largest `auralis-effects` Rust source file remains
  `src/chain.rs` at 942 lines; the largest newly split effect module is
  `src/fade.rs` at 233 lines.

### Feature 5.6.5: split `crates/auralis-wav/src/lib.rs`

Status: implemented.

Split WAV reader, writer, format validation, backend hooks, and tests.

Suggested modules:

- `reader`
- `writer`
- `format`
- `sample_conversion`
- `error`
- focused decode/encode/golden test modules

Acceptance tests:

- PCM16 decode/encode behavior is unchanged;
- unsupported-format diagnostics remain stable;
- scalar/SIMD WAV conformance tests still pass;
- SoX-ng WAV reference tests still pass;
- no resulting Rust source file exceeds 1,000 lines.

Implementation notes:

- `crates/auralis-wav/src/lib.rs` is now a crate-root facade with module
  declarations, public re-exports, and the result alias.
- Runtime behavior moved into `error`, `format`, `reader`, `writer`, and
  `sample_conversion` modules without changing exported type or function
  names.
- The previous monolithic unit-test module moved into focused integration tests
  under `crates/auralis-wav/tests/`, with shared WAV byte, temp-path, and
  assertion helpers in `tests/support/`.
- After the split, the largest `auralis-wav` Rust source file is
  `src/writer.rs` at 224 lines.

### Feature 5.6.6: enforce the no-thousand-line-file policy

Status: implemented.

Add a repeatable guard so future work does not recreate the same maintenance
problem.

Acceptance tests:

- a local command or script fails when a Rust source file exceeds 1,000 lines;
- generated or vendored files are either absent or explicitly exempted with a
  reason;
- the check is documented in the root development guide;
- the check can be added to CI later without changing semantics.

Implementation notes:

- `tools/check_rust_source_lines.py` scans checked-in Rust source files and
  fails when any file exceeds the 1,000-line policy.
- The remaining oversized CLI integration test file was split into focused
  workflow files with shared WAV helpers in `crates/auralis-cli/tests/support/`.
- The oversized testkit golden module now keeps runtime code in
  `src/golden.rs` and validates public behavior from
  `tests/golden_manifest.rs`.
- There are no generated or vendored Rust source exemptions.

## Milestone 5.7: layered test conformance

Status: planned. This milestone converts the README testing philosophy into
enforced, reusable test infrastructure.

Audit summary:

| Layer | Current coverage | Required follow-up |
|---|---|---|
| L0 deterministic corpus | partial | build one reusable corpus library covering every README signal family |
| L1 WAV I/O correctness | strong for PCM16 | fold into the shared corpus/metadata matrix |
| L2 SoX-ng golden regression | medium-strong | add missing standalone effect coverage and required metadata/artifacts |
| L3 analytical DSP tests | strong | keep as required acceptance for mathematical effects |
| L4 property/metamorphic tests | partial | add systematic property-test framework and core properties |
| L5 chunk invariance | basic | implement the required chunk-size matrix and seeded random chunks |
| L6 scalar-vs-SIMD differential | strong | keep as a required gate for data-parallel kernels |
| L7 fuzzing/sanitizers/coverage | baseline | keep fuzz targets current and expand coverage as new parser boundaries land |

### Feature 5.7.1: L0 deterministic corpus library

Status: implemented.

Create a unified Rust/Python corpus layer instead of constructing ad hoc samples
inside individual tests.

Required corpus families:

- silence;
- impulse;
- step;
- sine;
- sweep;
- seeded noise;
- full-scale signal;
- near-zero signal;
- odd-length buffers;
- mono and stereo buffers;
- short files shorter than filter windows or delay lines.

Acceptance tests:

- corpus generation is deterministic across runs;
- Rust and Python helpers agree on sample values for shared cases;
- existing golden tests can request corpus cases by stable ID;
- no large hand-picked audio files are required.

Implementation notes:

- `auralis-testkit::corpus` and `auralis_testkit.corpus` now expose matching
  stable corpus IDs for the README L0 families: silence, impulse, step, sine,
  sweep, seeded noise, full-scale, near-zero, odd-length, mono/stereo, and
  short-buffer cases.
- Existing golden manifests record `corpus_id` or `corpus_ids`, and Python
  golden runners resolve those IDs through the shared corpus layer instead of
  local per-test fixture switch statements.
- The corpus remains programmatic; no checked-in audio fixtures were added.

### Feature 5.7.2: L2 golden metadata and failure artifacts

Status: implemented.

Complete the README requirement that golden tests record reproducibility
metadata and useful numerical failure data.

Acceptance tests:

- every golden case records the Auralis command, SoX-ng command, corpus ID,
  metric thresholds, and output metadata;
- test reports include SoX-ng version and Auralis version or commit;
- JSON failure artifacts include case ID, backend, sample rate, channel count,
  frame count, failing metric, expected value, actual value, and first offending
  index where applicable;
- Python and Rust golden runners use the same report schema.

Implementation notes:

- `auralis-testkit::golden_report` defines the shared
  `auralis.golden.failure.v1` JSON schema for L2 failure artifacts, including
  case ID, backend, Auralis version, SoX-ng version, manifest inputs, corpus
  IDs, commands, thresholds, decoded output metadata, measured metrics, and
  structured threshold failures.
- `auralis_testkit.golden_report` mirrors the schema for Python golden runners
  and centralizes decoded metadata collection, standard max-abs/RMS/SNR/peak
  metrics, first offending sample index detection for max-abs failures, and
  deterministic JSON writing.
- Existing Python L2 runners for chain, combiner, automatic channel/rate, and
  output-level golden manifests now emit the shared report schema when a case
  fails while preserving feature-specific context such as effects-file command,
  combiner method, and automatic output policy flags.

### Feature 5.7.3: L2 standalone golden coverage for implemented effects

Status: implemented.

Ensure every implemented effect has direct SoX-ng golden coverage, not only
chain coverage.

Required initial effects:

- `gain`;
- `dcshift`;
- `trim`;
- `pad`;
- `reverse`;
- `fade`.

Acceptance tests:

- `fade` has standalone SoX-ng golden cases;
- all implemented effects have mono and stereo corpus coverage where meaningful;
- each case records whether SoX-ng automatic behavior such as rate, channels,
  guard, norm, or dither was disabled, absent, or explicitly tested;
- chain golden tests remain as integration coverage, not a substitute for
  standalone effect coverage.

Implementation notes:

- `tests/golden/effects.toml` now records standalone mono and stereo L2 golden
  cases for `gain`, `dcshift`, `trim`, `pad`, `reverse`, and `fade`; later
  Feature 6.1.4 extended the fade cases to cover all supported SoX-ng curve
  families.
- `crates/auralis-testkit/tests/golden_effects.rs` validates that the manifest
  covers every implemented effect directly, preserves deterministic command
  rendering, and keeps automatic rate/channel conversion absent for each case.
- `tools/pytest/tests/test_effect_golden_manifest.py` executes the standalone
  cases against SoX-ng and writes the shared `auralis.golden.failure.v1` report
  on drift; the runner invokes SoX-ng with `-R -D`, so dither is explicitly
  disabled while guard and norm remain absent from these effect-isolation cases.

### Feature 5.7.4: L4 property and metamorphic test framework

Status: implemented.

Move from hand-written examples toward systematic property tests.

Acceptance tests:

- introduce `proptest` for Rust-side property tests unless a better local
  reason is documented;
- add optional Python-side property tests only through `uv`, with `hypothesis`
  if needed;
- cover identity parameters for implemented effects;
- cover `reverse` twice equals original;
- cover full-range `trim` identity;
- cover `pad 0` identity;
- cover `gain +x` followed by `gain -x` approximately returns the original
  signal within documented tolerance for non-clipping input;
- finite-input behavior is checked for effects that should preserve finiteness.

Implementation notes:

- `auralis-effects` now uses Rust-side `proptest` integration tests for the L4
  property and metamorphic layer.
- The property suite covers identity parameters for the implemented effect set:
  `gain 0`, `dcshift 0`, zero-length `fade`, full-range `trim`, and `pad 0`.
- The same suite checks `reverse` twice equals the original signal, gain and
  inverse-gain round trips for bounded non-clipping input, and finite-output
  behavior for representative gain, dcshift, fade, trim, pad, and reverse
  transforms.
- Python-side property tests remain intentionally N/A for this feature because
  the Rust typed effect APIs are the behavior under test; future Python bindings
  should add `hypothesis` only through the uv-managed test harness.

### Feature 5.7.5: L5 chunk invariance matrix

Status: implemented.

Replace scattered chunk tests with a shared matrix.

Required chunk sizes:

```text
1, 2, 7, 15, 16, 17, 31, 32, 33, 64, 255, 1024, random seeded chunks
```

Acceptance tests:

- shared helpers run whole-buffer versus chunked processing for every
  streaming-capable effect;
- seeded random chunk schedules are deterministic and report the seed on
  failure;
- `Gain`, `DcShift`, `Fade`, chain execution, and future stateful effects use
  the same matrix;
- empty chunks and final flush behavior are covered where relevant.

Implementation notes:

- `auralis-testkit::chunk_invariance` now exposes the shared L5 matrix with
  fixed chunk sizes `1, 2, 7, 15, 16, 17, 31, 32, 33, 64, 255, 1024` plus a
  deterministic seeded random schedule.
- The shared runner injects empty chunks before, between, and after real chunks
  so current tests also exercise empty-input and final-flush paths.
- `crates/auralis-effects/tests/chunk_invariance.rs` applies the same matrix
  to `Gain`, `DcShift`, frame-position-aware `Fade`, and streaming-safe
  `EffectChain` execution.

### Feature 5.7.6: L7 fuzzing, sanitizers, and coverage baseline

Status: implemented.

Add the missing README L7 infrastructure.

Initial fuzz targets:

- WAV parser behavior;
- effect command parsing;
- effects-file parsing;
- pipeline manifest parsing when implemented;
- unsupported-format rejection paths.

Acceptance tests:

- fuzz targets compile and run for a short smoke duration;
- sanitizer commands are documented for Linux development;
- coverage command is documented and scoped to touched code/DSP modules rather
  than a misleading repository-wide percentage;
- failures produce minimized or reproducible inputs where the tool supports it.

Implementation notes:

- `crates/auralis-fuzz-targets` contains reusable stable-Rust drivers for WAV
  parser behavior, unsupported WAV format rejection, effect command parsing,
  effects-file parsing, and TOML golden-manifest parsing.
- `fuzz/` contains cargo-fuzz-compatible wrappers and seed corpus entries for
  those drivers. A future pipeline-manifest fuzz target remains not applicable
  until pipeline manifests are implemented.
- Stable smoke validation uses
  `cargo test -p auralis-fuzz-targets --all-features`; target compilation uses
  `cargo check --manifest-path fuzz/Cargo.toml --bins`; short target execution
  uses `cargo run --manifest-path fuzz/Cargo.toml --bin <target> -- fuzz/corpus/<target> -runs=1`.
- Linux sanitizer and touched-module coverage commands are documented in the
  README L7 section.

### Feature 5.7.7: layered coverage report and feature gate

Status: implemented.

Make L0-L7 coverage visible for every future feature.

Acceptance tests:

- add a machine-readable or Markdown coverage matrix for implemented effects
  and pipeline primitives;
- each row records L0-L7 status, N/A reason, and linked tests;
- the acceptance checklist in `DEVELOPMENT.md` is updated if the report reveals
  missing gates;
- future effect features must update the matrix in the same commit.

Implementation notes:

- `doc/development/05-layered-coverage.toml` is the machine-readable L0-L7
  coverage matrix for implemented effects and pipeline primitives.
- `tools/check_layered_coverage.py` validates the matrix shape, requires linked
  tests for covered layers, requires narrow reasons for N/A layers, and checks
  that effect and combiner rows match the currently implemented registry and
  `CombineMethod` surface.
- `tools/pytest/tests/test_layered_coverage.py` runs the validator as part of
  the uv-managed Python suite, making matrix updates a gate for future effect
  and pipeline work.
- `DEVELOPMENT.md` now includes the matrix-update requirement in the feature
  acceptance checklist.
