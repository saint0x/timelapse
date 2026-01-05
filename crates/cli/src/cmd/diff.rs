//! Show diff between checkpoints

use crate::util;
use anyhow::{anyhow, Context, Result};
use tl_core::{Store, TreeDiff};
use owo_colors::OwoColorize;
use std::path::Path;

pub async fn run(checkpoint_a: &str, checkpoint_b: &str, patch: bool, context: usize, max_files: usize) -> Result<()> {
    // 1. Find repository root
    let repo_root = util::find_repo_root()
        .context("Failed to find repository")?;

    let tl_dir = repo_root.join(".tl");

    // 2. Ensure daemon running (auto-starts if needed)
    crate::daemon::ensure_daemon_running().await?;

    // 3. Resolve checkpoint references via unified data access layer
    let refs = vec![checkpoint_a.to_string(), checkpoint_b.to_string()];
    let ids = crate::data_access::resolve_checkpoint_refs(&refs, &tl_dir).await?;

    let id_a = ids[0].ok_or_else(||
        anyhow!("Checkpoint '{}' not found or ambiguous", checkpoint_a))?;
    let id_b = ids[1].ok_or_else(||
        anyhow!("Checkpoint '{}' not found or ambiguous", checkpoint_b))?;

    // 4. Get checkpoints via unified data access layer
    let checkpoints = crate::data_access::get_checkpoints(&[id_a, id_b], &tl_dir).await?;

    let cp_a = checkpoints[0].as_ref().ok_or_else(|| anyhow!("Checkpoint not found"))?;
    let cp_b = checkpoints[1].as_ref().ok_or_else(|| anyhow!("Checkpoint not found"))?;

    // 5. Open store for tree diffs (read-only, safe)
    let store = Store::open(&repo_root)?;

    // 6. Load trees
    let tree_a = store.read_tree(cp_a.root_tree)?;
    let tree_b = store.read_tree(cp_b.root_tree)?;

    // 7. Compute diff
    let diff = TreeDiff::diff(&tree_a, &tree_b);

    // 8. Display diff
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

    // Line-by-line diff if --patch flag is set
    if patch && !diff.modified.is_empty() {
        println!();
        println!("{}", "Detailed Diffs".bold());
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();

        let modified_to_show = diff.modified.iter().take(max_files);
        let mut shown = 0;

        for (path, old_entry, new_entry) in modified_to_show {
            let path_str = std::str::from_utf8(path).unwrap_or("<invalid utf8>");

            // Read blob contents
            let old_content = store.blob_store().read_blob(old_entry.blob_hash)?;
            let new_content = store.blob_store().read_blob(new_entry.blob_hash)?;

            // Check for binary files
            if crate::diff_utils::is_binary(&old_content) || crate::diff_utils::is_binary(&new_content) {
                println!("  {} {} (binary file)", "~".yellow(), path_str);
                println!();
                shown += 1;
                continue;
            }

            // Generate and display diff
            println!("  {} {}", "~".yellow(), path_str);
            println!();
            let diff_output = crate::diff_utils::generate_unified_diff(
                &old_content,
                &new_content,
                path_str,
                context,
            );
            println!("{}", diff_output);
            println!();
            shown += 1;
        }

        if diff.modified.len() > max_files {
            println!(
                "{}",
                format!(
                    "(showing first {} of {} modified files)",
                    shown,
                    diff.modified.len()
                ).dimmed()
            );
        }
    }

    Ok(())
}
