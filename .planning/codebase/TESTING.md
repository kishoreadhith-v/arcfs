# TESTING
> Generated: 2026-05-07 | Focus: quality | Project: arcfs

## Summary
ArcFS has three testing tiers: unit tests embedded in source modules, Rust integration tests in `tests/`, and shell-based E2E tests that exercise the live FUSE mount. Temp directories are cleaned at test start (not end), so failures leave residue that the next run clears.

## Test Tiers

### 1. Unit Tests (`#[cfg(test)]` in source modules)
Located inline in each source file, exercising the module in isolation:

| Module | Tests |
|---|---|
| `chunker.rs` | Chunking consistency — same input → same chunk boundaries |
| `storage.rs` | CAS round-trip (write then read), compression cycle (compress → decompress) |
| `file_manager.rs` | Schema roundtrip — serialize/deserialize `InodeMetadata`, `FileRecipe`, `Dirent` |

Run with: `cargo test --lib`

### 2. Integration Tests (`tests/*.rs`)
Full-stack tests using a real `FileManager` + sled DB but no FUSE mount:

| File | Coverage |
|---|---|
| `backend_stress.rs` | Concurrent writes, deduplication under load, chunk boundary correctness |
| `gc_test.rs` | Garbage collection: unreferenced chunks removed, referenced chunks retained |
| `tagfs_test.rs` | Tag CRUD (set, get, delete), `tag_index` consistency, multi-tag queries |

Run with: `cargo test --test <name>`

### 3. Shell E2E Tests (`tests/*.sh`)
Require a working FUSE environment (`fusermount` available):

| Script | Coverage |
|---|---|
| `regression_e2e.sh` | Full mount lifecycle: mount → write → read → verify → unmount |
| `verify_single_backing.sh` | Deduplication invariant: two identical files share exactly one CAS chunk |
| `architecture_compliance.sh` | Source-level invariants via grep (no `unwrap()` on sled, lock ordering) |

## Test Infrastructure

### Setup Pattern
Each integration test file uses a `setup()` helper:
```rust
fn setup(n: usize) -> FileManager {
    let path = format!("./test_db_{n}");
    let _ = std::fs::remove_dir_all(&path); // clean previous residue
    std::fs::create_dir_all(&path).unwrap();
    FileManager::new(&path).unwrap()
}
```

### Temp Directory Naming
- `./test_db_N` — integration backend tests
- `./test_gc_*` — GC-specific tests  
- `./test_tagfs_N` — TagFS integration tests

All are relative to the working directory (repo root). They are **not** cleaned up on test success — only wiped at the start of the next run. This means a failed test run leaves directories that must be manually removed or will be cleared by the next `cargo test`.

### Benchmark Tests (`benchmarks/`)
Separate from correctness tests — require a pre-configured loopback arena (see `benchmarks/setup_arena.sh`) and compare ArcFS against ext4/btrfs reference mounts using `fio`. Always run against `--release` build.

## Coverage Gaps
- No tests for `fuse_handler.rs` directly (requires FUSE mount; covered by E2E shell scripts instead)
- No property-based testing (no `proptest`/`quickcheck` dependency)
- No mock layer — tests hit real sled DB instances
- Snapshot CoW behavior has limited integration test coverage

## Observations
- Test isolation is strong for integration tests (each gets its own numbered sled DB)
- E2E tests are environment-dependent and cannot run in CI without FUSE kernel support
- `architecture_compliance.sh` provides a fast grep-based sanity check that can run anywhere
