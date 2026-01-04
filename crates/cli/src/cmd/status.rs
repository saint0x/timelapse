//! Show daemon and checkpoint status

use crate::util;
use anyhow::{Context, Result};
use journal::Journal;
use owo_colors::OwoColorize;

pub async fn run() -> Result<()> {
    // 1. Find repository root
    let repo_root = util::find_repo_root()
        .context("Failed to find repository")?;

    let tl_dir = repo_root.join(".tl");

    // 2. Check daemon status
    let daemon_running = crate::daemon::is_running().await;

    // 3. Open journal
    let journal_path = tl_dir.join("journal");
    let journal = Journal::open(&journal_path)
        .context("Failed to open checkpoint journal")?;

    // 4. Get latest checkpoint
    let latest = journal.latest()?;

    // 5. Get storage stats
    let checkpoint_count = journal.count();
    let total_size = util::calculate_dir_size(&tl_dir)?;

    // 6. Display output
    println!("{}", "Repository Status".bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Repository path
    println!("Repository:    {}", repo_root.display().to_string().cyan());
    println!();

    // Daemon status
    print!("Daemon:        ");
    if daemon_running {
        println!("{}", "Running ✓".green());

        // If daemon is running, try to get detailed status via IPC
        let socket_path = tl_dir.join("state/daemon.sock");
        if let Ok(mut client) = crate::ipc::IpcClient::connect(&socket_path).await {
            if let Ok(status) = client.get_status().await {
                println!("  PID:         {}", status.pid);
                println!(
                    "  Uptime:      {} seconds",
                    status.uptime_secs
                );
                println!(
                    "  Checkpoints: {} created",
                    status.checkpoints_created
                );
                if let Some(ts) = status.last_checkpoint_time {
                    println!(
                        "  Last:        {}",
                        util::format_relative_time(ts)
                    );
                }
                println!("  Watching:    {} paths", status.watcher_paths);
            }
        }
    } else {
        println!("{}", "Not running".yellow());
        println!("  {}", "Tip: Start with 'tl start'".dimmed());
    }
    println!();

    // Latest checkpoint
    println!("Latest checkpoint:");
    if let Some(cp) = latest {
        let id_short = cp.id.to_string()[..8].to_string();
        let time_str = util::format_relative_time(cp.ts_unix_ms);
        let abs_time = util::format_absolute_time(cp.ts_unix_ms);

        println!("  ID:          {}", id_short.yellow());
        println!("  Time:        {} ({})", time_str, abs_time.dimmed());
        println!("  Files:       {}", cp.meta.files_changed);
        println!("  Reason:      {:?}", cp.reason);

        if !cp.touched_paths.is_empty() {
            println!("  Changed:");
            for path in cp.touched_paths.iter().take(5) {
                println!("    - {}", path.display());
            }
            if cp.touched_paths.len() > 5 {
                println!("    ... and {} more", cp.touched_paths.len() - 5);
            }
        }
    } else {
        println!("  {}", "No checkpoints yet".dimmed());
    }
    println!();

    // Storage summary
    println!("Storage:");
    println!("  Checkpoints: {}", checkpoint_count);
    println!("  Total size:  {}", util::format_size(total_size));
    println!();

    // Helpful hints
    if checkpoint_count == 0 && !daemon_running {
        println!(
            "{}",
            "Tip: Start the daemon to begin tracking changes automatically".dimmed()
        );
    } else if !daemon_running {
        println!(
            "{}",
            "Note: Daemon is not running. Automatic checkpoints are paused.".dimmed()
        );
    }

    Ok(())
}
