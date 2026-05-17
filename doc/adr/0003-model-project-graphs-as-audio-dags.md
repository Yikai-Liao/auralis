# Model project graphs as audio DAGs

Auralis project graphs will be finite audio DAGs: sources, processing nodes, combiners, and sinks connected without project-level cycles. Feedback, delay lines, recursive filters, and reverberation feedback remain internal effect state rather than graph edges, which keeps planning, cache invalidation, `plan` output, and `run` execution compatible with topological scheduling.

**Considered Options**

- Allow cyclic project graphs from the first graph engine: rejected because it would require a different scheduling model before Auralis has stable offline DAG execution.
- Model feedback-heavy DSP as internal effect state inside DAG nodes: accepted because it preserves expressive audio processing while keeping the project graph deterministic and inspectable.
