"""Shared Python helpers for Auralis pytest suites."""

from auralis_testkit.corpus import (
    CORPUS_IDS,
    CorpusCase,
    corpus_case,
    pcm16_corpus_fixture,
    pcm16_fixture,
    sine_wave,
)
from auralis_testkit.metrics import dc_offset, max_abs_error, peak, rms_error, snr_db
from auralis_testkit.sox_ng import SoxNgUnavailable, find_sox_ng, run_sox_ng

__all__ = [
    "SoxNgUnavailable",
    "CORPUS_IDS",
    "CorpusCase",
    "corpus_case",
    "dc_offset",
    "find_sox_ng",
    "max_abs_error",
    "pcm16_corpus_fixture",
    "pcm16_fixture",
    "peak",
    "rms_error",
    "run_sox_ng",
    "sine_wave",
    "snr_db",
]
