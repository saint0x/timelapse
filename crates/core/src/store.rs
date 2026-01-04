//! On-disk store management for blobs and trees

use crate::blob::BlobStore;
use crate::hash::Blake3Hash;
use crate::tree::Tree;
use anyhow::Result;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Main store for Timelapse checkpoint data
///
/// Manages the `.tl/` directory structure:
/// ```
/// .tl/
///   config.toml
///   HEAD
///   locks/
///     daemon.lock
///     gc.lock
///   journal/
///     ops.log
///     ops.log.idx
///   objects/
///     blobs/
///     trees/
///   refs/
///     pins/
///     heads/
///   state/
///     pathmap.bin
///     watcher.state
///     metrics.json
///   tmp/
///     ingest/
///     gc/
/// ```
pub struct Store {
    /// Root of repository
    root: PathBuf,
    /// Path to .tl directory
    tl_dir: PathBuf,
    /// Blob storage
    blob_store: BlobStore,
    /// Tree cache (hash -> tree)
    tree_cache: DashMap<Blake3Hash, Arc<Tree>>,
}

impl Store {
    /// Initialize a new store at the given repository root
    pub fn init(repo_root: &Path) -> Result<Self> {
        // TODO: Implement store initialization
        // - Create .tl/ directory
        // - Create all subdirectories
        // - Create config.toml with defaults
        // - Initialize empty ops.log
        // - Return Store instance
        todo!("Implement Store::init")
    }

    /// Open an existing store
    pub fn open(repo_root: &Path) -> Result<Self> {
        // TODO: Implement store opening
        // - Validate .tl/ directory exists
        // - Load configuration
        // - Initialize blob store
        // - Return Store instance
        todo!("Implement Store::open")
    }

    /// Write a tree to storage
    pub fn write_tree(&self, tree: &Tree) -> Result<Blake3Hash> {
        // TODO: Implement tree writing
        // - Serialize tree
        // - Compute hash
        // - Check if already exists
        // - Write to objects/trees/<hh>/<rest>
        // - Cache tree
        // - Return hash
        todo!("Implement Store::write_tree")
    }

    /// Read a tree from storage
    pub fn read_tree(&self, hash: Blake3Hash) -> Result<Tree> {
        // TODO: Implement tree reading
        // - Check cache first
        // - If not cached, read from disk
        // - Deserialize tree
        // - Add to cache
        // - Return tree
        todo!("Implement Store::read_tree")
    }

    /// Get the tree path for a given hash
    fn tree_path(&self, hash: Blake3Hash) -> PathBuf {
        // TODO: Implement tree path construction
        // Similar to blob_path: objects/trees/<hh>/<rest>
        todo!("Implement tree_path")
    }

    /// Get the blob store
    pub fn blob_store(&self) -> &BlobStore {
        &self.blob_store
    }

    /// Get the .tl directory path
    pub fn tl_dir(&self) -> &Path {
        &self.tl_dir
    }

    /// Get the repository root path
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Atomic write helper
///
/// Writes data to a temporary file, fsyncs it, then renames it to the target path.
/// This ensures crash safety.
pub fn atomic_write(tmp_dir: &Path, target: &Path, data: &[u8]) -> Result<()> {
    // TODO: Implement atomic write
    // - Generate unique temp file path in tmp_dir
    // - Write data to temp file
    // - Fsync temp file
    // - Rename to target
    // - Fsync parent directory
    todo!("Implement atomic_write")
}

/// Normalize a path for storage
///
/// - Converts to relative path with `/` separator
/// - Rejects `..` and absolute paths
/// - Removes `./` prefix
pub fn normalize_path(path: &Path) -> Result<PathBuf> {
    // TODO: Implement path normalization
    // - Check for absolute paths (reject)
    // - Check for .. components (reject)
    // - Remove ./ prefix
    // - Convert to forward slashes
    todo!("Implement normalize_path")
}

/// Check if a path should be ignored
///
/// Always ignores:
/// - `.tl/`
/// - `.git/`
pub fn should_ignore(path: &Path) -> bool {
    // TODO: Implement ignore check
    // - Check if path starts with .tl/ or .git/
    // - Future: support .gitignore-like rules
    path.starts_with(".tl") || path.starts_with(".git")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_init() {
        // TODO: Test store initialization
        // - Create temp directory
        // - Initialize store
        // - Verify .tl/ structure exists
        // - Verify all subdirectories exist
    }

    #[test]
    fn test_atomic_write() {
        // TODO: Test atomic write
        // - Write data using atomic_write
        // - Verify file exists at target path
        // - Verify content is correct
        // - Verify temp file is cleaned up
    }

    #[test]
    fn test_normalize_path() {
        // TODO: Test path normalization
        // - Test relative paths work
        // - Test ./ prefix is removed
        // - Test .. is rejected
        // - Test absolute paths are rejected
    }

    #[test]
    fn test_should_ignore() {
        // TODO: Test ignore rules
        // assert!(should_ignore(Path::new(".tl/config.toml")));
        // assert!(should_ignore(Path::new(".git/HEAD")));
        // assert!(!should_ignore(Path::new("src/main.rs")));
    }
}
