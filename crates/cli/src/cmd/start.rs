//! Start the Timelapse daemon

use anyhow::{Context, Result};
use std::time::Duration;

pub async fn run(foreground: bool) -> Result<()> {
    if foreground {
        // Run daemon in foreground (for debugging)
        crate::daemon::start().await
    } else {
        // Start daemon in background
        start_background().await
    }
}

async fn start_background() -> Result<()> {
    use crate::util;
    use std::process::Command;

    let repo_root = util::find_repo_root()?;
    let log_file = repo_root.join(".tl/logs/daemon.log");

    // Ensure logs directory exists
    std::fs::create_dir_all(repo_root.join(".tl/logs"))
        .context("Failed to create logs directory")?;

    // Get current executable path
    let exe = std::env::current_exe()
        .context("Failed to get current executable path")?;

    // Spawn daemon in background with nohup
    let log_file_writer = std::fs::File::create(&log_file)
        .context("Failed to create log file")?;

    Command::new("nohup")
        .arg(&exe)
        .arg("start")
        .arg("--foreground")
        .stdout(log_file_writer.try_clone()?)
        .stderr(log_file_writer)
        .spawn()
        .context("Failed to spawn daemon process")?;

    // Wait a moment to verify it started
    tokio::time::sleep(Duration::from_millis(500)).await;

    if crate::daemon::is_running().await {
        println!("Daemon started successfully");
        println!("Logs: {}", log_file.display());
        Ok(())
    } else {
        anyhow::bail!(
            "Daemon failed to start (check logs at {})",
            log_file.display()
        );
    }
}
