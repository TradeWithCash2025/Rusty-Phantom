//! Oracle client: persistent Node.js child process for calling TS functions.
//!
//! The oracle reads NDJSON from stdin and writes NDJSON to stdout.
//! A global singleton is used so the Node process is spawned once and
//! shared across all tests (behind a mutex).

use once_cell::sync::Lazy;
use serde_json::Value;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

/// Default timeout for oracle responses in seconds.
const ORACLE_TIMEOUT_SECS: u64 = 30;

/// Result from the TypeScript oracle.
#[derive(Debug, Clone)]
pub enum OracleResult {
    /// TS function returned successfully.
    Ok(Value),
    /// TS function threw an error.
    TsError(String),
}

impl OracleResult {
    /// Unwrap the Ok value or panic with the TS error.
    pub fn unwrap_ok(self) -> Value {
        match self {
            OracleResult::Ok(v) => v,
            OracleResult::TsError(e) => panic!("TS oracle returned error: {e}"),
        }
    }

    /// Returns true if the oracle returned Ok.
    pub fn is_ok(&self) -> bool {
        matches!(self, OracleResult::Ok(_))
    }

    /// Returns true if the oracle returned a TS error.
    pub fn is_ts_error(&self) -> bool {
        matches!(self, OracleResult::TsError(_))
    }
}

/// A persistent Node.js oracle process.
pub struct OracleProcess {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl Drop for OracleProcess {
    fn drop(&mut self) {
        // Flush any buffered stdin data, then kill the process.
        // Closing stdin would signal EOF to the Node process, but since
        // we hold a BufWriter<ChildStdin> we can't take ownership here.
        // Instead, just kill the child process directly.
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl OracleProcess {
    /// Spawn the oracle process.
    ///
    /// The oracle script is at `tools/ts_oracle.mjs` relative to the repo root.
    /// We discover the repo root by walking up from CARGO_MANIFEST_DIR.
    pub fn spawn() -> Result<Self, String> {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        // manifest_dir = .../rust/crates/differential-tests
        // repo root = .../  (3 levels up)
        let repo_root = std::path::Path::new(manifest_dir)
            .parent() // crates/
            .and_then(|p| p.parent()) // rust/
            .and_then(|p| p.parent()) // repo root
            .ok_or_else(|| "Cannot determine repo root from CARGO_MANIFEST_DIR".to_string())?;

        let oracle_script = repo_root.join("tools").join("ts_oracle.mjs");
        if !oracle_script.exists() {
            return Err(format!(
                "Oracle script not found at {}. Run `yarn build:packages` first.",
                oracle_script.display()
            ));
        }

        let mut child = Command::new("node")
            .arg(&oracle_script)
            .current_dir(repo_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to spawn oracle: {e}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to capture oracle stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture oracle stdout".to_string())?;

        Ok(Self {
            child,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
        })
    }

    /// Check whether the oracle process is still running.
    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    /// Call a TS function via the oracle.
    pub fn call(&mut self, fn_name: &str, args: &Value) -> Result<OracleResult, String> {
        if !self.is_alive() {
            return Err("Oracle process has exited".to_string());
        }

        let request = serde_json::json!({
            "fn": fn_name,
            "args": args,
        });

        let line = serde_json::to_string(&request)
            .map_err(|e| format!("Failed to serialize request: {e}"))?;

        self.stdin
            .write_all(line.as_bytes())
            .map_err(|e| format!("Failed to write to oracle stdin: {e}"))?;
        self.stdin
            .write_all(b"\n")
            .map_err(|e| format!("Failed to write newline: {e}"))?;
        self.stdin
            .flush()
            .map_err(|e| format!("Failed to flush oracle stdin: {e}"))?;

        // Read the response line. This blocks until the oracle writes back.
        // Note: A proper timeout would require async I/O or platform-specific
        // non-blocking reads. For test infrastructure this blocking approach
        // is acceptable — if the oracle hangs, the test runner's own timeout
        // (e.g., `cargo test` timeout or CI job timeout) will catch it.
        let mut response_line = String::new();
        self.stdout
            .read_line(&mut response_line)
            .map_err(|e| format!("Failed to read from oracle stdout: {e}"))?;

        if response_line.is_empty() {
            return Err("Oracle returned empty response (process may have crashed)".to_string());
        }

        let response: Value = serde_json::from_str(response_line.trim())
            .map_err(|e| format!("Failed to parse oracle response: {e}\nRaw: {response_line}"))?;

        Self::parse_response(&response, &response_line)
    }

    /// Parse an oracle JSON response into an `OracleResult`.
    fn parse_response(response: &Value, raw_line: &str) -> Result<OracleResult, String> {
        match response.get("ok").and_then(|v| v.as_bool()) {
            Some(true) => Ok(OracleResult::Ok(
                response.get("value").cloned().unwrap_or(Value::Null),
            )),
            Some(false) => {
                let error = response
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                Ok(OracleResult::TsError(error))
            }
            _ => Err(format!("Invalid oracle response format: {raw_line}")),
        }
    }
}

/// Global oracle singleton. All tests share one Node process.
static ORACLE: Lazy<Mutex<OracleProcess>> = Lazy::new(|| {
    Mutex::new(
        OracleProcess::spawn()
            .expect("Failed to spawn TS oracle. Ensure `yarn build:packages` has been run."),
    )
});

/// Call the TS oracle for a given function name and arguments.
///
/// This acquires the global mutex, sends the request, and returns the result.
/// Panics if the oracle process has crashed or the mutex is poisoned.
pub fn oracle_call(fn_name: &str, args: &Value) -> OracleResult {
    let mut oracle = ORACLE
        .lock()
        .expect("Oracle mutex poisoned (a previous test likely panicked)");
    oracle
        .call(fn_name, args)
        .unwrap_or_else(|e| panic!("Oracle call failed for {fn_name}: {e}"))
}

/// The oracle timeout in seconds (exposed for documentation/testing).
pub fn oracle_timeout_secs() -> u64 {
    ORACLE_TIMEOUT_SECS
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_response_ok() {
        let response = json!({"ok": true, "value": "hello"});
        let result = OracleProcess::parse_response(&response, "").unwrap();
        assert!(result.is_ok());
        assert_eq!(result.unwrap_ok(), json!("hello"));
    }

    #[test]
    fn parse_response_ok_null_value() {
        let response = json!({"ok": true});
        let result = OracleProcess::parse_response(&response, "").unwrap();
        assert!(result.is_ok());
        assert_eq!(result.unwrap_ok(), Value::Null);
    }

    #[test]
    fn parse_response_error() {
        let response = json!({"ok": false, "error": "something broke"});
        let result = OracleProcess::parse_response(&response, "").unwrap();
        assert!(result.is_ts_error());
    }

    #[test]
    fn parse_response_invalid_format() {
        let response = json!({"unexpected": "format"});
        let result = OracleProcess::parse_response(&response, "{\"unexpected\":\"format\"}");
        assert!(result.is_err());
    }

    #[test]
    fn oracle_result_is_ok() {
        let ok = OracleResult::Ok(json!(42));
        assert!(ok.is_ok());
        assert!(!ok.is_ts_error());
    }

    #[test]
    fn oracle_result_is_ts_error() {
        let err = OracleResult::TsError("oops".into());
        assert!(!err.is_ok());
        assert!(err.is_ts_error());
    }

    #[test]
    #[should_panic(expected = "TS oracle returned error")]
    fn oracle_result_unwrap_ok_panics_on_error() {
        let err = OracleResult::TsError("test error".into());
        let _ = err.unwrap_ok();
    }

    #[test]
    fn oracle_timeout_is_reasonable() {
        assert!(oracle_timeout_secs() >= 10);
        assert!(oracle_timeout_secs() <= 120);
    }
}
