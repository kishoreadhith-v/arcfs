# Project Context: ArcFS
**Role:** You are an expert Systems Engineer assisting with "ArcFS," a final-year CS project.
**Goal:** Build a high-performance, userspace file system in Rust using FUSE that supports Time Travel, Semantic Tagging, and Transparent Compression.

---

## 1. System Architecture & Core Concepts
The system is a User-Space File System using the `fuser` crate.
* **Core Structure:** An in-memory Inode Tree guarded by `Arc<RwLock<Inode>>` for thread safety.
* **Storage Backend:** Content-Addressable Storage (CAS). Files are split into chunks, hashed (SHA-256), compressed (Zstd), and stored in a flat directory structure.
* **Metadata Store:** `sled` (embedded DB) persists the `Inode` map and `FileRecipe` (list of chunk hashes) to disk.

### Critical Data Structures
* **Inode:** Represents a file or directory. Contains `id` (u64), `children` (HashMap), `attr` (FileAttr), and `recipe` (Option<FileRecipe>).
* **FileRecipe:** The "map" of a file. Contains `file_size`, `chunks` (Vec<String>), and `kind` (File/Dir).
* **Inode Registry:** A global `HashMap<u64, Arc<RwLock<Inode>>>` that allows O(1) lookup of any node by ID.

---

## 2. Feature Specifications & Detailed Design

### Phase 1: Core Engine & Memory Safety (Completed)
* **Thread Safety:** All mutable state is protected via `RwLock`. We use `Arc` to share nodes between the Live FS and Snapshots.
* **Garbage Collection (GC):** We rely on Rust's `Arc` reference counting. When a file is deleted (`unlink`), if `strong_count == 0`, the underlying memory is freed.
    * **Algorithm:** Reference Counting + Mark-and-Sweep (Backup).
* **FUSE Bindings:** We implement `lookup`, `getattr`, `readdir`, `mkdir`, `read`, `write`, `create`, `setattr`, and `unlink`.

### Phase 2: Chronos (Time Travel / Snapshots) (Current Focus)
* **Concept:** Instant, O(1) snapshots using "Lazy Cloning" (Copy-on-Write).
* **Snapshot Creation:** We clone the **Root Inode** `Arc`. This increments the reference count (e.g., from 1 to 2) but copies no data.
* **Virtual Directory:** A special path `.snapshots/` exposes all saved snapshots as read-only directories.
* **Copy-on-Write (CoW) Logic:**
    1.  **Intercept Write:** When a write request comes in (`write`, `setattr`), we check the target Inode.
    2.  **Check Sharing:** If `Arc::strong_count > 1`, the node is shared with a snapshot.
    3.  **Path Copying:** We traverse from Root to the target node. For every node in the path that is shared, we **Deep Clone** it, assign a **New Inode ID**, and update the parent's pointer to the new copy.
    4.  **Divergence:** This splits the Live Tree from the Snapshot Tree at the point of modification.
    5.  **Logs:** We must log `[CoW] Node shared! Cloning...` to demonstrate this behavior.

### Phase 3: TagFS (Semantic Tagging) (Planned)
* **Concept:** Files are not bound to a single directory hierarchy.
* **Data Structure:** **Inverted Index** (Tag -> List of Inode IDs).
* **Implementation:**
    * **Tag Storage:** Tags are stored in the `sled` database as a serialized `Vec<String>`.
    * **Query Logic:** Accessing `@tags/work/2026` dynamically generates a directory view containing files that match `tags.contains("work") AND tags.contains("2026")`.
    * **Virtual Inodes:** Directory entries in `@tags` are generated on-the-fly; they do not exist on disk.

### Phase 4: ZipFS (Transparent Archive Mounting) (Planned)
* **Concept:** Mount a `.zip` or `.tar.gz` file as a directory without extracting it.
* **Mechanism:** **Offset Mapping**.
    * We read the Central Directory of the ZIP file.
    * We create ephemeral Inodes for each file inside.
    * **Read Redirect:** A read request for `archive.zip/doc.txt` is translated into: "Seek to offset 1024 in `archive.zip`, read 500 bytes, decompress, and return."

---

## 3. Project Progress & Status
* **Phase 0 (Research):** Done.
* **Phase 1 (Core Engine):** **DONE.** Basic FUSE operations, Persistence, and CAS are working.
* **Phase 2 (Chronos):** **COMPLETE.** Snapshot system fully implemented with CoW divergence, virtual `.snapshots/` directory, and all write operations protected. See refinements below.
* **Phase 3 & 4:** Planned.

---

## 4. Coding Guidelines
* **Logs:** Use `println!` with prefixes like `[FUSE]`, `[CHRONOS]`, `[GC]` for demo visibility.
* **Error Handling:** Use `libc` error codes (`ENOENT`, `EACCES`, `EIO`).
* **Formatting:** Run `cargo fmt` before providing code snippets.
* **Safety:** Avoid `unsafe` blocks. Use `unwrap()` only in prototypes; prefer `map_err` or `match` for production logic.
