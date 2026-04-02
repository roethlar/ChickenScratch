//! # Snapshot Manifest
//!
//! Manages snapshot metadata and history.

use crate::utils::error::ChiknError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Snapshot manifest tracking all snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    /// List of snapshots (newest first)
    pub snapshots: Vec<SnapshotEntry>,
}

/// Individual snapshot entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// Snapshot filename
    pub filename: String,

    /// Creation timestamp (RFC3339)
    pub created: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Snapshot type
    pub snapshot_type: SnapshotType,

    /// File size in bytes
    pub size_bytes: u64,
}

/// Type of snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SnapshotType {
    /// Automatic snapshot (before compile, auto-save)
    Automatic,
    /// Manual snapshot created by user
    Manual,
    /// Before major operation
    BeforeOperation,
}

impl SnapshotManifest {
    /// Creates a new empty manifest
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    /// Loads manifest from revs/manifest.json
    pub fn load(revs_path: &Path) -> Result<Self, ChiknError> {
        let manifest_path = revs_path.join("manifest.json");

        if !manifest_path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&manifest_path)?;
        let manifest: SnapshotManifest = serde_json::from_str(&content)
            .map_err(|e| ChiknError::InvalidFormat(format!("Invalid manifest: {}", e)))?;

        Ok(manifest)
    }

    /// Saves manifest to revs/manifest.json
    pub fn save(&self, revs_path: &Path) -> Result<(), ChiknError> {
        let manifest_path = revs_path.join("manifest.json");

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ChiknError::Unknown(format!("JSON serialization error: {}", e)))?;

        fs::write(&manifest_path, json)?;

        Ok(())
    }

    /// Adds a new snapshot entry
    pub fn add_snapshot(&mut self, entry: SnapshotEntry) {
        self.snapshots.insert(0, entry); // Newest first
    }

    /// Removes snapshots beyond the keep count
    pub fn prune(&mut self, keep_count: usize) -> Vec<String> {
        if self.snapshots.len() <= keep_count {
            return Vec::new();
        }

        let to_remove: Vec<String> = self
            .snapshots
            .drain(keep_count..)
            .map(|e| e.filename)
            .collect();

        to_remove
    }
}

impl Default for SnapshotManifest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    #[test]
    fn test_new_manifest() {
        let manifest = SnapshotManifest::new();
        assert_eq!(manifest.snapshots.len(), 0);
    }

    #[test]
    fn test_add_snapshot() {
        let mut manifest = SnapshotManifest::new();

        let entry = SnapshotEntry {
            filename: "snapshot-001.tar.gz".to_string(),
            created: Utc::now().to_rfc3339(),
            description: Some("Test snapshot".to_string()),
            snapshot_type: SnapshotType::Manual,
            size_bytes: 1024,
        };

        manifest.add_snapshot(entry);
        assert_eq!(manifest.snapshots.len(), 1);
    }

    #[test]
    fn test_prune() {
        let mut manifest = SnapshotManifest::new();

        // Add 5 snapshots
        for i in 0..5 {
            manifest.add_snapshot(SnapshotEntry {
                filename: format!("snapshot-{}.tar.gz", i),
                created: Utc::now().to_rfc3339(),
                description: None,
                snapshot_type: SnapshotType::Automatic,
                size_bytes: 1024,
            });
        }

        // Prune to keep only 3
        let removed = manifest.prune(3);

        assert_eq!(manifest.snapshots.len(), 3);
        assert_eq!(removed.len(), 2);
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let revs_path = temp_dir.path().join("revs");
        fs::create_dir(&revs_path).unwrap();

        let mut manifest = SnapshotManifest::new();
        manifest.add_snapshot(SnapshotEntry {
            filename: "test.tar.gz".to_string(),
            created: Utc::now().to_rfc3339(),
            description: Some("Test".to_string()),
            snapshot_type: SnapshotType::Manual,
            size_bytes: 2048,
        });

        // Save
        manifest.save(&revs_path).unwrap();

        // Load
        let loaded = SnapshotManifest::load(&revs_path).unwrap();
        assert_eq!(loaded.snapshots.len(), 1);
        assert_eq!(loaded.snapshots[0].filename, "test.tar.gz");
    }
}
