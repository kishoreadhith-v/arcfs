# ARCHITECTURE
> Generated: 2026-05-07 | Focus: arch | Project: arcfs

## Summary
ArcFS is a FUSE userspace filesystem implementing content-defined chunking with SHA256 content-addressed storage and sled-backed metadata persistence. The architecture has four cleanly layered components: a CDC chunker, a CAS storage engine, a metadata/orchestration layer (FileManager), and a FUSE handler that presents a standard filesystem interface with virtual TagFS overlays.

## Component Map

```
┌─────────────────────────────────────────────────────────┐
│  CLI (main.rs)  ─  clap subcommand dispatch             │
└──────────────────────────┬──────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────┐
│  ArcFS (fuse_handler.rs)  ─  FUSE Filesystem impl       │
│  ┌─────────────────┐  ┌──────────────────────────────┐  │
│  │ inode_registry  │  │ TagFS virtual layer           │  │
│  │ RwLock<HashMap> │  │ tag_virtual_dirs/files        │  │
│  ├─────────────────┤  │ tag_dir/file_ids_by_key       │  │
│  │ page_cache      │  └──────────────────────────────┘  │
│  │ RwLock<HashMap> │  ┌──────────────────────────────┐  │
│  │ (data, dirty)   │  │ snapshots                    │  │
│  ├─────────────────┤  │ RwLock<HashMap<String,       │  │
│  │ cache_lru       │  │ Snapshot>>                   │  │
│  │ VecDeque<u64>   │  └──────────────────────────────┘  │
│  └─────────────────┘                                    │
└──────────────────────────┬──────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────┐
│  FileManager (file_manager.rs)                          │
│  ┌──────────────────────┐  ┌───────────────────────┐    │
│  │ sled DB              │  │ Storage               │    │
│  │ InodeMetadata        │  │ (storage.rs)          │    │
│  │ Dirent               │  │ SHA256 CAS + zstd     │    │
│  │ FileRecipe           │  └───────────────────────┘    │
│  │ SnapshotMetadata     │  ┌───────────────────────┐    │
│  │ tag sets             │  │ Chunker               │    │
│  └──────────────────────┘  │ (chunker.rs)          │    │
│                            │ Gear-hash CDC         │    │
│                            └───────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## Data Flow

### Write Path
```
write(data) → Chunker.process(data)
           → [chunk_1, chunk_2, ..., chunk_N]  (Gear-hash boundaries)
           → Storage.write(chunk_i)            (SHA256 hash → zstd compress → CAS file)
           → FileManager.save_recipe(inode_id, [hash_1...hash_N, total_size])
           → FileManager.save_inode(metadata)   (sled DB)
           → page_cache evict/dirty flush (on fsync/release)
```

### Read Path
```
read(inode_id, offset, size) → page_cache lookup
                             → [miss] FileManager.load_recipe(inode_id)
                             → [hash_1...hash_N] → Storage.read(hash_i) × N
                             → reassemble chunks → serve bytes at [offset, offset+size]
```

### Tag Query Path
```
readdir(.tags/<tag1>/<tag2>/) → FileManager.tag_query([tag1, tag2])
                              → sled: tag_index:<tag1> ∩ tag_index:<tag2>
                              → inode_ids → generate virtual inodes lazily
                              → populate tag_virtual_files (ephemeral, not persisted)
```

## Key Data Structures

### `ArcFS` (runtime state)
| Field | Type | Purpose |
|---|---|---|
| `inode_registry` | `Arc<RwLock<HashMap<u64, Arc<RwLock<Inode>>>>>` | All live inodes, rebuilt from sled at boot |
| `page_cache` | `Arc<RwLock<HashMap<u64, (Vec<u8>, bool)>>>` | Write-back cache; tuple = (data, dirty) |
| `cache_lru` | `Arc<RwLock<VecDeque<u64>>>` | LRU eviction order; capacity = 1024 |
| `snapshots` | `Arc<RwLock<HashMap<String, Snapshot>>>` | Named snapshots; roots are Arc-cloned inode trees |
| `tag_virtual_dirs` | `Arc<RwLock<HashMap<u64, TagVirtualDirContext>>>` | Ephemeral tag nav directories |
| `tag_virtual_files` | `Arc<RwLock<HashMap<u64, TagVirtualFileContext>>>` | Ephemeral tag nav file entries |
| `next_vnode` | `AtomicU64` | Virtual inode counter (starts at 1,000,000) |
| `next_inode` | `AtomicU64` | Real inode counter (starts at 100) |

### Inode Namespaces
- **Real inodes** (id < 1,000,000): backed by sled, persisted across mounts
- **Virtual inodes** (id ≥ 1,000,000 = `VIRTUAL_INODE_START`): ephemeral TagFS navigation, never persisted

### Reserved Inode IDs
| ID | Purpose |
|---|---|
| `1` | Filesystem root |
| `2` | `.snapshots/` virtual directory |
| `3` | `.snapshots/.snap` snapshot-create sentinel (write triggers snapshot) |
| `4` | `.tags/` virtual directory root |
| `5` | `.tags/.tagfs_control` write-command interface |

## Concurrency Model
ArcFS is fully synchronous — no async runtime. Thread safety is achieved entirely through `Arc<RwLock<_>>` on all shared state. FUSE kernel calls may arrive concurrently from multiple threads.

### Lock Ordering (must be strictly respected to prevent deadlocks)
1. `inode_registry` (global registry `RwLock`) — acquired first
2. Per-inode `Inode` lock — acquired while registry read lock is held
3. `page_cache` / `cache_lru` — acquired last, never while holding an inode lock

Violating this order risks deadlock under concurrent `fio`/FUSE workloads.

## Persistence Model
- **Source of truth**: sled embedded database at `<storage_dir>/`
- **At startup**: `hydrate_tree()` rebuilds `inode_registry` from sled; `restore_snapshots()` reconstructs snapshot trees
- **Chunk store**: separate directory tree at `<storage_dir>/cas/<hash[0:2]>/<hash[2:]>`
- **Deduplication**: implicit at write time — `Storage.write()` skips the write if the CAS path already exists

## Snapshot Design
Snapshots use structural sharing: `Snapshot.root` is an `Arc`-cloned subtree of the live inode tree at capture time. Modifying a snapshot-shared inode triggers Copy-on-Write (CoW) cloning before mutation. Snapshot metadata is persisted in sled under `snapshot:<name>` keys.

## TagFS Design
`.tags/` is a virtual filesystem overlay built lazily from sled tag index queries. No TagFS state is persisted — it is regenerated from the tag indexes on every directory listing. The `.tagfs_control` file accepts write commands (`set <path> <tags...>`, `del <path>`) to mutate tag indexes on live mounts without unmounting.

## Observations
- The page cache write-back design means `fsync` / file close triggers actual chunk persistence — data loss is possible on process crash before fsync
- `VIRTUAL_INODE_START = 1,000,000` provides a hard boundary between persisted and ephemeral inode ID spaces
- No network layer, no remote storage — entirely local filesystem abstraction
- Chunk deduplication is content-based (hash equality), not reference-counted — a chunk file persists until GC explicitly removes unreferenced hashes
