"""Shared Python helpers for Auralis pytest suites."""

from auralis_testkit.corpus import (
    CORPUS_IDS,
    CorpusCase,
    corpus_case,
    pcm16_corpus_fixture,
    pcm16_fixture,
    sine_wave,
)
from auralis_testkit.golden_report import (
    GOLDEN_FAILURE_REPORT_SCHEMA,
    audio_metadata,
    build_golden_failure_report,
    comparison_metadata,
    first_offending_index,
    golden_metric_failures,
    golden_metrics,
    write_golden_failure_report,
)
from auralis_testkit.metrics import dc_offset, max_abs_error, peak, rms_error, snr_db
from auralis_testkit.sox_ng import SoxNgUnavailable, find_sox_ng, run_sox_ng

__all__ = [
    "SoxNgUnavailable",
    "CORPUS_IDS",
    "CorpusCase",
    "GOLDEN_FAILURE_REPORT_SCHEMA",
    "audio_metadata",
    "build_golden_failure_report",
    "comparison_metadata",
    "corpus_case",
    "dc_offset",
    "find_sox_ng",
    "first_offending_index",
    "golden_metric_failures",
    "golden_metrics",
    "max_abs_error",
    "pcm16_corpus_fixture",
    "pcm16_fixture",
    "peak",
    "rms_error",
    "run_sox_ng",
    "sine_wave",
    "snr_db",
    "write_golden_failure_report",
]
