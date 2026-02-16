//! Cross-language differential testing infrastructure.
//!
//! Provides an oracle client to call TypeScript functions via a persistent
//! Node.js child process, and comparison utilities to verify that Rust and
//! TypeScript implementations produce identical outputs.

pub mod compare;
pub mod oracle;
