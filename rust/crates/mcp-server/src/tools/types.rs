//! MCP Tool types and interfaces.

use phantom_client::PhantomClient;

use crate::session::types::SessionData;
use crate::utils::logger::Logger;

/// Context provided to tool handlers.
pub struct ToolContext {
    /// Authenticated PhantomClient instance.
    pub client: PhantomClient,
    /// Current session data.
    pub session: SessionData,
    /// Logger instance for this tool.
    pub logger: Logger,
}

/// JSON Schema for MCP tool input validation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// MCP Tool definition.
pub struct ToolHandler {
    /// Tool name (used in tool calls).
    pub name: &'static str,
    /// Tool description (shown to LLM).
    pub description: &'static str,
    /// JSON schema for input validation.
    pub input_schema: ToolInputSchema,
    /// Tool handler function.
    #[allow(clippy::type_complexity)]
    pub handler: fn(
        serde_json::Value,
        &ToolContext,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>,
                > + Send
                + '_,
        >,
    >,
}
