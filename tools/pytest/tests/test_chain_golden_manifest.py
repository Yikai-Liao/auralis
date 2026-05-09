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
from auralis_testkit.sox_ng import SoxNgUnavailable, run_sox_ng

REPO_ROOT = Path(__file__).resolve().parents[3]
MANIFEST_PATH = REPO_ROOT / "tests" / "golden" / "chains.toml"
SAMPLE_RATE = 48_000
PCM16_SCALE = np.float64(32_768.0)


def _load_chain_cases() -> tuple[tuple[str, dict[str, Any]], ...]:
    manifest = tomllib.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    return tuple(sorted(manifest["id"].items()))


CHAIN_CASES = _load_chain_cases()


@pytest.mark.golden
@pytest.mark.parametrize(
    ("case_id", "case"),
    CHAIN_CASES,
    ids=[case_id for case_id, _ in CHAIN_CASES],
)
def test_cli_chain_matches_sox_ng_golden_manifest(
    case_id: str,
    case: dict[str, Any],
    tmp_path: Path,
) -> None:
    input_path = _write_fixture(case["input"], tmp_path / case["input"])
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
        str(input_path),
        str(auralis_output),
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
        sox_result = run_sox_ng(input_path, sox_output, case["sox_ng"])
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
            "input": case["input"],
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
        "chains/stereo_steps.wav": _stereo_steps,
        "chains/stereo_multitone.wav": _stereo_multitone,
        "chains/stereo_ramp_tone.wav": _stereo_ramp_tone,
    }
    try:
        samples = fixtures[input_name]()
    except KeyError as error:
        raise AssertionError(f"no fixture generator for {input_name}") from error

    path.parent.mkdir(parents=True, exist_ok=True)
    return pcm16_fixture(path, samples, sample_rate=SAMPLE_RATE)


def _stereo_steps() -> np.ndarray:
    left = np.array(
        [-0.6, -0.45, -0.3, -0.15, 0.0, 0.15, 0.3, 0.45, 0.6, 0.75],
        dtype=np.float32,
    )
    right = -left / np.float32(2.0)
    return np.stack([left, right])


def _stereo_multitone() -> np.ndarray:
    frame = np.arange(64, dtype=np.float32)
    left = (
        np.float32(0.25) * np.sin(np.float32(2.0 * math.pi / 16.0) * frame)
        + np.float32(0.1) * np.sin(np.float32(2.0 * math.pi / 7.0) * frame)
    )
    right = (
        np.float32(0.20) * np.cos(np.float32(2.0 * math.pi / 11.0) * frame)
        - np.float32(0.05) * np.sin(np.float32(2.0 * math.pi / 5.0) * frame)
    )
    return np.stack([left.astype(np.float32), right.astype(np.float32)])


def _stereo_ramp_tone() -> np.ndarray:
    frame = np.arange(32, dtype=np.float32)
    left = np.linspace(-0.5, 0.5, 32, dtype=np.float32)
    right = np.float32(0.35) * np.sin(np.float32(2.0 * math.pi / 8.0) * frame)
    return np.stack([left, right.astype(np.float32)])


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
