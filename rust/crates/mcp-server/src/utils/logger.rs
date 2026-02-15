//! Logger utility for MCP server.
//!
//! CRITICAL: All logging MUST go to stderr because stdout is reserved
//! for JSON-RPC protocol messages. Using stdout will break the MCP protocol.

use chrono::Utc;
use std::io::Write;

/// Log level for MCP server messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Error,
    Warn,
    Debug,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Debug => "DEBUG",
        }
    }
}

/// Logger that writes to stderr with timestamp, level, and context.
#[derive(Debug, Clone)]
pub struct Logger {
    context: String,
}

impl Logger {
    /// Create a new logger with the given context.
    pub fn new(context: &str) -> Self {
        Self {
            context: context.to_string(),
        }
    }

    /// Write a log message to stderr.
    fn log(&self, level: LogLevel, message: &str) {
        let timestamp = Utc::now().to_rfc3339();
        let log_message = format!(
            "[{}] [{}] [{}] {}\n",
            timestamp,
            level.as_str(),
            self.context,
            message
        );
        let _ = std::io::stderr().write_all(log_message.as_bytes());
    }

    /// Log an info message.
    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    /// Log an error message.
    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    /// Log a warning message.
    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    /// Log a debug message (only if DEBUG or PHANTOM_MCP_DEBUG env var is set).
    pub fn debug(&self, message: &str) {
        if std::env::var("DEBUG").is_ok() || std::env::var("PHANTOM_MCP_DEBUG").is_ok() {
            self.log(LogLevel::Debug, message);
        }
    }

    /// Create a child logger with combined context.
    /// Example: parent context "MCP" + child "Transport" = "MCP:Transport"
    pub fn child(&self, child_context: &str) -> Logger {
        Logger {
            context: format!("{}:{}", self.context, child_context),
        }
    }
}

/// Default singleton logger instance.
pub fn logger() -> Logger {
    Logger::new("MCP")
}
