from __future__ import annotations

import json
import math
import subprocess
import tomllib
from pathlib import Path
from typing import Any

import numpy as np
import pytest
from scipy.io import wavfile

from auralis_testkit.corpus import pcm16_fixture
from auralis_testkit.metrics import max_abs_error, peak, rms_error, snr_db
from auralis_testkit.sox_ng import SoxNgUnavailable, run_sox_ng_with_inputs

REPO_ROOT = Path(__file__).resolve().parents[3]
MANIFEST_PATHS = (
    REPO_ROOT / "tests" / "golden" / "concat.toml",
    REPO_ROOT / "tests" / "golden" / "sequence.toml",
)
SAMPLE_RATE = 48_000
PCM16_SCALE = np.float64(32_768.0)


def _load_combine_cases() -> tuple[tuple[str, dict[str, Any]], ...]:
    cases: list[tuple[str, dict[str, Any]]] = []
    for manifest_path in MANIFEST_PATHS:
        manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        for case_id, case in manifest["id"].items():
            case = dict(case)
            case.setdefault("combine", "concatenate")
            cases.append((case_id, case))

    return tuple(sorted(cases))


COMBINE_CASES = _load_combine_cases()


@pytest.mark.golden
@pytest.mark.parametrize(
    ("case_id", "case"),
    COMBINE_CASES,
    ids=[case_id for case_id, _ in COMBINE_CASES],
)
def test_cli_combine_matches_sox_ng_golden_manifest(
    case_id: str,
    case: dict[str, Any],
    tmp_path: Path,
) -> None:
    combine = case["combine"]
    input_paths = [
        _write_fixture(input_name, tmp_path / input_name)
        for input_name in case["inputs"]
    ]
    auralis_output = tmp_path / f"{case_id}.auralis.wav"
    sox_output = tmp_path / f"{case_id}.sox.wav"
    auralis_command = [
        "cargo",
        "run",
        "--quiet",
        "--package",
        "auralis-cli",
        "--",
        "run",
        str(input_paths[0]),
        str(auralis_output),
        "--combine",
        combine,
        *[
            option
            for input_path in input_paths[1:]
            for option in ("--input", str(input_path))
        ],
        *case["auralis"],
    ]
    auralis_result = subprocess.run(
        auralis_command,
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )

    try:
        sox_result = run_sox_ng_with_inputs(
            input_paths,
            sox_output,
            case["sox_ng"],
            combine=combine,
        )
    except SoxNgUnavailable as error:
        pytest.skip(str(error))

    auralis_rate, auralis_samples = _read_pcm16(auralis_output)
    sox_rate, sox_samples = _read_pcm16(sox_output)
    metadata = _metadata(auralis_rate, auralis_samples, sox_rate, sox_samples)

    assert metadata["auralis"]["sample_rate"] == metadata["sox_ng"]["sample_rate"]
    assert metadata["auralis"]["channels"] == metadata["sox_ng"]["channels"]
    assert metadata["auralis"]["frames"] == metadata["sox_ng"]["frames"]

    reference = _normalize_pcm16(sox_samples)
    actual = _normalize_pcm16(auralis_samples)
    metrics = {
        "max_abs": max_abs_error(reference, actual),
        "rms": rms_error(reference, actual),
        "snr_db": snr_db(reference, actual),
        "peak_delta": abs(peak(reference) - peak(actual)),
    }

    failures = []
    if metrics["max_abs"] > case["max_abs"]:
        failures.append(f"max_abs {metrics['max_abs']} > {case['max_abs']}")
    if metrics["rms"] > case["rms"]:
        failures.append(f"rms {metrics['rms']} > {case['rms']}")
    if metrics["snr_db"] < case["snr_db"]:
        failures.append(f"snr_db {metrics['snr_db']} < {case['snr_db']}")
    if metrics["peak_delta"] > case["max_abs"]:
        failures.append(f"peak_delta {metrics['peak_delta']} > {case['max_abs']}")

    if failures:
        report_path = tmp_path / f"{case_id}.failure.json"
        report = {
            "case_id": case_id,
            "inputs": case["inputs"],
            "combine": combine,
            "auralis_command": auralis_result.args,
            "sox_ng_command": sox_result.args,
            "tolerances": {
                "max_abs": case["max_abs"],
                "rms": case["rms"],
                "snr_db": case["snr_db"],
            },
            "metrics": {name: _json_number(value) for name, value in metrics.items()},
            "metadata": metadata,
        }
        report_path.write_text(
            json.dumps(report, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        pytest.fail("; ".join(failures) + f"; report={report_path}")


def _write_fixture(input_name: str, path: Path) -> Path:
    fixtures = {
        "combine/mono_short.wav": _mono_short,
        "combine/mono_long.wav": _mono_long,
        "combine/stereo_front.wav": _stereo_front,
        "combine/stereo_tail.wav": _stereo_tail,
    }
    try:
        samples = fixtures[input_name]()
    except KeyError as error:
        raise AssertionError(f"no fixture generator for {input_name}") from error

    path.parent.mkdir(parents=True, exist_ok=True)
    return pcm16_fixture(path, samples, sample_rate=SAMPLE_RATE)


def _mono_short() -> np.ndarray:
    return np.array([0.25, -0.5], dtype=np.float32)


def _mono_long() -> np.ndarray:
    return np.array([0.75, 0.0, -0.25], dtype=np.float32)


def _stereo_front() -> np.ndarray:
    return np.array(
        [[-0.5, -0.25, 0.0], [0.5, 0.25, 0.0]],
        dtype=np.float32,
    )


def _stereo_tail() -> np.ndarray:
    return np.array(
        [[0.25, 0.5], [-0.25, -0.5]],
        dtype=np.float32,
    )


def _read_pcm16(path: Path) -> tuple[int, np.ndarray]:
    sample_rate, samples = wavfile.read(path)
    assert samples.dtype == np.int16
    return sample_rate, samples


def _normalize_pcm16(samples: np.ndarray) -> np.ndarray:
    return samples.astype(np.float64) / PCM16_SCALE


def _metadata(
    auralis_rate: int,
    auralis_samples: np.ndarray,
    sox_rate: int,
    sox_samples: np.ndarray,
) -> dict[str, dict[str, int]]:
    auralis_frames, auralis_channels = _shape(auralis_samples)
    sox_frames, sox_channels = _shape(sox_samples)
    return {
        "auralis": {
            "sample_rate": auralis_rate,
            "channels": auralis_channels,
            "frames": auralis_frames,
        },
        "sox_ng": {
            "sample_rate": sox_rate,
            "channels": sox_channels,
            "frames": sox_frames,
        },
    }


def _shape(samples: np.ndarray) -> tuple[int, int]:
    if samples.ndim == 1:
        return int(samples.shape[0]), 1
    return int(samples.shape[0]), int(samples.shape[1])


def _json_number(value: float) -> float | str:
    if math.isfinite(value):
        return value
    return "inf" if value > 0 else "-inf"
