"""CLI wrapper for the SoX-ng effect benchmark runner."""

from __future__ import annotations

import sys
from pathlib import Path

TOOLS_PYTEST = Path(__file__).resolve().parent / "pytest"
if str(TOOLS_PYTEST) not in sys.path:
    sys.path.insert(0, str(TOOLS_PYTEST))

from auralis_testkit.benchmarks import main


if __name__ == "__main__":
    raise SystemExit(main())
