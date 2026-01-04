//! CLI command execution helpers with automatic timing
//!
//! This module provides a wrapper around the `tl` CLI binary that
//! automatically measures execution time and provides convenient
//! assertion methods.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

/// CLI command builder with timing
pub struct TlCommand {
    binary_path: PathBuf,
    working_dir: PathBuf,
    args: Vec<String>,
    env: HashMap<String, String>,
    stdin_data: Option<String>,
    timeout: Duration,
}

impl TlCommand {
    /// Create a new command in the given working directory
    pub fn new(working_dir: impl AsRef<Path>) -> Self {
        Self {
            binary_path: find_tl_binary(),
            working_dir: working_dir.as_ref().to_path_buf(),
            args: Vec::new(),
            env: HashMap::new(),
            stdin_data: None,
            timeout: Duration::from_secs(30),
        }
    }

    /// Add command arguments
    pub fn args(&mut self, args: &[&str]) -> &mut Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    /// Set environment variable
    pub fn env(&mut self, key: &str, value: &str) -> &mut Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    /// Provide stdin data
    pub fn stdin(&mut self, data: &str) -> &mut Self {
        self.stdin_data = Some(data.to_string());
        self
    }

    /// Set command timeout
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = timeout;
        self
    }

    /// Execute command and return result with timing
    pub fn execute(&self) -> Result<CommandResult> {
        let start = Instant::now();

        let mut command = Command::new(&self.binary_path);
        command
            .args(&self.args)
            .current_dir(&self.working_dir)
            .envs(&self.env);

        // Handle stdin if provided
        if self.stdin_data.is_some() {
            use std::process::Stdio;
            command.stdin(Stdio::piped());
        }

        let output = if let Some(stdin_str) = &self.stdin_data {
            // Spawn process with stdin
            let mut child = command
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn command")?;

            // Write stdin
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin.write_all(stdin_str.as_bytes())?;
            }

            // Wait for completion
            child.wait_with_output()
                .context("Failed to wait for command")?
        } else {
            // Simple execution without stdin
            command.output()
                .context("Failed to execute command")?
        };

        let elapsed = start.elapsed();

        Ok(CommandResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration: elapsed,
        })
    }

    /// Execute and assert success
    pub fn assert_success(&self) -> Result<CommandResult> {
        let result = self.execute()?;

        if !result.success() {
            anyhow::bail!(
                "Command failed (exit code: {}):\nArgs: {:?}\nStdout: {}\nStderr: {}",
                result.exit_code,
                self.args,
                result.stdout,
                result.stderr
            );
        }

        Ok(result)
    }

    /// Execute and expect failure
    pub fn assert_failure(&self) -> Result<CommandResult> {
        let result = self.execute()?;

        if result.success() {
            anyhow::bail!(
                "Command should have failed but succeeded:\nArgs: {:?}\nStdout: {}",
                self.args,
                result.stdout
            );
        }

        Ok(result)
    }
}

/// Command execution result with timing
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration: Duration,
}

impl CommandResult {
    /// Check if command succeeded
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Check if stdout contains text
    pub fn contains_stdout(&self, text: &str) -> bool {
        self.stdout.contains(text)
    }

    /// Check if stderr contains text
    pub fn contains_stderr(&self, text: &str) -> bool {
        self.stderr.contains(text)
    }

    /// Parse checkpoint ID from output (ULID format: 01XXXXXXXXXX...)
    pub fn parse_checkpoint_id(&self) -> Option<String> {
        // Match ULID pattern (26 characters starting with 01)
        for line in self.stdout.lines() {
            if let Some(id) = extract_ulid(line) {
                return Some(id);
            }
        }
        None
    }

    /// Parse all checkpoint IDs from output
    pub fn parse_checkpoint_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for line in self.stdout.lines() {
            if let Some(id) = extract_ulid(line) {
                ids.push(id);
            }
        }
        ids
    }
}

/// Extract ULID from a line of text
pub fn extract_ulid(line: &str) -> Option<String> {
    // ULID is 26 characters: 01XXXXXXXXXXXXXXXXXXXXXXXXX
    // Can be uppercase or mixed case alphanumeric

    // Try regex-based approach for reliability
    for (i, window) in line.as_bytes().windows(26).enumerate() {
        if window[0] == b'0' && window[1] == b'1' {
            let candidate = &line[i..i + 26];

            // Check if all characters are alphanumeric
            if candidate.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

/// Find the tl binary in the target directory
fn find_tl_binary() -> PathBuf {
    // Try to locate the binary relative to the current executable (test binary)
    let mut path = std::env::current_exe().expect("Failed to get current exe path");

    // Go up from test binary location
    path.pop(); // Remove test binary name
    path.pop(); // Remove deps/

    // Try debug first, then release
    let debug_bin = path.join("tl");
    if debug_bin.exists() {
        return debug_bin;
    }

    // Try release
    path.pop(); // Remove debug/
    let release_bin = path.join("release").join("tl");
    if release_bin.exists() {
        return release_bin;
    }

    // Fallback to debug
    path.join("debug").join("tl")
}

/// Macro for convenient command construction
///
/// Usage:
/// ```
/// tl!(dir, "init").assert_success()?;
/// tl!(dir, "restore", &checkpoint_id).stdin("y\n").assert_success()?;
/// ```
#[macro_export]
macro_rules! tl {
    ($dir:expr, $($arg:expr),*) => {{
        let mut cmd = $crate::common::cli::TlCommand::new($dir);
        cmd.args(&[$($arg),*]);
        cmd
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ulid_extraction() {
        let line = "Checkpoint ID: 01HXKJ7NVQW3Y2YMZK5VFZX3G8";
        let id = extract_ulid(line);
        assert_eq!(id, Some("01HXKJ7NVQW3Y2YMZK5VFZX3G8".to_string()));
    }

    #[test]
    fn test_ulid_extraction_multiple() {
        let text = "First: 01HXKJ7NVQW3Y2YMZK5VFZX3G8\nSecond: 01HXKJ8NVQW3Y2YMZK5VFZX3G9";
        let result = CommandResult {
            stdout: text.to_string(),
            stderr: String::new(),
            exit_code: 0,
            duration: Duration::from_millis(10),
        };

        let ids = result.parse_checkpoint_ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "01HXKJ7NVQW3Y2YMZK5VFZX3G8");
        assert_eq!(ids[1], "01HXKJ8NVQW3Y2YMZK5VFZX3G9");
    }
}
