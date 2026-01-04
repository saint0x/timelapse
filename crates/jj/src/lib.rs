//! JJ (Jujutsu) integration for Git interoperability
//!
//! This crate provides:
//! - Checkpoint → JJ commit materialization
//! - `tl publish` (create JJ commit from checkpoint)
//! - `tl push` / `tl pull` (Git interop via JJ)
//! - Checkpoint ↔ JJ commit mapping

pub mod materialize;
pub mod mapping;

use anyhow::Result;
use journal::Checkpoint;

/// Materialize a checkpoint as a JJ commit
pub fn materialize_checkpoint(checkpoint: &Checkpoint) -> Result<()> {
    // TODO: Implement checkpoint → JJ commit conversion
    // - Restore working tree to checkpoint state
    // - Create JJ commit
    // - Set bookmark
    todo!("Implement materialize_checkpoint")
}

/// Publish a checkpoint to JJ and Git
pub fn publish(checkpoint: &Checkpoint, bookmark: &str) -> Result<()> {
    // TODO: Implement publish
    // - Materialize checkpoint
    // - Create JJ bookmark
    // - Optionally push to Git remote
    todo!("Implement publish")
}

/// Pull from Git via JJ
pub fn pull() -> Result<()> {
    // TODO: Implement pull
    // - jj git fetch
    // - Update local checkpoints
    todo!("Implement pull")
}

/// Push to Git via JJ
pub fn push(bookmark: &str) -> Result<()> {
    // TODO: Implement push
    // - jj git push -b <bookmark>
    todo!("Implement push")
}
