//! BLAKE3 hashing primitives for content-addressed storage

use std::path::Path;
use std::time::Duration;
use std::thread::sleep;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

/// A BLAKE3 hash (32 bytes)
#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Blake3Hash([u8; 32]);

impl Blake3Hash {
    /// Create a new Blake3Hash from bytes
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the hash as a byte slice
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        const HEX_CHARS: &[u8] = b"0123456789abcdef";
        let mut hex = String::with_capacity(64);
        for &byte in &self.0 {
            hex.push(HEX_CHARS[(byte >> 4) as usize] as char);
            hex.push(HEX_CHARS[(byte & 0xf) as usize] as char);
        }
        hex
    }

    /// Parse from hex string
    pub fn from_hex(hex: &str) -> Result<Self> {
        if hex.len() != 64 {
            anyhow::bail!("Invalid hex length: expected 64 characters, got {}", hex.len());
        }

        let mut bytes = [0u8; 32];
        for i in 0..32 {
            let high = hex_char_to_nibble(hex.as_bytes()[i * 2])?;
            let low = hex_char_to_nibble(hex.as_bytes()[i * 2 + 1])?;
            bytes[i] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

/// Helper function to convert a hex character to a nibble
fn hex_char_to_nibble(c: u8) -> Result<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => anyhow::bail!("Invalid hex character: {}", c as char),
    }
}

impl std::fmt::Debug for Blake3Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blake3Hash({})", self.to_hex())
    }
}

impl std::fmt::Display for Blake3Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Hash bytes using BLAKE3
pub fn hash_bytes(data: &[u8]) -> Blake3Hash {
    let hash = blake3::hash(data);
    Blake3Hash::from_bytes(*hash.as_bytes())
}

/// Hash a file using BLAKE3 (streaming for large files)
pub fn hash_file(path: &Path) -> Result<Blake3Hash> {
    use std::fs::File;
    use std::io::{BufReader, Read};

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();

    let mut buffer = [0u8; 8192]; // 8KB buffer
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(Blake3Hash::from_bytes(*hash.as_bytes()))
}

/// Hash a file using memory-mapped I/O (optimized for large files > 4MB)
pub fn hash_file_mmap(path: &Path) -> Result<Blake3Hash> {
    use std::fs::File;
    use memmap2::Mmap;

    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let hash = blake3::hash(&mmap);
    Ok(Blake3Hash::from_bytes(*hash.as_bytes()))
}

/// Hash file with stability verification (double-stat pattern)
///
/// Ensures file is not changing during read by checking metadata
/// before and after read operation.
///
/// # Arguments
/// * `path` - File to hash
/// * `max_retries` - Maximum retry attempts (default: 3)
///
/// # Returns
/// * `Ok(hash)` - File is stable, hash is valid
/// * `Err(HashError::UnstableFile)` - File changed too many times during read
/// * `Err(...)` - Other I/O errors
///
/// # Example
/// ```no_run
/// use core::hash::hash_file_stable;
/// use std::path::Path;
///
/// let hash = hash_file_stable(Path::new("file.txt"), 3)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn hash_file_stable(path: &Path, max_retries: u8) -> Result<Blake3Hash> {
    use std::fs;

    for attempt in 0..max_retries {
        // 1. Stat before read
        let stat1 = fs::metadata(path)
            .with_context(|| format!("Failed to stat (pre): {}", path.display()))?;

        // 2. Hash file (existing implementation)
        let hash = hash_file(path)?;

        // 3. Stat after read
        let stat2 = fs::metadata(path)
            .with_context(|| format!("Failed to stat (post): {}", path.display()))?;

        // 4. Verify stability (size + mtime unchanged)
        if stat1.len() == stat2.len() &&
           stat1.modified()? == stat2.modified()? {
            return Ok(hash);
        }

        // File changed during read - exponential backoff
        if attempt < max_retries - 1 {
            let backoff_ms = 50 << attempt;  // 50ms, 100ms, 200ms
            sleep(Duration::from_millis(backoff_ms));
        }
    }

    // Failed after all retries
    Err(anyhow::anyhow!(
        "File {} is unstable after {} read attempts (file changing too rapidly)",
        path.display(),
        max_retries
    ))
}

/// Incremental hasher for building hashes across multiple chunks
pub struct IncrementalHasher {
    inner: blake3::Hasher,
}

impl IncrementalHasher {
    /// Create a new incremental hasher
    pub fn new() -> Self {
        Self {
            inner: blake3::Hasher::new(),
        }
    }

    /// Update the hash with more data
    pub fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    /// Finalize and return the hash
    pub fn finalize(self) -> Blake3Hash {
        let hash = self.inner.finalize();
        Blake3Hash::from_bytes(*hash.as_bytes())
    }
}

impl Default for IncrementalHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_hash_consistency() {
        let data = b"hello world";
        let hash1 = hash_bytes(data);
        let hash2 = hash_bytes(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hex_encoding_roundtrip() {
        let original = Blake3Hash::from_bytes([42; 32]);
        let hex = original.to_hex();
        let decoded = Blake3Hash::from_hex(&hex).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_hex_encoding_lowercase() {
        // Create a pattern that repeats [0xde, 0xad, 0xbe, 0xef] to fill 32 bytes
        let pattern = [0xde, 0xad, 0xbe, 0xef];
        let mut bytes = [0u8; 32];
        for (i, &byte) in pattern.iter().cycle().take(32).enumerate() {
            bytes[i] = byte;
        }
        let hash = Blake3Hash::from_bytes(bytes);
        let hex = hash.to_hex();
        assert!(hex.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        assert_eq!(hex.len(), 64);
    }

    #[test]
    fn test_hex_decoding_invalid_length() {
        assert!(Blake3Hash::from_hex("abc").is_err());
        assert!(Blake3Hash::from_hex("").is_err());
        assert!(Blake3Hash::from_hex(&"a".repeat(63)).is_err());
    }

    #[test]
    fn test_hex_decoding_invalid_chars() {
        let invalid = "g".repeat(64);
        assert!(Blake3Hash::from_hex(&invalid).is_err());
    }

    #[test]
    fn test_incremental_hasher() {
        let data = b"hello world";
        let hash_direct = hash_bytes(data);

        let mut incremental = IncrementalHasher::new();
        incremental.update(b"hello ");
        incremental.update(b"world");
        let hash_incremental = incremental.finalize();

        assert_eq!(hash_direct, hash_incremental);
    }

    #[test]
    fn test_hash_file() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let file_path = temp_dir.path().join("test.txt");

        let data = b"test file content";
        std::fs::write(&file_path, data)?;

        let hash_from_file = hash_file(&file_path)?;
        let hash_from_bytes = hash_bytes(data);

        assert_eq!(hash_from_file, hash_from_bytes);
        Ok(())
    }

    #[test]
    fn test_hash_file_mmap() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let file_path = temp_dir.path().join("test.txt");

        let data = b"test file content for mmap";
        std::fs::write(&file_path, data)?;

        let hash_mmap = hash_file_mmap(&file_path)?;
        let hash_bytes = hash_bytes(data);

        assert_eq!(hash_mmap, hash_bytes);
        Ok(())
    }

    #[test]
    fn test_hash_large_file() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let file_path = temp_dir.path().join("large.txt");

        // Create a 5MB file
        let mut file = std::fs::File::create(&file_path)?;
        let chunk = vec![0xAB; 1024 * 1024]; // 1MB chunk
        for _ in 0..5 {
            file.write_all(&chunk)?;
        }
        drop(file);

        // Both methods should produce same hash
        let hash_streaming = hash_file(&file_path)?;
        let hash_mmap = hash_file_mmap(&file_path)?;

        assert_eq!(hash_streaming, hash_mmap);
        Ok(())
    }

    #[test]
    fn test_hash_empty_data() {
        let data = b"";
        let hash = hash_bytes(data);
        // BLAKE3 of empty string is deterministic
        let hash2 = hash_bytes(data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_different_data_different_hash() {
        let hash1 = hash_bytes(b"hello");
        let hash2 = hash_bytes(b"world");
        assert_ne!(hash1, hash2);
    }

    // Double-stat verification tests

    #[test]
    fn test_stable_file_succeeds() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let file = temp_dir.path().join("stable.txt");
        std::fs::write(&file, b"stable content")?;

        // Stable file should hash successfully
        let hash = hash_file_stable(&file, 3)?;
        assert_eq!(hash, hash_bytes(b"stable content"));
        Ok(())
    }

    #[test]
    fn test_unstable_file_retries_then_fails() -> Result<()> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;

        let temp_dir = tempfile::tempdir()?;
        let file = temp_dir.path().join("unstable.txt");
        std::fs::write(&file, b"initial")?;

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();
        let file_clone = file.clone();

        // Spawn writer thread that constantly changes file VERY rapidly
        let writer = thread::spawn(move || {
            let mut counter = 0u64;
            while !stop_flag_clone.load(Ordering::Relaxed) {
                let _ = std::fs::write(&file_clone, format!("changing {}", counter));
                counter += 1;
                // No sleep - write as fast as possible
            }
        });

        // Give writer time to start
        thread::sleep(Duration::from_millis(50));

        // Should retry and eventually fail (only 2 retries to make test faster)
        let result = hash_file_stable(&file, 2);

        // Stop writer
        stop_flag.store(true, Ordering::Relaxed);
        writer.join().unwrap();

        // Should fail with unstable file error
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("unstable"), "Error message should mention 'unstable': {}", err_msg);
        Ok(())
    }

    #[test]
    fn test_eventually_stable_file_succeeds() -> Result<()> {
        use std::thread;
        use std::time::Instant;

        let temp_dir = tempfile::tempdir()?;
        let file = temp_dir.path().join("eventually.txt");
        std::fs::write(&file, b"initial")?;

        let file_clone = file.clone();

        // Write for 200ms then stop
        let writer = thread::spawn(move || {
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(200) {
                let _ = std::fs::write(&file_clone, b"changing...");
                thread::sleep(Duration::from_millis(20));
            }
            // Final stable write
            std::fs::write(&file_clone, b"stable now").unwrap();
        });

        // Start after some changes
        thread::sleep(Duration::from_millis(100));

        // Should eventually succeed (with retries)
        let result = hash_file_stable(&file, 10);  // Allow more retries

        writer.join().unwrap();

        // Should succeed
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_stable_hash_matches_regular_hash() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let file = temp_dir.path().join("test.txt");
        let data = b"test data for comparison";
        std::fs::write(&file, data)?;

        let hash_stable = hash_file_stable(&file, 3)?;
        let hash_regular = hash_file(&file)?;
        let hash_bytes = hash_bytes(data);

        assert_eq!(hash_stable, hash_regular);
        assert_eq!(hash_stable, hash_bytes);
        Ok(())
    }
}
