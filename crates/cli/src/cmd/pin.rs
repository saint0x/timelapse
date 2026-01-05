//! Pin a checkpoint with a name

use crate::util;
use anyhow::{anyhow, Context, Result};
use journal::PinManager;
use owo_colors::OwoColorize;

pub async fn run(checkpoint: &str, name: &str) -> Result<()> {
    // 1. Find repository root
    let repo_root = util::find_repo_root()
        .context("Failed to find repository")?;

    let tl_dir = repo_root.join(".tl");

    // 2. Ensure daemon running (auto-starts if needed)
    crate::daemon::ensure_daemon_running().await?;

    // 3. Resolve checkpoint reference via unified data access layer
    let ids = crate::data_access::resolve_checkpoint_refs(&[checkpoint.to_string()], &tl_dir).await?;
    let checkpoint_id = ids[0].ok_or_else(||
        anyhow!("Checkpoint '{}' not found or ambiguous", checkpoint))?;

    // 4. Verify checkpoint exists
    let checkpoints = crate::data_access::get_checkpoints(&[checkpoint_id], &tl_dir).await?;
    checkpoints[0].as_ref().ok_or_else(|| anyhow!("Checkpoint not found"))?;

    // 5. Pin the checkpoint (PinManager is safe to use directly - writes to separate files)
    let pin_manager = PinManager::new(&tl_dir);
    pin_manager.pin(name, checkpoint_id)?;

    let id_short = checkpoint_id.to_string()[..8].to_string();
    println!("{} Created pin '{}' → {}", "✓".green(), name.yellow(), id_short.cyan());

    Ok(())
}
