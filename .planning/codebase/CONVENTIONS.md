# CONVENTIONS
> Generated: 2026-05-07 | Focus: quality | Project: arcfs

## Summary
ArcFS follows consistent Rust idioms throughout: snake_case for files/functions, PascalCase for types, verb-first method naming, and a two-tier error handling model that separates internal logic errors from FUSE syscall boundaries. There is no use of `anyhow` or `thiserror` — errors are plain `String` or `i32` (libc errno).

## Naming Conventions

### Files & Modules
- All source files use `snake_case`: `chunker.rs`, `file_manager.rs`, `fuse_handler.rs`, `storage.rs`
- Module names mirror file names exactly

### Types
- Structs and enums: `PascalCase` — `InodeMetadata`, `FileRecipe`, `SnapshotMetadata`, `Dirent`
- Constants: `SCREAMING_SNAKE_CASE` — `VIRTUAL_INODE_START`, `MIN_CHUNK_SIZE`, `TARGET_CHUNK_SIZE`

### Methods
Verb-first naming pattern is strictly observed:

| Prefix | Purpose |
|---|---|
| `save_*` / `load_*` | sled DB persistence operations |
| `write_*` / `read_*` | File I/O operations |
| `is_*` | Boolean predicates |
| `*_key` | DB key builder helpers |

## Error Handling

### Two-Tier Model
ArcFS uses two distinct error styles depending on context:

1. **Internal logic** — `Result<T, String>` (no `anyhow`, no `thiserror`)
   - Errors are descriptive strings propagated with `?` or `.map_err(|e| e.to_string())`
   - Used throughout `file_manager.rs`, `storage.rs`, `chunker.rs`

2. **FUSE syscall boundaries** — `Result<T, i32>` using `libc` errno constants
   - `fuse_handler.rs` converts internal errors into FUSE reply errors via `reply.error(libc::EIO)` etc.
   - FUSE operations never bubble `String` errors up — always translated to errno

3. **Storage layer** — `std::io::Error` used directly for filesystem I/O

### `unwrap()` Policy
- **Permitted**: `RwLock` guard acquisition (poison = unrecoverable), test setup boilerplate
- **Forbidden**: sled DB operations (enforced by `architecture_compliance.sh` grep checks)
- `.expect()` used over raw `.unwrap()` when a message aids debugging

## Module Organization

### Dependency Direction
```
main.rs
  └── fuse_handler.rs
        ├── file_manager.rs
        │     ├── chunker.rs
        │     └── storage.rs
        └── (direct sled access for tag queries)
```
No circular dependencies. Lower modules have no knowledge of FUSE or CLI.

### `pub` vs Private
- Public surface is minimal: structs and functions only exported when required by `fuse_handler` or `main`
- Internal helpers are private by default; promoted to `pub(crate)` only when needed across modules

## Derive Patterns
- `#[derive(Debug, Clone, Serialize, Deserialize)]` is the standard set for persisted types
- `PartialEq` added only where equality comparison is needed (test assertions)
- No custom `Display` implementations — `Debug` format used for error messages

## Observations
- No macro-heavy code; minimal use of proc-macros beyond standard derives
- No async runtime — fully synchronous with FUSE handling thread safety via `Arc<RwLock<_>>`
- `architecture_compliance.sh` acts as a lightweight linting gate for source-level invariants (unwrap on sled, lock ordering)
