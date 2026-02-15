//! Phantom OpenClaw Plugin.
//!
//! Integrates Phantom wallet operations directly with OpenClaw agents
//! by wrapping the Phantom MCP Server tools.

pub mod client;
pub mod session;
pub mod tools;

use std::sync::Arc;

use client::types::OpenClawApi;
use session::{PluginSession, PluginSessionOptions};
use tools::register_tools::register_phantom_tools;

/// Known environment variable configuration keys.
const STRING_CONFIG_KEYS: &[&str] = &[
    "PHANTOM_APP_ID",
    "PHANTOM_CLIENT_ID",
    "PHANTOM_CLIENT_SECRET",
    "PHANTOM_AUTH_BASE_URL",
    "PHANTOM_CONNECT_BASE_URL",
    "PHANTOM_API_BASE_URL",
    "PHANTOM_CALLBACK_PATH",
    "PHANTOM_SSO_PROVIDER",
    "PHANTOM_MCP_DEBUG",
];

/// Apply config values from OpenClaw to environment variables.
fn apply_config_to_env(config: Option<&serde_json::Map<String, serde_json::Value>>) {
    let config = match config {
        Some(c) => c,
        None => return,
    };

    for &key in STRING_CONFIG_KEYS {
        if let Some(serde_json::Value::String(value)) = config.get(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                std::env::set_var(key, trimmed);
            }
        }
    }

    if let Some(raw_port) = config.get("PHANTOM_CALLBACK_PORT") {
        let parsed_port = match raw_port {
            serde_json::Value::Number(n) => n.as_u64().map(|v| v as u16),
            serde_json::Value::String(s) => s.trim().parse::<u16>().ok(),
            _ => None,
        };

        if let Some(port) = parsed_port {
            if port > 0 {
                std::env::set_var("PHANTOM_CALLBACK_PORT", port.to_string());
            }
        }
    }
}

/// Get or create the plugin session with configuration.
fn create_session(config: Option<&serde_json::Map<String, serde_json::Value>>) -> PluginSession {
    apply_config_to_env(config);

    let app_id = std::env::var("PHANTOM_APP_ID")
        .or_else(|_| std::env::var("PHANTOM_CLIENT_ID"))
        .ok();

    let callback_port = std::env::var("PHANTOM_CALLBACK_PORT")
        .ok()
        .and_then(|s| s.trim().parse::<u16>().ok())
        .filter(|&p| p > 0);

    PluginSession::new(PluginSessionOptions {
        app_id,
        callback_port,
        ..Default::default()
    })
}

/// Plugin registration function.
pub async fn register(
    api: &dyn OpenClawApi,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let session = Arc::new(create_session(api.config()));
    session.initialize().await?;
    register_phantom_tools(api, session);
    Ok(())
}
