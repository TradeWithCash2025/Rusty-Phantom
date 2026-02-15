//! PhantomMCPServer — main MCP server implementation.
//!
//! This server:
//! - Manages session lifecycle via SessionManager
//! - Registers MCP tool handlers
//! - Communicates via stdio transport (JSON-RPC)
//! - Handles tools/list and tools/call requests

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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

        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
        })) {
            Ok(result) => result,
            Err(_) => {
                self.logger.error("Failed to list tools: internal error");
                json!({
                    "content": [{
                        "type": "text",
                        "text": json!({ "error": "Failed to list tools" }).to_string(),
                    }],
                    "isError": true,
                })
            }
        }
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
    ///
    /// Initializes the session, then enters a JSON-RPC stdio loop that
    /// reads line-delimited JSON from stdin, dispatches to `list_tools()`
    /// or `call_tool()`, and writes JSON responses to stdout.
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Starting PhantomMCPServer");

        // Initialize session
        self.logger.info("Initializing session");
        self.session_manager.initialize().await?;
        self.logger.info("Session initialized successfully");

        // Connect stdio transport — read JSON-RPC from stdin, write to stdout.
        self.logger.info("Connecting stdio transport");
        self.logger
            .info("Server connected and ready to accept requests");

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                // EOF — stdin closed
                self.logger.info("stdin closed, shutting down");
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Parse the incoming JSON-RPC request
            let request: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    let error_response = json!({
                        "jsonrpc": "2.0",
                        "id": Value::Null,
                        "error": {
                            "code": -32700,
                            "message": format!("Parse error: {}", e),
                        }
                    });
                    let mut out = serde_json::to_string(&error_response).unwrap_or_default();
                    out.push('\n');
                    stdout.write_all(out.as_bytes()).await?;
                    stdout.flush().await?;
                    continue;
                }
            };

            let id = request.get("id").cloned().unwrap_or(Value::Null);
            let method = request
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let result = match method {
                "tools/list" => {
                    self.list_tools()
                }
                "tools/call" => {
                    let params = request.get("params").cloned().unwrap_or(json!({}));
                    let tool_name = params
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let arguments = params
                        .get("arguments")
                        .cloned()
                        .unwrap_or(json!({}));
                    self.call_tool(tool_name, arguments).await
                }
                "initialize" => {
                    // MCP initialize handshake
                    json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "phantom-mcp-server",
                            "version": "1.0.0"
                        }
                    })
                }
                "notifications/initialized" | "ping" => {
                    // Notifications and ping — respond with empty result
                    json!({})
                }
                _ => {
                    let error_response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("Method not found: {}", method),
                        }
                    });
                    let mut out = serde_json::to_string(&error_response).unwrap_or_default();
                    out.push('\n');
                    stdout.write_all(out.as_bytes()).await?;
                    stdout.flush().await?;
                    continue;
                }
            };

            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result,
            });

            let mut out = serde_json::to_string(&response).unwrap_or_default();
            out.push('\n');
            stdout.write_all(out.as_bytes()).await?;
            stdout.flush().await?;
        }

        Ok(())
    }
}
