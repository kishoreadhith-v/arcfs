# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

```bash
# Build (debug)
cargo build

# Build (release — required for benchmarks and performance testing)
cargo build --release

# Run (mounts at ./mnt; storage defaults to ./my_storage)
cargo run -- mount ./mnt

# CLI file operations (all share --storage-dir <path>, default: ./my_storage)
cargo run -- write <file_path>
cargo run -- read <file_name>
cargo run -- list
cargo run -- gc
cargo run -- inspect

# CDC chunk analysis
cargo run -- cdc-stats <file_path>

# Tag operations
cargo run -- tag-set-path <path> <tags...>
cargo run -- tag-query <tags...>
cargo run -- tag-next <partial_tags...>
```

## Testing

```bash
# Run all tests (multiple test DBs created in CWD — cleaned up per test)
cargo test

# Show println! output
cargo test -- --nocapture

# Run a specific integration test file
cargo test --test backend_stress
cargo test --test gc_test
cargo test --test tagfs_test

# Run a specific test by name
cargo test test_chunking_consistency -- --nocapture

# Unit tests only (lib modules)
cargo test --lib

# End-to-end regression suite (mounts ArcFS; requires fusermount)
bash tests/regression_e2e.sh

# Verify single-backing deduplication invariant end-to-end
bash tests/verify_single_backing.sh
```

Tests create temporary directories like `./test_db_N` in the working directory and clean them up at the start of each run (not end), so failures leave residue that the next run clears.

## Benchmarks

Benchmarks require a pre-configured loopback arena outside the repo (see `benchmarks/setup_arena.sh`). They use `fio` and compare ArcFS against ext4/btrfs reference mounts. Always run after `cargo build --release`.

```bash
# Full benchmark matrix (responsive/durable/worst_case × 5 jobs)
benchmarks/run_benchmarks.sh

# Single class override
CLASS=worst_case benchmarks/run_benchmarks.sh

# Integrity-only suite (CRC32c verify, separate from perf benchmarks)
benchmarks/run_integrity_suite.sh
INTEGRITY_PROFILE=fsync4k_verify benchmarks/run_integrity_suite.sh

# Validate results
benchmarks/validate_results.sh
benchmarks/validate_integrity_results.sh
```

## Architecture

ArcFS is a FUSE filesystem built on content-defined chunking and deduplication.

### Data Flow

```
File write → Chunker (Gear hash CDC) → Storage (SHA256 CAS + zstd compression)
                                      → FileManager (sled DB: inode/recipe/dirent metadata)
```

### Module Responsibilities

- **`chunker.rs`** — Gear-hash content-defined chunking (FastCDC). Boundaries at `(hash & MASK) == 0` with 2KB min, 4KB target, 64KB max chunk sizes. Call `reset()` between files.

- **`storage.rs`** — Content-addressed store: hashes raw bytes → SHA256, compresses with zstd (level 3), stores at `<storage_dir>/cas/<hash[0:2]>/<hash[2:]>`. Deduplication is implicit (skip write if path exists).

- **`file_manager.rs`** — Orchestrates chunking + storage. Persists `InodeMetadata`, `Dirent`, `FileRecipe`, `SnapshotMetadata`, and tag sets to a `sled` embedded database. Produces "recipes" (ordered list of chunk hashes + file size) for reconstruction. Also owns all tag CRUD operations.

- **`fuse_handler.rs`** — The FUSE `Filesystem` implementation. `ArcFS` holds:
  - `inode_registry: Arc<RwLock<HashMap<u64, Arc<RwLock<Inode>>>>>` — flat map of all live inodes
  - `page_cache: Arc<RwLock<HashMap<u64, (Vec<u8>, bool)>>>` — write-back cache; tuple is `(data, dirty_flag)`; flushed on `fsync`/`release`
  - `snapshots: Arc<RwLock<HashMap<String, Snapshot>>>` — named snapshot trees
  - `tag_virtual_dirs / tag_virtual_files` — ephemeral TagFS maps; never persisted
  - Maintains two inode namespaces:
    - **Real inodes** (id < 1,000,000): backed by `FileManager`/sled, reconstructed from DB at boot
    - **Virtual inodes** (id ≥ `VIRTUAL_INODE_START` = 1,000,000): ephemeral TagFS navigation, never persisted
  - Key reserved IDs: `1`=root, `2`=`.snapshots/`, `3`=snapshot-create sentinel, `4`=`@tags/`, `5`=`.tagfs_ctl` (at fs root)
  - `PAGE_CACHE_CAPACITY = 1024` — eviction kicks in when cache exceeds this many inodes; `touch_cache_entry()` is O(n) over the LRU `VecDeque`

- **`main.rs`** — CLI (`clap`) wiring. The global `--storage-dir` flag (default `./my_storage`) is shared by all subcommands.

### Sled DB Key Schema

All keys live in a single sled tree inside `<storage_dir>`:

| Prefix | Value |
|---|---|
| `ino_meta:<id>` | `InodeMetadata` (bincode) |
| `ino_recipe:<id>` | `FileRecipe` — ordered chunk hash list + file size |
| `dirent:<parent_id>:<name>` | child inode ID (u64) |
| `ino_tags:<inode_id>` | tag list for an inode |
| `tag_index:<tag>` | `HashSet<u64>` of inode IDs carrying this tag |
| `snapshot:<name>` | `SnapshotMetadata` |

Tags are lowercased and deduplicated before storage.

### Key Design Invariants

- The sled DB is the source of truth for persistence; the in-memory `inode_registry` is a cache rebuilt at startup.
- Chunk hashes are computed over **uncompressed** data; compression is a storage-layer concern invisible to upper layers.
- TagFS virtual directories are generated lazily from `FileManager::tag_*` DB queries and exist only in the running process's memory.
- Snapshots use structural sharing (Arc cloning of Inode trees) — modifying a snapshot-shared inode triggers CoW cloning.
- **Lock ordering must be strictly respected** to prevent deadlocks under concurrent `fio`/FUSE workloads:
  1. `inode_registry` (global registry `RwLock`)
  2. Per-inode `Inode` lock
  3. `page_cache` / `cache_lru`

  Always drop a higher-order lock before acquiring a lower-order one within the same call path.

### TagFS Control Interface

When ArcFS is mounted, `.tagfs_ctl` (at the fs root, inode 5) is a writable sentinel file for tag management without unmounting:

```bash
# Set tags on a live path
echo "set docs/report.txt work 2026" > mnt/.tagfs_ctl

# Delete all tags for a path
echo "del docs/report.txt" > mnt/.tagfs_ctl
```

Command format: `<verb> <relative-path> [tags...]`. The `@tags/` directory virtualizes tag-based navigation — `mnt/@tags/work/2026/` lists all inodes tagged with both `work` and `2026`.

### FUSE Threading Model

`fuser::mount2` dispatches all FUSE operations on a **single thread**. There is no thread pool. All benchmark IOPS figures reflect single-writer operation. Concurrent `fio` jobs serialize at the FUSE layer.

### Known Bug: Snapshot Deletion Orphans Chunks

`FileManager::delete_snapshot()` only removes the `snapshot:<name>` sled key. All `ino_recipe:` and `ino_meta:` entries cloned into the snapshot at creation time are left in the DB. GC's recipe-scan phase will find these recipes and mark their chunks reachable — so deleted-snapshot chunks are **never reclaimed by GC**.

### Development Artifacts

`src/chat.md` and `src/fuse_handler.rs.backup` are development scratch files — not source code. Do not parse or depend on them.
