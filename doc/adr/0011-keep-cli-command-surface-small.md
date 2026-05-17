# Keep CLI command surface small

Auralis will keep a small top-level CLI command surface and will not model each effect as its own top-level command. Effects enter the system through `render --fx`, TOML graph nodes, Rust helpers, and future bindings, all lowering into `GraphRequest`.

Target top-level commands are limited to workflow-level operations such as `render`, `run`, `convert`, `graph`, `ops`, `inspect`, `cache`, and `fmt`. Validation and planning are execution-mode flags: `--check` validates and reports diagnostics only, while `--plan` validates and then emits an execution plan. `--format json|text` controls machine-readable versus human-readable output.

**Considered Options**

- Keep adding one top-level command and args struct per effect: rejected because it duplicates effect schema, parsing, plan behavior, and documentation across CLI code.
- Keep the CLI as a small workflow adapter over the graph engine: accepted because it makes CLI code thin and lets op metadata drive CLI `--fx`, graph specs, schemas, and docs.
