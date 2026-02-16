//! Browser detection utility to identify browser name and version.

use regex::Regex;

/// Information about the detected browser.
#[derive(Debug, Clone)]
pub struct BrowserInfo {
    pub name: String,
    pub version: String,
    pub user_agent: String,
}

/// Parse browser information from a user agent string.
pub fn parse_browser_from_user_agent(user_agent: &str, has_brave_api: bool) -> BrowserInfo {
    let mut name = "unknown".to_string();
    let mut version = "unknown".to_string();

    if user_agent.is_empty() {
        return BrowserInfo {
            name,
            version,
            user_agent: "unknown".to_string(),
        };
    }

    let extract_major_version = |re: &Regex, ua: &str| -> Option<String> {
        re.captures(ua).and_then(|cap| cap.get(1)).map(|m| {
            m.as_str()
                .split('.')
                .next()
                .unwrap_or("unknown")
                .to_string()
        })
    };

    // Edge (Chromium-based) - must be before Chrome
    if user_agent.contains("Edg/") {
        name = "edge".to_string();
        let re = Regex::new(r"Edg/([0-9]+(?:\.[0-9]+)*)").unwrap();
        if let Some(v) = extract_major_version(&re, user_agent) {
            version = v;
        }
    }
    // Opera
    else if user_agent.contains("OPR/") || user_agent.contains("Opera/") {
        name = "opera".to_string();
        let re = Regex::new(r"(?:OPR|Opera)/([0-9]+(?:\.[0-9]+)*)").unwrap();
        if let Some(v) = extract_major_version(&re, user_agent) {
            version = v;
        }
    }
    // Samsung Internet
    else if user_agent.contains("SamsungBrowser/") {
        name = "samsung".to_string();
        let re = Regex::new(r"SamsungBrowser/([0-9]+(?:\.[0-9]+)*)").unwrap();
        if let Some(v) = extract_major_version(&re, user_agent) {
            version = v;
        }
    }
    // DuckDuckGo
    else if user_agent.contains("DuckDuckGo/") {
        name = "duckduckgo".to_string();
        let re = Regex::new(r"DuckDuckGo/([0-9]+(?:\.[0-9]+)*)").unwrap();
        if let Some(v) = extract_major_version(&re, user_agent) {
            version = v;
        }
    }
    // Brave
    else if user_agent.contains("Chrome/") && has_brave_api {
        name = "brave".to_string();
        let re = Regex::new(r"Chrome/([0-9]+(?:\.[0-9]+)*)").unwrap();
        if let Some(v) = extract_major_version(&re, user_agent) {
            version = v;
        }
    }
    // Mobile browsers
    else if user_agent.contains("Mobile/") || user_agent.contains("Android") {
        if user_agent.contains("Chrome/") {
            name = "chrome-mobile".to_string();
            let re = Regex::new(r"Chrome/([0-9]+(?:\.[0-9]+)*)").unwrap();
            if let Some(v) = extract_major_version(&re, user_agent) {
                version = v;
            }
        } else if user_agent.contains("Firefox/") {
            name = "firefox-mobile".to_string();
            let re = Regex::new(r"Firefox/([0-9]+(?:\.[0-9]+)*)").unwrap();
            if let Some(v) = extract_major_version(&re, user_agent) {
                version = v;
            }
        } else if user_agent.contains("Safari/") && user_agent.contains("Mobile/") {
            name = "safari-mobile".to_string();
            let re = Regex::new(r"Version/([0-9]+(?:\.[0-9]+)*)").unwrap();
            if let Some(v) = extract_major_version(&re, user_agent) {
                version = v;
            }
        } else {
            name = "mobile".to_string();
        }
    }
    // Chrome
    else if user_agent.contains("Chrome/") {
        name = "chrome".to_string();
        let re = Regex::new(r"Chrome/([0-9]+(?:\.[0-9]+)*)").unwrap();
        if let Some(v) = extract_major_version(&re, user_agent) {
            version = v;
        }
    }
    // Firefox
    else if user_agent.contains("Firefox/") {
        name = "firefox".to_string();
        let re = Regex::new(r"Firefox/([0-9]+(?:\.[0-9]+)*)").unwrap();
        if let Some(v) = extract_major_version(&re, user_agent) {
            version = v;
        }
    }
    // Safari
    else if user_agent.contains("Safari/") && !user_agent.contains("Chrome/") {
        name = "safari".to_string();
        let re = Regex::new(r"Version/([0-9]+(?:\.[0-9]+)*)").unwrap();
        if let Some(v) = extract_major_version(&re, user_agent) {
            version = v;
        }
    }

    // Secondary fallback loop: if still unknown, try basic patterns
    if name == "unknown" {
        let fallback_patterns: &[(&str, &str)] = &[
            (r"Chrome/([0-9]+)", "chrome"),
            (r"Firefox/([0-9]+)", "firefox"),
            (r"Safari/([0-9]+)", "safari"),
            (r"Edge/([0-9]+)", "edge"),
            (r"Opera/([0-9]+)", "opera"),
        ];

        for (pattern, browser_name) in fallback_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(cap) = re.captures(user_agent) {
                    name = browser_name.to_string();
                    if let Some(m) = cap.get(1) {
                        version = m.as_str().to_string();
                    }
                    break;
                }
            }
        }
    }

    BrowserInfo {
        name,
        version,
        user_agent: user_agent.to_string(),
    }
}

/// Detect the current browser from a user agent string.
///
/// This is a thin wrapper around [`parse_browser_from_user_agent`] that mirrors
/// the TypeScript `detectBrowser()` API. In the TS version this reads from
/// `window.navigator.userAgent`; since there is no `window` in Rust, the caller
/// must supply the user agent string directly.
pub fn detect_browser(user_agent: &str, has_brave_api: bool) -> BrowserInfo {
    parse_browser_from_user_agent(user_agent, has_brave_api)
}

/// Detect if the device is mobile, using a user agent string and optional
/// screen-dimension / touch-capability flags.
///
/// Mirrors the TypeScript `isMobileDevice()` function which checks
/// `window.navigator.userAgent`, `window.screen` dimensions, and
/// `ontouchstart` / `maxTouchPoints`.  Because Rust has no `window` object,
/// the caller passes these values explicitly.
///
/// * `user_agent` - The user agent string.
/// * `screen_width` - Optional screen width in pixels.
/// * `screen_height` - Optional screen height in pixels.
/// * `has_touch` - Whether the device supports touch input.
///
/// Returns `true` if the device is considered mobile based on the same
/// heuristic as the TS implementation: mobile UA **or** (small screen **and**
/// touch capable).
pub fn is_mobile_device(
    user_agent: &str,
    screen_width: Option<u32>,
    screen_height: Option<u32>,
    has_touch: bool,
) -> bool {
    let is_mobile_ua = is_mobile_user_agent(user_agent);

    let is_small_screen = match (screen_width, screen_height) {
        (Some(w), Some(h)) => w <= 768 || h <= 768,
        _ => false,
    };

    is_mobile_ua || (is_small_screen && has_touch)
}

/// Get a formatted platform name from browser info.
pub fn get_platform_name(info: &BrowserInfo) -> String {
    if info.version != "unknown" {
        format!("{}-v{}", info.name, info.version)
    } else {
        info.name.clone()
    }
}

/// Get a display-friendly browser name.
pub fn get_browser_display_name(info: &BrowserInfo) -> String {
    let capitalized = if info.name.is_empty() {
        "Unknown".to_string()
    } else {
        let mut chars = info.name.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        }
    };

    if info.version != "unknown" {
        format!("{} {}", capitalized, info.version)
    } else {
        capitalized
    }
}

/// Check if a user agent indicates a mobile device.
pub fn is_mobile_user_agent(user_agent: &str) -> bool {
    let ua = user_agent.to_lowercase();
    let mobile_patterns = [
        "android",
        "iphone",
        "ipad",
        "ipod",
        "blackberry",
        "windows phone",
        "mobile",
        "tablet",
        "silk",
        "kindle",
        "opera mini",
        "opera mobi",
    ];
    mobile_patterns.iter().any(|p| ua.contains(p))
}
