---
kind: development-plan
status: active
---

# Graph Execution Engine

This is the migration target for the shared Auralis graph engine design.

## Scope

- `GraphRequest` as the surface-neutral input model.
- Validation into `ValidGraph` with compiler-style diagnostics.
- Planning into `ExecutionPlan`.
- Whole-buffer execution only.
- Prompt release of temporary audio buffers after last use.

## Non-Goals

- No streaming graph execution.
- No chunked graph scheduler.
- No realtime graph API.
- No CLI string parsing inside the graph crate.

Detailed content will migrate from the current ADRs and legacy CLI planning
documents in the full migration commit.
