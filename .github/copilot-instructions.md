# BetterFS: Master Architectural Specification & RFC

> **CRITICAL AGENT INSTRUCTION:**
> NEVER use terminal utilities (e.g., `grep`, `cat`, `sed`, `awk`) to read, search, or edit files. YOU MUST strictly use the native API tools provided to you (e.g., `read_file`, `replace_string_in_file`, `create_file`, `grep_search`).

## 1. System Topology & Layering
BetterFS is a user-space filesystem operating via FUSE. The architecture is strictly divided into four isolated layers. High-level layers may call low-level layers, but NEVER vice versa.
1. **FUSE Interface Layer (`fuse_handler.rs`)**: Translates kernel VFS calls into BetterFS operations. Handles path-to-inode resolution and POSIX permission checks.
2. **Page Cache Layer (In-Memory)**: A write-back, LRU-evicted memory buffer. Absorbs 4KB random writes and coalesces them into sequential, chunk-aligned operations.
3. **Metadata Engine (`file_manager.rs`)**: Manages the Directed Acyclic Graph (DAG) of the filesystem namespace using an embedded KV store (`sled`). Strictly ACID compliant.
4. **CAS Storage Engine (`storage.rs`)**: The physical disk layer. Immutable, append-only, content-addressed blob storage.

## 2. On-Disk Data Layout (Physical Storage)
Data is decoupled from metadata. 
### 2.1 Content-Addressable Storage (CAS) Pool
- **Format**: Immutable Blobs.
- **Pathing**: Sharded by the first 2 hex characters of the SHA-256 hash to prevent directory entry limits (e.g., `cas/a1/b2c3d4...`).
- **Compression**: `zstd` (Level 3). Compression occurs *after* chunking, but *before* disk flush.

### 2.2 Relational Metadata Schema (`sled` embedded KV)
- **Namespaces**: Prefix-isolated.
- `meta:{inode_id}` $\rightarrow$ Binary representation of `Inode` (Owner, Permissions, Size, Parent Inode ID, Name).
- `recipe:{inode_id}` $\rightarrow$ Ordered Array of SHA-256 Hash Strings.
- `dirent:{parent_inode_id}:{child_name}` $\rightarrow$ `child_inode_id` (8-byte Little Endian).
- `refcount:{chunk_hash}` $\rightarrow$ `u64` count of inodes referencing this chunk (used for offline Garbage Collection).

## 3. The Memory Hierarchy & I/O Pipeline
Direct disk I/O on every FUSE write is PROHIBITED. The system must implement a Write-Back Cache.

### 3.1 The Page Cache State Machine
- **Structure**: `HashMap<u64, (Vec<u8>, bool)>` mapping `inode_id` to `(buffer, is_dirty)`.
- **Read Path**: 
  1. Check Page Cache. If hit, return bytes. 
  2. If miss, fetch recipe from Metadata Engine, stream compressed chunks from CAS, decompress, populate Cache, return bytes.
- **Write Path**: 
  1. Mutate buffer in Page Cache.
  2. Mark `is_dirty = true`. 
  3. Update volatile `inode.size`. Return success to VFS.
- **Flush/Eviction Path** (Triggered on `release()`, `fsync()`, or memory pressure):
  1. If `is_dirty`, pass the buffer to the FastCDC Chunker.
  2. Hash new chunks, compress, and write to CAS.
  3. Overwrite `recipe:{inode_id}` in the Metadata Engine.
  4. Mark `is_dirty = false`.

## 4. Algorithmic Specifications
### 4.1 Fast Content-Defined Chunking (FastCDC)
Polynomial rolling hashes (Rabin) are computationally expensive. The system MUST implement FastCDC using a Gear Hash mechanism.
- **Complexity**: $O(1)$ per byte boundary check.
- **Parameters**: 
  - Minimum Chunk Size: 2 KB
  - Target (Average) Chunk Size: 4 KB
  - Maximum Chunk Size: 64 KB
- **Math Definition**: The hash state is updated via bitwise shifts and an array lookup: 
  `hash = (hash << 1).wrapping_add(GEAR_MATRIX[byte])`.
- **Cut Point**: A boundary is declared when `(hash & MASK) == 0`.

### 4.2 Snapshot & Copy-on-Write (Chronos)
Snapshots operate strictly via Metadata Pointer Duplication. Data chunks are never copied.
- **Operation**: A snapshot clones the `Inode` metadata and its `recipe:{inode_id}`.
- **Graph Structure**: Live files and snapshot files point to the same underlying chunk hashes.
- **CoW Execution**: Because the CAS layer is append-only and immutable, modifying a live file simply generates new chunks, updates the live file's recipe, and leaves the snapshot's recipe pointing to the old chunks.

## 5. Concurrency & Lock Hierarchy
To prevent deadlocks under heavy multithreaded FUSE workloads, locks MUST be acquired in this strict total order. Never acquire a lower-order lock while holding a higher-order lock.
1. **Global `inode_registry` (RwLock)**: Protects the lookup table.
2. **Per-Inode `RwLock`**: Protects the specific file's attributes and memory state.
3. **Global `page_cache` (RwLock)**: Protects the volatile write buffers.

**Pattern**: `let cloned_arc = registry.read().get(id).clone();` $\rightarrow$ Drop Registry Lock $\rightarrow$ `let mut node = cloned_arc.write();`

## 6. Failure Domains & ACID Guarantees
- **Crash Consistency**: The `sled` database guarantees atomic flushes. If power is lost before `release()` completes, the `dirty` page cache is lost, but the underlying metadata and previous disk state remain perfectly consistent. Orphaned chunks in the CAS pool are ignored until the offline Garbage Collector runs.
- **Orphan Resolution**: At startup (`hydrate_tree`), the system relies exclusively on the `dirent:{parent}:{name}` edges to reconstruct the namespace graph. Unlinked `meta:{inode}` entries without a parent `dirent` are swept.

## Phase: Benchmarking & Performance Validation (Master Suite)

### 1. Environment & Safety Constraints (Strict)
* **The Arena Location:** NEVER use `/tmp` for loopback devices or heavy I/O testing. It mounts as a memory-capped `tmpfs` and will cause an Out-Of-Memory (OOM) crash. All benchmarks MUST be executed in a persistent directory on the SSD (e.g., `~/benchmark_arena`).
* **Cache Bypassing:** All `fio` scripts MUST include `direct=1` to bypass the Linux VFS page cache. We must measure true filesystem I/O, not RAM speed.
* **The Comparables:** The control baseline is `ext4` (measures raw throughput and speed limit). The direct architectural competitor is `btrfs` (measures CoW and snapshot latency).

### 2. The Loopback Arena Setup
Establish 5GB loopback devices to ensure complete system isolation:
1.  **Allocate:** `dd if=/dev/zero of=<fs>_disk.img bs=1M count=5120`
2.  **Format:** `mkfs.ext4` and `mkfs.btrfs`
3.  **Mount:** `mount -o loop <image> <mount_dir>`
4.  **Permissions:** `chown -R $USER:$USER <mount_dir>`

### 3. The `fio` Micro-Benchmark Matrix
Generate and execute these specific `.fio` configurations. Export results using `--output-format=json`.
* **`seq_write.fio` (Throughput Penalty):** `rw=write`, `bs=1M`, `size=2G`. 
    * *Goal:* Measure the maximum bandwidth (`bw`) penalty introduced by the CDC/Hashing/Compression pipeline against Ext4.
* **`rand_write.fio` (Metadata IOPS):** `rw=randwrite`, `bs=4K`, `size=1G`, `iodepth=32`. 
    * *Goal:* Stress the `sled` database transaction overhead. Extract mean completion latency (`clat_ns`).
* **`realistic_mix.fio` (Bimodal Workload):** `rw=randrw`, `rwmixread=70`, `bssplit=4k/50:64k/30:1m/20`. 
    * *Goal:* Test dynamic chunking adaptation under a realistic 70/30 read/write split.
* **`massive_stream.fio` (LRU Cache Test):** `rw=write`, `size=4G`, `buffer_compress_percentage=0`, `refill_buffers=1`.
    * *Goal:* Ingest 4GB of incompressible data to test the `VecDeque` LRU eviction pipeline under sustained memory pressure without OOMing.
* **`paranoid_db.fio` (Strict ACID/fsync):** `rw=randwrite`, `bs=4k`, `size=500M`, `fsync=1`.
    * *Goal:* Force a physical disk flush on every single write to prove the filesystem handles synchronous safety constraints without deadlocking.

### 4. Advanced Profiling & Degradation
* **Resource Profiling (The FUSE Tax):** During the `massive_stream` and `realistic_mix` tests on ArcFS, monitor the daemon's CPU and RAM footprint using: `pidstat -r -u -p $(pgrep better-fs) 1`. Record the P99 RAM usage to prove the LRU cache holds steady.
* **The Aged Filesystem Test:** Before running the final `realistic_mix`, fill the drive to 85% capacity with duplicated files, then randomly delete half of them. This fragments the Sled DB and CAS directory, proving lookup latency doesn't degrade logarithmically over time.

### 5. The Macro-Benchmark (Linux Kernel Extraction)
1.  **The Payload:** Download a recent Linux Kernel source tarball (`.tar.xz`).
2.  **Metadata Stress (Write):** Time the extraction (`time tar -xf...`) to measure the speed of rapid inode/dirent creation (approx. 80,000 tiny files).
3.  **Traversal Speed (Read):** Drop OS caches (`echo 3 | sudo tee /proc/sys/vm/drop_caches`), then time a full tree traversal (`time find <dir> -type f | wc -l`) to measure metadata read latency.

### 6. Snapshot Latency & Giant File Deduplication
* **CoW Latency:** Time `btrfs subvolume snapshot` against the ArcFS snapshot trigger (`time mkdir .snap_v1`) under active I/O load.
* **The VM Clone Test:** Copy a real 3GB+ `.iso` or `.qcow2` image into the drive. Verify near-instant completion time and use `du -sh` on the backend directory to prove zero additional physical space was consumed.