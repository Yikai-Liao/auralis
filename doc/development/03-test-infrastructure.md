# 3.x Test Infrastructure Milestone

This milestone established shared metric and golden-test infrastructure. The
README L0-L7 conformance audit is tracked separately in Feature 5.7.x because
the current infrastructure does not yet fully implement every README layer.

## Feature 3.1: Rust metrics module

Status: implemented.

Implement:

- `max_abs_error`
- `rms_error`
- `snr_db`
- `peak`
- `dc_offset`

Acceptance tests:

- exact known vectors;
- zero-reference behavior documented;
- NaN behavior documented.

## Feature 3.2: Python testkit under uv

Status: implemented.

Create `tools/pytest` with:

- `pyproject.toml`
- `auralis_testkit/corpus.py`
- `auralis_testkit/metrics.py`
- `auralis_testkit/sox_ng.py`
- smoke tests

Acceptance tests:

```bash
cd tools/pytest
uv sync
uv run pytest
```

## Feature 3.3: golden manifest format

Status: implemented.

Define a manifest format for golden tests:

```toml
[gain_minus_3_mono]
input = "sine_48k_mono.wav"
auralis = ["gain", "-3"]
sox_ng = ["gain", "-3"]
max_abs = 1e-4
rms = 1e-6
snr_db = 90.0
```

Acceptance tests:

- manifest parse;
- invalid manifest rejected;
- command rendering is deterministic.

## SoX-ng coverage strategy

From this point forward, prioritize SoX-ng effect and pipeline behavior before
adding more file formats. WAV PCM16 remains the canonical interchange container
for golden tests until the effect surface is broad and stable.

The current SoX-ng effect surface to track is:

```text
allpass band bandpass bandreject bass bend biquad centercut channels chorus
compand contrast dcshift deemph delay dither dolbyb dop downsample earwax echo
echos equalizer fade fir firfit flanger gain highpass hilbert ladspa loudness
lowpass mcompand noiseprof noisered norm oops overdrive pad phaser pitch rate
remix repeat reverb reverse riaa saturation sdm silence sinc softvol speed
splice stat stats stretch swap synth tempo treble tremolo trim upsample vad vol
```

Every new effect feature must add or update a SoX-ng coverage entry with:

- SoX-ng command syntax and supported options;
- Auralis typed API and CLI mapping;
- implemented, partial, blocked, or unsupported status;
- scalar backend status;
- SIMD backend status or an explicit N/A reason;
- golden cases and tolerances;
- known semantic differences from SoX-ng.

Do not claim SoX-ng parity for an effect until its options, edge cases,
pipeline positioning, CLI behavior, and golden tests are covered.

## Effect test contract

Every effect feature after Feature 3.3 must include the applicable README
layers:

- unit and validation tests for typed configs and parser behavior;
- analytical tests when the behavior has a mathematical model;
- property or metamorphic tests for identity, reversibility, length, and finite
  behavior where applicable;
- chunk invariance using the shared L5 matrix from Feature 5.7.5;
- scalar-vs-SIMD tests for data-parallel kernels;
- SoX-ng golden tests with explicit command, corpus ID, metrics, and failure
  artifact metadata;
- CLI and typed API equivalence tests.

If a layer is not applicable, the feature must say why in its coverage entry.
