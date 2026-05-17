# Development Documentation Map

This directory uses short, focused documents. A document should describe one
decision area, one effect, one DSP primitive, one SIMD kernel, or one CI job.
Split before a file becomes a dumping ground.

## Root Documents

| Path | Purpose | Should not contain |
| --- | --- | --- |
| `README.md` | project positioning, current status, quick commands, reader entry points | effect-by-effect plans, architecture debates, long roadmaps |
| `DEVELOPMENT.md` | development index, current priority, gnhf rules, links to detailed tracks | algorithm formulas, per-effect status tables, benchmark logs |
| `AGENTS.md` | repository collaboration rules for agents | product roadmap, DSP design, CLI examples beyond command policy |

`AGENTS.md` target contents:

- repository command rule: use `rtk` for shell commands;
- implementation discipline: think before coding, state assumptions, keep
  changes surgical, avoid speculative abstractions;
- verification discipline: define success criteria, run the smallest relevant
  checks, report skipped checks;
- git/worktree discipline: never revert user changes, do not overwrite the
  preserved benchmark baseline, keep commits focused;
- documentation discipline: update the target short document path when design
  decisions change;
- Rust layout rule: no single `.rs` file over 1000 lines after a change;
- benchmark rule: write new benchmark runs to fresh directories and keep
  `target/benchmarks/sox_ng` as preserved baseline evidence;
- Python status: Python packaging is not planned until graph/effect/library
  contracts stabilize.

## Stable Reference Documents

| Path | Purpose |
| --- | --- |
| `doc/architecture.md` | crate boundaries, public API rules, dependency policy, graph/effect/DSP ownership |
| `doc/testing.md` | testing layers, golden policy, fuzzing, coverage, oracle rules |
| `doc/status.md` | current implemented capability snapshot |
| `doc/development-commands.md` | runnable local commands only |

`doc/status.md` is a current-state snapshot. It must answer what is implemented
now and must not carry future plans. Future work belongs under
`doc/development/`.

## Shared Status Values

Use these status values in YAML front matter and index tables:

| Status | Meaning |
| --- | --- |
| `planned` | accepted future work, not currently being implemented |
| `active` | current implementation or migration work |
| `implemented` | implemented and covered by the required checks |
| `blocked` | accepted work with a named blocker |
| `not-planned` | deliberately out of scope |
| `superseded` | replaced by another document or plan |

## Target Development Documents

| Path | Purpose |
| --- | --- |
| `doc/development/README.md` | this path map |
| `doc/development/graph-engine.md` | `auralis-graph`, `GraphRequest`, validation, planning, whole-buffer execution, memory lifetime |
| `doc/development/cli.md` | lightweight CLI shape, `--fx`, `--check`, `--plan`, output formats, config input |
| `doc/development/op-registration.md` | `auralis-op`, distributed registration, duplicate checks, Cargo feature boundaries |
| `doc/development/testing.md` | testing development plan: CI gates, fuzzing, oracle replacement, coverage debt |
| `doc/development/oracles.md` | oracle/reference-target index |
| `doc/development/oracles/<subject>.md` | shared oracle/reference notes that apply to multiple effects |
| `doc/development/formats.md` | format/codec development index |
| `doc/development/formats/<format>.md` | one short format or codec boundary document |
| `doc/development/ci.md` | CI/CD strategy index: GitHub Actions, external binary release repository, benchmark/test jobs, Bencher upload |
| `doc/development/ci/<job>.md` | one short CI or benchmark job document |
| `doc/development/python.md` | explicit `not planned` status and future prerequisites |

## Effect Documents

The effect area has one short index and one flat file per effect:

```text
doc/development/effects.md
doc/development/effects/<effect>.md
```

`doc/development/effects.md` is only an index. It tracks effect name, status,
reference target, SoX-ng role, priority, and the link to the detailed file. It
must not contain formulas, pseudocode, or long rationale.

Each effect gets one short document:

```text
doc/development/effects/<effect>.md
```

Effect files are flat. Do not create family subdirectories under
`doc/development/effects/`. Put the family in YAML front matter instead. This
keeps links predictable from op names, CLI diagnostics, graph validation, and
the op registry.

Example:

```text
doc/development/effects/phaser.md
doc/development/effects/stretch.md
doc/development/effects/noisered.md
```

Required contents:

- one-line description;
- reference target;
- why not SoX-ng when SoX-ng is not the target;
- parameters and validation;
- signal model;
- mathematical formula;
- pseudocode;
- implementation notes;
- performance notes;
- tests, oracle fixtures, and benchmarks.

Every effect document must contain a mathematical formula and pseudocode. The
depth may scale with the effect: `gain` can use one equation and a five-line
loop, while `phaser`, `noisered`, or `stretch` must describe the full algorithm
structure. Do not replace algorithm content with only a reference link.

Use:

```text
doc/development/templates/effect.md
```

Example filled document:

```text
doc/development/examples/effect-phaser.md
```

## DSP Primitive Documents

The DSP primitive area has one short index and one flat file per primitive:

```text
doc/development/dsp.md
doc/development/dsp/<primitive>.md
```

`doc/development/dsp.md` is only an index. It tracks primitive name, status,
owner, callers, SIMD status, and the link to the detailed file. It must not
contain formulas, pseudocode, or API design details.

Each reusable DSP primitive gets one short document:

```text
doc/development/dsp/<primitive>.md
```

DSP primitive files are flat. Do not create primitive-family subdirectories
under `doc/development/dsp/`. Put category or planned callers in YAML front
matter and body text.

Examples:

```text
doc/development/dsp/axpy.md
doc/development/dsp/fractional-delay.md
doc/development/dsp/stft-profile.md
```

Required contents:

- one-line description;
- reason it belongs in `auralis-dsp`;
- mathematical contract;
- API sketch;
- pseudocode;
- callers;
- numerical notes;
- performance notes;
- tests and benchmarks.

Use:

```text
doc/development/templates/dsp-primitive.md
```

Example filled document:

```text
doc/development/examples/dsp-axpy.md
```

## SIMD Kernel Documents

The SIMD area has one short index and one flat file per kernel:

```text
doc/development/simd.md
doc/development/simd/<kernel>.md
```

`doc/development/simd.md` is only an index. It tracks kernel name, priority,
status, callers, backend, and the link to the detailed file. It must not
contain formulas, pseudocode, or benchmark narratives.

Each SIMD target gets one short document:

```text
doc/development/simd/<kernel>.md
```

SIMD kernel files are flat. Do not create priority or backend subdirectories
under `doc/development/simd/`. Put priority, backend, and callers in YAML front
matter.

Examples:

```text
doc/development/simd/finite-peak-rms.md
doc/development/simd/pcm32-conversion.md
doc/development/simd/remix-axpy.md
```

Required contents:

- one-line description;
- SIMD justification;
- scalar formula;
- SIMD pseudocode;
- numerical contract;
- API sketch;
- tail and fallback behavior;
- tests and benchmarks.

Use:

```text
doc/development/templates/simd-kernel.md
```

Example filled document:

```text
doc/development/examples/simd-finite-peak-rms.md
```

## Oracle Notes

The oracle area has one short index and flat subject files:

```text
doc/development/oracles.md
doc/development/oracles/<subject>.md
```

`doc/development/oracles.md` tracks subject, selected reference target,
applicable effects, status, and detail link. It must not contain formulas,
pseudocode, or long rationale.

Use a separate oracle note only when the reference policy is shared across
multiple effects or cannot fit cleanly inside one effect document.

```text
doc/development/oracles/<subject>.md
```

Examples:

```text
doc/development/oracles/time-pitch.md
doc/development/oracles/spectral-denoise.md
```

Single-effect oracle decisions stay in that effect file.

## Format And Codec Documents

The format area has one short index and flat format files:

```text
doc/development/formats.md
doc/development/formats/<format>.md
```

Examples:

```text
doc/development/formats/wav.md
doc/development/formats/flac.md
doc/development/formats/aiff.md
doc/development/formats/au.md
doc/development/formats/raw-pcm.md
```

`doc/development/formats.md` tracks format name, status, owner crate, codec
backend, encode/decode support, and detail link. Individual format files own
container semantics, sample representation, validation rules, benchmarks, and
test coverage.

## Testing Development Document

Use one development testing document:

```text
doc/development/testing.md
```

`doc/testing.md` remains the stable testing contract. `doc/development/testing.md`
owns future work: GitHub Actions gates, fuzzing expansion, oracle replacement,
coverage debt, benchmark correctness gates, and release validation.

## CI And Benchmark Job Documents

The CI area has one short index and one flat file per job:

```text
doc/development/ci.md
doc/development/ci/<job>.md
```

`doc/development/ci.md` tracks the CI/CD strategy, required third-party binary
source, GitHub Actions job list, artifact policy, and Bencher upload policy.
Individual jobs live in flat files such as:

```text
doc/development/ci/test-workspace.md
doc/development/ci/benchmark-effects.md
doc/development/ci/upload-bencher.md
```

Use:

```text
doc/development/templates/ci-benchmark-job.md
```

## Legacy Numbered Documents

Existing numbered roadmap files remain readable while unique historical or
coverage notes are being retired, but new detailed work belongs in the target
paths above.

Rules:

- new graph, CLI, op-registration, CI, Python, effect, DSP, and SIMD planning
  must be written in the target paths above;
- every existing effect, DSP primitive plan, SIMD plan, CI plan, and Python
  plan must either live in a target short document or be explicitly marked
  superseded;
- old numbered files are transition references, not the place to add new
  detailed plans;
- when a target document fully covers an old numbered document, add front matter
  to the old document with `status: superseded` and `superseded_by: [...]`,
  then keep only a short migration note unless a later focused cleanup removes
  the file;
- do not keep duplicate living plans for the same subject.
- remove superseded numbered documents once they no longer preserve useful
  links or historical evidence.

| Existing area | Target path |
| --- | --- |
| `doc/development/03-test-infrastructure.md` | `doc/development/testing.md` |
| `doc/development/10-cli-interaction-model.md` | `doc/development/cli.md` |
| `doc/development/06-effect-coverage.md` and `06-effects/*.md` | `doc/development/effects/*.md` |
| `doc/development/07-dsp-primitives.md` | `doc/development/dsp/*.md` plus summary in `doc/architecture.md` |
| `doc/development/04-simd.md` | `doc/development/simd/*.md` plus SIMD policy in `doc/testing.md` |
| `doc/development/08-format-support.md` | `doc/development/formats.md` and `doc/development/formats/*.md` |
| CI and benchmark planning scattered in status or commands docs | `doc/development/ci.md` |
| `doc/development/09-python-package.md` | `doc/development/python.md` |

## Size Rule

Target length:

- index or policy doc: 80-200 lines;
- effect, DSP primitive, SIMD kernel, or CI job doc: 80-250 lines;
- hard cap: split before 500 lines.

If a document needs a long table, split by effect family, primitive family, or
CI job instead of expanding one file indefinitely.
