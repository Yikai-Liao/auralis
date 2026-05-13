use thiserror::Error;

/// Crate-local result type for FIR primitive constructors.
pub type FirResult<T> = std::result::Result<T, FirError>;

/// Errors produced by reusable FIR DSP primitive constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum FirError {
    /// A FIR coefficient was not finite.
    #[error("fir coefficients must be finite numbers")]
    InvalidCoefficients,
}

/// A validated list of FIR coefficients for reusable DSP processors.
///
/// Coefficients are stored in convolution order. Empty coefficient lists are
/// allowed so effect boundaries can preserve SoX-ng's null-FIR behavior without
/// special-casing the numeric primitive.
///
/// # Examples
///
/// ```
/// use auralis_dsp::FirCoefficients;
///
/// let coefficients = FirCoefficients::new([0.25, 0.5, 0.25])?;
/// assert_eq!(coefficients.as_slice(), &[0.25, 0.5, 0.25]);
/// # Ok::<(), auralis_dsp::FirError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FirCoefficients {
    values: Vec<f64>,
    fast_values: Vec<f32>,
}

impl FirCoefficients {
    /// Creates FIR coefficients from already parsed numeric values.
    ///
    /// # Errors
    ///
    /// Returns [`FirError::InvalidCoefficients`] when any coefficient is NaN or
    /// infinite.
    pub fn new<I>(values: I) -> FirResult<Self>
    where
        I: IntoIterator<Item = f64>,
    {
        let values = values.into_iter().collect::<Vec<_>>();
        if values.iter().all(|value| value.is_finite()) {
            let fast_values = values.iter().map(|value| *value as f32).collect();
            Ok(Self {
                values,
                fast_values,
            })
        } else {
            Err(FirError::InvalidCoefficients)
        }
    }

    /// Returns the coefficients in convolution order.
    #[must_use]
    pub fn as_slice(&self) -> &[f64] {
        &self.values
    }

    /// Returns the coefficients as `f32` values for scalar fast paths.
    #[must_use]
    pub fn as_f32_slice(&self) -> &[f32] {
        &self.fast_values
    }

    /// Returns the number of coefficients.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns `true` when no coefficients were supplied.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Stateful scalar FIR processor for one mono sample stream.
///
/// The state emits samples with SoX-ng-compatible centered alignment. For
/// coefficient lists longer than two taps, output for the newest input sample
/// is delayed until enough look-ahead is available; callers must invoke
/// [`Self::finish`] once at end-of-stream to flush the final aligned samples.
#[derive(Debug, Clone)]
pub struct FirState {
    coefficients: FirCoefficients,
    history: Vec<f32>,
    history_start: usize,
    history_len: usize,
    samples_seen: usize,
    samples_emitted: usize,
    shift: usize,
}

impl FirState {
    /// Creates zero-initialized FIR state for one mono stream.
    #[must_use]
    pub fn new(coefficients: FirCoefficients) -> Self {
        let shift = coefficients.len().saturating_sub(1) / 2;
        let history_capacity = coefficients.len();
        Self {
            coefficients,
            history: vec![0.0; history_capacity],
            history_start: 0,
            history_len: 0,
            samples_seen: 0,
            samples_emitted: 0,
            shift,
        }
    }

    /// Processes one chunk of mono samples, appending available output samples.
    pub fn process_mono_samples(&mut self, input: &[f32], output: &mut Vec<f32>) {
        if self.coefficients.is_empty() {
            output.extend_from_slice(input);
            self.samples_seen += input.len();
            return;
        }

        for sample in input {
            self.push_sample(*sample);
            if self.samples_seen > self.shift {
                output.push(self.current_output_sample());
                self.samples_emitted += 1;
            }
        }
    }

    /// Processes a complete mono stream into an equally sized output slice.
    ///
    /// This length-preserving helper performs the same centered alignment as
    /// [`Self::process_mono_samples`] followed by [`Self::finish`], but writes
    /// directly into caller-owned storage.
    ///
    /// # Panics
    ///
    /// Panics when `output.len() != input.len()`.
    pub fn process_into(mut self, input: &[f32], output: &mut [f32]) {
        assert_eq!(input.len(), output.len());
        if self.coefficients.is_empty() {
            output.copy_from_slice(input);
            return;
        }

        let mut written = 0;
        for sample in input {
            self.push_sample(*sample);
            if self.samples_seen > self.shift {
                output[written] = self.current_output_sample();
                written += 1;
                self.samples_emitted += 1;
            }
        }

        let target_samples = self.samples_seen;
        for _ in 0..self.shift {
            self.push_sample(0.0);
            if self.samples_seen > self.shift && self.samples_emitted < target_samples {
                output[written] = self.current_output_sample();
                written += 1;
                self.samples_emitted += 1;
            }
        }

        debug_assert_eq!(written, output.len());
    }

    /// Flushes delayed end-of-stream samples by appending the remaining output.
    ///
    /// This method consumes the state so a stream cannot accidentally be
    /// flushed twice.
    pub fn finish(mut self, output: &mut Vec<f32>) {
        if self.coefficients.is_empty() {
            return;
        }

        let target_samples = self.samples_seen;
        for _ in 0..self.shift {
            self.push_sample(0.0);
            if self.samples_seen > self.shift && self.samples_emitted < target_samples {
                output.push(self.current_output_sample());
                self.samples_emitted += 1;
            }
        }
    }

    fn push_sample(&mut self, sample: f32) {
        if self.history.is_empty() {
            self.samples_seen += 1;
            return;
        }

        if self.history_len < self.history.len() {
            let write_index = (self.history_start + self.history_len) % self.history.len();
            self.history[write_index] = sample;
            self.history_len += 1;
        } else {
            self.history[self.history_start] = sample;
            self.history_start = (self.history_start + 1) % self.history.len();
        }
        self.samples_seen += 1;
    }

    fn current_output_sample(&self) -> f32 {
        let newest_index = self.samples_seen - 1;
        let oldest_index = self.samples_seen - self.history_len;
        let history_capacity = self.history.len();
        let mut value = 0.0_f32;
        for (coefficient_index, coefficient) in
            self.coefficients.as_f32_slice().iter().copied().enumerate()
        {
            if newest_index >= coefficient_index {
                let input_index = newest_index - coefficient_index;
                if input_index >= oldest_index {
                    let history_offset = input_index - oldest_index;
                    let history_index = (self.history_start + history_offset) % history_capacity;
                    value += self.history[history_index] * coefficient;
                }
            }
        }

        value
    }
}

#[cfg(test)]
mod tests {
    use super::{FirCoefficients, FirError, FirState};

    #[test]
    fn rejects_non_finite_coefficients() {
        assert_eq!(
            FirCoefficients::new([0.5, f64::NAN]).unwrap_err(),
            FirError::InvalidCoefficients
        );
        assert_eq!(
            FirCoefficients::new([f64::INFINITY]).unwrap_err(),
            FirError::InvalidCoefficients
        );
    }

    #[test]
    fn preserves_empty_coefficients_as_null_stream() {
        let mut state = FirState::new(FirCoefficients::new([]).unwrap());
        let mut output = Vec::new();

        state.process_mono_samples(&[0.0, 0.25, -0.5], &mut output);
        state.finish(&mut output);

        assert_eq!(output, [0.0, 0.25, -0.5]);
    }

    #[test]
    fn filters_mono_samples_with_centered_alignment() {
        let coefficients = FirCoefficients::new([1.0, 2.0, 3.0]).unwrap();
        let mut state = FirState::new(coefficients);
        let mut output = Vec::new();

        state.process_mono_samples(&[1.0, 0.0, 0.0, 0.0], &mut output);
        state.finish(&mut output);

        assert_eq!(output, [2.0, 3.0, 0.0, 0.0]);
    }

    #[test]
    fn preserves_alignment_across_chunks() {
        let whole = process_chunks(&[0.0, 1.0, 0.5, -0.5, 0.0], &[5]);
        let chunked = process_chunks(&[0.0, 1.0, 0.5, -0.5, 0.0], &[2, 1, 2]);

        assert_eq!(whole, chunked);
    }

    fn process_chunks(input: &[f32], chunks: &[usize]) -> Vec<f32> {
        let coefficients = FirCoefficients::new([0.25, 0.5, 0.25]).unwrap();
        let mut state = FirState::new(coefficients);
        let mut output = Vec::new();
        let mut start = 0;

        for chunk_len in chunks {
            let end = start + chunk_len;
            state.process_mono_samples(&input[start..end], &mut output);
            start = end;
        }
        state.finish(&mut output);
        output
    }
}
