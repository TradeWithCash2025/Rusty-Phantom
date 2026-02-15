//! Auth callback URL detection utilities.

use std::collections::HashMap;

/// Check if URL params indicate an auth failure callback.
pub fn is_auth_failure_callback(params: &HashMap<String, String>) -> bool {
    let response_type = params.get("response_type").map(|s| s.as_str());
    let session_id = params.get("session_id");
    response_type == Some("failure") && session_id.is_some()
}

/// Check if URL params indicate an auth callback (success or failure).
pub fn is_auth_callback_url(params: &HashMap<String, String>) -> bool {
    let session_id = params.get("session_id");
    session_id.is_some()
        && (params.contains_key("response_type") || params.contains_key("wallet_id"))
}
