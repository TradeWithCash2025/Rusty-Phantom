//! Cross-language differential testing infrastructure.
//!
//! Provides an oracle client to call TypeScript functions via a persistent
//! Node.js child process, and comparison utilities to verify that Rust and
//! TypeScript implementations produce identical outputs.

pub mod compare;
pub mod oracle;

#[cfg(test)]
mod tests {
    #[test]
    fn modules_are_accessible() {
        // Verify the public API surface is wired correctly
        let _ = crate::compare::CompareMode::Exact;
        let result = crate::oracle::OracleResult::Ok(serde_json::Value::Null);
        assert!(result.is_ok());
    }
}
