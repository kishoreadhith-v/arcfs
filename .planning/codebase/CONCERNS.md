# CONCERNS
> Generated: 2026-05-07 | Focus: concerns | Project: arcfs

## Summary

ArcFS carries meaningful risk in three areas: pervasive `unwrap()` calls (152 in `fuse_handler.rs` alone) that will panic the FUSE process on any poisoned lock or unexpected `None`; a silent inode/recipe leak on every `unlink` because neither `ino_meta` nor `ino_recipe` sled keys are removed; and O(N × M) snapshot-tree linear scans executed on every write, setattr, and lookup hot-path call. The codebase has no unit tests for `fuse_handler.rs` and no integration tests covering snapshot CoW, GC-during-write races, or concurrent rename/write interactions.

---

## Technical Debt

**Hardcoded uid/gid and permissions in `dir_attr`:**
- Issue: `dir_attr()` always emits `uid: 1000, gid: 1000, perm: 0o755`; the requesting user's UID/GID from `&Request` is never consulted.
- Files: `src/fuse_handler.rs:96-97`
- Impact: ArcFS mounted with `AllowOther` will refuse access to any user whose uid != 1000. `setattr` mode/uid/gid parameters are all prefixed `_` and ignored.
- Fix approach: Read `req.uid()` / `req.gid()` at mount time or per-call; honour `setattr` uid/gid/mode fields.

**Statfs reports fabricated disk stats:**
- Issue: `statfs` returns hardcoded `total_blocks = 1024*1024` (~4 GB) and `free_blocks = total_blocks / 2` regardless of actual storage.
- Files: `src/fuse_handler.rs:2063-2081`
- Impact: `df`, GUI file managers, and tools that check available space before writing will show incorrect data; programs that respect free-space thresholds may corrupt or fail.
- Fix approach: Query actual bytes used by `<storage_dir>/cas/` with `fs::metadata` or `du`; subtract from a configurable capacity.

**Legacy `write_file`/`read_file`/`list_files` API preserved in `FileManager`:**
- Issue: `write_file`, `read_file`, `list_files`, `delete_file`, `rename_file`, `create_directory`, and `get_file_metadata` all use a parallel `legacy_name:<name>` → inode-id indirection layer separate from the FUSE `dirent:` tree.
- Files: `src/file_manager.rs:605-813`
- Impact: CLI `write`/`read`/`list` commands operate on a completely different namespace from the mounted filesystem; files written via CLI do not appear in `ls mnt/` and vice versa. The two namespaces can diverge silently.
- Fix approach: Remove legacy layer; route CLI commands through the FUSE-compatible `dirent:` tree using inode 1 as root.

**Backup file committed to source tree:**
- Issue: `src/fuse_handler.rs.backup` (920 lines, an older revision) is checked into the repo.
- Files: `src/fuse_handler.rs.backup`
- Impact: Confuses IDEs, increases repo size, and may be diffed or compiled inadvertently.
- Fix approach: Delete file; add `*.backup` to `.gitignore`.

**`src/chat.md` is a personal AI conversation log committed to source:**
- Issue: `src/chat.md` contains a verbatim GitHub Copilot session about preparing for a university viva.
- Files: `src/chat.md`
- Impact: Exposes intent and prior knowledge; not source code; confuses any tooling that scans `src/`.
- Fix approach: Delete file from repo and add `src/*.md` or the specific file to `.gitignore`.

---

## Known Issues / TODOs

**Inode and recipe are never deleted on `unlink`:**
- What happens: `unlink` removes the child from `parent.children`, calls `delete_dirent`, `delete_file_tags`, and `evict_inode_cache` — but never calls `FileManager::delete_inode` or removes the `ino_recipe` key from sled. No `delete_inode` function exists in `FileManager`.
- Files: `src/fuse_handler.rs:1834-1896`, `src/file_manager.rs` (missing function)
- Impact: Every deleted file leaves `ino_meta:<id>` and `ino_recipe:<id>` rows in sled permanently. At restart, `hydrate_tree` tries to re-add these orphan inodes if a stale `dirent:` key also exists. GC recovers the CAS chunks but not the sled metadata rows.
- Fix approach: Add `pub fn delete_inode(&self, id: u64) -> Result<(), String>` to `FileManager` that removes both `ino_meta:<id>` and `ino_recipe:<id>`; call it from `unlink`, `rmdir`, and the CoW path when old inodes are displaced.

**`inode_registry` is never cleaned up after `unlink`:**
- What happens: `evict_inode_cache` removes from `page_cache` and `cache_lru` but the inode's `Arc<RwLock<Inode>>` remains in `inode_registry` for the process lifetime.
- Files: `src/fuse_handler.rs:523-529`, `src/fuse_handler.rs:1889-1892`
- Impact: `inode_registry` grows monotonically; long-running mounts with many create/delete cycles will consume unbounded memory. `statfs` reports this inflated count as the file count.
- Fix approach: Add `self.inode_registry.write().unwrap().remove(&removed_ino)` inside `unlink` after removing from parent's `children`.

**GC can delete chunks being written concurrently:**
- What happens: `run_gc` scans all `ino_recipe:` keys to build `active_hashes`, then deletes CAS files not in that set. If a concurrent write call inserts a new chunk into CAS but has not yet persisted its recipe, GC will see the chunk as unreferenced and delete it.
- Files: `src/file_manager.rs:682-709`, `src/storage.rs:20-47`
- Impact: Data loss. File content silently truncated or returns an IO error on next read.
- Fix approach: Run GC only when the filesystem is quiesced (unmounted or write-locked); or use a two-phase mark approach with a pending-recipe registry; or hold a shared lock around the write-then-recipe sequence.

**CoW (`get_mutable_inode`) is not called during `write` or `setattr`:**
- What happens: `mkdir` and `rename` call `get_mutable_inode` via `let _ = self.get_mutable_inode(...)`, but `write` and `setattr` mutate inode attributes directly without checking `Arc::strong_count` or performing CoW.
- Files: `src/fuse_handler.rs:1417-1529`, `src/fuse_handler.rs:1729-1785`
- Impact: Writing to a file that is also referenced by a snapshot will mutate the snapshot's inode in-place, breaking the snapshot's read-only guarantee and the structural-sharing invariant.
- Fix approach: Call `get_mutable_inode` at the start of `write` and `setattr` for any live inode before modifying it; discard the result if only used for side-effect CoW.

**`rmdir` on a top-level tag name deletes ALL tags from ALL matching files:**
- What happens: `rmdir` on a child of `TAGS_DIR_ID` iterates all files with that tag and calls `delete_file_tags` on each, removing every tag from each file — not just the one tag.
- Files: `src/fuse_handler.rs:1908-1923`
- Impact: `rm -rf mnt/@tags/work` irreversibly strips every tag from every file tagged "work", destroying unrelated tag metadata.
- Fix approach: `rmdir` on a tag should call `remove_tag_from_file(id, filename, tag_name)` (which already exists) rather than `delete_file_tags`; the latter should only be called when removing all tags intentionally.

**`name.to_str().unwrap()` on non-UTF-8 filenames will panic the FUSE process:**
- What happens: `lookup`, `unlink`, `create`, `mkdir`, `rename`, `rmdir` all call `name.to_str().unwrap()` directly on `OsStr` filename arguments.
- Files: `src/fuse_handler.rs:1018, 1546, 1601, 1839, 1909, 1958, 1959`
- Impact: Any client that creates a file with a non-UTF-8 name (valid on Linux) will immediately crash the FUSE server process, unmounting the filesystem and potentially losing unflushed dirty cache.
- Fix approach: Replace `name.to_str().unwrap()` with `name.to_str().ok_or(EINVAL)?` and propagate `reply.error(EINVAL)`.

---

## Risk Areas

**Snapshot O(N×M) linear scan on every hot-path call:**
- Files: `src/fuse_handler.rs:435-443` (`inode_in_snapshot`), `src/fuse_handler.rs:654-675` (`find_in_snapshot_tree`)
- Why risky: `inode_in_snapshot` is called from `write` (line 1463), `setattr` (line 1753), `create` (line 1542), `unlink` (line 1835), `rename` (lines 1962/1965), and `access` (line 2085). Each call iterates all snapshots × all inodes in each snapshot tree. With 10 snapshots of 1000-node trees and 1000 concurrent file operations, each FUSE call does up to 10,000 node traversals before performing any actual work.
- Safe modification: Add a `HashSet<u64>` of snapshot-owned inode IDs maintained incrementally; check membership in O(1).

**Nested lock acquisitions without consistent ordering in `evict_under_pressure`:**
- Files: `src/fuse_handler.rs:453-489`
- Why risky: The function holds `cache_lru` write lock (line 461) and then acquires `page_cache` read lock (line 464) inside the same block. If any other code path acquires `page_cache` first then tries `cache_lru`, a deadlock results. The documented lock order in `CLAUDE.md` does not mention `cache_lru` relative to `page_cache`.
- Safe modification: Separate the LRU drain and the cache membership check into sequential lock acquisitions; never hold both write locks simultaneously.

**`page_cache` miss in `write` handler acquires write lock and calls `read_file_by_id` under it:**
- Files: `src/fuse_handler.rs:1470-1481`
- Why risky: `cache_map.entry(target_ino).or_insert_with(|| { self.manager.read_file_by_id(target_ino).unwrap_or_default() ... })` holds the `page_cache` write lock while reading all chunks from disk and decompressing them. For large files this can block the entire FUSE thread for seconds, stalling all concurrent reads.
- Safe modification: Read the file data before acquiring the write lock; then insert only if still absent.

**`fuser::mount2` blocks the main thread without clean shutdown handling:**
- Files: `src/main.rs:163`
- Why risky: `fuser::mount2(...).unwrap()` blocks indefinitely; SIGINT is handled by `AutoUnmount` but there is no explicit flush of dirty cache on shutdown. If the process receives SIGKILL or crashes, dirty page-cache entries are lost.
- Safe modification: Register a signal handler that calls `flush_all_dirty_cache` before the FUSE session ends; or use `fuser::Session` with an explicit `unmount()` call on drop.

**`sled::open` in `FileManager::new` called with `.expect` — no error recovery at startup:**
- Files: `src/file_manager.rs:65`
- Why risky: If the metadata DB is corrupted, locked, or on a read-only filesystem, the process panics immediately with no diagnostic and no ability to recover or report to the caller.
- Safe modification: Return `Result<FileManager, …>` from `new`; propagate the error to `main.rs` and emit a clear message before exiting.

**`Storage::new` silently calls `unwrap` on `create_dir_all`:**
- Files: `src/storage.rs:15`
- Why risky: If the storage directory cannot be created (permissions, out-of-space), the process panics with no user-visible error.
- Safe modification: Return `Result` from `Storage::new`; propagate to `FileManager::new`.

---

## Observations

- There are 152 `.unwrap()` calls in `src/fuse_handler.rs` and 16 `.expect(...)` calls across the codebase. Every unwrap on a `RwLock` guard will panic if the lock was poisoned by a prior panic elsewhere, turning a single bug into a permanent crash loop.
- There are zero unit tests for `src/fuse_handler.rs`. All FUSE behaviour (lookup, read, write, snapshot, CoW, TagFS control file) is untested at the unit level and only covered by shell-based end-to-end scripts (`tests/regression_e2e.sh`) that require a FUSE mount.
- The `Inode` struct stores `recipe: Option<FileRecipe>` annotated `#[allow(dead_code)]`. It is never populated in any constructor or hydration path; the actual recipe is always retrieved from sled on demand. The field wastes memory and creates false expectations about where file content metadata lives.
- Timestamps (`atime`, `mtime`, `ctime`, `crtime`) are re-assigned `SystemTime::now()` every time `dir_attr()` is called, meaning directory attributes change on every `getattr` call and are never stable across calls. This can cause spurious cache invalidations in POSIX tools.
- The `Snapshot` struct fields `name` and `timestamp` are both `#[allow(dead_code)]`; they are written but never read after construction. Snapshot listing in `readdir` only uses `snapshot.root`.
- `AllowOther` is passed as a mount option unconditionally; this requires `user_allow_other` in `/etc/fuse.conf` on Linux. If absent, mount silently succeeds but non-root users cannot access the mount point.
- The `MountOption::RW` comment reads `// Read-Only` — the comment directly contradicts the code (`src/main.rs:155`).

---

*Concerns audit: 2026-05-07*
