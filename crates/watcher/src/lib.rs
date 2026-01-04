//! File system watching for Timelapse
//!
//! This crate provides platform-specific file system watching with:
//! - Per-path debouncing (200-500ms configurable)
//! - Event coalescing
//! - Overflow recovery
//! - Path interning for memory optimization

pub mod platform;
pub mod debounce;
pub mod coalesce;

use anyhow::Result;
use std::path::Path;

/// File system watcher
pub struct Watcher {
    // TODO: Add watcher implementation fields
}

impl Watcher {
    /// Create a new watcher for the given path
    pub fn new(path: &Path) -> Result<Self> {
        // TODO: Implement watcher initialization
        todo!("Implement Watcher::new")
    }

    /// Start watching for events
    pub fn start(&mut self) -> Result<()> {
        // TODO: Implement watcher start
        todo!("Implement Watcher::start")
    }

    /// Stop watching
    pub fn stop(&mut self) -> Result<()> {
        // TODO: Implement watcher stop
        todo!("Implement Watcher::stop")
    }
}

/// File system event
#[derive(Debug, Clone)]
pub struct WatchEvent {
    /// Path that changed
    pub path: std::path::PathBuf,
    /// Type of change
    pub kind: EventKind,
}

/// Type of file system event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// File created
    Create,
    /// File modified
    Modify,
    /// File deleted
    Delete,
    /// File renamed
    Rename,
}
