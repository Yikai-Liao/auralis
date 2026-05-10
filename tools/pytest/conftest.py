from __future__ import annotations

import pytest

from auralis_testkit.sox_ng import find_sox_ng


def pytest_configure(config: pytest.Config) -> None:
    markexpr = config.option.markexpr or ""
    if _is_golden_gate(markexpr) and find_sox_ng() is None:
        raise pytest.UsageError(
            "release/gnhf golden validation requires a real sox_ng oracle; "
            "set AURALIS_SOX_NG_BIN or install sox_ng on PATH"
        )


def _is_golden_gate(markexpr: str) -> bool:
    normalized = " ".join(markexpr.split())
    return "golden" in normalized and "not golden" not in normalized
