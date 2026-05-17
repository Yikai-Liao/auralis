---
kind: development-plan
status: active
---

# Testing Development Plan

`doc/testing.md` is the stable testing contract. This file tracks development
work needed to improve that contract: CI gates, fuzzing, oracle replacement,
coverage debt, benchmark correctness gates, and release validation.

## Migration Scope

- Move future work out of `doc/development/03-test-infrastructure.md`.
- Keep current-state claims out of this file unless they explain a planned
  migration.
- Keep runnable commands in `doc/development-commands.md`.

## Initial Work Items

| Item | Status | Target |
| --- | --- | --- |
| GitHub Actions test gate | planned | `doc/development/ci/test-workspace.md` |
| Oracle replacement coverage | planned | `doc/development/oracles.md` and effect files |
| Fuzz diagnostics for graph validation | planned | `doc/development/graph-engine.md` |
| Benchmark correctness gate | planned | `doc/development/ci/benchmark-effects.md` |
