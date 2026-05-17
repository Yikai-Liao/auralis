# Use global plan mode on execution commands

Auralis will use a global `--plan` mode on executable commands instead of maintaining mirrored `auralis plan <command> ...` subcommands. This ensures `render --plan`, `convert --plan`, and future executable commands reuse the exact same command parser and lowering path as execution, then stop after producing the execution plan without reading audio data, running DSP, writing outputs, or writing cache artifacts.

Graph spec planning should also use the same rule through the execution command surface, for example `auralis run Auralis.toml --plan --format json`. Plan output has two first-class formats: human-readable text and stable machine-readable JSON.

**Considered Options**

- Keep `auralis plan <command> ...`: rejected because clap requires a mirrored command enum and makes it too easy to duplicate parsing and lowering.
- Use `--plan` on execution commands: accepted because plan mode becomes an execution flag over the real command path, not a second command implementation.
