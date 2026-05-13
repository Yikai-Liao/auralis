from __future__ import annotations

from pathlib import Path

from auralis_testkit.benchmarks import (
    BackendMode,
    benchmark_case_catalog,
    render_markdown_report,
    supported_effect_names,
)

REPO_ROOT = Path(__file__).resolve().parents[3]


def test_benchmark_catalog_covers_every_supported_effect() -> None:
    supported = supported_effect_names(REPO_ROOT)
    cases = benchmark_case_catalog(REPO_ROOT)

    assert [case.effect_name for case in cases] == supported
    assert len(cases) == 63


def test_benchmark_catalog_marks_backend_capability_and_profile_needs() -> None:
    cases = {case.effect_name: case for case in benchmark_case_catalog(REPO_ROOT)}

    assert cases["gain"].backend_mode is BackendMode.SCALAR_AND_SIMD
    assert cases["chorus"].backend_mode is BackendMode.SCALAR_ONLY
    assert cases["reverse"].tokens == ("reverse",)
    assert cases["noisered"].needs_profile is True
    assert cases["noisered"].resolve_tokens(REPO_ROOT / "tmp/profile.prof") == (
        "noisered",
        str(REPO_ROOT / "tmp/profile.prof"),
        "0.25",
    )


def test_render_markdown_report_smoke() -> None:
    report = {
        "generated_at_utc": "2026-05-13T00:00:00+00:00",
        "auralis_version": "0.0.0",
        "sox_ng_version": "sox_ng 14.4.3",
        "config": {
            "duration_seconds": 90,
            "sample_rate": 48_000,
            "iterations": 5,
            "warmups": 1,
        },
        "cases": [
            {
                "effect_name": "gain",
                "backend_mode": "scalar_and_simd",
                "status": "ok",
                "runs": {
                    "sox_ng": {"status": "ok", "summary": {"median_ms": 11.0}},
                    "auralis_scalar": {"status": "ok", "summary": {"median_ms": 8.0}},
                    "auralis_simd": {"status": "ok", "summary": {"median_ms": 5.0}},
                },
                "comparisons": {
                    "scalar_vs_sox_ng": {"median_ratio": 0.727},
                    "simd_vs_sox_ng": {"median_ratio": 0.455},
                    "simd_vs_scalar": {"median_ratio": 0.625},
                },
            }
        ],
    }

    markdown = render_markdown_report(report)

    assert "SoX-ng Benchmark Report" in markdown
    assert "| gain | scalar_and_simd | 11.0 | 8.0 | 5.0 | 0.727 | 0.455 | 0.625 |" in markdown
