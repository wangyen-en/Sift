// ============================================================
// Sift - Quality Cache (persistent disk cache)
// Stores quality analysis results so re-opening a folder is instant.
// Cache key = hash(file_path + file_size + mtime) so stale entries
// are naturally invalidated when a file changes.
// ============================================================

use crate::models::photo::QualityData;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

pub struct QualityCache {
    dir: PathBuf,
}

impl QualityCache {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Ensure the cache directory exists.
    pub fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)
    }

    fn key(&self, path: &str, size: u64, mtime_secs: u64) -> String {
        let mut h = DefaultHasher::new();
        path.hash(&mut h);
        size.hash(&mut h);
        mtime_secs.hash(&mut h);
        format!("{:016x}.json", h.finish())
    }

    /// Return a cached result if it exists (key already encodes path+size+mtime).
    pub fn get(&self, path: &str, size: u64, mtime_secs: u64) -> Option<QualityData> {
        let file = self.dir.join(self.key(path, size, mtime_secs));
        let content = std::fs::read_to_string(file).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Write a result to the cache.
    pub fn put(&self, path: &str, size: u64, mtime_secs: u64, data: &QualityData) {
        let file = self.dir.join(self.key(path, size, mtime_secs));
        if let Ok(json) = serde_json::to_string_pretty(data) {
            // Best-effort write; cache misses are non-fatal.
            let _ = std::fs::write(file, json);
        }
    }
}

/// Read (file_size, mtime_secs) for a file, used for cache invalidation.
pub fn file_fingerprint(path: &Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let size = meta.len();
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((size, mtime))
}
