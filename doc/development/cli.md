---
kind: development-plan
status: active
---

# CLI Development Plan

This is the migration target for the lightweight CLI design.

## Scope

- Small command surface.
- `--fx` comma-separated effect syntax.
- `--check` for validation only.
- `--plan` for validation plus machine-readable execution planning.
- Output format handling for text, JSON, and later thin serde-compatible
  adapters.
- CLI lowering into `GraphRequest`.

## Non-Goals

- No per-effect top-level command explosion.
- No duplicated plan-mode parsing path.
- No CLI string parsing inside `auralis-graph`.
- No positional effect parameters in the target design.

Detailed content will migrate from `doc/development/10-cli-interaction-model.md`
in the full migration commit.
