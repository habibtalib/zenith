//! Pure logic for `zenith render`.
//!
//! Two public entry points:
//! - [`to_scene_json`] — parse → validate → compile → scene JSON string.
//! - [`to_png`]        — parse → validate → compile → PNG bytes.
//!
//! Both operate entirely on in-memory source text; the caller is responsible
//! for all filesystem I/O.
//!
//! This module is split across concern-grouped submodules:
//! - [`entry`]    — the error type, render artifacts, and the public entry points.
//! - [`assets`]   — font/asset provider construction and disk-based diagnostics.
//! - [`pipeline`] — shared parse/validate/page-resolution/hash helpers.

mod assets;
mod entry;
mod pipeline;

#[cfg(test)]
mod tests;

pub use assets::collect_image_dimension_diagnostics;
pub(crate) use assets::{
    build_asset_provider, build_font_provider, collect_missing_asset_diagnostics,
};
pub use entry::{
    PdfArtifact, PngArtifact, RenderCmdErr, SceneArtifact, to_pdf_with_dir, to_png,
    to_png_all_pages, to_png_spread, to_png_with_dir, to_scene_json,
};
