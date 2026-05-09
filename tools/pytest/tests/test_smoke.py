from __future__ import annotations

import math
from pathlib import Path

import numpy as np
import pytest
from scipy.io import wavfile

from auralis_testkit.corpus import CORPUS_IDS, corpus_case, pcm16_fixture, sine_wave
from auralis_testkit.metrics import dc_offset, max_abs_error, peak, rms_error, snr_db
from auralis_testkit.sox_ng import SoxNgUnavailable, run_sox_ng


def test_pytest_harness_imports_testkit_modules() -> None:
    assert callable(sine_wave)
    assert callable(corpus_case)
    assert callable(max_abs_error)
    assert callable(run_sox_ng)


def test_corpus_writes_pcm16_wav(tmp_path: Path) -> None:
    samples = np.array([[-1.0, -0.5, 0.0, 0.5, 1.0]], dtype=np.float32)
    wav_path = pcm16_fixture(tmp_path / "fixture.wav", samples, sample_rate=8_000)

    sample_rate, pcm = wavfile.read(wav_path)

    assert sample_rate == 8_000
    np.testing.assert_array_equal(
        pcm,
        np.array([-32768, -16384, 0, 16384, 32767], dtype=np.int16),
    )


def test_sine_wave_is_planar_and_deterministic() -> None:
    samples = sine_wave(sample_rate=8, frequency_hz=1.0, frames=4, channels=2)

    assert samples.shape == (2, 4)
    assert samples.dtype == np.float32
    np.testing.assert_allclose(samples[0], samples[1])
    np.testing.assert_allclose(samples[0], [0.0, 0.35355338, 0.5, 0.35355338])


def test_l0_corpus_cases_are_stable_and_cover_required_families() -> None:
    required_ids = {
        "l0/silence_mono_16",
        "l0/impulse_mono_16",
        "l0/step_mono_16",
        "l0/sine_mono_32",
        "l0/sweep_mono_64",
        "l0/noise_mono_32_seed_1",
        "l0/full_scale_mono_8",
        "l0/near_zero_mono_8",
        "l0/odd_length_mono_17",
        "l0/sine_stereo_32",
        "l0/short_mono_3",
        "l0/short_stereo_2",
    }

    assert required_ids.issubset(set(CORPUS_IDS))
    for corpus_id in required_ids:
        case = corpus_case(corpus_id)
        assert case.id == corpus_id
        assert case.sample_rate == 48_000
        assert case.samples.dtype == np.float32
        assert case.samples.shape == (case.channels, case.frames)
        assert np.all(np.isfinite(case.samples))


def test_python_corpus_matches_rust_documented_values() -> None:
    sine = corpus_case("l0/sine_mono_32")
    noise = corpus_case("l0/noise_mono_32_seed_1")
    chain_fixture = corpus_case("chains/stereo_steps")

    assert sine.samples[0, 0] == 0.0
    assert sine.samples[0, 1] == pytest.approx(0.065_263_09)
    assert noise.samples[0, 0] == pytest.approx(-0.263_544_5)
    assert chain_fixture.samples.shape == (2, 10)
    assert chain_fixture.samples[0, 0] == pytest.approx(-0.6)
    assert chain_fixture.samples[1, 0] == pytest.approx(0.3)


def test_metrics_match_documented_edge_behavior() -> None:
    reference = np.array([1.0, -1.0], dtype=np.float32)
    actual = np.array([1.0, 0.0, 0.5], dtype=np.float32)

    assert max_abs_error(reference, actual) == 1.0
    assert rms_error(reference, actual) == pytest.approx(math.sqrt(1.25 / 3.0))
    assert snr_db(reference, reference) == math.inf
    assert snr_db(np.zeros(2, dtype=np.float32), actual[:2]) == -math.inf
    assert peak(np.array([-0.25, 0.75], dtype=np.float32)) == 0.75
    assert dc_offset(np.array([1.0, -0.5], dtype=np.float32)) == 0.25


def test_sox_ng_wrapper_reports_missing_binary(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    monkeypatch.delenv("AURALIS_SOX_NG_BIN", raising=False)
    monkeypatch.setenv("PATH", "")

    with pytest.raises(SoxNgUnavailable):
        run_sox_ng(tmp_path / "in.wav", tmp_path / "out.wav", ["gain", "-3"])
