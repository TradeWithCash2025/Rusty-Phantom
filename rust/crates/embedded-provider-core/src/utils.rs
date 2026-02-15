//! Utility functions for the embedded provider.

use crate::interfaces::DebugLogger;
use std::time::Duration;

/// Retry an async operation with exponential backoff.
///
/// # Arguments
/// * `operation` - The async closure to retry.
/// * `operation_name` - Name for logging.
/// * `logger` - Debug logger.
/// * `max_retries` - Maximum number of attempts (default: 3).
/// * `base_delay_ms` - Base delay in milliseconds (default: 1000).
pub async fn retry_with_backoff<F, Fut, T, E>(
    operation: F,
    operation_name: &str,
    logger: &dyn DebugLogger,
    max_retries: u32,
    base_delay_ms: u64,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error: Option<E> = None;

    for attempt in 1..=max_retries {
        logger.log(
            "EMBEDDED_PROVIDER",
            &format!("Attempting {}", operation_name),
            Some(&serde_json::json!({
                "attempt": attempt,
                "maxRetries": max_retries,
            })),
        );

        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                logger.warn(
                    "EMBEDDED_PROVIDER",
                    &format!("{} failed", operation_name),
                    Some(&serde_json::json!({
                        "attempt": attempt,
                        "maxRetries": max_retries,
                        "error": error.to_string(),
                    })),
                );

                if attempt == max_retries {
                    logger.error(
                        "EMBEDDED_PROVIDER",
                        &format!(
                            "{} failed after {} attempts",
                            operation_name, max_retries
                        ),
                        Some(&serde_json::json!({
                            "finalError": error.to_string(),
                        })),
                    );
                    last_error = Some(error);
                    break;
                }

                // Exponential backoff: base * 2^(attempt-1)
                let delay = base_delay_ms * 2u64.pow(attempt - 1);
                logger.log(
                    "EMBEDDED_PROVIDER",
                    &format!("Retrying {} in {}ms", operation_name, delay),
                    Some(&serde_json::json!({
                        "attempt": attempt + 1,
                        "delay": delay,
                    })),
                );
                tokio::time::sleep(Duration::from_millis(delay)).await;

                last_error = Some(error);
            }
        }
    }

    // Safety: The loop always sets `last_error` before breaking, so the
    // None branch is unreachable. We match explicitly to avoid panic paths.
    match last_error {
        Some(e) => Err(e),
        None => unreachable!("retry loop should always set last_error before exiting"),
    }
}

/// Generate a unique session ID with timestamp.
pub fn generate_session_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random1: u64 = rng.gen();
    let random2: u64 = rng.gen();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!(
        "session_{:x}{:x}_{}",
        random1, random2, now
    )
}
