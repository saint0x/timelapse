//! Resolve command - Check and manage conflict resolution
//!
//! Shows status of conflicts during a merge and provides shortcuts
//! for continuing or aborting the merge.
//!
//! Usage:
//!   tl resolve              # Show conflict status
//!   tl resolve --list       # List files with resolution status
//!   tl resolve --continue   # Shortcut for 'tl merge --continue'
//!   tl resolve --abort      # Shortcut for 'tl merge --abort'

use anyhow::{anyhow, Result};
use crate::util;
use jj::MergeState;
use owo_colors::OwoColorize;

/// Run the resolve command
pub async fn run(
    list: bool,
    continue_merge: bool,
    abort: bool,
) -> Result<()> {
    // 1. Find repository root
    let repo_root = util::find_repo_root()?;
    let tl_dir = repo_root.join(".tl");

    // 2. Load merge state
    let merge_state = MergeState::load(&tl_dir)?;

    let state = match merge_state {
        Some(s) if s.in_progress => s,
        _ => {
            if continue_merge || abort {
                anyhow::bail!("No merge in progress.");
            }
            println!("{}", "No merge in progress.".dimmed());
            return Ok(());
        }
    };

    // Handle --continue (shortcut to merge --continue)
    if continue_merge {
        return crate::cmd::merge::run(None, false, true).await;
    }

    // Handle --abort (shortcut to merge --abort)
    if abort {
        return crate::cmd::merge::run(None, true, false).await;
    }

    // Show conflict status
    println!("{}", "Merge Status".bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Branch: {}", state.theirs_branch.bright_yellow());

    if let Some(base) = &state.base_commit {
        let short_base = &base[..12.min(base.len())];
        println!("Base:   {}", short_base.dimmed());
    }

    let short_ours = &state.ours_commit[..12.min(state.ours_commit.len())];
    let short_theirs = &state.theirs_commit[..12.min(state.theirs_commit.len())];
    println!("Ours:   {}", short_ours.dimmed());
    println!("Theirs: {}", short_theirs.dimmed());
    println!();

    // Check conflict status
    let mut resolved_count = 0;
    let mut unresolved_count = 0;

    if list || !state.conflicts.is_empty() {
        println!("{}", "Conflicts:".bold());

        for path in &state.conflicts {
            let file_path = repo_root.join(path);
            let has_markers = jj::has_conflict_markers(&file_path)?;

            if has_markers {
                println!("  {} {} {}", "✗".red(), path, "(unresolved)".red());
                unresolved_count += 1;
            } else if file_path.exists() {
                println!("  {} {} {}", "✓".green(), path, "(resolved)".green());
                resolved_count += 1;
            } else {
                println!("  {} {} {}", "?".yellow(), path, "(missing)".yellow());
                unresolved_count += 1;
            }
        }

        println!();
    }

    // Summary
    let total = state.conflicts.len();
    if unresolved_count == 0 {
        println!("{} All {} conflicts resolved!", "✓".green(), total);
        println!();
        println!("Run {} to complete the merge.", "'tl merge --continue'".bright_cyan());
    } else {
        println!("{} {}/{} conflicts resolved", "!".yellow(), resolved_count, total);
        println!();
        println!("{}", "To resolve:".bold());
        println!("  1. Edit the conflicted files (look for <<<<<<< markers)");
        println!("  2. Remove the conflict markers after choosing the correct version");
        println!("  3. Run {} again to check status", "'tl resolve'".bright_cyan());
        println!();
        println!("{}", "To complete:".bold());
        println!("  Run {} after resolving all conflicts", "'tl merge --continue'".bright_cyan());
        println!();
        println!("{}", "To abort:".bold());
        println!("  Run {} to restore pre-merge state", "'tl merge --abort'".bright_cyan());
    }

    Ok(())
}

/// Show detailed conflict information for a specific file
pub async fn show_file_conflicts(file_path: &str) -> Result<()> {
    let repo_root = util::find_repo_root()?;
    let full_path = repo_root.join(file_path);

    if !full_path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    if !jj::has_conflict_markers(&full_path)? {
        println!("{} {} has no conflict markers", "✓".green(), file_path);
        return Ok(());
    }

    let content = std::fs::read_to_string(&full_path)?;
    let regions = jj::parse_conflict_regions(&content);

    println!("{}", format!("Conflicts in {}", file_path).bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    for (i, region) in regions.iter().enumerate() {
        println!("{} Conflict {} (lines {}-{})", "•".red(), i + 1, region.start_line, region.end_line);
        println!();

        println!("  {} (your changes):", "LOCAL".cyan());
        for line in region.ours.lines() {
            println!("    {}", line);
        }

        if let Some(base) = &region.base {
            println!();
            println!("  {} (common ancestor):", "BASE".dimmed());
            for line in base.lines() {
                println!("    {}", line);
            }
        }

        println!();
        println!("  {} (incoming changes):", "REMOTE".yellow());
        for line in region.theirs.lines() {
            println!("    {}", line);
        }
        println!();
    }

    println!("{} {} conflict region(s) found", "!".yellow(), regions.len());

    Ok(())
}
