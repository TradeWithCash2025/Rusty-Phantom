//! Theme definitions for Phantom UI.
//!
//! Provides dark and light themes along with typography configurations
//! for web and native platforms.

use crate::utils::hex_to_rgba;
use serde::{Deserialize, Serialize};

/// Theme colors and styling for the Phantom modal/UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhantomTheme {
    /// Background color for modal.
    pub background: String,
    /// Secondary color for text, borders, dividers (hex for opacity derivation).
    pub secondary: String,
    /// Error color.
    pub error: String,
    /// Success color.
    pub success: String,
    /// Primary text color.
    pub text: String,
    /// Overlay background (with opacity) — can be rgba or hex.
    pub overlay: String,
    /// Border radius for buttons and modal.
    pub border_radius: String,
    /// Brand color.
    pub brand: String,
}

/// Partial theme override. Only specified (`Some`) fields will override the
/// base theme, matching the TS `Partial<PhantomTheme>` spread semantics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhantomThemeOverride {
    /// Background color for modal.
    pub background: Option<String>,
    /// Secondary color for text, borders, dividers (hex for opacity derivation).
    pub secondary: Option<String>,
    /// Error color.
    pub error: Option<String>,
    /// Success color.
    pub success: Option<String>,
    /// Primary text color.
    pub text: Option<String>,
    /// Overlay background (with opacity) — can be rgba or hex.
    pub overlay: Option<String>,
    /// Border radius for buttons and modal.
    pub border_radius: Option<String>,
    /// Brand color.
    pub brand: Option<String>,
}

/// Apply a partial override on top of a base theme, producing a new complete theme.
/// Each `Some` field in the override replaces the corresponding base field.
fn apply_overrides(base: &PhantomTheme, overrides: &PhantomThemeOverride) -> PhantomTheme {
    PhantomTheme {
        background: overrides.background.clone().unwrap_or_else(|| base.background.clone()),
        secondary: overrides.secondary.clone().unwrap_or_else(|| base.secondary.clone()),
        error: overrides.error.clone().unwrap_or_else(|| base.error.clone()),
        success: overrides.success.clone().unwrap_or_else(|| base.success.clone()),
        text: overrides.text.clone().unwrap_or_else(|| base.text.clone()),
        overlay: overrides.overlay.clone().unwrap_or_else(|| base.overlay.clone()),
        border_radius: overrides.border_radius.clone().unwrap_or_else(|| base.border_radius.clone()),
        brand: overrides.brand.clone().unwrap_or_else(|| base.brand.clone()),
    }
}

/// Computed theme: either a web theme or a native theme.
///
/// Mirrors the TS union type `ComputedPhantomTheme = ComputedPhantomWebTheme | ComputedPhantomNativeTheme`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComputedPhantomTheme {
    /// Web variant with string-based typography (CSS units).
    Web(ComputedPhantomWebTheme),
    /// Native variant with numeric typography (unitless).
    Native(ComputedPhantomNativeTheme),
}

/// Web typography configuration (sizes as strings with CSS units).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebTypography {
    pub caption: WebTypographyStyle,
    pub caption_bold: WebTypographyStyle,
    pub label: WebTypographyStyle,
}

/// A single web typography style.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebTypographyStyle {
    pub font_family: String,
    pub font_size: String,
    pub font_style: String,
    pub font_weight: String,
    pub line_height: String,
    pub letter_spacing: String,
}

/// Native typography configuration (sizes as numbers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeTypography {
    pub caption: NativeTypographyStyle,
    pub caption_bold: NativeTypographyStyle,
    pub label: NativeTypographyStyle,
}

/// A single native typography style.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeTypographyStyle {
    pub font_family: String,
    pub font_size: f64,
    pub font_style: String,
    pub font_weight: String,
    pub line_height: f64,
    pub letter_spacing: f64,
}

/// Computed theme for web (base theme + derived aux color + typography).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedPhantomWebTheme {
    #[serde(flatten)]
    pub theme: PhantomTheme,
    pub aux: String,
    pub typography: WebTypography,
}

/// Computed theme for native (base theme + derived aux color + typography).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedPhantomNativeTheme {
    #[serde(flatten)]
    pub theme: PhantomTheme,
    pub aux: String,
    pub typography: NativeTypography,
}

/// The Phantom brand color.
pub const LOGIN_WITH_PHANTOM_COLOR: &str = "#7C63E7";

/// Default dark theme.
pub fn dark_theme() -> PhantomTheme {
    PhantomTheme {
        background: "#181818".to_string(),
        text: "#FFFFFF".to_string(),
        secondary: "#98979C".to_string(),
        overlay: "rgba(0, 0, 0, 0.7)".to_string(),
        border_radius: "16px".to_string(),
        error: "#F00000".to_string(),
        success: "#1CC700".to_string(),
        brand: LOGIN_WITH_PHANTOM_COLOR.to_string(),
    }
}

/// Default light theme.
pub fn light_theme() -> PhantomTheme {
    PhantomTheme {
        background: "#FFFFFF".to_string(),
        text: "#181818".to_string(),
        secondary: "#98979C".to_string(),
        overlay: "rgba(0, 0, 0, 0.5)".to_string(),
        border_radius: "16px".to_string(),
        error: "#F00000".to_string(),
        success: "#1CC700".to_string(),
        brand: LOGIN_WITH_PHANTOM_COLOR.to_string(),
    }
}

const SF_PRO_FONT_FAMILY: &str =
    "\"SF Pro Text\", -apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif";

/// Merge a custom theme with the dark theme, producing a computed web theme.
///
/// When a full `PhantomTheme` is provided, all its fields replace the base.
/// For partial overrides, use [`merge_theme_with_overrides`] instead.
///
/// The secondary color must be a hex color (starts with `#`) so that the
/// auxiliary color can be derived from it.
pub fn merge_theme(custom: Option<&PhantomTheme>) -> Result<ComputedPhantomWebTheme, String> {
    let base = dark_theme();

    let merged = match custom {
        Some(c) => PhantomTheme {
            background: c.background.clone(),
            text: c.text.clone(),
            secondary: c.secondary.clone(),
            overlay: c.overlay.clone(),
            border_radius: c.border_radius.clone(),
            error: c.error.clone(),
            success: c.success.clone(),
            brand: c.brand.clone(),
        },
        None => base,
    };

    if !merged.secondary.starts_with('#') {
        return Err(
            "Secondary color must be a hex color to derive auxiliary color.".to_string(),
        );
    }

    let aux = hex_to_rgba(&merged.secondary, 0.1)?;

    Ok(ComputedPhantomWebTheme {
        theme: merged,
        aux,
        typography: default_web_typography(),
    })
}

/// Merge partial theme overrides with the dark theme, producing a computed web theme.
///
/// Only fields set to `Some` in the override will replace the corresponding
/// base-theme fields, matching the TS `{ ...darkTheme, ...customTheme }` spread
/// with `Partial<PhantomTheme>`.
pub fn merge_theme_with_overrides(
    overrides: Option<&PhantomThemeOverride>,
) -> Result<ComputedPhantomWebTheme, String> {
    let base = dark_theme();

    let merged = match overrides {
        Some(o) => apply_overrides(&base, o),
        None => base,
    };

    if !merged.secondary.starts_with('#') {
        return Err(
            "Secondary color must be a hex color to derive auxiliary color.".to_string(),
        );
    }

    let aux = hex_to_rgba(&merged.secondary, 0.1)?;

    Ok(ComputedPhantomWebTheme {
        theme: merged,
        aux,
        typography: default_web_typography(),
    })
}

/// Default web typography settings (SF Pro).
fn default_web_typography() -> WebTypography {
    WebTypography {
        caption: WebTypographyStyle {
            font_family: SF_PRO_FONT_FAMILY.to_string(),
            font_size: "14px".to_string(),
            font_style: "normal".to_string(),
            font_weight: "400".to_string(),
            line_height: "17px".to_string(),
            letter_spacing: "-0.14px".to_string(),
        },
        caption_bold: WebTypographyStyle {
            font_family: SF_PRO_FONT_FAMILY.to_string(),
            font_size: "14px".to_string(),
            font_style: "normal".to_string(),
            font_weight: "600".to_string(),
            line_height: "17px".to_string(),
            letter_spacing: "-0.14px".to_string(),
        },
        label: WebTypographyStyle {
            font_family: SF_PRO_FONT_FAMILY.to_string(),
            font_size: "12px".to_string(),
            font_style: "normal".to_string(),
            font_weight: "400".to_string(),
            line_height: "15px".to_string(),
            letter_spacing: "-0.12px".to_string(),
        },
    }
}

/// Merge a custom theme with the dark theme, producing a computed native theme.
///
/// When a full `PhantomTheme` is provided, all its fields replace the base.
/// For partial overrides, use [`merge_theme_native_with_overrides`] instead.
///
/// The secondary color must be a hex color (starts with `#`) so that the
/// auxiliary color can be derived from it.
pub fn merge_theme_native(
    custom: Option<&PhantomTheme>,
) -> Result<ComputedPhantomNativeTheme, String> {
    let base = dark_theme();

    let merged = match custom {
        Some(c) => PhantomTheme {
            background: c.background.clone(),
            text: c.text.clone(),
            secondary: c.secondary.clone(),
            overlay: c.overlay.clone(),
            border_radius: c.border_radius.clone(),
            error: c.error.clone(),
            success: c.success.clone(),
            brand: c.brand.clone(),
        },
        None => base,
    };

    if !merged.secondary.starts_with('#') {
        return Err(
            "Secondary color must be a hex color to derive auxiliary color.".to_string(),
        );
    }

    let aux = hex_to_rgba(&merged.secondary, 0.1)?;

    Ok(ComputedPhantomNativeTheme {
        theme: merged,
        aux,
        typography: default_native_typography(),
    })
}

/// Merge partial theme overrides with the dark theme, producing a computed native theme.
///
/// Only fields set to `Some` in the override will replace the corresponding
/// base-theme fields, matching the TS `{ ...darkTheme, ...customTheme }` spread
/// with `Partial<PhantomTheme>`.
pub fn merge_theme_native_with_overrides(
    overrides: Option<&PhantomThemeOverride>,
) -> Result<ComputedPhantomNativeTheme, String> {
    let base = dark_theme();

    let merged = match overrides {
        Some(o) => apply_overrides(&base, o),
        None => base,
    };

    if !merged.secondary.starts_with('#') {
        return Err(
            "Secondary color must be a hex color to derive auxiliary color.".to_string(),
        );
    }

    let aux = hex_to_rgba(&merged.secondary, 0.1)?;

    Ok(ComputedPhantomNativeTheme {
        theme: merged,
        aux,
        typography: default_native_typography(),
    })
}

/// Default native typography settings (System font).
fn default_native_typography() -> NativeTypography {
    NativeTypography {
        caption: NativeTypographyStyle {
            font_family: "System".to_string(),
            font_size: 14.0,
            font_style: "normal".to_string(),
            font_weight: "400".to_string(),
            line_height: 17.0,
            letter_spacing: -0.14,
        },
        caption_bold: NativeTypographyStyle {
            font_family: "System".to_string(),
            font_size: 14.0,
            font_style: "normal".to_string(),
            font_weight: "600".to_string(),
            line_height: 17.0,
            letter_spacing: -0.14,
        },
        label: NativeTypographyStyle {
            font_family: "System".to_string(),
            font_size: 12.0,
            font_style: "normal".to_string(),
            font_weight: "400".to_string(),
            line_height: 15.0,
            letter_spacing: -0.12,
        },
    }
}
