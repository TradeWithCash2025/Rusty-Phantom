//! Browser-specific debug logger implementation.
//!
//! Implements the [`DebugLogger`] trait from `phantom-embedded-provider-core`.
//! In the TypeScript SDK, this logs to `console.log/warn/error`. In Rust,
//! this uses the `tracing` crate for structured logging output, which can be
//! directed to stdout, files, or any `tracing` subscriber.

use phantom_embedded_provider_core::DebugLogger;

/// Browser debug logger that outputs to the `tracing` framework.
///
/// Maps the embedded provider's debug logging interface onto Rust's `tracing`
/// infrastructure. Each log call emits a `tracing` event at the corresponding
/// severity level, with the category as a structured field.
///
/// # Examples
///
/// ```
/// use phantom_browser_sdk::providers::embedded::BrowserLogger;
///
/// let logger = BrowserLogger::new(true);
/// // Logger is now enabled and will emit tracing events.
/// ```
pub struct BrowserLogger {
    enabled: bool,
}

impl BrowserLogger {
    /// Create a new browser logger.
    ///
    /// # Arguments
    /// * `enabled` - Whether logging is initially enabled. When disabled,
    ///   all log calls are no-ops.
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl DebugLogger for BrowserLogger {
    fn info(&self, category: &str, message: &str, data: Option<&serde_json::Value>) {
        if !self.enabled {
            return;
        }
        match data {
            Some(d) => tracing::info!(category = category, data = %d, "{}", message),
            None => tracing::info!(category = category, "{}", message),
        }
    }

    fn warn(&self, category: &str, message: &str, data: Option<&serde_json::Value>) {
        if !self.enabled {
            return;
        }
        match data {
            Some(d) => tracing::warn!(category = category, data = %d, "{}", message),
            None => tracing::warn!(category = category, "{}", message),
        }
    }

    fn error(&self, category: &str, message: &str, data: Option<&serde_json::Value>) {
        if !self.enabled {
            return;
        }
        match data {
            Some(d) => tracing::error!(category = category, data = %d, "{}", message),
            None => tracing::error!(category = category, "{}", message),
        }
    }

    fn log(&self, category: &str, message: &str, data: Option<&serde_json::Value>) {
        if !self.enabled {
            return;
        }
        match data {
            Some(d) => tracing::debug!(category = category, data = %d, "{}", message),
            None => tracing::debug!(category = category, "{}", message),
        }
    }
}
