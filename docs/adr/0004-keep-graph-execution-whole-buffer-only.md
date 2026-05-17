# Keep graph execution whole-buffer only

Auralis graph execution is whole-buffer offline rendering, not chunked, streaming, or realtime graph processing. `auralis-graph` should plan and execute finite audio DAGs over complete audio artifacts: inputs decode into complete `AudioBuffer` values, graph nodes consume complete buffers or buffer collections, and nodes produce complete buffers or output artifacts.

Parallelism is allowed only where it preserves offline determinism: independent DAG nodes, channel-level work, buffer-internal data-parallel kernels, and fixed-order reductions. Feedback-heavy DSP remains internal effect state inside a node rather than a streaming graph concern.

**Considered Options**

- Add chunked or streaming graph execution later: rejected because it would pull Auralis toward realtime scheduling, block state, and cross-node backpressure that are outside the product direction.
- Keep graph execution whole-buffer only: accepted because Auralis targets deterministic offline audio processing, reproducible plans, cacheable artifacts, and simple library/CLI semantics over finite audio files.
