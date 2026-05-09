"""Deterministic audio corpus helpers for pytest-based Auralis tests."""

from __future__ import annotations

import math
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from scipy.io import wavfile

DEFAULT_SAMPLE_RATE = 48_000

CORPUS_IDS = (
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
    "l0/opposite_phase_stereo_32",
    "l0/opposite_phase_stereo_8192",
    "l0/short_mono_3",
    "l0/short_stereo_2",
    "chains/stereo_steps",
    "chains/stereo_multitone",
    "chains/stereo_ramp_tone",
    "combine/mono_short",
    "combine/mono_long",
    "combine/stereo_front",
    "combine/stereo_tail",
    "auto/rate_48k_silence",
    "auto/stereo_channels",
    "auto/mono_channels",
    "auto/level_headroom",
)


@dataclass(frozen=True)
class CorpusCase:
    """One deterministic planar corpus case."""

    id: str
    sample_rate: int
    samples: np.ndarray

    @property
    def channels(self) -> int:
        """Return the channel count."""

        return int(self.samples.shape[0])

    @property
    def frames(self) -> int:
        """Return the frame count per channel."""

        return int(self.samples.shape[1])


def corpus_case(corpus_id: str) -> CorpusCase:
    """Return a deterministic corpus case by stable ID."""

    generators = {
        "l0/silence_mono_16": lambda: _mono(np.zeros(16, dtype=np.float32)),
        "l0/impulse_mono_16": _impulse_mono_16,
        "l0/step_mono_16": _step_mono_16,
        "l0/sine_mono_32": lambda: _mono(_sine(32, 1_000.0, 0.5, 0.0)),
        "l0/sweep_mono_64": lambda: _mono(_sweep(64, 200.0, 4_000.0, 0.45)),
        "l0/noise_mono_32_seed_1": lambda: _mono(_seeded_noise(32, 1, 0.5)),
        "l0/full_scale_mono_8": lambda: _mono(
            np.array([-1.0, 1.0, -1.0, 1.0, 0.999, -0.999, 0.0, -0.0], dtype=np.float32)
        ),
        "l0/near_zero_mono_8": lambda: _mono(
            np.array(
                [0.0, 0.000_001, -0.000_001, 0.000_01, -0.000_01, 0.000_1, -0.000_1, -0.0],
                dtype=np.float32,
            )
        ),
        "l0/odd_length_mono_17": lambda: _mono(
            np.array([-0.4 + frame * 0.8 / 16.0 for frame in range(17)], dtype=np.float32)
        ),
        "l0/sine_stereo_32": lambda: _stereo(
            _sine(32, 1_000.0, 0.5, 0.0),
            _sine(32, 500.0, 0.25, 0.25),
        ),
        "l0/opposite_phase_stereo_32": lambda: _opposite_phase_stereo(32),
        "l0/opposite_phase_stereo_8192": lambda: _opposite_phase_stereo_8192(),
        "l0/short_mono_3": lambda: _mono(np.array([0.25, -0.25, 0.0], dtype=np.float32)),
        "l0/short_stereo_2": lambda: _stereo(
            np.array([0.25, -0.25], dtype=np.float32),
            np.array([-0.5, 0.5], dtype=np.float32),
        ),
        "chains/stereo_steps": _stereo_steps,
        "chains/stereo_multitone": _stereo_multitone,
        "chains/stereo_ramp_tone": _stereo_ramp_tone,
        "combine/mono_short": lambda: _mono(np.array([0.25, -0.5], dtype=np.float32)),
        "combine/mono_long": lambda: _mono(np.array([0.75, 0.0, -0.25], dtype=np.float32)),
        "combine/stereo_front": lambda: _stereo(
            np.array([-0.5, -0.25, 0.0], dtype=np.float32),
            np.array([0.5, 0.25, 0.0], dtype=np.float32),
        ),
        "combine/stereo_tail": lambda: _stereo(
            np.array([0.25, 0.5], dtype=np.float32),
            np.array([-0.25, -0.5], dtype=np.float32),
        ),
        "auto/rate_48k_silence": lambda: _mono(np.zeros(4, dtype=np.float32)),
        "auto/stereo_channels": lambda: _stereo(
            np.array([0.25, -0.5], dtype=np.float32),
            np.array([0.75, 0.5], dtype=np.float32),
        ),
        "auto/mono_channels": lambda: _mono(np.array([0.25, -0.5, 0.0], dtype=np.float32)),
        "auto/level_headroom": lambda: _mono(
            np.array([0.0, 0.25, -0.5, 0.75], dtype=np.float32)
        ),
    }
    try:
        samples = generators[corpus_id]()
    except KeyError as error:
        raise ValueError(f"unknown corpus id {corpus_id!r}") from error

    return CorpusCase(corpus_id, DEFAULT_SAMPLE_RATE, samples)


def pcm16_corpus_fixture(path: Path, corpus_id: str) -> Path:
    """Write a corpus case as a PCM16 WAV fixture."""

    case = corpus_case(corpus_id)
    return pcm16_fixture(path, case.samples, sample_rate=case.sample_rate)


def sine_wave(
    *,
    sample_rate: int = 48_000,
    frequency_hz: float = 1_000.0,
    frames: int,
    channels: int = 1,
    amplitude: float = 0.5,
    phase: float = 0.0,
) -> np.ndarray:
    """Return a deterministic planar float32 sine wave."""

    if sample_rate <= 0:
        raise ValueError("sample_rate must be positive")
    if frames < 0:
        raise ValueError("frames must be non-negative")
    if channels <= 0:
        raise ValueError("channels must be positive")
    if not math.isfinite(frequency_hz):
        raise ValueError("frequency_hz must be finite")
    if not math.isfinite(amplitude):
        raise ValueError("amplitude must be finite")
    if not math.isfinite(phase):
        raise ValueError("phase must be finite")

    frame_index = np.arange(frames, dtype=np.float64)
    radians = (2.0 * math.pi * frequency_hz / sample_rate) * frame_index + phase
    mono = (np.sin(radians) * amplitude).astype(np.float32)
    return np.tile(mono, (channels, 1))


def pcm16_fixture(path: Path, samples: np.ndarray, *, sample_rate: int = 48_000) -> Path:
    """Write normalized planar float samples as a PCM16 WAV fixture."""

    if sample_rate <= 0:
        raise ValueError("sample_rate must be positive")

    normalized = np.asarray(samples, dtype=np.float32)
    if normalized.ndim == 1:
        normalized = normalized.reshape(1, -1)
    if normalized.ndim != 2:
        raise ValueError("samples must be a mono vector or planar channel matrix")
    if not np.all(np.isfinite(normalized)):
        raise ValueError("samples must be finite")

    quantized = np.rint(np.clip(normalized, -1.0, 1.0) * np.float32(32768.0))
    pcm = np.clip(quantized, -32768, 32767).astype(np.int16)
    interleaved = pcm.T if pcm.shape[0] > 1 else pcm.reshape(-1)
    wavfile.write(path, sample_rate, interleaved)
    return path


def _mono(samples: np.ndarray) -> np.ndarray:
    return np.asarray(samples, dtype=np.float32).reshape(1, -1)


def _stereo(left: np.ndarray, right: np.ndarray) -> np.ndarray:
    return np.stack(
        [np.asarray(left, dtype=np.float32), np.asarray(right, dtype=np.float32)]
    )


def _opposite_phase_stereo_8192() -> np.ndarray:
    return _opposite_phase_stereo(8192)


def _opposite_phase_stereo(frames: int) -> np.ndarray:
    left = _sine(frames, 750.0, 0.5, 0.0)
    return _stereo(left, -left)


def _impulse_mono_16() -> np.ndarray:
    samples = np.zeros(16, dtype=np.float32)
    samples[0] = np.float32(1.0)
    return _mono(samples)


def _step_mono_16() -> np.ndarray:
    return _mono(
        np.array([-0.5 if frame < 8 else 0.5 for frame in range(16)], dtype=np.float32)
    )


def _sine(frames: int, frequency_hz: float, amplitude: float, phase: float) -> np.ndarray:
    return np.array(
        [
            math.sin(2.0 * math.pi * frequency_hz * frame / DEFAULT_SAMPLE_RATE + phase)
            * amplitude
            for frame in range(frames)
        ],
        dtype=np.float32,
    )


def _sweep(frames: int, start_hz: float, end_hz: float, amplitude: float) -> np.ndarray:
    denominator = max(frames - 1, 1)
    phase = 0.0
    samples = []
    for frame in range(frames):
        frequency = start_hz + (end_hz - start_hz) * frame / denominator
        samples.append(math.sin(phase) * amplitude)
        phase += 2.0 * math.pi * frequency / DEFAULT_SAMPLE_RATE
    return np.array(samples, dtype=np.float32)


def _seeded_noise(frames: int, seed: int, amplitude: float) -> np.ndarray:
    state = seed & 0xFFFFFFFF
    samples = []
    for _ in range(frames):
        state = (state * 1_664_525 + 1_013_904_223) & 0xFFFFFFFF
        unit = (state >> 8) / 16_777_215.0
        samples.append((unit * 2.0 - 1.0) * amplitude)
    return np.array(samples, dtype=np.float32)


def _stereo_steps() -> np.ndarray:
    left = np.array(
        [-0.6, -0.45, -0.3, -0.15, 0.0, 0.15, 0.3, 0.45, 0.6, 0.75],
        dtype=np.float32,
    )
    right = -left / np.float32(2.0)
    return _stereo(left, right)


def _stereo_multitone() -> np.ndarray:
    frame = np.arange(64, dtype=np.float64)
    left = (
        0.25 * np.sin(2.0 * math.pi * frame / 16.0)
        + 0.1 * np.sin(2.0 * math.pi * frame / 7.0)
    )
    right = (
        0.20 * np.cos(2.0 * math.pi * frame / 11.0)
        - 0.05 * np.sin(2.0 * math.pi * frame / 5.0)
    )
    return _stereo(left.astype(np.float32), right.astype(np.float32))


def _stereo_ramp_tone() -> np.ndarray:
    left = np.linspace(-0.5, 0.5, 32, dtype=np.float32)
    right = _sine(32, 6_000.0, 0.35, 0.0)
    return _stereo(left, right)
