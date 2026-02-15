//! PhantomMCPServer — main MCP server implementation.
//!
//! This server:
//! - Manages session lifecycle via SessionManager
//! - Registers MCP tool handlers
//! - Communicates via stdio transport (JSON-RPC)
//! - Handles tools/list and tools/call requests

use serde_json::{json, Value};

use crate::session::manager::{SessionManager, SessionManagerOptions};
use crate::tools;
use crate::utils::logger::Logger;

/// Configuration options for PhantomMCPServer.
pub struct PhantomMCPServerOptions {
    /// Session manager configuration.
    pub session: Option<SessionManagerOptions>,
}

impl Default for PhantomMCPServerOptions {
    fn default() -> Self {
        Self { session: None }
    }
}

/// PhantomMCPServer — main server class that wires everything together.
pub struct PhantomMCPServer {
    session_manager: SessionManager,
    logger: Logger,
}

impl PhantomMCPServer {
    /// Create a new PhantomMCPServer instance.
    pub fn new(options: PhantomMCPServerOptions) -> Self {
        let logger = Logger::new("PhantomMCPServer");

        let session_manager = SessionManager::new(
            options.session.unwrap_or_default(),
        );

        logger.info("PhantomMCPServer initialized");

        Self {
            session_manager,
            logger,
        }
    }

    /// Handle a tools/list request.
    pub fn list_tools(&self) -> Value {
        self.logger.info("Handling tools/list request");

        let tool_list = tools::tools();
        let tool_definitions: Vec<Value> = tool_list
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": tool.input_schema,
                })
            })
            .collect();

        self.logger
            .info(&format!("Returning {} tool definitions", tool_definitions.len()));

        json!({ "tools": tool_definitions })
    }

    /// Handle a tools/call request.
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Value {
        self.logger
            .info(&format!("Handling tools/call request for: {}", tool_name));

        // Step 1: Get tool by name
        let tool = match tools::get_tool(tool_name) {
            Some(t) => t,
            None => {
                self.logger.error(&format!("Unknown tool: {}", tool_name));
                return json!({
                    "content": [{
                        "type": "text",
                        "text": json!({ "error": format!("Unknown tool: {}", tool_name) }).to_string(),
                    }],
                    "isError": true,
                });
            }
        };

        // Step 2: Get PhantomClient from SessionManager
        let client = self.session_manager.get_client().clone();
        let session = self.session_manager.get_session().clone();

        // Step 3: Create ToolContext
        let context = tools::types::ToolContext {
            client,
            session,
            logger: self.logger.child(tool_name),
        };

        // Step 4: Execute tool handler
        self.logger
            .info(&format!("Executing tool: {}", tool_name));

        match (tool.handler)(arguments, &context).await {
            Ok(result) => {
                self.logger
                    .info(&format!("Tool execution successful: {}", tool_name));
                json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result).unwrap_or_default(),
                    }],
                })
            }
            Err(error) => {
                self.logger.error(&format!(
                    "Tool execution failed for {}: {}",
                    tool_name, error
                ));
                json!({
                    "content": [{
                        "type": "text",
                        "text": json!({ "error": error.to_string() }).to_string(),
                    }],
                    "isError": true,
                })
            }
        }
    }

    /// Start the MCP server.
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Starting PhantomMCPServer");

        // Initialize session
        self.logger.info("Initializing session");
        self.session_manager.initialize().await?;
        self.logger.info("Session initialized successfully");

        // In a full implementation, this would start a stdio transport
        // reading JSON-RPC messages from stdin and writing to stdout.
        self.logger
            .info("Server initialized and ready to accept requests");

        Ok(())
    }
}
