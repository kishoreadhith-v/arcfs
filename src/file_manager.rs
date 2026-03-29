use crate::chunker::Chunker;
use crate::storage::Storage;
use fuser::FileType;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecipe {
    pub file_size: u64,
    pub chunks: Vec<String>,
    pub kind: FileKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InodeAttr {
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InodeMetadata {
    pub id: u64,
    pub parent_id: u64,
    pub name: String,
    pub is_dir: bool,
    pub attr: InodeAttr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dirent {
    pub parent_id: u64,
    pub name: String,
    pub child_inode_id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SnapshotMetadata {
    pub name: String,
    pub timestamp: u64,
    pub root_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTagSet {
    pub file_id: u64,
    pub filename: String,
    pub tags: Vec<String>,
}

pub struct FileManager {
    storage: Storage,
    db: sled::Db,
}

impl FileManager {
    pub fn new(storage_path: &str) -> Self {
        let storage = Storage::new(storage_path);
        let db_path = Path::new(storage_path).join("metadata_db");
        let db = sled::open(db_path).expect("Failed to open metadata database");
        Self { storage, db }
    }

    fn inode_key(id: u64) -> String {
        format!("ino_meta:{}", id)
    }

    fn recipe_key(id: u64) -> String {
        format!("ino_recipe:{}", id)
    }

    fn dirent_key(parent_id: u64, name: &str) -> String {
        format!("dirent:{}:{}", parent_id, name)
    }

    fn dirent_prefix(parent_id: u64) -> String {
        format!("dirent:{}:", parent_id)
    }

    fn ino_tags_key(inode_id: u64) -> String {
        format!("ino_tags:{}", inode_id)
    }

    fn tag_index_key(tag: &str) -> String {
        format!("tag_index:{}", tag)
    }

    fn normalize_tags(tags: Vec<String>) -> Vec<String> {
        let mut dedup = HashSet::new();
        for tag in tags {
            let normalized = tag.trim().to_lowercase();
            if !normalized.is_empty() {
                dedup.insert(normalized);
            }
        }

        let mut out: Vec<String> = dedup.into_iter().collect();
        out.sort();
        out
    }

    fn load_tag_index_set(&self, tag: &str) -> Result<HashSet<u64>, String> {
        let key = Self::tag_index_key(tag);
        let value = self
            .db
            .get(key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?;

        match value {
            Some(bytes) => {
                let ids = bincode::deserialize::<Vec<u64>>(&bytes)
                    .map_err(|e| format!("Tag index deserialization error: {}", e))?;
                Ok(ids.into_iter().collect())
            }
            None => Ok(HashSet::new()),
        }
    }

    fn save_tag_index_set(&self, tag: &str, ids: &HashSet<u64>) -> Result<(), String> {
        let key = Self::tag_index_key(tag);

        if ids.is_empty() {
            self.db
                .remove(key.as_bytes())
                .map_err(|e| format!("Database error: {}", e))?;
            return Ok(());
        }

        let mut vec_ids: Vec<u64> = ids.iter().copied().collect();
        vec_ids.sort_unstable();
        let encoded = bincode::serialize(&vec_ids)
            .map_err(|e| format!("Tag index serialization error: {}", e))?;

        self.db
            .insert(key.as_bytes(), encoded)
            .map_err(|e| format!("Database error: {}", e))?;

        Ok(())
    }

    fn map_inode(inode: &crate::fuse_handler::Inode) -> InodeMetadata {
        InodeMetadata {
            id: inode.id,
            parent_id: inode.parent_id,
            name: inode.name.clone(),
            is_dir: inode.attr.kind == FileType::Directory,
            attr: InodeAttr { size: inode.attr.size },
        }
    }

    pub fn set_file_tags(
        &self,
        inode_id: u64,
        filename: &str,
        tags: Vec<String>,
    ) -> Result<(), String> {
        let normalized = Self::normalize_tags(tags);
        let previous = self.get_file_tags(inode_id)?;

        if normalized.is_empty() {
            self.delete_file_tags(inode_id)?;
            return Ok(());
        }

        let tag_set = FileTagSet {
            file_id: inode_id,
            filename: filename.to_string(),
            tags: normalized.clone(),
        };

        let key = Self::ino_tags_key(inode_id);
        let encoded = bincode::serialize(&tag_set)
            .map_err(|e| format!("Tag set serialization error: {}", e))?;

        self.db
            .insert(key.as_bytes(), encoded)
            .map_err(|e| format!("Database error: {}", e))?;

        let previous_set: HashSet<String> = previous.into_iter().collect();
        let current_set: HashSet<String> = normalized.into_iter().collect();

        for removed_tag in previous_set.difference(&current_set) {
            let mut ids = self.load_tag_index_set(removed_tag)?;
            ids.remove(&inode_id);
            self.save_tag_index_set(removed_tag, &ids)?;
        }

        for added_tag in current_set {
            let mut ids = self.load_tag_index_set(&added_tag)?;
            ids.insert(inode_id);
            self.save_tag_index_set(&added_tag, &ids)?;
        }

        self.db.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    pub fn get_file_tags(&self, inode_id: u64) -> Result<Vec<String>, String> {
        let key = Self::ino_tags_key(inode_id);
        let value = self
            .db
            .get(key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?;

        match value {
            Some(bytes) => {
                let tag_set = bincode::deserialize::<FileTagSet>(&bytes)
                    .map_err(|e| format!("Tag set deserialization error: {}", e))?;
                Ok(tag_set.tags)
            }
            None => Ok(Vec::new()),
        }
    }

    pub fn get_files_with_tag(&self, tag: &str) -> Result<Vec<u64>, String> {
        let normalized = tag.trim().to_lowercase();
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        let mut ids: Vec<u64> = self.load_tag_index_set(&normalized)?.into_iter().collect();
        ids.sort_unstable();
        Ok(ids)
    }

    pub fn get_files_by_tags(&self, tags: &[String]) -> Result<Vec<u64>, String> {
        let normalized = Self::normalize_tags(tags.to_vec());
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        let mut iter = normalized.iter();
        let first = iter
            .next()
            .ok_or_else(|| "Missing first tag in query".to_string())?;

        let mut candidates = self.load_tag_index_set(first)?;
        for tag in iter {
            let ids = self.load_tag_index_set(tag)?;
            candidates.retain(|inode_id| ids.contains(inode_id));
            if candidates.is_empty() {
                return Ok(Vec::new());
            }
        }

        let mut out: Vec<u64> = candidates.into_iter().collect();
        out.sort_unstable();
        Ok(out)
    }

    pub fn get_next_level_tags(&self, current_tags: &[String]) -> Result<Vec<String>, String> {
        let normalized = Self::normalize_tags(current_tags.to_vec());
        if normalized.is_empty() {
            let mut all_tags = Vec::new();
            for item in self.db.scan_prefix(b"tag_index:") {
                let (key, _value) = item.map_err(|e| format!("Database error: {}", e))?;
                let key_str = String::from_utf8(key.to_vec())
                    .map_err(|e| format!("Invalid UTF-8 tag index key: {}", e))?;
                if let Some(tag) = key_str.strip_prefix("tag_index:") {
                    all_tags.push(tag.to_string());
                }
            }
            all_tags.sort();
            all_tags.dedup();
            return Ok(all_tags);
        }

        let matching_inodes = self.get_files_by_tags(&normalized)?;
        let current_set: HashSet<String> = normalized.into_iter().collect();
        let mut next = HashSet::new();

        for inode_id in matching_inodes {
            for tag in self.get_file_tags(inode_id)? {
                if !current_set.contains(&tag) {
                    next.insert(tag);
                }
            }
        }

        let mut out: Vec<String> = next.into_iter().collect();
        out.sort();
        Ok(out)
    }

    pub fn delete_file_tags(&self, inode_id: u64) -> Result<(), String> {
        let existing_tags = self.get_file_tags(inode_id)?;
        for tag in existing_tags {
            let mut ids = self.load_tag_index_set(&tag)?;
            ids.remove(&inode_id);
            self.save_tag_index_set(&tag, &ids)?;
        }

        let key = Self::ino_tags_key(inode_id);
        self.db
            .remove(key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?;

        self.db.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    pub fn add_tag_to_file(&self, inode_id: u64, filename: &str, tag: &str) -> Result<(), String> {
        let normalized_tag = tag.trim().to_lowercase();
        if normalized_tag.is_empty() {
            return Ok(());
        }

        let mut tags = self.get_file_tags(inode_id)?;
        tags.push(normalized_tag);
        self.set_file_tags(inode_id, filename, tags)
    }

    pub fn remove_tag_from_file(&self, inode_id: u64, filename: &str, tag: &str) -> Result<(), String> {
        let normalized_tag = tag.trim().to_lowercase();
        if normalized_tag.is_empty() {
            return Ok(());
        }

        let mut tags = self.get_file_tags(inode_id)?;
        tags.retain(|t| t != &normalized_tag);
        self.set_file_tags(inode_id, filename, tags)
    }

    pub fn resolve_inode_by_path(&self, path: &str) -> Result<Option<u64>, String> {
        let parts: Vec<&str> = path
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();

        if parts.is_empty() {
            return Ok(Some(1));
        }

        let mut current_inode = 1u64;
        for part in parts {
            let key = Self::dirent_key(current_inode, part);
            let value = self
                .db
                .get(key.as_bytes())
                .map_err(|e| format!("Database error: {}", e))?;

            let Some(bytes) = value else {
                return Ok(None);
            };

            current_inode = bincode::deserialize::<u64>(&bytes)
                .map_err(|e| format!("Dirent deserialization error: {}", e))?;
        }

        Ok(Some(current_inode))
    }

    pub fn set_file_tags_by_path(&self, path: &str, tags: Vec<String>) -> Result<u64, String> {
        let inode_id = self
            .resolve_inode_by_path(path)?
            .ok_or_else(|| format!("Path not found: {}", path))?;

        let filename = path
            .rsplit('/')
            .find(|part| !part.is_empty())
            .unwrap_or(path)
            .to_string();

        self.set_file_tags(inode_id, &filename, tags)?;
        Ok(inode_id)
    }

    fn create_recipe_from_data(&self, data: &[u8]) -> Result<FileRecipe, String> {
        let mut chunker = Chunker::new();
        let mut recipe = Vec::new();
        let mut current_chunk_buffer = Vec::new();
        let mut total_size = 0_u64;

        for &byte in data {
            current_chunk_buffer.push(byte);
            chunker.feed_byte(byte);

            if chunker.should_cut(current_chunk_buffer.len()) {
                let hash = self
                    .storage
                    .write_chunk(&current_chunk_buffer)
                    .map_err(|e| format!("Failed to write chunk: {}", e))?;
                recipe.push(hash);
                total_size += current_chunk_buffer.len() as u64;
                current_chunk_buffer.clear();
                chunker.reset();
            }
        }

        if !current_chunk_buffer.is_empty() {
            let hash = self
                .storage
                .write_chunk(&current_chunk_buffer)
                .map_err(|e| format!("Failed to write tail chunk: {}", e))?;
            recipe.push(hash);
            total_size += current_chunk_buffer.len() as u64;
        }

        Ok(FileRecipe {
            file_size: total_size,
            chunks: recipe,
            kind: FileKind::File,
        })
    }

    pub fn save_snapshot(&self, name: &str, timestamp: u64, root_id: u64) -> Result<(), String> {
        let key = format!("snapshot:{}", name);
        let metadata = SnapshotMetadata {
            name: name.to_string(),
            timestamp,
            root_id,
        };

        let encoded = bincode::serialize(&metadata)
            .map_err(|e| format!("Snapshot serialization error: {}", e))?;

        self.db
            .insert(key.as_bytes(), encoded)
            .map_err(|e| format!("Database error: {}", e))?;

        self.db.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    pub fn load_snapshots(&self) -> Vec<SnapshotMetadata> {
        let mut snapshots = Vec::new();

        for item in self.db.scan_prefix(b"snapshot:") {
            if let Ok((_, bytes)) = item
                && let Ok(metadata) = bincode::deserialize::<SnapshotMetadata>(&bytes)
            {
                snapshots.push(metadata);
            }
        }

        snapshots
    }

    pub fn delete_snapshot(&self, name: &str) -> Result<(), String> {
        let key = format!("snapshot:{}", name);
        self.db
            .remove(key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?;
        self.db.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    pub fn save_inode(&self, inode: &crate::fuse_handler::Inode) -> Result<(), String> {
        let metadata = Self::map_inode(inode);
        let key = Self::inode_key(metadata.id);
        let encoded = bincode::serialize(&metadata)
            .map_err(|e| format!("Inode serialization error: {}", e))?;

        self.db
            .insert(key.as_bytes(), encoded)
            .map_err(|e| format!("Database error: {}", e))?;

        self.db.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    pub fn load_inode(&self, inode_id: u64) -> Result<Option<InodeMetadata>, String> {
        let key = Self::inode_key(inode_id);
        let value = self
            .db
            .get(key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?;

        match value {
            Some(bytes) => {
                let inode = bincode::deserialize::<InodeMetadata>(&bytes)
                    .map_err(|e| format!("Inode deserialization error: {}", e))?;
                Ok(Some(inode))
            }
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub fn load_all_inodes(&self) -> Vec<InodeMetadata> {
        let mut inodes = Vec::new();

        for item in self.db.scan_prefix(b"ino_meta:") {
            if let Ok((_, bytes)) = item
                && let Ok(inode) = bincode::deserialize::<InodeMetadata>(&bytes)
            {
                inodes.push(inode);
            }
        }

        inodes
    }

    pub fn save_dirent(&self, parent_id: u64, name: &str, child_inode_id: u64) -> Result<(), String> {
        let key = Self::dirent_key(parent_id, name);
        let encoded = bincode::serialize(&child_inode_id)
            .map_err(|e| format!("Dirent serialization error: {}", e))?;

        self.db
            .insert(key.as_bytes(), encoded)
            .map_err(|e| format!("Database error: {}", e))?;

        self.db.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    pub fn delete_dirent(&self, parent_id: u64, name: &str) -> Result<(), String> {
        let key = Self::dirent_key(parent_id, name);
        self.db
            .remove(key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?;
        self.db.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    pub fn list_dirents(&self, parent_id: u64) -> Result<Vec<Dirent>, String> {
        let prefix = Self::dirent_prefix(parent_id);
        let mut out = Vec::new();

        for item in self.db.scan_prefix(prefix.as_bytes()) {
            let (key, value) = item.map_err(|e| format!("Database error: {}", e))?;

            let key_str = String::from_utf8(key.to_vec())
                .map_err(|e| format!("Invalid UTF-8 dirent key: {}", e))?;

            let name = key_str
                .strip_prefix(&prefix)
                .ok_or_else(|| format!("Invalid dirent key prefix: {}", key_str))?
                .to_string();

            let child_inode_id = bincode::deserialize::<u64>(&value)
                .map_err(|e| format!("Dirent deserialization error: {}", e))?;

            out.push(Dirent {
                parent_id,
                name,
                child_inode_id,
            });
        }

        Ok(out)
    }

    pub fn save_recipe(&self, inode_id: u64, recipe: &FileRecipe) -> Result<(), String> {
        let key = Self::recipe_key(inode_id);
        let encoded = bincode::serialize(recipe)
            .map_err(|e| format!("Recipe serialization error: {}", e))?;

        self.db
            .insert(key.as_bytes(), encoded)
            .map_err(|e| format!("Database error: {}", e))?;

        self.db.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    pub fn load_recipe(&self, inode_id: u64) -> Result<Option<FileRecipe>, String> {
        let key = Self::recipe_key(inode_id);
        let value = self
            .db
            .get(key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?;

        match value {
            Some(bytes) => {
                let recipe = bincode::deserialize::<FileRecipe>(&bytes)
                    .map_err(|e| format!("Recipe deserialization error: {}", e))?;
                Ok(Some(recipe))
            }
            None => Ok(None),
        }
    }

    pub fn write_file_by_id(&self, inode_id: u64, data: &[u8]) -> Result<(), String> {
        let recipe = self.create_recipe_from_data(data)?;
        self.save_recipe(inode_id, &recipe)
    }

    pub fn read_file_by_id(&self, inode_id: u64) -> Result<Vec<u8>, String> {
        let recipe = self
            .load_recipe(inode_id)?
            .ok_or_else(|| format!("Recipe not found for inode {}", inode_id))?;

        if recipe.kind == FileKind::Directory {
            return Ok(Vec::new());
        }

        let mut data = Vec::new();
        for hash in recipe.chunks {
            let chunk = self
                .storage
                .read_chunk(&hash)
                .map_err(|e| format!("Failed reading chunk {}: {}", hash, e))?;
            data.extend_from_slice(&chunk);
        }

        Ok(data)
    }

    fn get_legacy_key(name: &str) -> String {
        format!("legacy_name:{}", name)
    }

    fn get_next_legacy_ino(&self) -> Result<u64, String> {
        let key = b"legacy:next_ino";
        let current = match self.db.get(key).map_err(|e| format!("Database error: {}", e))? {
            Some(bytes) => bincode::deserialize::<u64>(&bytes)
                .map_err(|e| format!("Counter deserialization error: {}", e))?,
            None => 1_000_000,
        };

        let next = current + 1;
        let encoded = bincode::serialize(&next)
            .map_err(|e| format!("Counter serialization error: {}", e))?;
        self.db
            .insert(key, encoded)
            .map_err(|e| format!("Database error: {}", e))?;

        Ok(current)
    }

    pub fn write_file(&self, filename: &str, data: &[u8]) -> Result<(), String> {
        let legacy_key = Self::get_legacy_key(filename);

        let inode_id = match self
            .db
            .get(legacy_key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?
        {
            Some(bytes) => bincode::deserialize::<u64>(&bytes)
                .map_err(|e| format!("Legacy inode deserialization error: {}", e))?,
            None => {
                let id = self.get_next_legacy_ino()?;
                let encoded = bincode::serialize(&id)
                    .map_err(|e| format!("Legacy inode serialization error: {}", e))?;
                self.db
                    .insert(legacy_key.as_bytes(), encoded)
                    .map_err(|e| format!("Database error: {}", e))?;
                id
            }
        };

        self.write_file_by_id(inode_id, data)?;
        self.db.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    pub fn read_file(&self, filename: &str) -> Result<Vec<u8>, String> {
        let legacy_key = Self::get_legacy_key(filename);
        let inode_id = self
            .db
            .get(legacy_key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("File not found: {}", filename))
            .and_then(|bytes| {
                bincode::deserialize::<u64>(&bytes)
                    .map_err(|e| format!("Legacy inode deserialization error: {}", e))
            })?;

        self.read_file_by_id(inode_id)
    }

    pub fn list_files(&self) -> Vec<String> {
        let mut files = Vec::new();
        for item in self.db.scan_prefix(b"legacy_name:") {
            if let Ok((key, _)) = item
                && let Ok(key_str) = String::from_utf8(key.to_vec())
                && let Some(name) = key_str.strip_prefix("legacy_name:")
            {
                files.push(name.to_string());
            }
        }
        files.sort();
        files
    }

    pub fn run_gc(&self) -> Result<usize, String> {
        let mut active_hashes = HashSet::new();

        for item in self.db.scan_prefix(b"ino_recipe:") {
            let (_, value) = item.map_err(|e| format!("Database error: {}", e))?;
            if let Ok(recipe) = bincode::deserialize::<FileRecipe>(&value) {
                for hash in recipe.chunks {
                    active_hashes.insert(hash);
                }
            }
        }

        let all_chunks = self
            .storage
            .list_all_chunks()
            .map_err(|e| format!("Storage error: {}", e))?;

        let mut deleted_count = 0;
        for hash in all_chunks {
            if !active_hashes.contains(&hash) {
                self.storage
                    .delete_chunk(&hash)
                    .map_err(|e| format!("Failed to delete {}: {}", hash, e))?;
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }

    #[allow(dead_code)]
    pub fn get_file_metadata(&self, filename: &str) -> Option<(u64, FileKind)> {
        let legacy_key = Self::get_legacy_key(filename);
        let inode_id = self
            .db
            .get(legacy_key.as_bytes())
            .ok()
            .flatten()
            .and_then(|bytes| bincode::deserialize::<u64>(&bytes).ok())?;

        let recipe = self.load_recipe(inode_id).ok().flatten()?;
        Some((recipe.file_size, recipe.kind))
    }

    #[allow(dead_code)]
    pub fn delete_file(&self, filename: &str) -> Result<(), String> {
        let legacy_key = Self::get_legacy_key(filename);
        let inode_id = self
            .db
            .get(legacy_key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?
            .map(|bytes| {
                bincode::deserialize::<u64>(&bytes)
                    .map_err(|e| format!("Legacy inode deserialization error: {}", e))
            })
            .transpose()?;

        self.db
            .remove(legacy_key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?;

        if let Some(id) = inode_id {
            let recipe_key = Self::recipe_key(id);
            let inode_key = Self::inode_key(id);
            self.db
                .remove(recipe_key.as_bytes())
                .map_err(|e| format!("Database error: {}", e))?;
            self.db
                .remove(inode_key.as_bytes())
                .map_err(|e| format!("Database error: {}", e))?;
        }

        self.db.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn rename_file(&self, old_name: &str, new_name: &str) -> Result<(), String> {
        let old_key = Self::get_legacy_key(old_name);
        let new_key = Self::get_legacy_key(new_name);

        let value = self
            .db
            .get(old_key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("File not found: {}", old_name))?;

        self.db
            .insert(new_key.as_bytes(), value)
            .map_err(|e| format!("Database error: {}", e))?;
        self.db
            .remove(old_key.as_bytes())
            .map_err(|e| format!("Database error: {}", e))?;

        self.db.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn create_directory(&self, filename: &str) -> Result<(), String> {
        let recipe = FileRecipe {
            file_size: 0,
            chunks: Vec::new(),
            kind: FileKind::Directory,
        };

        let legacy_key = Self::get_legacy_key(filename);
        let inode_id = self.get_next_legacy_ino()?;
        let encoded = bincode::serialize(&inode_id)
            .map_err(|e| format!("Legacy inode serialization error: {}", e))?;

        self.db
            .insert(legacy_key.as_bytes(), encoded)
            .map_err(|e| format!("Database error: {}", e))?;

        self.save_recipe(inode_id, &recipe)?;
        Ok(())
    }

    pub fn inspect_records(&self) -> Vec<(String, Option<FileRecipe>)> {
        let mut records = Vec::new();

        for item in self.db.iter() {
            if let Ok((key, value)) = item {
                let key_str = String::from_utf8_lossy(&key).to_string();
                let parsed_recipe = bincode::deserialize::<FileRecipe>(&value).ok();
                records.push((key_str, parsed_recipe));
            }
        }

        records
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn metadata_schema_roundtrip() {
        let path = "./test_file_manager_schema";
        if Path::new(path).exists() {
            fs::remove_dir_all(path).expect("cleanup failed");
        }

        let fm = FileManager::new(path);

        let recipe = FileRecipe {
            file_size: 5,
            chunks: vec!["abc".to_string()],
            kind: FileKind::File,
        };

        fm.save_recipe(42, &recipe).expect("save recipe failed");
        let loaded = fm
            .load_recipe(42)
            .expect("load recipe failed")
            .expect("missing recipe");
        assert_eq!(loaded.file_size, 5);

        fm.save_dirent(1, "hello.txt", 42)
            .expect("save dirent failed");
        let dirents = fm.list_dirents(1).expect("list dirents failed");
        assert_eq!(dirents.len(), 1);
        assert_eq!(dirents[0].name, "hello.txt");
        assert_eq!(dirents[0].child_inode_id, 42);

        fm.delete_dirent(1, "hello.txt")
            .expect("delete dirent failed");
        let dirents_after = fm.list_dirents(1).expect("list dirents failed");
        assert!(dirents_after.is_empty());

        fs::remove_dir_all(path).expect("cleanup failed");
    }
}
