//! Public entry points: rasterize a scene to pixels or PNG bytes.

use zenith_core::{AssetProvider, FontProvider};
use zenith_scene::Scene;

use crate::backend::{RasterBackend, RasterImage};
use crate::error::RenderError;
use crate::tiny_skia::TinySkiaBackend;

/// Rasterize `scene` and encode the result as PNG bytes.
///
/// Uses the [`TinySkiaBackend`] internally.  The output is deterministic:
/// the same scene always produces identical bytes.
///
/// The `fonts` parameter is used to resolve font bytes for any
/// [`zenith_scene::SceneCommand::DrawGlyphRun`] commands in the scene; the
/// `assets` parameter resolves raster image bytes for any
/// [`zenith_scene::SceneCommand::DrawImage`] commands. Runs/images whose id
/// cannot be resolved are silently skipped.
///
/// # Errors
///
/// Returns [`RenderError`] when the scene dimensions are invalid or PNG
/// encoding fails.
pub fn render_png(
    scene: &Scene,
    fonts: &dyn FontProvider,
    assets: &dyn AssetProvider,
) -> Result<Vec<u8>, RenderError> {
    let backend = TinySkiaBackend;
    let image = backend.rasterize(scene, fonts, assets)?;
    backend.encode_png(&image)
}

/// Rasterize two scenes (`left`, `right`), composite them SIDE BY SIDE via
/// [`composite_spread`], and encode the result as deterministic PNG bytes.
///
/// `left` is blitted at `x = 0` and `right` at `x = left.width`. The shared
/// `fonts`/`assets` providers resolve glyph runs and images for both scenes.
///
/// # Errors
///
/// Returns [`RenderError`] when either scene's dimensions are invalid, the
/// combined width overflows, or PNG encoding fails.
pub fn render_spread_png(
    left: &Scene,
    right: &Scene,
    fonts: &dyn FontProvider,
    assets: &dyn AssetProvider,
) -> Result<Vec<u8>, RenderError> {
    let backend = TinySkiaBackend;
    let left_img = backend.rasterize(left, fonts, assets)?;
    let right_img = backend.rasterize(right, fonts, assets)?;
    let spread = composite_spread(&left_img, &right_img)?;
    backend.encode_png(&spread)
}

/// Composite two rasterized pages SIDE BY SIDE into one image: `left` blitted at
/// `x = 0` and `right` blitted at `x = left.width`, both at `y = 0`.
///
/// The output canvas is `width = left.width + right.width` and
/// `height = max(left.height, right.height)`. It starts fully transparent
/// (matching the renderer's default background), so any vertical gap below the
/// shorter page is transparent. Pixels are copied verbatim (straight-alpha
/// RGBA8) — there is no blending, so the result is deterministic.
///
/// # Errors
///
/// Returns [`RenderError`] if the combined width overflows `u32`.
pub fn composite_spread(
    left: &RasterImage,
    right: &RasterImage,
) -> Result<RasterImage, RenderError> {
    let width = left.width.checked_add(right.width).ok_or_else(|| {
        RenderError::new(format!(
            "spread width overflow: {} + {} exceeds u32",
            left.width, right.width
        ))
    })?;
    let height = left.height.max(right.height);

    let stride = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| RenderError::new(format!("spread row stride overflow for width {width}")))?;
    let total = stride.checked_mul(height as usize).ok_or_else(|| {
        RenderError::new(format!("spread buffer size overflow ({width}×{height})"))
    })?;

    // Fully transparent canvas (straight-alpha 0,0,0,0).
    let mut rgba = vec![0u8; total];

    blit(&mut rgba, stride, left, 0);
    blit(&mut rgba, stride, right, left.width as usize);

    Ok(RasterImage {
        width,
        height,
        rgba,
    })
}

/// Copy every row of `src` into `dst` (row stride `dst_stride` bytes) starting
/// at pixel column `x_offset`, with `y = 0`. Pixels are copied straight (no
/// blending). `dst` is assumed large enough (the caller sized it from
/// `composite_spread`).
fn blit(dst: &mut [u8], dst_stride: usize, src: &RasterImage, x_offset: usize) {
    let src_stride = src.width as usize * 4;
    let byte_offset = x_offset * 4;
    for row in 0..src.height as usize {
        let src_start = row * src_stride;
        let src_row = match src.rgba.get(src_start..src_start + src_stride) {
            Some(r) => r,
            None => break, // src buffer shorter than declared height — stop safely
        };
        let dst_start = row * dst_stride + byte_offset;
        if let Some(dst_row) = dst.get_mut(dst_start..dst_start + src_stride) {
            dst_row.copy_from_slice(src_row);
        }
    }
}

/// Rasterize `scene` to a [`RasterImage`] (straight-alpha RGBA8 pixels).
///
/// Useful for pixel-level assertions in tests without decoding a PNG.
///
/// The `fonts` parameter is used to resolve font bytes for any
/// [`zenith_scene::SceneCommand::DrawGlyphRun`] commands in the scene; the
/// `assets` parameter resolves raster image bytes for any
/// [`zenith_scene::SceneCommand::DrawImage`] commands. Runs/images whose id
/// cannot be resolved are silently skipped.
///
/// # Errors
///
/// Returns [`RenderError`] when the scene dimensions are invalid.
pub fn render_image(
    scene: &Scene,
    fonts: &dyn FontProvider,
    assets: &dyn AssetProvider,
) -> Result<RasterImage, RenderError> {
    let backend = TinySkiaBackend;
    backend.rasterize(scene, fonts, assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a solid-color RasterImage of `w`×`h` filled with `rgba`.
    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> RasterImage {
        let mut buf = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..(w * h) {
            buf.extend_from_slice(&rgba);
        }
        RasterImage {
            width: w,
            height: h,
            rgba: buf,
        }
    }

    #[test]
    fn composite_spread_width_is_sum() {
        let left = solid(30, 20, [255, 0, 0, 255]);
        let right = solid(40, 20, [0, 0, 255, 255]);
        let out = composite_spread(&left, &right).expect("composite");
        assert_eq!(out.width, 70, "spread width must be wA + wB");
        assert_eq!(out.height, 20, "spread height must be max(hA, hB)");
        assert_eq!(out.rgba.len(), (70 * 20 * 4) as usize);
    }

    #[test]
    fn composite_spread_height_is_max() {
        let left = solid(10, 50, [1, 2, 3, 255]);
        let right = solid(10, 30, [4, 5, 6, 255]);
        let out = composite_spread(&left, &right).expect("composite");
        assert_eq!(out.height, 50, "height must be the taller of the two");
    }

    #[test]
    fn composite_spread_places_pages_side_by_side() {
        let left = solid(2, 1, [10, 20, 30, 255]);
        let right = solid(3, 1, [40, 50, 60, 255]);
        let out = composite_spread(&left, &right).expect("composite");
        // Row 0: two left pixels, then three right pixels.
        assert_eq!(&out.rgba[0..4], &[10, 20, 30, 255], "x=0 is left page");
        assert_eq!(&out.rgba[4..8], &[10, 20, 30, 255], "x=1 is left page");
        assert_eq!(&out.rgba[8..12], &[40, 50, 60, 255], "x=2 is right page");
        assert_eq!(&out.rgba[12..16], &[40, 50, 60, 255], "x=3 is right page");
        assert_eq!(&out.rgba[16..20], &[40, 50, 60, 255], "x=4 is right page");
    }

    #[test]
    fn composite_spread_short_page_leaves_transparent_gap() {
        // Left is taller; the right page's bottom rows stay transparent.
        let left = solid(1, 2, [9, 9, 9, 255]);
        let right = solid(1, 1, [8, 8, 8, 255]);
        let out = composite_spread(&left, &right).expect("composite");
        assert_eq!(out.width, 2);
        assert_eq!(out.height, 2);
        // Row 1 (second row), right column (x=1) was never written → transparent.
        let stride = 2 * 4;
        let row1_right = &out.rgba[stride + 4..stride + 8];
        assert_eq!(
            row1_right,
            &[0, 0, 0, 0],
            "gap below short page is transparent"
        );
    }
}
