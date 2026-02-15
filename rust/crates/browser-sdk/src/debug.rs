//! Debug logging system for the browser SDK.
//!
//! Provides a singleton-style debug logger with configurable levels,
//! categories, and callbacks.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, OnceLock};

/// Debug severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum DebugLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

/// A debug log message.
#[derive(Debug, Clone)]
pub struct DebugMessage {
    pub timestamp: u64,
    pub level: DebugLevel,
    pub category: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Callback type for debug messages.
pub type DebugCallback = Arc<dyn Fn(&DebugMessage) + Send + Sync>;

/// Singleton debug logger.
pub struct Debug {
    callback: Mutex<Option<DebugCallback>>,
    level: Mutex<DebugLevel>,
    enabled: Mutex<bool>,
}

impl Debug {
    fn new() -> Self {
        Self {
            callback: Mutex::new(None),
            level: Mutex::new(DebugLevel::Error),
            enabled: Mutex::new(false),
        }
    }

    /// Set the debug callback.
    pub fn set_callback(&self, callback: DebugCallback) {
        *self.callback.lock().unwrap() = Some(callback);
    }

    /// Set the debug level.
    pub fn set_level(&self, level: DebugLevel) {
        *self.level.lock().unwrap() = level;
    }

    /// Enable debug logging.
    pub fn enable(&self) {
        *self.enabled.lock().unwrap() = true;
    }

    /// Disable debug logging.
    pub fn disable(&self) {
        *self.enabled.lock().unwrap() = false;
    }

    fn write_log(
        &self,
        level: DebugLevel,
        category: &str,
        message: &str,
        data: Option<serde_json::Value>,
    ) {
        let enabled = *self.enabled.lock().unwrap();
        let current_level = *self.level.lock().unwrap();

        if !enabled || (level as u8) > (current_level as u8) {
            return;
        }

        let msg = DebugMessage {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            level,
            category: category.to_string(),
            message: message.to_string(),
            data,
        };

        let callback = self.callback.lock().unwrap();
        if let Some(cb) = callback.as_ref() {
            cb(&msg);
        }
    }

    /// Log an error.
    pub fn error(&self, category: &str, message: &str, data: Option<serde_json::Value>) {
        self.write_log(DebugLevel::Error, category, message, data);
    }

    /// Log a warning.
    pub fn warn(&self, category: &str, message: &str, data: Option<serde_json::Value>) {
        self.write_log(DebugLevel::Warn, category, message, data);
    }

    /// Log info.
    pub fn info(&self, category: &str, message: &str, data: Option<serde_json::Value>) {
        self.write_log(DebugLevel::Info, category, message, data);
    }

    /// Log a debug message.
    pub fn debug(&self, category: &str, message: &str, data: Option<serde_json::Value>) {
        self.write_log(DebugLevel::Debug, category, message, data);
    }

    /// Log (alias for debug).
    pub fn log(&self, category: &str, message: &str, data: Option<serde_json::Value>) {
        self.write_log(DebugLevel::Debug, category, message, data);
    }
}

static DEBUG_INSTANCE: OnceLock<Debug> = OnceLock::new();

/// Get the global debug logger instance.
pub fn debug() -> &'static Debug {
    DEBUG_INSTANCE.get_or_init(Debug::new)
}

/// Debug category constants.
pub struct DebugCategory;

impl DebugCategory {
    pub const BROWSER_SDK: &'static str = "BrowserSDK";
    pub const PROVIDER_MANAGER: &'static str = "ProviderManager";
    pub const EMBEDDED_PROVIDER: &'static str = "EmbeddedProvider";
    pub const INJECTED_PROVIDER: &'static str = "InjectedProvider";
    pub const PHANTOM_CONNECT_AUTH: &'static str = "PhantomConnectAuth";
    pub const JWT_AUTH: &'static str = "JWTAuth";
    pub const STORAGE: &'static str = "Storage";
    pub const SESSION: &'static str = "Session";
}
