---
kind: effect
effect: "bend"
status: planned
owner: "crates/auralis-effects/src/bend.rs"
graph_op: "bend"
cli_example: "auralis render input.wav -o output.wav --fx bend,points=0s:0cents;1s:200cents"
family: "time/pitch"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/time-pitch.md"
---

# Bend

## One-Line Description

`bend` applies time-varying pitch shifts along a documented envelope.

## Reference Target

- Primary reference: Signalsmith Stretch-style time/pitch backend.
- Quality reference: Rubber Band CLI where an equivalent bend can be expressed.
- SoX-ng role: baseline and compatibility reference only.
- Why not SoX-ng: Auralis should own envelope interpolation and quality policy
  instead of inheriting SoX-ng's historical behavior.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `points` | list | required | sorted, finite | time to cents envelope |
| `interpolation` | enum | `linear` | `step`, `linear`, future `smooth` | envelope shape |
| `quality` | enum | `balanced` | `fast`, `balanced`, `high` | backend quality |

## Signal Model

- Inputs: one audio port.
- Outputs: one audio port named `audio`.
- Changes length: no unless a future mode says otherwise.
- Changes sample rate: no.
- Changes channels: no.
- Whole-buffer state: envelope and time/pitch backend state.

## Mathematics

```text
cents(t) = interpolate(points, t)
ratio(t) = 2 ^ (cents(t) / 1200)
y = variable_pitch_shift(x, ratio(t))
```

## Pseudocode

```text
validate sorted points
build per-frame or per-window pitch envelope
run variable pitch backend
preserve input frame count
```

## Implementation Notes

Envelope parsing belongs to frontend lowering; backend receives typed points.
Reuse time/pitch primitives where possible.

## Performance Notes

Avoid per-sample heap work. Benchmark smooth and fast-changing envelopes.

## Validation And Diagnostics

Reject missing points, unsorted times, non-finite cents, unsupported
interpolation, and envelope outside input range if policy requires it.

## Tests

Identity envelope, constant bend equivalence with `pitch`, envelope
interpolation, fixed duration, and validation failures.

## Benchmarks

Cases: slow bend, fast bend, and constant bend. Use fresh output dirs and
second-run confirmation for performance claims.

## Done When

Envelope semantics are deterministic and bend shares the time/pitch oracle
without duplicating backend policy.
