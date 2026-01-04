//! Checkpoint data structures

use core::Blake3Hash;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// A checkpoint represents a snapshot of the repository at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique ID (ULID for timestamp + uniqueness)
    pub id: Ulid,
    /// Parent checkpoint ID
    pub parent: Option<Ulid>,
    /// Root tree hash for this checkpoint
    pub root_tree: Blake3Hash,
    /// Timestamp (Unix milliseconds)
    pub ts_unix_ms: u64,
    /// Reason for checkpoint
    pub reason: CheckpointReason,
    /// Paths touched in this checkpoint
    pub touched_paths: Vec<std::path::PathBuf>,
    /// Checkpoint metadata
    pub meta: CheckpointMeta,
}

/// Checkpoint metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMeta {
    /// Number of files changed
    pub files_changed: u32,
    /// Bytes added
    pub bytes_added: u64,
    /// Bytes removed
    pub bytes_removed: u64,
}

/// Reason for creating a checkpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
        parent: Option<Ulid>,
        root_tree: Blake3Hash,
        reason: CheckpointReason,
        touched_paths: Vec<std::path::PathBuf>,
        meta: CheckpointMeta,
    ) -> Self {
        Self {
            id: Ulid::new(),
            parent,
            root_tree,
            ts_unix_ms: current_timestamp_ms(),
            reason,
            touched_paths,
            meta,
        }
    }

    /// Serialize checkpoint to bytes
    pub fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }

    /// Deserialize checkpoint from bytes
    pub fn deserialize(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(bincode::deserialize(bytes)?)
    }
}

fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
