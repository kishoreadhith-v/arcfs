// src/chunker.rs

const MIN_CHUNK_SIZE: usize = 2 * 1024;
const TARGET_CHUNK_SIZE: usize = 4 * 1024;
const MAX_CHUNK_SIZE: usize = 64 * 1024;
const MASK: u64 = (TARGET_CHUNK_SIZE as u64) - 1;

const GEAR_MATRIX: [u64; 200] = [
    0x6d81f2a5b4c3e291, 0x2c7f9a4e1d6b3f50, 0x9b3e6d1f5a2c8e47, 0x4f1a8c3d7e2b6d95,
    0x8a2d5f7c1e4b9d63, 0x3e7b1c9a5d2f4e86, 0x5c2a8e1d7f3b6d49, 0x1f9e4b2c6d8a3e75,
    0x7d3c1a9e5b2f6d84, 0x2e8a5d1c7f4b9e63, 0x9c4b2f6d1a8e3d75, 0x4a7e1d5c9b2f6a83,
    0x8e1c7b3d5f2a9d64, 0x3d9a4e1f6b2c8d75, 0x5b2f8c1d7e4a9d63, 0x1c6e3a9f5d2b8d74,
    0x7a3d1f8c5e2b9d64, 0x2f9b4d1a6e3c8d75, 0x9d2e6c1f5a4b8d73, 0x4c8a1d7e3f2b6d95,
    0x8b3f6d1c5a2e9d74, 0x3a7c1e9d5f2b8d64, 0x5d1f8b3c7e2a9d65, 0x1e6a4d9f2b5c8d73,
    0x7c2f1a8e5d3b9d64, 0x2a9d5c1f7e3b6d84, 0x9e4b1d6c2f8a3d75, 0x4d8c2f1a7e5b9d63,
    0x8c1e7b3d5a2f9d64, 0x3b9d4f1c6e2a8d75, 0x5e2a8c1d7f3b9d64, 0x1d7b3e9a5c2f8d64,
    0x76e2b9a5d1c3f84a, 0x2bd91f6e4a8c3d75, 0x98af3d2c6e1b7d54, 0x4ca76e1d9b2f3d85,
    0x8f2c5d1a7e3b9d46, 0x3c8e1a9d5f2b6d74, 0x5a1d7e3c9b2f8d64, 0x1b6f4d9a2c5e8d73,
    0x7e3a1c8f5d2b9d64, 0x2c9f5d1a7e4b8d63, 0x9f4c2e6d1a8b3d75, 0x4e8b1d7c3f2a6d95,
    0x8d1f7c3b5a2e9d64, 0x3f9a4c1e6b2d8d75, 0x5f2b8d1c7e4a9d63, 0x1a6e3c9f5d2b8d74,
    0x79d3f8a5c1e2b64d, 0x2d8a5f1c7e3b9d64, 0x9a5e2d1f6c4b8d73, 0x4b9c1e7d3f2a6d85,
    0x8a1d6f3c5e2b9d74, 0x3e8b4c1f7a2d9d63, 0x5c1f9a3d7e2b8d64, 0x1f7c3e9b5a2d8d64,
    0x7b2e1d8f5c3a9d64, 0x2e9c5a1f7d3b6d84, 0x9c5a2f1d6e4b8d73, 0x4f8d2c1a7e3b9d64,
    0x8f1a7d3c5b2e9d64, 0x3a9f4d1c6e2b8d75, 0x5d2c8f1a7e3b9d64, 0x1c7d3f9a5e2b8d64,
    0x75a9e3d1c6f2b84d, 0x2f8b5c1d7e4a9d63, 0x99d4e2a1c6b3f85d, 0x4a8f1d7e3c2b6d95,
    0x8e2b6d1f5a3c9d74, 0x3d8c1f9a5e2b6d74, 0x5b1e7d3c9a2f8d64, 0x1e6d4c9f2a5b8d73,
    0x7d3a2e8f5b1c9d64, 0x2a8f5d1c7e3b9d64, 0x9e5b2d1f6c4a8d73, 0x4d9a1e7c3f2b6d84,
    0x8b1f6c3d5e2a9d74, 0x3c9d4a1f7e2b8d64, 0x5e1b8d3c7a2f9d64, 0x1d7e3a9f5c2b8d74,
    0x78c2f9a5d1e3b64d, 0x2c7e5a1f9d3b6d84, 0x9d6b2e1f5a4c8d73, 0x4c9e1a7d3f2b6d95,
    0x8c2a7d1f5e3b9d64, 0x3b8f4d1c6e2a9d74, 0x5f1c9e3d7a2b8d64, 0x1a7f3d9c5e2b8d74,
    0x77d3a8e5c1f2b64d, 0x2d9c5e1a7f3b6d84, 0x9b6e2d1f5a4c8d73, 0x4e9a1c7d3f2b6d95,
    0x8d2b7f1c5e3a9d64, 0x3e9c4b1d6f2a8d74, 0x5a1f8d3e7c2b9d64, 0x1f6d3a9c5e2b8d74,
    0x74b9e2d5c1a3f86d, 0x2b8e5d1f7c3a9d64, 0x98d5e2b1c6a3f74d, 0x4b8d1f7a3e2c6d95,
    0x8a2e6f1d5c3b9d74, 0x3d7f4c1e9a2b8d64, 0x5c1e9f3a7d2b8d64, 0x1e7a3d9f5c2b8d74,
    0x7a2c1f8d5e3b9d64, 0x2f8d5b1c7e3a9d64, 0x9c5e2a1f6d4b8d73, 0x4f9b1d7c3e2a6d84,
    0x8f1c6d3a5e2b9d74, 0x3a8e4d1f7c2b9d64, 0x5d1b9f3c7e2a8d64, 0x1c7f3a9d5e2b8d74,
    0x79b2e8d5c1f3a64d, 0x2e9d5a1c7f3b6d84, 0x9d4e2b1f6a3c8d73, 0x4c8f1e7d3a2b6d95,
    0x8c1f7a3d5e2b9d64, 0x3b9e4c1d6f2a8d74, 0x5e1a8f3d7c2b9d64, 0x1d6f3b9a5e2c8d74,
    0x76c3f9a5d1e2b84d, 0x2c8f5d1a7e3b9d64, 0x9a6d2e1f5b4c8d73, 0x4d8e1b7c3f2a6d95,
    0x8b2a7e1d5f3c9d64, 0x3e8d4b1f6a2c9d74, 0x5b1f9c3d7e2a8d64, 0x1f7b3d9e5c2a8d74,
    0x7b2d1e8f5a3c9d64, 0x2a9e5c1d7f3b6d84, 0x9e5c2b1f6a4d8d73, 0x4e9d1a7c3f2b6d95,
    0x8d1e6f3b5a2c9d74, 0x3c9f4a1d7e2b8d64, 0x5f1a8e3d7c2b9d64, 0x1a7d3f9c5e2b8d74,
    0x75c2e9d5a1f3b84d, 0x2d8e5a1f7c3b9d64, 0x99e4d2b1f6a3c85d, 0x4a8c1f7d3e2b6d95,
    0x8e2d6b1f5c3a9d74, 0x3d7e4f1c9a2b8d64, 0x5c1a9e3f7d2b8d64, 0x1e6f3a9d5c2b8d74,
    0x78b3e8d5c1f2a64d, 0x2f9d5c1a7e3b6d84, 0x9b5e2a1f6d4c8d73, 0x4c9d1f7a3e2b6d95,
    0x8a1f6e3d5c2b9d74, 0x3f8c4d1a7e2b9d64, 0x5d1a8f3e7c2b9d64, 0x1c7e3d9a5f2b8d74,
    0x77c2f8e5d1a3b64d, 0x2c9e5b1d7f3a6d84, 0x9d6a2e1f5c4b8d73, 0x4d8b1f7c3e2a6d95,
    0x8b2c7d1f5a3e9d74, 0x3e9f4b1a6d2c8d64, 0x5a1d8e3f7c2b9d64, 0x1f6b3d9a5e2c8d74,
    0x74a3e9d5c1f2b84d, 0x2b8f5c1a7e3d9d64, 0x98e5d2b1f6c3a74d, 0x4b9a1f7d3c2e6d95,
    0x8c2e6a1f5d3b9d74, 0x3d8f4c1a7e2b9d64, 0x5e1b9a3f7d2c8d64, 0x1d7a3e9f5c2b8d74,
    0x7a3c1d8e5f2b9d64, 0x2e8d5a1b7f3c6d84, 0x9f5b2d1a6e4c8d73, 0x4e8c1f7b3d2a6d95,
    0x8f1d6b3e5a2c9d74, 0x3b9f4d1a7c2e8d64, 0x5f1c8a3d7e2b9d64, 0x1a6d3f9e5c2b8d74,
    0x79a2e8f5d1c3b64d, 0x2d9f5b1a7e3c6d84, 0x9a4d2e1f6b3c8d73, 0x4c8b1f7e3d2a6d95,
    0x8d2a6c1f5e3b9d74, 0x3c8f4e1a7d2b9d64, 0x5b1f9d3a7e2c8d64, 0x1f7c3a9e5d2b8d74,
    0x76a3f8d5c1e2b64d, 0x2a9f5c1b7e3d6d84, 0x9e6b2d1a5f4c8d73, 0x4d9c1f7a3b2e6d95,
    0x8c1b6f3d5e2a9d74, 0x3f9e4a1c7d2b8d64, 0x5d1c8f3a7e2b9d64, 0x1c6e3d9f5a2b8d74,
    0x75b2e9f5d1a3c64d, 0x2e8f5d1b7c3a9d64, 0x99f4e2a1d6b3c85d, 0x4a8e1f7d3b2c6d95,
    0x8e2c6b1d5f3a9d74, 0x3d9f4a1b7e2c8d64, 0x5c1b8e3f7d2a9d64, 0x1e7d3a9c5f2b8d74,
];

pub struct Chunker {
    hash: u64,
}

impl Chunker {
    pub fn new() -> Self {
        Self { hash: 0 }
    }

    pub fn feed_byte(&mut self, new_byte: u8) {
        let idx = (new_byte as usize) % GEAR_MATRIX.len();
        self.hash = (self.hash << 1).wrapping_add(GEAR_MATRIX[idx]);
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
