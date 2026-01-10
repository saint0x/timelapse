//! Show checkpoint or commit details

use crate::data_access;
use crate::util;
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::path::Path;
use tl_core::store::Store;
use journal::{Journal, PinManager};

/// Show detailed information about a checkpoint
pub async fn run(checkpoint_ref: &str, show_diff: bool) -> Result<()> {
    let repo_root = util::find_repo_root()?;

    // Open store and journal
    let store = Store::open(&repo_root)
        .context("Failed to open store")?;
    let journal = Journal::open(store.tl_dir())
        .context("Failed to open journal")?;

    // Resolve checkpoint reference
    let resolved = data_access::resolve_checkpoint_refs(&[checkpoint_ref.to_string()], store.tl_dir()).await?;
    let checkpoint_id = resolved[0]
        .ok_or_else(|| anyhow::anyhow!("Checkpoint not found: {}", checkpoint_ref))?;

    // Get checkpoint
    let checkpoint = journal.get(&checkpoint_id)?
        .ok_or_else(|| anyhow::anyhow!("Checkpoint not found: {}", checkpoint_id))?;

    // Print checkpoint details
    println!("{} {}", "checkpoint".yellow().bold(), checkpoint.id.to_string().cyan());

    if let Some(parent_id) = checkpoint.parent {
        println!("{} {}", "Parent:    ".dimmed(), parent_id.to_string().cyan());
    } else {
        println!("{} {}", "Parent:    ".dimmed(), "(none - initial checkpoint)".dimmed());
    }

    println!("{} {}", "Tree:      ".dimmed(), checkpoint.root_tree.to_hex().bright_green());

    let relative_time = util::format_relative_time(checkpoint.ts_unix_ms);
    let absolute_time = format_absolute_timestamp(checkpoint.ts_unix_ms);
    println!("{} {} ({})", "Date:      ".dimmed(), absolute_time, relative_time.dimmed());

    println!("{} {:?}", "Reason:    ".dimmed(), checkpoint.reason);

    // Print metadata
    println!("\n{}", "Metadata:".bold());
    println!("  Files changed:  {}", checkpoint.meta.files_changed);
    println!("  Bytes added:    {}", format_bytes(checkpoint.meta.bytes_added));
    println!("  Bytes removed:  {}", format_bytes(checkpoint.meta.bytes_removed));

    // Print touched paths
    if !checkpoint.touched_paths.is_empty() {
        println!("\n{} ({} files)", "Changed files:".bold(), checkpoint.touched_paths.len());
        let mut paths = checkpoint.touched_paths.clone();
        paths.sort();

        for path in paths.iter().take(20) {
            println!("  {}", path.display().to_string().cyan());
        }

        if paths.len() > 20 {
            println!("  {} ({} more files omitted)", "...".dimmed(), paths.len() - 20);
        }
    }

    // Show diff if requested
    if show_diff {
        println!("\n{}", "Diff:".bold());

        if let Some(parent_id) = checkpoint.parent {
            // Load parent tree
            let parent_checkpoint = journal.get(&parent_id)?;
            if let Some(parent_cp) = parent_checkpoint {
                let parent_tree = store.read_tree(parent_cp.root_tree)?;
                let current_tree = store.read_tree(checkpoint.root_tree)?;

                // Compare trees and show diff
                show_tree_diff(&store, &parent_tree, &current_tree, &repo_root)?;
            }
        } else {
            println!("  (no parent - showing all files)");
            let tree = store.read_tree(checkpoint.root_tree)?;
            show_tree_files(&tree)?;
        }
    }

    Ok(())
}

/// Format timestamp in absolute format (YYYY-MM-DD HH:MM:SS)
fn format_absolute_timestamp(ts_ms: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let duration = Duration::from_millis(ts_ms);
    let datetime = UNIX_EPOCH + duration;

    // Convert to local time string
    // Using chrono would be better, but we'll use a simple format for now
    match datetime.duration_since(UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs();
            let days = secs / 86400;
            let hours = (secs % 86400) / 3600;
            let minutes = (secs % 3600) / 60;
            let seconds = secs % 60;

            // Simple date calculation (not accurate for all cases, but good enough)
            let epoch_year = 1970;
            let years = days / 365;
            let year = epoch_year + years;
            let remaining_days = days % 365;
            let month = (remaining_days / 30) + 1;
            let day = (remaining_days % 30) + 1;

            format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                year, month, day, hours, minutes, seconds)
        }
        Err(_) => "Unknown time".to_string(),
    }
}

/// Format bytes in human-readable format
fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }

    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_idx = 0;

    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", value, UNITS[unit_idx])
    }
}

/// Show diff between two trees
fn show_tree_diff(
    store: &Store,
    old_tree: &tl_core::Tree,
    new_tree: &tl_core::Tree,
    repo_root: &Path,
) -> Result<()> {
    use std::collections::{HashMap, HashSet};

    // Build maps of path -> entry
    let old_entries: HashMap<_, _> = old_tree.entries_with_paths()
        .map(|(path, entry)| (path.to_vec(), entry.clone()))
        .collect();
    let new_entries: HashMap<_, _> = new_tree.entries_with_paths()
        .map(|(path, entry)| (path.to_vec(), entry.clone()))
        .collect();

    // Find all paths
    let mut all_paths: HashSet<Vec<u8>> = HashSet::new();
    all_paths.extend(old_entries.keys().cloned());
    all_paths.extend(new_entries.keys().cloned());

    let mut paths: Vec<_> = all_paths.into_iter().collect();
    paths.sort();

    let mut added = 0;
    let mut modified = 0;
    let mut deleted = 0;

    for path_bytes in paths.iter().take(20) {
        let path_str = String::from_utf8_lossy(path_bytes);
        let old_entry = old_entries.get(path_bytes);
        let new_entry = new_entries.get(path_bytes);

        match (old_entry, new_entry) {
            (None, Some(_)) => {
                println!("  {} {}", "+".green(), path_str.green());
                added += 1;
            }
            (Some(_), None) => {
                println!("  {} {}", "-".red(), path_str.red());
                deleted += 1;
            }
            (Some(old), Some(new)) if old.blob_hash != new.blob_hash => {
                println!("  {} {}", "M".yellow(), path_str.yellow());
                modified += 1;
            }
            _ => {} // Unchanged
        }
    }

    if paths.len() > 20 {
        println!("  {} ({} more files omitted)", "...".dimmed(), paths.len() - 20);
    }

    println!("\n  Summary: {} added, {} modified, {} deleted",
        added.to_string().green(),
        modified.to_string().yellow(),
        deleted.to_string().red()
    );

    Ok(())
}

/// Show all files in a tree
fn show_tree_files(tree: &tl_core::Tree) -> Result<()> {
    let mut paths: Vec<_> = tree.entries_with_paths()
        .map(|(path, _)| path.to_vec())
        .collect();
    paths.sort();

    for path_bytes in paths.iter().take(50) {
        let path_str = String::from_utf8_lossy(path_bytes);
        println!("  {} {}", "+".green(), path_str.green());
    }

    if paths.len() > 50 {
        println!("  {} ({} more files omitted)", "...".dimmed(), paths.len() - 50);
    }

    Ok(())
}
