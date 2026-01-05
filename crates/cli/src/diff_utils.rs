//! Utilities for generating line-by-line diffs

use owo_colors::OwoColorize;
use similar::{ChangeTag, TextDiff};

/// Check if content is binary (contains null bytes in first 8KB)
pub fn is_binary(content: &[u8]) -> bool {
    content.iter().take(8192).any(|&b| b == 0)
}

/// Generate a unified diff with colored output
///
/// Returns a formatted string with colored diff hunks showing additions (+) and deletions (-)
pub fn generate_unified_diff(
    old_content: &[u8],
    new_content: &[u8],
    path: &str,
    context_lines: usize,
) -> String {
    // Convert bytes to UTF-8 (with replacement chars for invalid UTF-8)
    let old_text = String::from_utf8_lossy(old_content);
    let new_text = String::from_utf8_lossy(new_content);

    // Create text diff with unified output
    let diff = TextDiff::from_lines(&old_text, &new_text);

    let mut output = String::new();

    // Iterate through hunks
    for (hunk_idx, hunk) in diff.unified_diff().context_radius(context_lines).iter_hunks().enumerate() {
        if hunk_idx > 0 {
            output.push('\n');
        }

        // Hunk header (e.g., @@ -12,7 +12,8 @@)
        let header = format!("{}", hunk.header());
        output.push_str(&format!("    {}\n", header.cyan()));

        // Iterate through changes in the hunk
        for change in hunk.iter_changes() {
            let line: &str = change.value();

            match change.tag() {
                ChangeTag::Delete => {
                    // Red for deletions
                    output.push_str(&format!("    {}", format!("-{}", line).red()));
                }
                ChangeTag::Insert => {
                    // Green for additions
                    output.push_str(&format!("    {}", format!("+{}", line).green()));
                }
                ChangeTag::Equal => {
                    // Dimmed for context
                    output.push_str(&format!("    {}", format!(" {}", line).dimmed()));
                }
            }

            // Add newline if the line doesn't end with one
            if !line.ends_with('\n') {
                output.push('\n');
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_binary() {
        assert!(!is_binary(b"Hello, world!"));
        assert!(!is_binary(b"Line 1\nLine 2\nLine 3"));
        assert!(is_binary(b"Hello\x00world"));
        assert!(is_binary(&[0u8; 100]));
    }

    #[test]
    fn test_generate_unified_diff_simple() {
        let old = b"line 1\nline 2\nline 3\n";
        let new = b"line 1\nline 2 modified\nline 3\n";

        let diff = generate_unified_diff(old, new, "test.txt", 1);

        // Should contain the modified line
        assert!(diff.contains("line 2"));
        assert!(diff.contains("line 2 modified"));
    }

    #[test]
    fn test_generate_unified_diff_addition() {
        let old = b"line 1\nline 2\n";
        let new = b"line 1\nline 1.5\nline 2\n";

        let diff = generate_unified_diff(old, new, "test.txt", 1);

        assert!(diff.contains("line 1.5"));
    }
}
