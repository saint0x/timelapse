//! Publish checkpoint(s) to JJ

use anyhow::{anyhow, Context, Result};
use crate::util;
use owo_colors::OwoColorize;
use tl_core::Store;
use journal::{Journal, PinManager};
use jj::{JjMapping, publish};
use jj::materialize::{CommitMessageOptions, PublishOptions};

pub async fn run(
    checkpoint_ref: &str,
    bookmark: Option<String>,
    compact: bool,
    no_pin: bool,
    message_template: Option<String>,
) -> Result<()> {
    // 1. Find repository root
    let repo_root = util::find_repo_root()
        .context("Failed to find repository")?;

    let tl_dir = repo_root.join(".tl");

    // 2. Verify JJ workspace exists
    if jj::detect_jj_workspace(&repo_root)?.is_none() {
        anyhow::bail!("No JJ workspace found. Run 'jj git init' first.");
    }

    // 3. Open components
    let store = Store::open(&repo_root)?;
    let journal = Journal::open(&tl_dir.join("journal"))?;
    let pin_manager = PinManager::new(&tl_dir);
    let mapping = JjMapping::open(&tl_dir)?;

    // 4. Parse checkpoint reference (support ranges like HEAD~10..HEAD)
    let checkpoints = if checkpoint_ref.contains("..") {
        // Range syntax
        parse_checkpoint_range(checkpoint_ref, &journal, &pin_manager)?
    } else {
        // Single checkpoint
        let checkpoint_id = util::resolve_checkpoint_ref(
            checkpoint_ref, &journal, &pin_manager
        )?;
        let cp = journal.get(&checkpoint_id)?
            .ok_or_else(|| anyhow!("Checkpoint not found"))?;
        vec![cp]
    };

    // 5. Configure publish options
    let mut msg_options = CommitMessageOptions::default();
    if let Some(template) = message_template {
        msg_options.template = Some(template);
    }

    let publish_options = PublishOptions {
        auto_pin: if no_pin { None } else { Some("published".to_string()) },
        message_options: msg_options,
        compact_range: compact,
    };

    // 6. Publish checkpoint(s)
    println!("{}", "Publishing checkpoints to JJ...".dimmed());

    let commit_ids = publish::publish_range(
        checkpoints.clone(),
        &store,
        &repo_root,
        &mapping,
        &publish_options,
    )?;

    // 7. Create bookmark if specified
    if let Some(bookmark_name) = bookmark {
        let bookmark = format!("snap/{}", bookmark_name);
        let last_commit_id = commit_ids.last().unwrap();

        std::process::Command::new("jj")
            .current_dir(&repo_root)
            .args(&["bookmark", "create", &bookmark, "-r", last_commit_id])
            .status()?;

        println!("{} Created bookmark: {}", "✓".green(), bookmark.yellow());
    }

    // 8. Auto-pin if configured
    if !no_pin {
        for checkpoint in &checkpoints {
            pin_manager.pin("published", checkpoint.id)?;
        }
    }

    // 9. Display results
    println!();
    println!("{} Published {} checkpoint(s)",
        "✓".green(),
        commit_ids.len().to_string().green()
    );

    for (i, commit_id) in commit_ids.iter().enumerate() {
        let short_id = &checkpoints[i].id.to_string()[..8];
        let short_commit = &commit_id[..12.min(commit_id.len())];
        println!("  {} → {}",
            short_id.yellow(),
            short_commit.cyan()
        );
    }

    Ok(())
}

fn parse_checkpoint_range(
    range: &str,
    journal: &Journal,
    pin_manager: &PinManager,
) -> Result<Vec<journal::Checkpoint>> {
    let parts: Vec<&str> = range.split("..").collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid range syntax. Use: <start>..<end>");
    }

    let start_id = util::resolve_checkpoint_ref(parts[0], journal, pin_manager)?;
    let end_id = util::resolve_checkpoint_ref(parts[1], journal, pin_manager)?;

    // Walk backwards from end until we hit start
    let mut checkpoints = Vec::new();
    let mut current_id = end_id;

    loop {
        let cp = journal.get(&current_id)?
            .ok_or_else(|| anyhow!("Checkpoint not found in range"))?;

        checkpoints.push(cp.clone());

        if current_id == start_id {
            break;
        }

        current_id = cp.parent
            .ok_or_else(|| anyhow!("Range includes root checkpoint"))?;
    }

    checkpoints.reverse(); // Oldest first
    Ok(checkpoints)
}
