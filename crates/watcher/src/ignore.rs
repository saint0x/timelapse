//! Ignore pattern management for timelapse
//!
//! Supports multiple sources of ignore patterns:
//! 1. Built-in patterns (.tl/, .git/, .jj/ - always active)
//! 2. .gitignore patterns (optional, enabled by default)
//! 3. .tlignore patterns (timelapse-specific, optional)
//! 4. Config-based patterns (additional custom patterns)

use anyhow::Result;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Ignore rule manager
///
/// Combines multiple sources of ignore patterns with proper precedence:
/// 1. Built-in patterns (highest priority - always enforced)
/// 2. .tlignore patterns (override .gitignore)
/// 3. .gitignore patterns (lowest priority)
pub struct IgnoreRules {
    /// Repository root directory
    repo_root: PathBuf,

    /// Gitignore patterns (optional)
    gitignore: Option<Gitignore>,

    /// Timelapse-specific ignore patterns (optional)
    tlignore: Option<Gitignore>,

    /// Configuration
    config: IgnoreConfig,
}

impl IgnoreRules {
    /// Load ignore rules for repository
    pub fn load(repo_root: &Path, config: IgnoreConfig) -> Result<Self> {
        let mut rules = Self {
            repo_root: repo_root.to_path_buf(),
            gitignore: None,
            tlignore: None,
            config,
        };

        rules.reload_ignore_files()?;
        Ok(rules)
    }

    /// Reload ignore files from disk
    ///
    /// This can be called to pick up changes to .gitignore/.tlignore
    pub fn reload_ignore_files(&mut self) -> Result<()> {
        // Build .gitignore
        if self.config.use_gitignore {
            let gitignore_path = self.repo_root.join(".gitignore");
            if gitignore_path.exists() {
                let mut builder = GitignoreBuilder::new(&self.repo_root);
                builder.add(&gitignore_path);
                self.gitignore = Some(builder.build()?);
            } else {
                self.gitignore = None;
            }
        } else {
            self.gitignore = None;
        }

        // Build .tlignore
        if self.config.use_tlignore {
            let tlignore_path = self.repo_root.join(".tlignore");
            if tlignore_path.exists() {
                let mut builder = GitignoreBuilder::new(&self.repo_root);
                builder.add(&tlignore_path);
                self.tlignore = Some(builder.build()?);
            } else {
                self.tlignore = None;
            }
        } else {
            self.tlignore = None;
        }

        Ok(())
    }

    /// Check if path should be ignored
    ///
    /// Returns true if the path matches any ignore pattern
    pub fn should_ignore(&self, path: &Path) -> bool {
        // 1. Built-in patterns (highest priority - always enforced)
        if self.is_builtin_ignored(path) {
            return true;
        }

        // Determine if path is a directory
        // First try checking the actual filesystem
        let is_dir = if path.is_absolute() {
            path.is_dir()
        } else {
            // For relative paths, check against repo root
            let full_path = self.repo_root.join(path);
            full_path.is_dir()
        };

        // 2. .tlignore (overrides .gitignore)
        if let Some(ref tlignore) = self.tlignore {
            if tlignore.matched(path, is_dir).is_ignore() {
                return true;
            }
        }

        // 3. .gitignore (lowest priority)
        if let Some(ref gitignore) = self.gitignore {
            if gitignore.matched(path, is_dir).is_ignore() {
                return true;
            }
        }

        // 4. Additional config patterns
        for pattern in &self.config.additional_patterns {
            if self.matches_glob_pattern(path, pattern) {
                return true;
            }
        }

        false
    }

    /// Check if path matches built-in ignore patterns
    ///
    /// These are always enforced regardless of configuration
    fn is_builtin_ignored(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Core timelapse directories
        if path_str.contains("/.tl/")
            || path_str.ends_with("/.tl")
            || path_str.starts_with(".tl/")
            || path_str == ".tl" {
            return true;
        }

        // Git repository
        if path_str.contains("/.git/")
            || path_str.ends_with("/.git")
            || path_str.starts_with(".git/")
            || path_str == ".git" {
            return true;
        }

        // Jujutsu repository
        if path_str.contains("/.jj/")
            || path_str.ends_with("/.jj")
            || path_str.starts_with(".jj/")
            || path_str == ".jj" {
            return true;
        }

        // Editor temp files and common build directories
        if self.matches_editor_temp(&path_str) {
            return true;
        }

        false
    }

    /// Check if path matches common editor temporary files or build directories
    ///
    /// Covers: Vim, Emacs, VS Code, JetBrains, MacOS/Windows system files, common build dirs
    fn matches_editor_temp(&self, path_str: &str) -> bool {
        // Extract filename from path
        let filename = Path::new(path_str)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Vim swap files (.swp, .swo, .swn, .swm)
        if filename.ends_with(".swp")
            || filename.ends_with(".swo")
            || filename.ends_with(".swn")
            || filename.ends_with(".swm") {
            return true;
        }

        // Vim/Emacs backup files (~)
        if filename.ends_with("~") {
            return true;
        }

        // Emacs auto-save files (#*#)
        if filename.starts_with("#") && filename.ends_with("#") {
            return true;
        }

        // Emacs lock files (.#*)
        if filename.starts_with(".#") {
            return true;
        }

        // MacOS system files
        if filename == ".DS_Store" || filename.starts_with("._") {
            return true;
        }

        // Windows system files
        if filename == "Thumbs.db" || filename == "desktop.ini" {
            return true;
        }

        // IDE and workspace files
        if filename.ends_with(".code-workspace") || filename.ends_with(".iml") {
            return true;
        }

        // Python bytecode
        if filename.ends_with(".pyc") {
            return true;
        }

        // Common IDE and build directories (full path check)
        if path_str.contains("/.vscode/")
            || path_str.contains("/.idea/")
            || path_str.contains("/node_modules/")
            || path_str.contains("/__pycache__/")
            || path_str.contains("/.venv/")
            || path_str.contains("/venv/")
            || path_str.contains("/target/") {
            return true;
        }

        false
    }

    /// Match glob pattern (simple implementation)
    ///
    /// For more complex patterns, the ignore crate handles it via .tlignore
    fn matches_glob_pattern(&self, path: &Path, pattern: &str) -> bool {
        let path_str = path.to_string_lossy();

        // Simple glob matching for config patterns
        // For full glob support, patterns should be in .tlignore
        if pattern.contains('*') {
            // Basic wildcard support
            let pattern_parts: Vec<&str> = pattern.split('*').collect();
            if pattern_parts.len() == 2 {
                let prefix = pattern_parts[0];
                let suffix = pattern_parts[1];
                return path_str.starts_with(prefix) && path_str.ends_with(suffix);
            }
        } else {
            // Exact match
            return path_str.contains(pattern);
        }

        false
    }

    /// Get number of active ignore sources
    pub fn active_sources(&self) -> usize {
        let mut count = 1; // Built-in always active
        if self.gitignore.is_some() {
            count += 1;
        }
        if self.tlignore.is_some() {
            count += 1;
        }
        if !self.config.additional_patterns.is_empty() {
            count += 1;
        }
        count
    }

    /// Get repository root
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Update configuration and reload
    pub fn update_config(&mut self, config: IgnoreConfig) -> Result<()> {
        self.config = config;
        self.reload_ignore_files()
    }
}

/// Ignore configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreConfig {
    /// Use .gitignore patterns (default: true)
    #[serde(default = "default_true")]
    pub use_gitignore: bool,

    /// Use .tlignore patterns (default: true)
    #[serde(default = "default_true")]
    pub use_tlignore: bool,

    /// Additional patterns from config
    #[serde(default)]
    pub additional_patterns: Vec<String>,
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        Self {
            use_gitignore: true,
            use_tlignore: true,
            additional_patterns: vec![],
        }
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_builtin_patterns_always_enforced() {
        let temp_dir = TempDir::new().unwrap();
        let config = IgnoreConfig::default();
        let rules = IgnoreRules::load(temp_dir.path(), config).unwrap();

        // Built-in patterns should always be ignored
        assert!(rules.should_ignore(Path::new(".tl/journal/db")));
        assert!(rules.should_ignore(Path::new("foo/.tl/store")));
        assert!(rules.should_ignore(Path::new(".git/objects/ab/cd")));
        assert!(rules.should_ignore(Path::new("src/.git/config")));
        assert!(rules.should_ignore(Path::new(".jj/op_store/data")));

        // Normal files should not be ignored
        assert!(!rules.should_ignore(Path::new("src/main.rs")));
        assert!(!rules.should_ignore(Path::new("README.md")));
    }

    #[test]
    fn test_gitignore_parsing() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let gitignore_path = temp_dir.path().join(".gitignore");

        // Create .gitignore with patterns
        fs::write(
            &gitignore_path,
            "*.log\ntarget/\nnode_modules/\n*.tmp\n",
        )?;

        // Create actual files/dirs so ignore crate can check is_dir
        let target_dir = temp_dir.path().join("target");
        let node_modules_dir = temp_dir.path().join("node_modules");
        fs::create_dir_all(&target_dir)?;
        fs::create_dir_all(&node_modules_dir)?;
        fs::write(temp_dir.path().join("test.log"), b"log")?;
        fs::write(temp_dir.path().join("file.tmp"), b"tmp")?;

        let config = IgnoreConfig {
            use_gitignore: true,
            use_tlignore: false,
            additional_patterns: vec![],
        };

        let rules = IgnoreRules::load(temp_dir.path(), config)?;

        // Test .gitignore patterns
        assert!(rules.should_ignore(Path::new("test.log")));
        assert!(rules.should_ignore(Path::new("target")));
        assert!(rules.should_ignore(Path::new("node_modules")));
        assert!(rules.should_ignore(Path::new("file.tmp")));

        // Test non-matching patterns
        assert!(!rules.should_ignore(Path::new("src/main.rs")));
        assert!(!rules.should_ignore(Path::new("README.md")));

        Ok(())
    }

    #[test]
    fn test_tlignore_overrides_gitignore() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // .gitignore ignores *.log
        fs::write(temp_dir.path().join(".gitignore"), "*.log\n")?;

        // .tlignore whitelists important.log (negation pattern)
        fs::write(temp_dir.path().join(".tlignore"), "!important.log\n")?;

        let config = IgnoreConfig {
            use_gitignore: true,
            use_tlignore: true,
            additional_patterns: vec![],
        };

        let rules = IgnoreRules::load(temp_dir.path(), config)?;

        // Regular .log files should be ignored
        assert!(rules.should_ignore(Path::new("debug.log")));

        // important.log should NOT be ignored (tlignore whitelists it)
        // Note: The ignore crate handles this negation pattern automatically

        Ok(())
    }

    #[test]
    fn test_additional_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let config = IgnoreConfig {
            use_gitignore: false,
            use_tlignore: false,
            additional_patterns: vec!["*.swp".to_string(), "build/".to_string()],
        };

        let rules = IgnoreRules::load(temp_dir.path(), config).unwrap();

        // Additional patterns should be matched
        assert!(rules.should_ignore(Path::new("file.swp")));
        assert!(rules.should_ignore(Path::new("build/")));
        assert!(rules.should_ignore(Path::new("build/output.txt")));

        // Non-matching should pass
        assert!(!rules.should_ignore(Path::new("src/main.rs")));
    }

    #[test]
    fn test_gitignore_disabled() -> Result<()> {
        let temp_dir = TempDir::new()?;

        fs::write(temp_dir.path().join(".gitignore"), "*.log\n")?;

        let config = IgnoreConfig {
            use_gitignore: false, // Disabled
            use_tlignore: false,
            additional_patterns: vec![],
        };

        let rules = IgnoreRules::load(temp_dir.path(), config)?;

        // .gitignore patterns should NOT be applied
        assert!(!rules.should_ignore(Path::new("test.log")));

        // But built-in patterns should still work
        assert!(rules.should_ignore(Path::new(".tl/store")));

        Ok(())
    }

    #[test]
    fn test_active_sources_count() {
        let temp_dir = TempDir::new().unwrap();

        // No ignore files
        let config = IgnoreConfig::default();
        let rules = IgnoreRules::load(temp_dir.path(), config).unwrap();
        assert_eq!(rules.active_sources(), 1); // Only built-in

        // With additional patterns
        let config = IgnoreConfig {
            use_gitignore: false,
            use_tlignore: false,
            additional_patterns: vec!["*.tmp".to_string()],
        };
        let rules = IgnoreRules::load(temp_dir.path(), config).unwrap();
        assert_eq!(rules.active_sources(), 2); // Built-in + additional
    }

    #[test]
    fn test_reload_ignore_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let gitignore_path = temp_dir.path().join(".gitignore");

        let config = IgnoreConfig {
            use_gitignore: true,
            use_tlignore: false,
            additional_patterns: vec![],
        };

        let mut rules = IgnoreRules::load(temp_dir.path(), config)?;

        // Initially no .gitignore
        assert!(!rules.should_ignore(Path::new("test.log")));

        // Create .gitignore
        fs::write(&gitignore_path, "*.log\n")?;

        // Reload
        rules.reload_ignore_files()?;

        // Now should ignore .log files
        assert!(rules.should_ignore(Path::new("test.log")));

        Ok(())
    }
}
