//! JJ (Jujutsu) integration for Git interoperability
//!
//! This crate provides:
//! - Checkpoint → JJ commit materialization
//! - `tl publish` (create JJ commit from checkpoint)
//! - `tl push` / `tl pull` (Git interop via JJ)
//! - Checkpoint ↔ JJ commit mapping
//!
//! All operations are designed to be configurable via CLI flags to give users
//! maximum control over behavior.

pub mod git_ops;
pub mod mapping;
pub mod materialize;
pub mod publish;
pub mod workspace;

// Re-export public types
pub use mapping::JjMapping;
pub use materialize::{CommitMessageOptions, PublishOptions};
pub use publish::{publish_checkpoint, publish_range};
pub use workspace::{validate_workspace_name, JjWorkspace, WorkspaceManager, WorkspaceState};

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Errors specific to JJ integration
#[derive(Debug, thiserror::Error)]
pub enum JjError {
    #[error("JJ workspace not found. Run 'jj git init' first.")]
    WorkspaceNotFound,

    #[error("JJ workspace invalid: {0}")]
    InvalidWorkspace(String),

    #[error("Failed to create JJ commit: {0}")]
    CommitFailed(String),

    #[error("Failed to import JJ tree: {0}")]
    ImportFailed(String),

    #[error("Checkpoint not mapped to JJ commit")]
    NoMapping,

    #[error("JJ operation failed: {0}")]
    OperationFailed(String),
}

/// Detect if a JJ workspace exists at the repository root
///
/// Returns Some(PathBuf) if .jj/ directory exists and appears valid,
/// None otherwise.
pub fn detect_jj_workspace(repo_root: &Path) -> Result<Option<PathBuf>> {
    let jj_dir = repo_root.join(".jj");

    if !jj_dir.exists() || !jj_dir.is_dir() {
        return Ok(None);
    }

    // Validate it's a proper JJ workspace by checking for required subdirectories
    // JJ stores data in .jj/repo/ or .jj/store/ depending on version
    let has_repo = jj_dir.join("repo").exists();
    let has_store = jj_dir.join("store").exists();

    if has_repo || has_store {
        Ok(Some(jj_dir))
    } else {
        Ok(None)
    }
}

/// Load a JJ workspace from the repository root
///
/// This initializes the JJ workspace using jj-lib's APIs.
///
/// # Errors
///
/// Returns `JjError::WorkspaceNotFound` if no .jj/ directory exists.
/// Returns `JjError::InvalidWorkspace` if the workspace cannot be loaded.
pub fn load_workspace(repo_root: &Path) -> Result<jj_lib::workspace::Workspace> {
    use jj_lib::repo::StoreFactories;
    use jj_lib::local_working_copy::LocalWorkingCopy;
    use std::collections::HashMap;
    use std::sync::Arc;

    // First check if workspace exists
    detect_jj_workspace(repo_root)?
        .ok_or(JjError::WorkspaceNotFound)?;

    // Create default user settings from empty config
    let config = config::Config::builder().build()
        .map_err(|e| JjError::InvalidWorkspace(format!("Failed to create config: {}", e)))?;
    let user_settings = jj_lib::settings::UserSettings::from_config(config);

    // Create default store factories
    let store_factories = StoreFactories::default();

    // Register the local working copy factory (required for production)
    let mut working_copy_factories = HashMap::new();
    working_copy_factories.insert(
        "local".to_string(),
        Box::new(|store: &Arc<jj_lib::store::Store>, working_copy_path: &std::path::Path, state_path: &std::path::Path| {
            Box::new(LocalWorkingCopy::load(
                store.clone(),
                working_copy_path.to_path_buf(),
                state_path.to_path_buf(),
            )) as Box<dyn jj_lib::working_copy::WorkingCopy>
        }) as jj_lib::workspace::WorkingCopyFactory,
    );

    // Load the workspace
    let workspace = jj_lib::workspace::Workspace::load(
        &user_settings,
        repo_root,
        &store_factories,
        &working_copy_factories,
    )
    .map_err(|e| JjError::InvalidWorkspace(e.to_string()))?;

    Ok(workspace)
}

/// Initialize JJ with colocated git (creates both .git and .jj)
///
/// This function creates a new JJ workspace with a colocated Git repository,
/// where .git/ lives in the repository root alongside .jj/. This is the
/// recommended setup for new projects.
///
/// # Errors
///
/// Returns `JjError::OperationFailed` if initialization fails.
pub fn init_jj_colocated(repo_root: &Path) -> Result<()> {
    use jj_lib::workspace::Workspace;

    // Create default user settings
    let config = config::Config::builder().build()
        .map_err(|e| JjError::OperationFailed(format!("Failed to create config: {}", e)))?;
    let user_settings = jj_lib::settings::UserSettings::from_config(config);

    // Initialize colocated workspace
    Workspace::init_colocated_git(&user_settings, repo_root)
        .map_err(|e| JjError::OperationFailed(format!("Failed to initialize JJ workspace: {}", e)))?;

    Ok(())
}

/// Initialize JJ with existing git (creates .jj only, links to .git)
///
/// This function creates a JJ workspace that links to an existing Git repository.
/// The Git repository at `git_dir` is used as the backend store.
///
/// # Arguments
///
/// * `repo_root` - The repository root where .jj/ will be created
/// * `git_dir` - Path to the existing .git directory (usually repo_root/.git)
///
/// # Errors
///
/// Returns `JjError::OperationFailed` if initialization fails.
pub fn init_jj_external(repo_root: &Path, git_dir: &Path) -> Result<()> {
    use jj_lib::workspace::Workspace;

    // Create default user settings
    let config = config::Config::builder().build()
        .map_err(|e| JjError::OperationFailed(format!("Failed to create config: {}", e)))?;
    let user_settings = jj_lib::settings::UserSettings::from_config(config);

    // Initialize workspace with external git backend
    Workspace::init_external_git(&user_settings, repo_root, git_dir)
        .map_err(|e| JjError::OperationFailed(format!("Failed to initialize JJ with external git: {}", e)))?;

    Ok(())
}

/// Configure JJ bookmarks for optimal timelapse workflow
///
/// Sets up JJ configuration for:
/// - Bookmark prefix for timelapse snapshots (snap/)
/// - Default revset for log display
/// - Empty default commit description
///
/// # Errors
///
/// Returns `JjError::OperationFailed` if configuration fails.
/// Warnings are logged but don't cause failure.
pub fn configure_jj_bookmarks(repo_root: &Path) -> Result<()> {
    // Configure JJ settings via jj config command
    // These settings make the timelapse workflow smoother

    let configs = vec![
        ("revsets.log", "bookmarks() | @"),
        ("git.push-bookmark-prefix", "snap/"),
        ("ui.default-description", ""),
    ];

    for (key, value) in configs {
        let status = std::process::Command::new("jj")
            .args(&["config", "set", "--repo", key, value])
            .current_dir(repo_root)
            .status();

        match status {
            Ok(s) if s.success() => {
                // Successfully set config
            }
            Ok(_) | Err(_) => {
                // Failed to set config - this is optional, so we continue
            }
        }
    }

    Ok(())
}

/// Check if JJ binary is available in PATH
///
/// This is useful for operations that shell out to JJ CLI (like git fetch/push).
pub fn check_jj_binary() -> Result<bool> {
    let output = std::process::Command::new("which")
        .arg("jj")
        .output();

    match output {
        Ok(output) => Ok(output.status.success()),
        Err(_) => Ok(false),
    }
}
