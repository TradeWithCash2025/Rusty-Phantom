//! Browser-injected SDK for Phantom wallet integration.
//!
//! Provides a high-level, plugin-based interface for interacting with
//! the Phantom browser extension. Supports Solana, Ethereum, extension
//! detection, and auto-confirm functionality through a modular plugin system.

pub mod auto_confirm;
pub mod ethereum;
pub mod extension;
pub mod solana;
pub mod types;

use std::collections::HashMap;

// ============================================================================
// Plugin system
// ============================================================================

/// A plugin that can extend the Phantom instance.
///
/// In TypeScript, plugins use declaration merging to extend the Phantom interface.
/// In Rust, plugins are registered by name and stored as trait objects.
pub struct Plugin {
    /// Plugin name (e.g., "solana", "ethereum", "extension").
    pub name: String,
    /// Factory function that creates the plugin instance.
    pub create: Box<dyn Fn() -> Box<dyn std::any::Any + Send + Sync> + Send + Sync>,
}

/// Configuration for creating a Phantom instance.
#[derive(Default)]
pub struct CreatePhantomConfig {
    /// List of plugins to register.
    pub plugins: Vec<Plugin>,
}

/// A Phantom instance composed of plugins.
///
/// Plugins are stored by name and can be retrieved as `dyn Any`.
pub struct Phantom {
    plugins: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
}

impl Phantom {
    /// Get a plugin by name, downcasting to the expected type.
    pub fn get<T: 'static>(&self, name: &str) -> Option<&T> {
        self.plugins.get(name).and_then(|p| p.downcast_ref::<T>())
    }

    /// Check if a plugin is registered.
    pub fn has(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }
}

/// Create a Phantom instance with the provided plugins.
///
/// Each plugin extends the Phantom instance by registering under its name.
pub fn create_phantom(config: CreatePhantomConfig) -> Phantom {
    let mut plugins = HashMap::new();

    for plugin in config.plugins {
        let instance = (plugin.create)();
        plugins.insert(plugin.name, instance);
    }

    Phantom { plugins }
}

// Re-export key types
pub use auto_confirm::{
    AutoConfirm, AutoConfirmEnableParams, AutoConfirmProvider, AutoConfirmResult,
    AutoConfirmSupportedChainsResult,
};
pub use ethereum::{
    create_ethereum_plugin, create_siwe_message, Ethereum, EthereumSignInData, EthereumTransaction,
};
pub use extension::{create_extension_plugin, Extension, ExtensionDetector};
pub use solana::{create_solana_plugin, Solana, SolanaSignInData};
pub use types::ProviderStrategy;
