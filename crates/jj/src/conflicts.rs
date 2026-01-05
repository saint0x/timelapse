//! Git-style conflict marker handling
//!
//! This module provides utilities for:
//! - Writing conflict markers to files
//! - Detecting files with conflict markers
//! - Parsing conflict markers

use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

/// Conflict marker strings (Git-compatible)
pub const CONFLICT_MARKER_START: &str = "<<<<<<<";
pub const CONFLICT_MARKER_BASE: &str = "|||||||";
pub const CONFLICT_MARKER_SEPARATOR: &str = "=======";
pub const CONFLICT_MARKER_END: &str = ">>>>>>>";

/// Write Git-style conflict markers to a file
///
/// This creates a file with 3-way merge conflict markers that are compatible
/// with Git, VS Code, IntelliJ, vim, and all standard merge tools.
///
/// # Format
/// ```text
/// <<<<<<< LOCAL (your changes)
/// local content here
/// ||||||| BASE (common ancestor)
/// original content here
/// =======
/// remote content here
/// >>>>>>> REMOTE (snap/main)
/// ```
///
/// # Arguments
/// * `file_path` - Path to write the conflicted file
/// * `base` - Base (common ancestor) content, if available
/// * `ours` - Local ("ours") content
/// * `theirs` - Remote ("theirs") content
/// * `ours_label` - Label for local side (e.g., "LOCAL" or "HEAD")
/// * `theirs_label` - Label for remote side (e.g., "REMOTE" or "snap/main")
pub fn write_conflict_markers(
    file_path: &Path,
    base: Option<&[u8]>,
    ours: &[u8],
    theirs: &[u8],
    ours_label: &str,
    theirs_label: &str,
) -> Result<()> {
    let mut output = Vec::new();

    // Convert to strings for line-by-line processing
    let ours_str = String::from_utf8_lossy(ours);
    let theirs_str = String::from_utf8_lossy(theirs);

    // Write conflict start marker
    writeln!(output, "{} {}", CONFLICT_MARKER_START, ours_label)?;

    // Write "ours" content
    output.extend_from_slice(ours_str.as_bytes());
    if !ours_str.ends_with('\n') && !ours_str.is_empty() {
        output.push(b'\n');
    }

    // Optionally write base content (diff3 style)
    if let Some(base_content) = base {
        let base_str = String::from_utf8_lossy(base_content);
        writeln!(output, "{} BASE", CONFLICT_MARKER_BASE)?;
        output.extend_from_slice(base_str.as_bytes());
        if !base_str.ends_with('\n') && !base_str.is_empty() {
            output.push(b'\n');
        }
    }

    // Write separator
    writeln!(output, "{}", CONFLICT_MARKER_SEPARATOR)?;

    // Write "theirs" content
    output.extend_from_slice(theirs_str.as_bytes());
    if !theirs_str.ends_with('\n') && !theirs_str.is_empty() {
        output.push(b'\n');
    }

    // Write conflict end marker
    writeln!(output, "{} {}", CONFLICT_MARKER_END, theirs_label)?;

    // Create parent directories if needed
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create parent directories")?;
    }

    // Write file
    std::fs::write(file_path, output)
        .context("Failed to write conflict file")?;

    Ok(())
}

/// Write a file with conflict markers using smart merging
///
/// This function attempts to merge files line-by-line and only writes
/// conflict markers for regions that actually differ.
pub fn write_smart_conflict_markers(
    file_path: &Path,
    base: Option<&[u8]>,
    ours: &[u8],
    theirs: &[u8],
    ours_label: &str,
    theirs_label: &str,
) -> Result<usize> {
    // For now, use simple conflict markers for the entire file
    // A more sophisticated implementation would use diff algorithms
    // to identify conflicting regions

    write_conflict_markers(file_path, base, ours, theirs, ours_label, theirs_label)?;

    // Return number of conflicts (1 for simple case)
    Ok(1)
}

/// Check if a file contains conflict markers
///
/// Returns true if the file contains Git-style conflict markers.
pub fn has_conflict_markers(file_path: &Path) -> Result<bool> {
    if !file_path.exists() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(file_path)
        .context("Failed to read file")?;

    Ok(content.contains(CONFLICT_MARKER_START) && content.contains(CONFLICT_MARKER_END))
}

/// Count the number of conflict regions in a file
///
/// Returns the number of `<<<<<<<` markers found.
pub fn count_conflicts(file_path: &Path) -> Result<usize> {
    if !file_path.exists() {
        return Ok(0);
    }

    let content = std::fs::read_to_string(file_path)
        .context("Failed to read file")?;

    let count = content.matches(CONFLICT_MARKER_START).count();
    Ok(count)
}

/// Parse conflict markers from a file
///
/// Returns a list of conflict regions with their line numbers and content.
#[derive(Debug, Clone)]
pub struct ConflictRegion {
    /// Start line number (1-indexed)
    pub start_line: usize,
    /// End line number (1-indexed)
    pub end_line: usize,
    /// "Ours" (local) content
    pub ours: String,
    /// Base content (if present)
    pub base: Option<String>,
    /// "Theirs" (remote) content
    pub theirs: String,
}

/// Parse conflict regions from file content
pub fn parse_conflict_regions(content: &str) -> Vec<ConflictRegion> {
    let mut regions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        if lines[i].starts_with(CONFLICT_MARKER_START) {
            let start_line = i + 1; // 1-indexed
            let mut ours = String::new();
            let mut base = None;
            let mut theirs = String::new();
            let mut current_section = "ours";

            i += 1;
            while i < lines.len() {
                let line = lines[i];

                if line.starts_with(CONFLICT_MARKER_BASE) {
                    current_section = "base";
                    base = Some(String::new());
                } else if line.starts_with(CONFLICT_MARKER_SEPARATOR) {
                    current_section = "theirs";
                } else if line.starts_with(CONFLICT_MARKER_END) {
                    let end_line = i + 1; // 1-indexed
                    regions.push(ConflictRegion {
                        start_line,
                        end_line,
                        ours,
                        base,
                        theirs,
                    });
                    break;
                } else {
                    match current_section {
                        "ours" => {
                            if !ours.is_empty() {
                                ours.push('\n');
                            }
                            ours.push_str(line);
                        }
                        "base" => {
                            if let Some(ref mut b) = base {
                                if !b.is_empty() {
                                    b.push('\n');
                                }
                                b.push_str(line);
                            }
                        }
                        "theirs" => {
                            if !theirs.is_empty() {
                                theirs.push('\n');
                            }
                            theirs.push_str(line);
                        }
                        _ => {}
                    }
                }
                i += 1;
            }
        }
        i += 1;
    }

    regions
}

/// Check if all conflicts in a file have been resolved
///
/// A file is considered resolved if it no longer contains conflict markers.
pub fn is_resolved(file_path: &Path) -> Result<bool> {
    Ok(!has_conflict_markers(file_path)?)
}

/// Resolution status for conflict checking
#[derive(Debug, Clone, PartialEq)]
pub enum ResolutionStatus {
    /// File has no conflicts
    Clean,
    /// File has unresolved conflicts
    Conflicted,
    /// File was resolved (markers removed)
    Resolved,
    /// File not found
    Missing,
}

/// Check the resolution status of a file
pub fn check_resolution_status(file_path: &Path, was_conflicted: bool) -> Result<ResolutionStatus> {
    if !file_path.exists() {
        return Ok(ResolutionStatus::Missing);
    }

    if has_conflict_markers(file_path)? {
        return Ok(ResolutionStatus::Conflicted);
    }

    if was_conflicted {
        Ok(ResolutionStatus::Resolved)
    } else {
        Ok(ResolutionStatus::Clean)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_conflict_markers() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        write_conflict_markers(
            &file_path,
            Some(b"base content"),
            b"local content",
            b"remote content",
            "LOCAL",
            "REMOTE",
        ).unwrap();

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("<<<<<<< LOCAL"));
        assert!(content.contains("local content"));
        assert!(content.contains("||||||| BASE"));
        assert!(content.contains("base content"));
        assert!(content.contains("======="));
        assert!(content.contains("remote content"));
        assert!(content.contains(">>>>>>> REMOTE"));
    }

    #[test]
    fn test_has_conflict_markers() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Write file without conflicts
        std::fs::write(&file_path, "normal content").unwrap();
        assert!(!has_conflict_markers(&file_path).unwrap());

        // Write file with conflicts
        std::fs::write(&file_path, "<<<<<<< LOCAL\ncontent\n=======\nother\n>>>>>>> REMOTE").unwrap();
        assert!(has_conflict_markers(&file_path).unwrap());
    }

    #[test]
    fn test_parse_conflict_regions() {
        let content = r#"some code
<<<<<<< LOCAL
local version
||||||| BASE
original version
=======
remote version
>>>>>>> REMOTE
more code
"#;

        let regions = parse_conflict_regions(content);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].ours, "local version");
        assert_eq!(regions[0].base.as_ref().unwrap(), "original version");
        assert_eq!(regions[0].theirs, "remote version");
    }

    #[test]
    fn test_count_conflicts() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let content = r#"<<<<<<< A
=======
>>>>>>> B
some code
<<<<<<< A
=======
>>>>>>> B
"#;
        std::fs::write(&file_path, content).unwrap();

        assert_eq!(count_conflicts(&file_path).unwrap(), 2);
    }
}
