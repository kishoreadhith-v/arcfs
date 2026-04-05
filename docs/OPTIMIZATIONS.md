# ArcFS Performance & Architectural Optimizations

This document serves as a historical and architectural ledger of the performance optimizations made to ArcFS (formerly ArcFS). It details the evolution from initial naive implementations to production-grade, highly parallelized, and memory-efficient algorithms.

## 1. FUSE Page Cache: O(1) In-Place Memory Mutations
**The Bottleneck:** 
During the initial FUSE implementation, both `read` and `write` kernel requests were processed using a monolithic cache retrieval function (`read_cached_or_load`). If a user wrote 2GB of data in 1MB FUSE chunks, the daemon explicitly cloned the entire file buffer on *every single 1MB write* (`file_data.clone()`). This resulted in a catastrophic $\mathcal{O}(N^2)$ memory bandwidth bottleneck, locking up the daemon when processing large files.

**The Optimization:** 
The `read_cached_or_load` cloning pattern was completely removed. ArcFS now uses granular, in-place data mutations:
- **Write Path:** Acquires an `RwLock` write-guard over the `page_cache` HashMap, retrieves the mutable reference to the buffer, calls `resize` on the vector locally, and uses `copy_from_slice` for the incoming chunk. The lock is held for microseconds, and zero bytes are cloned.
- **Read Path:** Acquires a read-guard over the `page_cache` and serves the data slice pointer `&data[start..end]` directly back to the FUSE reply handler.
- **Truncate/SetAttr:** File truncations now interact directly with the locked buffer's `.resize()` method instead of cloning the vector to shrink it.

## 2. Deduplication Algorithm: FastCDC over Rabin-Karp
**The Bottleneck:**
Early implementations of Content-Defined Chunking (CDC) for the deduplication engine typically rely on Rabin-Karp rolling hashes to find chunk boundaries. However, Rabin-Karp involves polynomial arithmetic at every single byte step, resulting in prohibitive CPU overhead which restricts disk throughput.

**The Optimization:**
ArcFS implements **Fast Content-Defined Chunking (FastCDC)** utilizing a Gear Hash matrix. 
- Instead of polynomial math, Gear Hashing calculates rolling boundaries strictly via array lookups and bitwise shifts: `hash = (hash << 1).wrapping_add(GEAR_MATRIX[byte])`.
- Boundary testing checks `(hash & MASK) == 0`.
- This ensures byte-boundary checking operates in true $\mathcal{O}(1)$ CPU instructions per byte, massively multiplying chunking throughput and keeping average chunk sizes dynamically pinned to 4KB limits.

## 3. Storage Pipeline: Zstd Dictionary Compression on Chunk Level
**The Bottleneck:** 
Traditional filesystems compress whole files, which entirely ruins Content Addressable Storage (CAS) deduplication potential because changing 1 byte cascades the compression tree and alters the file's final hash.

**The Optimization:**
Compression is strictly performed *after* chunking but *before* disk flushing. By compressing the chunk pools independently via `zstd` (Level 3), the filesystem keeps memory and CPU footprints low, while maximizing deduplication block hits. Identical data across an `.iso` clone will always chunk and hash exactly the same way before compression logic touches the data string.

## 4. Concurrent Threading: Strict Total Lock Ordering
**The Bottleneck:**
Highly parallelized IO workloads (such as `fio` random write benchmarks) over a FUSE boundary quickly generate thread deadlocks if the metadata graph (Sled KV) and memory page caches attempt to synchronize state dynamically.

**The Optimization:**
A strict lock hierarchy is uniformly enforced to guarantee deadlock freedom:
1. **Global Inode Registry** (`RwLock`): Protects the table topology.
2. **Per-Inode State** (`RwLock`): Protects file size, block counts, and attributes.
3. **Global Page Cache** (`RwLock`): Protects the physical volatile buffers.
Under this protocol, higher-order locks are always safely dropped before lower-order locks are awaited.

## 5. Artifact Compilation: Rust LLVM Optimizations
**The Bottleneck:**
Performing byte-shifting deduplication and structural memory manipulation via unoptimized debug artifacts (`cargo run`) stalls out heavily due to Rust's bounds checking and lack of register optimizations.

**The Optimization:**
All benchmarks and production runtime deployments strictly mandate `--release` compilation profiles, leveraging LLVM's auto-vectorization and loop unrolling to push FastCDC throughput past standard gigabit NVMe limits.
