//! Phantom MCP Server entry point.
//!
//! Creates a PhantomMCPServer and starts the JSON-RPC stdio transport,
//! matching the TypeScript index.ts main() function.

use phantom_mcp_server::{PhantomMCPServer, PhantomMCPServerOptions};

#[tokio::main]
async fn main() {
    let mut server = PhantomMCPServer::new(PhantomMCPServerOptions::default());

    if let Err(error) = server.start().await {
        eprintln!("Fatal error: {}", error);
        std::process::exit(1);
    }
}
