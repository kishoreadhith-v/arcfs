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
use libc::{ EEXIST, EINVAL, EISDIR, ENOENT, EROFS };
use std::collections::{ HashMap, HashSet, VecDeque };
use std::ffi::OsStr;
use std::sync::atomic::{ AtomicU64, Ordering };
use std::sync::{ Arc, RwLock };
use std::time::{ Duration, SystemTime };

const TTL: Duration = Duration::from_secs(1);
const FUSE_ROOT_ID: u64 = 1;
const SNAPSHOT_DIR_ID: u64 = 2;
const SNAPSHOT_CREATE_ID: u64 = 3;
const TAGS_DIR_ID: u64 = 4;
const TAGFS_CONTROL_ID: u64 = 5;
const PAGE_CACHE_CAPACITY: usize = 1024;
const VIRTUAL_INODE_START: u64 = 1_000_000;

// ===========================================================================
// 1. DATA STRUCTURES
// ===========================================================================

#[derive(Clone)]
pub struct Inode {
    pub id: u64,
    pub parent_id: u64,
    pub name: String,
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
            parent_id: 0,
            name: String::new(),
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

#[derive(Clone)]
pub struct TagVirtualDirContext {
    pub tags: Vec<String>,
}

#[derive(Clone)]
pub struct TagVirtualFileContext {
    pub real_inode_id: u64,
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

fn snapshot_create_attr() -> FileAttr {
    let mut attr = dir_attr(SNAPSHOT_CREATE_ID);
    attr.kind = FileType::RegularFile;
    attr.perm = 0o644;
    attr.nlink = 1;
    attr.size = 0;
    attr
}

fn tagfs_control_attr() -> FileAttr {
    let mut attr = dir_attr(TAGFS_CONTROL_ID);
    attr.kind = FileType::RegularFile;
    attr.perm = 0o644;
    attr.nlink = 1;
    attr.size = 0;
    attr
}

pub struct ArcFS {
    pub manager: FileManager,
    pub inode_registry: Arc<RwLock<HashMap<u64, Arc<RwLock<Inode>>>>>,
    pub root: Arc<RwLock<Inode>>,
    pub snapshots: Arc<RwLock<HashMap<String, Snapshot>>>,
    pub page_cache: Arc<RwLock<HashMap<u64, (Vec<u8>, bool)>>>,
    pub cache_lru: Arc<RwLock<VecDeque<u64>>>,
    pub cache_capacity: usize,
    pub tag_virtual_dirs: Arc<RwLock<HashMap<u64, TagVirtualDirContext>>>,
    pub tag_virtual_files: Arc<RwLock<HashMap<u64, TagVirtualFileContext>>>,
    pub tag_dir_ids_by_key: Arc<RwLock<HashMap<String, u64>>>,
    pub tag_file_ids_by_key: Arc<RwLock<HashMap<String, u64>>>,
    next_vnode: AtomicU64,
    next_inode: AtomicU64,
}

impl ArcFS {
    pub fn new(manager: FileManager) -> Self {
        let registry = Arc::new(RwLock::new(HashMap::new()));
        let root = Arc::new(RwLock::new(Inode::new(FUSE_ROOT_ID, FileType::Directory)));
        registry.write().unwrap().insert(FUSE_ROOT_ID, root.clone());

        let mut fs = ArcFS {
            manager,
            inode_registry: registry,
            root,
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            page_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_lru: Arc::new(RwLock::new(VecDeque::new())),
            cache_capacity: PAGE_CACHE_CAPACITY,
            tag_virtual_dirs: Arc::new(RwLock::new(HashMap::new())),
            tag_virtual_files: Arc::new(RwLock::new(HashMap::new())),
            tag_dir_ids_by_key: Arc::new(RwLock::new(HashMap::new())),
            tag_file_ids_by_key: Arc::new(RwLock::new(HashMap::new())),
            next_vnode: AtomicU64::new(VIRTUAL_INODE_START),
            next_inode: AtomicU64::new(100),
        };

        {
            let root_guard = fs.root.read().unwrap();
            let _ = fs.manager.save_inode(&root_guard);
        }

        fs.hydrate_tree();
        fs.restore_snapshots();
        fs
    }

    // Restore snapshots from persistent storage
    fn restore_snapshots(&mut self) {
        let snapshot_metadata = self.manager.load_snapshots();

        for meta in snapshot_metadata {
            let mut visited = HashSet::new();
            let root = match self.restore_snapshot_subtree(meta.root_id, &mut visited) {
                Ok(root) => root,
                Err(e) => {
                    eprintln!(
                        "[CHRONOS] Failed restoring snapshot '{}' (root {}): {}",
                        meta.name,
                        meta.root_id,
                        e
                    );
                    if let Err(delete_err) = self.manager.delete_snapshot(&meta.name) {
                        eprintln!(
                            "[CHRONOS] Failed pruning stale snapshot metadata '{}': {}",
                            meta.name,
                            delete_err
                        );
                    } else {
                        eprintln!("[CHRONOS] Pruned stale snapshot metadata '{}'", meta.name);
                    }
                    continue;
                }
            };

            let snapshot = Snapshot {
                name: meta.name.clone(),
                timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(meta.timestamp),
                root,
            };

            self.snapshots.write().unwrap().insert(meta.name, snapshot);
        }
    }

    fn generate_id(&self) -> u64 {
        self.next_inode.fetch_add(1, Ordering::Relaxed)
    }

    fn hydrate_tree(&mut self) {
        println!("[BOOT] Reconstructing ArcFS Inode Tree...");
        self.hydrate_live_children(self.root.clone(), FUSE_ROOT_ID);
    }

    fn hydrate_live_children(&self, parent_arc: Arc<RwLock<Inode>>, parent_id: u64) {
        let dirents = match self.manager.list_dirents(parent_id) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("[BOOT] Failed loading dirents for {}: {}", parent_id, e);
                return;
            }
        };

        for dirent in dirents {
            let inode_meta = match self.manager.load_inode(dirent.child_inode_id) {
                Ok(Some(meta)) => meta,
                Ok(None) => {
                    continue;
                }
                Err(e) => {
                    eprintln!("[BOOT] Failed loading inode {}: {}", dirent.child_inode_id, e);
                    continue;
                }
            };

            let kind = if inode_meta.is_dir { FileType::Directory } else { FileType::RegularFile };

            let mut child_inode = Inode::new(inode_meta.id, kind);
            child_inode.parent_id = inode_meta.parent_id;
            child_inode.name = inode_meta.name.clone();
            child_inode.attr.size = inode_meta.attr.size;
            child_inode.attr.blocks = inode_meta.attr.size.div_ceil(512);

            let child_arc = Arc::new(RwLock::new(child_inode));
            self.inode_registry.write().unwrap().insert(inode_meta.id, child_arc.clone());

            parent_arc.write().unwrap().children.insert(dirent.name, child_arc.clone());

            let current_max = self.next_inode.load(Ordering::SeqCst);
            if inode_meta.id >= current_max {
                self.next_inode.store(inode_meta.id + 1, Ordering::SeqCst);
            }

            if inode_meta.is_dir {
                self.hydrate_live_children(child_arc, inode_meta.id);
            }
        }
    }

    fn restore_snapshot_subtree(
        &self,
        inode_id: u64,
        visited: &mut HashSet<u64>
    ) -> Result<Arc<RwLock<Inode>>, String> {
        if !visited.insert(inode_id) {
            return Err(format!("Cycle detected while restoring inode {}", inode_id));
        }

        let inode_meta = self.manager
            .load_inode(inode_id)?
            .ok_or_else(|| format!("Missing inode metadata for {}", inode_id))?;

        let kind = if inode_meta.is_dir { FileType::Directory } else { FileType::RegularFile };

        let mut inode = Inode::new(inode_meta.id, kind);
        inode.parent_id = inode_meta.parent_id;
        inode.name = inode_meta.name.clone();
        inode.attr.size = inode_meta.attr.size;
        inode.attr.blocks = inode_meta.attr.size.div_ceil(512);

        let node_arc = Arc::new(RwLock::new(inode));
        let children = self.manager.list_dirents(inode_id)?;

        for child in children {
            let child_arc = self.restore_snapshot_subtree(child.child_inode_id, visited)?;
            node_arc.write().unwrap().children.insert(child.name, child_arc);
        }

        Ok(node_arc)
    }

    fn clone_subtree_from_metadata(
        &self,
        source_inode_id: u64,
        parent_id: u64,
        node_name: &str,
        register_live: bool
    ) -> Result<Arc<RwLock<Inode>>, String> {
        let source_meta = self.manager
            .load_inode(source_inode_id)?
            .ok_or_else(|| format!("Missing source inode metadata {}", source_inode_id))?;

        let source_kind = if source_meta.is_dir {
            FileType::Directory
        } else {
            FileType::RegularFile
        };

        let new_id = self.generate_id();
        let mut new_inode = Inode::new(new_id, source_kind);
        new_inode.parent_id = parent_id;
        new_inode.name = node_name.to_string();
        new_inode.attr.size = source_meta.attr.size;
        new_inode.attr.blocks = source_meta.attr.size.div_ceil(512);

        self.manager.save_inode(&new_inode)?;
        self.manager.save_dirent(parent_id, node_name, new_id)?;

        if source_kind == FileType::RegularFile {
            let recipe = self.manager.load_recipe(source_inode_id)?.unwrap_or(FileRecipe {
                file_size: 0,
                chunks: Vec::new(),
                kind: FileKind::File,
            });
            self.manager.save_recipe(new_id, &recipe)?;
        }

        let new_arc = Arc::new(RwLock::new(new_inode));

        if register_live {
            self.inode_registry.write().unwrap().insert(new_id, new_arc.clone());
        }

        let children = self.manager.list_dirents(source_inode_id)?;
        for child in children {
            let cloned_child = self.clone_subtree_from_metadata(
                child.child_inode_id,
                new_id,
                &child.name,
                register_live
            )?;
            new_arc.write().unwrap().children.insert(child.name, cloned_child);
        }

        Ok(new_arc)
    }

    fn create_snapshot_named(&self, snap_name: &str) -> Result<(), String> {
        if self.snapshots.read().unwrap().contains_key(snap_name) {
            return Err(format!("Snapshot '{}' already exists", snap_name));
        }

        self.flush_all_dirty_cache()?;

        let frozen_root = self.clone_subtree_from_metadata(
            FUSE_ROOT_ID,
            SNAPSHOT_DIR_ID,
            snap_name,
            false
        )?;

        let timestamp = SystemTime::now();
        let root_id = frozen_root.read().unwrap().id;
        self.snapshots.write().unwrap().insert(snap_name.to_string(), Snapshot {
            name: snap_name.to_string(),
            timestamp,
            root: frozen_root,
        });

        let unix_timestamp = timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.manager.save_snapshot(snap_name, unix_timestamp, root_id)?;

        Ok(())
    }

    fn restore_live_from_snapshot(&self, snap_name: &str) -> Result<(), String> {
        let snapshot_root_id = {
            let snaps = self.snapshots.read().unwrap();
            let snapshot = snaps
                .get(snap_name)
                .ok_or_else(|| format!("Snapshot '{}' not found", snap_name))?;
            snapshot.root.read().unwrap().id
        };

        let old_root_dirents = self.manager.list_dirents(FUSE_ROOT_ID)?;
        for dirent in old_root_dirents {
            let _ = self.manager.delete_dirent(FUSE_ROOT_ID, &dirent.name);
        }

        {
            let mut root = self.root.write().unwrap();
            root.children.clear();
            root.id = FUSE_ROOT_ID;
            root.attr.ino = FUSE_ROOT_ID;
            root.name = String::new();
            root.parent_id = 0;
            root.attr.kind = FileType::Directory;
            root.attr.size = 0;
            root.attr.blocks = 0;
        }

        {
            let mut registry = self.inode_registry.write().unwrap();
            registry.clear();
            registry.insert(FUSE_ROOT_ID, self.root.clone());
        }

        self.page_cache.write().unwrap().clear();
        self.cache_lru.write().unwrap().clear();

        {
            let root_guard = self.root.read().unwrap();
            self.manager.save_inode(&root_guard)?;
        }

        let snap_children = self.manager.list_dirents(snapshot_root_id)?;
        for child in snap_children {
            let cloned_child = self.clone_subtree_from_metadata(
                child.child_inode_id,
                FUSE_ROOT_ID,
                &child.name,
                true
            )?;
            self.root.write().unwrap().children.insert(child.name, cloned_child);
        }

        Ok(())
    }

    fn inode_in_snapshot(&self, ino: u64) -> bool {
        let snaps = self.snapshots.read().unwrap();
        for (_name, snapshot) in snaps.iter() {
            if self.find_in_snapshot_tree(&snapshot.root, ino).is_some() {
                return true;
            }
        }
        false
    }

    fn touch_cache_entry(&self, ino: u64) {
        let mut lru = self.cache_lru.write().unwrap();
        if let Some(pos) = lru.iter().position(|id| *id == ino) {
            lru.remove(pos);
        }
        lru.push_back(ino);
    }

    fn evict_under_pressure(&self) -> Result<(), String> {
        loop {
            let cache_len = self.page_cache.read().unwrap().len();
            if cache_len <= self.cache_capacity {
                return Ok(());
            }

            let victim = {
                let mut lru = self.cache_lru.write().unwrap();
                let mut victim = None;
                while let Some(candidate) = lru.pop_front() {
                    if self.page_cache.read().unwrap().contains_key(&candidate) {
                        victim = Some(candidate);
                        break;
                    }
                }
                victim
            };

            let Some(victim_ino) = victim else {
                return Ok(());
            };

            let is_dirty = self.page_cache
                .read()
                .unwrap()
                .get(&victim_ino)
                .map(|(_, dirty)| *dirty)
                .unwrap_or(false);

            if is_dirty {
                self.flush_inode_cache(victim_ino)?;
            }

            self.page_cache.write().unwrap().remove(&victim_ino);
        }
    }

    fn flush_inode_cache(&self, ino: u64) -> Result<(), String> {
        let dirty_data = {
            let cache = self.page_cache.read().unwrap();
            match cache.get(&ino) {
                Some((buffer, true)) => Some(buffer.clone()),
                _ => None,
            }
        };

        if let Some(buffer) = dirty_data {
            self.manager.write_file_by_id(ino, &buffer)?;

            if let Some(node_arc) = self.inode_registry.read().unwrap().get(&ino).cloned() {
                let node = node_arc.read().unwrap();
                self.manager.save_inode(&node)?;
            }

            self.page_cache.write().unwrap().insert(ino, (buffer, false));
            self.touch_cache_entry(ino);
        }

        Ok(())
    }

    fn flush_all_dirty_cache(&self) -> Result<(), String> {
        let keys: Vec<u64> = self.page_cache.read().unwrap().keys().copied().collect();
        for ino in keys {
            self.flush_inode_cache(ino)?;
        }
        Ok(())
    }

    fn evict_inode_cache(&self, ino: u64) {
        self.page_cache.write().unwrap().remove(&ino);
        self.cache_lru
            .write()
            .unwrap()
            .retain(|id| *id != ino);
    }

    /// Walks the path, applying Copy-on-Write (Shadow Paging) to any shared nodes.
    /// Returns a mutable lock to the final requested file/directory.
    fn get_mutable_inode(&mut self, path: &str) -> Result<Arc<RwLock<Inode>>, i32> {
        // 1. If modifying the root directly, we MUST replace the root.
        if path.is_empty() {
            if Arc::strong_count(&self.root) > 1 {
                // TASK 1: The root is shared with a snapshot. We must deep clone it.
                // Step A: Read the old root.
                let old_root = self.root.read().unwrap();

                // Step B: Create a new Inode. Give it a NEW ID!
                let new_root = Inode {
                    id: old_root.id,
                    parent_id: old_root.parent_id,
                    name: old_root.name.clone(),
                    attr: old_root.attr.clone(),
                    recipe: old_root.recipe.clone(),
                    // We clone the children pointers. We will CoW them later if needed.
                    children: old_root.children.clone(),
                };

                drop(old_root);

                // Step C: Wrap it in a new Arc and overwrite self.root
                self.root = Arc::new(RwLock::new(new_root));

                // Note: The snapshot still holds the old root via its own Arc!
            }
            return Ok(self.root.clone());
        }

        // 2. Split the path into a Vector of strings (e.g., ["projects", "backend", "api.rs"])
        let parts: Vec<&str> = path
            .split('/')
            .filter(|p| !p.is_empty())
            .collect();

        // 3. We keep a tracking pointer as we walk down the tree. Start at the root.
        let mut current_arc = self.root.clone();

        // 4. Walk down the tree, checking every directory in the path.
        for part in parts.iter() {
            // We need to look at the current directory's children.
            let mut current_node = current_arc.write().unwrap();

            // Find the child in the hashmap.
            // `get_mut` gives us a mutable reference to the Arc inside the map.
            if let Some(child_arc_ref) = current_node.children.get_mut(*part) {
                // Is this child shared with a snapshot?
                if Arc::strong_count(child_arc_ref) > 1 {
                    // TASK 2: SHADOW PAGING (The actual CoW logic)
                    let old_child = child_arc_ref.read().unwrap();

                    let new_id = self.next_inode.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                    // Step A: Create the replacement Inode with a NEW ID.
                    let new_child = Inode {
                        id: new_id,
                        parent_id: old_child.parent_id,
                        name: old_child.name.clone(),
                        attr: old_child.attr.clone(),
                        recipe: old_child.recipe.clone(),
                        // We clone the children pointers. We will CoW them later if needed.
                        children: old_child.children.clone(),
                    };

                    // Step B: Wrap it in our armor.
                    let new_child_arc = Arc::new(RwLock::new(new_child));

                    // Step C: Update the Global Registry so the kernel can find the new ID!
                    self.inode_registry.write().unwrap().insert(new_id, new_child_arc.clone());

                    drop(old_child);

                    // Step D: Replace the pointer in the parent's Hash Map!
                    // *child_arc_ref safely overwrites the value in `current_node.children`
                    *child_arc_ref = new_child_arc.clone();
                }

                // Move our tracking pointer down to the child we just checked/cloned.
                let next_arc = child_arc_ref.clone();

                // Drop the write lock on the parent BEFORE we loop, or we deadlock!
                drop(current_node);

                current_arc = next_arc;
            } else {
                // If a part of the path doesn't exist, return ENOENT (Error No Entity)
                return Err(libc::ENOENT);
            }
        }

        // 5. We reached the bottom of the path. Return the final, safely isolated file.
        Ok(current_arc)
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

    fn canonicalize_tags(&self, tags: Vec<String>) -> Vec<String> {
        let mut out = Vec::new();
        for tag in tags {
            let normalized = tag.trim().to_lowercase();
            if !normalized.is_empty() && !out.contains(&normalized) {
                out.push(normalized);
            }
        }
        out.sort();
        out
    }

    fn tag_key(&self, tags: &[String]) -> String {
        tags.join("/")
    }

    fn tag_file_key(&self, tags: &[String], real_inode_id: u64) -> String {
        format!("{}|{}", self.tag_key(tags), real_inode_id)
    }

    fn is_tag_virtual_dir_inode(&self, ino: u64) -> bool {
        ino == TAGS_DIR_ID || self.tag_virtual_dirs.read().unwrap().contains_key(&ino)
    }

    fn is_tag_virtual_file_inode(&self, ino: u64) -> bool {
        self.tag_virtual_files.read().unwrap().contains_key(&ino)
    }

    fn tag_dir_attr(&self, ino: u64) -> FileAttr {
        let mut attr = dir_attr(ino);
        attr.perm = 0o755;
        attr
    }

    fn existing_tag_virtual_dir(&self, tags: &[String]) -> Option<u64> {
        let canonical = self.canonicalize_tags(tags.to_vec());
        let key = self.tag_key(&canonical);
        self.tag_dir_ids_by_key.read().unwrap().get(&key).copied()
    }

    fn get_or_create_tag_virtual_dir(&self, tags: &[String]) -> u64 {
        let canonical = self.canonicalize_tags(tags.to_vec());
        let key = self.tag_key(&canonical);

        if let Some(existing) = self.tag_dir_ids_by_key.read().unwrap().get(&key).copied() {
            return existing;
        }

        let virtual_ino = self.next_vnode.fetch_add(1, Ordering::Relaxed);
        self.tag_virtual_dirs.write().unwrap().insert(virtual_ino, TagVirtualDirContext {
            tags: canonical.clone(),
        });
        self.tag_dir_ids_by_key.write().unwrap().insert(key, virtual_ino);
        virtual_ino
    }

    fn get_or_create_tag_virtual_file(&self, tags: &[String], real_inode_id: u64) -> u64 {
        let canonical = self.canonicalize_tags(tags.to_vec());
        let key = self.tag_file_key(&canonical, real_inode_id);

        if let Some(existing) = self.tag_file_ids_by_key.read().unwrap().get(&key).copied() {
            return existing;
        }

        let virtual_ino = self.next_vnode.fetch_add(1, Ordering::Relaxed);
        self.tag_virtual_files
            .write()
            .unwrap()
            .insert(virtual_ino, TagVirtualFileContext { real_inode_id });
        self.tag_file_ids_by_key.write().unwrap().insert(key, virtual_ino);
        virtual_ino
    }

    fn get_virtual_dir_tags(&self, ino: u64) -> Option<Vec<String>> {
        self.tag_virtual_dirs
            .read()
            .unwrap()
            .get(&ino)
            .map(|ctx| ctx.tags.clone())
    }

    fn ensure_real_path_for_tags(&mut self, tags: &[String]) -> Result<u64, i32> {
        let canonical = self.canonicalize_tags(tags.to_vec());
        let mut current_parent = FUSE_ROOT_ID;

        for tag in canonical {
            let parent_arc = {
                let registry = self.inode_registry.read().unwrap();
                registry.get(&current_parent).cloned().ok_or(ENOENT)?
            };

            let mut parent_guard = parent_arc.write().unwrap();
            if let Some(existing_child) = parent_guard.children.get(&tag).cloned() {
                let child_guard = existing_child.read().unwrap();
                if child_guard.attr.kind != FileType::Directory {
                    return Err(EINVAL);
                }
                current_parent = child_guard.id;
                continue;
            }

            let new_id = self.generate_id();
            let mut new_inode = Inode::new(new_id, FileType::Directory);
            new_inode.name = tag.clone();
            new_inode.parent_id = current_parent;

            if self.manager.save_inode(&new_inode).is_err() {
                return Err(libc::EIO);
            }
            if self.manager.save_dirent(current_parent, &tag, new_id).is_err() {
                return Err(libc::EIO);
            }

            let new_arc = Arc::new(RwLock::new(new_inode));
            self.inode_registry.write().unwrap().insert(new_id, new_arc.clone());
            parent_guard.children.insert(tag, new_arc);
            current_parent = new_id;
        }

        Ok(current_parent)
    }

    fn create_live_file_under_parent(&mut self, parent: u64, name: &str) -> Result<u64, i32> {
        let new_id = self.generate_id();

        let mut new_inode = Inode::new(new_id, FileType::RegularFile);
        new_inode.name = name.to_string();
        new_inode.parent_id = parent;

        if self.manager.save_inode(&new_inode).is_err() {
            return Err(libc::EIO);
        }
        if self.manager.save_dirent(parent, name, new_id).is_err() {
            return Err(libc::EIO);
        }

        let recipe = FileRecipe {
            file_size: 0,
            chunks: vec![],
            kind: FileKind::File,
        };
        if self.manager.save_recipe(new_id, &recipe).is_err() {
            return Err(libc::EIO);
        }

        let auto_tags = self.derive_tags_from_parent_ancestry(parent);
        if !auto_tags.is_empty() {
            let _ = self.manager.set_file_tags(new_id, name, auto_tags);
        }

        let new_arc = Arc::new(RwLock::new(new_inode));
        if let Some(parent_node) = self.inode_registry.read().unwrap().get(&parent) {
            parent_node.write().unwrap().children.insert(name.to_string(), new_arc.clone());
        } else {
            return Err(ENOENT);
        }
        self.inode_registry.write().unwrap().insert(new_id, new_arc);
        Ok(new_id)
    }

    fn resolve_real_file_in_tag_dir(&self, parent_virtual: u64, name: &str) -> Result<u64, i32> {
        let Some(tags) = self.get_virtual_dir_tags(parent_virtual) else {
            return Err(ENOENT);
        };

        let candidates = self.manager.get_files_by_tags(&tags).map_err(|_| libc::EIO)?;
        let mut matches = Vec::new();
        for inode_id in candidates {
            if let Some(inode_name) = self.lookup_live_inode_name(inode_id) && inode_name == name {
                matches.push(inode_id);
            }
        }

        match matches.len() {
            0 => Err(ENOENT),
            1 => Ok(matches[0]),
            _ => Err(EINVAL),
        }
    }

    fn real_inode_for_virtual_file(&self, virtual_ino: u64) -> Option<u64> {
        self.tag_virtual_files
            .read()
            .unwrap()
            .get(&virtual_ino)
            .map(|ctx| ctx.real_inode_id)
    }

    fn derive_tags_from_parent_ancestry(&self, parent_inode_id: u64) -> Vec<String> {
        let mut tags = Vec::new();
        let mut current = parent_inode_id;

        while current != FUSE_ROOT_ID && current != 0 {
            match self.manager.load_inode(current) {
                Ok(Some(meta)) => {
                    if !meta.name.is_empty() {
                        tags.push(meta.name);
                    }
                    current = meta.parent_id;
                }
                _ => {
                    let registry = self.inode_registry.read().unwrap();
                    if let Some(node_arc) = registry.get(&current) {
                        let node = node_arc.read().unwrap();
                        if !node.name.is_empty() {
                            tags.push(node.name.clone());
                        }
                        current = node.parent_id;
                    } else {
                        break;
                    }
                }
            }
        }

        tags.reverse();
        tags
    }

    fn lookup_live_inode_name(&self, inode_id: u64) -> Option<String> {
        self.inode_registry
            .read()
            .unwrap()
            .get(&inode_id)
            .map(|node| node.read().unwrap().name.clone())
    }

    fn lookup_live_inode_parent(&self, inode_id: u64) -> Option<u64> {
        self.inode_registry
            .read()
            .unwrap()
            .get(&inode_id)
            .map(|node| node.read().unwrap().parent_id)
    }

    fn lookup_live_inode_attr(&self, inode_id: u64) -> Option<FileAttr> {
        self.inode_registry
            .read()
            .unwrap()
            .get(&inode_id)
            .map(|node| node.read().unwrap().attr)
    }

    fn resolve_tag_lookup(&self, parent: u64, name: &str) -> Result<Option<(u64, FileAttr)>, i32> {
        if parent == FUSE_ROOT_ID && name == "@tags" {
            return Ok(Some((TAGS_DIR_ID, self.tag_dir_attr(TAGS_DIR_ID))));
        }

        if parent == TAGS_DIR_ID {
            let files = self.manager.get_files_with_tag(name).map_err(|_| libc::EIO)?;

            if files.is_empty() {
                let tags = vec![name.to_string()];
                if let Some(existing) = self.existing_tag_virtual_dir(&tags) {
                    return Ok(Some((existing, self.tag_dir_attr(existing))));
                }
                return Ok(None);
            }

            let tags = vec![name.to_string()];
            let virtual_ino = self.get_or_create_tag_virtual_dir(&tags);
            return Ok(Some((virtual_ino, self.tag_dir_attr(virtual_ino))));
        }

        if let Some(current_tags) = self.get_virtual_dir_tags(parent) {
            let mut next_tags = current_tags.clone();
            next_tags.push(name.to_string());
            let next_tags = self.canonicalize_tags(next_tags);

            let tag_matches = self.manager.get_files_by_tags(&next_tags).map_err(|_| libc::EIO)?;

            if !tag_matches.is_empty() {
                let virtual_ino = self.get_or_create_tag_virtual_dir(&next_tags);
                return Ok(Some((virtual_ino, self.tag_dir_attr(virtual_ino))));
            }

            if let Some(existing) = self.existing_tag_virtual_dir(&next_tags) {
                return Ok(Some((existing, self.tag_dir_attr(existing))));
            }

            let current_matches = self.manager
                .get_files_by_tags(&current_tags)
                .map_err(|_| libc::EIO)?;
            for inode_id in current_matches {
                if
                    let Some(inode_name) = self.lookup_live_inode_name(inode_id) &&
                    inode_name == name &&
                    let Some(mut attr) = self.lookup_live_inode_attr(inode_id)
                {
                    let virtual_ino = self.get_or_create_tag_virtual_file(&current_tags, inode_id);
                    attr.ino = virtual_ino;
                    return Ok(Some((virtual_ino, attr)));
                }
            }

            return Ok(None);
        }

        Ok(None)
    }

    fn handle_tagfs_control_write(&self, data: &[u8]) -> Result<(), i32> {
        let command = String::from_utf8_lossy(data).trim().to_string();
        if command.is_empty() {
            return Err(EINVAL);
        }

        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(EINVAL);
        }

        match parts[0] {
            "set" => {
                if parts.len() < 3 {
                    return Err(EINVAL);
                }
                let path = parts[1];
                let tags: Vec<String> = parts[2..]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                self.manager.set_file_tags_by_path(path, tags).map_err(|_| libc::EIO)?;
                Ok(())
            }
            "del" => {
                let path = parts[1];
                let inode_id = self.manager
                    .resolve_inode_by_path(path)
                    .map_err(|_| libc::EIO)?
                    .ok_or(ENOENT)?;
                self.manager.delete_file_tags(inode_id).map_err(|_| libc::EIO)?;
                Ok(())
            }
            _ => Err(EINVAL),
        }
    }
}

impl Filesystem for ArcFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name_str = name.to_str().unwrap();

        if parent == FUSE_ROOT_ID && name_str == ".tagfs_ctl" {
            return reply.entry(&TTL, &tagfs_control_attr(), 0);
        }

        match self.resolve_tag_lookup(parent, name_str) {
            Ok(Some((_ino, attr))) => {
                return reply.entry(&TTL, &attr, 0);
            }
            Ok(None) => {}
            Err(code) => {
                return reply.error(code);
            }
        }

        if parent == FUSE_ROOT_ID && name_str == ".snapshots" {
            return reply.entry(&TTL, &dir_attr(SNAPSHOT_DIR_ID), 0);
        }

        if parent == SNAPSHOT_DIR_ID {
            if name_str == ".create" {
                return reply.entry(&TTL, &snapshot_create_attr(), 0);
            }

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
        if ino == TAGFS_CONTROL_ID {
            return reply.attr(&TTL, &tagfs_control_attr());
        }

        if ino == TAGS_DIR_ID {
            return reply.attr(&TTL, &self.tag_dir_attr(TAGS_DIR_ID));
        }

        if self.is_tag_virtual_dir_inode(ino) {
            return reply.attr(&TTL, &self.tag_dir_attr(ino));
        }

        if
            self.is_tag_virtual_file_inode(ino) &&
            let Some(real_ino) = self.real_inode_for_virtual_file(ino) &&
            let Some(mut attr) = self.lookup_live_inode_attr(real_ino)
        {
            attr.ino = ino;
            return reply.attr(&TTL, &attr);
        }

        if ino == SNAPSHOT_DIR_ID {
            return reply.attr(&TTL, &dir_attr(SNAPSHOT_DIR_ID));
        }

        if ino == SNAPSHOT_CREATE_ID {
            return reply.attr(&TTL, &snapshot_create_attr());
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

        // Check if ino is a TagFS virtual directory
        if let Some(_context) = self.get_virtual_dir_tags(ino) {
            return reply.attr(&TTL, &dir_attr(ino));
        }

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
        if ino == TAGS_DIR_ID {
            if offset == 0 {
                let _ = reply.add(TAGS_DIR_ID, 0, FileType::Directory, ".");
                let _ = reply.add(FUSE_ROOT_ID, 1, FileType::Directory, "..");

                let mut top_tags: HashSet<String> = self.manager
                    .get_next_level_tags(&[])
                    .unwrap_or_default()
                    .into_iter()
                    .collect();

                for ctx in self.tag_virtual_dirs.read().unwrap().values() {
                    if ctx.tags.len() == 1 {
                        top_tags.insert(ctx.tags[0].clone());
                    }
                }

                let mut top_tags_vec: Vec<String> = top_tags.into_iter().collect();
                top_tags_vec.sort();
                for (i, tag) in top_tags_vec.iter().enumerate() {
                    let virtual_ino = self.get_or_create_tag_virtual_dir(std::slice::from_ref(tag));
                    let _ = reply.add(virtual_ino, (i + 2) as i64, FileType::Directory, tag);
                }
            }
            reply.ok();
            return;
        }

        if let Some(current_tags) = self.get_virtual_dir_tags(ino) {
            if offset == 0 {
                let _ = reply.add(ino, 0, FileType::Directory, ".");
                let _ = reply.add(TAGS_DIR_ID, 1, FileType::Directory, "..");

                let mut next_offset = 2i64;
                let mut next_tags: HashSet<String> = self.manager
                    .get_next_level_tags(&current_tags)
                    .unwrap_or_default()
                    .into_iter()
                    .collect();

                let current_set: HashSet<String> = current_tags.iter().cloned().collect();
                for ctx in self.tag_virtual_dirs.read().unwrap().values() {
                    if ctx.tags.len() == current_tags.len() + 1 {
                        let ctx_set: HashSet<String> = ctx.tags.iter().cloned().collect();
                        if current_set.is_subset(&ctx_set) {
                            for tag in &ctx.tags {
                                if !current_set.contains(tag) {
                                    next_tags.insert(tag.clone());
                                }
                            }
                        }
                    }
                }

                let mut next_tags_vec: Vec<String> = next_tags.into_iter().collect();
                next_tags_vec.sort();
                for tag in next_tags_vec {
                    let mut extended = current_tags.clone();
                    extended.push(tag.clone());
                    let virtual_ino = self.get_or_create_tag_virtual_dir(&extended);
                    let _ = reply.add(virtual_ino, next_offset, FileType::Directory, tag);
                    next_offset += 1;
                }

                let matching_inodes = self.manager
                    .get_files_by_tags(&current_tags)
                    .unwrap_or_default();
                for inode_id in matching_inodes {
                    if let Some(name) = self.lookup_live_inode_name(inode_id) {
                        let virtual_ino = self.get_or_create_tag_virtual_file(
                            &current_tags,
                            inode_id
                        );
                        let _ = reply.add(virtual_ino, next_offset, FileType::RegularFile, name);
                        next_offset += 1;
                    }
                }
            }
            reply.ok();
            return;
        }

        if ino == SNAPSHOT_DIR_ID {
            if offset == 0 {
                let _ = reply.add(SNAPSHOT_DIR_ID, 0, FileType::Directory, ".");
                let _ = reply.add(FUSE_ROOT_ID, 1, FileType::Directory, "..");
                let _ = reply.add(SNAPSHOT_CREATE_ID, 2, FileType::RegularFile, ".create");

                let snaps = self.snapshots.read().unwrap();
                for (i, (name, snapshot)) in snaps.iter().enumerate() {
                    let snap_root_id = snapshot.root.read().unwrap().id;
                    let _ = reply.add(snap_root_id, (i + 3) as i64, FileType::Directory, name);
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

                let mut next_offset = 2i64;
                if ino == FUSE_ROOT_ID {
                    let _ = reply.add(
                        SNAPSHOT_DIR_ID,
                        next_offset,
                        FileType::Directory,
                        ".snapshots"
                    );
                    next_offset += 1;
                    let _ = reply.add(TAGS_DIR_ID, next_offset, FileType::Directory, "@tags");
                    next_offset += 1;
                    let _ = reply.add(
                        TAGFS_CONTROL_ID,
                        next_offset,
                        FileType::RegularFile,
                        ".tagfs_ctl"
                    );
                    next_offset += 1;
                }

                for (i, (name, child_arc)) in guard.children.iter().enumerate() {
                    let child = child_arc.read().unwrap();
                    let _ = reply.add(child.id, next_offset + (i as i64), child.attr.kind, name);
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
        if ino == TAGFS_CONTROL_ID {
            return reply.data(&[]);
        }

        if self.is_tag_virtual_dir_inode(ino) {
            return reply.error(EISDIR);
        }

        let target_ino = self.real_inode_for_virtual_file(ino).unwrap_or(ino);

        if ino == SNAPSHOT_CREATE_ID {
            return reply.data(&[]);
        }

        // 1. Check if the Inode exists in our live registry or snapshots
        let registry = self.inode_registry.read().unwrap();
        let exists =
            registry.contains_key(&target_ino) ||
            ({
                let snaps = self.snapshots.read().unwrap();
                snaps
                    .iter()
                    .any(|(_name, snapshot)|
                        self.find_in_snapshot_tree(&snapshot.root, target_ino).is_some()
                    )
            });
        drop(registry);

        if !exists {
            return reply.error(libc::ENOENT);
        }

        // 2. Handle the Data Retrieval efficiently
        // Ensure file is in cache first, without cloning
        let in_cache = self.page_cache.read().unwrap().contains_key(&target_ino);
        if !in_cache {
            if let Ok(data) = self.manager.read_file_by_id(target_ino) {
                // Populate cache with freshly read data
                self.page_cache.write().unwrap().insert(target_ino, (data, false));
                let _ = self.evict_under_pressure();
            } else {
                // Return empty if file retrieval fails (e.g. empty file)
                return reply.data(&[]);
            }
        }

        self.touch_cache_entry(target_ino);

        // Serve directly from the cache reference
        let cache = self.page_cache.read().unwrap();
        if let Some((data, _)) = cache.get(&target_ino) {
            let start = offset as usize;
            if start < data.len() {
                let end = std::cmp::min(start + (size as usize), data.len());
                reply.data(&data[start..end]);
            } else {
                reply.data(&[]);
            }
        } else {
            reply.data(&[]);
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
        let target_ino = self.real_inode_for_virtual_file(ino).unwrap_or(ino);

        if ino == TAGFS_CONTROL_ID {
            if offset != 0 {
                return reply.error(EINVAL);
            }

            return match self.handle_tagfs_control_write(data) {
                Ok(_) => reply.written(data.len() as u32),
                Err(code) => reply.error(code),
            };
        }

        if self.is_tag_virtual_dir_inode(ino) {
            return reply.error(EROFS);
        }

        if ino == SNAPSHOT_CREATE_ID {
            if offset != 0 {
                return reply.error(EINVAL);
            }

            let snap_name = String::from_utf8_lossy(data).trim().to_string();
            if snap_name.is_empty() {
                return reply.error(EINVAL);
            }

            return match self.create_snapshot_named(&snap_name) {
                Ok(_) => reply.written(data.len() as u32),
                Err(e) if e.contains("already exists") => reply.error(EEXIST),
                Err(_) => reply.error(libc::EIO),
            };
        }

        if self.inode_in_snapshot(target_ino) {
            return reply.error(EROFS);
        }

        // 1. In-place Write: Overlay new data directly into the write cache to prevent O(N^2) memory cloning
        let end = (offset as usize) + data.len();
        {
            let mut cache_map = self.page_cache.write().unwrap();
            let entry = cache_map
                .entry(target_ino)
                .or_insert_with(|| {
                    (self.manager.read_file_by_id(target_ino).unwrap_or_default(), false)
                });

            if end > entry.0.len() {
                entry.0.resize(end, 0);
            }
            entry.0[offset as usize..end].copy_from_slice(data);
            entry.1 = true; // Mark as dirty
        }

        self.touch_cache_entry(target_ino);

        // 2. Trigger potential eviction if memory pressure is high
        if self.evict_under_pressure().is_err() {
            return reply.error(libc::EIO);
        }

        if let Some(node_arc) = self.inode_registry.read().unwrap().get(&target_ino) {
            let mut node = node_arc.write().unwrap();
            if node.attr.size < (end as u64) {
                node.attr.size = end as u64;
                node.attr.blocks = node.attr.size.div_ceil(512);
            }
            node.attr.mtime = SystemTime::now();
        }

        if self.manager.get_file_tags(target_ino).unwrap_or_default().is_empty() {
            let filename = self
                .lookup_live_inode_name(target_ino)
                .or_else(|| {
                    self.manager
                        .load_inode(target_ino)
                        .ok()
                        .flatten()
                        .map(|meta| meta.name)
                })
                .unwrap_or_default();

            let parent_ino = self
                .lookup_live_inode_parent(target_ino)
                .or_else(|| {
                    self.manager
                        .load_inode(target_ino)
                        .ok()
                        .flatten()
                        .map(|meta| meta.parent_id)
                })
                .unwrap_or(FUSE_ROOT_ID);

            let auto_tags = self.derive_tags_from_parent_ancestry(parent_ino);
            if !auto_tags.is_empty() && !filename.is_empty() {
                let _ = self.manager.set_file_tags(target_ino, &filename, auto_tags);
            }
        }

        reply.written(data.len() as u32);
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
        if parent == SNAPSHOT_DIR_ID || self.inode_in_snapshot(parent) {
            return reply.error(EROFS);
        }

        let name_str = name.to_str().unwrap().to_string();

        let mut real_parent = parent;
        let mut virtual_tags: Option<Vec<String>> = None;

        if parent == TAGS_DIR_ID {
            return reply.error(EINVAL);
        }

        if
            self.is_tag_virtual_dir_inode(parent) &&
            let Some(tags) = self.get_virtual_dir_tags(parent)
        {
            match self.ensure_real_path_for_tags(&tags) {
                Ok(parent_ino) => {
                    real_parent = parent_ino;
                    virtual_tags = Some(tags);
                }
                Err(code) => {
                    return reply.error(code);
                }
            }
        }

        let new_id = match self.create_live_file_under_parent(real_parent, &name_str) {
            Ok(id) => id,
            Err(code) => {
                return reply.error(code);
            }
        };

        let mut attr = match self.lookup_live_inode_attr(new_id) {
            Some(a) => a,
            None => {
                return reply.error(libc::EIO);
            }
        };

        if let Some(tags) = virtual_tags {
            let virtual_ino = self.get_or_create_tag_virtual_file(&tags, new_id);
            attr.ino = virtual_ino;
        }

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

        if parent == TAGS_DIR_ID {
            let tags = vec![name_str.to_string()];
            let virtual_ino = self.get_or_create_tag_virtual_dir(&tags);
            if let Err(code) = self.ensure_real_path_for_tags(&tags) {
                return reply.error(code);
            }
            return reply.entry(&TTL, &self.tag_dir_attr(virtual_ino), 0);
        }

        if
            self.is_tag_virtual_dir_inode(parent) &&
            let Some(mut tags) = self.get_virtual_dir_tags(parent)
        {
            tags.push(name_str.to_string());
            let canonical = self.canonicalize_tags(tags);
            let virtual_ino = self.get_or_create_tag_virtual_dir(&canonical);
            if let Err(code) = self.ensure_real_path_for_tags(&canonical) {
                return reply.error(code);
            }
            return reply.entry(&TTL, &self.tag_dir_attr(virtual_ino), 0);
        }

        if parent == SNAPSHOT_DIR_ID && name_str != ".create" {
            return reply.error(EROFS);
        }

        if self.inode_in_snapshot(parent) {
            return reply.error(EROFS);
        }

        // Handle snapshot restoration
        if let Some(snap_name) = name_str.strip_prefix(".restore_") {
            println!("[CHRONOS] Restore requested: {}", snap_name);

            let snap_exists = self.snapshots.read().unwrap().contains_key(snap_name);
            if snap_exists {
                // Auto-backup current state before restore
                let backup_name = format!(
                    "before_restore_{}",
                    SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                );
                println!("[CHRONOS] Auto-saving current state as '{}'", backup_name);

                if let Err(e) = self.create_snapshot_named(&backup_name) {
                    println!(
                        "[CHRONOS] Error: Failed to create backup snapshot '{}': {}",
                        backup_name,
                        e
                    );
                    return reply.error(libc::EIO);
                }

                // Restore the snapshot content into live filesystem and persist it
                println!("[CHRONOS] Restoring snapshot '{}'...", snap_name);
                if let Err(e) = self.restore_live_from_snapshot(snap_name) {
                    println!("[CHRONOS] Error: Failed to restore snapshot '{}': {}", snap_name, e);
                    return reply.error(libc::EIO);
                }

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

            return match self.create_snapshot_named(snap_name) {
                Ok(_) => reply.entry(&TTL, &dir_attr(9999), 0),
                Err(e) if e.contains("already exists") => reply.error(EEXIST),
                Err(_) => reply.error(libc::EIO),
            };
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
        let mut new_inode = Inode::new(new_id, FileType::Directory);
        new_inode.name = name_str.to_string();
        new_inode.parent_id = parent;

        if self.manager.save_inode(&new_inode).is_err() {
            return reply.error(libc::EIO);
        }
        if self.manager.save_dirent(parent, name_str, new_id).is_err() {
            return reply.error(libc::EIO);
        }

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
        if self.is_tag_virtual_dir_inode(ino) {
            return reply.error(EROFS);
        }

        let target_ino = self.real_inode_for_virtual_file(ino).unwrap_or(ino);

        if self.inode_in_snapshot(target_ino) {
            return reply.error(EROFS);
        }

        if let Some(new_size) = size {
            // Modify size in-place cache to prevent monolithic clones
            {
                let mut cache_map = self.page_cache.write().unwrap();
                let entry = cache_map
                    .entry(target_ino)
                    .or_insert_with(|| {
                        (self.manager.read_file_by_id(target_ino).unwrap_or_default(), false)
                    });
                entry.0.resize(new_size as usize, 0);
                entry.1 = true;
            }

            self.touch_cache_entry(target_ino);
            if self.evict_under_pressure().is_err() {
                return reply.error(libc::EIO);
            }

            if let Some(node_arc) = self.inode_registry.read().unwrap().get(&target_ino) {
                let mut node = node_arc.write().unwrap();
                node.attr.size = new_size;
                node.attr.blocks = new_size.div_ceil(512);
                node.attr.mtime = SystemTime::now();
            }
        }

        // Always return the current attributes
        self.getattr(_req, ino, reply);
    }

    fn open(&mut self, _req: &Request, _ino: u64, _flags: i32, reply: ReplyOpen) {
        if self.is_tag_virtual_dir_inode(_ino) {
            return reply.error(EISDIR);
        }

        if _ino == TAGFS_CONTROL_ID {
            return reply.opened(0, 0);
        }

        reply.opened(0, 0);
    }

    fn fsync(&mut self, _req: &Request, ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty) {
        if self.is_tag_virtual_dir_inode(ino) {
            return reply.error(EROFS);
        }

        let target_ino = self.real_inode_for_virtual_file(ino).unwrap_or(ino);
        if self.flush_inode_cache(target_ino).is_ok() {
            reply.ok();
        } else {
            reply.error(libc::EIO);
        }
    }

    fn release(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty
    ) {
        if self.is_tag_virtual_dir_inode(ino) {
            return reply.error(EROFS);
        }

        let target_ino = self.real_inode_for_virtual_file(ino).unwrap_or(ino);
        if self.flush_inode_cache(target_ino).is_ok() {
            reply.ok();
        } else {
            reply.error(libc::EIO);
        }
    }

    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        if parent == SNAPSHOT_DIR_ID || self.inode_in_snapshot(parent) {
            return reply.error(EROFS);
        }

        let name_str = name.to_str().unwrap();

        if self.is_tag_virtual_dir_inode(parent) {
            let real_ino = match self.resolve_real_file_in_tag_dir(parent, name_str) {
                Ok(ino) => ino,
                Err(code) => {
                    return reply.error(code);
                }
            };

            let (real_parent, real_name) = {
                let registry = self.inode_registry.read().unwrap();
                let Some(node_arc) = registry.get(&real_ino) else {
                    return reply.error(ENOENT);
                };
                let node = node_arc.read().unwrap();
                (node.parent_id, node.name.clone())
            };

            let registry = self.inode_registry.read().unwrap();
            let parent_node = match registry.get(&real_parent) {
                Some(node) => node.clone(),
                None => {
                    return reply.error(ENOENT);
                }
            };
            drop(registry);

            let mut parent_guard = parent_node.write().unwrap();
            if parent_guard.children.remove(&real_name).is_some() {
                let _ = self.manager.delete_dirent(real_parent, &real_name);
                let _ = self.manager.delete_file_tags(real_ino);
                self.evict_inode_cache(real_ino);
                return reply.ok();
            }
            return reply.error(ENOENT);
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

        if let Some(removed_node) = parent_guard.children.remove(name_str) {
            let removed_ino = removed_node.read().unwrap().id;
            let _ = self.manager.delete_dirent(parent, name_str);
            let _ = self.manager.delete_file_tags(removed_ino);
            self.evict_inode_cache(removed_ino);
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = name.to_str().unwrap();

        if parent == TAGS_DIR_ID || self.is_tag_virtual_dir_inode(parent) {
            return reply.error(EINVAL);
        }

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
