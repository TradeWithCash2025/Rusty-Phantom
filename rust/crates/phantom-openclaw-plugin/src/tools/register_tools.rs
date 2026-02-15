//! Register Phantom MCP tools as OpenClaw tools.

use phantom_mcp_server::tools;
use serde_json::{json, Value};

use crate::client::types::{OpenClawApi, ToolDefinition, ToolExecutionResult};
use crate::session::PluginSession;
use std::sync::Arc;

/// Register all Phantom MCP tools with OpenClaw.
pub fn register_phantom_tools(api: &dyn OpenClawApi, session: Arc<PluginSession>) {
    let tool_list = tools::tools();

    for mcp_tool in tool_list {
        let session = session.clone();
        let tool_name = mcp_tool.name.to_string();
        let tool_name_for_closure = tool_name.clone();

        api.register_tool(ToolDefinition {
            name: mcp_tool.name.to_string(),
            description: mcp_tool.description.to_string(),
            parameters: serde_json::to_value(&mcp_tool.input_schema).unwrap_or(Value::Null),
            execute: Box::new(move |_id, params| {
                let session = session.clone();
                let tool_name = tool_name_for_closure.clone();

                Box::pin(async move {
                    let tool = match tools::get_tool(&tool_name) {
                        Some(t) => t,
                        None => {
                            return ToolExecutionResult {
                                content: json!([{
                                    "type": "text",
                                    "text": json!({ "error": format!("Tool not found: {}", tool_name) }).to_string(),
                                }]),
                                is_error: true,
                            };
                        }
                    };

                    let create_logger = |prefix: &str| {
                        phantom_mcp_server::utils::logger::Logger::new(prefix)
                    };

                    let session_data = session.get_session().await;
                    let client = session.get_client().await.clone();

                    let context = tools::types::ToolContext {
                        client,
                        session: session_data,
                        logger: create_logger(&tool_name),
                    };

                    let params_value = Value::Object(params);

                    match (tool.handler)(params_value, &context).await {
                        Ok(result) => {
                            let text = match &result {
                                Value::String(s) => s.clone(),
                                other => serde_json::to_string_pretty(other).unwrap_or_default(),
                            };
                            ToolExecutionResult {
                                content: json!([{
                                    "type": "text",
                                    "text": text,
                                }]),
                                is_error: false,
                            }
                        }
                        Err(error) => ToolExecutionResult {
                            content: json!([{
                                "type": "text",
                                "text": json!({ "error": error.to_string() }).to_string(),
                            }]),
                            is_error: true,
                        },
                    }
                })
            }),
        });
    }
}
