use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Cached DSID cookie with expiry information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DsidCache {
    pub dsid: String,
    /// Unix timestamp (seconds) when the cookie expires.
    pub expires_unix: u64,
    pub domain: String,
}

impl DsidCache {
    /// Returns `true` if the cached DSID has not yet expired.
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.expires_unix > 0 && now < self.expires_unix
    }

    fn cache_path() -> Option<std::path::PathBuf> {
        crate::utils::get_user_data_dir()
            .ok()
            .map(|p| p.join("dsid_cache.json"))
    }

    /// Loads a cached DSID from disk. Returns `None` if no cache exists or it is unreadable.
    pub fn load() -> Option<Self> {
        let path = Self::cache_path()?;
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Persists this cache entry to disk.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::cache_path().ok_or("Cannot determine cache path")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Removes the cached DSID from disk, if it exists.
    pub fn clear() -> Result<(), Box<dyn std::error::Error>> {
        if let Some(path) = Self::cache_path() {
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }
        Ok(())
    }
}
