User: i have a viva on this project tomorrow. i need you to help me understand this project inside out perfectly to answer anything that they might ask

GitHub Copilot: Let me thoroughly explore the entire codebase so I can give you a complete breakdown.   

Read all source files

Read all docs and tests

Read [](file:///home/i763702/better-fs/docs/PHASE2_SUMMARY.md)

Read [](file:///home/i763702/better-fs/README.md)

Read [](file:///home/i763702/better-fs/docs/MANUAL_DEMO.md)

Read [](file:///home/i763702/better-fs/docs/OPTIMIZATIONS.md)

Read [](file:///home/i763702/better-fs/docs/DEMO_COMMANDS.txt)

Read [](file:///home/i763702/better-fs/docs/MDs/TAGFS_API_REFERENCE.md)

Read [](file:///home/i763702/better-fs/src/chunker.rs)

Read [](file:///home/i763702/better-fs/src/storage.rs)

Read [](file:///home/i763702/better-fs/src/main.rs)

Read [](file:///home/i763702/better-fs/src/fuse_handler.rs)

Read [](file:///home/i763702/better-fs/src/file_manager.rs)

Read [](file:///home/i763702/better-fs/tests/backend_stress.rs)

Read [](file:///home/i763702/better-fs/src/lib.rs)

Read [](file:///home/i763702/better-fs/tests/gc_test.rs)

Read [](file:///home/i763702/better-fs/Cargo.toml)

Read [](file:///home/i763702/better-fs/tests/tagfs_test.rs)

Read [](file:///home/i763702/better-fs/docs/MDs/TAGFS_KNOWN_LIMITS_CHECKLIST.md)

Read [](file:///home/i763702/better-fs/benchmarks/results/BENCHMARK_REPORT.md)

Read [](file:///home/i763702/better-fs/.github/copilot-instructions.md)

Read [](file:///home/i763702/better-fs/tests/architecture_compliance.sh)

Read [](file:///home/i763702/better-fs/docs/MDs/TAGFS_COMPLETE_REBUILD.md)

Read [](file:///home/i763702/better-fs/tests/regression_e2e.sh)

Read memory [](file:///home/i763702/.vscode-server/data/User/workspaceStorage/be8979ced36f329efefe0eb82c6311ab/GitHub.copilot-chat/memory-tool/memories/repo/arcfs_tagfs_analysis.md)

Read [](file:///home/i763702/.vscode-server/data/User/workspaceStorage/be8979ced36f329efefe0eb82c6311ab/GitHub.copilot-chat/chat-session-resources/9dda1ecd-4753-43b6-8f70-e5f3d0330a74/toolu_vrtx_0197NYdmfGLVuLnS9L6sVexj__vscode-1777434668170/content.txt#1-1), lines 1 to 500

Read [](file:///home/i763702/.vscode-server/data/User/workspaceStorage/be8979ced36f329efefe0eb82c6311ab/GitHub.copilot-chat/chat-session-resources/9dda1ecd-4753-43b6-8f70-e5f3d0330a74/toolu_vrtx_0197NYdmfGLVuLnS9L6sVexj__vscode-1777434668170/content.txt#500-500), lines 500 to 1000

Read [](file:///home/i763702/.vscode-server/data/User/workspaceStorage/be8979ced36f329efefe0eb82c6311ab/GitHub.copilot-chat/chat-session-resources/9dda1ecd-4753-43b6-8f70-e5f3d0330a74/toolu_vrtx_0197NYdmfGLVuLnS9L6sVexj__vscode-1777434668170/content.txt#1000-1000), lines 1000 to 1055

Now let me read the full `file_manager.rs` and `fuse_handler.rs` since those are the largest and most critical files:  

Read [](file:///home/i763702/better-fs/src/file_manager.rs#1-1), lines 1 to 200

Read [](file:///home/i763702/better-fs/src/fuse_handler.rs#1-1), lines 1 to 200

Read [](file:///home/i763702/better-fs/src/file_manager.rs#200-200), lines 200 to 500

Read [](file:///home/i763702/better-fs/src/fuse_handler.rs#200-200), lines 200 to 600

Read [](file:///home/i763702/better-fs/src/file_manager.rs#500-500), lines 500 to 850

Read [](file:///home/i763702/better-fs/src/fuse_handler.rs#600-600), lines 600 to 1050

Read [](file:///home/i763702/better-fs/src/fuse_handler.rs#1050-1050), lines 1050 to 1500

Read [](file:///home/i763702/better-fs/src/fuse_handler.rs#1500-1500), lines 1500 to 2000

Read [](file:///home/i763702/better-fs/src/fuse_handler.rs#2000-2000), lines 2000 to 2138

Now I have a complete picture of the entire codebase. Let me also check the docs and benchmark results:  

Read [](file:///home/i763702/better-fs/README.md#1-1), lines 1 to 100

Read [](file:///home/i763702/better-fs/docs/OPTIMIZATIONS.md)

Now I have comprehensive knowledge of the entire project. Let me create a thorough viva preparation guide for you. 

Created memory file [](file:///home/i763702/.vscode-server/data/User/workspaceStorage/be8979ced36f329efefe0eb82c6311ab/GitHub.copilot-chat/memory-tool/memories/OWRkYTFlY2QtNDc1My00M2I2LThmNzAtZTVmM2QwMzMwYTc0/viva_prep.md)

Here's your **complete ArcFS viva preparation guide**, organized by likely question categories:

---

## 1. PROJECT OVERVIEW — "What does your project do?"

ArcFS is a **user-space filesystem** built in Rust using **FUSE** (Filesystem in Userspace). It provides:
- **Content-defined deduplication** — identical data stored only once
- **Snapshot/time-travel** (Chronos) — instant, zero-copy snapshots via Copy-on-Write
- **Tag-based virtual directories** (TagFS) — navigate files by tags, not paths
- **Compression** — zstd-compressed chunk storage
- **Write-back page cache** with LRU eviction

It mounts like a normal directory (`cargo run -- mount ./mnt`) and intercepts all VFS calls (read, write, mkdir, etc.) via the FUSE protocol.

---

## 2. ARCHITECTURE — "Explain the layering"

Four strict layers (high calls low, **never** vice versa):

| Layer | File | Role |
|-------|------|------|
| **FUSE Interface** | fuse_handler.rs | Translates kernel VFS calls → ArcFS ops. Path/inode resolution, POSIX checks |
| **Page Cache** | In-memory (`HashMap<u64, (Vec<u8>, bool)>`) | Write-back buffer. Absorbs 4KB writes, coalesces into chunk-aligned flushes |
| **Metadata Engine** | file_manager.rs | DAG namespace via `sled` embedded KV. ACID-compliant inode/dirent/recipe/tag storage |
| **CAS Storage** | storage.rs | Immutable, append-only, content-addressed blob store on disk |

---

## 3. CHUNKING — "How does content-defined chunking work?"

**Algorithm**: FastCDC with **Gear Hash** (NOT Rabin-Karp)

```
hash = (hash << 1).wrapping_add(GEAR_MATRIX[byte_value])
```

- **Gear Matrix**: 256-entry lookup table, pre-computed at compile time via `const fn build_gear_matrix()` using SplitMix64 PRNG
- **Cut point**: `(hash & MASK) == 0` where `MASK = TARGET_SIZE - 1 = 4095`
- **Chunk sizes**: Min 2KB, Target 4KB, Max 64KB
- **Complexity**: $O(1)$ per byte — just a shift + array lookup + addition
- **Why not Rabin-Karp?** Rabin uses polynomial arithmetic per byte — much slower

**Key insight for viva**: The chunks are **content-defined**, so inserting bytes at the start of a file only shifts boundaries locally. Fixed-size chunking would invalidate ALL chunks after the insertion point.

---

## 4. STORAGE (CAS) — "How is data stored on disk?"

Defined in storage.rs:

1. **Hash**: SHA-256 of raw chunk data → 64-char hex string
2. **Shard**: First 2 hex chars become a subdirectory (`cas/a1/remaining_hash...`)
3. **Dedup check**: If file already exists at that path → skip (content-addressed!)
4. **Compress**: `zstd::encode_all(data, 3)` — compression happens **after** chunking, **before** disk write
5. **Write**: Compressed bytes written to sharded path

**Read path**: Look up `cas/{hash[0..2]}/{hash[2..]}` → decompress with `zstd::decode_all` → return raw bytes

**Why compress after chunking?** If you compress first, a 1-byte change cascades through the compression tree and destroys dedup potential.

---

## 5. METADATA (sled DB) — "What's in the database?"

Defined in file_manager.rs. Prefix-isolated namespaces:

| Key Pattern | Value | Purpose |
|-------------|-------|---------|
| `ino_meta:{id}` | `InodeMetadata` (bincode) | Inode: id, parent_id, name, is_dir, size |
| `ino_recipe:{id}` | `FileRecipe` (bincode) | Ordered list of SHA-256 chunk hashes |
| `dirent:{parent}:{name}` | `u64` child inode ID | Directory entry edges in the DAG |
| `ino_tags:{id}` | `FileTagSet` (bincode) | Tags for a file |
| `tag_index:{tag}` | `Vec<u64>` (bincode) | Inverted index: tag → set of inode IDs |
| `snapshot:{name}` | `SnapshotMetadata` | Snapshot name, timestamp, root inode ID |

**Why sled?** Embedded, zero-config, ACID-compliant, crash-safe (atomic flushes). No external database needed.

---

## 6. PAGE CACHE — "How does caching work?"

Structure: `HashMap<u64, (Vec<u8>, bool)>` — maps inode ID → (buffer, is_dirty)

**Write path** (fast path):
1. Get mutable ref to buffer in cache
2. `resize()` if needed, then `copy_from_slice()` — **zero clones**, $O(1)$ amortized
3. Set `is_dirty = true`
4. Return success immediately (write-back, NOT write-through)

**Read path**:
1. Cache hit → serve `&data[start..end]` directly
2. Cache miss → load recipe from sled → fetch chunks from CAS → decompress → populate cache

**Flush triggers**: `release()`, `fsync()`, `flush()`, or LRU eviction under memory pressure

**LRU eviction** (`VecDeque<u64>`):
- Capacity: 1024 entries (`PAGE_CACHE_CAPACITY`)
- When full: pop front of deque, if dirty → flush to disk first, then remove from cache
- `touch_cache_entry()` moves accessed entries to back of deque

**Critical optimization**: The old code cloned the entire buffer on every write ($O(N^2)$ for large files). Now it mutates in-place.

---

## 7. SNAPSHOTS (Chronos) — "How do snapshots work?"

**Create**: `mkdir .snap_v1` in the mount, or `echo "v1" > .snapshots/.create`

**Mechanism** — Metadata Pointer Duplication (NOT data copying):
1. `flush_all_dirty_cache()` — ensure all dirty buffers are persisted
2. `clone_subtree_from_metadata()` — recursively clone inodes with NEW IDs, copy recipes (same chunk hashes)
3. Store snapshot root in `snapshots` HashMap and persist to sled

**Why zero-copy?** The CAS layer is immutable/append-only. Snapshots and live files point to the **same** chunk hashes. Modifying live data creates **new** chunks → live recipe updates → snapshot recipe untouched.

**Copy-on-Write (CoW)** via `get_mutable_inode()`:
- Walks the path from root to target
- At each level, checks `Arc::strong_count()` — if > 1, the node is shared with a snapshot
- If shared: deep-clone the node with a **new inode ID**, replace pointer in parent, register in inode_registry
- Snapshot retains old `Arc` pointing to old, unmodified node

**Restore**: `mkdir .restore_v1`
1. Auto-creates backup snapshot of current state
2. Clears live tree (root children, registry, page cache)
3. Clones snapshot's subtree into live filesystem with new IDs

**Browse**: `ls .snapshots/v1/` — read-only (perm `0o555`), all writes return `EROFS`

---

## 8. TagFS — "How do tags work?"

**Tagging a file**: Write to `.tagfs_ctl`:
```bash
echo "set docs/report.txt work 2026" > .tagfs_ctl
```

**Auto-tagging**: When files are created or written, `derive_tags_from_parent_ancestry()` walks up the inode tree collecting ancestor directory names as tags. File under `projects/backend/api.rs` gets tags `["projects", "backend"]`.

**Browsing**: Navigate `@tags/` virtual directory:
- `ls @tags/` → shows all top-level tags
- `ls @tags/work/` → shows sub-tags + files tagged "work"
- `ls @tags/work/2026/` → intersection (AND) of tags

**Virtual inodes** (start at 1,000,000):
- `TagVirtualDirContext` — maps virtual inode to tag combination
- `TagVirtualFileContext` — maps virtual inode back to real inode
- Ephemeral, created on-the-fly during navigation

**Tag query semantics**: AND intersection — `get_files_by_tags(["work", "2026"])` returns only files with BOTH tags.

**Normalized**: Tags are lowercased, deduplicated, and sorted for canonical representation.

---

## 9. CONCURRENCY — "How do you prevent deadlocks?"

**Strict lock ordering** (acquire in this order only, never reverse):
1. `inode_registry` (RwLock) — global lookup table
2. Per-inode `RwLock` — individual file's attributes/state
3. `page_cache` (RwLock) — volatile write buffers

**Pattern**:
```rust
let cloned_arc = registry.read().get(id).clone();  // Step 1: get + clone Arc
drop(registry_lock);                                 // Step 2: DROP registry lock
let mut node = cloned_arc.write();                   // Step 3: lock individual node
```

**Why `Arc<RwLock<Inode>>`?** Multiple readers can hold the registry simultaneously (RwLock read). Individual inodes can be independently locked without blocking the whole tree.

---

## 10. CRASH CONSISTENCY — "What happens on power loss?"

- **sled** guarantees atomic flushes → metadata is always consistent
- If crash before `release()`/`fsync()`: dirty page cache is lost, but the last-flushed metadata + CAS chunks remain valid
- **Orphaned chunks** (chunks written to CAS but not yet referenced in a recipe) are ignored until **Garbage Collection** runs
- **Startup recovery** (`hydrate_tree()`): Reconstructs namespace graph from `dirent:{parent}:{name}` edges. Orphaned `ino_meta` entries without parent dirents are swept.

---

## 11. GARBAGE COLLECTION — "How does GC work?"

In file_manager.rs `run_gc()`:
1. Scan ALL `ino_recipe:*` entries → collect every referenced chunk hash into `active_hashes` set
2. List ALL physical chunk files from CAS directory
3. Any chunk NOT in `active_hashes` → delete from disk
4. Return count of deleted orphans

**When do orphans appear?** When a file is overwritten (new chunks created, old recipe replaced) or deleted.

---

## 12. BOOT SEQUENCE — "What happens at startup?"

`ArcFS::new()`:
1. Create root inode (ID 1), insert into registry
2. `hydrate_tree()` — recursively walks `dirent:1:*` edges, loads `ino_meta:*` for each child, builds in-memory tree of `Arc<RwLock<Inode>>`
3. `restore_snapshots()` — loads all `snapshot:*` entries from sled, rebuilds snapshot subtrees. If a snapshot references missing inodes, it's pruned.

---

## 13. KEY DATA STRUCTURES

```rust
Inode {
    id: u64,
    parent_id: u64,
    name: String,
    children: HashMap<String, Arc<RwLock<Inode>>>,  // In-memory tree
    attr: FileAttr,      // POSIX attrs (size, perm, timestamps)
    recipe: Option<FileRecipe>,
}

FileRecipe {
    file_size: u64,
    chunks: Vec<String>,  // Ordered SHA-256 hashes
    kind: FileKind,       // File or Directory
}
```

---

## 14. SPECIAL INODES

| ID | Name | Purpose |
|----|------|---------|
| 1 | `/` | FUSE root |
| 2 | `.snapshots` | Snapshot directory |
| 3 | `.create` | Write snapshot name here to create |
| 4 | `@tags` | TagFS root |
| 5 | `.tagfs_ctl` | Tag control file (write commands) |
| ≥100 | Live inodes | Generated via `next_inode` atomic counter |
| ≥1,000,000 | Virtual inodes | TagFS virtual dirs/files |

---

## 15. DEPENDENCIES (Cargo.toml)

| Crate | Purpose |
|-------|---------|
| `fuser` | Rust FUSE binding |
| `sled` | Embedded KV database |
| `sha2` | SHA-256 hashing |
| `zstd` | Compression |
| `bincode` + `serde` | Struct serialization |
| `clap` | CLI argument parsing |
| `libc` | POSIX error codes |

---

## 16. LIKELY TOUGH QUESTIONS & ANSWERS

**Q: Why FUSE instead of a kernel module?**
A: Safety (crashes don't panic the kernel), easier development (standard Rust), no need for root/kernel builds. Tradeoff: context switches between user/kernel space add latency (the "FUSE tax").

**Q: What's the FUSE tax?**
A: Every I/O operation requires 2 extra context switches (kernel → userspace daemon → kernel). This adds ~10-50μs latency per op compared to in-kernel filesystems.

**Q: Why variable-size chunks vs fixed-size?**
A: Inserting a byte at the start of a file with fixed-size chunks invalidates ALL subsequent chunks. CDC boundaries are content-determined, so only the affected region shifts. This is critical for deduplication of edited files.

**Q: What happens if two files share chunks and you delete one?**
A: The chunks remain on disk. Only recipes/metadata are deleted. GC later scans all recipes, identifies chunks not referenced by ANY recipe, and removes only those. This is a mark-and-sweep approach.

**Q: Is this ACID compliant?**
A: sled provides atomic writes and crash recovery. The page cache is volatile (intentionally — speed over safety for dirty writes). `fsync()` forces a flush. Worst case on crash: you lose unflushed writes but never corrupt existing data.

**Q: How does deduplication compare to btrfs?**
A: btrfs does block-level dedup (fixed 4K pages). ArcFS does content-defined dedup (variable 2-64KB chunks), which handles insertions/edits better. But btrfs runs in-kernel (no FUSE tax) and has decades of hardening.

**Q: Can snapshots diverge infinitely?**
A: Yes. Each snapshot points to its own recipe copies. As the live tree changes, old chunks accumulate. GC won't delete them as long as ANY snapshot references them. You'd need to delete old snapshots to reclaim space.

**Q: What's the `rename()` situation?**
A: It IS implemented (lines 1900-2060 of fuse_handler.rs). It handles CoW on both parents, POSIX overwrite semantics, and auto-tag updates after rename.

**Q: How do you handle concurrent writes?**
A: RwLock on the page cache. Multiple readers can coexist, writers get exclusive access. The lock is held briefly (just for the `copy_from_slice`). No fine-grained per-page locking — the whole cache map is locked, but operations are microsecond-level.

**Q: What are the limitations?**
A: No hard links, no symlinks, no extended attributes (stubs only), no POSIX ACLs, no `mmap` support, single-node only (no distribution), LRU cache is simple (no frequency-based eviction like ARC).

Good luck tomorrow!

User: prepare a study plan. assume i know nothing about this project or even this domain in general, i am completely new to this system level stuff. prepare a plan that i can follow to get upto speed

GitHub Copilot: Created memory file [](file:///home/i763702/.vscode-server/data/User/workspaceStorage/be8979ced36f329efefe0eb82c6311ab/GitHub.copilot-chat/memory-tool/memories/OWRkYTFlY2QtNDc1My00M2I2LThmNzAtZTVmM2QwMzMwYTc0/study_plan.md)

Here's your study plan. Follow it in order — each stage builds on the last.

---

## Stage 1: How Normal Filesystems Work (45 min)

**You need this because**: Every viva starts with "why does your project exist?" You can't answer that without knowing what you're replacing.

**Concepts to understand**:
- A filesystem is just a bookkeeping system. It maps human-readable names (`report.txt`) to raw bytes on a disk.
- **Inodes**: Every file/directory gets a unique number (inode). The inode stores metadata (size, permissions, timestamps) and points to where the data lives on disk. Think of it as a file's ID card.
- **Directory entries (dirents)**: A directory is just a table mapping names → inode numbers. When you do `ls`, the kernel reads that table.
- **The tree structure**: Root `/` is inode 1. It has dirents pointing to its children. Each child directory has dirents pointing to its children. It's a tree (actually a DAG — Directed Acyclic Graph — because hard links exist).
- **Read/Write flow**: App calls `write()` → kernel receives it → kernel finds the filesystem driver → driver writes bytes to disk and updates the inode.

**Connect to your code**: Open fuse_handler.rs and look at the `Inode` struct (line ~42). It has `id`, `parent_id`, `name`, `children` — that's exactly what a real filesystem inode does. Your `dirent:{parent}:{name} → child_id` keys in sled are directory entries.

---

## Stage 2: What is FUSE? (30 min)

**You need this because**: This is your project's foundation. "How does your filesystem actually run?" is a guaranteed question.

**Concepts**:
- Normally, filesystem code runs **inside the kernel** (ext4, NTFS, btrfs). This is dangerous — a bug crashes the entire machine.
- **FUSE** = Filesystem in Userspace. It lets you write filesystem logic as a normal program.
- **How it works**:
  1. Your program tells the kernel: "I'm handling the directory mnt"
  2. When ANY program does `ls ./mnt` or `cat ./mnt/file.txt`, the kernel intercepts it
  3. Kernel sends the request to YOUR program via fuse
  4. Your program processes it (lookup, read, write, etc.) and sends back a response
  5. Kernel forwards the response to the original program
- **The tradeoff** (FUSE tax): Every I/O operation requires extra context switches (kernel → your program → kernel). Real kernel filesystems skip this. This adds ~10-50 microseconds per operation.

**Connect to your code**: In fuse_handler.rs, `impl Filesystem for ArcFS` (line ~1030) is where you implement the FUSE interface. Each method (`lookup`, `read`, `write`, `mkdir`, etc.) is a callback the kernel invokes. In main.rs line ~143, `fuser::mount2(fs_impl, mount_point, &options)` is where your program registers with FUSE and starts receiving calls.

**Key vocabulary**:
- **VFS** (Virtual File System): The kernel's abstraction layer that routes filesystem calls to the right driver (ext4, FUSE, NFS, etc.)
- **inode number (ino)**: The unique ID the kernel uses to refer to files. Your root is always ino=1.

---

## Stage 3: Hashing & Content-Addressable Storage (45 min)

**You need this because**: This is the core innovation that makes dedup possible.

**Concepts**:
- **SHA-256 hash**: A function that takes ANY data → produces a fixed 64-character hex string. Same input ALWAYS produces same output. Different input virtually NEVER collides.
  - `SHA256("Hello") → "185f8db..."` — always.
  - Change one letter: `SHA256("Hellp") → "completely different hash"`
- **Content-Addressable Storage (CAS)**: Instead of storing files by name, you store them by their hash. The hash IS the filename on disk.
  - You store `"Hello World"` → hash is `a591a6d4...` → saved at `cas/a5/91a6d4...`
  - If someone stores `"Hello World"` again → same hash → file already exists → **skip** (deduplication!)
- **Sharding**: The `cas/` directory uses the first 2 hex chars as subdirectories (`cas/a5/`, `cas/1f/`, etc.) to prevent any single directory from having millions of files (which slows down the OS).

**Connect to your code**: Open storage.rs.
- `write_chunk()` (line ~24): Hash raw data → check if file exists (dedup!) → compress → write
- `read_chunk()` (line ~44): Find file by hash → read → decompress → return
- The compression (`zstd`) happens AFTER hashing. This is critical — if you compressed first, identical data with different compression settings would hash differently, breaking dedup.

---

## Stage 4: Content-Defined Chunking (1 hour)

**You need this because**: This is the most technically interesting part and the most likely to be asked about in depth.

**The problem**: You can't just hash entire files. If you edit 1 byte of a 1GB file, the whole hash changes → you store 1GB again → no dedup.

**The solution**: Split files into small chunks (2-64KB), hash each chunk independently. Edit 1 byte → only that chunk's hash changes → only 1 chunk re-stored.

**Fixed-size vs Content-Defined chunking**:
- **Fixed-size** (split every 4KB): Insert 1 byte at the start → ALL 4KB boundaries shift → ALL chunks change → dedup destroyed.
- **Content-Defined** (split based on content): Boundaries are determined by what the data contains, not position. Insert 1 byte → boundaries shift locally → only 1-2 chunks change → dedup preserved.

**How FastCDC (Gear Hash) works**:
1. You slide a window over the data byte by byte
2. For each byte, update a rolling hash: `hash = (hash << 1) + GEAR_TABLE[byte_value]`
   - `<< 1` = shift bits left (multiply by 2)
   - `GEAR_TABLE` = pre-computed random lookup table (256 entries, one per possible byte value)
3. Check: `(hash & MASK) == 0`? If yes → cut here (this is a chunk boundary)
   - `MASK = 4095` (binary: `111111111111`) — this means the bottom 12 bits of the hash must all be zero
   - Probability of that ≈ 1/4096 per byte → average chunk ≈ 4KB (your target)
4. Enforce minimum (2KB) and maximum (64KB) chunk sizes

**Why Gear Hash over Rabin-Karp?**: Rabin-Karp uses polynomial modular arithmetic per byte (multiply, add, modulo). Gear Hash uses one shift + one addition + one array lookup. Much faster: $O(1)$ with tiny constants.

**Connect to your code**: Open chunker.rs.
- `GEAR_MATRIX` (line ~28): The 256-entry lookup table, built at **compile time** via `const fn`
- `feed_byte()` (line ~38): The one-line rolling hash update
- `should_cut()` (line ~42): Min/Max enforcement + the mask check
- `chunk_lengths()` (line ~55): Takes raw bytes → returns list of chunk sizes

**Practice explaining this**: "We feed each byte through a Gear hash. The hash uses a bitwise shift and a lookup table, so it's O(1) per byte. When the bottom 12 bits of the hash are all zero, we declare a chunk boundary. This gives us an average 4KB chunk size with content-defined boundaries."

---

## Stage 5: The File Recipe & Read/Write Pipeline (45 min)

**You need this because**: This ties together chunking + CAS into a working filesystem.

**Concept — File Recipe**:
A recipe is a file's "assembly instructions". It's an ordered list of chunk hashes:
```
Recipe for "video.mp4":
  file_size: 10485760
  chunks: ["a1b2c3...", "d4e5f6...", "789abc...", ...]
```
To reconstruct the file: fetch each chunk from CAS in order → concatenate → original file.

**Write pipeline** (what happens when you `echo "hello" > mnt/file.txt`):
1. FUSE receives `write()` call → lands in fuse_handler.rs `write()` method
2. Data goes into the **page cache** (in-memory buffer): `cache[inode_id] = (data, dirty=true)`
3. Return success to the user immediately (fast!)
4. Later, on `release()`/`fsync()` → `flush_inode_cache()`:
   - Pass buffer to the FastCDC chunker → get chunks
   - Hash each chunk → `storage.write_chunk()` → deduplicate & compress & write to disk
   - Save the recipe: `ino_recipe:{inode_id} → [hash1, hash2, ...]` in sled
   - Mark cache entry as `dirty = false`

**Read pipeline** (what happens when you `cat mnt/file.txt`):
1. FUSE receives `read()` call
2. Check page cache → hit? Return bytes directly from memory
3. Miss? Load recipe from sled → fetch each chunk hash → `storage.read_chunk()` → decompress → concatenate → populate cache → return bytes

**Why write-back (not write-through)?**: Write-through would chunk+hash+compress on every single 4KB FUSE write. Write-back batches it — the user writes 100 times into the buffer, and we only chunk/hash/compress once on flush. Massive performance win.

**Connect to your code**: file_manager.rs `create_recipe_from_data()` (line ~380) does the chunking+hashing pipeline. `write_file_by_id()` and `read_file_by_id()` are the high-level wrappers.

---

## Stage 6: Snapshots & Copy-on-Write (1 hour)

**You need this because**: Snapshots are the "Chronos" feature — likely a major talking point.

**Core insight**: Because CAS is immutable and append-only, snapshots are almost free.

**How snapshot creation works** (`mkdir .snap_v1`):
1. Flush all dirty cache (ensure everything is persisted)
2. Clone the metadata tree: for every inode under root, create a NEW inode with a NEW ID, but copy the SAME recipe (same chunk hashes)
3. Save snapshot metadata to sled
4. No data is copied at all — just metadata pointers

**Why is this zero-copy?** Live file recipe: `[hash_A, hash_B, hash_C]`. Snapshot recipe: `[hash_A, hash_B, hash_C]`. Both point to the same physical chunks. If you later modify the live file → new chunks created → live recipe becomes `[hash_A, hash_D, hash_C]` → snapshot recipe unchanged → old chunks preserved.

**Copy-on-Write (CoW)** — the tricky part:
In memory, the inode tree uses `Arc<RwLock<Inode>>`. `Arc` is a reference-counted smart pointer. When a snapshot is created, multiple `Arc`s point to the same nodes.

When you want to MODIFY a node that's shared:
1. Check `Arc::strong_count() > 1` — if yes, this node is shared with a snapshot
2. Deep-clone the node with a NEW inode ID
3. Replace the pointer in the parent's children map
4. The snapshot still holds its old `Arc` pointing to the old, unmodified node

This is implemented in `get_mutable_inode()` in fuse_handler.rs.

**Key vocabulary**:
- **Arc**: Atomic Reference Counted pointer. Multiple owners of the same data. When last Arc is dropped, data is freed.
- **RwLock**: Read-Write Lock. Multiple readers OR one exclusive writer.
- **Shadow Paging**: Another name for this CoW technique — you create a "shadow" copy of the page before modifying it.

**Restore** (`mkdir .restore_v1`):
1. Auto-backup current state
2. Clear live tree
3. Clone snapshot's subtree back into live tree with new IDs

---

## Stage 7: TagFS — Virtual Tag Directories (45 min)

**You need this because**: This is the Phase 3 feature that differentiates your project.

**The problem**: Traditional filesystems force a single hierarchy. A file can only be in ONE folder. But what if `report.txt` is both a "work" file AND a "2026" file AND a "backend" file?

**The solution**: Tags. A file can have multiple tags. Navigate by tag combinations:
```
@tags/work/          → all files tagged "work"
@tags/work/2026/     → files tagged BOTH "work" AND "2026"
@tags/work/2026/backend/ → files tagged all three
```

**How it works**:
- Tags stored in sled: `ino_tags:{inode_id} → FileTagSet`
- Inverted index: `tag_index:{tag_name} → [inode_id1, inode_id2, ...]`
- Query is AND intersection: `get_files_by_tags(["work", "2026"])` loads both index sets, intersects them
- Virtual inodes (≥1,000,000) are created on-the-fly during navigation — they don't exist on disk

**Auto-tagging**: When you create a file under `projects/backend/api.rs`, the system walks up the tree collecting ancestor names → auto-tags with `["projects", "backend"]`. This means TagFS works without manual tagging.

**Tagging a file manually**: `echo "set path/to/file.txt work 2026" > .tagfs_ctl`

**Tags are normalized**: Lowercased, deduplicated, sorted. "Work" and "work" are the same tag.

---

## Stage 8: Putting It All Together — System-Level Understanding (45 min)

**Practice these end-to-end flows**:

### Flow 1: "I type `echo hello > mnt/test.txt`. Walk me through everything that happens."
1. Shell calls `open()` → FUSE → `create()` in your code → new inode (id=100), empty recipe, register in tree + sled
2. Shell calls `write(ino=100, offset=0, data="hello\n")` → FUSE → your `write()` → data goes into page cache: `{100: (b"hello\n", dirty=true)}`
3. Shell calls `release(ino=100)` → FUSE → your `release()` → `flush_inode_cache(100)`:
   - FastCDC chunks `"hello\n"` → one chunk (6 bytes, under min size, so it's the tail chunk)
   - SHA-256 hash the chunk → `"abc123..."`
   - zstd compress → write to `cas/ab/c123...`
   - Save recipe: `ino_recipe:100 → {size: 6, chunks: ["abc123..."]}`
   - Mark cache clean: `{100: (b"hello\n", dirty=false)}`

### Flow 2: "I create a snapshot and then modify a file. What happens?"
1. `mkdir .snap_v1` → clone metadata tree, recipes point to same chunks
2. `echo "world" >> mnt/test.txt` → CoW triggers if inode shared → new inode copy → write to cache
3. `release()` → new chunks created → new recipe → old chunks still exist for snapshot
4. `ls .snapshots/v1/test.txt` → reads snapshot's recipe → fetches old chunks → old data intact

### Flow 3: "What guarantees do you have on crash?"
- sled is ACID: metadata is always consistent
- Worst case: lose unflushed dirty cache (same as any write-back cache, like your laptop's RAM)
- CAS is append-only: never overwrite existing chunks → no data corruption possible
- Orphaned chunks (written but unreferenced): cleaned up by GC

---

## Quick Reference: Key Numbers

| Parameter | Value | Why |
|-----------|-------|-----|
| Min chunk | 2 KB | Prevent tiny chunks (overhead per chunk) |
| Target chunk | 4 KB | Matches OS page size, good dedup granularity |
| Max chunk | 64 KB | Prevent enormous chunks (poor dedup for large regions) |
| Cache capacity | 1024 entries | Balance RAM usage vs cache hit rate |
| Zstd level | 3 | Good compression/speed tradeoff (1=fast, 22=max compress) |
| Virtual inode start | 1,000,000 | Keep virtual IDs well above real ones |
| Root inode | 1 | FUSE convention |

---

## Study Order Checklist

- [ ] Stage 1 — Normal filesystems (inodes, dirents, trees)
- [ ] Stage 2 — FUSE (userspace, context switches, VFS)
- [ ] Stage 3 — Hashing & CAS (SHA-256, dedup, sharding)
- [ ] Stage 4 — FastCDC chunking (Gear hash, rolling hash, mask, boundaries)
- [ ] Stage 5 — Recipe pipeline (write-back cache, flush, read path)
- [ ] Stage 6 — Snapshots & CoW (Arc, strong_count, shadow paging)
- [ ] Stage 7 — TagFS (inverted index, virtual inodes, auto-tagging)
- [ ] Stage 8 — End-to-end flows (practice narrating 3 scenarios aloud)

After completing all stages, re-read the full answer I gave in my previous message — it will make complete sense now and serve as a quick revision sheet.

User: yes good plan. lets start with stage 1

GitHub Copilot: ## Stage 1: How Normal Filesystems Work

### Start from the very bottom: What IS a disk?

Your hard drive (SSD/HDD) is just a massive array of bytes. Think of it like a giant Excel spreadsheet with one column and billions of rows. Each row holds 1 byte. Each row has an address (0, 1, 2, 3, ...).

That's it. The disk has no concept of "files" or "folders". It's just bytes at addresses.

So when you do `cat report.txt`, something needs to:
1. Know that "report.txt" exists
2. Know WHERE on disk its bytes live (which addresses)
3. Know HOW MANY bytes it has
4. Fetch those bytes and hand them back

That "something" is the **filesystem**.

---

### The Inode: A file's ID card

Every file and directory gets a small data structure called an **inode** (index node). Think of it as a record in a database:

```
Inode #42:
  type: regular file
  size: 1,024 bytes
  owner: user "john"
  permissions: rw-r--r--
  created: 2026-04-28 10:30
  modified: 2026-04-29 14:15
  data location: blocks [1001, 1002]
```

Key points:
- The inode does NOT contain the filename. (This surprises people.)
- The inode does NOT contain the file's data. It contains a pointer TO the data.
- Every inode has a unique number. The kernel refers to files by this number, never by name.

**Now look at your code.** Open fuse_handler.rs:

```rust
pub struct Inode {
    pub id: u64,           // The unique inode number
    pub parent_id: u64,    // Who is my parent directory
    pub name: String,      // The name (you store it here for convenience)
    pub children: HashMap<String, Arc<RwLock<Inode>>>,  // If I'm a dir, my children
    pub attr: FileAttr,    // Size, permissions, timestamps — the "ID card"
    pub recipe: Option<FileRecipe>,  // Where my data lives (chunk hashes)
}
```

That's your inode. `FileAttr` is the FUSE library's struct that holds size, permissions, timestamps, etc. `recipe` is your equivalent of "data location" — instead of disk block numbers, you store chunk hashes (we'll get to that in Stage 3).

---

### Directory Entries (dirents): The name → number mapping

So if the inode doesn't store the filename, where IS the filename?

In the **parent directory**. A directory is really just a lookup table:

```
Directory Inode #1 (the root "/"):
  "documents"  →  inode #10
  "photos"     →  inode #11
  "report.txt" →  inode #42
```

Each row in this table is called a **directory entry** or **dirent**. It maps a name (string) to an inode number.

When you type `cat /documents/notes.txt`:
1. Start at inode #1 (root, always inode 1)
2. Look up "documents" in root's dirent table → inode #10
3. Look up "notes.txt" in inode #10's dirent table → inode #55
4. Read inode #55's data

**Now look at your code.** In file_manager.rs, you store dirents in sled:

```rust
fn dirent_key(parent_id: u64, name: &str) -> String {
    format!("dirent:{}:{}", parent_id, name)
}
```

So a dirent for "report.txt" inside root (inode 1) is stored as:
```
key:   "dirent:1:report.txt"
value: 42                      (the child's inode number)
```

To list all children of a directory, you scan all keys that start with `dirent:1:` — that's what `list_dirents()` does (line ~530).

---

### The tree structure

This gives you a tree:

```
inode 1 (root "/")
├── dirent "documents" → inode 10
│   ├── dirent "notes.txt" → inode 55
│   └── dirent "todo.txt"  → inode 56
├── dirent "photos" → inode 11
│   └── dirent "cat.jpg"   → inode 70
└── dirent "report.txt" → inode 42
```

Every path like `/documents/notes.txt` is just a chain of dirent lookups starting from inode 1.

**In your code**, this tree lives in memory as nested `HashMap`s inside each `Inode`'s `children` field. At boot, `hydrate_tree()` in fuse_handler.rs reconstructs this tree by reading all the `dirent:*` entries from sled.

---

### The four operations the kernel cares about

When the kernel talks to a filesystem, it speaks in terms of inode numbers, not paths. Here are the core operations:

| Operation | What the kernel asks | Your code |
|-----------|---------------------|-----------|
| **lookup**(parent_ino, "name") | "In directory #1, does 'report.txt' exist? What's its inode?" | `fn lookup()` in fuse_handler.rs |
| **getattr**(ino) | "What are the attributes of inode #42?" (size, perms, etc.) | `fn getattr()` |
| **read**(ino, offset, size) | "Give me 4096 bytes starting at byte 0 of inode #42" | `fn read()` |
| **write**(ino, offset, data) | "Write these bytes at offset 0 of inode #42" | `fn write()` |

Notice: the kernel NEVER says "read /documents/report.txt". It says "lookup inode 1 for name 'documents'" → gets inode 10 → "lookup inode 10 for name 'report.txt'" → gets inode 42 → "read inode 42". The path resolution happens step-by-step.

---

### A few more operations you implement

| Operation | What it does |
|-----------|-------------|
| **mkdir**(parent_ino, "name") | Create a new directory inode, add a dirent in parent |
| **create**(parent_ino, "name") | Create a new file inode, add a dirent in parent |
| **unlink**(parent_ino, "name") | Remove a dirent from parent (delete a file) |
| **readdir**(ino) | List all dirents in a directory (for `ls`) |
| **setattr**(ino, new_size) | Change attributes (e.g., truncate a file) |
| **release**(ino) | File handle closed — flush any cached data |

---

### Recap — The mental model

```
User types:    cat /work/report.txt

Kernel does:   lookup(ino=1, "work")      → ino=10
               lookup(ino=10, "report.txt") → ino=42
               getattr(ino=42)             → size=1024
               read(ino=42, offset=0, size=1024) → bytes

Your code:     lookup() checks inode_registry HashMap
               getattr() returns the FileAttr from the Inode struct
               read() checks page_cache, or loads from CAS
```

The filesystem is really just two data structures:
1. **Inodes** — the ID cards (your `Inode` struct)
2. **Dirents** — the name→number links (your `dirent:{parent}:{name}` keys in sled)

Everything else (chunking, snapshots, tags) is built ON TOP of this foundation.

---

### Check your understanding

Before moving to Stage 2, make sure you can answer these:

1. Why doesn't the inode store the filename?
2. If you know a file's inode number is 42, what do you still need to read it?
3. How does `ls /work/` work in terms of inode operations?
4. What's the difference between an inode and a dirent?

When you're ready, say "next" and we'll do Stage 2 (FUSE).

User: next

GitHub Copilot: ## Stage 2: What is FUSE?

### First: Where does filesystem code normally live?

Your operating system has two zones:

```
┌─────────────────────────────────────────┐
│           USER SPACE                     │
│  Your apps: Chrome, VS Code, cat, ls    │
│  They call open(), read(), write()      │
├─────────────────────────────────────────┤  ← System call boundary
│           KERNEL SPACE                   │
│  VFS layer (Virtual File System)        │
│  ext4 driver, NTFS driver, btrfs, ...  │
│  Disk drivers, memory management        │
└─────────────────────────────────────────┘
```

**Kernel space** is privileged. Code here can access hardware directly, manage memory, and do anything. But if it crashes → the entire machine crashes (kernel panic / blue screen).

**User space** is sandboxed. Your apps run here. If Chrome crashes, you restart Chrome. The kernel keeps running.

Traditionally, filesystem code (ext4, NTFS, btrfs) runs in **kernel space**. This makes it fast (no boundary crossing), but dangerous to develop. A single bug can take down the whole machine. Writing kernel code also requires deep C expertise, special build tools, and testing on real hardware or VMs.

---

### FUSE: Moving the filesystem to user space

FUSE (Filesystem in USErspace) is a kernel module that acts as a **bridge**. It lets you write filesystem logic as a normal program — in any language (Rust, Python, Go, C) — running safely in user space.

Here's the flow when a user types `cat mnt/hello.txt` and your ArcFS is mounted at mnt:

```
Step 1:  cat calls read()
              │
Step 2:  ─────▼──────────────────
         KERNEL: VFS receives read()
         VFS asks: "who owns mnt/?"
         Answer: "FUSE module"
              │
Step 3:  FUSE module writes the request
         into /dev/fuse (a special device file)
              │
Step 4:  ─────▼──────────────────
         YOUR PROGRAM (ArcFS) is blocking on
         /dev/fuse, waiting for requests.
         It receives: "read(ino=42, offset=0, size=4096)"
              │
Step 5:  Your code runs: check cache, load
         chunks, decompress, return bytes
              │
Step 6:  Your program writes the response
         back to /dev/fuse
              │
Step 7:  ─────▼──────────────────
         KERNEL: FUSE module receives response
         VFS forwards bytes to cat
              │
Step 8:  cat prints "Hello World"
```

Count the boundary crossings: user→kernel→user→kernel→user. That's **4 extra crossings** compared to a kernel filesystem (which would be user→kernel→user). Each crossing costs ~5-25 microseconds. This is the **FUSE tax**.

---

### The Filesystem trait: Your contract with the kernel

The `fuser` Rust library handles all the fuse communication for you. Your job is to implement a **trait** (think of it as an interface/contract). The trait says: "You must provide these methods, and I'll call them when the kernel needs them."

Look at fuse_handler.rs:

```rust
impl Filesystem for ArcFS {
    fn lookup(...)  { /* kernel asks: does this name exist? */ }
    fn getattr(...) { /* kernel asks: what are this inode's attributes? */ }
    fn read(...)    { /* kernel asks: give me bytes from this file */ }
    fn write(...)   { /* kernel asks: store these bytes in this file */ }
    fn mkdir(...)   { /* kernel asks: create a directory */ }
    fn create(...)  { /* kernel asks: create a file */ }
    fn readdir(...) { /* kernel asks: list directory contents */ }
    fn unlink(...)  { /* kernel asks: delete a file */ }
    fn setattr(...) { /* kernel asks: change file attributes */ }
    fn release(...) { /* kernel asks: file handle closed */ }
    fn fsync(...)   { /* kernel asks: flush data to disk NOW */ }
    fn flush(...)   { /* kernel asks: flush before close */ }
    fn rmdir(...)   { /* kernel asks: remove a directory */ }
    fn rename(...)  { /* kernel asks: move/rename a file */ }
    fn open(...)    { /* kernel asks: open a file handle */ }
    fn statfs(...)  { /* kernel asks: how much space is available? */ }
    fn access(...)  { /* kernel asks: does user have permission? */ }
}
```

Each of these is a callback. You don't call them — the FUSE library calls them when the kernel sends a request. Your code processes the request and calls `reply.entry()`, `reply.data()`, `reply.ok()`, or `reply.error()` to send back the response.

---

### The reply pattern: How you respond

Every FUSE method receives a `reply` object. You MUST call exactly one method on it. Examples:

```rust
// "Yes, this inode exists, here are its attributes"
reply.entry(&TTL, &file_attr, 0);

// "Here's the data you asked for"
reply.data(&bytes[start..end]);

// "Operation succeeded"
reply.ok();

// "Error: file not found"
reply.error(ENOENT);  // ENOENT = Error NO ENTry = file doesn't exist
```

If you forget to call reply, the calling program (cat, ls, etc.) hangs forever waiting for a response.

---

### How mounting works

Look at main.rs:

```rust
Commands::Mount { mount_point } => {
    // 1. Create the mount directory if it doesn't exist
    fs::create_dir_all(&mount_point).unwrap();

    // 2. Set FUSE options
    let options = vec![
        fuser::MountOption::RW,           // Read-Write mount
        fuser::MountOption::AllowOther,    // Other users can access
        fuser::MountOption::FSName("arcfs".to_string()),
        fuser::MountOption::AutoUnmount,   // Clean up on exit
    ];

    // 3. Create your filesystem implementation
    let fs_impl = fuse_handler::ArcFS::new(manager);

    // 4. THIS BLOCKS FOREVER — it starts the FUSE event loop
    fuser::mount2(fs_impl, mount_point, &options).unwrap();
}
```

`fuser::mount2()` does three things:
1. Tells the kernel: "I'm handling this directory now"
2. Starts an event loop that reads requests from fuse
3. For each request, calls the appropriate method on your `ArcFS`

This call **blocks** — your program sits here forever, processing FUSE requests, until the filesystem is unmounted (`fusermount -u ./mnt` or Ctrl+C).

---

### Let's trace a real `ls` command through your code

User types: `ls mnt/`

**Step 1 — Kernel calls `readdir(ino=1)`** (inode 1 = root)

Your `readdir()` in fuse_handler.rs runs:

```rust
fn readdir(&mut self, _req: &Request, ino: u64, ..., mut reply: ReplyDirectory) {
    // ... (tag/snapshot checks omitted for clarity)

    // For the root directory (ino=1):
    let registry = self.inode_registry.read().unwrap();
    if let Some(node) = registry.get(&ino) {
        let guard = node.read().unwrap();

        // Add standard entries
        reply.add(ino, 0, FileType::Directory, ".");     // Current dir
        reply.add(ino, 1, FileType::Directory, "..");    // Parent dir

        // Add special virtual entries at root level
        reply.add(SNAPSHOT_DIR_ID, ..., ".snapshots");
        reply.add(TAGS_DIR_ID, ..., "@tags");
        reply.add(TAGFS_CONTROL_ID, ..., ".tagfs_ctl");

        // Add actual children from the in-memory tree
        for (name, child_arc) in guard.children.iter() {
            let child = child_arc.read().unwrap();
            reply.add(child.id, ..., child.attr.kind, name);
        }
    }
    reply.ok();
}
```

**Step 2 — For each entry, kernel calls `getattr(ino)`**

`ls -l` needs file sizes and permissions. For each inode returned by `readdir`, the kernel calls `getattr()`:

```rust
fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
    // Look up in the registry
    let registry = self.inode_registry.read().unwrap();
    if let Some(node) = registry.get(&ino) {
        let guard = node.read().unwrap();
        reply.attr(&TTL, &guard.attr);  // Return the FileAttr
    } else {
        reply.error(ENOENT);
    }
}
```

**Step 3 — User sees output**:
```
.snapshots/  @tags/  .tagfs_ctl  documents/  report.txt
```

---

### Error codes: How you say "no"

Your code uses POSIX error codes from the `libc` crate. The important ones:

| Code | Name | Meaning | When you use it |
|------|------|---------|----------------|
| 2 | `ENOENT` | No such file or directory | File not found in lookup |
| 5 | `EIO` | I/O error | Database or storage failure |
| 21 | `EISDIR` | Is a directory | Tried to read() a directory |
| 22 | `EINVAL` | Invalid argument | Bad input (empty snapshot name, etc.) |
| 30 | `EROFS` | Read-only filesystem | Tried to write to a snapshot |
| 17 | `EEXIST` | Already exists | Snapshot name already taken |

---

### TTL: The kernel's cache timer

Notice `TTL` appears everywhere:
```rust
const TTL: Duration = Duration::from_secs(1);

reply.entry(&TTL, &attr, 0);  // "These attributes are valid for 1 second"
```

This tells the kernel: "You can cache this answer for 1 second. After that, ask me again." Without this, the kernel would call `getattr()` on every single file operation, which is slow over FUSE. With a 1-second TTL, repeated `ls` calls within a second don't re-enter your code.

---

### Recap — The mental model

```
Normal filesystem:
  App → Kernel → ext4 (in kernel) → Disk

FUSE filesystem:
  App → Kernel → FUSE module → /dev/fuse → YOUR PROGRAM → Disk
                                    ↑
                              Extra round-trip
                              = "FUSE tax"
```

Your program is essentially a **server**. It sits in a loop, receives requests from the kernel, processes them, and sends back responses. The `fuser` library handles the communication; you just implement the `Filesystem` trait methods.

---

### Check your understanding

Before moving on:

1. Why is FUSE slower than a kernel filesystem? What exactly causes the overhead?
2. What happens if your `read()` method crashes? Does the whole machine crash?
3. When `ls mnt/` runs, which two FUSE methods get called and in what order?
4. What does `reply.error(ENOENT)` communicate back to the user program?
5. What does `fuser::mount2()` do and why does it block?

Say "next" for Stage 3 (Hashing & Content-Addressable Storage).

User: next

GitHub Copilot: 

## Stage 3: Hashing & Content-Addressable Storage

### Start from the concept: What is a hash?

A **hash function** takes input of ANY size and produces a fixed-size output. Think of it as a fingerprint machine:

```
Input: "Hello"           → SHA-256 → "185f8db32271fe25f561a6fc938b2e26..."  (64 hex chars)
Input: "Hello World"     → SHA-256 → "a591a6d40bf420404a011733cfb7b190..."  (64 hex chars)
Input: (entire 4GB movie)→ SHA-256 → "7f83b1657ff1fc53b92dc18148a1d65a..."  (64 hex chars)
```

Three critical properties:

**1. Deterministic**: Same input → same output. Always. Run it a million times, you get the same hash. This is what makes deduplication possible.

**2. Avalanche effect**: Change ONE bit of input → output changes completely.
```
SHA-256("Hello")  → "185f8db3..."
SHA-256("Hellp")  → "1bc1a691..."   ← Totally different. Not "similar".
```

**3. Collision resistant**: It's astronomically unlikely that two different inputs produce the same hash. SHA-256 has $2^{256}$ possible outputs. That's more than the number of atoms in the observable universe. You'd need to hash about $2^{128}$ inputs before getting a 50% chance of a collision (birthday problem). In practice, we treat it as impossible.

---

### The key insight: Hash as an address

Traditional storage uses filenames as addresses:
```
/home/john/report.txt → data bytes
```

**Content-Addressable Storage (CAS)** uses the hash of the data as the address:
```
SHA-256(data bytes) → "a591a6d4..."
Store at: cas/a5/91a6d4...
```

The content itself determines where it's stored. This is the single idea that makes everything else work.

---

### Why CAS enables deduplication (for free)

Imagine you store two files:

```
File A: "The quick brown fox"
File B: "The quick brown fox"   ← Exact same content
```

With traditional storage:
```
/files/A.txt → "The quick brown fox"     (20 bytes on disk)
/files/B.txt → "The quick brown fox"     (20 bytes on disk)
Total: 40 bytes
```

With CAS:
```
SHA-256("The quick brown fox") = "d7a8fbb..."

Step 1: Store A → hash = "d7a8fbb..." → write to cas/d7/a8fbb...  → 20 bytes written
Step 2: Store B → hash = "d7a8fbb..." → file cas/d7/a8fbb... already exists → SKIP!
Total: 20 bytes
```

Deduplication isn't a feature you "add" — it's a **natural consequence** of content-addressing. If the same data already exists, the hash will be the same, the file will already be on disk, and you simply don't write again.

---

### Now look at YOUR code

Open storage.rs. The entire file is ~100 lines. Let's walk through `write_chunk()`:

```rust
pub fn write_chunk(&self, data: &[u8]) -> Result<String, std::io::Error> {
    // STEP 1: Calculate SHA-256 hash of the raw data
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let hash_string = hex::encode(result);   // e.g., "a591a6d40bf420..."

    // STEP 2: Determine the file path (sharding by first 2 hex chars)
    let subdir = self.root_dir.join("cas").join(&hash_string[0..2]);
    let file_path = subdir.join(&hash_string[2..]);
    // So hash "a591a6d4..." → cas/a5/91a6d4...

    // STEP 3: Deduplication — does this chunk already exist?
    if file_path.exists() {
        return Ok(hash_string);  // Already stored! Skip. Free dedup.
    }

    // STEP 4: Compress the data with zstd (level 3)
    let compressed_data = zstd::encode_all(data, 3)?;

    // STEP 5: Write compressed bytes to disk
    fs::create_dir_all(&subdir)?;
    let mut file = File::create(&file_path)?;
    file.write_all(&compressed_data)?;

    Ok(hash_string)  // Return the hash as the "address"
}
```

That's the entire write path. Five steps. Let's understand each:

---

### Step 1: Why SHA-256 specifically?

There are many hash functions. Why SHA-256?

| Hash | Output size | Speed | Security |
|------|------------|-------|----------|
| MD5 | 128-bit | Fast | **Broken** — collisions found |
| SHA-1 | 160-bit | Fast | **Broken** — Google found a collision in 2017 |
| SHA-256 | 256-bit | Moderate | No known collisions. Used by Bitcoin, Git |
| BLAKE3 | 256-bit | Fastest | Newer, equally secure |

SHA-256 is the standard choice. Secure enough to be collision-free in practice, fast enough for filesystem use, and universally supported.

If asked "why not BLAKE3?" — SHA-256 is the conservative, widely-understood choice. BLAKE3 would be a valid optimization.

---

### Step 2: Why sharding (the `cas/a5/` subdirectory)?

If you stored all chunks as flat files:
```
cas/a591a6d40bf420...
cas/7f83b1657ff1fc...
cas/d7a8fbb307d778...
... (100,000 more files)
```

Most operating systems slow down dramatically when a single directory has more than ~10,000 files. The directory listing becomes an $O(n)$ scan.

Sharding splits by the first 2 hex characters:
```
cas/a5/91a6d40bf420...
cas/7f/83b1657ff1fc...
cas/d7/a8fbb307d778...
```

2 hex chars = 16 × 16 = **256 possible subdirectories**. If you have 256,000 chunks, each subdirectory holds ~1,000 files. Much faster to scan.

This is exactly what Git does with its objects directory too.

---

### Step 3: The dedup check

```rust
if file_path.exists() {
    return Ok(hash_string);  // Skip!
}
```

This is the entire deduplication logic. One `if` statement. Because the path IS the hash, and the hash IS determined by content, identical content will always hit this early return. No database query needed. No comparison of bytes. Just a filesystem `exists()` check.

---

### Step 4: Why compress AFTER hashing?

This is a critical design decision and a likely viva question. The order is:

```
Raw data → Hash (SHA-256) → Compress (zstd) → Write to disk
```

**Why not compress first?**

If you compressed first:
```
Compress("Hello World") → [some compressed bytes]  → SHA-256 → "hash_X"
```

Problem: compression is not always deterministic across versions, settings, or implementations. Even if it were, you'd be hashing compressed bytes instead of raw bytes. Two systems with different zstd versions might compress the same data differently → different hashes → dedup broken.

By hashing raw data first, you guarantee that identical content ALWAYS produces the same hash, regardless of compression settings.

**But then why compress at all?**

To save disk space. A 4KB text chunk might compress to 1KB. You store 1KB on disk but use the 4KB-based hash as the address. Best of both worlds.

---

### Step 5: Reading back

Now `read_chunk()` in storage.rs:

```rust
pub fn read_chunk(&self, hash: &str) -> Result<Vec<u8>, std::io::Error> {
    // Find the file by hash
    let subdir = self.root_dir.join("cas").join(&hash[0..2]);
    let file_path = subdir.join(&hash[2..]);

    // Read compressed bytes from disk
    let mut file = File::open(file_path)?;
    let mut compressed_data = Vec::new();
    file.read_to_end(&mut compressed_data)?;

    // Decompress and return raw bytes
    let raw_data = zstd::decode_all(&compressed_data[..])?;
    Ok(raw_data)
}
```

Exact reverse: find by hash → read compressed → decompress → return raw. Simple.

---

### How CAS connects to the filesystem

Here's where it fits in the bigger picture. A file is NOT stored as one blob. It's stored as multiple **chunks** (Stage 4 will explain how chunks are created). Each chunk is stored separately in CAS:

```
File "video.mp4" (12KB)
  │
  ├── Chunk 1 (4KB)  → SHA-256 → "abc123..." → cas/ab/c123...
  ├── Chunk 2 (4KB)  → SHA-256 → "def456..." → cas/de/f456...
  └── Chunk 3 (4KB)  → SHA-256 → "789abc..." → cas/78/9abc...

Recipe stored in sled: ino_recipe:42 → ["abc123...", "def456...", "789abc..."]
```

To reconstruct the file: fetch each hash from CAS in order → decompress each → concatenate → original file.

Now imagine another file "video_copy.mp4" with identical content:
- Same chunks → same hashes → CAS says "already exists" for all three → zero new disk writes
- Recipe: `ino_recipe:43 → ["abc123...", "def456...", "789abc..."]` (same hashes, different inode)
- Only the recipe (a few hundred bytes of metadata) is new. The 12KB of data is stored once.

---

### The two remaining storage methods

```rust
pub fn list_all_chunks(&self) -> Result<Vec<String>, std::io::Error> {
    // Walk cas/ → for each subdir → for each file → reconstruct full hash
    // Used by garbage collection to find ALL chunks on disk
}

pub fn delete_chunk(&self, hash: &str) -> Result<(), std::io::Error> {
    // Delete cas/{hash[0..2]}/{hash[2..]}
    // Used by garbage collection to remove orphaned chunks
}
```

These support GC. `list_all_chunks()` tells you everything on disk. Compare against what's referenced in recipes → anything not referenced is an orphan → delete it.

---

### Compression details (for if they ask)

**zstd** (Zstandard) by Facebook/Meta:
- Level 3 = balanced speed/ratio (levels go 1-22, higher = smaller output but slower)
- Typical text compression: 3:1 to 5:1 ratio
- Typical binary/media: 1.1:1 to 2:1 (already compressed data barely shrinks)
- Decompression is MUCH faster than compression (asymmetric — good for read-heavy workloads)

---

### Recap — The mental model

```
                    Hash determines location
                    ┌──────────────────────┐
Raw bytes ──SHA-256──▶ "a591a6d4..."       │
                    │                      │
                    │  Exists on disk?      │
                    │  YES → skip (dedup!) │
                    │  NO  → compress      │
                    │        → write to    │
                    │    cas/a5/91a6d4...   │
                    └──────────────────────┘
```

Key principles:
1. **Hash = address**. Content determines location.
2. **Same content = same hash = same location**. Dedup is automatic.
3. **Hash raw, THEN compress**. Preserves dedup across compression changes.
4. **Shard directories** by first 2 hex chars. Prevents OS directory bloat.
5. **Immutable**. Once written, a chunk is never modified. Overwriting a file creates NEW chunks; old ones stay until GC.

---

### Check your understanding

1. You store `"Hello World"` twice. How many bytes are written to disk the second time?
2. Why would hashing after compression break deduplication?
3. What is the `cas/a5/` prefix in the path? Why not just `cas/a591a6d4...`?
4. A chunk is stored compressed on disk. How do you verify its integrity later?
5. What does "immutable, append-only" mean for CAS? Why is this important for snapshots?

Say "next" for Stage 4 (Content-Defined Chunking — the most technically interesting part).

User: next

GitHub Copilot: 

## Stage 4: Content-Defined Chunking

### The problem: Why not hash entire files?

From Stage 3, you know that CAS gives you dedup for free: same data → same hash → stored once. So why not just hash entire files?

Imagine a 1GB video. You store it. Hash = `"abc123..."`. Now you edit 1 second of the video — maybe 10KB of actual change. Re-hash the entire 1GB file → completely different hash `"xyz789..."`. CAS sees a new hash → stores the entire 1GB again.

**You changed 10KB but stored 1GB. That's a 100,000x overhead.**

The fix: don't hash whole files. Break them into small **chunks** (a few KB each), hash each chunk independently.

```
Original file (1GB):
[chunk1][chunk2][chunk3]...[chunk250000]
  hash_A  hash_B  hash_C      hash_Z

Edit 10KB in chunk 47:
[chunk1][chunk2]...[chunk47_modified]...[chunk250000]
  hash_A  hash_B     hash_NEW            hash_Z

Only chunk 47 changed → only 1 new hash → only ~4KB re-stored
The other 249,999 chunks dedup perfectly
```

This is the fundamental principle. Now the question is: **how do you decide where to split?**

---

### Approach 1: Fixed-size chunking (simple but broken)

The obvious approach: split every 4096 bytes.

```
Byte 0        4096      8192      12288
│─── chunk1 ───│─── chunk2 ───│─── chunk3 ───│
```

This works fine until someone **inserts** data. Say you insert 1 byte at position 0:

```
BEFORE insertion:
Position:  0    4096    8192    12288
           │─ C1 ─│─ C2 ─│─ C3 ──│

AFTER inserting 1 byte at position 0:
Position:  0  1    4097    8193    12289
           │X│─ C1' ─│─ C2' ─│─ C3' ──│
                 ↑         ↑         ↑
          All boundaries shifted by 1 byte!
          ALL chunks contain different bytes!
          ALL hashes change!
          ZERO deduplication!
```

One byte inserted → every single chunk boundary shifts → every chunk changes → every hash changes → you re-store the entire file. This defeats the purpose.

---

### Approach 2: Content-Defined Chunking (CDC)

Instead of splitting at fixed positions, split based on **what the data contains**. The boundary depends on the content at that location, not on the position.

The idea: slide a window over the data. At each position, compute a hash of the local content. If the hash meets a certain condition → declare a boundary here.

```
Data:  [a][b][c][d][e][f][g][h][i][j][k][l][m][n]...
        ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓
Hash:   7  3  9  1  0  4  8  0  2  5  ...
                     ↑           ↑
              "hash == 0? CUT!"  "hash == 0? CUT!"
        │── chunk 1 ──│── chunk 2 ──│── chunk 3...
```

Now what happens when you insert 1 byte at position 0?

```
BEFORE:
[a][b][c][d][e][f][g][h][i][j][k]...
                  ↑           ↑
               boundary    boundary

AFTER inserting [X] at position 0:
[X][a][b][c][d][e][f][g][h][i][j][k]...
     boundaries shift locally...
                     ↑           ↑
                  boundary    boundary
                  (same content here → same boundary!)
```

The insertion disrupts boundaries near position 0, but once the rolling hash "resynchronizes" with the original data (which happens within a few bytes), boundaries settle back to the same places. Only 1-2 chunks change. All other chunks remain identical → hashes match → dedup works.

**This is the key insight of CDC**: boundaries are determined by local content, so local edits cause only local boundary changes.

---

### How your Gear Hash works

There are several rolling hash algorithms for CDC. The classic is Rabin-Karp (polynomial rolling hash). Your project uses **Gear Hash** from the FastCDC paper, which is much faster.

Open chunker.rs. Let's go line by line.

**The Gear Table** — a lookup table of 256 random numbers:

```rust
const fn build_gear_matrix() -> [u64; 256] {
    let mut out = [0u64; 256];
    let mut i = 0;
    let mut seed = 0x1234_5678_9ABC_DEF0u64;

    while i < 256 {
        seed = splitmix64(seed ^ (i as u64).wrapping_mul(0x9E3779B97F4A7C15));
        out[i] = seed;
        i += 1;
    }
    out
}

const GEAR_MATRIX: [u64; 256] = build_gear_matrix();
```

This creates 256 pseudo-random 64-bit numbers, one for each possible byte value (0-255). The `const fn` and `const` mean this is computed **at compile time** — zero runtime cost. The values are baked into your binary.

If asked "why SplitMix64?": it's a fast, high-quality PRNG for generating the table. The choice doesn't affect chunking behavior — any well-distributed random table works. What matters is that the table is fixed (deterministic).

**The hash update** — one line:

```rust
pub fn feed_byte(&mut self, new_byte: u8) {
    self.hash = (self.hash << 1).wrapping_add(GEAR_MATRIX[new_byte as usize]);
}
```

Let's break this down operation by operation:

1. `self.hash << 1` — shift all bits left by 1 position. The leftmost bit falls off, a 0 enters from the right. This is like multiplying by 2.

2. `GEAR_MATRIX[new_byte as usize]` — look up the random number for this byte value. If the byte is `65` (ASCII 'A'), you get `GEAR_MATRIX[65]`, which is some 64-bit random number.

3. `.wrapping_add(...)` — add them together. "Wrapping" means if the result overflows 64 bits, it wraps around (no crash, no error — standard behavior for hashes).

That's the entire hash computation per byte. **One shift, one lookup, one add.** Compare to Rabin-Karp, which needs multiply + add + modulo per byte. Gear Hash is roughly 3x faster in practice.

Why does this work as a "rolling" hash? The `<< 1` shift means old bytes gradually lose influence. After 64 bytes, the oldest byte's contribution has been shifted out entirely. So the hash effectively captures a window of the most recent ~64 bytes.

**The boundary decision**:

```rust
pub fn should_cut(&self, current_chunk_size: usize) -> bool {
    if current_chunk_size < MIN_CHUNK_SIZE {    // < 2KB? Never cut.
        return false;
    }

    if current_chunk_size >= MAX_CHUNK_SIZE {    // ≥ 64KB? Always cut.
        return true;
    }

    (self.hash & MASK) == 0    // Check bottom 12 bits
}
```

Three rules, in priority order:

**Rule 1: Minimum chunk size (2KB)**. Never create a chunk smaller than 2KB. Tiny chunks have too much metadata overhead per byte of actual data (each chunk needs a 64-char hash stored in the recipe).

**Rule 2: Maximum chunk size (64KB)**. If we haven't found a natural boundary in 64KB, force-cut. This prevents pathological inputs (like all zeros) from creating one enormous chunk.

**Rule 3: The mask check**. `MASK = TARGET_CHUNK_SIZE - 1 = 4096 - 1 = 4095 = 0x0FFF`.

In binary: `MASK = 0000...0000111111111111` (twelve 1s).

`hash & MASK` extracts the bottom 12 bits of the hash.

`(hash & MASK) == 0` checks: are ALL bottom 12 bits zero?

Since each bit is roughly random (50/50 chance of 0 or 1), the probability is:

$$P = \frac{1}{2^{12}} = \frac{1}{4096}$$

So on average, this condition is true once every 4096 bytes → **average chunk size ≈ 4KB**.

This is elegant: you control the average chunk size just by changing the mask. Want 8KB chunks? Use `MASK = 8191` (13 bits). Want 16KB? `MASK = 16383` (14 bits).

---

### The chunking loop: Putting it all together

```rust
pub fn chunk_lengths(data: &[u8]) -> Vec<usize> {
    let mut chunker = Chunker::new();
    let mut sizes = Vec::new();
    let mut current_size = 0usize;

    for &byte in data {
        chunker.feed_byte(byte);         // Update rolling hash
        current_size += 1;

        if chunker.should_cut(current_size) {  // Boundary?
            sizes.push(current_size);     // Record this chunk's size
            chunker.reset();              // Reset hash to 0
            current_size = 0;             // Start counting next chunk
        }
    }

    if current_size > 0 {
        sizes.push(current_size);         // Don't forget the tail chunk
    }

    sizes
}
```

Walk through it mentally with 10KB of data:
1. Bytes 0-3500: hash never hits zero → keep going
2. Byte 3501: `(hash & 0xFFF) == 0` → CUT! Chunk 1 = 3501 bytes. Reset hash.
3. Bytes 3502-7800: hash never hits zero → keep going
4. Byte 7801: `(hash & 0xFFF) == 0` → CUT! Chunk 2 = 4299 bytes. Reset hash.
5. Bytes 7802-10239: file ends before next boundary → tail chunk = 2438 bytes.

Result: `[3501, 4299, 2438]` — three variable-sized chunks averaging ~3.4KB.

---

### Where chunking plugs into the system

Chunking is called from file_manager.rs `create_recipe_from_data()`:

```rust
fn create_recipe_from_data(&self, data: &[u8]) -> Result<FileRecipe, String> {
    let mut chunker = Chunker::new();
    let mut recipe = Vec::new();           // Will hold the ordered hash list
    let mut current_chunk_buffer = Vec::new();

    for &byte in data {
        current_chunk_buffer.push(byte);
        chunker.feed_byte(byte);

        if chunker.should_cut(current_chunk_buffer.len()) {
            // Chunk complete! Store it in CAS.
            let hash = self.storage.write_chunk(&current_chunk_buffer)?;
            recipe.push(hash);             // Add hash to recipe
            current_chunk_buffer.clear();   // Start fresh
            chunker.reset();
        }
    }

    // Handle the tail (last chunk that didn't hit a boundary)
    if !current_chunk_buffer.is_empty() {
        let hash = self.storage.write_chunk(&current_chunk_buffer)?;
        recipe.push(hash);
    }

    Ok(FileRecipe {
        file_size: total_size,
        chunks: recipe,                    // Ordered list of hashes
        kind: FileKind::File,
    })
}
```

This is the function that ties Stage 3 and Stage 4 together. For each chunk, it:
1. Feeds bytes into the Gear Hash chunker
2. When a boundary is found → sends the chunk to `storage.write_chunk()` (SHA-256 + dedup + compress + store)
3. Collects the returned hash into the recipe

The recipe is then saved to sled as `ino_recipe:{inode_id}`.

---

### Visual summary

```
  File data (10KB)
  ┌──────────────────────────────────────┐
  │ bytes bytes bytes bytes bytes bytes  │
  └──┬────────┬──────────┬───────────────┘
     │        │          │
  Gear Hash finds      Gear Hash finds     End of file
  boundary here       boundary here
     │        │          │
     ▼        ▼          ▼
  ┌──────┐┌────────┐┌────────┐
  │Chunk1││Chunk 2 ││Chunk 3 │
  │3.5KB ││ 4.3KB  ││ 2.2KB  │
  └──┬───┘└───┬────┘└───┬────┘
     │        │         │
  SHA-256  SHA-256   SHA-256
     │        │         │
     ▼        ▼         ▼
  "a1b2.." "c3d4.." "e5f6.."
     │        │         │
  ┌──┴────────┴─────────┴──┐
  │  Recipe for inode 42:   │
  │  ["a1b2..", "c3d4..",   │
  │   "e5f6.."]             │
  │  size: 10000            │
  └─────────────────────────┘
```

---

### The test: Proof it works

Look at the unit test in chunker.rs:

```rust
#[test]
fn test_chunking_consistency() {
    // Generate 100KB of pseudo-random data
    let data: Vec<u8> = (0u32..100_000)
        .map(|i| {
            let x = i.wrapping_mul(1103515245).wrapping_add(12345);
            ((x / 65536) % 256) as u8
        })
        .collect();

    // Feed through chunker, record cut points
    // ...

    assert!(cut_points.len() > 5,
        "Statistically unlikely to have fewer than 5 chunks in 100KB data");
}
```

100KB ÷ 4KB average = ~25 expected chunks. The test conservatively checks for > 5. The `println!` outputs the actual average chunk size, which should be close to 4KB.

---

### What to say if asked...

**"Why not just use fixed-size chunks?"**
→ Fixed-size boundaries shift when data is inserted or deleted, destroying deduplication across edits. Content-defined boundaries resynchronize naturally after local changes.

**"Why Gear Hash over Rabin-Karp?"**
→ Rabin-Karp uses polynomial modular arithmetic (multiply + modulo) per byte. Gear Hash uses a bit shift + table lookup + addition — all constant-time CPU operations with much smaller constants. In practice, Gear Hash is roughly 3x faster. The academic reference is the FastCDC paper (Wen Xia et al., 2016).

**"How do you control the average chunk size?"**
→ The MASK. `MASK = TARGET_SIZE - 1`. The probability of `(hash & MASK) == 0` is `1/TARGET_SIZE`. So the average distance between boundaries equals the target size. Making TARGET_SIZE a power of 2 makes the mask a clean bitmask.

**"What if the data is all zeros?"**
→ The Gear Hash would produce the same sequence of hash values repeatedly, which could create very regular boundaries. But the MAX_CHUNK_SIZE (64KB) guard ensures no chunk exceeds 64KB regardless. The MIN_CHUNK_SIZE (2KB) prevents excessively tiny chunks if boundaries fire too frequently.

**"Is Gear Hash cryptographic?"**
→ No. It's not meant to be. It's a fast, non-cryptographic hash used only for finding chunk boundaries. The actual content integrity hash is SHA-256 (done separately in storage.rs). They serve completely different purposes.

---

### Check your understanding

1. You have a 100KB file. You insert 5 bytes at position 50,000. With fixed-size 4KB chunks, how many chunks change? With CDC?
2. `MASK = 4095`. What is the expected average chunk size? What if `MASK = 16383`?
3. Walk through what `feed_byte(65)` does to the hash state.
4. Why is there a minimum chunk size? What goes wrong without it?
5. Why does `chunker.reset()` set hash to 0 after each boundary?

Say "next" for Stage 5 (the Recipe pipeline — how read/write tie everything together).

User: next

GitHub Copilot: 

## Stage 5: The File Recipe & Read/Write Pipeline

### What is a recipe?

From the previous stages, you know that a file is broken into chunks, each chunk is hashed and stored in CAS. But how do you put the file back together? You need an ordered list of chunk hashes. That's the **recipe**.

```rust
// From file_manager.rs
pub struct FileRecipe {
    pub file_size: u64,          // Total size in bytes
    pub chunks: Vec<String>,     // Ordered list of SHA-256 hashes
    pub kind: FileKind,          // File or Directory
}
```

Example:
```
Recipe for inode 42 (report.txt, 10KB):
  file_size: 10240
  chunks: ["a1b2c3...", "d4e5f6...", "789abc..."]
  kind: File
```

To reconstruct the file:
1. Fetch `cas/a1/b2c3...` → decompress → 3500 raw bytes
2. Fetch `cas/d4/e5f6...` → decompress → 4300 raw bytes
3. Fetch `cas/78/9abc...` → decompress → 2440 raw bytes
4. Concatenate: 3500 + 4300 + 2440 = 10240 bytes. Done.

The recipe is stored in sled: `ino_recipe:42 → FileRecipe (bincode-serialized)`

---

### The full write journey: From `echo` to disk

Let's trace what happens when you type `echo "Hello World" > mnt/newfile.txt` against your mounted filesystem. This touches all four layers.

**Phase 1: File creation** — FUSE calls `create(parent=1, name="newfile.txt")`

Your code in fuse_handler.rs `create()`:
```
1. Check parent isn't a snapshot or tag dir → OK
2. Call create_live_file_under_parent(parent=1, name="newfile.txt")
   a. Generate new inode ID (say 100)
   b. Create Inode struct: id=100, parent=1, name="newfile.txt", kind=RegularFile
   c. Save to sled: ino_meta:100 → InodeMetadata
   d. Save dirent: dirent:1:newfile.txt → 100
   e. Save empty recipe: ino_recipe:100 → {size:0, chunks:[], kind:File}
   f. Auto-tag from parent ancestry (if parents have names)
   g. Insert into in-memory tree: root.children["newfile.txt"] = Arc(inode)
   h. Insert into inode_registry: {100 → Arc(inode)}
3. Reply to kernel: "Created, here are the attributes"
```

At this point, the file exists in the namespace (metadata), but contains zero bytes.

**Phase 2: Writing data** — FUSE calls `write(ino=100, offset=0, data="Hello World\n")`

Your code in fuse_handler.rs `write()`:
```
1. Resolve target inode (handle virtual files, tag files, etc.)
2. Put data into the page cache:
   ┌─────────────────────────────────────────────────┐
   │  page_cache: {                                   │
   │    100: (b"Hello World\n", dirty=true)           │
   │  }                                               │
   └─────────────────────────────────────────────────┘
3. Touch LRU (move inode 100 to back of eviction queue)
4. Check memory pressure (evict old entries if cache > 1024)
5. Update inode attributes: size=12, blocks=1, mtime=now
6. Reply to kernel: "Wrote 12 bytes" ← FAST! No disk I/O happened!
```

Notice: **no chunking, no hashing, no compression, no disk writes yet**. The data sits in RAM, marked "dirty". This is the write-back cache strategy.

**Phase 3: Flushing** — FUSE calls `release(ino=100)` when the file handle is closed

Your code in fuse_handler.rs `release()` → calls `flush_inode_cache(100)`:

```
1. Check: is inode 100 dirty in cache? YES
2. Get the buffer: b"Hello World\n" (12 bytes)
3. Call manager.write_file_by_id(100, buffer):
   a. create_recipe_from_data(buffer):
      - Feed each byte through Gear Hash chunker
      - 12 bytes < MIN_CHUNK_SIZE (2KB) → no boundary found
      - Tail chunk: entire 12 bytes is one chunk
      - storage.write_chunk(b"Hello World\n"):
        * SHA-256 → "7509e5bda0c7..."
        * Check cas/75/09e5bda0c7... → doesn't exist
        * Compress with zstd
        * Write to cas/75/09e5bda0c7...
      - Recipe: {size:12, chunks:["7509e5bda0c7..."], kind:File}
   b. save_recipe(100, recipe):
      - sled insert: ino_recipe:100 → recipe (bincode)
4. Save inode metadata to sled
5. Mark cache entry: (buffer, dirty=false)
```

NOW the data is on disk. The pipeline was:
```
RAM buffer → Chunker → SHA-256 → Dedup check → zstd compress → CAS file on disk
                                                                    ↓
                                              Recipe saved to sled (ino_recipe:100)
```

---

### The full read journey: From `cat` to screen

Now: `cat mnt/newfile.txt`

**Step 1**: Kernel calls `lookup(parent=1, name="newfile.txt")` → your code finds inode 100 in registry, returns its attributes.

**Step 2**: Kernel calls `open(ino=100)` → your code returns `opened(0, 0)` (no special flags).

**Step 3**: Kernel calls `read(ino=100, offset=0, size=4096)`

Your code in fuse_handler.rs `read()`:

```
1. Resolve target inode (virtual file check, etc.)

2. Is inode 100 in the page cache?
   ├── YES → serve directly from cache buffer
   └── NO  → load from disk:
       a. manager.read_file_by_id(100):
          * Load recipe from sled: ino_recipe:100
          * For each hash in recipe.chunks:
              - storage.read_chunk("7509e5bda0c7..."):
                  * Open cas/75/09e5bda0c7...
                  * Read compressed bytes
                  * zstd decompress
                  * Return raw bytes
          * Concatenate all chunk bytes → full file data
       b. Insert into page cache: {100: (data, dirty=false)}
       c. Trigger eviction if cache is over capacity

3. Touch LRU entry (mark as recently used)

4. Serve the requested slice:
   start = offset = 0
   end = min(start + size, data.len()) = min(4096, 12) = 12
   reply.data(&data[0..12])  → "Hello World\n"
```

**Step 4**: Kernel calls `release(ino=100)` → data is clean (dirty=false) → no flush needed.

---

### The page cache in detail

The cache is defined in fuse_handler.rs:

```rust
pub page_cache: Arc<RwLock<HashMap<u64, (Vec<u8>, bool)>>>,
pub cache_lru: Arc<RwLock<VecDeque<u64>>>,
pub cache_capacity: usize,  // 1024
```

Two data structures work together:

**`page_cache`** — the actual data:
```
HashMap {
    42:  (b"contents of file 42...", false),   // clean
    100: (b"Hello World\n",          true),    // dirty (modified, not yet saved)
    77:  (b"some other file...",     false),   // clean
}
```

**`cache_lru`** — tracks access order (Least Recently Used):
```
VecDeque: [42, 77, 100]
           ↑            ↑
       oldest        newest
     (evict first)  (evict last)
```

When you access a file, `touch_cache_entry()` moves it to the back:
```rust
fn touch_cache_entry(&self, ino: u64) {
    let mut lru = self.cache_lru.write().unwrap();
    if let Some(pos) = lru.iter().position(|id| *id == ino) {
        lru.remove(pos);      // Remove from current position
    }
    lru.push_back(ino);       // Add to back (most recent)
}
```

When the cache exceeds 1024 entries, `evict_under_pressure()` kicks in:
```rust
fn evict_under_pressure(&self) -> Result<(), String> {
    loop {
        if cache.len() <= self.cache_capacity { return Ok(()); }

        // Pop the oldest entry from LRU
        let victim = lru.pop_front();

        // If it's dirty, flush it to disk first!
        if is_dirty {
            self.flush_inode_cache(victim)?;
        }

        // Remove from cache
        cache.remove(&victim);
    }
}
```

Key insight: dirty entries are **flushed before eviction**, never just discarded. This prevents data loss.

---

### Write-back vs Write-through: Why it matters

**Write-through**: Every write goes to disk immediately. Simple. Safe. Slow.
```
write() → chunk → hash → compress → disk write → reply "done"
         └──── all of this on EVERY 4KB FUSE write ────┘
```

**Write-back** (what ArcFS uses): Writes go to RAM. Disk write happens later.
```
write() → put in RAM → reply "done"     ← FAST (microseconds)

... later, on release()/fsync() ...

flush() → chunk → hash → compress → disk write    ← Batched
```

Why is write-back so much better?

Imagine writing a 10MB file. FUSE sends it as ~2,500 separate 4KB `write()` calls.

With **write-through**: 2,500 × (chunk + hash + compress + disk write) = 2,500 disk operations.

With **write-back**: 2,500 × (copy 4KB into RAM buffer) = 2,500 memory copies (microseconds each). Then ONE flush at the end: chunk the full 10MB → ~2,500 chunks → hash + compress + write each. Same number of disk writes, but you eliminate the per-call overhead and can do them sequentially (faster for SSDs).

The tradeoff: if your program crashes before flush, dirty data is lost. But this is the same tradeoff every OS makes (your laptop's RAM is a write-back cache for the SSD). For explicit safety, apps call `fsync()`, which triggers `flush_inode_cache()`.

---

### The in-place mutation optimization

Look at how `write()` handles data in fuse_handler.rs:

```rust
let end = (offset as usize) + data.len();
{
    let mut cache_map = self.page_cache.write().unwrap();
    let entry = cache_map
        .entry(target_ino)
        .or_insert_with(|| {
            // If not in cache, load from disk first
            (self.manager.read_file_by_id(target_ino).unwrap_or_default(), false)
        });

    if end > entry.0.len() {
        entry.0.resize(end, 0);    // Grow buffer if needed
    }
    entry.0[offset as usize..end].copy_from_slice(data);  // Overwrite in-place
    entry.1 = true;  // Mark dirty
}
```

This is the $O(1)$ in-place mutation described in OPTIMIZATIONS.md. The old code used to **clone** the entire buffer on every write. For a 2GB file with 4KB writes, that's:

$$\sum_{i=1}^{n} i \times 4\text{KB} \approx O(n^2) \text{ memory bandwidth}$$

The fix: get a mutable reference to the existing Vec, `resize()` if needed, then `copy_from_slice()`. The buffer is modified in-place. No cloning. The lock is held for microseconds (just the memcpy duration).

---

### How flush connects recipes to CAS

When `flush_inode_cache()` fires in fuse_handler.rs:

```rust
fn flush_inode_cache(&self, ino: u64) -> Result<(), String> {
    // 1. Get the dirty buffer (if any)
    let dirty_data = {
        let cache = self.page_cache.read().unwrap();
        match cache.get(&ino) {
            Some((buffer, true)) => Some(buffer.clone()),  // Only clone for flushing
            _ => None,
        }
    };

    if let Some(buffer) = dirty_data {
        // 2. Chunk → hash → compress → store → save recipe
        self.manager.write_file_by_id(ino, &buffer)?;

        // 3. Persist inode metadata (updated size, mtime)
        if let Some(node_arc) = self.inode_registry.read().unwrap().get(&ino).cloned() {
            let node = node_arc.read().unwrap();
            self.manager.save_inode(&node)?;
        }

        // 4. Mark clean
        self.page_cache.write().unwrap().insert(ino, (buffer, false));
    }
    Ok(())
}
```

`write_file_by_id()` in file_manager.rs is just:
```rust
pub fn write_file_by_id(&self, inode_id: u64, data: &[u8]) -> Result<(), String> {
    let recipe = self.create_recipe_from_data(data)?;  // Chunk + hash + store
    self.save_recipe(inode_id, &recipe)                 // Save recipe to sled
}
```

And `read_file_by_id()` in file_manager.rs:
```rust
pub fn read_file_by_id(&self, inode_id: u64) -> Result<Vec<u8>, String> {
    let recipe = self.load_recipe(inode_id)?;           // Load recipe from sled
    let mut data = Vec::new();
    for hash in recipe.chunks {
        let chunk = self.storage.read_chunk(&hash)?;    // Fetch + decompress
        data.extend_from_slice(&chunk);                 // Concatenate
    }
    Ok(data)
}
```

---

### Three flush triggers

Your code has three FUSE callbacks that trigger flushing:

| Callback | When kernel calls it | Your behavior |
|----------|---------------------|---------------|
| `release()` | File handle closed (last close) | Flush this inode's dirty cache |
| `fsync()` | App explicitly calls `fsync()` | Flush this inode's dirty cache |
| `flush()` | Before close (may be called multiple times) | Flush this inode's dirty cache |

Plus one implicit trigger: **LRU eviction**. When the cache is full and a new file needs space, the oldest dirty entry is flushed before removal.

---

### End-to-end data flow diagram

```
USER:    echo "data" > mnt/file.txt

         ┌─────────────┐
         │   write()    │ ← FUSE callback
         │  offset=0    │
         │  data="data" │
         └──────┬───────┘
                │
         ┌──────▼────────────────────┐
         │     PAGE CACHE (RAM)       │
         │  {ino: (b"data", dirty)}   │
         └──────┬────────────────────┘
                │ (on release/fsync)
         ┌──────▼───────┐
         │   CHUNKER     │  Gear Hash → find boundaries
         │  (chunker.rs) │
         └──────┬───────┘
                │ raw chunks
         ┌──────▼──────────────┐
         │   CAS STORAGE        │  SHA-256 → dedup check → zstd → disk
         │   (storage.rs)       │
         └──────┬──────────────┘
                │ chunk hashes
         ┌──────▼──────────────────┐
         │   SLED METADATA DB       │  ino_recipe:{id} → [hash1, hash2, ...]
         │   (file_manager.rs)      │
         └─────────────────────────┘
```

---

### Check your understanding

1. When `write()` is called, does anything get written to disk? Why?
2. What triggers the actual chunking + hashing + CAS storage?
3. What happens if the power goes out while data is in the page cache but hasn't been flushed?
4. How does `evict_under_pressure()` handle a dirty cache entry it needs to evict?
5. Why is the old `clone()`-based write approach $O(N^2)$, while `copy_from_slice` is $O(N)$?
6. What's in a `FileRecipe`? How is it used during read vs write?

Say "next" for Stage 6 (Snapshots & Copy-on-Write — the Chronos system).

User: next

GitHub Copilot: 

## Stage 6: Snapshots & Copy-on-Write (Chronos)

### What is a snapshot?

A snapshot is a frozen-in-time picture of your entire filesystem. You take a snapshot at 2:00 PM. At 3:00 PM you delete files, modify things, create chaos. You can then browse the 2:00 PM snapshot and see exactly how everything looked. You can even restore back to it.

Real-world examples: macOS Time Machine, Windows System Restore, btrfs snapshots, ZFS snapshots, VM snapshots.

---

### The naive approach (and why it's terrible)

The obvious way to snapshot a filesystem: **copy everything**.

```
Filesystem: 50GB of files
Snapshot v1: copy all 50GB → 50GB used
Snapshot v2: copy all 50GB → 50GB used
Snapshot v3: copy all 50GB → 50GB used
Total: 200GB for 50GB of data + 3 snapshots
```

This is $O(n)$ in time AND space per snapshot. Useless for anything practical.

---

### The smart approach: Pointer duplication

Since your data lives in CAS (immutable, content-addressed chunks), here's the key realization:

**You don't need to copy any data. You only need to copy the recipes.**

```
Live filesystem:
  inode 100 (report.txt):
    recipe → ["chunk_A", "chunk_B", "chunk_C"]
                  ↓           ↓          ↓
             cas/ab/...   cas/cd/...  cas/ef/...   (physical chunks on disk)

Snapshot v1 (copies recipes with NEW inode IDs):
  inode 200 (report.txt):
    recipe → ["chunk_A", "chunk_B", "chunk_C"]   ← SAME hashes!
                  ↓           ↓          ↓
             cas/ab/...   cas/cd/...  cas/ef/...   ← SAME physical chunks!
```

Both the live file and the snapshot point to the **same physical chunks**. No data was copied. The snapshot "cost" is just the metadata (a few hundred bytes of inode + recipe per file).

Now you modify the live file — change the middle section:

```
Live filesystem (after edit):
  inode 100 (report.txt):
    recipe → ["chunk_A", "chunk_NEW", "chunk_C"]   ← middle chunk changed
                  ↓           ↓            ↓
             cas/ab/...   cas/99/...    cas/ef/...  ← new chunk in CAS

Snapshot v1 (untouched):
  inode 200 (report.txt):
    recipe → ["chunk_A", "chunk_B", "chunk_C"]      ← still the old recipe
                  ↓           ↓          ↓
             cas/ab/...   cas/cd/...  cas/ef/...     ← old chunks still exist!
```

The old chunk_B is still on disk because the snapshot's recipe references it. GC won't delete it. The live file now has chunk_NEW. Both versions coexist, sharing chunks A and C.

**This is why CAS being immutable matters for snapshots.** You never overwrite a chunk. You only create new ones. Old chunks stay as long as any recipe references them.

---

### How it works in your code: Taking a snapshot

User types: `mkdir mnt/.snap_v1`

FUSE calls `mkdir(parent=1, name=".snap_v1")`. Your code in fuse_handler.rs detects the `.snap_` prefix:

```rust
if let Some(snap_name) = name_str.strip_prefix(".snap_") {
    return match self.create_snapshot_named(snap_name) {
        Ok(_) => reply.entry(&TTL, &dir_attr(9999), 0),
        Err(e) if e.contains("already exists") => reply.error(EEXIST),
        Err(_) => reply.error(libc::EIO),
    };
}
```

Now `create_snapshot_named()` in fuse_handler.rs:

```rust
fn create_snapshot_named(&self, snap_name: &str) -> Result<(), String> {
    // 1. Don't allow duplicate names
    if self.snapshots.read().unwrap().contains_key(snap_name) {
        return Err(format!("Snapshot '{}' already exists", snap_name));
    }

    // 2. CRITICAL: Flush all dirty cache to disk first!
    //    Without this, dirty data in RAM wouldn't be captured
    self.flush_all_dirty_cache()?;

    // 3. Clone the entire metadata tree (NOT the data!)
    let frozen_root = self.clone_subtree_from_metadata(
        FUSE_ROOT_ID,    // Start from root
        SNAPSHOT_DIR_ID,  // Parent of snapshot = .snapshots dir
        snap_name,        // Name of this snapshot
        false             // Don't register in live inode_registry
    )?;

    // 4. Store in memory
    self.snapshots.write().unwrap().insert(snap_name.to_string(), Snapshot {
        name: snap_name.to_string(),
        timestamp,
        root: frozen_root,
    });

    // 5. Persist to sled for crash recovery
    self.manager.save_snapshot(snap_name, unix_timestamp, root_id)?;

    Ok(())
}
```

The key function is `clone_subtree_from_metadata()`. Let's trace it:

```rust
fn clone_subtree_from_metadata(
    &self,
    source_inode_id: u64,   // The inode to clone
    parent_id: u64,         // New parent for the clone
    node_name: &str,        // Name in the new parent
    register_live: bool     // Add to live inode_registry?
) -> Result<Arc<RwLock<Inode>>, String> {
    // 1. Load source inode from sled
    let source_meta = self.manager.load_inode(source_inode_id)?;

    // 2. Create NEW inode with a NEW ID
    let new_id = self.generate_id();
    let mut new_inode = Inode::new(new_id, source_kind);
    new_inode.parent_id = parent_id;
    new_inode.name = node_name.to_string();
    new_inode.attr.size = source_meta.attr.size;

    // 3. Save new inode + dirent to sled
    self.manager.save_inode(&new_inode)?;
    self.manager.save_dirent(parent_id, node_name, new_id)?;

    // 4. For files: COPY THE RECIPE (same chunk hashes!)
    if source_kind == FileType::RegularFile {
        let recipe = self.manager.load_recipe(source_inode_id)?;
        self.manager.save_recipe(new_id, &recipe)?;
        //                              ↑
        //  New inode ID, but SAME chunk hashes inside the recipe
        //  No physical data is copied!
    }

    // 5. RECURSE into children (for directories)
    let children = self.manager.list_dirents(source_inode_id)?;
    for child in children {
        let cloned_child = self.clone_subtree_from_metadata(
            child.child_inode_id,  // Source child
            new_id,                // New parent = our newly created inode
            &child.name,
            register_live
        )?;
        new_inode.children.insert(child.name, cloned_child);
    }

    Ok(Arc::new(RwLock::new(new_inode)))
}
```

Visualize it:

```
BEFORE snapshot:

Live tree (in inode_registry):
  root (ino=1)
  ├── docs/ (ino=10)
  │   └── report.txt (ino=11) → recipe: ["hash_A", "hash_B"]
  └── data/ (ino=20)
      └── log.txt (ino=21) → recipe: ["hash_C"]


AFTER "mkdir .snap_v1":

Live tree (unchanged):
  root (ino=1)
  ├── docs/ (ino=10)
  │   └── report.txt (ino=11) → recipe: ["hash_A", "hash_B"]
  └── data/ (ino=20)
      └── log.txt (ino=21) → recipe: ["hash_C"]

Snapshot tree (separate, in self.snapshots):
  snap_root (ino=102)         ← NEW inode ID
  ├── docs/ (ino=103)         ← NEW inode ID
  │   └── report.txt (ino=104) → recipe: ["hash_A", "hash_B"]  ← SAME hashes!
  └── data/ (ino=105)          ← NEW inode ID
      └── log.txt (ino=106) → recipe: ["hash_C"]               ← SAME hashes!

Physical CAS (unchanged — nothing added or removed):
  cas/ab/hash_A_remainder...
  cas/cd/hash_B_remainder...
  cas/ef/hash_C_remainder...
```

The entire snapshot operation creates only new metadata entries in sled. The CAS doesn't change at all.

---

### Browsing snapshots: Read-only enforcement

When you `ls mnt/.snapshots/v1/`, the kernel calls `readdir(ino=2)` (SNAPSHOT_DIR_ID), then `lookup` and `readdir` on the snapshot root.

Your code handles this throughout `lookup()`, `readdir()`, and `getattr()` by checking if an inode belongs to a snapshot tree. Look at `lookup()` in fuse_handler.rs:

```rust
// Check if parent is inside a snapshot tree
for (_snap_name, snapshot) in snaps.iter() {
    if let Some(node_arc) = self.find_in_snapshot_tree(&snapshot.root, parent) {
        let node = node_arc.read().unwrap();
        if let Some(child_arc) = node.children.get(name_str) {
            let child_guard = child_arc.read().unwrap();
            let mut child_attr = child_guard.attr;
            child_attr.perm = 0o555;  // Read-only! r-xr-xr-x
            return reply.entry(&TTL, &child_attr, 0);
        }
    }
}
```

Two things enforce read-only:
1. **Permissions set to `0o555`** — the kernel shows files as read-only to user tools
2. **Write operations check and reject**: `write()`, `mkdir()`, `create()`, `unlink()` all check `inode_in_snapshot()` and return `EROFS` (Error: Read-Only File System)

---

### Copy-on-Write: The tricky part

Now we get to the hardest concept. You have a live tree and a snapshot tree. What happens when you modify a live file?

**The problem**: In the current design, `clone_subtree_from_metadata()` creates entirely separate inode objects with new IDs. The live tree and snapshot tree DON'T share in-memory `Arc` pointers — they're independent trees. So modifying a live inode doesn't affect the snapshot.

BUT there's a subtlety: what if you want a **cheaper** snapshot that DOES share `Arc` pointers? That's what `get_mutable_inode()` handles. It implements Copy-on-Write for the in-memory tree.

**What is `Arc`?**

`Arc` = Atomic Reference Counted pointer. Multiple variables can "own" the same data.

```rust
let a = Arc::new(RwLock::new(Inode { ... }));
let b = a.clone();  // b and a point to the SAME Inode in memory
                     // Arc::strong_count(&a) == 2
```

If you modify the data through `a`, `b` sees the change too — they share the same memory.

**What is `Arc::strong_count()`?**

It tells you how many `Arc` pointers point to this data. If `strong_count > 1`, someone else also has a pointer (potentially a snapshot).

**The CoW algorithm** — `get_mutable_inode()` in fuse_handler.rs:

Walk the tree from root to the target. At each node:

```
Is Arc::strong_count > 1?
  YES → This node is shared! Someone else (snapshot) holds a reference.
        1. Deep-clone the node with a NEW inode ID
        2. Wrap in a new Arc
        3. Replace the pointer in parent's children map
        4. Register the new ID in inode_registry
        → Snapshot still holds the OLD Arc → sees the OLD data
        → Live tree now has a PRIVATE copy → safe to modify

  NO  → This node is not shared. Safe to modify directly.
```

Let's trace an example:

```
INITIAL STATE (root is shared — strong_count = 2):

Live tree:          Snapshot tree:
root ──Arc──┐   ┌──Arc── snap_root
            ▼   ▼
         [Inode id=1]        ← Both point to same object!
         strong_count = 2
              │
        children["docs"]
              │
              ▼
         [Inode id=10]
         strong_count = 2    ← Also shared
```

Now: `mkdir mnt/docs/newfolder`

1. `get_mutable_inode("docs")` is called
2. Start at root. `strong_count(&root) == 2` → SHARED!
   - Clone root with same ID (special case for root)
   - `self.root = Arc::new(RwLock::new(new_root))`
   - Snapshot still holds old Arc
3. Walk to "docs". `strong_count(&docs_node) == 2` → SHARED!
   - Clone docs node → new ID (say 110)
   - `new_docs_node.id = 110`
   - Register in inode_registry
   - Replace in root.children: `children["docs"] = new_arc`
   - Snapshot still holds old Arc pointing to id=10

```
AFTER CoW:

Live tree:                    Snapshot tree:
self.root ──Arc──▶ [Inode id=1']     snap_root ──Arc──▶ [Inode id=1]
                   strong_count=1                       strong_count=1
                        │                                    │
                  children["docs"]                     children["docs"]
                        │                                    │
                        ▼                                    ▼
                   [Inode id=110]                       [Inode id=10]
                   strong_count=1                       strong_count=1
                   (private copy,                       (old copy,
                    safe to modify)                      frozen)
```

Now `mkdir` can safely add "newfolder" to inode 110's children. Inode 10 (in the snapshot) is untouched.

---

### Snapshot restore

User types: `mkdir mnt/.restore_v1`

Your code in fuse_handler.rs detects the `.restore_` prefix:

```rust
if let Some(snap_name) = name_str.strip_prefix(".restore_") {
    // 1. Auto-backup current state (safety net)
    let backup_name = format!("before_restore_{}", timestamp);
    self.create_snapshot_named(&backup_name)?;

    // 2. Restore snapshot into live tree
    self.restore_live_from_snapshot(snap_name)?;
}
```

`restore_live_from_snapshot()` in fuse_handler.rs:

```rust
fn restore_live_from_snapshot(&self, snap_name: &str) -> Result<(), String> {
    // 1. Get the snapshot's root inode ID
    let snapshot_root_id = ...;

    // 2. Delete all live dirents from root
    let old_root_dirents = self.manager.list_dirents(FUSE_ROOT_ID)?;
    for dirent in old_root_dirents {
        self.manager.delete_dirent(FUSE_ROOT_ID, &dirent.name);
    }

    // 3. Clear the live in-memory tree
    root.children.clear();
    registry.clear();
    registry.insert(FUSE_ROOT_ID, self.root.clone());

    // 4. Clear page cache (all cached data is now stale)
    page_cache.clear();
    cache_lru.clear();

    // 5. Clone snapshot's subtree INTO the live tree (with new IDs)
    let snap_children = self.manager.list_dirents(snapshot_root_id)?;
    for child in snap_children {
        let cloned_child = self.clone_subtree_from_metadata(
            child.child_inode_id,
            FUSE_ROOT_ID,
            &child.name,
            true   // ← register in live inode_registry!
        )?;
        root.children.insert(child.name, cloned_child);
    }
    Ok(())
}
```

This is a destructive operation: it wipes the live tree and replaces it with the snapshot's content. But the auto-backup in step 1 means you can always restore back.

---

### Snapshot persistence: Surviving restarts

Snapshots are stored in both memory and sled:

**In memory**: `self.snapshots: HashMap<String, Snapshot>` — for fast access during FUSE operations.

**In sled**: `snapshot:{name} → SnapshotMetadata {name, timestamp, root_id}` — for persistence.

At startup, `restore_snapshots()` in fuse_handler.rs reads all `snapshot:*` entries from sled and rebuilds the in-memory snapshot trees using `restore_snapshot_subtree()`, which recursively loads inodes and their children from sled.

If a snapshot references an inode that no longer exists in sled (corruption/orphan), the snapshot is pruned:

```rust
Err(e) => {
    eprintln!("[CHRONOS] Failed restoring snapshot '{}': {}", meta.name, e);
    self.manager.delete_snapshot(&meta.name);  // Prune stale snapshot
    continue;
}
```

---

### Snapshot deletion

User types: `rmdir mnt/.snapshots/v1`

Your code in `rmdir()` in fuse_handler.rs:

```rust
if parent == SNAPSHOT_DIR_ID {
    let mut snaps = self.snapshots.write().unwrap();
    if snaps.remove(name_str).is_some() {
        self.manager.delete_snapshot(name_str)?;
        return reply.ok();
    }
    return reply.error(ENOENT);
}
```

This removes the snapshot from memory and sled. The snapshot's inode metadata stays in sled (orphaned), and its CAS chunks may now be unreferenced. GC will clean both up eventually.

---

### Visual summary of the snapshot lifecycle

```
1. CREATE (mkdir .snap_v1)
   Live tree ──clone recipes──▶ Snapshot tree
   CAS: unchanged (both point to same chunks)

2. MODIFY LIVE FILE
   Live: new chunks created → new recipe
   Snap: old recipe still points to old chunks
   CAS: old + new chunks coexist

3. BROWSE (ls .snapshots/v1/)
   Reads from snapshot tree
   Permissions forced to 0o555 (read-only)
   Writes return EROFS

4. RESTORE (mkdir .restore_v1)
   Auto-backup current state
   Clear live tree
   Clone snapshot into live tree

5. DELETE (rmdir .snapshots/v1)
   Remove snapshot from memory + sled
   Old chunks become orphans → GC cleans them
```

---

### What to say if asked...

**"How fast is snapshot creation?"**
→ Proportional to the number of files/directories, NOT the amount of data. A 50GB filesystem with 1000 files creates a snapshot in milliseconds — it only copies ~1000 inode records and their recipes. No data is copied.

**"How much space does a snapshot use?"**
→ Initially: nearly zero (just metadata). Over time: as the live tree diverges, old chunks accumulate. Space = sum of chunks referenced ONLY by snapshots (not shared with live). Worst case: if you completely rewrite every file, snapshot holds a full copy's worth of chunks.

**"What happens to old chunks when a snapshot is deleted?"**
→ They may become orphans (no recipe references them). They sit harmlessly on disk until GC scans all recipes, builds the active set, and deletes unreferenced chunks.

**"Is this the same as btrfs/ZFS snapshots?"**
→ Conceptually yes — both use CoW so snapshots are cheap. btrfs operates at the block level in-kernel (faster, but more complex). Yours operates at the chunk level in userspace (simpler, easier to reason about, but has the FUSE tax).

**"What's the difference between your CoW and btrfs's CoW?"**
→ btrfs uses a B-tree with CoW at the block layer. When you modify a block, it copies the block, modifies the copy, and updates the parent pointer. Your system does the same conceptually: when you modify a shared inode, `get_mutable_inode()` clones it and updates the parent's children map. The principle is identical; the granularity differs (disk blocks vs. inode objects).

---

### Check your understanding

1. When you create a snapshot, how many bytes of file data are copied from CAS?
2. Why do you need to flush dirty cache before creating a snapshot?
3. After a snapshot, you modify `report.txt`. How many copies of the old chunk exist on disk? Where are they referenced?
4. What does `Arc::strong_count() > 1` mean in the context of CoW? Why do you check it?
5. If you take 10 snapshots and then delete 9 of them, what happens to the chunks that only the last remaining snapshot references?
6. Why does restore auto-create a backup snapshot before proceeding?

Say "next" for Stage 7 (TagFS — virtual tag-based directories).

User: next

GitHub Copilot: 

## Stage 7: TagFS — Virtual Tag-Based Directories

### The problem with traditional directories

In a normal filesystem, a file lives in **exactly one place**:

```
/projects/backend/api.rs
```

But what if this file is also related to "work", "2026", and "rust"? You'd have to choose ONE folder. You can't browse your files by tag, topic, or category without manually maintaining symlinks or scripts.

Think of it like a physical filing cabinet. You have one folder labeled "Backend". You put `api.rs` in it. Now you want to find all "Rust" files across all projects — you'd have to open every folder and check each file manually.

**Tags solve this.** A file can have multiple tags simultaneously, and you can browse files by any tag or combination of tags:

```
@tags/work/           → all files tagged "work" (across all folders)
@tags/rust/           → all files tagged "rust"
@tags/work/rust/      → files tagged BOTH "work" AND "rust"
@tags/work/2026/rust/ → files tagged ALL THREE
```

No file is moved. No duplication. The tags are a **view** on top of the existing filesystem.

---

### How TagFS stores tags

Two data structures in sled work together:

**1. Per-file tag set** — "what tags does this file have?"
```
Key:   ino_tags:42
Value: FileTagSet { file_id: 42, filename: "api.rs", tags: ["backend", "rust", "work"] }
```

**2. Inverted index** — "which files have this tag?"
```
Key:   tag_index:work     → Value: [42, 55, 78]
Key:   tag_index:rust     → Value: [42, 91]
Key:   tag_index:backend  → Value: [42, 55]
```

The per-file set answers: "What are api.rs's tags?" → `["backend", "rust", "work"]`

The inverted index answers: "Which files are tagged 'work'?" → `[42, 55, 78]`

You need both. The per-file set is needed when you update or delete a file's tags. The inverted index is needed for fast queries (you don't want to scan every file to find which ones have a tag).

Look at file_manager.rs:
```rust
fn ino_tags_key(inode_id: u64) -> String {
    format!("ino_tags:{}", inode_id)
}

fn tag_index_key(tag: &str) -> String {
    format!("tag_index:{}", tag)
}
```

---

### Tag normalization

Tags are normalized to prevent duplicates. In file_manager.rs:

```rust
fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut dedup = HashSet::new();
    for tag in tags {
        let normalized = tag.trim().to_lowercase();  // "Work" → "work", " Rust " → "rust"
        if !normalized.is_empty() {
            dedup.insert(normalized);                 // HashSet removes duplicates
        }
    }
    let mut out: Vec<String> = dedup.into_iter().collect();
    out.sort();                                       // Canonical ordering
    out
}
```

So `["Work", "WORK", " work "]` → `["work"]`. This prevents "Work" and "work" from being treated as different tags.

---

### Setting tags: The write path

When you tag a file, `set_file_tags()` in file_manager.rs does:

```
Input: set_file_tags(inode_id=42, filename="api.rs", tags=["Work", "Rust", "Backend"])

1. Normalize: ["Work", "Rust", "Backend"] → ["backend", "rust", "work"]

2. Load previous tags for this inode (if any): ["old_tag"]

3. Save the FileTagSet to sled:
   ino_tags:42 → { file_id: 42, filename: "api.rs", tags: ["backend", "rust", "work"] }

4. Update inverted indices:
   - Removed tags (in old but not in new): "old_tag"
     → Load tag_index:old_tag → [42, 55] → remove 42 → [55] → save
   
   - Added tags (in new but not in old): "backend", "rust", "work"
     → Load tag_index:backend → [55] → add 42 → [42, 55] → save
     → Load tag_index:rust → [] → add 42 → [42] → save
     → Load tag_index:work → [78] → add 42 → [42, 78] → save

5. Flush sled
```

The key insight: both the per-file set AND every affected index entry are updated. This keeps them in sync.

---

### Querying tags: AND intersection

When you navigate `@tags/work/rust/`, the system needs: "files tagged BOTH work AND rust."

`get_files_by_tags()` in file_manager.rs:

```rust
pub fn get_files_by_tags(&self, tags: &[String]) -> Result<Vec<u64>, String> {
    let normalized = Self::normalize_tags(tags.to_vec());

    // Start with the first tag's file set
    let first = normalized.first().unwrap();
    let mut candidates = self.load_tag_index_set(first)?;
    // candidates = {42, 55, 78}  (all files tagged "work")

    // Intersect with each subsequent tag's set
    for tag in normalized.iter().skip(1) {
        let ids = self.load_tag_index_set(tag)?;
        candidates.retain(|id| ids.contains(id));
        // After "rust": candidates = {42}  (only 42 has both "work" AND "rust")
    }

    let mut out: Vec<u64> = candidates.into_iter().collect();
    out.sort_unstable();
    Ok(out)
}
```

This is set intersection. Start with all files matching tag 1, then filter to only those also matching tag 2, tag 3, etc. The more tags you specify, the narrower the result.

---

### The "next level tags" query

When you do `ls @tags/work/`, you see subdirectories for further narrowing. But which subdirectories should appear? Only tags that would produce non-empty results.

`get_next_level_tags()` in file_manager.rs:

```
Current path: @tags/work/
Current tags: ["work"]
Matching files: [42, 55, 78]

For each matching file, collect ALL its tags:
  42 → ["backend", "rust", "work"]
  55 → ["backend", "work"]
  78 → ["frontend", "work"]

Remove current tags ("work") from each set:
  42 → ["backend", "rust"]
  55 → ["backend"]
  78 → ["frontend"]

Union all remaining tags: ["backend", "frontend", "rust"]

These become the subdirectories shown under @tags/work/:
  @tags/work/backend/
  @tags/work/frontend/
  @tags/work/rust/
```

Each subdirectory represents a tag you can drill into further. Only tags that would show actual files appear — no dead ends.

---

### Virtual inodes: The magic trick

Here's a conceptual challenge: `@tags/` doesn't exist on disk. `@tags/work/` doesn't exist on disk. These are **virtual directories** that only exist when you look at them.

The kernel communicates with FUSE using inode numbers. So these virtual directories need inode numbers, but they can't conflict with real file inodes.

Solution: reserve a range starting at 1,000,000 for virtual inodes.

```
Real inodes:     1 (root), 2 (.snapshots), 3 (.create), 4 (@tags), 5 (.tagfs_ctl), 100+
Virtual inodes:  1,000,000+
```

Two types of virtual inodes exist in fuse_handler.rs:

```rust
// A virtual directory representing a tag combination
struct TagVirtualDirContext {
    pub tags: Vec<String>,   // e.g., ["work", "rust"]
}

// A virtual file that points back to a real file
struct TagVirtualFileContext {
    pub real_inode_id: u64,  // e.g., 42 (the real inode)
}
```

These are stored in four HashMaps:

```rust
tag_virtual_dirs:     HashMap<u64, TagVirtualDirContext>   // vino → tags
tag_virtual_files:    HashMap<u64, TagVirtualFileContext>  // vino → real inode
tag_dir_ids_by_key:   HashMap<String, u64>                // "work/rust" → vino
tag_file_ids_by_key:  HashMap<String, u64>                // "work|42" → vino
```

Virtual inodes are **ephemeral** — created on-the-fly when you navigate, not stored on disk. After a restart, they're regenerated as you browse.

---

### Walking through a tag directory listing

Let's trace `ls mnt/@tags/work/` step by step.

**Step 1: Kernel calls `lookup(parent=1, name="@tags")`**

In `resolve_tag_lookup()` at fuse_handler.rs:
```rust
if parent == FUSE_ROOT_ID && name == "@tags" {
    return Ok(Some((TAGS_DIR_ID, self.tag_dir_attr(TAGS_DIR_ID))));
}
```
Returns inode 4 (`TAGS_DIR_ID`) with directory attributes.

**Step 2: Kernel calls `lookup(parent=4, name="work")`**

Still in `resolve_tag_lookup()`:
```rust
if parent == TAGS_DIR_ID {
    let files = self.manager.get_files_with_tag("work")?;
    if !files.is_empty() {
        let tags = vec!["work".to_string()];
        let virtual_ino = self.get_or_create_tag_virtual_dir(&tags);
        return Ok(Some((virtual_ino, self.tag_dir_attr(virtual_ino))));
    }
}
```
Checks: does tag "work" have any files? Yes → create virtual directory inode (say 1,000,000) → return it.

**Step 3: Kernel calls `readdir(ino=1000000)`**

In `readdir()` at fuse_handler.rs:
```rust
if let Some(current_tags) = self.get_virtual_dir_tags(ino) {
    // current_tags = ["work"]

    // A. List subdirectory tags (further narrowing)
    let next_tags = self.manager.get_next_level_tags(&current_tags)?;
    // next_tags = ["backend", "frontend", "rust"]
    for tag in next_tags {
        let mut extended = current_tags.clone();
        extended.push(tag.clone());
        let virtual_ino = self.get_or_create_tag_virtual_dir(&extended);
        reply.add(virtual_ino, ..., FileType::Directory, tag);
    }
    // Shows: backend/  frontend/  rust/

    // B. List matching files
    let matching_inodes = self.manager.get_files_by_tags(&current_tags)?;
    // matching_inodes = [42, 55, 78]
    for inode_id in matching_inodes {
        let name = self.lookup_live_inode_name(inode_id)?;  // "api.rs"
        let virtual_ino = self.get_or_create_tag_virtual_file(&current_tags, inode_id);
        reply.add(virtual_ino, ..., FileType::RegularFile, name);
    }
    // Shows: api.rs  config.yaml  app.js
}
```

Result of `ls @tags/work/`:
```
backend/  frontend/  rust/  api.rs  config.yaml  app.js
```

**Step 4: User does `cat mnt/@tags/work/api.rs`**

Kernel calls `lookup(parent=1000000, name="api.rs")` → `resolve_tag_lookup()` matches the file against the tag combination → creates a virtual file inode (say 1,000,005) that maps back to real inode 42.

Kernel calls `read(ino=1000005)` → your `read()` resolves:
```rust
let target_ino = self.real_inode_for_virtual_file(ino).unwrap_or(ino);
// target_ino = 42
```
Then reads inode 42's data from cache/CAS as normal. The virtual inode is just a redirect.

---

### Auto-tagging: No manual work needed

When a file is created under `projects/backend/`, the system automatically tags it. Look at `derive_tags_from_parent_ancestry()` in fuse_handler.rs:

```rust
fn derive_tags_from_parent_ancestry(&self, parent_inode_id: u64) -> Vec<String> {
    let mut tags = Vec::new();
    let mut current = parent_inode_id;

    // Walk UP the tree, collecting directory names
    while current != FUSE_ROOT_ID && current != 0 {
        match self.manager.load_inode(current) {
            Ok(Some(meta)) => {
                if !meta.name.is_empty() {
                    tags.push(meta.name);   // Collect "backend", then "projects"
                }
                current = meta.parent_id;   // Move to parent
            }
            _ => break,
        }
    }

    tags.reverse();  // ["projects", "backend"]
    tags
}
```

This walks up from the parent to the root, collecting every directory name. So a file created at `projects/backend/api.rs` gets auto-tagged `["projects", "backend"]`.

This is called from `create_live_file_under_parent()` and `write()`:

```rust
let auto_tags = self.derive_tags_from_parent_ancestry(parent);
if !auto_tags.is_empty() {
    let _ = self.manager.set_file_tags(new_id, name, auto_tags);
}
```

The effect: every file is automatically browsable through `@tags/` based on its directory path. A file at `work/2026/report.txt` appears in `@tags/work/`, `@tags/2026/`, and `@tags/work/2026/` — all without the user explicitly tagging anything.

---

### Manual tagging: The control file

For explicit tagging beyond what auto-tag provides, users write commands to `.tagfs_ctl`:

```bash
# Tag a file
echo "set docs/report.txt urgent priority" > mnt/.tagfs_ctl

# Remove all tags from a file
echo "del docs/report.txt" > mnt/.tagfs_ctl
```

When FUSE calls `write(ino=5)` (TAGFS_CONTROL_ID), `handle_tagfs_control_write()` in fuse_handler.rs parses the command:

```rust
fn handle_tagfs_control_write(&self, data: &[u8]) -> Result<(), i32> {
    let command = String::from_utf8_lossy(data).trim().to_string();
    let parts: Vec<&str> = command.split_whitespace().collect();

    match parts[0] {
        "set" => {
            // parts = ["set", "docs/report.txt", "urgent", "priority"]
            let path = parts[1];
            let tags = parts[2..].to_vec();
            self.manager.set_file_tags_by_path(path, tags)?;
        }
        "del" => {
            // parts = ["del", "docs/report.txt"]
            let path = parts[1];
            let inode_id = self.manager.resolve_inode_by_path(path)?;
            self.manager.delete_file_tags(inode_id)?;
        }
    }
}
```

`set_file_tags_by_path()` resolves the path to an inode ID by walking the dirent chain, then calls `set_file_tags()`.

---

### Tag deletion cleanup

When a file is unlinked, its tags are cleaned up. Look at `unlink()` in fuse_handler.rs:

```rust
if let Some(removed_node) = parent_guard.children.remove(name_str) {
    let removed_ino = removed_node.read().unwrap().id;
    let _ = self.manager.delete_dirent(parent, name_str);
    let _ = self.manager.delete_file_tags(removed_ino);  // ← Clean up tags!
    self.evict_inode_cache(removed_ino);
    reply.ok();
}
```

And `delete_file_tags()` in file_manager.rs removes the file from every tag index it appeared in:

```rust
pub fn delete_file_tags(&self, inode_id: u64) -> Result<(), String> {
    // 1. Get all tags this file has
    let existing_tags = self.get_file_tags(inode_id)?;

    // 2. Remove this file from each tag's inverted index
    for tag in existing_tags {
        let mut ids = self.load_tag_index_set(&tag)?;
        ids.remove(&inode_id);
        self.save_tag_index_set(&tag, &ids)?;
        // If ids is now empty, the key is removed entirely
    }

    // 3. Remove the per-file tag set
    self.db.remove(ino_tags_key)?;
}
```

---

### Creating files inside tag directories

What if you do `echo "hello" > mnt/@tags/work/newfile.txt`? You're creating a file inside a virtual tag directory.

Your `create()` in fuse_handler.rs handles this:

```rust
if self.is_tag_virtual_dir_inode(parent) {
    if let Some(tags) = self.get_virtual_dir_tags(parent) {
        // tags = ["work"]
        match self.ensure_real_path_for_tags(&tags) {
            Ok(parent_ino) => {
                real_parent = parent_ino;    // Real directory for "work"
                virtual_tags = Some(tags);
            }
        }
    }
}

let new_id = self.create_live_file_under_parent(real_parent, &name_str)?;
```

`ensure_real_path_for_tags()` creates actual directories in the live tree matching the tag path. If tags are `["work"]`, it ensures a directory called `work/` exists under root. The file is created there as a real file, and tagged with `["work"]`.

---

### Visual summary

```
Real filesystem tree:          Tag virtual overlay:
                               
root (ino=1)                   @tags/ (ino=4)
├── projects/                  ├── backend/     → [api.rs, config.yaml]
│   ├── backend/               ├── frontend/    → [app.js]
│   │   ├── api.rs             ├── projects/    → [api.rs, config.yaml, app.js]
│   │   └── config.yaml        ├── rust/        → [api.rs]
│   └── frontend/              └── work/        → [report.txt]
│       └── app.js                 └── 2026/    → [report.txt]
└── work/
    └── report.txt             Files exist in ONE place on disk.
                               Tags provide MANY views into the same data.
```

---

### What to say if asked...

**"How is this different from symlinks?"**
→ Symlinks are static, one-to-one, and must be manually created/maintained. Tags are many-to-many (a file has many tags, a tag has many files), automatically updated, and support dynamic intersection queries. Symlinks also break when the target moves; virtual inodes always resolve to the current real inode.

**"What's the time complexity of a tag query?"**
→ Loading one tag index is $O(1)$ from sled (key lookup). Intersection of $k$ tags with $n$ files each is $O(k \cdot n)$. For typical usage (small tag sets), this is effectively instant.

**"Why not use a relational database for tags?"**
→ sled's prefix scanning (`tag_index:*`) gives us efficient iteration. The inverted index pattern gives us O(1) per-tag lookups. A full SQL database would be overkill for this simple key-value pattern and would add external dependency overhead.

**"What happens to tags when you rename a file?"**
→ Look at `rename()` in fuse_handler.rs: after updating the dirent, it re-derives auto-tags based on the new parent ancestry and updates them. So moving a file from `work/` to `personal/` updates its auto-tags accordingly.

**"Are tags persistent across restarts?"**
→ Yes. Both `ino_tags:{id}` and `tag_index:{tag}` are stored in sled, which persists to disk. The virtual inode mappings are ephemeral (regenerated on navigation), but the underlying tag data survives restarts.

---

### Check your understanding

1. What's the difference between `ino_tags:42` and `tag_index:work`? Why do you need both?
2. You navigate to `@tags/work/rust/`. What query runs against the database? What's the result?
3. A file is created at `projects/backend/api.rs`. What tags does it get, and how?
4. What is a virtual inode? Why can't you use the real inode number directly in `@tags/`?
5. You delete a file that had tags `["work", "rust"]`. What database operations occur?
6. What does `get_next_level_tags(["work"])` return, and why is it needed for `readdir`?

Say "next" for Stage 8 (Putting it all together — end-to-end flows and final review).

User: next

GitHub Copilot: 

## Stage 8: Putting It All Together

This stage is about **connecting everything** and practicing the kind of end-to-end narration you'll need in the viva. No new concepts — just synthesis.

---

### The complete architecture in one picture

```
┌──────────────────────────────────────────────────────────────────┐
│  USER PROGRAMS: cat, ls, echo, vim, cp, GUI file managers       │
└────────────────────────┬─────────────────────────────────────────┘
                         │ system calls (open, read, write, ...)
┌────────────────────────▼─────────────────────────────────────────┐
│  LINUX KERNEL                                                     │
│  VFS → FUSE module → /dev/fuse                                    │
└────────────────────────┬─────────────────────────────────────────┘
                         │ requests/responses
┌────────────────────────▼─────────────────────────────────────────┐
│  YOUR PROGRAM (ArcFS daemon)                                      │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ LAYER 1: FUSE Interface (fuse_handler.rs)                    │ │
│  │  impl Filesystem for ArcFS                                   │ │
│  │  lookup, read, write, mkdir, create, unlink, readdir, ...    │ │
│  │  Snapshot triggers (.snap_, .restore_)                       │ │
│  │  TagFS virtual navigation (@tags/, .tagfs_ctl)               │ │
│  └───────────────────────┬─────────────────────────────────────┘ │
│                          │                                        │
│  ┌───────────────────────▼─────────────────────────────────────┐ │
│  │ LAYER 2: Page Cache (in fuse_handler.rs)                     │ │
│  │  HashMap<u64, (Vec<u8>, bool)> — inode → (buffer, dirty)    │ │
│  │  VecDeque<u64> — LRU eviction order                         │ │
│  │  Capacity: 1024 entries                                      │ │
│  │  Write-back: writes go to RAM, flush to disk on release      │ │
│  └───────────────────────┬─────────────────────────────────────┘ │
│                          │                                        │
│  ┌───────────────────────▼─────────────────────────────────────┐ │
│  │ LAYER 3: Metadata Engine (file_manager.rs)                   │ │
│  │  sled embedded KV database                                   │ │
│  │  ino_meta:{id} → InodeMetadata                               │ │
│  │  ino_recipe:{id} → FileRecipe (ordered chunk hashes)         │ │
│  │  dirent:{parent}:{name} → child_id                          │ │
│  │  ino_tags:{id} → FileTagSet                                  │ │
│  │  tag_index:{tag} → [inode_ids]                               │ │
│  │  snapshot:{name} → SnapshotMetadata                          │ │
│  └───────────────────────┬─────────────────────────────────────┘ │
│                          │                                        │
│  ┌───────────────────────▼─────────────────────────────────────┐ │
│  │ LAYER 4: CAS Storage Engine (storage.rs)                     │ │
│  │  Immutable, append-only blobs                                │ │
│  │  SHA-256 hash = address                                      │ │
│  │  Sharded: cas/{first2chars}/{rest}                           │ │
│  │  zstd compressed (level 3)                                   │ │
│  │  Dedup: if hash exists on disk → skip                        │ │
│  └─────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

---

### Flow 1: "Walk me through writing a file"

**Scenario**: `echo "Hello ArcFS" > mnt/greeting.txt`

Narrate this in order:

> The shell calls `open()` with the `O_CREAT` flag. The kernel translates this to a FUSE `create()` call on my daemon with parent inode 1 (root) and name "greeting.txt".
>
> My `create()` handler generates a new inode ID — say 100 — using an atomic counter. It creates an `Inode` struct with kind=RegularFile, saves the inode metadata to sled (`ino_meta:100`), saves a directory entry (`dirent:1:greeting.txt → 100`), and creates an empty recipe (`ino_recipe:100 → {size:0, chunks:[]}`). It inserts the inode into the in-memory tree (root's children map) and the inode registry. It also derives auto-tags from the parent ancestry — since the parent is root, which has no name, there are no auto-tags. Finally, it replies to the kernel with the new inode's attributes.
>
> The shell then calls `write()` with inode 100, offset 0, and the bytes "Hello ArcFS\n". My `write()` handler puts this data into the page cache: `{100: (b"Hello ArcFS\n", dirty=true)}`. It updates the inode's size to 12 bytes and mtime to now. It touches the LRU to mark this entry as recently used. No disk I/O happens — the reply goes back immediately.
>
> The shell calls `release()` to close the file handle. My `release()` handler calls `flush_inode_cache(100)`. This reads the dirty buffer from the cache, passes it to `write_file_by_id()`, which calls `create_recipe_from_data()`. The chunker feeds each byte through the Gear Hash. Since 12 bytes is below the 2KB minimum chunk size, no boundary is found — the entire buffer becomes a single tail chunk. This chunk is passed to `storage.write_chunk()`: SHA-256 hashes the raw bytes, checks if that hash exists in the CAS directory (it doesn't), compresses with zstd, and writes to `cas/{first2}/{rest}`. The recipe `{size:12, chunks:["hash..."]}` is saved to sled. The inode metadata is persisted. The cache entry is marked clean.
>
> The data is now safely on disk in three places: the chunk in CAS, the recipe in sled, and the inode/dirent metadata in sled.

---

### Flow 2: "Walk me through reading that file back"

**Scenario**: `cat mnt/greeting.txt`

> `cat` calls `open()`, which triggers FUSE `lookup(parent=1, name="greeting.txt")`. My handler checks the inode registry, finds inode 100 in root's children, and returns its attributes.
>
> The kernel calls `open(ino=100)`. My handler returns `opened(0, 0)` — no special handling needed.
>
> The kernel calls `read(ino=100, offset=0, size=4096)`. My handler checks the page cache. If the data is still cached from the write (cache entry `{100: (b"Hello ArcFS\n", false)}`), it serves directly: `reply.data(&data[0..12])`. If the cache was evicted, it calls `read_file_by_id(100)`, which loads the recipe from sled, fetches each chunk hash from CAS, decompresses, concatenates, populates the cache, and then serves the slice.
>
> `cat` receives "Hello ArcFS\n" and prints it to the terminal.

---

### Flow 3: "Walk me through a snapshot and subsequent edit"

**Scenario**: Create snapshot, edit file, browse snapshot

> I type `mkdir mnt/.snap_v1`. The kernel calls `mkdir(parent=1, name=".snap_v1")`. My handler detects the `.snap_` prefix and calls `create_snapshot_named("v1")`.
>
> First, `flush_all_dirty_cache()` ensures every dirty buffer is persisted to CAS and sled. Then `clone_subtree_from_metadata()` recursively walks the live tree starting from root inode 1. For each inode, it creates a NEW inode with a new ID but copies the SAME recipe — the same chunk hashes. For directories, it copies the dirent structure. No physical chunks are touched. The snapshot root (say inode 200) is stored in the `snapshots` HashMap and persisted to sled as `snapshot:v1`.
>
> Now I edit the file: `echo " World" >> mnt/greeting.txt`. The kernel calls `write(ino=100, offset=12, data=" World\n")`. The page cache buffer for inode 100 is extended and modified in-place: `b"Hello ArcFS\n World\n"`. Size updates to 19 bytes. Nothing goes to disk yet.
>
> When the file is released, `flush_inode_cache(100)` chunks the new 19-byte buffer. The chunker produces one chunk (still under 2KB minimum). This chunk has different content than before, so SHA-256 produces a different hash. A NEW chunk is written to CAS. The recipe for inode 100 is updated with the new hash. The OLD chunk (from before the edit) is still on disk because the snapshot's recipe (inode 200) still references it.
>
> Now I browse the snapshot: `cat mnt/.snapshots/v1/greeting.txt`. The kernel calls `lookup(parent=2, name="v1")` — I find the snapshot and return its root's attributes with read-only permissions (0o555). Then `lookup` and `read` on the snapshot's copy of greeting.txt (inode 201). My `read()` handler finds inode 201 in the snapshot tree, loads its recipe (which has the OLD hash), fetches the OLD chunk from CAS, decompresses, and returns "Hello ArcFS\n" — the original content, preserved.

---

### Flow 4: "Walk me through tagging and browsing"

**Scenario**: Tag a file, then navigate by tags

> I type `echo "set greeting.txt demo fun" > mnt/.tagfs_ctl`. The kernel calls `write(ino=5, offset=0, data="set greeting.txt demo fun\n")`. My handler recognizes inode 5 as `TAGFS_CONTROL_ID` and calls `handle_tagfs_control_write()`. It parses the command: `set`, path=`greeting.txt`, tags=`["demo", "fun"]`. It calls `set_file_tags_by_path("greeting.txt", ["demo", "fun"])`.
>
> This resolves the path: `dirent:1:greeting.txt → inode 100`. Then calls `set_file_tags(100, "greeting.txt", ["demo", "fun"])`. Tags are normalized (lowercased, sorted, deduped) → `["demo", "fun"]`. The per-file record is saved: `ino_tags:100 → FileTagSet{id:100, filename:"greeting.txt", tags:["demo","fun"]}`. The inverted indices are updated: `tag_index:demo → [100]`, `tag_index:fun → [100]`.
>
> Now I browse: `ls mnt/@tags/`. The kernel calls `readdir(ino=4)` (TAGS_DIR_ID). My handler calls `get_next_level_tags([])` which scans all `tag_index:*` keys and returns `["demo", "fun"]`. These appear as subdirectories.
>
> I drill in: `ls mnt/@tags/demo/`. Kernel calls `lookup(parent=4, name="demo")` → my `resolve_tag_lookup()` checks `get_files_with_tag("demo")` → returns `[100]` (non-empty) → creates virtual dir inode 1,000,000 for tags `["demo"]`. Then `readdir(ino=1000000)` → next-level tags: `["fun"]` (since inode 100 also has "fun") → shows `fun/` subdirectory AND `greeting.txt` (the matching file, shown as virtual file inode 1,000,001 pointing to real inode 100).
>
> I read it: `cat mnt/@tags/demo/greeting.txt`. The kernel calls `read(ino=1000001)`. My handler resolves: `real_inode_for_virtual_file(1000001) → 100`. Then reads inode 100's data from cache or CAS. The virtual inode is just a redirect — the actual data comes from the real file.

---

### Flow 5: "Walk me through garbage collection"

**Scenario**: Delete a file, then run GC

> I delete greeting.txt: `rm mnt/greeting.txt`. FUSE calls `unlink(parent=1, name="greeting.txt")`. My handler removes "greeting.txt" from root's children map, deletes `dirent:1:greeting.txt` from sled, deletes tags for inode 100, and evicts inode 100 from the page cache. But the recipe (`ino_recipe:100`) and the CAS chunks are NOT deleted yet.
>
> At this point, there may be orphaned chunks — chunks on disk that no recipe references. But they're harmless; they just waste space.
>
> I run GC: `cargo run -- gc`. This calls `run_gc()` in file_manager.rs. Step 1: scan ALL `ino_recipe:*` entries in sled → collect every chunk hash into a HashSet called `active_hashes`. Step 2: list ALL physical files in the `cas/` directory. Step 3: for each file on disk, check if its hash is in `active_hashes`. If not → it's an orphan → delete from disk.
>
> If the snapshot still exists and its recipe references the old chunk, that chunk IS in `active_hashes` and won't be deleted. GC only removes chunks that NO recipe references — not even snapshots.

---

### Flow 6: "What happens at startup?"

> `cargo run -- mount ./mnt` starts the daemon. `ArcFS::new(manager)` runs:
>
> 1. Create root inode (id=1, kind=Directory), insert into the inode registry.
>
> 2. `hydrate_tree()`: starting from root (inode 1), scan sled for all `dirent:1:*` entries. For each child, load `ino_meta:{child_id}`, create an `Inode` struct, wrap in `Arc<RwLock>`, insert into the registry and root's children map. If the child is a directory, recurse: scan `dirent:{child_id}:*`, and so on. This rebuilds the entire in-memory tree from the persistent metadata.
>
> 3. Update `next_inode` counter to be higher than any existing inode ID (prevents ID collisions).
>
> 4. `restore_snapshots()`: scan all `snapshot:*` entries in sled. For each, call `restore_snapshot_subtree()` which recursively loads the snapshot's inode tree from sled. If any inode is missing (corruption), prune the snapshot.
>
> 5. `fuser::mount2()` registers with the kernel and enters the event loop — now accepting FUSE requests.
>
> The page cache starts empty. Data is loaded on-demand as files are read.

---

### Concurrency: How parallel requests are handled

FUSE sends requests from multiple threads. Your code uses `Arc<RwLock<...>>` throughout. Here's the lock hierarchy again — this is likely a direct question:

```
LOCK ORDER (always acquire top-to-bottom, never reverse):

1. inode_registry (RwLock)     ← Global. Protects the inode lookup table.
2. Per-Inode (RwLock)          ← Fine-grained. Protects one file's attributes.
3. page_cache (RwLock)         ← Global. Protects the volatile buffers.
```

**Why this order prevents deadlocks**: A deadlock occurs when thread A holds lock X and waits for lock Y, while thread B holds lock Y and waits for lock X. By enforcing a total order, both threads acquire locks in the same sequence — no circular wait is possible.

**The pattern**:
```rust
// CORRECT: acquire registry, clone Arc, DROP registry, then lock individual node
let cloned_arc = self.inode_registry.read().unwrap().get(&id).cloned();
// registry lock is dropped here (end of temporary borrow)
let node = cloned_arc.unwrap().write().unwrap();  // now lock the individual node
```

If you held the registry lock while trying to lock an individual node, and another thread held that node's lock while trying to access the registry — deadlock.

---

### Known limitations — Be honest about these

Interviewers respect honesty about limitations. If asked "what are the weaknesses?":

| Limitation | Why |
|-----------|-----|
| **No hard links** | Each inode has one parent. Supporting multiple parents would require reference counting in the dirent layer. |
| **No symlinks** | Not implemented. Would need a new inode kind and `readlink()` handler. |
| **No mmap support** | FUSE `mmap` requires `read()`/`write()` forwarding. Direct memory mapping to CAS chunks isn't straightforward. |
| **Single-node only** | No network protocol. Can't distribute CAS across machines. |
| **FUSE tax** | Every I/O operation has extra context switches vs kernel filesystems. Measurable latency overhead. |
| **Coarse page cache locking** | One RwLock for the entire cache HashMap. Under heavy concurrent load, this becomes a bottleneck. Per-inode locks would help. |
| **No partial chunk updates** | Editing 1 byte in a 4KB chunk requires re-chunking, re-hashing, and re-storing the entire chunk. |
| **GC is offline** | Must be run manually. No background GC thread. No reference counting for automatic cleanup. |

---

### What makes this project interesting — Your selling points

If asked "what are you proud of?" or "what's innovative?":

1. **Content-defined deduplication via FastCDC** — not just fixed-block dedup. Handles insertions/edits gracefully.
2. **Zero-copy snapshots** — metadata-only cloning. O(metadata) time and space, not O(data).
3. **Tag-based virtual navigation** — goes beyond traditional hierarchies. AND-intersection queries with auto-tagging from directory paths.
4. **Write-back cache with in-place mutation** — identified and fixed an O(N²) performance bug.
5. **Strict lock ordering** — principled approach to deadlock prevention in a multithreaded FUSE daemon.
6. **Everything in Rust** — memory safety without garbage collection pauses. No null pointer crashes, no buffer overflows, no data races (enforced at compile time).

---

### Rapid-fire prep: Quick answers to likely questions

**"What language and why?"** → Rust. Memory safety without GC. Critical for a filesystem where bugs corrupt data. Compile-time guarantees against data races in the multithreaded FUSE handler.

**"What's the Rust edition?"** → 2024 (in Cargo.toml). Latest stable edition.

**"How do you run it?"** → `cargo run --release -- mount ./mnt`. Mounts at mnt. Ctrl+C or `fusermount -u ./mnt` to unmount.

**"How do you test it?"** → `cargo test` runs unit tests (chunker, storage, file_manager, tagfs) and integration tests (backend_stress, gc_test). Shell scripts for E2E (regression_e2e.sh).

**"What's bincode?"** → Binary serialization format. Converts Rust structs to/from bytes for sled storage. Compact and fast, unlike JSON.

**"What's serde?"** → Rust's serialization framework. `#[derive(Serialize, Deserialize)]` on your structs lets bincode (or JSON, or any format) convert them automatically.

**"What's the total lines of code?"** → ~2,200 lines across 4 source files. Plus ~300 lines of tests.

**"What would you add next?"** → Background GC thread, symlink support, POSIX ACLs, per-inode cache locks for better concurrency, a FUSE `notify_inval_entry` for cache invalidation, and potentially a network protocol for distributed CAS.

---

### Study complete. Final checklist.

Go back through each stage and make sure you can:

- [ ] **Stage 1**: Explain inodes and dirents. Point to `Inode` struct and `dirent:{}:{}` keys in your code.
- [ ] **Stage 2**: Explain FUSE request/response flow. Explain the FUSE tax. Point to `impl Filesystem for ArcFS`.
- [ ] **Stage 3**: Explain SHA-256, CAS, dedup, sharding, compress-after-hash. Point to `write_chunk()` and `read_chunk()`.
- [ ] **Stage 4**: Explain CDC vs fixed-size, Gear Hash, mask, min/max. Point to `feed_byte()`, `should_cut()`, `chunk_lengths()`.
- [ ] **Stage 5**: Explain write-back cache, flush triggers, recipe, in-place mutation. Point to `write()`, `flush_inode_cache()`, `create_recipe_from_data()`.
- [ ] **Stage 6**: Explain zero-copy snapshots, CoW with `Arc::strong_count`, restore. Point to `create_snapshot_named()`, `get_mutable_inode()`.
- [ ] **Stage 7**: Explain tags, inverted index, virtual inodes, auto-tagging. Point to `set_file_tags()`, `resolve_tag_lookup()`, `derive_tags_from_parent_ancestry()`.
- [ ] **Stage 8**: Narrate 3 end-to-end flows aloud without looking at notes.

Good luck tomorrow.