# BetterFS: Master Architectural Specification & RFC

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