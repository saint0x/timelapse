//! Retention policies and garbage collection

use crate::Checkpoint;

/// Retention policy configuration
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// Number of checkpoints to keep (default: 2000)
    pub retain_dense_count: usize,
    /// Time window to keep dense checkpoints (default: 24h)
    pub retain_dense_window_ms: u64,
    /// Always retain pinned checkpoints
    pub retain_pins: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            retain_dense_count: 2000,
            retain_dense_window_ms: 24 * 60 * 60 * 1000, // 24 hours
            retain_pins: true,
        }
    }
}

/// Garbage collector
pub struct GarbageCollector {
    policy: RetentionPolicy,
}

impl GarbageCollector {
    /// Create a new GC with the given policy
    pub fn new(policy: RetentionPolicy) -> Self {
        Self { policy }
    }

    /// Run garbage collection
    pub fn collect(&self, checkpoints: &[Checkpoint]) -> anyhow::Result<Vec<Vec<u8>>> {
        // TODO: Implement mark-and-sweep GC
        // 1. Determine live checkpoint set (pins, last N, recent)
        // 2. Walk reachable trees/blobs
        // 3. Return list of checkpoint IDs to delete
        todo!("Implement GarbageCollector::collect")
    }
}
