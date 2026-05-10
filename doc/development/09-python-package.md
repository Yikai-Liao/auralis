# 9.x Python and Package Milestone

Do not add PyO3 bindings until effect pipeline behavior is stable.

## Future work

- Python package via PyO3.
- NumPy-compatible buffer interface.
- stable public crate release.

## Preconditions

Only add PyO3 bindings when all are true:

1. Rust library API is stable enough to expose;
2. WAV pipeline is tested;
3. basic effects are tested;
4. error model is stable;
5. buffer model is stable;
6. Python package behavior can be documented clearly;
7. README L0-L7 testing infrastructure is implemented well enough to cover the
   binding boundary.

Before then, Python remains a test harness for corpus generation, golden
comparison, metrics, and failure artifacts.
