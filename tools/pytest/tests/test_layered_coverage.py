"""Checks for the checked-in L0-L7 layered coverage matrix."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]


def test_layered_coverage_matrix_matches_implemented_surface() -> None:
    result = subprocess.run(
        [sys.executable, str(REPO_ROOT / "tools" / "check_layered_coverage.py")],
        cwd=REPO_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0, result.stderr
