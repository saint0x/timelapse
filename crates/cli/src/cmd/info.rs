//! Show detailed repository information and statistics

use anyhow::{Context, Result};
use tl_core::store::Store;
use std::fs;
use std::path::{Path, PathBuf};

/// Repository information structure
#[derive(Debug)]
pub struct RepoInfo {
    pub root: PathBuf,
    pub tl_dir: PathBuf,
    pub total_size: u64,
    pub blob_count: usize,
    pub blob_size: u64,
    pub tree_count: usize,
    pub tree_size: u64,
    pub checkpoint_count: usize,
    pub journal_size: u64,
    pub pin_count: usize,
    pub latest_checkpoint_id: Option<String>,
    pub latest_checkpoint_time: Option<String>,
}

pub async fn run() -> Result<()> {
    // Find repository root
    let repo_root = find_repo_root()?;
    let tl_dir = repo_root.join(".tl");

    // Ensure daemon running (auto-starts if needed)
    crate::daemon::ensure_daemon_running().await?;

    // Get checkpoint info via unified data access layer
    let (checkpoint_count, checkpoint_ids, _store_size_from_daemon) =
        crate::data_access::get_info_data(&tl_dir).await?;

    // Get latest checkpoint if available
    let latest_checkpoint = if !checkpoint_ids.is_empty() {
        // Parse the first ID (most recent)
        let latest_id = ulid::Ulid::from_string(&checkpoint_ids[0])?;
        let checkpoints = crate::data_access::get_checkpoints(&[latest_id], &tl_dir).await?;
        checkpoints[0].clone()
    } else {
        None
    };

    // Open store for object stats
    let store = Store::open(&repo_root)
        .context("Failed to open Timelapse store. Is this a Timelapse repository?")?;

    // Gather statistics
    let info = gather_info(&repo_root, &store, checkpoint_count, latest_checkpoint)?;

    // Display information
    display_info(&info);

    Ok(())
}

fn find_repo_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        let tl_dir = current.join(".tl");
        if tl_dir.exists() && tl_dir.is_dir() {
            return Ok(current);
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => anyhow::bail!("Not a Timelapse repository (no .tl directory found)"),
        }
    }
}

fn gather_info(
    repo_root: &Path,
    _store: &Store,
    checkpoint_count: usize,
    latest_checkpoint: Option<journal::Checkpoint>,
) -> Result<RepoInfo> {
    let tl_dir = repo_root.join(".tl");

    // Count blobs
    let blobs_dir = tl_dir.join("objects/blobs");
    let (blob_count, blob_size) = count_objects(&blobs_dir)?;

    // Count trees
    let trees_dir = tl_dir.join("objects/trees");
    let (tree_count, tree_size) = count_objects(&trees_dir)?;

    // Journal size
    let journal_dir = tl_dir.join("journal");
    let journal_size = calculate_dir_size(&journal_dir)?;

    // Latest checkpoint
    let (latest_checkpoint_id, latest_checkpoint_time) = match latest_checkpoint {
        Some(checkpoint) => {
            let id = checkpoint.id.to_string();
            let timestamp = checkpoint.ts_unix_ms;
            let datetime = format_timestamp(timestamp);
            (Some(id), Some(datetime))
        }
        None => (None, None),
    };

    // Count pins
    let pins_dir = tl_dir.join("refs/pins");
    let pin_count = if pins_dir.exists() {
        fs::read_dir(&pins_dir)?.count()
    } else {
        0
    };

    // Total .tl directory size
    let total_size = calculate_dir_size(&tl_dir)?;

    Ok(RepoInfo {
        root: repo_root.to_path_buf(),
        tl_dir,
        total_size,
        blob_count,
        blob_size,
        tree_count,
        tree_size,
        checkpoint_count,
        journal_size,
        pin_count,
        latest_checkpoint_id,
        latest_checkpoint_time,
    })
}

fn count_objects(dir: &Path) -> Result<(usize, u64)> {
    if !dir.exists() {
        return Ok((0, 0));
    }

    let mut count = 0;
    let mut total_size = 0u64;

    // Walk through all subdirectories (objects are stored in 2-char prefix dirs)
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Read files in subdirectory
            for sub_entry in fs::read_dir(&path)? {
                let sub_entry = sub_entry?;
                if sub_entry.path().is_file() {
                    count += 1;
                    total_size += sub_entry.metadata()?.len();
                }
            }
        }
    }

    Ok((count, total_size))
}

fn calculate_dir_size(dir: &Path) -> Result<u64> {
    if !dir.exists() {
        return Ok(0);
    }

    let mut total = 0u64;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            total += entry.metadata()?.len();
        } else if path.is_dir() {
            total += calculate_dir_size(&path)?;
        }
    }

    Ok(total)
}

fn format_timestamp(ts_ms: u64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let duration = Duration::from_millis(ts_ms);
    let datetime = UNIX_EPOCH + duration;

    // Format as human-readable string
    if let Ok(elapsed) = SystemTime::now().duration_since(datetime) {
        let seconds = elapsed.as_secs();

        if seconds < 60 {
            format!("{} seconds ago", seconds)
        } else if seconds < 3600 {
            format!("{} minutes ago", seconds / 60)
        } else if seconds < 86400 {
            format!("{} hours ago", seconds / 3600)
        } else {
            format!("{} days ago", seconds / 86400)
        }
    } else {
        "in the future".to_string()
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn display_info(info: &RepoInfo) {
    println!("Timelapse Repository Information");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Repository location
    println!("Repository:");
    println!("  Location:     {}", info.root.display());
    println!("  .tl directory: {}", info.tl_dir.display());
    println!("  Total size:   {}", format_size(info.total_size));
    println!();

    // Checkpoint information
    println!("Checkpoints:");
    println!("  Total:        {}", info.checkpoint_count);
    if let Some(ref id) = info.latest_checkpoint_id {
        println!("  Latest ID:    {}", id);
    }
    if let Some(ref time) = info.latest_checkpoint_time {
        println!("  Latest time:  {}", time);
    }
    println!("  Journal size: {}", format_size(info.journal_size));
    println!();

    // Object storage
    println!("Storage:");
    println!("  Blobs:        {} objects, {}",
        info.blob_count, format_size(info.blob_size));
    println!("  Trees:        {} objects, {}",
        info.tree_count, format_size(info.tree_size));
    println!("  Total objects: {} objects, {}",
        info.blob_count + info.tree_count,
        format_size(info.blob_size + info.tree_size));
    println!();

    // Pins
    println!("Pins:");
    println!("  Total:        {}", info.pin_count);
    println!();

    // Storage breakdown
    let object_storage = info.blob_size + info.tree_size;
    let overhead = info.total_size.saturating_sub(object_storage + info.journal_size);

    println!("Storage breakdown:");
    println!("  Objects:      {} ({:.1}%)",
        format_size(object_storage),
        (object_storage as f64 / info.total_size as f64 * 100.0));
    println!("  Journal:      {} ({:.1}%)",
        format_size(info.journal_size),
        (info.journal_size as f64 / info.total_size as f64 * 100.0));
    println!("  Overhead:     {} ({:.1}%)",
        format_size(overhead),
        (overhead as f64 / info.total_size as f64 * 100.0));

    // Efficiency metrics
    if info.checkpoint_count > 0 {
        let avg_checkpoint_size = info.total_size / info.checkpoint_count as u64;
        println!();
        println!("Efficiency:");
        println!("  Avg checkpoint: {}", format_size(avg_checkpoint_size));

        if info.blob_count > 0 {
            let avg_blob_size = info.blob_size / info.blob_count as u64;
            println!("  Avg blob size:  {}", format_size(avg_blob_size));
        }
    }
}
