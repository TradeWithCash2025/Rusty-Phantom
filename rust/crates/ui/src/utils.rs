//! UI utility functions.

/// Convert a hex color string to an rgba string.
///
/// # Arguments
/// * `hex` - A hex color string (e.g., "#FF0000" or "FF0000")
/// * `opacity` - Opacity value between 0.0 and 1.0
///
/// # Returns
/// An rgba color string (e.g., "rgba(255, 0, 0, 0.5)")
pub fn hex_to_rgba(hex: &str, opacity: f64) -> Result<String, String> {
    let clean = hex.strip_prefix('#').unwrap_or(hex);

    if clean.len() != 6 {
        return Err(format!("Invalid hex color: {hex}"));
    }

    let r =
        u8::from_str_radix(&clean[0..2], 16).map_err(|_| format!("Invalid hex color: {hex}"))?;
    let g =
        u8::from_str_radix(&clean[2..4], 16).map_err(|_| format!("Invalid hex color: {hex}"))?;
    let b =
        u8::from_str_radix(&clean[4..6], 16).map_err(|_| format!("Invalid hex color: {hex}"))?;

    Ok(format!("rgba({r}, {g}, {b}, {opacity})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_rgba_with_hash() {
        assert_eq!(hex_to_rgba("#FF0000", 1.0).unwrap(), "rgba(255, 0, 0, 1)");
    }

    #[test]
    fn test_hex_to_rgba_without_hash() {
        assert_eq!(hex_to_rgba("00FF00", 0.5).unwrap(), "rgba(0, 255, 0, 0.5)");
    }

    #[test]
    fn test_hex_to_rgba_invalid() {
        assert!(hex_to_rgba("ZZZ", 1.0).is_err());
    }
}
