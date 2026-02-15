//! Phantom MCP Server — Model Context Protocol server for Phantom wallet interactions.
//!
//! Provides session management, OAuth authentication, and MCP tool handlers
//! for wallet operations (get addresses, sign messages, sign transactions,
//! transfer tokens, buy tokens).

pub mod auth;
pub mod server;
pub mod session;
pub mod tools;
pub mod utils;

// Re-export key types
pub use server::{PhantomMCPServer, PhantomMCPServerOptions};
pub use session::manager::SessionManager;
pub use session::types::SessionData;
pub use tools::types::{ToolContext, ToolHandler, ToolInputSchema};
pub use tools::{get_tool, get_tool_names, tools};
pub use utils::logger::Logger;
