---
kind: development-plan
status: not-planned
---

# Python Bindings

Python packaging and bindings are not planned until the Rust graph, effect,
parameter, error, and buffer contracts stabilize.

## Current Decision

- Do not build or document a public Python API in the current development
  phase.
- Do not let Python needs shape the first graph executor implementation beyond
  keeping `GraphRequest` surface-neutral.
- Revisit Python only after `auralis-graph`, `auralis-op`, and the effect
  registry contract are stable enough for downstream bindings.

## Future Preconditions

- Stable Rust library graph API.
- Stable parameter and diagnostic serialization.
- Stable audio buffer ownership and lifetime model.
- Stable op registry metadata for binding generation.
- CI coverage for binding builds and wheel packaging.
