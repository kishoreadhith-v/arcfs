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
use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::atomic::{ AtomicU64, Ordering };
use std::sync::{ Arc, RwLock };
use std::time::{ Duration, SystemTime };

const TTL: Duration = Duration::from_secs(1);
const FUSE_ROOT_ID: u64 = 1;
const SNAPSHOT_DIR_ID: u64 = 2;
const TAGFS_ROOT_ID: u64 = 3;

// TagFS virtual inode ID range: 50000-99999
const TAGFS_VNODE_START: u64 = 50000;

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

// TagFS Phase 3: Virtual directory context for tag-based paths
#[derive(Debug, Clone)]
pub struct VirtualDirContext {
    pub path: String,                    // e.g., "/@tags/work/2026"
    pub tags: Vec<String>,               // e.g., ["work", "2026"]
    pub virtual_inode_id: u64,           // e.g., 50001
    pub children: HashMap<String, u64>,  // Maps child name -> inode (real or virtual)
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
    pub virtual_dir_cache: Arc<RwLock<HashMap<u64, VirtualDirContext>>>, // TagFS: virtual inode → context
    pub inode_to_tags: Arc<RwLock<HashMap<u64, Vec<String>>>>, // TagFS: real inode → tags (for mkdir dirs)
    pub tag_index: Arc<RwLock<HashMap<String, Vec<u64>>>>, // TagFS: tag -> set of inode IDs
    pub inode_tag_cache: Arc<RwLock<HashMap<u64, Vec<String>>>>, // TagFS: inode -> tags (denormalized)
    next_inode: AtomicU64,
    next_vnode: AtomicU64, // Counter for virtual inode IDs
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
            virtual_dir_cache: Arc::new(RwLock::new(HashMap::new())), // TagFS: new cache
            inode_to_tags: Arc::new(RwLock::new(HashMap::new())), // TagFS: new reverse map
            tag_index: Arc::new(RwLock::new(HashMap::new())), // TagFS: tag lookup index
            inode_tag_cache: Arc::new(RwLock::new(HashMap::new())), // TagFS: inode tag cache
            next_inode: AtomicU64::new(100),
            next_vnode: AtomicU64::new(TAGFS_VNODE_START), // Start from 50000
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

    // ===========================
    // TAGFS Phase 3: Helpers
    // ===========================

    /// Check if a path is a TagFS path (starts with /@tags/)
    fn is_tagfs_path(path: &str) -> bool {
        path.starts_with("/@tags/") || path == "/@tags"
    }

    /// Extract tags from a TagFS path
    /// Examples:
    ///   "/@tags/work/2026" → ["work", "2026"]
    ///   "/@tags/work" → ["work"]
    fn extract_tags_from_path(path: &str) -> Vec<String> {
        if !Self::is_tagfs_path(path) {
            return Vec::new();
        }

        // Remove "/@tags/" prefix and split by '/'
        let without_prefix = path.strip_prefix("/@tags/").unwrap_or("");
        without_prefix
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Generate or retrieve a virtual inode ID for a tag set
    fn get_or_create_virtual_inode(&self, tags: &[String]) -> u64 {
        // Check if we already have this path cached
        let cache = self.virtual_dir_cache.read().unwrap();
        for (vnode_id, context) in cache.iter() {
            if context.tags == tags {
                return *vnode_id;
            }
        }
        drop(cache);

        // Create new virtual inode
        let vnode_id = self.next_vnode.fetch_add(1, Ordering::Relaxed);
        let path = format!("/@tags/{}", tags.join("/"));
        let context = VirtualDirContext {
            path,
            tags: tags.to_vec(),
            virtual_inode_id: vnode_id,
            children: HashMap::new(),
        };

        self.virtual_dir_cache.write().unwrap().insert(vnode_id, context);
        println!("[TAGFS] Created virtual inode {} for tags: {:?}", vnode_id, tags);
        vnode_id
    }

    /// Retrieve virtual directory context by inode ID
    fn get_virtual_dir_context(&self, inode_id: u64) -> Option<VirtualDirContext> {
        self.virtual_dir_cache.read().unwrap().get(&inode_id).cloned()
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
        
        // ===========================
        // TAGFS Phase 3: Build Tag Index on Startup
        // ===========================
        println!("[TAGFS] Building tag index from sled database...");
        
        // Scan all inodes in the registry
        let registry = self.inode_registry.read().unwrap();
        for inode_id in registry.keys() {
            // Try to load tags for this inode from manager
            if let Ok(tags) = self.manager.get_file_tags(*inode_id) {
                if !tags.is_empty() {
                    println!("[TAGFS] Inode {}: tags = {:?}", inode_id, tags);
                    
                    // Store in inode_tag_cache
                    self.inode_tag_cache.write().unwrap().insert(*inode_id, tags.clone());
                    
                    // Update tag_index (tag -> list of inodes)
                    for tag in &tags {
                        self.tag_index
                            .write()
                            .unwrap()
                            .entry(tag.clone())
                            .or_insert_with(Vec::new)
                            .push(*inode_id);
                    }
                }
            }
        }
        drop(registry);
        
        println!("[TAGFS] Tag index built with {} tags", self.tag_index.read().unwrap().len());
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
        // ===========================
        // Live Filesystem Path (Original Logic)
        // Files are stored in the live tree, NOT in canonical locations
        // Tags are just metadata for enabling tag-based queries
        // ===========================
        
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

    // ===========================
    // TAGFS Phase 3: Tag Index Queries
    // ===========================
    
    /// Find all inodes matching a set of tags (intersection - all tags required)
    fn find_inodes_by_tags(&self, tags: &[String]) -> Vec<u64> {
        if tags.is_empty() {
            return Vec::new();
        }
        
        let tag_index = self.tag_index.read().unwrap();
        
        // Start with inodes matching the first tag
        let first_tag_inodes = match tag_index.get(&tags[0]) {
            Some(inodes) => inodes.clone(),
            None => return Vec::new(),
        };
        
        // Filter to only inodes matching ALL tags
        let mut result = Vec::new();
        for inode_id in first_tag_inodes {
            if let Some(inode_tags) = self.inode_tag_cache.read().unwrap().get(&inode_id) {
                // Check if this inode has all required tags
                if tags.iter().all(|tag| inode_tags.contains(tag)) {
                    result.push(inode_id);
                }
            }
        }
        
        result
    }
    
    /// Get all possible next tags given a set of current tags
    fn find_next_level_tags(&self, current_tags: &[String]) -> Vec<String> {
        let mut next_tags = std::collections::HashSet::new();
        
        // Find all inodes with current tags
        let matching_inodes = self.find_inodes_by_tags(current_tags);
        
        // For each matching inode, get its tags and find those not in current_tags
        for inode_id in matching_inodes {
            if let Some(inode_tags) = self.inode_tag_cache.read().unwrap().get(&inode_id) {
                for tag in inode_tags {
                    if !current_tags.contains(tag) {
                        next_tags.insert(tag.clone());
                    }
                }
            }
        }
        
        let mut tags_vec: Vec<_> = next_tags.into_iter().collect();
        tags_vec.sort();
        tags_vec
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

        // ===========================
        // TAGFS Phase 3: Transparent Tag Lookup
        // ===========================
        
        // Strategy: First try live tree, then try tag system
        // No special /@tags/ prefix needed - transparent to user
        
        // Check if parent is a TagFS virtual directory
        if let Some(context) = self.get_virtual_dir_context(parent) {
            // We're inside a virtual tag directory
            
            // First: Check if child was created via mkdir in this virtual dir
            if let Some(&child_inode_id) = context.children.get(name_str) {
                if let Some(child_node) = self.inode_registry.read().unwrap().get(&child_inode_id) {
                    let child_guard = child_node.read().unwrap();
                    println!("[TAGFS] Found child in virtual dir context: {} -> inode {}", name_str, child_inode_id);
                    let attr = child_guard.attr.clone();
                    return reply.entry(&TTL, &attr, 0);
                }
            }
            
            // Second: Try to extend tags with the new component
            let mut next_tags = context.tags.clone();
            next_tags.push(name_str.to_string());
            
            println!("[TAGFS] In virtual dir {:?}, trying tag: {}", context.tags, name_str);
            
            // Query: Do files exist with all these tags?
            match self.manager.get_files_by_tags(&next_tags) {
                Ok(file_ids) if !file_ids.is_empty() => {
                    // At least one file matches these tags
                    let vnode_id = self.get_or_create_virtual_inode(&next_tags);
                    println!("[TAGFS] ✓ Found virtual dir for tags {:?}, inode {}", next_tags, vnode_id);
                    return reply.entry(&TTL, &dir_attr(vnode_id), 0);
                }
                Ok(_) => {
                    // No files with these tags, but maybe it's a filename in the current tag dir?
                    // Try querying files that match CURRENT tags and see if any have this filename
                    if let Ok(file_ids) = self.manager.get_files_by_tags(&context.tags) {
                        for file_id in file_ids {
                            // Check if this file has the name we're looking for
                            if let Some(tags_vec) = self.inode_tag_cache.read().unwrap().get(&file_id).cloned() {
                                // Try to get filename from the live tree
                                if let Some(node_arc) = self.inode_registry.read().unwrap().get(&file_id) {
                                    let node = node_arc.read().unwrap();
                                    // For now, just return it - we found a matching file
                                    println!("[TAGFS] ✓ Found file in tag dir: {} (inode {})", name_str, file_id);
                                    return reply.entry(&TTL, &node.attr, 0);
                                }
                            }
                        }
                    }
                    
                    // None found, check if it's a valid next tag
                    match self.manager.get_next_level_tags(&context.tags) {
                        Ok(next_level) if next_level.contains(&name_str.to_string()) => {
                            // This tag exists as a next-level option
                            let vnode_id = self.get_or_create_virtual_inode(&next_tags);
                            println!("[TAGFS] ✓ Valid next tag: {:?}, inode {}", next_tags, vnode_id);
                            return reply.entry(&TTL, &dir_attr(vnode_id), 0);
                        }
                        _ => {
                            println!("[TAGFS] ✗ Not a valid tag combination or file: {:?}", next_tags);
                            return reply.error(ENOENT);
                        }
                    }
                }
                Err(e) => {
                    println!("[TAGFS] Tag query error: {}", e);
                    return reply.error(ENOENT);
                }
            }
        }

        // ===========================
        // TagFS: Root-level lookups prioritize tags over live tree
        // ===========================
        
        if parent == FUSE_ROOT_ID {
            println!("[TAGFS] Root lookup for '{}' - checking tags first...", name_str);
            
            // Check if this name exists as a tag (files have this tag)
            match self.manager.get_files_with_tag(name_str) {
                Ok(file_ids) if !file_ids.is_empty() => {
                    let single_tag = vec![name_str.to_string()];
                    let vnode_id = self.get_or_create_virtual_inode(&single_tag);
                    println!("[TAGFS] ✓ Found tag directory for '{}', inode {}", name_str, vnode_id);
                    return reply.entry(&TTL, &dir_attr(vnode_id), 0);
                }
                _ => {
                    // No tag found - fall through to live tree check below
                    println!("[TAGFS] No tag found for '{}', checking live tree...", name_str);
                }
            }
        }

        // ===========================
        // Live Filesystem (Original Logic)
        // ===========================
        
        // Check: Is it in the live tree?
        {
            let registry = self.inode_registry.read().unwrap();
            if let Some(parent_node) = registry.get(&parent) {
                let parent_guard = parent_node.read().unwrap();
                if let Some(child_arc) = parent_guard.children.get(name_str) {
                    let child_guard = child_arc.read().unwrap();
                    println!("[FS] Live filesystem: found {}", name_str);
                    let attr = child_guard.attr.clone();
                    return reply.entry(&TTL, &attr, 0);
                }
            }
        } // registry and parent_guard dropped here

        // ===========================
        // Snapshots (Existing)
        // ===========================
        
        if parent == FUSE_ROOT_ID && name_str == ".snapshots" {
            return reply.entry(&TTL, &dir_attr(SNAPSHOT_DIR_ID), 0);
        }

        if parent == SNAPSHOT_DIR_ID {
            let snaps = self.snapshots.read().unwrap();
            if let Some(snap) = snaps.get(name_str) {
                let mut root_attr = snap.root.read().unwrap().attr;
                root_attr.perm = 0o555;
                return reply.entry(&TTL, &root_attr, 0);
            }
            return reply.error(ENOENT);
        }

        // Check if parent is inside a snapshot tree
        let snaps = self.snapshots.read().unwrap();
        for (_snap_name, snapshot) in snaps.iter() {
            let snap_root = snapshot.root.read().unwrap();
            if parent == snap_root.id {
                if let Some(child_arc) = snap_root.children.get(name_str) {
                    let child_guard = child_arc.read().unwrap();
                    let mut child_attr = child_guard.attr;
                    child_attr.perm = 0o555;
                    return reply.entry(&TTL, &child_attr, 0);
                } else {
                    return reply.error(ENOENT);
                }
            }
            drop(snap_root);

            if let Some(node_arc) = self.find_in_snapshot_tree(&snapshot.root, parent) {
                let node = node_arc.read().unwrap();
                if let Some(child_arc) = node.children.get(name_str) {
                    let child_guard = child_arc.read().unwrap();
                    let mut child_attr = child_guard.attr;
                    child_attr.perm = 0o555;
                    return reply.entry(&TTL, &child_attr, 0);
                }
                return reply.error(ENOENT);
            }
        }
        drop(snaps);

        // Default: not found
        reply.error(ENOENT);
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
        // ===========================
        // TAGFS Phase 3c: Readdir for Virtual Directories
        // ===========================
        
        if let Some(context) = self.get_virtual_dir_context(ino) {
            println!("[TAGFS] Readdir virtual dir with tags: {:?}", context.tags);
            
            if offset == 0 {
                let _ = reply.add(ino, 0, FileType::Directory, ".");
                let _ = reply.add(FUSE_ROOT_ID, 1, FileType::Directory, "..");
                
                let mut entry_offset = 2i64;
                
                // Get all files matching current tag set
                if let Ok(file_ids) = self.manager.get_files_by_tags(&context.tags) {
                    for file_id in file_ids {
                        // We need to get the filename for this inode
                        // For now, use inode ID as name (not ideal, will improve in 3d)
                        if let Some(parent_node) = self.inode_registry.read().unwrap().get(&file_id) {
                            let parent_guard = parent_node.read().unwrap();
                            let filename = format!("file_{}", file_id);
                            let _ = reply.add(file_id, entry_offset, FileType::RegularFile, &filename);
                            entry_offset += 1;
                            println!("[TAGFS]   → File: {} (inode {})", filename, file_id);
                        }
                    }
                }
                
                // Get next-level tags as subdirectories
                if let Ok(next_tags) = self.manager.get_next_level_tags(&context.tags) {
                    for tag in next_tags {
                        let mut tag_path = context.tags.clone();
                        tag_path.push(tag.clone());
                        let vnode_id = self.get_or_create_virtual_inode(&tag_path);
                        let _ = reply.add(vnode_id, entry_offset, FileType::Directory, &tag);
                        entry_offset += 1;
                        println!("[TAGFS]   → Tag: {} (inode {})", tag, vnode_id);
                    }
                }
            }
            reply.ok();
            return;
        }

        // ===========================
        // Snapshots (Existing)
        // ===========================
        
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

        // ===========================
        // TAGFS Phase 3d: Auto-tagging on Write
        // ===========================
        
        // ===========================
        // Handle Canonical Tagged File Paths
        // ===========================
        
        if path.starts_with("/_tagged_files/") {
            // This is a canonical path for a tagged file
            // Skip CoW logic and directly write
            println!("[TAGFS] Writing to canonical tagged file path: {}", path);
            
            let mut file_data = self.manager.read_file(&path).unwrap_or_default();
            
            let end = (offset as usize) + data.len();
            if end > file_data.len() {
                file_data.resize(end, 0);
            }
            file_data[offset as usize..end].copy_from_slice(data);
            
            if self.manager.write_file(&path, &file_data).is_err() {
                return reply.error(libc::EIO);
            }
            
            // Update inode size
            if let Some(inode_id) = path.strip_prefix("/_tagged_files/").and_then(|s| s.parse::<u64>().ok()) {
                if let Some(inode_arc) = self.inode_registry.read().unwrap().get(&inode_id) {
                    let mut inode = inode_arc.write().unwrap();
                    inode.attr.size = file_data.len() as u64;
                    inode.attr.blocks = (file_data.len() as u64).div_ceil(512);
                    inode.attr.mtime = SystemTime::now();
                }
            }
            
            return reply.written(data.len() as u32);
        }
        
        // ===========================
        // Live Filesystem Write (Original Logic with CoW)
        // ===========================
        
        // Check if the file being written is in a virtual tag directory
        // (by checking if its parent is a virtual inode)
        let parent_is_virtual = self.get_virtual_dir_context(self.inode_registry
            .read()
            .unwrap()
            .values()
            .next()
            .map(|_| 0u64)
            .unwrap_or(0)).is_some(); // Simplified check
        
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
                
                // AUTO-TAG: If we know the tags for this inode, store them
                // (This will be improved when we track the parent context better)
                
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
        println!("[CREATE] Creating '{}' in parent inode {}", name_str, parent);

        // ===========================
        // TAGFS: Auto-tag files based on parent directory path
        // ===========================
        
        // Get all parent directories from root to this parent
        // This will become the file's tags
        let tags_for_this_file: Vec<String> = if let Some(parent_path) = self.get_path_from_inode(parent) {
            // Extract directory names from the path as tags
            parent_path
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };
        
        if !tags_for_this_file.is_empty() {
            println!("[TAGFS] Auto-tagging file '{}' with tags: {:?}", name_str, tags_for_this_file);
        }

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

        let new_id = self.generate_id();
        let new_inode = Inode::new(new_id, FileType::RegularFile);
        let new_arc = Arc::new(RwLock::new(new_inode));

        self.inode_registry.write().unwrap().insert(new_id, new_arc.clone());
        
        let mut parent_guard = parent_node.write().unwrap();
        parent_guard.children.insert(name_str.to_string(), new_arc.clone());
        drop(parent_guard);

        // Store tags for this file if it has tags
        if !tags_for_this_file.is_empty() {
            if let Err(e) = self.manager.set_file_tags(new_id, name_str, tags_for_this_file.clone()) {
                println!("[TAGFS] Warning: Failed to tag file: {}", e);
            } else {
                println!("[TAGFS] Stored tags for inode {}: {:?}", new_id, tags_for_this_file);
                
                // Update in-memory caches
                self.inode_tag_cache.write().unwrap().insert(new_id, tags_for_this_file.clone());
                
                // Update tag_index  
                for tag in &tags_for_this_file {
                    self.tag_index
                        .write()
                        .unwrap()
                        .entry(tag.clone())
                        .or_insert_with(Vec::new)
                        .push(new_id);
                }
            }
        }

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
            let live_root = self.root.read().unwrap();
            let mut frozen_root_node = live_root.clone();
            drop(live_root);

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

        // Standard mkdir - just create a regular directory
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
