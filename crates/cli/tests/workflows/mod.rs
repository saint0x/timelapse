//! Workflow integration tests
//!
//! Tests for complete workflows that exercise multiple commands
//! and validate end-to-end behavior.

pub mod checkpoint_lifecycle;
pub mod restore_rewind;
pub mod edge_cases;
pub mod pin_unpin_gc;
pub mod large_files;
pub mod deep_history;
pub mod publish_pull;
