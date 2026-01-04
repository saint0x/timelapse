//! Run garbage collection

use crate::util;
use anyhow::{Context, Result};
use std::collections::HashSet;
use tl_core::Store;
use journal::{GarbageCollector, Journal, PinManager, RetentionPolicy};
use owo_colors::OwoColorize;
use ulid::Ulid;

pub async fn run() -> Result<()> {
    // 1. Find repository root
    let repo_root = util::find_repo_root()
        .context("Failed to find repository")?;

    let tl_dir = repo_root.join(".tl");

    // 2. Open journal and store
    let journal_path = tl_dir.join("journal");
    let mut journal = Journal::open(&journal_path)
        .context("Failed to open checkpoint journal")?;

    let mut store = Store::open(&repo_root)?;

    // 3. Create pin manager
    let pin_manager = PinManager::new(&tl_dir);

    // 4. Collect workspace checkpoints (if JJ workspace exists)
    let workspace_checkpoints = if jj::detect_jj_workspace(&repo_root)?.is_some() {
        let ws_manager = jj::WorkspaceManager::open(&tl_dir, &repo_root)?;
        let mut checkpoints = HashSet::new();

        for state in ws_manager.list_states()? {
            if let Some(cp_id) = state.current_checkpoint {
                checkpoints.insert(cp_id);
            }
        }

        Some(checkpoints)
    } else {
        None
    };

    // 5. Create GC with default retention policy
    let policy = RetentionPolicy::default();
    let gc = GarbageCollector::new(policy);

    println!("{}", "Running Garbage Collection...".bold());
    println!();

    // 6. Run GC with workspace checkpoint protection
    let metrics = gc.collect(&mut journal, &mut store, &pin_manager, workspace_checkpoints.as_ref())?;

    // 7. Display results
    println!("{}", "GC Complete".green().bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    if metrics.checkpoints_deleted == 0 && metrics.trees_deleted == 0 && metrics.blobs_deleted == 0 {
        println!("{}", "No garbage found - repository is already clean".dimmed());
    } else {
        println!("Checkpoints deleted: {}", metrics.checkpoints_deleted.to_string().yellow());
        println!("Trees deleted:       {}", metrics.trees_deleted.to_string().yellow());
        println!("Blobs deleted:       {}", metrics.blobs_deleted.to_string().yellow());
        println!();
        println!(
            "Space freed:         {}",
            util::format_size(metrics.bytes_freed).green()
        );
    }

    Ok(())
}
