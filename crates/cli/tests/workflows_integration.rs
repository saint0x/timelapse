//! Integration tests for Timelapse
//!
//! Comprehensive end-to-end testing with real subprojects,
//! timing measurement, and workflow validation.

// Test modules
mod common;
mod workflows;

// Re-export common for test modules
#[allow(unused_imports)]
use common::{ProjectSize, ProjectTemplate, TestProject};
