# Use linkme for compile-time op registration

Auralis will use `linkme`, hidden behind `auralis-op` / `auralis-op-macros`, for compile-time distributed graph-op registration. All graph ops are registered by default; CLI availability is controlled globally by Cargo features and runtime configuration, not by per-op surface flags. Catalog construction must sort by canonical op name and reject duplicate canonical names or aliases.

Duplicate op names or aliases are build failures, not runtime warnings. Because distributed linker slices cannot by themselves prove uniqueness during Rust type checking, Auralis must add deterministic catalog validation under `cargo test` so ordinary development, CI, release, and gnhf gates fail before artifacts are accepted.

The catalog test must check at least canonical op-name uniqueness, alias uniqueness, aliases not colliding with canonical names, stable sorted output, complete input/output signatures, parameter/schema availability, default-parameter decoding, and plan-JSON schema generation.

**Considered Options**

- Maintain a large central registration list: rejected because it recreates the current registry boilerplate and makes every op touch central files.
- Use runtime plugin registration: rejected because initialization order and dynamic loading conflict with deterministic offline rendering.
- Use `linkme` behind Auralis macros: accepted because it gives effect-local declarations without a central `.with(...)` list while keeping the linker mechanism isolated behind project-owned APIs.
