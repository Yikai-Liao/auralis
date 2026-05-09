"""Numerical metrics shared by Python golden tests."""

from __future__ import annotations

import math

import numpy as np


def _as_float64_vector(samples: np.ndarray) -> np.ndarray:
    return np.asarray(samples, dtype=np.float64).reshape(-1)


def _align(reference: np.ndarray, actual: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    reference_vector = _as_float64_vector(reference)
    actual_vector = _as_float64_vector(actual)
    length = max(reference_vector.size, actual_vector.size)
    if length == 0:
        return reference_vector, actual_vector
    return (
        np.pad(reference_vector, (0, length - reference_vector.size)),
        np.pad(actual_vector, (0, length - actual_vector.size)),
    )


def max_abs_error(reference: np.ndarray, actual: np.ndarray) -> float:
    """Return maximum absolute error, padding unequal inputs with silence."""

    lhs, rhs = _align(reference, actual)
    if lhs.size == 0:
        return 0.0
    return float(np.max(np.abs(lhs - rhs)))


def rms_error(reference: np.ndarray, actual: np.ndarray) -> float:
    """Return root-mean-square error, padding unequal inputs with silence."""

    lhs, rhs = _align(reference, actual)
    if lhs.size == 0:
        return 0.0
    return float(np.sqrt(np.mean(np.square(lhs - rhs))))


def snr_db(reference: np.ndarray, actual: np.ndarray) -> float:
    """Return signal-to-noise ratio in dB with deterministic silence behavior."""

    lhs, rhs = _align(reference, actual)
    if lhs.size == 0:
        return math.inf

    signal_power = float(np.mean(np.square(lhs)))
    error_power = float(np.mean(np.square(lhs - rhs)))
    if error_power == 0.0:
        return math.inf
    if signal_power == 0.0:
        return -math.inf
    return 10.0 * math.log10(signal_power / error_power)


def peak(samples: np.ndarray) -> float:
    """Return peak absolute sample level, or 0.0 for empty inputs."""

    vector = _as_float64_vector(samples)
    if vector.size == 0:
        return 0.0
    return float(np.max(np.abs(vector)))


def dc_offset(samples: np.ndarray) -> float:
    """Return arithmetic mean sample level, or 0.0 for empty inputs."""

    vector = _as_float64_vector(samples)
    if vector.size == 0:
        return 0.0
    return float(np.mean(vector))
