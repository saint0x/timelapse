//! Platform-specific file watching implementations

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

// TODO: Add platform-specific watcher implementations
