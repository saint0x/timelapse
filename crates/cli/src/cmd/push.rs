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

    // 3. Pre-push validation: check git remote exists
    let remote_check = Command::new("git")
        .current_dir(&repo_root)
        .args(&["remote", "-v"])
        .output()
        .context("Failed to check git remotes")?;

    if remote_check.stdout.is_empty() {
        println!("{} No git remotes configured.", "Warning:".yellow());
        println!("{}", "Add a remote first: git remote add origin <url>".dimmed());
        anyhow::bail!("No git remotes configured");
    }

    // 4. Build push command
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

    // 5. Execute push with detailed error capture
    println!("{}", "Pushing to Git remote...".dimmed());
    let output = Command::new("jj")
        .current_dir(&repo_root)
        .args(&args)
        .output()
        .context("Failed to execute jj push")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse common error scenarios
        if stderr.contains("authentication") || stderr.contains("Authentication") {
            println!("{} Authentication failed", "Error:".red());
            println!("{}", "Configure credentials for your Git provider:".dimmed());
            println!("{}", "  - GitHub: Use SSH keys or GitHub CLI (gh auth login)".dimmed());
            println!("{}", "  - GitLab: Use SSH keys or personal access tokens".dimmed());
            anyhow::bail!("Authentication failed");
        } else if stderr.contains("rejected") || stderr.contains("non-fast-forward") {
            println!("{} Push rejected by remote", "Error:".red());
            println!("{}", "The remote has changes you don't have locally.".dimmed());
            println!("{}", "Try: tl pull && jj rebase".dimmed());
            anyhow::bail!("Push rejected (non-fast-forward)");
        } else if stderr.contains("No such remote") || stderr.contains("not found") {
            println!("{} Remote repository not found", "Error:".red());
            println!("{}", "Verify the remote URL is correct: git remote -v".dimmed());
            anyhow::bail!("Remote repository not found");
        } else if stderr.contains("network") || stderr.contains("timeout") || stderr.contains("Connection") {
            println!("{} Network error", "Error:".red());
            println!("{}", "Check your internet connection and try again.".dimmed());
            anyhow::bail!("Network error during push");
        } else {
            // Generic error with stderr output
            println!("{} Push failed:", "Error:".red());
            println!("{}", stderr.trim());
            anyhow::bail!("JJ push failed");
        }
    }

    // 6. Success - display what was pushed
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{} Pushed to remote", "✓".green());

    if let Some(ref b) = bookmark {
        println!("  Bookmark: {}", format!("snap/{}", b).cyan());
    } else if all {
        println!("  All bookmarks pushed");
    }

    // Show any informational output from jj
    if !stdout.is_empty() && !stdout.trim().is_empty() {
        println!("{}", stdout.trim().dimmed());
    }

    Ok(())
}
