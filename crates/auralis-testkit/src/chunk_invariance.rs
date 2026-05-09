//! Shared L5 chunk-invariance schedules.
//!
//! These helpers keep streaming tests on the same required chunk-size matrix
//! and make the seeded random schedule reproducible across effects.

use std::fmt;

/// Required fixed chunk sizes from the README L5 test contract.
pub const REQUIRED_CHUNK_SIZES: [usize; 12] = [1, 2, 7, 15, 16, 17, 31, 32, 33, 64, 255, 1024];

/// Seed used by the default deterministic random chunk schedule.
pub const DEFAULT_RANDOM_CHUNK_SEED: u64 = 0xA4A1_15C5_7EED_0005;

/// A deterministic chunk schedule used for L5 chunk-invariance tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkSchedule {
    /// Repeats one fixed non-zero chunk size until the input is consumed.
    Fixed {
        /// Fixed chunk length in samples or frames, depending on the caller.
        chunk_size: usize,
    },
    /// Uses a checked-in pseudo-random chunk-size sequence.
    SeededRandom {
        /// Seed used to generate the sequence.
        seed: u64,
        /// Chunk sizes generated from `seed`.
        chunk_sizes: Vec<usize>,
    },
}

impl ChunkSchedule {
    /// Creates a fixed-size schedule.
    ///
    /// # Panics
    ///
    /// Panics when `chunk_size` is zero. Empty chunks are injected by
    /// [`process_chunks_mut`] for every schedule.
    #[must_use]
    pub fn fixed(chunk_size: usize) -> Self {
        assert!(chunk_size > 0, "fixed chunk size must be non-zero");
        Self::Fixed { chunk_size }
    }

    /// Creates a deterministic pseudo-random chunk schedule for `total_len`.
    #[must_use]
    pub fn seeded_random(total_len: usize, seed: u64) -> Self {
        Self::SeededRandom {
            seed,
            chunk_sizes: seeded_random_chunk_sizes(total_len, seed),
        }
    }

    /// Returns the seed when this is a seeded random schedule.
    #[must_use]
    pub const fn seed(&self) -> Option<u64> {
        match self {
            Self::Fixed { .. } => None,
            Self::SeededRandom { seed, .. } => Some(*seed),
        }
    }
}

impl fmt::Display for ChunkSchedule {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixed { chunk_size } => write!(formatter, "fixed chunk size {chunk_size}"),
            Self::SeededRandom { seed, .. } => {
                write!(formatter, "seeded random chunks seed=0x{seed:016x}")
            }
        }
    }
}

/// Returns the complete L5 schedule matrix for an input of `total_len`.
#[must_use]
pub fn l5_chunk_schedules(total_len: usize) -> Vec<ChunkSchedule> {
    let mut schedules = REQUIRED_CHUNK_SIZES
        .into_iter()
        .map(ChunkSchedule::fixed)
        .collect::<Vec<_>>();
    schedules.push(ChunkSchedule::seeded_random(
        total_len,
        DEFAULT_RANDOM_CHUNK_SEED,
    ));
    schedules
}

/// Processes `samples` according to `schedule`.
///
/// The callback receives each chunk plus its starting offset in the original
/// slice. Empty chunks are passed before the first real chunk, between real
/// chunks, and once more after all samples have been consumed so callers also
/// exercise empty input and final-flush behavior.
pub fn process_chunks_mut<F>(samples: &mut [f32], schedule: &ChunkSchedule, mut process: F)
where
    F: FnMut(&mut [f32], usize),
{
    let mut offset = 0_usize;
    process_empty(&mut process, offset);

    match schedule {
        ChunkSchedule::Fixed { chunk_size } => {
            process_fixed_chunks(samples, *chunk_size, &mut offset, &mut process);
        }
        ChunkSchedule::SeededRandom { chunk_sizes, .. } => {
            process_scheduled_chunks(samples, chunk_sizes, &mut offset, &mut process);
        }
    }

    process_empty(&mut process, offset);
}

fn process_fixed_chunks<F>(
    samples: &mut [f32],
    chunk_size: usize,
    offset: &mut usize,
    process: &mut F,
) where
    F: FnMut(&mut [f32], usize),
{
    let mut remaining = samples;
    while !remaining.is_empty() {
        let size = chunk_size.min(remaining.len());
        let (chunk, rest) = remaining.split_at_mut(size);
        process(chunk, *offset);
        *offset += chunk.len();
        process_empty(process, *offset);
        remaining = rest;
    }
}

fn process_scheduled_chunks<F>(
    samples: &mut [f32],
    chunk_sizes: &[usize],
    offset: &mut usize,
    process: &mut F,
) where
    F: FnMut(&mut [f32], usize),
{
    let mut remaining = samples;
    for &size in chunk_sizes {
        if remaining.is_empty() {
            break;
        }
        if size == 0 {
            process_empty(process, *offset);
            continue;
        }
        let size = size.min(remaining.len());
        let (chunk, rest) = remaining.split_at_mut(size);
        process(chunk, *offset);
        *offset += chunk.len();
        process_empty(process, *offset);
        remaining = rest;
    }
    if !remaining.is_empty() {
        process(remaining, *offset);
        *offset += remaining.len();
    }
}

fn process_empty<F>(process: &mut F, offset: usize)
where
    F: FnMut(&mut [f32], usize),
{
    let empty = &mut [];
    process(empty, offset);
}

fn seeded_random_chunk_sizes(total_len: usize, seed: u64) -> Vec<usize> {
    let mut state = seed;
    let mut remaining = total_len;
    let mut chunks = Vec::new();

    while remaining > 0 {
        state = next_xorshift64(state);
        let size = usize::try_from(state % 257).expect("modulo result fits usize");
        chunks.push(size.min(remaining));
        remaining = remaining.saturating_sub(size);
    }

    chunks.push(0);
    chunks
}

fn next_xorshift64(mut state: u64) -> u64 {
    if state == 0 {
        state = DEFAULT_RANDOM_CHUNK_SEED;
    }
    state ^= state << 13;
    state ^= state >> 7;
    state ^ (state << 17)
}

#[cfg(test)]
mod tests {
    use super::{
        ChunkSchedule, DEFAULT_RANDOM_CHUNK_SEED, REQUIRED_CHUNK_SIZES, l5_chunk_schedules,
        process_chunks_mut,
    };

    #[test]
    fn l5_matrix_contains_required_fixed_sizes_and_seeded_random() {
        let schedules = l5_chunk_schedules(128);

        for chunk_size in REQUIRED_CHUNK_SIZES {
            assert!(
                schedules.contains(&ChunkSchedule::fixed(chunk_size)),
                "missing fixed chunk size {chunk_size}"
            );
        }
        assert!(schedules.iter().any(|schedule| {
            matches!(
                schedule,
                ChunkSchedule::SeededRandom { seed, .. } if *seed == DEFAULT_RANDOM_CHUNK_SEED
            )
        }));
    }

    #[test]
    fn seeded_random_schedule_is_deterministic() {
        assert_eq!(
            ChunkSchedule::seeded_random(256, DEFAULT_RANDOM_CHUNK_SEED),
            ChunkSchedule::seeded_random(256, DEFAULT_RANDOM_CHUNK_SEED)
        );
    }

    #[test]
    fn process_chunks_injects_empty_chunks_and_final_flush() {
        let mut samples = [1.0, 2.0, 3.0];
        let mut calls = Vec::new();

        process_chunks_mut(&mut samples, &ChunkSchedule::fixed(2), |chunk, offset| {
            calls.push((offset, chunk.len()));
            for sample in chunk {
                *sample += 1.0;
            }
        });

        assert_eq!(
            samples.map(f32::to_bits),
            [2.0_f32.to_bits(), 3.0_f32.to_bits(), 4.0_f32.to_bits()]
        );
        assert_eq!(calls, [(0, 0), (0, 2), (2, 0), (2, 1), (3, 0), (3, 0)]);
    }
}
