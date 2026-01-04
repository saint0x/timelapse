//! Append-only checkpoint journal using sled

use crate::Checkpoint;
use anyhow::Result;
use parking_lot::RwLock;
use sled::Db;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use ulid::Ulid;

/// Append-only journal for checkpoints
pub struct Journal {
    /// Sled database
    db: Db,
    /// In-memory index: checkpoint_id -> sequence_number
    index: RwLock<BTreeMap<Ulid, u64>>,
    /// Monotonic sequence counter
    seq_counter: AtomicU64,
}

impl Journal {
    /// Open or create a journal at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let db = sled::open(path.join("checkpoints.db"))?;

        // Build in-memory index on startup
        let mut index = BTreeMap::new();
        let mut max_seq = 0u64;

        for item in db.iter() {
            let (key, value) = item?;
            let seq = u64::from_le_bytes(key.as_ref().try_into()?);
            let checkpoint = Checkpoint::deserialize(&value)?;
            index.insert(checkpoint.id, seq);
            max_seq = max_seq.max(seq);
        }

        Ok(Self {
            db,
            index: RwLock::new(index),
            seq_counter: AtomicU64::new(max_seq + 1),
        })
    }

    /// Append a checkpoint to the journal
    pub fn append(&self, checkpoint: &Checkpoint) -> Result<u64> {
        let seq = self.seq_counter.fetch_add(1, Ordering::SeqCst);
        let key = seq.to_le_bytes();
        let value = checkpoint.serialize()?;

        self.db.insert(&key, value)?;

        // Update index
        self.index.write().insert(checkpoint.id, seq);

        // Flush to ensure durability
        self.db.flush()?;

        Ok(seq)
    }

    /// Get a checkpoint by ID
    pub fn get(&self, id: &Ulid) -> Result<Option<Checkpoint>> {
        let seq = match self.index.read().get(id) {
            Some(&seq) => seq,
            None => return Ok(None),
        };

        let key = seq.to_le_bytes();
        let value = match self.db.get(&key)? {
            Some(v) => v,
            None => return Ok(None),
        };

        Ok(Some(Checkpoint::deserialize(&value)?))
    }

    /// Get the latest checkpoint
    pub fn latest(&self) -> Result<Option<Checkpoint>> {
        let index = self.index.read();
        if index.is_empty() {
            return Ok(None);
        }

        let max_seq = index.values().max().copied().unwrap();
        drop(index);

        let key = max_seq.to_le_bytes();
        let value = self.db.get(&key)?.unwrap();
        Ok(Some(Checkpoint::deserialize(&value)?))
    }

    /// Get the last N checkpoints
    pub fn last_n(&self, count: usize) -> Result<Vec<Checkpoint>> {
        let index = self.index.read();
        let mut seqs: Vec<_> = index.values().copied().collect();
        seqs.sort_unstable();

        let start_idx = seqs.len().saturating_sub(count);
        let recent_seqs = &seqs[start_idx..];

        drop(index);

        let mut checkpoints = Vec::new();
        for &seq in recent_seqs {
            let key = seq.to_le_bytes();
            let value = self.db.get(&key)?.unwrap();
            checkpoints.push(Checkpoint::deserialize(&value)?);
        }

        Ok(checkpoints)
    }

    /// Get checkpoints since a timestamp
    pub fn since(&self, timestamp_ms: u64) -> Result<Vec<Checkpoint>> {
        let index = self.index.read();

        let mut checkpoints = Vec::new();
        for (&id, &seq) in index.iter() {
            let ulid_ts_ms = id.timestamp_ms();
            if ulid_ts_ms >= timestamp_ms {
                let key = seq.to_le_bytes();
                let value = self.db.get(&key)?.unwrap();
                checkpoints.push(Checkpoint::deserialize(&value)?);
            }
        }

        Ok(checkpoints)
    }

    /// Get all checkpoint IDs
    pub fn all_checkpoint_ids(&self) -> Result<HashSet<Ulid>> {
        Ok(self.index.read().keys().copied().collect())
    }

    /// Delete a checkpoint
    pub fn delete(&self, id: &Ulid) -> Result<()> {
        let seq = match self.index.write().remove(id) {
            Some(seq) => seq,
            None => return Ok(()), // Already deleted
        };

        let key = seq.to_le_bytes();
        self.db.remove(&key)?;
        Ok(())
    }

    /// Get the total number of checkpoints
    pub fn count(&self) -> usize {
        self.index.read().len()
    }
}
