//! MCP Tools Registry.
//!
//! This module exports all available MCP tools for the Phantom wallet service.

pub mod buy_token;
pub mod get_wallet_addresses;
pub mod sign_message;
pub mod sign_transaction;
pub mod transfer_tokens;
pub mod types;

use types::ToolHandler;

/// Get all available tools.
pub fn tools() -> Vec<ToolHandler> {
    vec![
        get_wallet_addresses::get_wallet_addresses_tool(),
        sign_transaction::sign_transaction_tool(),
        sign_message::sign_message_tool(),
        transfer_tokens::transfer_tokens_tool(),
        buy_token::buy_token_tool(),
    ]
}

/// Get a tool by name.
pub fn get_tool(name: &str) -> Option<ToolHandler> {
    tools().into_iter().find(|tool| tool.name == name)
}

/// Get all tool names.
pub fn get_tool_names() -> Vec<&'static str> {
    vec![
        "get_wallet_addresses",
        "sign_transaction",
        "sign_message",
        "transfer_tokens",
        "buy_token",
    ]
}
