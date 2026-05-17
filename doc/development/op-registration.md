---
kind: development-plan
status: active
---

# Op Registration

This is the migration target for the `auralis-op` and distributed registration
design.

## Scope

- Thin `auralis-op` contract crate.
- Compile-time distributed registration with deterministic catalog sorting.
- Duplicate op and alias checks integrated into `cargo test`.
- Graph registration always available.
- CLI availability controlled by Cargo features and binary/runtime config.

## Non-Goals

- No CLI dependency from effect implementations.
- No graph crate dependency from concrete effect implementations.
- No centralized giant registration table for every effect.

Detailed content will migrate from ADRs and registration discussion in the full
migration commit.
