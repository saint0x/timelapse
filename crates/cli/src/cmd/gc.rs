//! Run garbage collection

use anyhow::Result;

pub async fn run() -> Result<()> {
    // TODO: Implement gc command
    // - Determine live checkpoints
    // - Walk reachable trees/blobs
    // - Delete unreferenced objects
    todo!("Implement snap gc")
}
