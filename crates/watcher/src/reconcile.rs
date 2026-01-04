//! Periodic reconciliation scanner
//!
//! Periodically scans repository for changes that may have been missed
//! by the file watcher (due to overflow, race conditions, etc.)

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use tokio::time::interval;
use walkdir::WalkDir;
use tracing::{info, debug, warn};
use anyhow::Result;

/// Periodic reconciliation scanner
///
/// Periodically scans repository for changes that may have been missed
/// by the file watcher (due to overflow, race conditions, etc.)
pub struct PeriodicReconciler {
    /// Repository root directory
    repo_root: PathBuf,

    /// Scan interval (default: 5 minutes)
    interval: Duration,

    /// Last checkpoint timestamp (used for mtime comparison)
    last_checkpoint: SystemTime,

    /// Sender for detected changes
    change_tx: mpsc::Sender<Vec<PathBuf>>,
}

impl PeriodicReconciler {
    /// Create new periodic reconciler
    pub fn new(
        repo_root: PathBuf,
        interval: Duration,
        change_tx: mpsc::Sender<Vec<PathBuf>>,
    ) -> Self {
        Self {
            repo_root,
            interval,
            last_checkpoint: SystemTime::now(),
            change_tx,
        }
    }

    /// Run periodic reconciliation loop
    ///
    /// This spawns a background task that runs indefinitely.
    /// Call from daemon startup.
    pub async fn run(self) -> Result<()> {
        let mut timer = interval(self.interval);

        info!("Starting periodic reconciliation (interval: {:?})", self.interval);

        loop {
            timer.tick().await;

            match self.scan_for_changes().await {
                Ok(changed) => {
                    if !changed.is_empty() {
                        info!("Periodic reconciliation found {} missed changes", changed.len());

                        // Send to checkpoint pipeline
                        if let Err(e) = self.change_tx.send(changed).await {
                            warn!("Failed to send reconciliation changes: {}", e);
                        }
                    } else {
                        debug!("Periodic reconciliation: no missed changes");
                    }
                }
                Err(e) => {
                    warn!("Periodic reconciliation scan failed: {}", e);
                }
            }
        }
    }

    /// Scan repository for changes since last checkpoint
    ///
    /// Uses mtime-based heuristic (same as overflow recovery)
    async fn scan_for_changes(&self) -> Result<Vec<PathBuf>> {
        let checkpoint_time = self.last_checkpoint;
        let mut changed = Vec::new();

        // Walk repository
        for entry in WalkDir::new(&self.repo_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !self.should_ignore(e.path()))
        {
            let entry = entry?;

            // Only check files
            if !entry.file_type().is_file() {
                continue;
            }

            // Check mtime
            let metadata = entry.metadata()?;
            let mtime = metadata.modified()?;

            if mtime > checkpoint_time {
                let rel_path = entry.path().strip_prefix(&self.repo_root)?;
                changed.push(rel_path.to_path_buf());
            }
        }

        Ok(changed)
    }

    /// Check if path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        // Check each component of the path
        for component in path.components() {
            if let Some(comp_str) = component.as_os_str().to_str() {
                match comp_str {
                    ".tl" | ".git" | ".jj" | "target" | "node_modules" | ".cache" => return true,
                    _ => {}
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_reconciliation_finds_missed_changes() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create initial state
        let file1 = repo_root.join("file1.txt");
        let file2 = repo_root.join("file2.txt");
        fs::write(&file1, b"content 1").unwrap();
        fs::write(&file2, b"content 2").unwrap();

        // Create reconciler with 1-second interval
        let (tx, mut rx) = mpsc::channel(10);
        let reconciler = PeriodicReconciler::new(
            repo_root.to_path_buf(),
            Duration::from_secs(1),
            tx,
        );

        // Spawn reconciler
        tokio::spawn(reconciler.run());

        // Wait for first scan
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Modify file (simulating missed watcher event)
        fs::write(&file1, b"modified content").unwrap();

        // Wait for next scan
        let changed = tokio::time::timeout(
            Duration::from_secs(2),
            rx.recv()
        ).await.unwrap().unwrap();

        // Should find file1
        assert_eq!(changed.len(), 1);
        assert!(changed[0].ends_with("file1.txt"));
    }

    #[tokio::test]
    async fn test_reconciliation_ignores_unchanged_files() {
        use filetime::{FileTime, set_file_mtime};

        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create old file
        let file = repo_root.join("old.txt");
        fs::write(&file, b"old").unwrap();

        // Backdate mtime to 10 minutes ago
        let old_time = SystemTime::now() - Duration::from_secs(600);
        set_file_mtime(&file, FileTime::from_system_time(old_time)).unwrap();

        // Create reconciler
        let (tx, mut rx) = mpsc::channel(10);
        let reconciler = PeriodicReconciler::new(
            repo_root.to_path_buf(),
            Duration::from_millis(100),
            tx,
        );

        tokio::spawn(reconciler.run());

        // Wait for scan
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should NOT report old file
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_should_ignore_standard_paths() {
        let temp_dir = TempDir::new().unwrap();
        let reconciler = PeriodicReconciler::new(
            temp_dir.path().to_path_buf(),
            Duration::from_secs(300),
            mpsc::channel(1).0,
        );

        assert!(reconciler.should_ignore(Path::new(".tl/journal/db")));
        assert!(reconciler.should_ignore(Path::new(".git/objects/ab/cd")));
        assert!(reconciler.should_ignore(Path::new(".jj/op_store/data")));
        assert!(reconciler.should_ignore(Path::new("target/debug/app")));
        assert!(reconciler.should_ignore(Path::new("node_modules/pkg/index.js")));
        assert!(!reconciler.should_ignore(Path::new("src/main.rs")));
    }
}
