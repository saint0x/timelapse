//! Pin a checkpoint with a name

use anyhow::Result;

pub async fn run(checkpoint: &str, name: &str) -> Result<()> {
    // TODO: Implement pin command
    // - Lookup checkpoint
    // - Create pin in .snap/refs/pins/<name>
    todo!("Implement snap pin {} {}", checkpoint, name)
}
