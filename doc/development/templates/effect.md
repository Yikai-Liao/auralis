---
kind: effect
effect: "<effect-name>"
status: planned
owner: "crates/auralis-effects/src/<effect>.rs"
graph_op: "<op-name>"
cli_example: "auralis render input.wav -o output.wav --fx <op>,param=value"
family: "<effect-family>"
---

# Effect Template

Use this for one effect. Put document metadata in the YAML front matter above.
The body must explain the DSP target, not the project workflow. Keep the filled
file under 250 lines.

## One-Line Description

Write one sentence that says what the effect does to audio.

Example:

`phaser` creates moving notches by mixing the input with an all-pass-filtered
version whose phase shift is modulated over time.

## Reference Target

- Primary reference:
- Secondary reference:
- SoX-ng role:
- Why this reference:
- Why not SoX-ng, if applicable:

Example:

- Primary reference: Faust `phaser2_mono` / `phaser2_stereo`
- Secondary reference: DaisySP Phaser for sanity checks
- SoX-ng role: not an oracle
- Why this reference: Faust models true phaser notches with documented all-pass
  structure and exposed controls.
- Why not SoX-ng: SoX-ng `phaser` is documented as flanger-like behavior, so it
  is not the Auralis target for a true phaser.

## Parameters

List only named params.

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `name` | `Type` | value | rule | audible meaning |

Example:

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `notches` | `u8` | `6` | `>= 1` | number of moving notch pairs |
| `depth` | `Percent` | `80%` | `0%..=100%` | LFO modulation depth |
| `rate` | `Hertz` | `0.5Hz` | `> 0Hz` | LFO frequency |
| `feedback` | `f32` | `0.4` | `-0.95..=0.95` | all-pass feedback |

## Signal Model

Define input and output shape.

- Inputs:
- Outputs:
- Changes length:
- Changes sample rate:
- Changes channels:
- Whole-buffer state:

Example:

- Inputs: one audio port, any positive channel count
- Outputs: one audio port named `audio`
- Changes length: no
- Changes sample rate: no
- Changes channels: no
- Whole-buffer state: per-channel all-pass delay/filter state, reset per render

## Mathematics

Write the actual algorithm in equations. Use simple ASCII if needed.

Example:

For a first-order all-pass section:

```text
y[n] = -a[n] * x[n] + x[n - 1] + a[n] * y[n - 1]
```

For `K` cascaded sections:

```text
z_0[n] = x[n]
z_k[n] = allpass_k(z_{k-1}[n], a_k[n]) for k = 1..K
out[n] = dry * x[n] + wet * z_K[n]
```

The modulation coefficient is derived from an LFO:

```text
lfo[n] = sin(2*pi*rate*n/sample_rate)
a_k[n] = map_lfo_to_allpass_coefficient(lfo[n], depth, notch_k)
```

## Pseudocode

Write implementation-level pseudocode.

Example:

```text
for channel in channels:
  reset section state
  for n in 0..frames:
    lfo = sin(phase)
    sample = input[channel][n]
    wet = sample
    for section in sections:
      a = coefficient(section, lfo, params)
      wet = allpass(section.state, wet, a)
    output[channel][n] = dry_mix * sample + wet_mix * wet
    phase += phase_step
```

## Implementation Notes

State where code lives and what must stay out of it.

- Runtime type:
- Params type:
- DSP helper or primitive:
- Edge params:
- Reports or non-audio outputs:

Example:

- Runtime type: `Phaser`
- Params type: `PhaserParams`
- DSP helper: keep all-pass section local first; extract only if flanger/chorus
  reuse the exact same primitive contract.
- Edge params: none
- Reports: none

## Performance Notes

Write specific optimization constraints, not generic wishes.

Example:

- Precompute `phase_step = 2*pi*rate/sample_rate`.
- Avoid heap allocation inside the sample loop.
- Keep per-channel state in contiguous arrays.
- Do not use SIMD for recursive all-pass sections in the first version.
- Benchmark long stereo buffers because modulation state cost is per sample.

## Validation And Diagnostics

List errors `--check` / `--plan` must catch before execution.

Example:

- `notches == 0`
- `rate <= 0Hz`
- `feedback` outside allowed range
- unsupported parameter name with spelling suggestion

## Tests

- Formula-level cases:
- Edge cases:
- Graph validation cases:
- Oracle/reference cases:
- Fuzz cases:

Example:

- Silence remains finite silence.
- Impulse response stays finite for valid feedback.
- Invalid feedback fails in validation.
- Compare broad spectral notch movement against Faust-generated reference
  fixtures, not SoX-ng.

## Benchmarks

- Case:
- Input shape:
- Metric:
- Baseline:
- Confirmation:

Example:

- Case: `phaser,notches=6,depth=80%,rate=0.5Hz,feedback=0.4`
- Input shape: 48 kHz stereo, 30 s and 300 s
- Metric: whole-buffer render time
- Baseline: current scalar implementation
- Confirmation: rerun with `--skip-build` before claiming a win

## Done When

- The algorithm target is explicit.
- Formula and pseudocode match implementation.
- Params are named and validated before execution.
- Tests cover math, validation, and reference behavior.
- Performance notes are reflected in benchmarks or marked not applicable.
