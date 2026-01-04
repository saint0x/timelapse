//! Append-only checkpoint journal using sled

use crate::Checkpoint;
use anyhow::Result;

/// Append-only journal for checkpoints
pub struct Journal {
    // TODO: Add sled database
}

impl Journal {
    /// Open or create a journal at the given path
    pub fn open(path: &std::path::Path) -> Result<Self> {
        // TODO: Open sled database
        todo!("Implement Journal::open")
    }

    /// Append a checkpoint to the journal
    pub fn append(&mut self, checkpoint: Checkpoint) -> Result<()> {
        // TODO: Serialize and append checkpoint
        todo!("Implement Journal::append")
    }

    /// Get a checkpoint by ID
    pub fn get(&self, id: &[u8]) -> Result<Option<Checkpoint>> {
        // TODO: Lookup checkpoint by ID
        todo!("Implement Journal::get")
    }

    /// Get the latest checkpoint
    pub fn latest(&self) -> Result<Option<Checkpoint>> {
        // TODO: Get most recent checkpoint
        todo!("Implement Journal::latest")
    }
}
