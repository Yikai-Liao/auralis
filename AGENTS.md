# Auralis Agent Instructions

## Think Before Coding

- State assumptions before implementing when the task has multiple plausible
  interpretations.
- If code can answer a question, inspect the code instead of guessing.
- If a simpler approach satisfies the request, say so.
- Ask only when the answer cannot be discovered locally and a wrong assumption
  would be costly.

## Keep Changes Surgical

- Touch only files needed for the request.
- Match existing style.
- Do not refactor adjacent code unless the task requires it.
- Remove only unused code created by your change.
- Never revert user changes unless explicitly asked.

## Commands

- Use `rtk` for shell commands in this repository.
- Keep `doc/development-commands.md` command-only.
- For benchmark tooling under `tools/pytest`, use `uv` from that directory.

## Verification

- Define success criteria for non-trivial work.
- Run the smallest relevant checks before finishing.
- Report checks that were skipped or unavailable.
- For CLI-heavy changes, `rtk cargo test -p auralis-cli` is the fast health
  check.

## Git And Worktree

- Assume a dirty worktree may contain user changes.
- Do not use destructive git commands such as `git reset --hard` or
  `git checkout --` unless explicitly requested.
- Keep commits focused.
- Commit messages must include:

```text
Co-authored-by: Codex <noreply@openai.com>
```

## Documentation

- Use `doc/development/README.md` as the path map for development documents.
- Root `README.md` is an entry point, not a long roadmap.
- Root `DEVELOPMENT.md` is an index and priority guide, not an algorithm dump.
- `doc/status.md` is a current-state snapshot, not a future plan.
- Graph, CLI, op-registration, testing, Python, effect, DSP, SIMD, oracle,
  format, and CI plans must use their target short document paths under
  `doc/development/`.
- Old numbered development roadmap Markdown files have been retired. Do not
  recreate them for new work.
- Do not keep duplicate living plans for the same subject.
- Remove migration-only scaffolding such as commit-split notes,
  "intentionally incomplete" markers, migration-target stubs, and superseded
  numbered docs that no longer carry unique information.

## Architecture Boundaries

- The graph execution model is whole-buffer offline rendering only.
- Do not introduce streaming, chunked, or realtime graph execution goals.
- CLI must lower user input into structured graph requests; graph code must not
  parse CLI strings.
- Python packaging is not planned until graph, effect, error, and buffer
  contracts stabilize.

## Rust Layout

- A single `.rs` file must not exceed 1000 lines after a change.
- If a Rust source file would exceed 1000 lines, split it by functional
  ownership before finishing the task.

## Benchmarks

- Never overwrite `target/benchmarks/sox_ng`; it is preserved baseline
  evidence.
- Write new benchmark runs to fresh output directories.
- Do not claim a benchmark win from one run; confirm with a second
  `--skip-build` run when applicable.
