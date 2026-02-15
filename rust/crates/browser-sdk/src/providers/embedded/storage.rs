//! Browser-specific storage implementation.
//!
//! Implements the [`EmbeddedStorage`] trait from `phantom-embedded-provider-core`.
//! In the TypeScript SDK, this uses `localStorage` for session persistence.
//! In native Rust, this uses file-based storage in a platform-appropriate
//! data directory, providing equivalent persistence semantics.

use phantom_embedded_provider_core::{EmbeddedStorage, Session};
use std::path::PathBuf;
use tokio::sync::RwLock;

/// Storage key for the session data file.
const SESSION_KEY: &str = "phantom_embedded_session";

/// Storage key for the "should clear previous session" flag.
const CLEAR_PREVIOUS_SESSION_KEY: &str = "phantom_should_clear_previous_session";

/// Browser storage implementation backed by the local filesystem.
///
/// In a browser environment (via wasm), this would use `localStorage`.
/// In native Rust, it persists JSON session data to files in a configurable
/// directory (defaulting to a `.phantom` directory in the user's home folder
/// or the system temp directory).
///
/// Thread-safe via interior `RwLock` guards on cached state.
///
/// # Examples
///
/// ```no_run
/// use phantom_browser_sdk::providers::embedded::BrowserStorage;
///
/// // Use default storage directory
/// let storage = BrowserStorage::new(None);
///
/// // Use a custom directory
/// let storage = BrowserStorage::new(Some("/tmp/phantom-test".into()));
/// ```
pub struct BrowserStorage {
    /// Directory where session files are stored.
    storage_dir: PathBuf,
    /// Cached session to avoid repeated file reads.
    cached_session: RwLock<Option<Session>>,
    /// Cached "should clear" flag.
    cached_should_clear: RwLock<bool>,
}

impl BrowserStorage {
    /// Create a new browser storage instance.
    ///
    /// # Arguments
    /// * `storage_dir` - Optional directory for session files. If `None`,
    ///   defaults to `$HOME/.phantom` or `$TMPDIR/.phantom`.
    pub fn new(storage_dir: Option<PathBuf>) -> Self {
        let dir = storage_dir.unwrap_or_else(|| {
            let base = std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| std::env::temp_dir());
            base.join(".phantom")
        });

        Self {
            storage_dir: dir,
            cached_session: RwLock::new(None),
            cached_should_clear: RwLock::new(false),
        }
    }

    /// Ensure the storage directory exists.
    fn ensure_dir(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.storage_dir.exists() {
            std::fs::create_dir_all(&self.storage_dir).map_err(|e| {
                format!(
                    "Failed to create storage directory '{}': {}",
                    self.storage_dir.display(),
                    e
                )
            })?;
        }
        Ok(())
    }

    /// Get the file path for a given storage key.
    fn key_path(&self, key: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", key))
    }

    /// Read a JSON value from a file.
    fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Box<dyn std::error::Error + Send + Sync>> {
        let path = self.key_path(key);
        if !path.exists() {
            return Ok(None);
        }
        let contents = std::fs::read_to_string(&path).map_err(|e| {
            format!("Failed to read '{}': {}", path.display(), e)
        })?;
        if contents.trim().is_empty() {
            return Ok(None);
        }
        let value: T = serde_json::from_str(&contents).map_err(|e| {
            format!("Failed to parse '{}': {}", path.display(), e)
        })?;
        Ok(Some(value))
    }

    /// Write a JSON value to a file.
    fn write_json<T: serde::Serialize>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.ensure_dir()?;
        let path = self.key_path(key);
        let contents = serde_json::to_string_pretty(value).map_err(|e| {
            format!("Failed to serialize data for '{}': {}", key, e)
        })?;
        std::fs::write(&path, contents).map_err(|e| {
            format!("Failed to write '{}': {}", path.display(), e)
        })?;
        Ok(())
    }

    /// Remove a file by key.
    fn remove_key(
        &self,
        key: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = self.key_path(key);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                format!("Failed to remove '{}': {}", path.display(), e)
            })?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl EmbeddedStorage for BrowserStorage {
    async fn get_session(
        &self,
    ) -> Result<Option<Session>, Box<dyn std::error::Error + Send + Sync>> {
        // Try cache first.
        {
            let cached = self.cached_session.read().await;
            if cached.is_some() {
                return Ok(cached.clone());
            }
        }

        // Fall back to file.
        let session = self.read_json::<Session>(SESSION_KEY)?;

        if let Some(ref s) = session {
            let mut cached = self.cached_session.write().await;
            *cached = Some(s.clone());
        }

        Ok(session)
    }

    async fn save_session(
        &self,
        session: &Session,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.write_json(SESSION_KEY, session)?;

        let mut cached = self.cached_session.write().await;
        *cached = Some(session.clone());

        Ok(())
    }

    async fn clear_session(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.remove_key(SESSION_KEY)?;

        let mut cached = self.cached_session.write().await;
        *cached = None;

        Ok(())
    }

    async fn get_should_clear_previous_session(
        &self,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let cached = self.cached_should_clear.read().await;
        if *cached {
            return Ok(true);
        }

        let value = self.read_json::<bool>(CLEAR_PREVIOUS_SESSION_KEY)?;
        Ok(value.unwrap_or(false))
    }

    async fn set_should_clear_previous_session(
        &self,
        should: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.write_json(CLEAR_PREVIOUS_SESSION_KEY, &should)?;

        let mut cached = self.cached_should_clear.write().await;
        *cached = should;

        Ok(())
    }
}
