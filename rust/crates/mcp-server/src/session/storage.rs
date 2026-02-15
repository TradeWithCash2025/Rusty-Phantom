//! SessionStorage manages secure filesystem storage for OAuth sessions.
//!
//! - Sessions are stored in ~/.phantom-mcp/ by default
//! - Directory permissions are 0o700 (user-only rwx)
//! - File permissions are 0o600 (user-only rw)

use std::fs;
use std::path::PathBuf;

use super::types::SessionData;

/// Secure filesystem storage for session data.
pub struct SessionStorage {
    session_dir: PathBuf,
    session_file: PathBuf,
}

impl SessionStorage {
    /// Create a new session storage.
    pub fn new(session_dir: Option<&str>) -> Self {
        let dir = match session_dir {
            Some(d) => PathBuf::from(d),
            None => dirs_home().join(".phantom-mcp"),
        };
        let file = dir.join("session.json");
        Self {
            session_dir: dir,
            session_file: file,
        }
    }

    /// Ensure session directory exists with secure permissions (0o700).
    fn ensure_session_dir(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.session_dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o700);
            fs::set_permissions(&self.session_dir, perms)?;
        }
        Ok(())
    }

    /// Load session data from disk.
    /// Returns `None` if the session file doesn't exist or is invalid.
    pub fn load(&self) -> Option<SessionData> {
        if !self.session_file.exists() {
            return None;
        }

        let data = fs::read_to_string(&self.session_file).ok()?;
        let session: SessionData = serde_json::from_str(&data).ok()?;

        // Validate required fields
        if session.wallet_id.is_empty()
            || session.organization_id.is_empty()
            || session.auth_user_id.is_empty()
            || session.stamper_keys.public_key.is_empty()
            || session.stamper_keys.secret_key.is_empty()
        {
            return None;
        }

        Some(session)
    }

    /// Save session data to disk with secure permissions (0o600).
    pub fn save(&self, session: &SessionData) -> Result<(), Box<dyn std::error::Error>> {
        self.ensure_session_dir()?;

        let data = serde_json::to_string_pretty(session)?;
        fs::write(&self.session_file, data)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&self.session_file, perms)?;
        }

        Ok(())
    }

    /// Delete the session file from disk.
    pub fn delete(&self) {
        let _ = fs::remove_file(&self.session_file);
    }

    /// Check if a session is expired.
    /// SSO sessions don't expire (stamper keys are permanent).
    pub fn is_expired(&self, _session: &SessionData) -> bool {
        false
    }
}

/// Get the user's home directory.
fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
