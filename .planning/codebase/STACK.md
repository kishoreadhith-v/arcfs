# STACK
> Generated: 2026-05-07 | Focus: tech | Project: arcfs

## Summary

ArcFS is a pure-Rust FUSE filesystem implementing content-defined chunking (CDC) with SHA-256 content-addressed storage and zstd compression. The entire project is a single Rust binary (`arcfs`) driven by a `clap`-based CLI, persisting all metadata to an embedded `sled` key-value database and raw chunk data to the local filesystem.

## Languages

**Primary:**
- Rust — 2024 edition — all core filesystem logic (`src/`)

**Secondary:**
- Bash — benchmark harnesses, regression test runners, and arena setup scripts (`benchmarks/`, `tests/`)
- Python — chart generation for benchmark results (`benchmarks/generate_charts.py`, `benchmarks/generate_disk_usage_charts.py`)

## Runtime

**Environment:**
- Native binary; no VM or interpreter required at runtime
- Rust 1.89.0 (confirmed via `rustc --version`; project uses `edition = "2024"`)

**Package Manager:**
- Cargo 1.89.0
- Lockfile: `Cargo.lock` present and committed

## Frameworks

**Core:**
- `fuser` 0.12.0 — Rust FUSE bindings; wraps `libfuse` via `fusermount`/`macFUSE` syscall bridge; powers the `Filesystem` trait implementation in `src/fuse_handler.rs`

**CLI:**
- `clap` 4.5.53 (derive feature) — declarative CLI parsing in `src/main.rs`

**Testing:**
- Rust built-in `#[test]` and `#[cfg(test)]` — unit tests co-located in source modules
- Integration tests in `tests/` (cargo test `--test <name>`)
- `fio` (external tool, not a Rust dep) — I/O benchmark workload generator used by `benchmarks/`

**Build/Dev:**
- `cargo build` / `cargo build --release` — standard Cargo build
- No custom build.rs; no workspace; single-crate project

## Key Dependencies

**Critical:**
- `fuser` 0.12.0 — the entire FUSE mount surface; without it there is no mountable filesystem
- `sled` 0.34.7 — embedded database; stores all inode metadata, dirents, recipes, tag indices, and snapshot records at `<storage_dir>/metadata_db`
- `sha2` 0.10.9 — SHA-256 hashing of raw chunk bytes; forms the CAS content address in `src/storage.rs`
- `zstd` 0.13.3 (wraps `zstd-sys` 2.0.16 / libzstd 1.5.7) — per-chunk compression at level 3; stored in `<storage_dir>/cas/<hash[0:2]>/<hash[2:]>`
- `bincode` 1.3.3 — binary serialisation of all sled-persisted structs (`InodeMetadata`, `FileRecipe`, `Dirent`, `SnapshotMetadata`, `FileTagSet`)
- `serde` 1.0.228 (derive feature) — serialisation derive macros used on every persisted struct

**Infrastructure:**
- `libc` 0.2 — POSIX errno constants (`ENOENT`, `EEXIST`, `EISDIR`, `EROFS`, `EINVAL`) used in `src/fuse_handler.rs`
- `hex` 0.4.3 — encodes raw SHA-256 digest bytes to hex string for use as CAS path segments
- `env_logger` 0.10.2 + `log` 0.4 — structured log output initialised via `env_logger::init()` in `main`
- `parking_lot` 0.11.2 — used transitively by `sled`
- `crossbeam-*` — used transitively by `sled` for concurrent epoch-based reclamation

## Configuration

**Environment:**
- No `.env` files used; configuration is entirely CLI-flag driven
- Global `--storage-dir <path>` flag (default: `./my_storage`) shared across all subcommands
- Log level controlled by `RUST_LOG` environment variable (standard `env_logger` convention)

**Build:**
- `Cargo.toml` — single manifest, no workspace
- `Cargo.lock` — pinned dependency tree committed to repo

## Platform Requirements

**Development:**
- Linux or macOS (FUSE requires kernel FUSE module on Linux, or macFUSE on macOS)
- `fusermount` (Linux) or `mount_macfuse` (macOS) must be present for the `mount` subcommand
- `fio` must be installed separately for running benchmarks
- `python3` with matplotlib/pandas for chart generation scripts

**Production:**
- Self-contained native binary; target architecture: host platform (no cross-compile configuration present)
- FUSE kernel module must be loaded; mount requires either `CAP_SYS_ADMIN` or user-space FUSE with `AllowOther`
- Benchmark arena expects Linux loopback device support (`/dev/loop*`) and `mkfs.ext4` / `mkfs.btrfs`

## Observations

- The project targets Rust edition 2024 — the newest stable edition; this restricts compatible `rustc` to 1.85+
- `sled` 0.34.7 is the last release of the `sled` 0.x line (project has been in pre-1.0 limbo); there is no maintained upgrade path without a significant rewrite of the persistence layer
- `fuser` 0.12 wraps FUSE protocol version 7.31; the `AutoUnmount` mount option is used, which relies on the FUSE daemon cleaning up on process exit
- `zstd-sys` embeds libzstd 1.5.7 via a build-time C compile (`cc` crate); no system zstd dependency required
- No async runtime (tokio, async-std) is used — the FUSE event loop is synchronous and single-threaded at the FUSE handler level; concurrency comes only from `RwLock` contention management
- No web framework, no HTTP client, no external API SDKs present
