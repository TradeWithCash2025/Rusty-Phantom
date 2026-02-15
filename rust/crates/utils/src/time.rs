//! Secure time service that fetches server time from Phantom's time API
//! instead of relying on local machine time which can be manipulated.

use std::sync::Mutex;
use std::time::Instant;
use thiserror::Error;

/// Errors that can occur during time service operations.
#[derive(Debug, Error)]
pub enum TimeError {
    /// HTTP request to the time API failed.
    #[error("Time API request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    /// Time API returned a non-OK status.
    #[error("Time API responded with status: {0}")]
    BadStatus(u16),
    /// Invalid timestamp received from the time API.
    #[error("Invalid timestamp received: {0}")]
    InvalidTimestamp(String),
}

struct TimeCache {
    timestamp: u64,
    fetched_at: Instant,
}

/// Secure time service with caching.
///
/// Fetches time from Phantom's secure time API and caches results
/// to reduce API calls. Falls back to local time if unavailable.
pub struct TimeService {
    cache: Mutex<Option<TimeCache>>,
    cache_duration_ms: u64,
    time_api_url: String,
}

/// Cache duration in milliseconds (30 seconds).
const DEFAULT_CACHE_DURATION: u64 = 30_000;
/// Default time API URL.
const TIME_API_URL: &str = "https://time.phantom.app/utc";

impl TimeService {
    /// Create a new TimeService instance.
    fn new() -> Self {
        Self {
            cache: Mutex::new(None),
            cache_duration_ms: DEFAULT_CACHE_DURATION,
            time_api_url: TIME_API_URL.to_string(),
        }
    }

    /// Get the singleton instance of the time service.
    pub fn instance() -> &'static TimeService {
        use once_cell::sync::Lazy;
        static INSTANCE: Lazy<TimeService> = Lazy::new(TimeService::new);
        &INSTANCE
    }

    /// Get current timestamp from Phantom's secure time API.
    ///
    /// Includes basic caching to reduce API calls.
    /// Falls back to `now_local_ms()` if the time service is unavailable.
    pub async fn now(&self) -> u64 {
        let local_now = now_local_ms();

        // Check cache
        {
            let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref c) = *cache {
                let elapsed = c.fetched_at.elapsed().as_millis() as u64;
                if elapsed < self.cache_duration_ms {
                    return c.timestamp + elapsed;
                }
            }
        }

        // Fetch from API
        match self.fetch_time().await {
            Ok(timestamp) => {
                let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
                *cache = Some(TimeCache {
                    timestamp,
                    fetched_at: Instant::now(),
                });
                timestamp
            }
            Err(_) => {
                // Fallback to local time if the time service is unavailable
                local_now
            }
        }
    }

    /// Synchronous version that uses cached time if available,
    /// otherwise falls back to local time.
    pub fn now_sync(&self) -> u64 {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref c) = *cache {
            let elapsed = c.fetched_at.elapsed().as_millis() as u64;
            if elapsed < self.cache_duration_ms {
                return c.timestamp + elapsed;
            }
        }
        now_local_ms()
    }

    /// Clear the cache (useful for testing).
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        *cache = None;
    }

    async fn fetch_time(&self) -> Result<u64, TimeError> {
        let response = reqwest::get(&self.time_api_url).await?;
        if !response.status().is_success() {
            return Err(TimeError::BadStatus(response.status().as_u16()));
        }
        let text = response.text().await?;
        text.trim()
            .parse::<u64>()
            .map_err(|_| TimeError::InvalidTimestamp(text))
    }
}

/// Get local system time in milliseconds since Unix epoch.
fn now_local_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Get current timestamp from Phantom's secure time API.
///
/// Convenience function that uses the singleton TimeService.
pub async fn get_secure_timestamp() -> u64 {
    TimeService::instance().now().await
}

/// Get current timestamp synchronously, using cached time if available.
///
/// Convenience function that uses the singleton TimeService.
pub fn get_secure_timestamp_sync() -> u64 {
    TimeService::instance().now_sync()
}

/// Clear the time cache (test helper).
pub fn clear_time_cache() {
    TimeService::instance().clear_cache();
}
