//! OpenClaw Plugin API types.

use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Tool execution result returned to OpenClaw.
pub struct ToolExecutionResult {
    pub content: Value,
    pub is_error: bool,
}

/// Tool definition for registration with OpenClaw.
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    #[allow(clippy::type_complexity)]
    pub execute: Box<
        dyn Fn(
                String,
                serde_json::Map<String, Value>,
            ) -> Pin<Box<dyn Future<Output = ToolExecutionResult> + Send>>
            + Send
            + Sync,
    >,
}

/// OpenClaw Plugin API interface.
#[async_trait::async_trait]
pub trait OpenClawApi: Send + Sync {
    /// Plugin configuration provided by OpenClaw.
    fn config(&self) -> Option<&serde_json::Map<String, Value>>;

    /// Register a tool with OpenClaw.
    fn register_tool(&self, definition: ToolDefinition);
}
