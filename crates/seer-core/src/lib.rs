//! Seer Core - Content-addressed storage primitives for Seer checkpoint system
//!
//! This crate provides the foundational storage layer:
//! - BLAKE3 hashing
//! - Blob storage with compression
//! - Tree representation and diffing
//! - On-disk store management

pub mod hash;
pub mod blob;
pub mod tree;
pub mod store;

// Re-export main types for convenience
pub use hash::{Blake3Hash, IncrementalHasher};
pub use blob::{Blob, BlobStore, BlobHeaderV1};
pub use tree::{Tree, Entry, EntryKind, TreeDiff};
pub use store::Store;

/// Common result type used throughout seer-core
pub type Result<T> = anyhow::Result<T>;
