//! Phantom OpenClaw Plugin.
//!
//! Integrates Phantom wallet operations directly with OpenClaw agents
//! by wrapping the Phantom MCP Server tools.

pub mod client;
pub mod session;
pub mod tools;

use std::sync::{Arc, Mutex, OnceLock};

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

/// Singleton session instance, matching the TS `sessionInstance` pattern.
///
/// The outer `OnceLock` ensures single initialization of the container.
/// The inner `Mutex<Option<...>>` allows clearing the session via `reset_session()`.
static INSTANCE: OnceLock<Mutex<Option<Arc<PluginSession>>>> = OnceLock::new();

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

/// Get or create the singleton plugin session with configuration.
///
/// Lazily creates and caches the session on first call. Subsequent calls
/// return the cached instance. Mirrors the TS `getSession()` function.
fn get_session(config: Option<&serde_json::Map<String, serde_json::Value>>) -> Arc<PluginSession> {
    let container = INSTANCE.get_or_init(|| Mutex::new(None));
    let mut guard = container.lock().expect("session singleton lock poisoned");

    if let Some(ref session) = *guard {
        return Arc::clone(session);
    }

    apply_config_to_env(config);

    let app_id = std::env::var("PHANTOM_APP_ID")
        .or_else(|_| std::env::var("PHANTOM_CLIENT_ID"))
        .ok();

    let callback_port = std::env::var("PHANTOM_CALLBACK_PORT")
        .ok()
        .and_then(|s| s.trim().parse::<u16>().ok())
        .filter(|&p| p > 0);

    let session = Arc::new(PluginSession::new(PluginSessionOptions {
        app_id,
        callback_port,
        ..Default::default()
    }));

    *guard = Some(Arc::clone(&session));
    session
}

/// Reset the session singleton (used for cleanup on initialization failure).
///
/// Clears the cached session so the next call to `get_session()` creates a
/// fresh instance. Mirrors the TS `resetSession()` function.
fn reset_session() {
    let container = INSTANCE.get_or_init(|| Mutex::new(None));
    let mut guard = container.lock().expect("session singleton lock poisoned");
    *guard = None;
}

/// Plugin registration function.
///
/// On initialization failure, resets the singleton and logs the error
/// before returning it, matching the TS error-recovery behavior.
pub async fn register(
    api: &dyn OpenClawApi,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let session = get_session(api.config());

    match session.initialize().await {
        Ok(()) => {
            register_phantom_tools(api, session);
            Ok(())
        }
        Err(err) => {
            eprintln!("Failed to initialize Phantom OpenClaw plugin: {err}");
            reset_session();
            Err(err)
        }
    }
}
