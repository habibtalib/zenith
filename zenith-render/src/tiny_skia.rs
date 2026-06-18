//! Concrete rasterization backend powered by `tiny-skia`.
//!
//! This is the **only** module in the crate that names `tiny_skia` types.
//! All other modules see only the backend-neutral types from `backend.rs`.

use tiny_skia::{Paint, Pixmap, Rect, Transform};
use zenith_scene::{Scene, SceneCommand};

use crate::backend::{RasterBackend, RasterImage};
use crate::error::RenderError;

/// Maximum allowed dimension in either axis (width or height).
///
/// Prevents gigantic allocations from malformed or adversarial scenes.
const MAX_DIMENSION: u32 = 16_384;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Convert scene `f64` dimensions to `u32` pixels, enforcing sanity rules.
///
/// Returns `Err` when:
/// - The value is non-finite (`NaN`, `±inf`).
/// - `value.round()` is `<= 0` (page must have positive extent).
/// - The rounded value exceeds [`MAX_DIMENSION`].
fn f64_to_px(value: f64, axis: &str) -> Result<u32, RenderError> {
    if !value.is_finite() {
        return Err(RenderError::new(format!(
            "scene {axis} is non-finite ({value})"
        )));
    }
    let px = value.round();
    if px <= 0.0 {
        return Err(RenderError::new(format!(
            "scene {axis} rounds to a non-positive value ({px})"
        )));
    }
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let px_u32 = px as u32;
    if px_u32 > MAX_DIMENSION {
        return Err(RenderError::new(format!(
            "scene {axis} ({px_u32}) exceeds maximum allowed dimension ({MAX_DIMENSION})"
        )));
    }
    Ok(px_u32)
}

/// Intersect two axis-aligned rectangles expressed as `(x, y, x2, y2)`.
///
/// Returns `None` when the intersection is empty.
fn intersect_rects(
    (ax, ay, ax2, ay2): (f64, f64, f64, f64),
    (bx, by, bx2, by2): (f64, f64, f64, f64),
) -> Option<(f64, f64, f64, f64)> {
    let ix = ax.max(bx);
    let iy = ay.max(by);
    let ix2 = ax2.min(bx2);
    let iy2 = ay2.min(by2);
    if ix < ix2 && iy < iy2 {
        Some((ix, iy, ix2, iy2))
    } else {
        None
    }
}

/// Convert premultiplied RGBA8 (tiny-skia's internal storage) to straight-alpha RGBA8.
fn premultiplied_to_straight(r: u8, g: u8, b: u8, a: u8) -> (u8, u8, u8, u8) {
    if a == 0 {
        return (0, 0, 0, 0);
    }
    let a_u16 = u16::from(a);
    // Round via (v * 255 + a/2) / a
    let un = |v: u8| -> u8 {
        let v_u16 = u16::from(v);
        // (v * 255 + a/2) / a, clamped to 255
        let result = (v_u16 * 255 + a_u16 / 2) / a_u16;
        result.min(255) as u8
    };
    (un(r), un(g), un(b), a)
}

// ── TinySkiaBackend ───────────────────────────────────────────────────────────

/// CPU rasterization backend backed by the `tiny-skia` library.
///
/// Determinism guarantees:
/// - Anti-aliasing is disabled for all fills → integer-aligned rects produce
///   exact, reproducible pixels with no sub-pixel variance.
/// - No `HashMap`, no random numbers, no timestamps.
/// - PNG encoding via `tiny_skia::Pixmap::encode_png` writes no timestamps.
pub struct TinySkiaBackend;

impl RasterBackend for TinySkiaBackend {
    fn rasterize(&self, scene: &Scene) -> Result<RasterImage, RenderError> {
        let width = f64_to_px(scene.width, "width")?;
        let height = f64_to_px(scene.height, "height")?;

        let mut pixmap = Pixmap::new(width, height).ok_or_else(|| {
            RenderError::new(format!("failed to allocate pixmap ({width}×{height})"))
        })?;
        // Background starts fully transparent (0,0,0,0) — the deterministic default.

        // Clip stack: each entry is (x, y, x2, y2) in scene coordinates.
        // The outermost clip is the page rectangle.
        let page_clip = (0.0_f64, 0.0_f64, scene.width, scene.height);
        let mut clip_stack: Vec<(f64, f64, f64, f64)> = vec![page_clip];

        for cmd in &scene.commands {
            match cmd {
                SceneCommand::PushClip { x, y, w, h } => {
                    let new_rect = (*x, *y, x + w, y + h);
                    let current = *clip_stack.last().unwrap_or(&page_clip);
                    // Push the intersection so the stack always represents the
                    // effective clip at the current nesting depth.
                    let intersected =
                        intersect_rects(current, new_rect).unwrap_or((0.0, 0.0, 0.0, 0.0)); // empty → degenerate
                    clip_stack.push(intersected);
                }

                // Never pop below the page clip (index 0).
                SceneCommand::PopClip if clip_stack.len() > 1 => {
                    clip_stack.pop();
                }

                SceneCommand::FillRect { x, y, w, h, color } => {
                    let fill_rect = (*x, *y, x + w, y + h);
                    let effective_clip = *clip_stack.last().unwrap_or(&page_clip);

                    // Intersect the fill rect with the current effective clip.
                    let (ix, iy, ix2, iy2) = match intersect_rects(fill_rect, effective_clip) {
                        Some(r) => r,
                        None => continue, // nothing to draw
                    };

                    let iw = ix2 - ix;
                    let ih = iy2 - iy;

                    // tiny-skia requires positive, finite values for Rect::from_xywh.
                    if iw <= 0.0
                        || ih <= 0.0
                        || !ix.is_finite()
                        || !iy.is_finite()
                        || !iw.is_finite()
                        || !ih.is_finite()
                    {
                        continue;
                    }

                    let rect = match Rect::from_xywh(ix as f32, iy as f32, iw as f32, ih as f32) {
                        Some(r) => r,
                        None => continue,
                    };

                    let mut paint = Paint::default();
                    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
                    paint.anti_alias = false; // deterministic: no edge AA variance

                    // Drawing outside the pixmap simply touches no pixels; not an error.
                    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                }

                // All other variants are not emitted yet; skip them deterministically.
                _ => {}
            }
        }

        // Convert tiny-skia's premultiplied RGBA8 to straight-alpha RGBA8.
        let raw = pixmap.data(); // &[u8], len = width*height*4, premul RGBA
        let mut rgba = Vec::with_capacity(raw.len());
        for chunk in raw.chunks_exact(4) {
            let (sr, sg, sb, sa) =
                premultiplied_to_straight(chunk[0], chunk[1], chunk[2], chunk[3]);
            rgba.push(sr);
            rgba.push(sg);
            rgba.push(sb);
            rgba.push(sa);
        }

        Ok(RasterImage {
            width,
            height,
            rgba,
        })
    }

    fn encode_png(&self, image: &RasterImage) -> Result<Vec<u8>, RenderError> {
        // Re-premultiply straight-alpha back to premultiplied for tiny-skia.
        let mut premul = Vec::with_capacity(image.rgba.len());
        for chunk in image.rgba.chunks_exact(4) {
            let (r, g, b, a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
            if a == 0 {
                premul.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let a_u16 = u16::from(a);
                let mul = |v: u8| -> u8 {
                    let result = (u16::from(v) * a_u16 + 127) / 255;
                    result.min(255) as u8
                };
                premul.push(mul(r));
                premul.push(mul(g));
                premul.push(mul(b));
                premul.push(a);
            }
        }

        let mut pixmap = Pixmap::new(image.width, image.height).ok_or_else(|| {
            RenderError::new(format!(
                "failed to allocate pixmap for encoding ({}×{})",
                image.width, image.height
            ))
        })?;

        let dst = pixmap.data_mut();
        if dst.len() != premul.len() {
            return Err(RenderError::new(
                "pixel buffer length mismatch during PNG encoding",
            ));
        }
        dst.copy_from_slice(&premul);

        pixmap
            .encode_png()
            .map_err(|e| RenderError::new(format!("PNG encoding failed: {e}")))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use zenith_scene::{Color, Scene, SceneCommand};

    use crate::backend::RasterBackend;

    use super::TinySkiaBackend;

    fn red() -> Color {
        Color {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    fn make_solid_red_scene(page: f64) -> Scene {
        let mut s = Scene::new(page, page);
        s.commands.push(SceneCommand::PushClip {
            x: 0.0,
            y: 0.0,
            w: page,
            h: page,
        });
        s.commands.push(SceneCommand::FillRect {
            x: 0.0,
            y: 0.0,
            w: page,
            h: page,
            color: red(),
        });
        s.commands.push(SceneCommand::PopClip);
        s
    }

    /// Index into a straight-alpha RGBA8 buffer for pixel (px, py) in an image
    /// of the given `width`.
    fn pixel(rgba: &[u8], width: u32, px: u32, py: u32) -> (u8, u8, u8, u8) {
        let base = ((py * width + px) * 4) as usize;
        (rgba[base], rgba[base + 1], rgba[base + 2], rgba[base + 3])
    }

    // ── pixel correctness ─────────────────────────────────────────────────

    #[test]
    fn pixel_correctness_solid_red() {
        let scene = make_solid_red_scene(4.0);
        let backend = TinySkiaBackend;
        let img = backend.rasterize(&scene).expect("rasterize must succeed");
        assert_eq!(img.width, 4);
        assert_eq!(img.height, 4);
        // center pixel
        assert_eq!(pixel(&img.rgba, img.width, 2, 2), (255, 0, 0, 255));
        // corner pixel
        assert_eq!(pixel(&img.rgba, img.width, 0, 0), (255, 0, 0, 255));
    }

    // ── determinism ───────────────────────────────────────────────────────

    #[test]
    fn determinism_identical_png_bytes() {
        let scene = make_solid_red_scene(4.0);
        let backend = TinySkiaBackend;
        let png1 = backend
            .rasterize(&scene)
            .and_then(|img| backend.encode_png(&img))
            .expect("first render");
        let png2 = backend
            .rasterize(&scene)
            .and_then(|img| backend.encode_png(&img))
            .expect("second render");
        assert_eq!(
            png1, png2,
            "PNG output must be byte-identical for the same scene"
        );
    }

    // ── PNG validity ──────────────────────────────────────────────────────

    #[test]
    fn png_magic_bytes() {
        let scene = make_solid_red_scene(4.0);
        let backend = TinySkiaBackend;
        let png = backend
            .rasterize(&scene)
            .and_then(|img| backend.encode_png(&img))
            .expect("render");
        assert_eq!(
            &png[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "output must start with PNG magic bytes"
        );
    }

    // ── clip enforced ─────────────────────────────────────────────────────

    #[test]
    fn clip_clamps_fill_to_page() {
        // 4×4 page; FillRect extends well beyond the page edge.
        let mut scene = Scene::new(4.0, 4.0);
        scene.commands.push(SceneCommand::PushClip {
            x: 0.0,
            y: 0.0,
            w: 4.0,
            h: 4.0,
        });
        scene.commands.push(SceneCommand::FillRect {
            x: 2.0,
            y: 2.0,
            w: 10.0,
            h: 10.0,
            color: red(),
        });
        scene.commands.push(SceneCommand::PopClip);

        let backend = TinySkiaBackend;
        let img = backend.rasterize(&scene).expect("must not panic or error");
        assert_eq!(img.width, 4);
        assert_eq!(img.height, 4);
        // Pixel inside the overlap region (3,3) should be red.
        assert_eq!(pixel(&img.rgba, img.width, 3, 3), (255, 0, 0, 255));
        // Pixel outside the fill (0,0) should be transparent.
        assert_eq!(pixel(&img.rgba, img.width, 0, 0), (0, 0, 0, 0));
    }

    // ── transparent default ───────────────────────────────────────────────

    #[test]
    fn transparent_default_no_fill() {
        let mut scene = Scene::new(4.0, 4.0);
        scene.commands.push(SceneCommand::PushClip {
            x: 0.0,
            y: 0.0,
            w: 4.0,
            h: 4.0,
        });
        scene.commands.push(SceneCommand::PopClip);

        let backend = TinySkiaBackend;
        let img = backend.rasterize(&scene).expect("must succeed");
        // All pixels must be fully transparent.
        for i in 0..(img.width * img.height) {
            let base = (i * 4) as usize;
            assert_eq!(
                &img.rgba[base..base + 4],
                &[0, 0, 0, 0],
                "pixel {i} must be transparent"
            );
        }
    }

    // ── invalid size ──────────────────────────────────────────────────────

    #[test]
    fn invalid_zero_size_returns_error() {
        let scene = Scene::new(0.0, 0.0);
        let backend = TinySkiaBackend;
        assert!(
            backend.rasterize(&scene).is_err(),
            "zero-size scene must return RenderError"
        );
    }
}
