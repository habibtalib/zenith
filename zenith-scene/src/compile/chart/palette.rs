//! Categorical color palette for chart series.
//!
//! A deterministic 8-color sequence used when a series declares no explicit
//! color. Colors are indexed by series position modulo 8.

use crate::ir::Color;

/// Deterministic categorical palette, indexed by series position when a series
/// declares no explicit color. Perceptually distinct at typical chart sizes.
pub(super) const SERIES_PALETTE: [Color; 8] = [
    Color::srgb(66, 133, 244, 255), // blue
    Color::srgb(234, 67, 53, 255),  // red
    Color::srgb(52, 168, 83, 255),  // green
    Color::srgb(251, 188, 4, 255),  // yellow
    Color::srgb(255, 109, 0, 255),  // orange
    Color::srgb(103, 58, 183, 255), // purple
    Color::srgb(0, 172, 193, 255),  // cyan
    Color::srgb(233, 30, 99, 255),  // pink
];
