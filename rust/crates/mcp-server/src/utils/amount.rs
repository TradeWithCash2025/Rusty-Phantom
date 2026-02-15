//! Amount parsing utility functions.

/// Parses an amount string in base units (atomic units).
///
/// # Errors
/// Returns an error if the amount is not a valid non-negative integer string.
pub fn parse_base_unit_amount(amount: &str) -> Result<u128, String> {
    if !amount.chars().all(|c| c.is_ascii_digit()) || amount.is_empty() {
        return Err(
            "amount must be a non-negative integer string when amountUnit is 'base'".to_string(),
        );
    }
    amount
        .parse::<u128>()
        .map_err(|e| format!("Failed to parse amount: {}", e))
}

/// Parses an amount string in UI units (human-readable) and converts to base units.
///
/// # Errors
/// Returns an error if the amount format is invalid or has too many decimal places.
pub fn parse_ui_amount(amount: &str, decimals: u32) -> Result<u128, String> {
    // Validate format: digits with optional decimal point
    let valid = if let Some(dot_pos) = amount.find('.') {
        amount[..dot_pos].chars().all(|c| c.is_ascii_digit())
            && !amount[..dot_pos].is_empty()
            && amount[dot_pos + 1..].chars().all(|c| c.is_ascii_digit())
    } else {
        amount.chars().all(|c| c.is_ascii_digit()) && !amount.is_empty()
    };

    if !valid {
        return Err("amount must be a non-negative decimal string".to_string());
    }

    let parts: Vec<&str> = amount.split('.').collect();
    let whole = parts[0];
    let fraction = if parts.len() > 1 { parts[1] } else { "" };

    if fraction.len() > decimals as usize {
        return Err(format!(
            "amount has too many decimal places for token decimals ({})",
            decimals
        ));
    }

    // Pad the fraction to `decimals` places
    let padded_fraction = format!("{:0<width$}", fraction, width = decimals as usize);
    let combined = format!("{}{}", whole, padded_fraction);

    // Strip leading zeros
    let stripped = combined.trim_start_matches('0');
    let stripped = if stripped.is_empty() { "0" } else { stripped };

    stripped
        .parse::<u128>()
        .map_err(|e| format!("Failed to parse amount: {}", e))
}

/// Validates that an amount is positive (greater than zero).
///
/// # Errors
/// Returns an error if the amount is not greater than zero.
pub fn require_positive_amount(amount: u128) -> Result<(), String> {
    if amount == 0 {
        return Err("amount must be greater than 0".to_string());
    }
    Ok(())
}
