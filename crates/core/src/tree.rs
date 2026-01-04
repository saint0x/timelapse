//! Tree representation for repository snapshots

use crate::hash::Blake3Hash;
use anyhow::Result;
use ahash::AHashMap;
use smallvec::SmallVec;
use std::path::Path;

/// Type of tree entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// Regular file
    File,
    /// Symbolic link
    Symlink,
    /// Submodule (optional for MVP)
    Submodule,
}

/// Entry in a tree (file, symlink, etc.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Kind of entry
    pub kind: EntryKind,
    /// Unix permission bits (mode)
    pub mode: u32,
    /// Hash of the blob containing this entry's content
    pub blob_hash: Blake3Hash,
}

impl Entry {
    /// Create a new file entry
    pub fn file(mode: u32, blob_hash: Blake3Hash) -> Self {
        Self {
            kind: EntryKind::File,
            mode,
            blob_hash,
        }
    }

    /// Create a new symlink entry
    pub fn symlink(blob_hash: Blake3Hash) -> Self {
        Self {
            kind: EntryKind::Symlink,
            mode: 0o120000, // Standard symlink mode
            blob_hash,
        }
    }
}

/// A tree represents the complete repository state at a point in time
///
/// Uses SmallVec for paths to optimize stack allocation for short paths (< 64 bytes)
#[derive(Debug, Clone)]
pub struct Tree {
    /// Mapping from path to entry
    /// Uses AHashMap (faster for small keys) and SmallVec (stack allocation for short paths)
    entries: AHashMap<SmallVec<[u8; 64]>, Entry>,
}

impl Tree {
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            entries: AHashMap::new(),
        }
    }

    /// Insert an entry into the tree
    pub fn insert(&mut self, path: &Path, entry: Entry) {
        // TODO: Convert path to SmallVec<[u8; 64]>
        todo!("Implement Tree::insert")
    }

    /// Get an entry from the tree
    pub fn get(&self, path: &Path) -> Option<&Entry> {
        // TODO: Convert path to SmallVec and lookup
        todo!("Implement Tree::get")
    }

    /// Remove an entry from the tree
    pub fn remove(&mut self, path: &Path) -> Option<Entry> {
        // TODO: Convert path to SmallVec and remove
        todo!("Implement Tree::remove")
    }

    /// Get the number of entries in the tree
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serialize the tree to bytes (TreeV1 format)
    ///
    /// Format:
    /// - magic: "SNT1" (4 bytes)
    /// - entry_count: u32
    /// - entries (sorted lexicographically by path):
    ///   - path_len: u16
    ///   - path_bytes: [u8; path_len]
    ///   - kind: u8 (0=file, 1=symlink, 2=submodule)
    ///   - mode: u32
    ///   - blob_hash: [u8; 32]
    pub fn serialize(&self) -> Vec<u8> {
        // TODO: Implement TreeV1 serialization
        // - Write magic bytes
        // - Write entry count
        // - Sort entries by path (deterministic)
        // - Write each entry
        todo!("Implement Tree::serialize")
    }

    /// Deserialize a tree from bytes (TreeV1 format)
    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        // TODO: Implement TreeV1 deserialization
        // - Check magic bytes
        // - Read entry count
        // - Parse each entry
        // - Build tree
        todo!("Implement Tree::deserialize")
    }

    /// Compute the hash of this tree
    ///
    /// Hash is deterministic - same tree content always produces same hash
    pub fn hash(&self) -> Blake3Hash {
        // TODO: Serialize tree and hash the bytes
        todo!("Implement Tree::hash")
    }

    /// Update entries in the tree
    ///
    /// Changes: Vec<(Path, Option<Entry>)>
    /// - None = remove entry
    /// - Some(entry) = insert/update entry
    pub fn update_entries(
        base: &Tree,
        changes: Vec<(&Path, Option<Entry>)>,
    ) -> Self {
        // TODO: Implement incremental tree update
        // - Clone base tree
        // - Apply all changes
        // - Return new tree
        todo!("Implement Tree::update_entries")
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

/// Differences between two trees
#[derive(Debug, Clone)]
pub struct TreeDiff {
    /// Entries added in new tree
    pub added: Vec<(SmallVec<[u8; 64]>, Entry)>,
    /// Entries removed in new tree
    pub removed: Vec<(SmallVec<[u8; 64]>, Entry)>,
    /// Entries modified in new tree (old, new)
    pub modified: Vec<(SmallVec<[u8; 64]>, Entry, Entry)>,
}

impl TreeDiff {
    /// Compute the diff between two trees
    pub fn diff(old: &Tree, new: &Tree) -> Self {
        // TODO: Implement tree diffing
        // - Compare entries
        // - Detect additions, removals, modifications
        // - Return TreeDiff
        todo!("Implement TreeDiff::diff")
    }

    /// Check if there are any changes
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_serialization_deterministic() {
        // TODO: Test that serialization is deterministic
        // - Create tree with entries
        // - Serialize twice
        // - Assert bytes are identical
    }

    #[test]
    fn test_tree_hash_deterministic() {
        // TODO: Test that hash is deterministic
        // - Create two identical trees
        // - Assert hashes are equal
    }

    #[test]
    fn test_tree_diff() {
        // TODO: Test tree diffing
        // - Create old tree with some entries
        // - Create new tree with added/removed/modified entries
        // - Compute diff
        // - Assert diff is correct
    }

    #[test]
    fn test_tree_update_entries() {
        // TODO: Test incremental updates
        // - Create base tree
        // - Apply changes
        // - Assert resulting tree is correct
    }
}
