//! Publish checkpoints to JJ commits
//!
//! This module handles converting Timelapse checkpoints into JJ commits.
//! It uses a hybrid approach:
//! - Materializes checkpoint trees to temp directories (using Timelapse APIs)
//! - Creates JJ commits via CLI (battle-tested, handles all edge cases)
//!
//! This is the production-ready approach used by real jj integrations.

use anyhow::{anyhow, Context, Result};
use journal::Checkpoint;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tl_core::Store;

use crate::mapping::JjMapping;
use crate::materialize::{format_commit_message, CommitMessageOptions, PublishOptions};

/// Materialize a checkpoint tree to a target directory
///
/// This recreates the exact file structure from the checkpoint in the given directory.
pub fn materialize_checkpoint_to_dir(
    checkpoint: &Checkpoint,
    store: &Store,
    target_dir: &Path,
) -> Result<()> {
    // Load the tree
    let tree = store.read_tree(checkpoint.root_tree)
        .context("Failed to read checkpoint tree")?;

    // Restore each file (pattern from restore.rs)
    for (path_bytes, entry) in tree.entries_with_paths() {
        let path_str = std::str::from_utf8(path_bytes)
            .context("Invalid UTF-8 in file path")?;

        // Skip protected directories
        if path_str.starts_with(".tl/") || path_str.starts_with(".git/") || path_str.starts_with(".jj/") {
            continue;
        }

        let file_path = target_dir.join(path_str);

        // Create parent directories
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Read blob content
        let content = store.blob_store().read_blob(entry.blob_hash)
            .with_context(|| format!("Failed to read blob for {}", path_str))?;

        // Write file
        fs::write(&file_path, content)
            .with_context(|| format!("Failed to write file: {}", file_path.display()))?;

        // Set permissions (Unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(entry.mode);
            fs::set_permissions(&file_path, permissions)
                .with_context(|| format!("Failed to set permissions: {}", file_path.display()))?;
        }
    }

    Ok(())
}

/// Copy directory recursively
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Publish a single checkpoint to JJ
///
/// This creates a JJ commit from the checkpoint using a temp directory approach:
/// 1. Materialize checkpoint tree to temp dir
/// 2. Copy .jj/ directory to temp dir (preserve JJ state)
/// 3. Run `jj commit` in temp dir
/// 4. Copy .jj/ back to persist the commit
/// 5. Store checkpoint ↔ commit mapping
pub fn publish_checkpoint(
    checkpoint: &Checkpoint,
    store: &Store,
    repo_root: &Path,
    mapping: &JjMapping,
    options: &PublishOptions,
) -> Result<String> {
    // Create temp directory on same filesystem (enables hardlinks)
    let temp_dir = tempfile::tempdir_in(repo_root)
        .context("Failed to create temporary directory")?;

    // Materialize checkpoint tree to temp dir
    materialize_checkpoint_to_dir(checkpoint, store, temp_dir.path())?;

    // Copy .jj/ directory to temp (preserve JJ workspace state)
    let jj_dir = repo_root.join(".jj");
    let temp_jj_dir = temp_dir.path().join(".jj");
    copy_dir_all(&jj_dir, &temp_jj_dir)
        .context("Failed to copy .jj directory")?;

    // Format commit message
    let commit_message = format_commit_message(checkpoint, &options.message_options);

    // Create JJ commit in temp directory
    let output = Command::new("jj")
        .current_dir(temp_dir.path())
        .args(&["commit", "-m", &commit_message])
        .output()
        .context("Failed to execute jj commit")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("JJ commit failed: {}", stderr);
    }

    // Get the commit ID
    let commit_id_output = Command::new("jj")
        .current_dir(temp_dir.path())
        .args(&["log", "--no-graph", "--limit", "1", "-T", "commit_id"])
        .output()
        .context("Failed to get JJ commit ID")?;

    let jj_commit_id = String::from_utf8(commit_id_output.stdout)?
        .trim()
        .to_string();

    // Copy .jj/ directory back to repo (persist commit)
    // Remove old .jj first
    fs::remove_dir_all(&jj_dir)
        .context("Failed to remove old .jj directory")?;
    copy_dir_all(&temp_jj_dir, &jj_dir)
        .context("Failed to copy .jj directory back")?;

    // Store mapping
    mapping.set(checkpoint.id, &jj_commit_id)
        .context("Failed to store checkpoint mapping")?;
    mapping.set_reverse(&jj_commit_id, checkpoint.id)
        .context("Failed to store reverse mapping")?;

    Ok(jj_commit_id)
}

/// Publish a range of checkpoints to JJ
///
/// Behavior depends on options.compact_range:
/// - If true: Create single JJ commit from last checkpoint (squash)
/// - If false: Create one JJ commit per checkpoint (preserve history)
pub fn publish_range(
    checkpoints: Vec<Checkpoint>,
    store: &Store,
    repo_root: &Path,
    mapping: &JjMapping,
    options: &PublishOptions,
) -> Result<Vec<String>> {
    if options.compact_range {
        // Compact mode: only publish the last checkpoint
        if let Some(last) = checkpoints.last() {
            let commit_id = publish_checkpoint(last, store, repo_root, mapping, options)?;
            Ok(vec![commit_id])
        } else {
            Ok(vec![])
        }
    } else {
        // Expand mode: publish each checkpoint
        let mut commit_ids = Vec::new();
        for checkpoint in checkpoints {
            let commit_id = publish_checkpoint(&checkpoint, store, repo_root, mapping, options)?;
            commit_ids.push(commit_id);
        }
        Ok(commit_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_materialize_creates_files() {
        // This would require a full integration test with a real store
        // Skipped for unit tests
    }
}
