//! Pull from Git remote and optionally import to checkpoints

use anyhow::{Context, Result};
use crate::util;
use owo_colors::OwoColorize;
use std::process::Command;

pub async fn run(
    fetch_only: bool,
    _no_pin: bool, // Reserved for future checkpoint import feature
) -> Result<()> {
    // 1. Find repository root
    let repo_root = util::find_repo_root()?;

    // 2. Verify JJ workspace exists
    if jj::detect_jj_workspace(&repo_root)?.is_none() {
        anyhow::bail!("No JJ workspace found. Run 'jj git init' first.");
    }

    // 3. Run jj git fetch
    println!("{}", "Fetching from Git remote...".dimmed());
    let status = Command::new("jj")
        .current_dir(&repo_root)
        .args(&["git", "fetch"])
        .status()
        .context("Failed to execute jj fetch")?;

    if !status.success() {
        anyhow::bail!("JJ fetch failed");
    }

    println!("{} Fetched from remote", "✓".green());

    if fetch_only {
        println!();
        println!("{}", "Fetch complete. Use 'jj rebase' to integrate changes.".dimmed());
        return Ok(());
    }

    // Note: Checkpoint import from JJ commits would be implemented here
    // For now, we just fetch

    println!();
    println!("{}", "Note: Use 'jj rebase' to integrate changes.".dimmed());

    Ok(())
}
