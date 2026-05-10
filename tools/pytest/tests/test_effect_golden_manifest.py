from __future__ import annotations

import subprocess
import tomllib
from pathlib import Path
from typing import Any

import numpy as np
import pytest
from scipy.io import wavfile

from auralis_testkit.corpus import corpus_case, pcm16_corpus_fixture
from auralis_testkit.golden_report import (
    build_golden_failure_report,
    comparison_metadata,
    golden_metric_failures,
    golden_metrics,
    write_golden_failure_report,
)
from auralis_testkit.sox_ng import SoxNgUnavailable, run_sox_ng

REPO_ROOT = Path(__file__).resolve().parents[3]
MANIFEST_PATH = REPO_ROOT / "tests" / "golden" / "effects.toml"
PCM16_SCALE = np.float64(32_768.0)


def _load_effect_cases() -> tuple[tuple[str, dict[str, Any]], ...]:
    manifest = tomllib.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    return tuple(sorted(manifest["id"].items()))


EFFECT_CASES = _load_effect_cases()


@pytest.mark.golden
@pytest.mark.parametrize(
    ("case_id", "case"),
    EFFECT_CASES,
    ids=[case_id for case_id, _ in EFFECT_CASES],
)
def test_cli_standalone_effect_matches_sox_ng_golden_manifest(
    case_id: str,
    case: dict[str, Any],
    tmp_path: Path,
) -> None:
    input_path = _write_fixture(case["corpus_id"], tmp_path / case["input"])
    effect_tokens = _resolve_effect_tokens(case, tmp_path)
    auralis_output = tmp_path / f"{case_id}.auralis.wav"
    sox_output = tmp_path / f"{case_id}.sox.wav"
    output_sample_rate = case.get("output_sample_rate")

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
        *(["--rate", str(output_sample_rate)] if output_sample_rate is not None else []),
        *effect_tokens["auralis"],
    ]
    auralis_result = subprocess.run(
        auralis_command,
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )

    try:
        sox_result = run_sox_ng(
            input_path,
            sox_output,
            effect_tokens["sox_ng"],
            output_sample_rate=output_sample_rate,
            disable_auto_dither=not _is_explicit_dither_case(case),
        )
    except SoxNgUnavailable as error:
        pytest.skip(str(error))

    auralis_rate, auralis_samples = _read_pcm16(auralis_output)
    sox_rate, sox_samples = _read_pcm16(sox_output)
    metadata = comparison_metadata(auralis_rate, auralis_samples, sox_rate, sox_samples)

    assert metadata["auralis"]["sample_rate"] == metadata["sox_ng"]["sample_rate"]
    assert metadata["auralis"]["channel_count"] == metadata["sox_ng"]["channel_count"]
    assert metadata["auralis"]["frame_count"] == metadata["sox_ng"]["frame_count"]

    reference = _normalize_pcm16(sox_samples)
    actual = _normalize_pcm16(auralis_samples)
    metrics = golden_metrics(reference, actual)
    failures = golden_metric_failures(metrics, case, reference, actual)
    if _is_explicit_dither_case(case):
        failures = [failure for failure in failures if failure["metric"] != "snr_db"]

    if failures:
        report_path = tmp_path / f"{case_id}.failure.json"
        report = build_golden_failure_report(
            case_id=case_id,
            case=case,
            backend="scalar",
            repo_root=REPO_ROOT,
            auralis_command=auralis_result.args,
            sox_ng_command=sox_result.args,
            outputs=metadata,
            metrics=metrics,
            failures=failures,
        )
        report["sox_ng_automatic_behavior"] = {
            "channels": "absent",
            "rate": "explicit-output-rate" if output_sample_rate is not None else "absent",
            "guard": "absent",
            "norm": "absent",
            "dither": "explicit-effect" if _is_explicit_dither_case(case) else "disabled",
        }
        write_golden_failure_report(report_path, report)
        pytest.fail(_failure_summary(failures) + f"; report={report_path}")


def _write_fixture(corpus_id: str, path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    return pcm16_corpus_fixture(path, corpus_id)


def _resolve_effect_tokens(case: dict[str, Any], tmp_path: Path) -> dict[str, list[str]]:
    profile_placeholders = {
        token
        for command in (case["auralis"], case["sox_ng"])
        for token in command
        if token == "{zero_noise_profile}"
    }
    replacements: dict[str, str] = {}
    if profile_placeholders:
        channels = _corpus_channels(case["corpus_id"])
        profile_path = tmp_path / "zero_noise_profile.prof"
        _write_zero_noise_profile(profile_path, channels)
        replacements["{zero_noise_profile}"] = str(profile_path)

    return {
        name: [replacements.get(token, token) for token in case[name]]
        for name in ("auralis", "sox_ng")
    }


def _is_explicit_dither_case(case: dict[str, Any]) -> bool:
    return bool(case["sox_ng"] and case["sox_ng"][0] == "dither")


def _write_zero_noise_profile(path: Path, channels: int) -> None:
    bins = ", ".join(["0.000000"] * 1025)
    text = "".join(f"Channel {channel}: {bins}\n" for channel in range(channels))
    path.write_text(text, encoding="utf-8")


def _corpus_channels(corpus_id: str) -> int:
    return corpus_case(corpus_id).channels


def _read_pcm16(path: Path) -> tuple[int, np.ndarray]:
    sample_rate, samples = wavfile.read(path)
    assert samples.dtype == np.int16
    return sample_rate, samples


def _normalize_pcm16(samples: np.ndarray) -> np.ndarray:
    return samples.astype(np.float64) / PCM16_SCALE


def _failure_summary(failures: list[dict[str, Any]]) -> str:
    return "; ".join(
        f"{failure['metric']} {failure['actual']} {failure['comparison']} {failure['expected']}"
        for failure in failures
    )
