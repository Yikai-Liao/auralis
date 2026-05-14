use std::sync::Arc;

use realfft::{ComplexToReal, RealFftPlanner, RealToComplex, num_complex::Complex32};
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
            #[allow(
                clippy::cast_possible_truncation,
                reason = "The f32 cache is an explicit fast path; f64 coefficients remain available as the reference representation."
            )]
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

/// FFT overlap-save FIR processor for complete mono streams.
///
/// The processor preserves the same centered SoX-ng alignment as
/// [`FirState::process_into`]. It is intended for long filters where direct
/// scalar convolution is too expensive; short filters should keep using
/// [`FirState`] to avoid FFT setup overhead.
#[derive(Clone)]
pub struct DftFir {
    coefficients: FirCoefficients,
    dft_len: usize,
    overlap: usize,
    advance: usize,
    kernel_spectrum: Vec<Complex32>,
    forward: Arc<dyn RealToComplex<f32>>,
    inverse: Arc<dyn ComplexToReal<f32>>,
}

impl DftFir {
    /// Creates an overlap-save FIR processor using a SoX-style FFT length.
    #[must_use]
    pub fn new(coefficients: FirCoefficients) -> Self {
        let dft_len = dft_len_for_taps(coefficients.len());
        Self::with_dft_len(coefficients, dft_len)
    }

    /// Creates an overlap-save FIR processor with a caller-provided FFT length.
    ///
    /// The requested length is rounded up so it is always greater than the
    /// filter overlap and is friendly to FFT planners.
    ///
    /// # Panics
    ///
    /// Panics only if the internally allocated real FFT buffers do not match
    /// the planned FFT length.
    #[must_use]
    pub fn with_dft_len(coefficients: FirCoefficients, dft_len: usize) -> Self {
        let overlap = coefficients.len().saturating_sub(1);
        let dft_len = dft_len.max(overlap + 1).next_power_of_two();
        let advance = dft_len - overlap;
        let mut planner = RealFftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(dft_len);
        let inverse = planner.plan_fft_inverse(dft_len);
        let mut kernel = vec![0.0; dft_len];
        kernel[..coefficients.len()].copy_from_slice(coefficients.as_f32_slice());
        let mut kernel_spectrum = forward.make_output_vec();
        forward
            .process(&mut kernel, &mut kernel_spectrum)
            .expect("real FFT kernel length matches plan");

        Self {
            coefficients,
            dft_len,
            overlap,
            advance,
            kernel_spectrum,
            forward,
            inverse,
        }
    }

    /// Returns the FFT length used for each overlap-save block.
    #[must_use]
    pub const fn dft_len(&self) -> usize {
        self.dft_len
    }

    /// Processes a complete mono stream into an equally sized output slice.
    ///
    /// # Panics
    ///
    /// Panics when `output.len() != input.len()`.
    pub fn process_into(&self, input: &[f32], output: &mut [f32]) {
        assert_eq!(input.len(), output.len());
        if self.coefficients.is_empty() {
            output.copy_from_slice(input);
            return;
        }
        if input.is_empty() {
            return;
        }

        let shift = self.coefficients.len().saturating_sub(1) / 2;
        let total_virtual_input = input.len() + shift;
        let mut history = vec![0.0; self.overlap];
        let mut block = vec![0.0; self.dft_len];
        let mut spectrum = self.forward.make_output_vec();
        let mut convolved = self.inverse.make_output_vec();
        #[allow(
            clippy::cast_precision_loss,
            reason = "FFT lengths are small power-of-two block sizes and are exactly representable for practical FIR use."
        )]
        let scale = 1.0 / self.dft_len as f32;
        let mut consumed = 0;
        let mut skip = shift;
        let mut written = 0;

        while consumed < total_virtual_input && written < output.len() {
            if self.overlap != 0 {
                block[..self.overlap].copy_from_slice(&history);
            }
            block[self.overlap..].fill(0.0);

            let new_samples = self.advance.min(total_virtual_input - consumed);
            for index in 0..new_samples {
                let source_index = consumed + index;
                block[self.overlap + index] = input.get(source_index).copied().unwrap_or(0.0);
            }
            consumed += new_samples;

            self.forward
                .process(&mut block, &mut spectrum)
                .expect("real FFT block length matches plan");
            for (sample, kernel) in spectrum.iter_mut().zip(&self.kernel_spectrum) {
                *sample *= *kernel;
            }
            self.inverse
                .process(&mut spectrum, &mut convolved)
                .expect("real inverse FFT spectrum length matches plan");

            let valid = &convolved[self.overlap..self.overlap + new_samples];
            let valid_start = skip.min(valid.len());
            skip -= valid_start;
            for sample in &valid[valid_start..] {
                if written == output.len() {
                    break;
                }
                output[written] = *sample * scale;
                written += 1;
            }

            if self.overlap != 0 {
                history.copy_from_slice(&block[self.advance..self.advance + self.overlap]);
            }
        }

        debug_assert_eq!(written, output.len());
    }
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

fn dft_len_for_taps(taps: usize) -> usize {
    let mut len = 256_usize;
    let target = taps.saturating_mul(4).max(2);
    while len < target {
        len <<= 1;
    }
    len
}

#[cfg(test)]
mod tests {
    use super::{DftFir, FirCoefficients, FirError, FirState};

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

    #[test]
    fn dft_fir_matches_direct_centered_alignment() {
        let coefficients = FirCoefficients::new((0_u16..257).map(|index| {
            let phase = f64::from(index) / 17.0;
            phase.sin() * 0.001
        }))
        .unwrap();
        let input = (0_u16..2049)
            .map(|index| {
                let phase = f32::from(index) / 29.0;
                phase.sin() * 0.25 + phase.cos() * 0.125
            })
            .collect::<Vec<_>>();
        let mut direct = vec![0.0; input.len()];
        let mut dft = vec![0.0; input.len()];

        FirState::new(coefficients.clone()).process_into(&input, &mut direct);
        DftFir::with_dft_len(coefficients, 512).process_into(&input, &mut dft);

        assert_close(&dft, &direct, 0.000_02);
    }

    #[test]
    fn dft_fir_preserves_empty_coefficients_as_null_stream() {
        let coefficients = FirCoefficients::new([]).unwrap();
        let input = [0.0, 0.25, -0.5];
        let mut output = [0.0; 3];

        DftFir::new(coefficients).process_into(&input, &mut output);

        assert_eq!(output.map(f32::to_bits), input.map(f32::to_bits));
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

    fn assert_close(actual: &[f32], expected: &[f32], epsilon: f32) {
        assert_eq!(actual.len(), expected.len());
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (*actual - *expected).abs() <= epsilon,
                "sample {index}: actual {actual}, expected {expected}"
            );
        }
    }
}
