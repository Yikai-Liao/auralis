---
kind: effect
effect: "vad"
status: planned
owner: "crates/auralis-effects/src/vad.rs"
graph_op: "vad"
cli_example: "auralis render input.wav -o speech.wav --fx vad,mode=trim-both,aggressiveness=2,pre_roll=100ms,post_roll=200ms"
family: "speech/edit"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/voice-activity.md"
---

# VAD

## One-Line Description

`vad` detects speech-active regions and applies Auralis trim/padding semantics
around those regions.

## Reference Target

- Primary reference: WebRTC VAD for frame-level speech activity decisions.
- Secondary reference: analytical trim/padding tests for Auralis edit behavior.
- SoX-ng role: legacy behavior reference only.
- Why this reference: WebRTC VAD is a mature engineering baseline with fixed
  frame sizes and aggressiveness levels.
- Why not SoX-ng: SoX-ng VAD uses a simple cepstral-power heuristic, can be
  fooled by music, and has awkward one-sided trim behavior.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `mode` | enum | `trim-front` | `trim-front`, `trim-back`, `trim-both`, `mark` | edit behavior |
| `aggressiveness` | `u8` | `2` | `0..=3` | WebRTC VAD strictness |
| `frame` | `Milliseconds` | `20ms` | `10ms`, `20ms`, or `30ms` | VAD frame length |
| `pre_roll` | `Milliseconds` | `100ms` | `>= 0ms` | audio kept before detected speech |
| `post_roll` | `Milliseconds` | `200ms` | `>= 0ms` | audio kept after detected speech |
| `min_speech` | `Milliseconds` | `100ms` | `>= frame` | minimum speech island |
| `min_silence` | `Milliseconds` | `200ms` | `>= frame` | minimum silence gap |

## Signal Model

- Inputs: one audio port.
- Outputs: one audio port named `audio`, plus optional report metadata later.
- Changes length: yes for trim modes; no for `mark`.
- Changes sample rate: no.
- Changes channels: no, but detection uses the documented mono fold-down.
- Whole-buffer state: frame decisions, hangover smoothing, and edit-region
  selection.

## Mathematics

Fold the input to mono decision frames:

```text
m[n] = channel_average(x_c[n])
frame_i = m[i * frame_len .. (i + 1) * frame_len]
d_i = WebRtcVad(frame_i, sample_rate, aggressiveness)
```

Smooth decisions into speech regions:

```text
speech_i = hangover_filter(d_i, min_speech, min_silence)
region = expand(first_last_true(speech_i), pre_roll, post_roll)
out = trim(input, region, mode)
```

If the sample rate is unsupported by the VAD backend, validation must either
reject the input or insert an explicit documented resampling step before VAD.

## Pseudocode

```text
validate mode, aggressiveness, frame, and sample_rate
mono = fold_down_for_detection(input)
decisions = []
for frame in exact_or_padded_frames(mono, frame_len):
  decisions.push(webrtc_vad(frame, aggressiveness))
regions = smooth_and_merge(decisions, min_speech, min_silence)
expanded = apply_pre_post_roll(regions, pre_roll, post_roll, input_frames)
output = apply_mode(input, expanded, mode)
```

## Implementation Notes

- Runtime type: `Vad`.
- Params type: `VadParams`.
- DSP helper: decision smoothing may belong in `auralis-dsp` only if reused by
  silence or speech-denoise tooling.
- Edge params: none.
- Reports: optional speech-region report after trim behavior is stable.
- WebRTC VAD backend integration must be hidden behind an Auralis-owned trait
  or adapter.

## Performance Notes

- Work frame-by-frame over borrowed slices.
- Avoid allocating one vector per frame.
- Detection is not a SIMD first-pass target.
- Benchmark long speech files and music false-positive fixtures.

## Validation And Diagnostics

- invalid `mode`.
- `aggressiveness > 3`.
- unsupported `frame` duration.
- unsupported sample rate for the selected backend.
- `min_speech < frame` or `min_silence < frame`.
- no detected speech under trim modes, with documented empty-output policy.

## Tests

- Synthetic voiced/silence fixture trims to expected boundaries.
- `pre_roll` and `post_roll` clamp at file boundaries.
- `trim-front`, `trim-back`, and `trim-both` differ as documented.
- Music/noise false-positive fixtures are tracked as quality benchmarks, not
  strict correctness unless a reference label set exists.
- WebRTC VAD frame constraints fail during validation.

## Benchmarks

- Case: `vad,mode=trim-both,aggressiveness=2,frame=20ms`.
- Input shape: 16 kHz and 48 kHz mono/stereo, 10 min speech fixture.
- Metric: detection and trim planning time.
- Baseline: first WebRTC-backed implementation.
- Confirmation: rerun with `--skip-build` before claiming a win.

## Done When

- Frame-level VAD and Auralis edit semantics are separate in code and docs.
- Validation catches unsupported sample rates and frame sizes.
- Tests cover trim modes, roll padding, and empty-detection behavior.
- SoX-ng is not used as the default VAD quality oracle.
