//! Show diff between checkpoints

use crate::util;
use anyhow::{anyhow, Context, Result};
use tl_core::{Store, TreeDiff};
use owo_colors::OwoColorize;
use std::path::Path;
use ulid::Ulid;

pub async fn run(checkpoint_a: &str, checkpoint_b: &str) -> Result<()> {
    // 1. Find repository root
    let repo_root = util::find_repo_root()
        .context("Failed to find repository")?;

    let tl_dir = repo_root.join(".tl");

    // 2. Ensure daemon is running (auto-start with supervisor)
    crate::daemon::ensure_daemon_running().await?;

    // 3. Connect to daemon with retry
    let socket_path = tl_dir.join("state/daemon.sock");
    let resilient_client = crate::ipc::ResilientIpcClient::new(socket_path);
    let mut client = resilient_client.connect_with_retry().await
        .context("Failed to connect to daemon")?;

    // 4. Parse checkpoint IDs
    let id_a = Ulid::from_string(checkpoint_a)
        .context("Invalid checkpoint ID format for first argument")?;
    let id_b = Ulid::from_string(checkpoint_b)
        .context("Invalid checkpoint ID format for second argument")?;

    // 5. Batch fetch both checkpoints in ONE IPC call
    let ids = vec![id_a.to_string(), id_b.to_string()];
    let checkpoints = client.get_checkpoint_batch(ids).await?;

    let cp_a = checkpoints[0].as_ref()
        .ok_or_else(|| anyhow!("Checkpoint {} not found", id_a))?;
    let cp_b = checkpoints[1].as_ref()
        .ok_or_else(|| anyhow!("Checkpoint {} not found", id_b))?;

    // 6. Open store for tree diffs (read-only, safe)
    let store = Store::open(&repo_root)?;

    // 7. Load trees
    let tree_a = store.read_tree(cp_a.root_tree)?;
    let tree_b = store.read_tree(cp_b.root_tree)?;

    // 8. Compute diff
    let diff = TreeDiff::diff(&tree_a, &tree_b);

    // 9. Display diff
    println!("{}", "Diff Summary".bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    let id_a_short = id_a.to_string()[..8].to_string();
    let id_b_short = id_b.to_string()[..8].to_string();
    println!("From: {} {}", id_a_short.yellow(), util::format_relative_time(cp_a.ts_unix_ms).dimmed());
    println!("To:   {} {}", id_b_short.yellow(), util::format_relative_time(cp_b.ts_unix_ms).dimmed());
    println!();

    let total_changes = diff.added.len() + diff.removed.len() + diff.modified.len();
    if total_changes == 0 {
        println!("{}", "No changes between checkpoints".dimmed());
        return Ok(());
    }

    // Display added files
    if !diff.added.is_empty() {
        println!("{} Added ({} files)", "A".green().bold(), diff.added.len());
        for (path, _entry) in &diff.added {
            let path_str = Path::new(std::str::from_utf8(path).unwrap_or("<invalid utf8>"));
            println!("  {} {}", "+".green(), path_str.display());
        }
        println!();
    }

    // Display removed files
    if !diff.removed.is_empty() {
        println!("{} Removed ({} files)", "D".red().bold(), diff.removed.len());
        for (path, _entry) in &diff.removed {
            let path_str = Path::new(std::str::from_utf8(path).unwrap_or("<invalid utf8>"));
            println!("  {} {}", "-".red(), path_str.display());
        }
        println!();
    }

    // Display modified files
    if !diff.modified.is_empty() {
        println!("{} Modified ({} files)", "M".yellow().bold(), diff.modified.len());
        for (path, _old_entry, _new_entry) in &diff.modified {
            let path_str = Path::new(std::str::from_utf8(path).unwrap_or("<invalid utf8>"));
            println!("  {} {}", "~".yellow(), path_str.display());
        }
        println!();
    }

    // Summary
    println!(
        "{}",
        format!(
            "Total: {} added, {} removed, {} modified",
            diff.added.len().to_string().green(),
            diff.removed.len().to_string().red(),
            diff.modified.len().to_string().yellow()
        ).dimmed()
    );

    Ok(())
}
