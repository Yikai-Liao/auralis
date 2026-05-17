---
kind: effect
effect: "phaser"
status: planned
owner: "crates/auralis-effects/src/phaser.rs"
graph_op: "phaser"
cli_example: "auralis render input.wav -o output.wav --fx phaser,notches=6,depth=80%,rate=0.5Hz,feedback=0.4"
family: "delay/modulation"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/modulation-delay.md"
---

# Phaser

## One-Line Description

`phaser` creates moving spectral notches by mixing the dry signal with a
cascaded all-pass-filtered signal whose phase response is modulated over time.

## Reference Target

- Primary reference: Faust `phaser2_mono` / `phaser2_stereo`.
- Secondary reference: DaisySP Phaser for stability and API sanity checks.
- SoX-ng role: rejected as the default oracle; keep only for a future explicit
  compatibility mode if needed.
- Why this reference: Faust documents true all-pass phaser controls, including
  notch placement, sweep bounds, depth, feedback, and stereo variants.
- Why not SoX-ng: SoX-ng `phaser` is flanger-like, so it is the wrong algorithm
  family for Auralis' default `phaser`.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `notches` | `u8` | `6` | `1..=24` | moving notch pairs |
| `depth` | `Percent` | `80%` | `0%..=100%` | LFO sweep depth |
| `rate` | `Hertz` | `0.5Hz` | `> 0Hz` | LFO frequency |
| `feedback` | `f32` | `0.4` | `-0.95..=0.95` | feedback around the all-pass cascade |
| `min_freq` | `Hertz` | `300Hz` | `> 0Hz` | lower sweep bound |
| `max_freq` | `Hertz` | `1600Hz` | `> min_freq`, `< sample_rate/2` | upper sweep bound |
| `mix` | `Percent` | `50%` | `0%..=100%` | wet signal amount |

## Signal Model

- Inputs: one audio port, any positive channel count.
- Outputs: one audio port named `audio`.
- Changes length: no.
- Changes sample rate: no.
- Changes channels: no.
- Whole-buffer state: per-channel all-pass state, reset per render.

## Mathematics

One first-order all-pass section:

```text
y[n] = -a[n] * x[n] + x[n - 1] + a[n] * y[n - 1]
```

For `K` cascaded sections:

```text
z_0[n] = x[n] + feedback * z_K[n - 1]
z_k[n] = allpass_k(z_{k-1}[n], a_k[n]) for k = 1..K
wet[n] = z_K[n]
out[n] = (1 - mix) * x[n] + mix * wet[n]
```

The LFO maps to a logarithmic sweep:

```text
lfo[n] = 0.5 + 0.5 * sin(phase[n])
sweep[n] = clamp(0.5 + depth * (lfo[n] - 0.5), 0, 1)
f_k[n] = exp(lerp(ln(min_freq), ln(max_freq), sweep[n] + offset_k))
w_k[n] = 2*pi*f_k[n]/sample_rate
a_k[n] = (tan(w_k[n]/2) - 1) / (tan(w_k[n]/2) + 1)
```

Clamp `f_k[n]` below Nyquist before coefficient conversion.

## Pseudocode

```text
validate params against sample_rate
precompute phase_step = 2*pi*rate/sample_rate
for channel in channels:
  reset allpass states
  phase = channel_phase_offset(channel)
  previous_wet = 0
  for n in 0..frames:
    x = input[channel][n]
    lfo = 0.5 + 0.5 * sin(phase)
    wet = x + feedback * previous_wet
    for section in sections:
      freq = section_frequency(section, lfo, params)
      a = allpass_coefficient(freq, sample_rate)
      wet = process_allpass(section.state, wet, a)
    output[channel][n] = (1 - mix) * x + mix * wet
    previous_wet = wet
    phase += phase_step
```

## Implementation Notes

- Runtime type: `Phaser`.
- Params type: `PhaserParams`.
- DSP helper: start with a private `FirstOrderAllpass` state type. Extract only
  if another effect needs the same exact contract.
- Edge params: none.
- Reports: none.
- Graph validation owns sample-rate-dependent frequency checks.

## Performance Notes

- Allocate output and per-channel section state once per render.
- Precompute phase step, log frequency bounds, dry gain, and wet gain.
- Avoid heap allocation and string lookup inside the sample loop.
- Keep the first implementation scalar; recursive all-pass sections are not an
  early SIMD target.
- Benchmark 30 s and 300 s stereo buffers because cost is per sample per
  section.

## Validation And Diagnostics

- `notches == 0`.
- `rate <= 0Hz`.
- `feedback` outside `-0.95..=0.95`.
- `max_freq <= min_freq`.
- `max_freq >= sample_rate / 2`.
- unsupported parameter name with spelling suggestion.

## Tests

- Silence remains finite silence.
- Impulse response remains finite for valid feedback values.
- `mix=0%` returns the dry input.
- Invalid sample-rate-dependent frequency bounds fail in graph validation.
- Reference fixtures compare spectral notch movement against Faust-generated
  summaries, not sample equality against SoX-ng.

## Benchmarks

- Case: `phaser,notches=6,depth=80%,rate=0.5Hz,feedback=0.4`.
- Input shape: 48 kHz stereo, 30 s and 300 s.
- Metric: whole-buffer render time.
- Baseline: first scalar implementation.
- Confirmation: rerun with `--skip-build` before claiming a win.

## Done When

- The implementation follows the all-pass phaser target.
- Graph validation catches invalid params before execution.
- Faust-derived fixtures cover notch movement and finite output.
- Benchmarks exist for realistic stereo buffers.
