//! Phantom UI theme definitions and utilities.
//!
//! This crate provides:
//! - Theme types (`PhantomTheme`, `WebTypography`, `NativeTypography`)
//! - Built-in dark and light themes
//! - Theme merging functions for web and native platforms
//! - Color utility functions (`hex_to_rgba`)
//!
//! Note: React-specific components (Button, Text, Icon, Modal, etc.) and hooks
//! are platform-specific and not included in this Rust crate. This crate focuses
//! on the data-layer theme definitions that can be used by any rendering backend.

pub mod themes;
pub mod utils;

pub use themes::{
    dark_theme, light_theme, merge_theme, merge_theme_native, ComputedPhantomNativeTheme,
    ComputedPhantomWebTheme, NativeTypography, NativeTypographyStyle, PhantomTheme, WebTypography,
    WebTypographyStyle, LOGIN_WITH_PHANTOM_COLOR,
};
pub use utils::hex_to_rgba;
