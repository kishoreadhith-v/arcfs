// src/chunker.rs

const MIN_CHUNK_SIZE: usize = 2 * 1024;
const TARGET_CHUNK_SIZE: usize = 4 * 1024;
const MAX_CHUNK_SIZE: usize = 64 * 1024;
const MASK: u64 = (TARGET_CHUNK_SIZE as u64) - 1;

const fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

const fn build_gear_matrix() -> [u64; 256] {
    let mut out = [0u64; 256];
    let mut i = 0;
    let mut seed = 0x1234_5678_9ABC_DEF0u64;

    while i < 256 {
        seed = splitmix64(seed ^ (i as u64).wrapping_mul(0x9E3779B97F4A7C15));
        out[i] = seed;
        i += 1;
    }

    out
}

const GEAR_MATRIX: [u64; 256] = build_gear_matrix();

pub struct Chunker {
    hash: u64,
}

impl Chunker {
    pub fn new() -> Self {
        Self { hash: 0 }
    }

    pub fn feed_byte(&mut self, new_byte: u8) {
        self.hash = (self.hash << 1).wrapping_add(GEAR_MATRIX[new_byte as usize]);
    }

    pub fn should_cut(&self, current_chunk_size: usize) -> bool {
        if current_chunk_size < MIN_CHUNK_SIZE {
            return false;
        }

        if current_chunk_size >= MAX_CHUNK_SIZE {
            return true;
        }

        (self.hash & MASK) == 0
    }

    pub fn reset(&mut self) {
        self.hash = 0;
    }
}

pub fn chunk_lengths(data: &[u8]) -> Vec<usize> {
    let mut chunker = Chunker::new();
    let mut sizes = Vec::new();
    let mut current_size = 0usize;

    for &byte in data {
        chunker.feed_byte(byte);
        current_size += 1;

        if chunker.should_cut(current_size) {
            sizes.push(current_size);
            chunker.reset();
            current_size = 0;
        }
    }

    if current_size > 0 {
        sizes.push(current_size);
    }

    sizes
}

// UNIT TEST: Run this with 'cargo test'
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking_consistency() {
        let mut chunker = Chunker::new();

        // Create more random-looking synthetic data
        // Using a simple LCG (Linear Congruential Generator) pattern
        let data: Vec<u8> = (0u32..100_000)
            .map(|i| {
                let x = i.wrapping_mul(1103515245).wrapping_add(12345);
                ((x / 65536) % 256) as u8
            })
            .collect();

        let mut cut_points = Vec::new();
        let mut current_chunk_size = 0usize;

        for (i, &byte) in data.iter().enumerate() {
            chunker.feed_byte(byte);
            current_chunk_size += 1;

            if chunker.should_cut(current_chunk_size) {
                cut_points.push(i);
                chunker.reset();
                current_chunk_size = 0;
            }
        }

        println!(
            "Found {} chunks at positions: {:?}",
            cut_points.len(),
            &cut_points[..cut_points.len().min(10)]
        );
        println!(
            "Average chunk size: ~{} bytes",
            if cut_points.len() > 0 {
                100_000 / cut_points.len()
            } else {
                0
            }
        );

        assert!(
            cut_points.len() > 5,
            "Statistically unlikely to have fewer than 5 chunks in 100KB data"
        );
    }
}
