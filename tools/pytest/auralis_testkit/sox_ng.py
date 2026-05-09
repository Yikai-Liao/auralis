"""Small SoX-ng subprocess wrapper for golden tests."""

from __future__ import annotations

import os
import shutil
import subprocess
from collections.abc import Sequence
from pathlib import Path


class SoxNgUnavailable(RuntimeError):
    """Raised when sox_ng cannot be found for a golden test."""


def find_sox_ng() -> str | None:
    """Return the configured SoX-ng executable path, if available."""

    configured = os.environ.get("AURALIS_SOX_NG_BIN")
    if configured:
        return configured
    return shutil.which("sox_ng")


def run_sox_ng(
    input_path: Path,
    output_path: Path,
    effect_args: Sequence[str],
    *,
    output_encoding: Sequence[str] = ("-b", "16", "-e", "signed-integer"),
) -> subprocess.CompletedProcess[bytes]:
    """Run SoX-ng with deterministic flags and return the completed process."""

    executable = find_sox_ng()
    if executable is None:
        raise SoxNgUnavailable("sox_ng is not available in PATH")

    command = [
        executable,
        "-R",
        "-D",
        str(input_path),
        *output_encoding,
        str(output_path),
        *effect_args,
    ]
    return subprocess.run(command, check=True, capture_output=True)


def run_sox_ng_with_inputs(
    input_paths: Sequence[Path],
    output_path: Path,
    effect_args: Sequence[str],
    *,
    combine: str = "concatenate",
    output_encoding: Sequence[str] = ("-b", "16", "-e", "signed-integer"),
) -> subprocess.CompletedProcess[bytes]:
    """Run SoX-ng with multiple inputs and deterministic combine settings."""

    executable = find_sox_ng()
    if executable is None:
        raise SoxNgUnavailable("sox_ng is not available in PATH")
    if not input_paths:
        raise ValueError("input_paths must not be empty")

    command = [
        executable,
        "-R",
        "-D",
        "--combine",
        combine,
        *(str(path) for path in input_paths),
        *output_encoding,
        str(output_path),
        *effect_args,
    ]
    return subprocess.run(command, check=True, capture_output=True)
