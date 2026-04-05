# TagFS API Surface & Implementation Checklist

**Quick Reference for Rebuilding TagFS from Scratch**

---

## FileManager API (file_manager.rs)

All tag operations are in `impl FileManager` block.

### Add This Data Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTagSet {
    pub file_id: u64,
    pub filename: String,
    pub tags: Vec<String>,
}
```

### Implement These Methods

```rust
impl FileManager {
    /// Store tags for a file
    pub fn set_file_tags(
        &self,
        inode_id: u64,
        filename: &str,
        tags: Vec<String>,
    ) -> Result<(), String> {
        let tag_set = FileTagSet {
            file_id: inode_id,
            filename: filename.to_string(),
            tags: tags.clone(),
        };
        let key = format!("file_tags:{}", inode_id);
        let encoded = bincode::serialize(&tag_set)?;
        self.db.insert(key.as_bytes(), encoded)?;
        self.db.flush()?;
        println!("[TAGFS] Set tags for inode {}: {:?}", inode_id, tags);
        Ok(())
    }

    /// Retrieve tags for a file
    pub fn get_file_tags(&self, inode_id: u64) -> Result<Vec<String>, String> {
        let key = format!("file_tags:{}", inode_id);
        match self.db.get(key.as_bytes()) {
            Ok(Some(bytes)) => {
                let tag_set: FileTagSet = bincode::deserialize(&bytes)?;
                Ok(tag_set.tags)
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Get all files (inode IDs) with a specific tag
    pub fn get_files_with_tag(&self, tag: &str) -> Result<Vec<u64>, String> {
        let mut result = Vec::new();
        for item in self.db.scan_prefix(b"file_tags:").flatten() {
            if let Ok(tag_set) = bincode::deserialize::<FileTagSet>(&item.1) {
                if tag_set.tags.contains(&tag.to_string()) {
                    result.push(tag_set.file_id);
                }
            }
        }
        Ok(result)
    }

    /// Get all files (inode IDs) with ALL tags (intersection)
    pub fn get_files_by_tags(&self, tags: &[String]) -> Result<Vec<u64>, String> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut candidates = self.get_files_with_tag(&tags[0])?;
        for tag in &tags[1..] {
            let files_with_tag = self.get_files_with_tag(tag)?;
            candidates.retain(|ino| files_with_tag.contains(ino));
        }
        println!("[TAGFS] Query for tags {:?} returned {} files", tags, candidates.len());
        Ok(candidates)
    }

    /// Get possible next-level tags for navigation
    pub fn get_next_level_tags(
        &self,
        current_tags: &[String],
    ) -> Result<Vec<String>, String> {
        let mut next_tags = std::collections::HashSet::new();
        let matching = self.get_files_by_tags(current_tags)?;
        
        for inode_id in matching {
            if let Ok(all_tags) = self.get_file_tags(inode_id) {
                for tag in all_tags {
                    if !current_tags.contains(&tag) {
                        next_tags.insert(tag);
                    }
                }
            }
        }
        
        let mut result: Vec<_> = next_tags.into_iter().collect();
        result.sort();
        println!("[TAGFS] Next-level tags for {:?}: {:?}", current_tags, result);
        Ok(result)
    }

    /// Delete tags for a file
    pub fn delete_file_tags(&self, inode_id: u64) -> Result<(), String> {
        let key = format!("file_tags:{}", inode_id);
        self.db.remove(key.as_bytes())?;
        self.db.flush()?;
        println!("[TAGFS] Deleted tags for inode {}", inode_id);
        Ok(())
    }
}
```

---

## ArcFS Structure Changes (fuse_handler.rs)

### Add These Fields

```rust
pub struct ArcFS {
    // ... existing fields ...
    
    /// Virtual directory contexts: virtual_inode_id → (tags, children, etc.)
    pub virtual_dir_cache: Arc<RwLock<HashMap<u64, VirtualDirContext>>>,
    
    /// Reverse map: inode_id → tags (mkdir'd dirs)
    pub inode_to_tags: Arc<RwLock<HashMap<u64, Vec<String>>>>,
    
    /// Index: tag → [inode_ids] for fast forward lookups
    pub tag_index: Arc<RwLock<HashMap<String, Vec<u64>>>>,
    
    /// Cache: inode_id → tags for quick reverse lookups
    pub inode_tag_cache: Arc<RwLock<HashMap<u64, Vec<String>>>>,
    
    /// Counter for generating unique virtual inode IDs (start at 10000)
    pub next_vnode: AtomicU64,
}

pub struct VirtualDirContext {
    pub path: String,                           // e.g., "/@tags/a/b"
    pub tags: Vec<String>,                      // e.g., ["a", "b"]
    pub virtual_inode_id: u64,
    pub children: HashMap<String, u64>,         // mkdir'd children
}
```

### Modify ArcFS::new()

```rust
pub fn new(manager: FileManager) -> Self {
    // ... existing initialization ...
    
    ArcFS {
        manager,
        inode_registry: registry,
        root,
        snapshots: Arc::new(RwLock::new(HashMap::new())),
        virtual_dir_cache: Arc::new(RwLock::new(HashMap::new())),      // NEW
        inode_to_tags: Arc::new(RwLock::new(HashMap::new())),          // NEW
        tag_index: Arc::new(RwLock::new(HashMap::new())),              // NEW
        inode_tag_cache: Arc::new(RwLock::new(HashMap::new())),        // NEW
        next_inode: AtomicU64::new(100),
        next_vnode: AtomicU64::new(10000),                             // NEW
    }
}
```

### Add to ArcFS Impl Block

```rust
impl ArcFS {
    // ===========================
    // TAGFS Phase 3: Helpers
    // ===========================
    
    /// Generate or retrieve a virtual inode ID for a tag set
    fn get_or_create_virtual_inode(&self, tags: &[String]) -> u64 {
        let cache = self.virtual_dir_cache.read().unwrap();
        for (vnode_id, context) in cache.iter() {
            if context.tags == tags {
                return *vnode_id;
            }
        }
        drop(cache);
        
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
    
    /// Find all inodes matching a set of tags (intersection)
    fn find_inodes_by_tags(&self, tags: &[String]) -> Vec<u64> {
        if tags.is_empty() {
            return Vec::new();
        }
        
        let tag_index = self.tag_index.read().unwrap();
        let first_tag_inodes = match tag_index.get(&tags[0]) {
            Some(inodes) => inodes.clone(),
            None => return Vec::new(),
        };
        
        let mut result = Vec::new();
        for inode_id in first_tag_inodes {
            if let Some(inode_tags) = self.inode_tag_cache.read().unwrap().get(&inode_id) {
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
        let matching_inodes = self.find_inodes_by_tags(current_tags);
        
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
    
    /// Rebuild tag indexes from persistent storage on startup
    fn hydrate_tree(&mut self) {
        let registry = self.inode_registry.read().unwrap();
        
        for (inode_id, _) in registry.iter() {
            match self.manager.get_file_tags(*inode_id) {
                Ok(tags) if !tags.is_empty() => {
                    self.inode_tag_cache
                        .write()
                        .unwrap()
                        .insert(*inode_id, tags.clone());
                    
                    for tag in &tags {
                        self.tag_index
                            .write()
                            .unwrap()
                            .entry(tag.clone())
                            .or_insert_with(Vec::new)
                            .push(*inode_id);
                    }
                    
                    println!("[TAGFS] Loaded tags for inode {}: {:?}", inode_id, tags);
                }
                _ => {}
            }
        }
        
        println!("[TAGFS] Hydration complete. Tag index: {} tags",
                 self.tag_index.read().unwrap().len());
    }
}
```

---

## Filesystem Implementation Changes

### Modify lookup()

```rust
fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
    let name_str = name.to_str().unwrap();
    
    // ===== TIER 1: Virtual Directory Navigation =====
    if let Some(context) = self.get_virtual_dir_context(parent) {
        // Check if child was created via mkdir
        if let Some(&child_inode_id) = context.children.get(name_str) {
            if let Some(child_node) = self.inode_registry.read().unwrap().get(&child_inode_id) {
                let attr = child_node.read().unwrap().attr.clone();
                println!("[TAGFS] Found child in virtual dir: {} -> {}", name_str, child_inode_id);
                return reply.entry(&TTL, &attr, 0);
            }
        }
        
        // Try to extend tags
        let mut next_tags = context.tags.clone();
        next_tags.push(name_str.to_string());
        
        println!("[TAGFS] Extending tags {:?} + {}", context.tags, name_str);
        
        match self.manager.get_files_by_tags(&next_tags) {
            Ok(file_ids) if !file_ids.is_empty() => {
                let vnode_id = self.get_or_create_virtual_inode(&next_tags);
                println!("[TAGFS] ✓ Found virtual dir for tags {:?}", next_tags);
                return reply.entry(&TTL, &dir_attr(vnode_id), 0);
            }
            Ok(_) => {
                // Maybe it's a filename?
                if let Ok(file_ids) = self.manager.get_files_by_tags(&context.tags) {
                    for file_id in file_ids {
                        if let Some(node_arc) = self.inode_registry.read().unwrap().get(&file_id) {
                            let node = node_arc.read().unwrap();
                            if node.attr.ino == file_id {
                                println!("[TAGFS] ✓ Found file by name in tag context");
                                return reply.entry(&TTL, &node.attr, 0);
                            }
                        }
                    }
                }
            }
            Err(_) => return reply.error(ENOENT),
        }
    }
    
    // ===== TIER 2: Root-Level Tag Discovery =====
    if parent == FUSE_ROOT_ID {
        println!("[TAGFS] Root lookup for '{}' - checking tags first", name_str);
        
        match self.manager.get_files_with_tag(name_str) {
            Ok(file_ids) if !file_ids.is_empty() => {
                let single_tag = vec![name_str.to_string()];
                let vnode_id = self.get_or_create_virtual_inode(&single_tag);
                println!("[TAGFS] ✓ Tag directory found: {}", name_str);
                return reply.entry(&TTL, &dir_attr(vnode_id), 0);
            }
            _ => {
                println!("[TAGFS] Not a tag, checking live tree");
            }
        }
    }
    
    // ===== TIER 3: Live Filesystem Tree =====
    let registry = self.inode_registry.read().unwrap();
    if let Some(parent_node) = registry.get(&parent) {
        let parent_guard = parent_node.read().unwrap();
        if let Some(child_arc) = parent_guard.children.get(name_str) {
            let attr = child_arc.read().unwrap().attr.clone();
            println!("[TAGFS] ✓ Found in live tree: {}", name_str);
            return reply.entry(&TTL, &attr, 0);
        }
    }
    
    println!("[TAGFS] ✗ Not found: {}", name_str);
    reply.error(ENOENT);
}
```

### Modify create()

Add this code block in the file creation section:

```rust
// After creating inode and registering it...

// ===== TAGFS: Auto-tagging =====
let parent_path = self.get_path_from_inode(parent).unwrap_or_default();
let tags_for_this_file: Vec<String> = parent_path
    .split('/')
    .filter(|s| !s.is_empty())
    .map(|s| s.to_string())
    .collect();

if !tags_for_this_file.is_empty() {
    match self.manager.set_file_tags(new_id, name_str, tags_for_this_file.clone()) {
        Ok(_) => {
            println!("[TAGFS] Stored tags for inode {}: {:?}", new_id, tags_for_this_file);
            
            // Update in-memory caches
            self.inode_tag_cache
                .write()
                .unwrap()
                .insert(new_id, tags_for_this_file.clone());
            
            for tag in &tags_for_this_file {
                self.tag_index
                    .write()
                    .unwrap()
                    .entry(tag.clone())
                    .or_insert_with(Vec::new)
                    .push(new_id);
            }
        }
        Err(e) => {
            println!("[TAGFS] Warning: Failed to tag file: {}", e);
        }
    }
}
```

### Modify readdir()

Add this code block at the beginning:

```rust
// ===== TAGFS: Virtual Directory Listing =====
if let Some(context) = self.get_virtual_dir_context(ino) {
    println!("[TAGFS] Readdir virtual dir: {:?}", context.tags);
    
    if offset == 0 {
        let _ = reply.add(ino, 0, FileType::Directory, ".");
        let _ = reply.add(FUSE_ROOT_ID, 1, FileType::Directory, "..");
        
        let mut entry_offset = 2i64;
        
        // List files matching current tags
        if let Ok(file_ids) = self.manager.get_files_by_tags(&context.tags) {
            for file_id in file_ids {
                if let Some(node_arc) = self.inode_registry.read().unwrap().get(&file_id) {
                    let node = node_arc.read().unwrap();
                    let name = format!("file_{}", file_id);
                    let _ = reply.add(file_id, entry_offset, node.attr.kind, &name);
                    entry_offset += 1;
                }
            }
        }
        
        // List possible next tags
        match self.manager.get_next_level_tags(&context.tags) {
            Ok(next_tags) => {
                for tag in next_tags {
                    let mut extended = context.tags.clone();
                    extended.push(tag.clone());
                    let tag_vnode = self.get_or_create_virtual_inode(&extended);
                    let _ = reply.add(tag_vnode, entry_offset, FileType::Directory, &tag);
                    entry_offset += 1;
                }
            }
            Err(_) => {}
        }
    }
    
    reply.ok();
    return;
}

// Original readdir logic continues...
```

---

## Initialization (main.rs)

After creating the FUSE filesystem, call:

```rust
fs.hydrate_tree();  // Rebuild tag indexes from persistent storage
```

---

## Constants

```rust
const FUSE_ROOT_ID: u64 = 1;
const TTL: &Timespec = &Timespec { sec: 0, nsec: 0 };
```

---

## Testing Checklist

```rust
✓ Tag storage and retrieval
✓ Single-tag queries
✓ Multi-tag AND intersection
✓ All 6 permutations of 3-tag file return same inode
✓ Next-level tag discovery
✓ Tag persistence across restart
✓ No tag leakage between files
✓ Empty tag handling
✓ Virtual directory listing
✓ Read/write via any tag permutation
```

---

## Debugging Checklist

- [ ] Is `hydrate_tree()` called on startup?
- [ ] Are both `inode_tag_cache` and `tag_index` updated during file creation?
- [ ] Virtual inode IDs not colliding with real inodes? (use 10000+)
- [ ] All three storage layers synchronized?
- [ ] Tag queries returning correct intersection results?
- [ ] Virtual directory listing showing files AND next tags?
- [ ] Are file reads/writes working via any permutation?

