//! Push to Git remote via JJ

use anyhow::{Context, Result};
use crate::util;
use owo_colors::OwoColorize;
use std::process::Command;

pub async fn run(
    bookmark: Option<String>,
    all: bool,
    force: bool,
) -> Result<()> {
    // 1. Find repository root
    let repo_root = util::find_repo_root()?;

    // 2. Verify JJ workspace exists
    if jj::detect_jj_workspace(&repo_root)?.is_none() {
        anyhow::bail!("No JJ workspace found. Run 'jj git init' first.");
    }

    // 3. Build push command
    let mut args = vec!["git", "push"];
    let bookmark_str; // Hold formatted string outside if block

    if all {
        args.push("--all");
    } else if let Some(ref b) = bookmark {
        args.push("-b");
        bookmark_str = format!("snap/{}", b);
        args.push(&bookmark_str);
    }

    if force {
        args.push("--force");
    }

    // 4. Execute push
    println!("{}", "Pushing to Git remote...".dimmed());
    let status = Command::new("jj")
        .current_dir(&repo_root)
        .args(&args)
        .status()
        .context("Failed to execute jj push")?;

    if !status.success() {
        anyhow::bail!("JJ push failed");
    }

    // 5. Success
    println!("{} Pushed to remote", "✓".green());
    Ok(())
}
