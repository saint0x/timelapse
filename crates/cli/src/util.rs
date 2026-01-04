//! Shared utilities for CLI commands

use anyhow::{Context, Result};
use journal::{Checkpoint, Journal, PinManager};
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use ulid::Ulid;

/// Find repository root by walking up from cwd to find .tl/
pub fn find_repo_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()
        .context("Failed to get current directory")?;

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

/// Resolve checkpoint reference to ULID
/// Supports:
/// - Full ULID: "01HN8XYZ..."
/// - Short ULID prefix: "01HN8" (must be unique)
/// - Pin name: "my-pin"
pub fn resolve_checkpoint_ref(
    reference: &str,
    journal: &Journal,
    pin_manager: &PinManager,
) -> Result<Ulid> {
    // Try parsing as ULID first
    if let Ok(ulid) = Ulid::from_string(reference) {
        // Verify it exists
        if journal.get(&ulid)?.is_some() {
            return Ok(ulid);
        } else {
            anyhow::bail!("Checkpoint not found: {}", reference);
        }
    }

    // Try as ULID prefix
    if reference.len() >= 4 {
        let all_checkpoints = journal.all_checkpoint_ids()?;
        let matching: Vec<_> = all_checkpoints
            .iter()
            .filter(|id| id.to_string().starts_with(reference))
            .collect();

        if matching.len() == 1 {
            return Ok(*matching[0]);
        } else if matching.len() > 1 {
            anyhow::bail!(
                "Ambiguous checkpoint prefix '{}': matches {} checkpoints",
                reference,
                matching.len()
            );
        }
    }

    // Try as pin name
    let pins = pin_manager.list_pins()?;
    for (name, ulid) in pins {
        if name == reference {
            return Ok(ulid);
        }
    }

    anyhow::bail!("Unknown checkpoint reference: '{}'", reference)
}

/// Format timestamp as relative time ("2 hours ago")
pub fn format_relative_time(ts_ms: u64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let duration = Duration::from_millis(ts_ms);
    let datetime = UNIX_EPOCH + duration;

    if let Ok(elapsed) = SystemTime::now().duration_since(datetime) {
        let seconds = elapsed.as_secs();

        if seconds < 60 {
            format!("{} seconds ago", seconds)
        } else if seconds < 3600 {
            format!("{} minutes ago", seconds / 60)
        } else if seconds < 86400 {
            format!("{} hours ago", seconds / 3600)
        } else if seconds < 604800 {
            format!("{} days ago", seconds / 86400)
        } else {
            format!("{} weeks ago", seconds / 604800)
        }
    } else {
        "in the future".to_string()
    }
}

/// Format timestamp as absolute time ("2024-01-03 14:30:00")
pub fn format_absolute_time(ts_ms: u64) -> String {
    use std::time::Duration;

    let duration = Duration::from_millis(ts_ms);

    // Format as UTC time string (simplified civil calendar calculation)
    let secs = duration.as_secs();
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    // Calculate approximate date from Unix epoch
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let epoch_days = days + 719468; // Days from 0000-01-01 to 1970-01-01
    let era = epoch_days / 146097;
    let doe = epoch_days - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, m, d, hours, minutes, seconds
    )
}

/// Format file size in human-readable format
pub fn format_size(bytes: u64) -> String {
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

/// Display a checkpoint in compact format
pub fn display_checkpoint_compact(cp: &Checkpoint, show_ulid: bool) {
    let id_short = cp.id.to_string()[..8].to_string();
    let time_str = format_relative_time(cp.ts_unix_ms);
    let reason = match cp.reason {
        journal::CheckpointReason::FsBatch => "fs",
        journal::CheckpointReason::Manual => "manual",
        journal::CheckpointReason::Restore => "restore",
        journal::CheckpointReason::Publish => "publish",
        journal::CheckpointReason::GcCompact => "gc",
    };

    if show_ulid {
        println!(
            "{} {} {} - {} files",
            id_short.yellow(),
            time_str.dimmed(),
            reason.cyan(),
            cp.meta.files_changed
        );
    } else {
        println!(
            "{} {} - {} files",
            time_str.dimmed(),
            reason.cyan(),
            cp.meta.files_changed
        );
    }
}

/// Build a map from checkpoint ULID to pin names
pub fn build_pin_map(pin_manager: &PinManager) -> Result<HashMap<Ulid, Vec<String>>> {
    let pins = pin_manager.list_pins()?;
    let mut map: HashMap<Ulid, Vec<String>> = HashMap::new();
    for (name, ulid) in pins {
        map.entry(ulid).or_insert_with(Vec::new).push(name);
    }
    Ok(map)
}

/// Calculate directory size recursively
pub fn calculate_dir_size(dir: &Path) -> Result<u64> {
    if !dir.exists() {
        return Ok(0);
    }

    let mut total = 0u64;

    for entry in std::fs::read_dir(dir)? {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_size(1536), "1.50 KB");
    }

    #[test]
    fn test_format_relative_time() {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Test current time
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let result = format_relative_time(now_ms);
        assert!(result.contains("seconds ago") || result == "0 seconds ago");

        // Test 1 hour ago
        let one_hour_ago = now_ms - (3600 * 1000);
        let result = format_relative_time(one_hour_ago);
        assert!(result.contains("hour"));

        // Test 1 day ago
        let one_day_ago = now_ms - (86400 * 1000);
        let result = format_relative_time(one_day_ago);
        assert!(result.contains("day"));
    }
}
