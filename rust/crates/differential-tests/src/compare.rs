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
            keys.sort_unstable();
            for key in keys {
                sorted.insert(key.clone(), canonicalize(&map[key]));
            }
            Value::Object(sorted)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

/// Format a JSON value for diff summaries, falling back to Debug on serialization failure.
fn format_value(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| format!("{v:?}"))
}

/// Strictly convert a JSON array to a byte vector.
///
/// Unlike `filter_map`, this validates that every element is an integer in
/// the 0..=255 range. Returns `None` if the input is not an array, or
/// `Err(summary)` if any element is out of range or not an integer.
fn strict_byte_array(value: &Value) -> Result<Vec<u8>, String> {
    let arr = value
        .as_array()
        .ok_or_else(|| format!("Expected array, got {}", format_value(value)))?;

    let mut bytes = Vec::with_capacity(arr.len());
    for (i, elem) in arr.iter().enumerate() {
        let n = elem
            .as_u64()
            .ok_or_else(|| format!("Element [{i}] is not a u64: {}", format_value(elem)))?;
        if n > 255 {
            return Err(format!("Element [{i}] out of byte range: {n}"));
        }
        bytes.push(n as u8);
    }
    Ok(bytes)
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
                        format_value(ts_value),
                        format_value(rust_value),
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
                        format_value(&ts_canon),
                        format_value(&rs_canon),
                    )),
                }
            }
        }
        CompareMode::ByteArray => {
            let ts_bytes = strict_byte_array(ts_value);
            let rs_bytes = strict_byte_array(rust_value);
            match (ts_bytes, rs_bytes) {
                (Ok(a), Ok(b)) if a == b => CompareResult {
                    matches: true,
                    diff_summary: None,
                },
                (Ok(a), Ok(b)) => {
                    let first_diff = a
                        .iter()
                        .zip(b.iter())
                        .enumerate()
                        .find(|(_, (x, y))| x != y);
                    let summary = if let Some((i, (x, y))) = first_diff {
                        format!(
                            "Byte array mismatch at index {i}: TS={x}, Rust={y} \
                             (TS len={}, Rust len={})",
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
                (Err(e), _) => CompareResult {
                    matches: false,
                    diff_summary: Some(format!("TS side byte array error: {e}")),
                },
                (_, Err(e)) => CompareResult {
                    matches: false,
                    diff_summary: Some(format!("Rust side byte array error: {e}")),
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
    let content = serde_json::to_string_pretty(&failure).unwrap_or_else(|e| {
        format!(
            "{{\"error\": \"Failed to serialize failure artifact: {e}\", \"diff\": \"{diff}\"}}"
        )
    });
    if let Err(e) = fs::write(&path, content) {
        eprintln!(
            "Warning: failed to write failure artifact to {}: {e}",
            path.display()
        );
    }
}

/// Assert that TS and Rust values match, writing a failure artifact if they don't.
///
/// This is the main assertion function for differential tests.
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
            format_value(args)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exact_match_equal_values() {
        let result = compare_values(&CompareMode::Exact, &json!("hello"), &json!("hello"));
        assert!(result.matches);
        assert!(result.diff_summary.is_none());
    }

    #[test]
    fn exact_match_unequal_values() {
        let result = compare_values(&CompareMode::Exact, &json!("hello"), &json!("world"));
        assert!(!result.matches);
        assert!(result.diff_summary.is_some());
    }

    #[test]
    fn epsilon_match_within_tolerance() {
        let result = compare_values(&CompareMode::Epsilon(0.01), &json!(1.005), &json!(1.01));
        assert!(result.matches);
    }

    #[test]
    fn epsilon_match_outside_tolerance() {
        let result = compare_values(&CompareMode::Epsilon(0.001), &json!(1.0), &json!(1.01));
        assert!(!result.matches);
    }

    #[test]
    fn epsilon_non_numeric() {
        let result = compare_values(&CompareMode::Epsilon(0.01), &json!("a"), &json!(1.0));
        assert!(!result.matches);
        assert!(result
            .diff_summary
            .as_ref()
            .unwrap()
            .contains("Cannot compare as floats"));
    }

    #[test]
    fn canonicalize_sorts_keys() {
        let input = json!({"b": 2, "a": 1, "c": {"z": 1, "y": 2}});
        let result = canonicalize(&input);
        let keys: Vec<&String> = result.as_object().unwrap().keys().collect();
        assert_eq!(keys, vec!["a", "b", "c"]);

        // Nested keys are also sorted
        let nested_keys: Vec<&String> = result["c"].as_object().unwrap().keys().collect();
        assert_eq!(nested_keys, vec!["y", "z"]);
    }

    #[test]
    fn canonicalize_sorted_keys_mode() {
        let ts = json!({"b": 1, "a": 2});
        let rust = json!({"a": 2, "b": 1});
        let result = compare_values(&CompareMode::CanonicalizeSortedKeys, &ts, &rust);
        assert!(result.matches);
    }

    #[test]
    fn byte_array_match() {
        let result = compare_values(
            &CompareMode::ByteArray,
            &json!([1, 2, 3]),
            &json!([1, 2, 3]),
        );
        assert!(result.matches);
    }

    #[test]
    fn byte_array_mismatch_content() {
        let result = compare_values(
            &CompareMode::ByteArray,
            &json!([1, 2, 3]),
            &json!([1, 99, 3]),
        );
        assert!(!result.matches);
        assert!(result.diff_summary.as_ref().unwrap().contains("index 1"));
    }

    #[test]
    fn byte_array_mismatch_length() {
        let result = compare_values(&CompareMode::ByteArray, &json!([1, 2, 3]), &json!([1, 2]));
        assert!(!result.matches);
        assert!(result
            .diff_summary
            .as_ref()
            .unwrap()
            .contains("length mismatch"));
    }

    #[test]
    fn byte_array_rejects_out_of_range() {
        let result = compare_values(&CompareMode::ByteArray, &json!([256]), &json!([0]));
        assert!(!result.matches);
        assert!(result
            .diff_summary
            .as_ref()
            .unwrap()
            .contains("out of byte range"));
    }

    #[test]
    fn byte_array_rejects_non_integer() {
        let result = compare_values(
            &CompareMode::ByteArray,
            &json!(["not_a_number"]),
            &json!([0]),
        );
        assert!(!result.matches);
        assert!(result.diff_summary.as_ref().unwrap().contains("not a u64"));
    }

    #[test]
    fn byte_array_empty() {
        let result: CompareResult = compare_values(&CompareMode::ByteArray, &json!([]), &json!([]));
        assert!(result.matches);
    }

    #[test]
    fn byte_array_non_array_input() {
        let result = compare_values(&CompareMode::ByteArray, &json!("not_array"), &json!([1]));
        assert!(!result.matches);
        assert!(result
            .diff_summary
            .as_ref()
            .unwrap()
            .contains("Expected array"));
    }

    #[test]
    fn format_value_produces_json() {
        assert_eq!(format_value(&json!("hello")), "\"hello\"");
        assert_eq!(format_value(&json!(42)), "42");
    }

    #[test]
    fn strict_byte_array_valid() {
        let result = strict_byte_array(&json!([0, 127, 255]));
        assert_eq!(result.unwrap(), vec![0, 127, 255]);
    }

    #[test]
    fn strict_byte_array_empty() {
        let result = strict_byte_array(&json!([]));
        assert_eq!(result.unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn strict_byte_array_overflow() {
        let result = strict_byte_array(&json!([256]));
        assert!(result.is_err());
    }
}
