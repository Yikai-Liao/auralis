---
kind: development-plan
status: superseded
superseded_by:
  - doc/development/cli.md
  - doc/development/graph-engine.md
  - doc/development/op-registration.md
---

# Superseded: CLI Interaction Model

This numbered roadmap has been superseded by the short target documents:

- [`cli.md`](cli.md) for CLI shape, execution-mode flags, and frontend lowering;
- [`graph-engine.md`](graph-engine.md) for `GraphRequest`, `ValidGraph`,
  `ExecutionPlan`, and whole-buffer graph execution;
- [`op-registration.md`](op-registration.md) for `auralis-op`, named
  parameters, distributed registration, and `ops` discovery.

Do not add new detailed plans here. Migrate any remaining unique information
into the target short documents, then delete this file when no transition
references depend on it.

Important corrections from the superseding ADRs:

- use `--check` and `--plan` on executable commands instead of mirrored
  `auralis check <command>` or `auralis plan <command>` families;
- use comma-separated named `--fx` syntax such as `gain,by=-3dB`;
- lower CLI input into `GraphRequest` before graph validation;
- keep graph execution whole-buffer and offline only;
- do not introduce streaming, chunked, or realtime graph execution goals.
