//! Publish checkpoint to JJ

use anyhow::Result;

pub async fn run(checkpoint: &str) -> Result<()> {
    // TODO: Implement publish command
    // - Lookup checkpoint
    // - Materialize to JJ commit
    // - Set bookmark
    todo!("Implement snap publish {}", checkpoint)
}
