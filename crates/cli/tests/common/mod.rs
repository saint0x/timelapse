//! Common utilities for integration tests

pub mod fixtures;
pub mod cli;

// Re-export commonly used items
pub use fixtures::{ProjectSize, ProjectTemplate, TestProject};
