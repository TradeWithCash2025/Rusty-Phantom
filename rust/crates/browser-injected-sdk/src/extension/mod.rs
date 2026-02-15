//! Extension detection for the Phantom browser-injected SDK.

use crate::Plugin;

/// Trait for checking if the Phantom extension is installed.
///
/// In the TypeScript SDK, this checks `window.phantom`.
/// In Rust, the platform provides this implementation.
pub trait ExtensionDetector: Send + Sync {
    /// Check if the Phantom extension is installed.
    fn is_installed(&self) -> bool;
}

/// Extension plugin result type.
pub struct Extension {
    detector: Box<dyn ExtensionDetector>,
}

impl Extension {
    /// Create a new Extension with the given detector.
    pub fn new(detector: Box<dyn ExtensionDetector>) -> Self {
        Self { detector }
    }

    /// Check if the Phantom extension is installed.
    pub fn is_installed(&self) -> bool {
        self.detector.is_installed()
    }
}

/// Create an extension detection plugin.
pub fn create_extension_plugin(_detector: Box<dyn ExtensionDetector>) -> Plugin {
    Plugin {
        name: "extension".to_string(),
        create: Box::new(move || Box::new(NoopExtension)),
    }
}

/// No-op extension for when no detector is provided.
struct NoopExtension;
