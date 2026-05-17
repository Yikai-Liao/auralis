# Use check and plan as execution-mode flags

Auralis will use `--check` and `--plan` as execution-mode flags on executable commands instead of maintaining separate command mirrors. `--check` runs graph validation and emits diagnostics without producing an execution schedule. `--plan` runs validation and, if there are no errors, emits an `ExecutionPlan` without executing DSP or writing outputs.

`--check` and `--plan` are mutually exclusive. Passing both is a command error because execution mode is exactly one of execute, check, or plan.

Examples:

```bash
auralis run Auralis.toml --check --format json
auralis run Auralis.toml --plan --format json
auralis render input.wav -o output.wav --fx gain,by=-3dB --check
auralis render input.wav -o output.wav --fx gain,by=-3dB --plan --format json
```

Both modes are implemented by the graph engine after frontend lowering to `GraphRequest`; CLI only selects the mode and output format.

**Considered Options**

- Keep `auralis check` and `auralis plan` command mirrors: rejected because they encourage duplicate parsing and lowering paths.
- Use `--check` and `--plan` flags on executable commands: accepted because validation, planning, and execution reuse the same frontend lowering path and graph engine.
