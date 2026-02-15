//! Utility modules for the browser SDK.

pub mod auth_callback;
pub mod browser_detection;
pub mod deeplink;

pub use auth_callback::{is_auth_callback_url, is_auth_failure_callback};
pub use browser_detection::{
    get_browser_display_name, get_platform_name, is_mobile_user_agent,
    parse_browser_from_user_agent, BrowserInfo,
};
pub use deeplink::get_deeplink_to_phantom;
