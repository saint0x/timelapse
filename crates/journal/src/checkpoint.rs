//! Checkpoint data structures

use core::Blake3Hash;

/// A checkpoint represents a snapshot of the repository at a point in time
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Unique ID (ULID for timestamp + uniqueness)
    pub id: Vec<u8>, // TODO: Use ULID type
    /// Parent checkpoint ID
    pub parent: Option<Vec<u8>>,
    /// Root tree hash for this checkpoint
    pub root_tree: Blake3Hash,
    /// Timestamp (Unix milliseconds)
    pub ts_unix_ms: u64,
    /// Reason for checkpoint
    pub reason: CheckpointReason,
    /// Paths touched in this checkpoint
    pub touched_paths: Vec<std::path::PathBuf>,
}

/// Reason for creating a checkpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointReason {
    /// File system batch
    FsBatch,
    /// Manual checkpoint
    Manual,
    /// Restore operation
    Restore,
    /// Publish to JJ
    Publish,
    /// GC compact
    GcCompact,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(
        parent: Option<Vec<u8>>,
        root_tree: Blake3Hash,
        reason: CheckpointReason,
        touched_paths: Vec<std::path::PathBuf>,
    ) -> Self {
        // TODO: Generate ULID for ID
        // TODO: Get current timestamp
        todo!("Implement Checkpoint::new")
    }
}
