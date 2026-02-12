// src/fuse_handler.rs
use crate::file_manager::{ FileKind, FileManager, FileRecipe };
use fuser::{
    FileAttr,
    FileType,
    Filesystem,
    ReplyAttr,
    ReplyCreate,
    ReplyData,
    ReplyDirectory,
    ReplyEmpty,
    ReplyEntry,
    ReplyOpen,
    ReplyWrite,
    Request,
};
use libc::ENOENT; 
use std::ffi::OsStr;
use std::time::{ Duration, SystemTime };
use std::collections::hash_map::DefaultHasher;
use libc::ENOENT;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::atomic::{ AtomicU64, Ordering };
use std::sync::{ Arc, RwLock };
use std::time::{ Duration, SystemTime };

const TTL: Duration = Duration::from_secs(1);
const FUSE_ROOT_ID: u64 = 1;
const SNAPSHOT_DIR_ID: u64 = 2;

// ===========================================================================
// 1. DATA STRUCTURES
// ===========================================================================

#[derive(Clone)]
pub struct Inode {
    pub id: u64,
    pub children: HashMap<String, Arc<RwLock<Inode>>>,
    pub attr: FileAttr,
    #[allow(dead_code)]
    pub recipe: Option<FileRecipe>,
}

impl Inode {
    fn new(id: u64, kind: FileType) -> Self {
        let mut attr = dir_attr(id);
        attr.kind = kind;
        Inode {
            id,
            children: HashMap::new(),
            attr,
            recipe: None,
        }
    }
}

#[derive(Clone)]
pub struct Snapshot {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub timestamp: SystemTime,
    pub root: Arc<RwLock<Inode>>,
}

fn dir_attr(ino: u64) -> FileAttr {
    FileAttr {
        ino,
        size: 0,
        blocks: 0,
        atime: SystemTime::now(),
        mtime: SystemTime::now(),
        ctime: SystemTime::now(),
        crtime: SystemTime::now(),
        kind: FileType::Directory,
        perm: 0o755,
        nlink: 2,
        uid: 1000,
        gid: 1000,
        rdev: 0,
        flags: 0,
        blksize: 512,
    }
}

pub struct BetterFS {
    pub manager: FileManager,
    pub inode_registry: Arc<RwLock<HashMap<u64, Arc<RwLock<Inode>>>>>,
    pub root: Arc<RwLock<Inode>>,
    pub snapshots: Arc<RwLock<HashMap<String, Snapshot>>>,
    next_inode: AtomicU64,
}

impl BetterFS {
    pub fn new(manager: FileManager) -> Self {
        let registry = Arc::new(RwLock::new(HashMap::new()));
        let root = Arc::new(RwLock::new(Inode::new(FUSE_ROOT_ID, FileType::Directory)));
        registry.write().unwrap().insert(FUSE_ROOT_ID, root.clone());

        let mut fs = BetterFS {
            manager,
            inode_registry: registry,
            root,
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            next_inode: AtomicU64::new(100),
        };

        fs.hydrate_tree();
        fs.restore_snapshots();
        fs
    }

    // Restore snapshots from persistent storage
    fn restore_snapshots(&mut self) {
        let snapshot_metadata = self.manager.load_snapshots();

        for meta in snapshot_metadata {
            // Note: We're reusing the current root as a placeholder
            // In production, we'd need to save/restore the actual tree state
            let snapshot = Snapshot {
                name: meta.name.clone(),
                timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(meta.timestamp),
                root: self.root.clone(), // Simplified: using current root
            };

            self.snapshots.write().unwrap().insert(meta.name, snapshot);
        }
    }

    fn generate_id(&self) -> u64 {
        self.next_inode.fetch_add(1, Ordering::Relaxed)
    }

    fn hydrate_tree(&mut self) {
        println!("FUSE: Hydrating Inode Tree from Database...");
        let files = self.manager.list_files();

        for path in files {
            if let Some((size, kind)) = self.manager.get_file_metadata(&path) {
                let ftype = match kind {
                    FileKind::File => FileType::RegularFile,
                    FileKind::Directory => FileType::Directory,
                };
                self.add_node_to_tree(&path, size, ftype);
            }
        }
    }

    fn add_node_to_tree(&self, path: &str, size: u64, kind: FileType) {
        let mut current = self.root.clone();
        let parts: Vec<&str> = path.split('/').collect();

        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            let is_last = i == parts.len() - 1;

            let mut node = current.write().unwrap();

            if !node.children.contains_key(*part) {
                let id = self.generate_id();
                let mut child_inode = Inode::new(id, if is_last {
                    kind
                } else {
                    FileType::Directory
                });

                if is_last {
                    child_inode.attr.size = size;
                    child_inode.attr.blocks = size.div_ceil(512);
                }

                let child_arc = Arc::new(RwLock::new(child_inode));
                self.inode_registry.write().unwrap().insert(id, child_arc.clone());
                node.children.insert(part.to_string(), child_arc);
            }

            let next = node.children.get(*part).unwrap().clone();
            drop(node);
            current = next;
        }
    }

    fn get_mutable_inode(&self, path: &str) -> Result<Arc<RwLock<Inode>>, i32> {
        if path.is_empty() {
            // Special case: modifying root directly
            if Arc::strong_count(&self.root) > 1 {
                println!(
                    "[CoW] WARNING: Root is shared but cannot be replaced (architectural limitation)"
                );
            }
            return Ok(self.root.clone());
        }

        let parts: Vec<&str> = path
            .split('/')
            .filter(|p| !p.is_empty())
            .collect();

        // Build the path from root, CoW-ing each shared node along the way
        let mut path_stack: Vec<(Arc<RwLock<Inode>>, String)> = vec![(
            self.root.clone(),
            String::new(),
        )];

        for part in parts.iter() {
            let current_arc = path_stack.last().unwrap().0.clone();
            let current = current_arc.read().unwrap();

            if let Some(child_arc) = current.children.get(*part) {
                let child_clone = child_arc.clone();
                drop(current); // Drop before mutable borrow
                path_stack.push((child_clone, part.to_string()));
            } else {
                return Err(ENOENT);
            }
        }

        // Now walk backwards, CoW-ing shared nodes
        for i in (0..path_stack.len()).rev() {
            let (node_arc, node_name) = &path_stack[i];

            if Arc::strong_count(node_arc) > 1 {
                if i == 0 {
                    // Root is shared - we can't replace it, but we can still modify its children
                    println!(
                        "[CoW] Root is shared (ref_count={}), but modifying children directly",
                        Arc::strong_count(node_arc)
                    );
                    // For root-level files, we'll clone them in-place in the root's children map
                    if path_stack.len() == 2 {
                        // Modifying a direct child of root
                        let root = self.root.write().unwrap();
                        let child_name = &path_stack[1].1;
                        if
                            let Some(child_arc) = root.children.get(child_name) &&
                            Arc::strong_count(child_arc) > 1
                        {
                            println!("[CoW] Cloning root-level node '{}' (keeping same ID)", child_name);
                            let old_child = child_arc.read().unwrap();
                            let new_child = old_child.clone(); // Keep same ID!
                            let old_id = new_child.id;
                            drop(old_child);

                            // Create new Arc with cloned data but SAME ID
                            let new_child_arc = Arc::new(RwLock::new(new_child));
                            self.inode_registry
                                .write()
                                .unwrap()
                                .insert(old_id, new_child_arc.clone());
                            drop(root);

                            // Update root's children map
                            self.root
                                .write()
                                .unwrap()
                                .children.insert(child_name.clone(), new_child_arc.clone());
                            return Ok(new_child_arc);
                        }
                    }
                } else {
                    // Clone this node and update parent's pointer (KEEP SAME ID)
                    println!("[CoW] Node '{}' is shared! Cloning (keeping same ID)...", node_name);

                    let old_inode = node_arc.read().unwrap();
                    let new_inode = old_inode.clone(); // Keep same ID
                    let same_id = new_inode.id;
                    drop(old_inode);

                    let new_arc = Arc::new(RwLock::new(new_inode));
                    self.inode_registry.write().unwrap().insert(same_id, new_arc.clone());

                    // Update parent's children map
                    let (parent_arc, _) = &path_stack[i - 1];
                    parent_arc.write().unwrap().children.insert(node_name.clone(), new_arc.clone());

                    // Update path_stack for remaining iterations
                    path_stack[i] = (new_arc.clone(), node_name.clone());
                }
            }
        }

        Ok(path_stack.last().unwrap().0.clone())
    }

    fn get_path_from_inode(&self, target_ino: u64) -> Option<String> {
        if target_ino == FUSE_ROOT_ID {
            return Some(String::new());
        }

        let mut queue = vec![(self.root.clone(), String::new())];

        while let Some((node_arc, path)) = queue.pop() {
            let node = node_arc.read().unwrap();

            if node.id == target_ino {
                return Some(path);
            }

            for (name, child_arc) in node.children.iter() {
                let child_path = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", path, name)
                };
                queue.push((child_arc.clone(), child_path));
            }
        }

        None
    }

    fn get_path_from_snapshot_tree(
        &self,
        root: &Arc<RwLock<Inode>>,
        target_ino: u64
    ) -> Option<String> {
        let mut queue = vec![(root.clone(), String::new())];

        while let Some((node_arc, path)) = queue.pop() {
            let node = node_arc.read().unwrap();

            if node.id == target_ino {
                return Some(path);
            }

            for (name, child_arc) in node.children.iter() {
                let child_path = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", path, name)
                };
                queue.push((child_arc.clone(), child_path));
            }
        }

        None
    }

    fn find_in_snapshot_tree(
        &self,
        root: &Arc<RwLock<Inode>>,
        target_ino: u64
    ) -> Option<Arc<RwLock<Inode>>> {
        let mut queue = vec![root.clone()];

        while let Some(node_arc) = queue.pop() {
            let node = node_arc.read().unwrap();

            if node.id == target_ino {
                drop(node);
                return Some(node_arc.clone());
            }

            for (_name, child_arc) in node.children.iter() {
                queue.push(child_arc.clone());
            }
        }

        None
    }
}

impl Filesystem for BetterFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name_str = name.to_str().unwrap();

        if parent == FUSE_ROOT_ID && name_str == ".snapshots" {
            return reply.entry(&TTL, &dir_attr(SNAPSHOT_DIR_ID), 0);
        }

        if parent == SNAPSHOT_DIR_ID {
            let snaps = self.snapshots.read().unwrap();
            if let Some(snap) = snaps.get(name_str) {
                let mut root_attr = snap.root.read().unwrap().attr;
                // Mark snapshot roots as read-only
                root_attr.perm = 0o555; // r-xr-xr-x
                return reply.entry(&TTL, &root_attr, 0);
            }
            return reply.error(ENOENT);
        }

        // Check if parent is inside a snapshot tree
        // First, check if parent matches any snapshot root
        let snaps = self.snapshots.read().unwrap();
        for (_snap_name, snapshot) in snaps.iter() {
            let snap_root = snapshot.root.read().unwrap();
            if parent == snap_root.id {
                // Looking up inside a snapshot root
                if let Some(child_arc) = snap_root.children.get(name_str) {
                    let child_guard = child_arc.read().unwrap();
                    let mut child_attr = child_guard.attr;
                    child_attr.perm = 0o555; // Read-only
                    return reply.entry(&TTL, &child_attr, 0);
                } else {
                    return reply.error(ENOENT);
                }
            }
            drop(snap_root);

            // Recursively search snapshot tree
            if let Some(node_arc) = self.find_in_snapshot_tree(&snapshot.root, parent) {
                let node = node_arc.read().unwrap();
                if let Some(child_arc) = node.children.get(name_str) {
                    let child_guard = child_arc.read().unwrap();
                    let mut child_attr = child_guard.attr;
                    child_attr.perm = 0o555; // Read-only
                    return reply.entry(&TTL, &child_attr, 0);
                }
                return reply.error(ENOENT);
            }
        }
        drop(snaps);

        // Not in snapshots, check live filesystem
        let registry = self.inode_registry.read().unwrap();
        if let Some(parent_node) = registry.get(&parent) {
            let parent_guard = parent_node.read().unwrap();

            if let Some(child_arc) = parent_guard.children.get(name_str) {
                let child_guard = child_arc.read().unwrap();
                reply.entry(&TTL, &child_guard.attr, 0);
            } else {
                reply.error(ENOENT);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        if ino == SNAPSHOT_DIR_ID {
            return reply.attr(&TTL, &dir_attr(SNAPSHOT_DIR_ID));
        }

        // Check if ino is in a snapshot tree
        let snaps = self.snapshots.read().unwrap();
        for (_snap_name, snapshot) in snaps.iter() {
            if let Some(node_arc) = self.find_in_snapshot_tree(&snapshot.root, ino) {
                let mut attr = node_arc.read().unwrap().attr;
                attr.perm = 0o555; // Read-only
                return reply.attr(&TTL, &attr);
            }
        }
        drop(snaps);

        let registry = self.inode_registry.read().unwrap();
        if let Some(node) = registry.get(&ino) {
            let guard = node.read().unwrap();
            reply.attr(&TTL, &guard.attr);
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory
    ) {
        if ino == SNAPSHOT_DIR_ID {
            if offset == 0 {
                let _ = reply.add(SNAPSHOT_DIR_ID, 0, FileType::Directory, ".");
                let _ = reply.add(FUSE_ROOT_ID, 1, FileType::Directory, "..");

                let snaps = self.snapshots.read().unwrap();
                for (i, (name, snapshot)) in snaps.iter().enumerate() {
                    let snap_root_id = snapshot.root.read().unwrap().id;
                    let _ = reply.add(snap_root_id, (i + 2) as i64, FileType::Directory, name);
                }
            }
            reply.ok();
            return;
        }

        // Check if ino is in a snapshot tree (prioritize snapshots over live tree)
        let snaps = self.snapshots.read().unwrap();
        for (_snap_name, snapshot) in snaps.iter() {
            // Check if this ino is the snapshot root itself or inside it
            let snap_root_id = snapshot.root.read().unwrap().id;
            if ino == snap_root_id {
                // This is a snapshot root directory
                let guard = snapshot.root.read().unwrap();

                if offset == 0 {
                    let _ = reply.add(ino, 0, FileType::Directory, ".");
                    let _ = reply.add(SNAPSHOT_DIR_ID, 1, FileType::Directory, "..");

                    for (i, (name, child_arc)) in guard.children.iter().enumerate() {
                        let child = child_arc.read().unwrap();
                        let _ = reply.add(child.id, (i + 2) as i64, child.attr.kind, name);
                    }
                }
                reply.ok();
                return;
            }

            // Check if it's somewhere inside this snapshot tree
            if let Some(node_arc) = self.find_in_snapshot_tree(&snapshot.root, ino) {
                let guard = node_arc.read().unwrap();

                if offset == 0 {
                    let _ = reply.add(ino, 0, FileType::Directory, ".");
                    let _ = reply.add(ino, 1, FileType::Directory, "..");

                    for (i, (name, child_arc)) in guard.children.iter().enumerate() {
                        let child = child_arc.read().unwrap();
                        let _ = reply.add(child.id, (i + 2) as i64, child.attr.kind, name);
                    }
                }
                reply.ok();
                return;
            }
        }
        drop(snaps);

        // Not in snapshot, check live filesystem
        let registry = self.inode_registry.read().unwrap();
        if let Some(node) = registry.get(&ino) {
            let guard = node.read().unwrap();

            if offset == 0 {
                let _ = reply.add(ino, 0, FileType::Directory, ".");
                let _ = reply.add(ino, 1, FileType::Directory, "..");

                for (i, (name, child_arc)) in guard.children.iter().enumerate() {
                    let child = child_arc.read().unwrap();
                    let _ = reply.add(child.id, (i + 2) as i64, child.attr.kind, name);
                }
            }
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData
    ) {
        // Try live filesystem first
        let path = self.get_path_from_inode(ino);

        // If not in live tree, search snapshots
        let path = if path.is_none() {
            let snaps = self.snapshots.read().unwrap();
            let mut found_path = None;
            for (_snap_name, snapshot) in snaps.iter() {
                if let Some(p) = self.get_path_from_snapshot_tree(&snapshot.root, ino) {
                    found_path = Some(p);
                    break;
                }
            }
            found_path
        } else {
            path
        };

        let path = match path {
            Some(p) => p,
            None => {
                return reply.error(ENOENT);
            }
        };

        match self.manager.read_file(&path) {
            Ok(data) => {
                let start = offset as usize;
                if start < data.len() {
                    let end = std::cmp::min(start + (size as usize), data.len());
                    reply.data(&data[start..end]);
                } else {
                    reply.data(&[]);
                }
            }
            Err(_) => reply.error(libc::EIO),
        }
    }

    fn write(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite
    ) {
        let path = match self.get_path_from_inode(ino) {
            Some(p) => p,
            None => {
                return reply.error(ENOENT);
            }
        };

        println!("[WRITE] Request to modify '{}'", path);

        match self.get_mutable_inode(&path) {
            Ok(inode_arc) => {
                let mut file_data = self.manager.read_file(&path).unwrap_or_default();

                let end = (offset as usize) + data.len();
                if end > file_data.len() {
                    file_data.resize(end, 0);
                }
                file_data[offset as usize..end].copy_from_slice(data);

                if self.manager.write_file(&path, &file_data).is_err() {
                    return reply.error(libc::EIO);
                }

                // Update inode size after successful write
                let mut inode = inode_arc.write().unwrap();
                inode.attr.size = file_data.len() as u64;
                inode.attr.blocks = (file_data.len() as u64).div_ceil(512);
                inode.attr.mtime = SystemTime::now();
                drop(inode);

                reply.written(data.len() as u32);
            }
            Err(e) => reply.error(e),
        }
    }

    fn create(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate
    ) {
        let name_str = name.to_str().unwrap();

        // [CoW] Trigger CoW on parent path before modification
        if let Some(parent_path) = self.get_path_from_inode(parent) {
            println!("[CREATE] Ensuring parent '{}' is mutable", if parent_path.is_empty() {
                "/"
            } else {
                &parent_path
            });
            let _ = self.get_mutable_inode(&parent_path);
        }

        let registry = self.inode_registry.read().unwrap();
        let parent_node = match registry.get(&parent) {
            Some(node) => node.clone(),
            None => {
                return reply.error(ENOENT);
            }
        };
        drop(registry);

        let mut parent_guard = parent_node.write().unwrap();

        let new_id = self.generate_id();
        let new_inode = Inode::new(new_id, FileType::RegularFile);
        let new_arc = Arc::new(RwLock::new(new_inode));

        self.inode_registry.write().unwrap().insert(new_id, new_arc.clone());
        parent_guard.children.insert(name_str.to_string(), new_arc.clone());

        let attr = new_arc.read().unwrap().attr;
        reply.created(&TTL, &attr, 0, 0, 0);
    }

    fn mkdir(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry
    ) {
        let name_str = name.to_str().unwrap();

        // Handle snapshot restoration
        if let Some(snap_name) = name_str.strip_prefix(".restore_") {
            println!("[CHRONOS] Restore requested: {}", snap_name);

            let snaps = self.snapshots.read().unwrap();
            if snaps.get(snap_name).is_some() {
                drop(snaps);

                // Auto-backup current state before restore
                let backup_name = format!(
                    "before_restore_{}",
                    SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                );
                println!("[CHRONOS] Auto-saving current state as '{}'", backup_name);

                // Create backup snapshot
                let live_root = self.root.read().unwrap();
                let mut backup_root_node = live_root.clone();
                drop(live_root);

                let backup_root_id = self.generate_id();
                backup_root_node.id = backup_root_id;
                backup_root_node.attr.ino = backup_root_id;
                let backup_root = Arc::new(RwLock::new(backup_root_node));

                let backup_timestamp = SystemTime::now();
                let mut snaps_write = self.snapshots.write().unwrap();
                snaps_write.insert(backup_name.clone(), Snapshot {
                    name: backup_name.clone(),
                    timestamp: backup_timestamp,
                    root: backup_root,
                });
                drop(snaps_write);

                // Persist backup
                let unix_ts = backup_timestamp
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let _ = self.manager.save_snapshot(&backup_name, unix_ts, backup_root_id);

                // Now restore: deep clone snapshot root to make it the new live root
                println!("[CHRONOS] Restoring snapshot '{}'...", snap_name);
                let snaps_read = self.snapshots.read().unwrap();
                let target_snap = snaps_read.get(snap_name).unwrap();
                let snap_root = target_snap.root.read().unwrap();
                let mut restored_root_node = snap_root.clone();
                drop(snap_root);
                drop(snaps_read);

                // Restored root keeps ID=1 for live filesystem
                restored_root_node.id = FUSE_ROOT_ID;
                restored_root_node.attr.ino = FUSE_ROOT_ID;

                // Swap the root
                self.root = Arc::new(RwLock::new(restored_root_node));

                // Update inode registry
                self.inode_registry.write().unwrap().insert(FUSE_ROOT_ID, self.root.clone());

                println!(
                    "[CHRONOS] ✓ Restored to snapshot '{}' (backup saved as '{}')",
                    snap_name,
                    backup_name
                );
                return reply.entry(&TTL, &dir_attr(9999), 0);
            } else {
                println!("[CHRONOS] Error: Snapshot '{}' not found", snap_name);
                return reply.error(ENOENT);
            }
        }

        if let Some(snap_name) = name_str.strip_prefix(".snap_") {
            println!("[CHRONOS] Taking Snapshot: {}", snap_name);

            // Deep clone the root node to freeze its state
            // This creates a NEW root with its own children HashMap
            let live_root = self.root.read().unwrap();
            let mut frozen_root_node = live_root.clone(); // Deep clone the Inode struct
            drop(live_root);

            // Assign unique ID for FUSE
            let snap_root_id = self.generate_id();
            frozen_root_node.id = snap_root_id;
            frozen_root_node.attr.ino = snap_root_id;

            let frozen_root = Arc::new(RwLock::new(frozen_root_node));
            let count = Arc::strong_count(&self.root);
            println!("[GC] Root Inode ref_count: {}", count);
            println!("[CHRONOS] Snapshot root ID: {} (live root ID: 1)", snap_root_id);

            let timestamp = SystemTime::now();
            let mut snaps = self.snapshots.write().unwrap();

            snaps.insert(snap_name.to_string(), Snapshot {
                name: snap_name.to_string(),
                timestamp,
                root: frozen_root,
            });

            // Persist snapshot metadata to disk
            let unix_timestamp = timestamp
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let root_id = self.root.read().unwrap().id;
            if let Err(e) = self.manager.save_snapshot(snap_name, unix_timestamp, root_id) {
                println!("[CHRONOS] Warning: Failed to persist snapshot: {}", e);
            }

            return reply.entry(&TTL, &dir_attr(9999), 0);
        }

        // [CoW] Trigger CoW on parent path before modification
        if let Some(parent_path) = self.get_path_from_inode(parent) {
            println!("[MKDIR] Ensuring parent '{}' is mutable", if parent_path.is_empty() {
                "/"
            } else {
                &parent_path
            });
            let _ = self.get_mutable_inode(&parent_path);
        }

        let registry = self.inode_registry.read().unwrap();
        let parent_node = match registry.get(&parent) {
            Some(node) => node.clone(),
            None => {
                return reply.error(ENOENT);
            }
        };
        drop(registry);

        let mut parent_guard = parent_node.write().unwrap();

        let new_id = self.generate_id();
        let new_inode = Inode::new(new_id, FileType::Directory);
        let new_arc = Arc::new(RwLock::new(new_inode));

        self.inode_registry.write().unwrap().insert(new_id, new_arc.clone());
        parent_guard.children.insert(name_str.to_string(), new_arc.clone());

        let child_guard = new_arc.read().unwrap();
        reply.entry(&TTL, &child_guard.attr, 0);
    }

    fn setattr(
        &mut self,
        _req: &Request,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<std::time::SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr
    ) {
        // [CoW] Trigger CoW if truncating/modifying file size
        if let Some(new_size) = size && let Some(path) = self.get_path_from_inode(ino) {
            println!("[SETATTR] Truncating '{}' to {} bytes", path, new_size);

            match self.get_mutable_inode(&path) {
                Ok(inode_arc) => {
                    // Perform truncation
                    let mut file_data = self.manager.read_file(&path).unwrap_or_default();
                    file_data.resize(new_size as usize, 0);

                    if self.manager.write_file(&path, &file_data).is_err() {
                        return reply.error(libc::EIO);
                    }

                    // Update inode attributes
                    let mut inode = inode_arc.write().unwrap();
                    inode.attr.size = new_size;
                    inode.attr.blocks = new_size.div_ceil(512);
                    inode.attr.mtime = SystemTime::now();
                    drop(inode);
                }
                Err(e) => {
                    return reply.error(e);
                }
            }
        }

        self.getattr(_req, ino, reply);
    }

    fn open(&mut self, _req: &Request, _ino: u64, _flags: i32, reply: ReplyOpen) {
        reply.opened(0, 0);
    }

    fn release(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty
    ) {
        reply.ok();
    }

    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = name.to_str().unwrap();

        let registry = self.inode_registry.read().unwrap();
        let parent_node = match registry.get(&parent) {
            Some(node) => node.clone(),
            None => {
                return reply.error(ENOENT);
            }
        };
        drop(registry);

        let mut parent_guard = parent_node.write().unwrap();

        if parent_guard.children.remove(name_str).is_some() {
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = name.to_str().unwrap();

        // Special case: Deleting snapshots
        if parent == SNAPSHOT_DIR_ID {
            let mut snaps = self.snapshots.write().unwrap();
            if snaps.remove(name_str).is_some() {
                println!("[CHRONOS] Deleted snapshot: {}", name_str);

                // Remove from persistent storage
                if let Err(e) = self.manager.delete_snapshot(name_str) {
                    println!("[CHRONOS] Warning: Failed to delete snapshot metadata: {}", e);
                }

                return reply.ok();
            }
            return reply.error(ENOENT);
        }

        self.unlink(_req, parent, name, reply);
    }
}
