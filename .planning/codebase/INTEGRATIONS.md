# INTEGRATIONS
> Generated: 2026-05-07 | Focus: tech | Project: arcfs

## Summary

ArcFS has no external network services, cloud APIs, or third-party SaaS integrations. All integrations are local system-level: the Linux/macOS FUSE kernel interface, the local filesystem for chunk storage, and an embedded `sled` database for metadata. The only "external" tooling integrations are `fio` for benchmarking and `fusermount`/`macFUSE` for mounting.

## APIs & External Services

**None.** ArcFS makes no outbound network calls and depends on no external APIs, cloud providers, or web services.

## Data Storage

**Metadata Database:**
- Type: Embedded key-value store — `sled` 0.34.7
- Location: `<storage_dir>/metadata_db/` (defaults to `./my_storage/metadata_db/`)
- Client: `sled::Db` opened directly in `src/file_manager.rs` via `sled::open(db_path)`
- Serialisation: `bincode` 1.3.3 over `serde`-derived structs
- Key schema: prefix-namespaced byte keys (`ino_meta:`, `ino_recipe:`, `dirent:`, `ino_tags:`, `tag_index:`, `snapshot:`, `legacy_name:`, `legacy:next_ino`)
- Flush: explicit `self.db.flush()` call after every mutating operation; no WAL replay dependency at startup

**Chunk Object Store (CAS):**
- Type: Local filesystem directory tree
- Location: `<storage_dir>/cas/<hash[0:2]>/<hash[2:]>` (defaults to `./my_storage/cas/`)
- Format: zstd-compressed raw bytes (level 3); hash computed over uncompressed data
- Access: direct `std::fs` calls in `src/storage.rs` — `File::create`, `File::open`, `fs::remove_file`
- Deduplication: implicit existence check (`file_path.exists()`) before write; no locks — relies on single-writer semantics

**File Storage:**
- Local filesystem only; no remote object storage, no S3, no NFS backing

**Caching:**
- In-process write-back page cache: `page_cache: Arc<RwLock<HashMap<u64, (Vec<u8>, bool)>>>` in `src/fuse_handler.rs`
- Capacity: `PAGE_CACHE_CAPACITY = 1024` entries (LRU eviction via `VecDeque` LRU tracker)
- Flush trigger: FUSE `fsync` and `release` operations

## Authentication & Identity

**Auth Provider:** None. No authentication layer exists.
- File ownership uses the calling process's UID/GID via `libc` at inode creation time
- The `AllowOther` FUSE mount option permits any local user to access the mounted filesystem
- No ACLs, no capability checks beyond standard POSIX file permission bits stored in `FileAttr`

## FUSE System Interface

**Kernel Integration:**
- Interface: Linux FUSE kernel module (or macFUSE on macOS)
- Library: `fuser` 0.12.0 — Rust bindings wrapping `libfuse`
- Mount options used: `RW`, `AllowOther`, `FSName("arcfs")`, `AutoUnmount`
- Mount entry point: `fuser::mount2(fs_impl, mount_point, &options)` in `src/main.rs`
- Implementation: `ArcFS` struct in `src/fuse_handler.rs` implements the `fuser::Filesystem` trait
- FUSE ops implemented: `lookup`, `getattr`, `setattr`, `readdir`, `mkdir`, `mknod`, `create`, `open`, `read`, `write`, `fsync`, `release`, `unlink`, `rmdir`, `rename`, `statfs`, `getxattr`, `listxattr`, `setxattr`, `removexattr`

## Monitoring & Observability

**Error Tracking:** None — no Sentry, Datadog, or similar integration.

**Logs:**
- `env_logger` + `log` facade; output to stderr
- Log level set via `RUST_LOG` environment variable (e.g., `RUST_LOG=debug cargo run -- mount ./mnt`)
- No structured log format (JSON, etc.); plain text only

**Metrics:** None — no Prometheus, StatsD, or telemetry endpoints.

## CI/CD & Deployment

**Hosting:** Local binary; no cloud deployment, no container image, no systemd unit.

**CI Pipeline:** None detected — no `.github/`, `.gitlab-ci.yml`, `.circleci/`, or similar CI configuration found in the repository.

**Release Build:**
- `cargo build --release` — required for benchmarks; no packaging/distribution pipeline configured

## Benchmarking System Integration

**Tool:** `fio` (Flexible I/O Tester) — external Linux tool, must be installed on the host
- Job files: `benchmarks/fio_jobs/{responsive,durable,worst_case,integrity}/`
- I/O engine: `io_uring` (requires Linux kernel 5.1+ with io_uring support)
- Benchmark runner: `benchmarks/run_benchmarks.sh`; `benchmarks/run_integrity_suite.sh`
- Arena setup: `benchmarks/setup_arena.sh` — creates 5 GB loopback images for ext4 and btrfs reference mounts

**Reference filesystems compared:**
- ext4 (loopback image at `~/benchmark_arena/ext4_mount`)
- btrfs with `compress=zstd:3` (loopback image at `~/benchmark_arena/btrfs_mount`)
- ArcFS (mounted via FUSE at `~/benchmark_arena/arcfs_mount`, backend at `~/benchmark_arena/arcfs_backend`)

**Chart generation:**
- `benchmarks/generate_charts.py` — requires Python 3 with matplotlib/pandas
- `benchmarks/generate_disk_usage_charts.py` — disk efficiency visualisation

## Webhooks & Callbacks

**Incoming:** None.

**Outgoing:** None.

## TagFS Control Interface (In-Process "Integration")

The `.tags/.tagfs_control` virtual file at FUSE inode 5 is a write-only sentinel that acts as an in-band control channel. Writing to it from the mount point triggers tag management without unmounting:

```bash
# Set tags on a live path
echo "set docs/report.txt work 2026" > mnt/.tags/.tagfs_control

# Delete all tags for a path
echo "del docs/report.txt" > mnt/.tags/.tagfs_control
```

- Command parsing: handled inside `fuse_handler.rs` `write()` implementation for inode 5
- Persistence: calls `FileManager::set_file_tags_by_path` / `delete_file_tags` which write to the `sled` DB

## Observations

- There are no network dependencies whatsoever; the binary is fully air-gapped
- `sled` 0.34 is the last published release; the project is effectively unmaintained upstream — this is the highest-risk dependency
- `AutoUnmount` relies on `fusermount -u` being available on PATH; missing it leaves stale FUSE mount points on crash
- `io_uring` in the fio jobs requires Linux 5.1+; macOS benchmark runs would require substituting `psync` or `posixaio` ioengine
- No secret management, no environment variable injection for credentials — consistent with the absence of any external service dependencies
