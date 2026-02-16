//! Comparison utilities for differential testing.
//!
//! Provides configurable comparison modes, value canonicalization,
//! and failure artifact writing.

use serde_json::Value;
use std::fs;
use std::path::PathBuf;

/// How to compare TS and Rust values.
#[derive(Debug, Clone)]
pub enum CompareMode {
    /// Exact JSON structural equality.
    Exact,
    /// Float comparison with epsilon tolerance.
    Epsilon(f64),
    /// Sort object keys recursively, then exact compare.
    CanonicalizeSortedKeys,
    /// Compare byte arrays (as JSON arrays of numbers).
    ByteArray,
}

/// Result of a comparison.
#[derive(Debug)]
pub struct CompareResult {
    pub matches: bool,
    pub diff_summary: Option<String>,
}

/// Recursively sort all object keys in a JSON value.
pub fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted: serde_json::Map<String, Value> = serde_json::Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted.insert(key.clone(), canonicalize(&map[key]));
            }
            Value::Object(sorted)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

/// Compare two JSON values according to the given mode.
pub fn compare_values(mode: &CompareMode, ts_value: &Value, rust_value: &Value) -> CompareResult {
    match mode {
        CompareMode::Exact => {
            if ts_value == rust_value {
                CompareResult {
                    matches: true,
                    diff_summary: None,
                }
            } else {
                CompareResult {
                    matches: false,
                    diff_summary: Some(format!(
                        "Exact mismatch:\n  TS:   {}\n  Rust: {}",
                        serde_json::to_string(ts_value).unwrap_or_default(),
                        serde_json::to_string(rust_value).unwrap_or_default(),
                    )),
                }
            }
        }
        CompareMode::Epsilon(eps) => {
            let ts_f = ts_value.as_f64();
            let rs_f = rust_value.as_f64();
            match (ts_f, rs_f) {
                (Some(a), Some(b)) if (a - b).abs() <= *eps => CompareResult {
                    matches: true,
                    diff_summary: None,
                },
                (Some(a), Some(b)) => CompareResult {
                    matches: false,
                    diff_summary: Some(format!(
                        "Epsilon mismatch (eps={eps}): TS={a}, Rust={b}, diff={}",
                        (a - b).abs()
                    )),
                },
                _ => CompareResult {
                    matches: false,
                    diff_summary: Some(format!(
                        "Cannot compare as floats: TS={ts_value}, Rust={rust_value}"
                    )),
                },
            }
        }
        CompareMode::CanonicalizeSortedKeys => {
            let ts_canon = canonicalize(ts_value);
            let rs_canon = canonicalize(rust_value);
            if ts_canon == rs_canon {
                CompareResult {
                    matches: true,
                    diff_summary: None,
                }
            } else {
                CompareResult {
                    matches: false,
                    diff_summary: Some(format!(
                        "Canonicalized mismatch:\n  TS:   {}\n  Rust: {}",
                        serde_json::to_string(&ts_canon).unwrap_or_default(),
                        serde_json::to_string(&rs_canon).unwrap_or_default(),
                    )),
                }
            }
        }
        CompareMode::ByteArray => {
            // Both should be arrays of numbers
            let ts_bytes: Option<Vec<u8>> = ts_value
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect());
            let rs_bytes: Option<Vec<u8>> = rust_value
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect());
            match (ts_bytes, rs_bytes) {
                (Some(a), Some(b)) if a == b => CompareResult {
                    matches: true,
                    diff_summary: None,
                },
                (Some(a), Some(b)) => {
                    let first_diff = a
                        .iter()
                        .zip(b.iter())
                        .enumerate()
                        .find(|(_, (x, y))| x != y);
                    let summary = if let Some((i, (x, y))) = first_diff {
                        format!(
                            "Byte array mismatch at index {i}: TS={x}, Rust={y} (TS len={}, Rust len={})",
                            a.len(),
                            b.len()
                        )
                    } else {
                        format!(
                            "Byte array length mismatch: TS len={}, Rust len={}",
                            a.len(),
                            b.len()
                        )
                    };
                    CompareResult {
                        matches: false,
                        diff_summary: Some(summary),
                    }
                }
                _ => CompareResult {
                    matches: false,
                    diff_summary: Some(format!(
                        "Cannot compare as byte arrays: TS={ts_value}, Rust={rust_value}"
                    )),
                },
            }
        }
    }
}

/// Get the FAILURES directory path.
fn failures_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let repo_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("Cannot determine repo root");
    repo_root.join("FAILURES")
}

/// Write a failure artifact to FAILURES/.
pub fn write_failure(
    fn_name: &str,
    args: &Value,
    ts_result: &Value,
    rust_result: &Value,
    diff: &str,
) {
    let dir = failures_dir();
    fs::create_dir_all(&dir).ok();

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%SZ").to_string();
    let safe_fn = fn_name.replace('.', "__");
    let filename = format!("{safe_fn}__{timestamp}.json");

    let failure = serde_json::json!({
        "function": fn_name,
        "args": args,
        "ts_result": ts_result,
        "rust_result": rust_result,
        "diff_summary": diff,
        "timestamp": timestamp,
    });

    let path = dir.join(filename);
    if let Err(e) = fs::write(&path, serde_json::to_string_pretty(&failure).unwrap_or_default()) {
        eprintln!("Warning: failed to write failure artifact to {}: {e}", path.display());
    }
}

/// Assert that TS and Rust values match, writing a failure artifact if they don't.
///
/// This is the main assertion macro for differential tests.
pub fn assert_diff_match(
    mode: &CompareMode,
    fn_name: &str,
    args: &Value,
    ts_value: &Value,
    rust_value: &Value,
) {
    let result = compare_values(mode, ts_value, rust_value);
    if !result.matches {
        let diff = result.diff_summary.as_deref().unwrap_or("unknown diff");
        write_failure(fn_name, args, ts_value, rust_value, diff);
        panic!(
            "Differential test mismatch for {fn_name}:\n{diff}\n  args: {}",
            serde_json::to_string(args).unwrap_or_default()
        );
    }
}
