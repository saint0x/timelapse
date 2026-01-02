//! Incremental tree update algorithm
//!
//! The performance linchpin: update tree from dirty paths without full rescan

use seer_core::{Blake3Hash, Tree, Entry};
use crate::PathMap;
use std::path::Path;

/// Update a tree incrementally from a set of dirty paths
///
/// This is the core algorithm that enables < 10ms checkpoint creation
pub fn incremental_update(
    base_map: &PathMap,
    dirty_paths: Vec<&Path>,
    repo_root: &Path,
) -> anyhow::Result<(Tree, Blake3Hash)> {
    // TODO: Implement incremental tree update
    // Step A: Coalesce and normalize dirty paths
    // Step B: Reconcile each candidate path
    //   - Case 1: file exists -> hash and update entry
    //   - Case 2: path doesn't exist -> remove entry
    // Step C: Build new tree from updated map
    // Step D: Compute new tree hash
    todo!("Implement incremental_update")
}

/// Normalize and deduplicate dirty paths
fn normalize_dirty_paths(paths: Vec<&Path>, repo_root: &Path) -> Vec<std::path::PathBuf> {
    // TODO: Implement path normalization
    // - Convert to repo-relative
    // - Drop .snap/ and .git/
    // - Deduplicate
    todo!("Implement normalize_dirty_paths")
}
