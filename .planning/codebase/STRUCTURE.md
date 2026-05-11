# STRUCTURE
> Generated: 2026-05-07 | Focus: arch | Project: arcfs

## Summary
ArcFS is organized as a single Cargo workspace with a flat `src/` module structure. Source files map 1:1 to architectural layers. Tests are separated by tier into `tests/` (integration) and inline `#[cfg(test)]` blocks (unit). Benchmark infrastructure lives in `benchmarks/` with its own shell scripts and Python chart generators.

## Directory Layout

```
arcfs/
├── src/
│   ├── lib.rs              # Re-exports: chunker, file_manager, fuse_handler, storage
│   ├── main.rs             # CLI entry point (clap); all subcommand wiring
│   ├── chunker.rs          # Gear-hash CDC chunker (FastCDC)
│   ├── storage.rs          # SHA256 CAS + zstd content store
│   ├── file_manager.rs     # sled DB orchestration + tag CRUD
│   ├── fuse_handler.rs     # FUSE Filesystem impl (ArcFS struct)
│   └── fuse_handler.rs.backup  # (backup artifact, not compiled)
│
├── tests/                  # Rust integration tests
│   ├── backend_stress.rs   # Concurrent writes, dedup, chunk boundaries
│   ├── gc_test.rs          # Garbage collection correctness
│   ├── tagfs_test.rs       # Tag CRUD and index consistency
│   ├── regression_e2e.sh   # Full FUSE mount lifecycle E2E
│   ├── verify_single_backing.sh  # Deduplication invariant check
│   ├── architecture_compliance.sh  # Source invariant grep checks
│   └── report_local_e2e_status.sh
│
├── benchmarks/
│   ├── setup_arena.sh      # One-time loopback arena setup
│   ├── run_benchmarks.sh   # Full matrix: responsive/durable/worst_case × 5 jobs
│   ├── run_arcfs_benchmarks.sh
│   ├── run_bindfs_benchmarks.sh
│   ├── run_disk_usage_benchmarks.sh
│   ├── run_integrity_suite.sh
│   ├── validate_results.sh
│   ├── validate_integrity_results.sh
│   ├── generate_charts.py  # fio result → chart rendering
│   ├── generate_disk_usage_charts.py
│   ├── fio_jobs/           # fio job definition files
│   ├── results/            # Benchmark result outputs
│   ├── charts/             # Generated chart images
│   ├── dashboards/         # Dashboard configs
│   ├── INTEGRITY_SUITE.md
│   └── WORST_CASE_SUITE.md
│
├── research_paper/         # IEEE-format LaTeX research paper
├── Cargo.toml              # Single crate, edition 2024
├── Cargo.lock
├── CLAUDE.md               # Claude Code project instructions
└── .planning/              # GSD planning artifacts
    └── codebase/           # This codebase map
```

## Module Boundaries

### `src/lib.rs`
Thin re-export module. Exposes `chunker`, `file_manager`, `fuse_handler`, `storage` as public modules. The binary (`main.rs`) uses `arcfs::` paths; integration tests reference `arcfs::file_manager::FileManager` etc.

### `src/chunker.rs` (136 lines)
Self-contained. No dependencies on other arcfs modules. Takes `&[u8]` input, produces chunk boundary slices. Stateful — must call `reset()` between files. Exports `chunk_lengths()` for CDC stats CLI command.

### `src/storage.rs` (180 lines)
Depends only on `std` + `sha2`/`hex`/`zstd`. Knows nothing about inodes or FUSE. Single responsibility: hash → compress → write to CAS path, or read → decompress → return bytes.

### `src/file_manager.rs` (857 lines)
The coordination layer. Depends on `chunker` and `storage`. Owns the sled DB handle. All sled read/write operations live here. Exports the key types used by `fuse_handler`: `FileManager`, `FileRecipe`, `InodeMetadata`, `FileKind`, `Dirent`, `SnapshotMetadata`.

### `src/fuse_handler.rs` (2137 lines)
The largest module. Depends on `file_manager`. Implements the `fuser::Filesystem` trait on `ArcFS`. All FUSE operations (`lookup`, `getattr`, `read`, `write`, `create`, `unlink`, `readdir`, `fsync`, `setxattr`, etc.) live here. Virtual inode allocation for TagFS happens here.

### `src/main.rs` (294 lines)
CLI only. Constructs `FileManager`, then either mounts (`ArcFS::new(manager)` + `fuser::mount2`) or dispatches to CLI file operations. Contains no business logic — purely wiring.

## File Size Distribution

| File | Lines | Role |
|---|---|---|
| `fuse_handler.rs` | 2137 | Largest — all FUSE ops + TagFS + snapshot logic |
| `file_manager.rs` | 857 | DB orchestration + tag CRUD |
| `main.rs` | 294 | CLI entry + subcommand dispatch |
| `storage.rs` | 180 | CAS engine |
| `chunker.rs` | 136 | CDC chunker |
| `lib.rs` | 4 | Re-exports only |

## Runtime Storage Layout

At runtime, `--storage-dir` (default `./my_storage`) contains:

```
my_storage/
├── <sled DB files>     # Metadata: inodes, dirents, recipes, tags, snapshots
└── cas/
    ├── 00/             # Chunk files: cas/<hash[0:2]>/<hash[2:]>
    ├── 1a/
    ├── ff/
    └── ...
```

## Observations
- `fuse_handler.rs` at 2137 lines is a candidate for decomposition (TagFS layer and snapshot logic could be separate modules)
- The `fuse_handler.rs.backup` artifact in `src/` should be removed — it is ignored by the compiler but adds noise
- No `config.rs` or settings layer — all configuration is via CLI flags at mount time
- `research_paper/` is a documentation artifact, not part of the build graph
