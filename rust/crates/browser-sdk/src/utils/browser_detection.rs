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
        re.captures(ua)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().split('.').next().unwrap_or("unknown").to_string())
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

    BrowserInfo {
        name,
        version,
        user_agent: user_agent.to_string(),
    }
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
