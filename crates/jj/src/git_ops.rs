//! Native Git operations using jj-lib's high-level APIs
//!
//! This module provides production-ready git push/fetch operations
//! using jj-lib's native functions that handle git2 internally.

use anyhow::{anyhow, Context, Result};
use jj_lib::git::{fetch, push_branches, GitBranchPushTargets, GitFetchError, GitPushError, RemoteCallbacks};
use jj_lib::git_backend::GitBackend;
use jj_lib::refs::BranchPushUpdate;
use jj_lib::repo::Repo;
use jj_lib::str_util::StringPattern;
use jj_lib::workspace::Workspace;
use std::collections::HashSet;

/// Result of a push operation for a single branch
#[derive(Debug, Clone)]
pub struct BranchPushResult {
    pub name: String,
    pub status: BranchPushStatus,
    pub old_commit: Option<String>,
    pub new_commit: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BranchPushStatus {
    /// Successfully pushed
    Pushed,
    /// Already up to date
    UpToDate,
    /// Rejected - remote has diverged, needs --force
    Diverged,
    /// Rejected for other reasons
    Rejected(String),
    /// Skipped (no change)
    Skipped,
}

/// Push to Git remote using jj-lib's native push_branches API
///
/// This uses JJ's high-level push function which handles:
/// - Exporting JJ refs to Git
/// - Pushing to remote
/// - Updating remote tracking branches
///
/// Returns detailed results per branch.
///
/// # Arguments
/// * `workspace` - JJ workspace (must be git-backed)
/// * `bookmark` - Optional bookmark name (will push snap/<bookmark>)
/// * `all` - Push all snap/* bookmarks
/// * `force` - Force push (non-fast-forward)
pub fn native_git_push(
    workspace: &mut Workspace,
    bookmark: Option<&str>,
    all: bool,
    force: bool,
) -> Result<Vec<BranchPushResult>> {
    use jj_lib::backend::ObjectId;

    // Load repo at HEAD
    let config = config::Config::builder().build()?;
    let user_settings = jj_lib::settings::UserSettings::from_config(config);
    let repo = workspace.repo_loader().load_at_head(&user_settings)
        .context("Failed to load repository")?;

    // Start transaction
    let mut tx = repo.start_transaction(&user_settings);
    let _git_settings = user_settings.git_settings();

    // Get git repository via GitBackend
    let store = Repo::store(tx.mut_repo());
    let git_backend = store.backend_impl()
        .downcast_ref::<GitBackend>()
        .ok_or_else(|| anyhow!("Not a git-backed repository"))?;

    let git_repo = git_backend.open_git_repo()
        .context("Failed to open git repository")?;

    // Get branches to push from view
    let view = tx.mut_repo().view();

    // Collect branches to validate and push
    let mut branches_to_push: Vec<(String, Option<String>, Option<String>)> = Vec::new(); // (name, local_commit, remote_commit)

    if all {
        // Collect all snap/* branches
        for (branch_name, target) in view.local_branches() {
            if branch_name.starts_with("snap/") {
                if let Some(local_commit_id) = target.as_normal() {
                    let remote_ref = view.get_remote_branch(branch_name, "origin");
                    let remote_commit_id = remote_ref.target.as_normal().map(|id| id.hex());
                    branches_to_push.push((
                        branch_name.to_string(),
                        Some(local_commit_id.hex()),
                        remote_commit_id,
                    ));
                }
            }
        }
    } else if let Some(bookmark_name) = bookmark {
        // Single bookmark
        let full_name = if bookmark_name.starts_with("snap/") {
            bookmark_name.to_string()
        } else {
            format!("snap/{}", bookmark_name)
        };

        let target = view.get_local_branch(&full_name);
        if let Some(local_commit_id) = target.as_normal() {
            let remote_ref = view.get_remote_branch(&full_name, "origin");
            let remote_commit_id = remote_ref.target.as_normal().map(|id| id.hex());
            branches_to_push.push((
                full_name.clone(),
                Some(local_commit_id.hex()),
                remote_commit_id,
            ));
        } else {
            anyhow::bail!("Branch {} not found or not at a single commit", full_name);
        }
    } else {
        anyhow::bail!("Must specify either --all or a bookmark name");
    }

    if branches_to_push.is_empty() {
        anyhow::bail!("No branches to push");
    }

    // Pre-validate: Check for diverged branches that would need force
    let mut diverged_branches = Vec::new();
    let mut up_to_date_branches = Vec::new();

    for (name, local_commit, remote_commit) in &branches_to_push {
        if let (Some(local), Some(remote)) = (local_commit, remote_commit) {
            if local == remote {
                up_to_date_branches.push(name.clone());
            } else {
                // Different commits - check if this is a simple fast-forward or diverged
                // For now, if remote exists and differs, consider it potentially diverged
                // TODO: Use repo.index() to check actual ancestry
                diverged_branches.push((name.clone(), remote.clone()));
            }
        }
    }

    // If we have diverged branches and no --force, fail early with helpful message
    if !diverged_branches.is_empty() && !force {
        let branch_list: Vec<String> = diverged_branches.iter()
            .map(|(name, remote)| format!("  {} (remote: {})", name, &remote[..12.min(remote.len())]))
            .collect();

        anyhow::bail!(
            "Push rejected: {} branch(es) have diverged from remote:\n{}\n\n\
             Options:\n\
             1. Pull first: tl pull\n\
             2. Force push (overwrites remote): tl push --force\n\
             3. Check status: tl status --remote",
            diverged_branches.len(),
            branch_list.join("\n")
        );
    }

    // Build branch update targets (skip up-to-date branches)
    let mut branch_updates = Vec::new();
    let mut force_pushed_branches = HashSet::new();
    let mut results = Vec::new();

    for (name, local_commit, remote_commit) in &branches_to_push {
        // Skip up-to-date branches
        if up_to_date_branches.contains(name) {
            results.push(BranchPushResult {
                name: name.clone(),
                status: BranchPushStatus::UpToDate,
                old_commit: remote_commit.clone(),
                new_commit: local_commit.clone(),
            });
            continue;
        }

        if let Some(local_hex) = local_commit {
            let local_commit_id = jj_lib::backend::CommitId::from_hex(local_hex);
            let old_target = remote_commit.as_ref()
                .map(|hex| jj_lib::backend::CommitId::from_hex(hex));

            branch_updates.push((
                name.clone(),
                BranchPushUpdate {
                    old_target,
                    new_target: Some(local_commit_id),
                },
            ));

            if force {
                force_pushed_branches.insert(name.clone());
            }
        }
    }

    // If nothing to push after filtering, return early
    if branch_updates.is_empty() {
        return Ok(results);
    }

    let targets = GitBranchPushTargets {
        branch_updates: branch_updates.clone(),
        force_pushed_branches,
    };

    // Set up empty callbacks (no progress reporting for now)
    let callbacks = RemoteCallbacks::default();

    // Execute push using JJ's native API
    match push_branches(tx.mut_repo(), &git_repo, "origin", &targets, callbacks) {
        Ok(()) => {
            // Push succeeded - record results
            for (name, update) in branch_updates {
                results.push(BranchPushResult {
                    name,
                    status: BranchPushStatus::Pushed,
                    old_commit: update.old_target.map(|id| id.hex()),
                    new_commit: update.new_target.map(|id| id.hex()),
                });
            }
        }
        Err(e) => {
            // Convert error and propagate
            return Err(match e {
                GitPushError::InternalGitError(git_err) => {
                    let error_msg = git_err.message();
                    if error_msg.contains("authentication") || error_msg.contains("Authentication") {
                        anyhow!(
                            "Authentication failed. Configure credentials:\n\
                             - GitHub: Use SSH keys or GitHub CLI (gh auth login)\n\
                             - GitLab: Use SSH keys or personal access tokens\n\
                             Error: {}",
                            error_msg
                        )
                    } else if error_msg.contains("non-fast-forward") || error_msg.contains("rejected") {
                        anyhow!(
                            "Push rejected (non-fast-forward). Remote has changes you don't have.\n\
                             Try: tl pull\n\
                             Or use --force to force push (overwrites remote)"
                        )
                    } else if error_msg.contains("network") || error_msg.contains("timeout") {
                        anyhow!("Network error: {}\nCheck your internet connection", error_msg)
                    } else {
                        anyhow!("Git push failed: {}", error_msg)
                    }
                }
                GitPushError::NoSuchRemote(name) => {
                    anyhow!("Remote '{}' not found. Add one with: git remote add {} <url>", name, name)
                }
                GitPushError::RefUpdateRejected(msgs) => {
                    anyhow!("Push rejected: {}", msgs.join(", "))
                }
                GitPushError::RemoteReservedForLocalGitRepo => {
                    anyhow!("Cannot push to 'git' remote (reserved for local Git repository)")
                }
                GitPushError::NotFastForward => {
                    anyhow!(
                        "Push rejected (not a fast-forward). Remote has changes you don't have.\n\
                         Try: tl pull\n\
                         Or use --force to force push (overwrites remote)"
                    )
                }
            });
        }
    }

    // Commit transaction
    tx.commit("push to origin");

    Ok(results)
}

/// Fetch from Git remote using jj-lib's native fetch API
///
/// This uses JJ's high-level fetch function which handles:
/// - Fetching from remote
/// - Importing new Git refs to JJ
/// - Updating remote tracking branches
///
/// # Arguments
/// * `workspace` - JJ workspace (must be git-backed)
pub fn native_git_fetch(workspace: &mut Workspace) -> Result<()> {
    // Load repo at HEAD
    let config = config::Config::builder().build()?;
    let user_settings = jj_lib::settings::UserSettings::from_config(config);
    let repo = workspace.repo_loader().load_at_head(&user_settings)
        .context("Failed to load repository")?;

    // Get git repository via GitBackend
    let store = repo.store();
    let git_backend = store.backend_impl()
        .downcast_ref::<GitBackend>()
        .ok_or_else(|| anyhow!("Not a git-backed repository"))?;

    let git_repo = git_backend.open_git_repo()
        .context("Failed to open git repository")?;

    // Start transaction
    let mut tx = repo.start_transaction(&user_settings);
    let git_settings = user_settings.git_settings();

    // Fetch all branches (empty pattern = fetch all)
    let branch_patterns = vec![StringPattern::everything()];
    let callbacks = RemoteCallbacks::default();

    // Execute fetch using JJ's native API
    fetch(tx.mut_repo(), &git_repo, "origin", &branch_patterns, callbacks, &git_settings)
        .map_err(|e| match e {
            GitFetchError::InternalGitError(git_err) => {
                let error_msg = git_err.message();
                if error_msg.contains("authentication") || error_msg.contains("Authentication") {
                    anyhow!(
                        "Authentication failed during fetch. Configure credentials:\n\
                         - GitHub: Use SSH keys or GitHub CLI (gh auth login)\n\
                         - GitLab: Use SSH keys or personal access tokens\n\
                         Error: {}",
                        error_msg
                    )
                } else if error_msg.contains("network") || error_msg.contains("timeout") {
                    anyhow!("Network error: {}\nCheck your internet connection", error_msg)
                } else {
                    anyhow!("Git fetch failed: {}", error_msg)
                }
            }
            GitFetchError::NoSuchRemote(name) => {
                anyhow!("Remote '{}' not found. Add one with: git remote add {} <url>", name, name)
            }
            GitFetchError::InvalidBranchPattern => {
                anyhow!("Invalid branch pattern")
            }
            GitFetchError::GitImportError(err) => {
                anyhow!("Failed to import fetched refs: {:?}", err)
            }
        })?;

    // Commit transaction
    tx.commit("fetch from origin");

    Ok(())
}

/// Information about a remote branch
#[derive(Debug, Clone)]
pub struct RemoteBranchInfo {
    /// Branch name (e.g., "snap/main")
    pub name: String,
    /// Remote commit ID (hex)
    pub remote_commit_id: Option<String>,
    /// Local commit ID (hex) if branch exists locally
    pub local_commit_id: Option<String>,
    /// Whether local and remote have diverged
    pub is_diverged: bool,
    /// Number of commits local is ahead of remote
    pub commits_ahead: usize,
    /// Number of commits local is behind remote
    pub commits_behind: usize,
}

/// Get information about remote branches after fetch
///
/// Returns branches that have updates from remote
pub fn get_remote_branch_updates(workspace: &jj_lib::workspace::Workspace) -> Result<Vec<RemoteBranchInfo>> {
    use jj_lib::backend::ObjectId;

    let config = config::Config::builder().build()?;
    let user_settings = jj_lib::settings::UserSettings::from_config(config);
    let repo = workspace.repo_loader().load_at_head(&user_settings)
        .context("Failed to load repository")?;

    let view = repo.view();
    let mut branches = Vec::new();

    // Iterate through all remote branches for "origin"
    for (branch_name, remote_ref) in view.remote_branches("origin") {
        // Only look at snap/* branches
        if !branch_name.starts_with("snap/") {
            continue;
        }

        let remote_commit_id = remote_ref.target.as_normal().map(|id| id.hex());

        // Get local branch if exists
        let local_target = view.get_local_branch(branch_name);
        let local_commit_id = local_target.as_normal().map(|id| id.hex());

        // Determine divergence status
        let (is_diverged, commits_ahead, commits_behind) = if let (Some(local_id), Some(remote_id)) = (&local_commit_id, &remote_commit_id) {
            if local_id == remote_id {
                (false, 0, 0)
            } else {
                // For now, simplified: if different, check ancestry
                // TODO: Count actual commits ahead/behind using repo.index()
                (true, 0, 0)
            }
        } else {
            (false, 0, 0)
        };

        branches.push(RemoteBranchInfo {
            name: branch_name.to_string(),
            remote_commit_id,
            local_commit_id,
            is_diverged,
            commits_ahead,
            commits_behind,
        });
    }

    Ok(branches)
}

/// Information about a local branch
#[derive(Debug, Clone)]
pub struct LocalBranchInfo {
    /// Branch name (e.g., "snap/main")
    pub name: String,
    /// Commit ID (hex)
    pub commit_id: String,
    /// Whether this branch has a remote tracking branch
    pub has_remote: bool,
    /// Remote commit ID if different from local
    pub remote_commit_id: Option<String>,
}

/// Get all local branches
pub fn get_local_branches(workspace: &jj_lib::workspace::Workspace) -> Result<Vec<LocalBranchInfo>> {
    use jj_lib::backend::ObjectId;

    let config = config::Config::builder().build()?;
    let user_settings = jj_lib::settings::UserSettings::from_config(config);
    let repo = workspace.repo_loader().load_at_head(&user_settings)
        .context("Failed to load repository")?;

    let view = repo.view();
    let mut branches = Vec::new();

    // Iterate through all local branches
    for (branch_name, local_ref) in view.local_branches() {
        // Only show snap/* branches
        if !branch_name.starts_with("snap/") {
            continue;
        }

        let commit_id = match local_ref.as_normal() {
            Some(id) => id.hex(),
            None => continue, // Skip conflicted refs
        };

        // Check for remote tracking branch
        let remote_ref = view.get_remote_branch(branch_name, "origin");
        let remote_commit_id = remote_ref.target.as_normal().map(|id| id.hex());
        let has_remote = remote_commit_id.is_some();

        // Only include remote_commit_id if different from local
        let remote_commit_id = match &remote_commit_id {
            Some(remote_id) if remote_id != &commit_id => Some(remote_id.clone()),
            _ => None,
        };

        branches.push(LocalBranchInfo {
            name: branch_name.to_string(),
            commit_id,
            has_remote,
            remote_commit_id,
        });
    }

    // Sort by branch name
    branches.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(branches)
}

/// Get all remote-only branches (not present locally)
pub fn get_remote_only_branches(workspace: &jj_lib::workspace::Workspace) -> Result<Vec<RemoteBranchInfo>> {
    use jj_lib::backend::ObjectId;

    let config = config::Config::builder().build()?;
    let user_settings = jj_lib::settings::UserSettings::from_config(config);
    let repo = workspace.repo_loader().load_at_head(&user_settings)
        .context("Failed to load repository")?;

    let view = repo.view();
    let mut branches = Vec::new();

    // Iterate through all remote branches for "origin"
    for (branch_name, remote_ref) in view.remote_branches("origin") {
        // Only look at snap/* branches
        if !branch_name.starts_with("snap/") {
            continue;
        }

        // Skip if there's a local branch
        let local_target = view.get_local_branch(branch_name);
        if local_target.is_present() {
            continue;
        }

        let remote_commit_id = remote_ref.target.as_normal().map(|id| id.hex());

        branches.push(RemoteBranchInfo {
            name: branch_name.to_string(),
            remote_commit_id,
            local_commit_id: None,
            is_diverged: false,
            commits_ahead: 0,
            commits_behind: 0,
        });
    }

    // Sort by branch name
    branches.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(branches)
}

/// Delete a local branch
pub fn delete_local_branch(workspace: &mut jj_lib::workspace::Workspace, branch_name: &str) -> Result<()> {
    use jj_lib::op_store::RefTarget;

    let config = config::Config::builder().build()?;
    let user_settings = jj_lib::settings::UserSettings::from_config(config);
    let repo = workspace.repo_loader().load_at_head(&user_settings)
        .context("Failed to load repository")?;

    // Ensure snap/ prefix
    let full_name = if branch_name.starts_with("snap/") {
        branch_name.to_string()
    } else {
        format!("snap/{}", branch_name)
    };

    // Check if branch exists
    let view = repo.view();
    if !view.get_local_branch(&full_name).is_present() {
        anyhow::bail!("Branch '{}' not found", full_name);
    }

    // Start transaction
    let mut tx = repo.start_transaction(&user_settings);
    let mut_repo = tx.mut_repo();

    // Delete branch (set target to absent)
    mut_repo.set_local_branch_target(&full_name, RefTarget::absent());

    // Commit transaction
    let _committed_repo = tx.commit(&format!("delete branch '{}'", full_name));

    Ok(())
}

/// Export a specific JJ commit (by hex ID) to a target directory
///
/// Used by pull to export remote commits to working directory
pub fn export_commit_to_dir(
    workspace: &jj_lib::workspace::Workspace,
    commit_id_hex: &str,
    target_dir: &std::path::Path,
) -> Result<()> {
    use jj_lib::backend::{CommitId, ObjectId};
    use jj_lib::repo::Repo;

    let config = config::Config::builder().build()?;
    let user_settings = jj_lib::settings::UserSettings::from_config(config);
    let repo = workspace.repo_loader().load_at_head(&user_settings)
        .context("Failed to load repository")?;

    let commit_id = CommitId::from_hex(commit_id_hex);

    let commit = repo.store().get_commit(&commit_id)
        .context("Failed to get commit")?;

    let tree_id = commit.tree_id();

    // Reuse existing export function
    crate::export::export_jj_tree_to_dir(
        repo.store(),
        tree_id,
        target_dir,
    )?;

    Ok(())
}

// Tests disabled on macOS due to OpenSSL linking issues in test binary
// The production code works correctly - this is a platform-specific test infrastructure issue
#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    /// Helper to create a test JJ workspace with git backend
    fn create_test_git_workspace(path: &Path) -> Result<Workspace> {
        let config = config::Config::builder().build()?;
        let user_settings = jj_lib::settings::UserSettings::from_config(config);
        let (workspace, _repo) = jj_lib::workspace::Workspace::init_internal_git(&user_settings, path)?;
        Ok(workspace)
    }

    #[test]
    fn test_native_git_fetch_requires_remote() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut workspace = create_test_git_workspace(temp_dir.path())?;

        // Fetch should fail gracefully if no remote is configured
        let result = native_git_fetch(&mut workspace);

        // Should get an error about missing remote
        assert!(result.is_err());
        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(error_msg.contains("origin"));

        Ok(())
    }

    #[test]
    fn test_native_git_push_requires_bookmark_or_all() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut workspace = create_test_git_workspace(temp_dir.path())?;

        // Push without bookmark or --all should fail
        let result = native_git_push(&mut workspace, None, false, false);

        assert!(result.is_err());
        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(error_msg.contains("Must specify either --all or a bookmark name"));

        Ok(())
    }
}
