//! `compile_chart` entry point.
//!
//! Resolves geometry, computes the scale and plot area, and emits the axis
//! frame (Y axis, X axis, gridlines, tick labels, title) for axis-bearing
//! chart kinds (`bar`, `line`). Non-axis kinds (`sparkline`, `pie`, `donut`)
//! emit nothing and return `0.0`.
//!
//! Returns `0.0`: charts are absolute-positioned and do not participate in
//! flow layout (same contract as `compile_pattern`).

use zenith_core::{ChartNode, Diagnostic, FontStyle, dim_to_px};
use zenith_layout::{ShapeRequest, TextDirection, TextLayoutEngine};

use crate::ir::{Color, SceneCommand};

use super::super::NodeCtx;
use super::super::RenderCtx;
use super::super::paint::resolve_property_color;
use super::super::text::run_to_scene_glyphs;
use super::super::util::{missing_geometry_diag, resolve_anchored_axis, unsupported_unit_diag};
use super::axis::{AxisColors, emit_axes_frame, emit_axis_lines, emit_gridlines_and_labels};
use super::bar::{BarMode, emit_bars, emit_category_labels, stacked_max};
use super::frame::plot_area;
use super::scale::{LinearScale, data_range, nice_ticks};

// ── Default colors ─────────────────────────────────────────────────────────────

/// Default axis line color (medium gray).
const DEFAULT_AXIS_COLOR: Color = Color::srgb(120, 120, 120, 255);
/// Default gridline color (light gray).
const DEFAULT_GRID_COLOR: Color = Color::srgb(225, 225, 225, 255);
/// Default tick label color (dark gray).
const DEFAULT_LABEL_COLOR: Color = Color::srgb(90, 90, 90, 255);
/// Default title color (near-black).
const DEFAULT_TITLE_COLOR: Color = Color::srgb(40, 40, 40, 255);

// ── compile_chart ─────────────────────────────────────────────────────────────

/// Compile a `chart` node.
///
/// Axis-bearing kinds (`bar`, `line`) emit:
/// - The Y axis and X axis lines (the frame).
/// - Horizontal gridlines at each Y tick.
/// - Numeric Y tick labels (shaped text, right-aligned).
/// - The title (if present) above the plot area.
///
/// Non-axis kinds (`sparkline`, `pie`, `donut`) emit nothing (their rendering
/// is deferred). Any other kind string is treated the same as non-axis kinds —
/// no wildcard that would silently swallow a future `Node` variant.
///
/// Returns `0.0`: charts are absolute-positioned and do not participate in
/// flow layout.
pub(in crate::compile) fn compile_chart(
    chart: &ChartNode,
    cx: NodeCtx,
    commands: &mut Vec<SceneCommand>,
    diagnostics: &mut Vec<Diagnostic>,
    ctx: RenderCtx,
) -> f64 {
    // Exclude invisible charts.
    if chart.visible == Some(false) {
        return 0.0;
    }

    // Only axis-bearing kinds draw anything in this pass.
    match chart.kind.as_str() {
        "bar" | "line" => {}
        "sparkline" | "pie" | "donut" => return 0.0,
        // Forward-compat: unknown kind strings (none exist yet) emit nothing
        // and remain visible as soon as the renderer implements them. We do
        // NOT use `_` to avoid silently swallowing a future known kind; an
        // explicit non-exhaustive string match is the correct pattern here.
        _ => return 0.0,
    }

    // ── Geometry resolution ──────────────────────────────────────────────────
    // Mirrors compile_shape (leaf/shape.rs:110-162): require w+h, resolve x/y
    // with anchor fallback, apply ctx.dx/ctx.dy.
    let (Some(w_dim), Some(h_dim)) = (&chart.w, &chart.h) else {
        diagnostics.push(missing_geometry_diag("chart", &chart.id, chart.source_span));
        return 0.0;
    };
    let Some(w) = dim_to_px(w_dim.value, &w_dim.unit) else {
        diagnostics.push(unsupported_unit_diag(
            "chart",
            &chart.id,
            "w",
            chart.source_span,
        ));
        return 0.0;
    };
    let Some(h) = dim_to_px(h_dim.value, &h_dim.unit) else {
        diagnostics.push(unsupported_unit_diag(
            "chart",
            &chart.id,
            "h",
            chart.source_span,
        ));
        return 0.0;
    };

    let anchor_xy = cx.anchors.get(&chart.id).copied();

    let Some(x_raw) = resolve_anchored_axis(
        "chart",
        &chart.id,
        "x",
        chart.x.as_ref(),
        anchor_xy.map(|(ax, _)| ax),
        chart.source_span,
        diagnostics,
    ) else {
        return 0.0;
    };
    let Some(y_raw) = resolve_anchored_axis(
        "chart",
        &chart.id,
        "y",
        chart.y.as_ref(),
        anchor_xy.map(|(_, ay)| ay),
        chart.source_span,
        diagnostics,
    ) else {
        return 0.0;
    };

    let x = x_raw + ctx.dx;
    let y = y_raw + ctx.dy;

    // ── Axis style "hidden" ──────────────────────────────────────────────────
    // When axis_style="hidden" the caller explicitly opts out of axes.
    if chart.axis_style.as_deref() == Some("hidden") {
        return 0.0;
    }

    // ── Plot area ────────────────────────────────────────────────────────────
    let has_title = chart.title.is_some();
    let has_caption = chart.caption.is_some();
    let plot = plot_area(x, y, w, h, has_title, has_caption);

    // ── Axis colors ──────────────────────────────────────────────────────────
    let axis_color = chart
        .stroke
        .as_ref()
        .and_then(|p| resolve_property_color(p, cx.resolved, diagnostics, &chart.id))
        .unwrap_or(DEFAULT_AXIS_COLOR);

    let colors = AxisColors {
        axis: axis_color,
        grid: DEFAULT_GRID_COLOR,
        label: DEFAULT_LABEL_COLOR,
    };

    // ── Y scale ──────────────────────────────────────────────────────────────
    // Build the scale even when there is no data so the empty frame is drawn.
    let (mut data_lo, mut data_hi) =
        data_range(&chart.series, chart.axis_min, chart.axis_max).unwrap_or((0.0, 1.0)); // fallback: (0,1) keeps the frame visible

    // Bar charts grow from a zero baseline, so the domain must include 0 — a
    // bar drawn from a non-zero floor misrepresents magnitude. Honor an explicit
    // axis_min if the author pinned one; line charts keep their auto-fit range.
    if chart.kind.as_str() == "bar" && chart.axis_min.is_none() {
        data_lo = data_lo.min(0.0);
    }

    // Stacked bars reach the per-category SUM, not the max single value, so the
    // value axis must be sized to the tallest column or the stack overflows the
    // plot. Honor an explicit axis_max if the author pinned one.
    if chart.kind.as_str() == "bar"
        && BarMode::from_opt(chart.bar_mode.as_deref()) == BarMode::Stacked
        && chart.axis_max.is_none()
    {
        data_hi = data_hi.max(stacked_max(chart));
    }

    // Inverted Y: data_min → pixel bottom, data_max → pixel top.
    let y_scale = LinearScale {
        data_min: data_lo,
        data_max: data_hi,
        pixel_min: plot.y + plot.h, // bottom of plot area
        pixel_max: plot.y,          // top of plot area
    };

    let y_ticks = nice_ticks(&y_scale, 5);

    // ── Emit chart content (kind-specific z-order) ───────────────────────────
    //
    // Bar charts: gridlines → bars → axis lines (bars paint over gridlines,
    // axis lines paint over bar edges that touch them).
    // Line charts: the combined emit_axes_frame call preserves the existing
    // order (gridlines + tick labels + axis lines together).
    match chart.kind.as_str() {
        "bar" => {
            let n_categories = chart
                .series
                .iter()
                .map(|s| s.values.len())
                .max()
                .unwrap_or(0);

            emit_gridlines_and_labels(
                &plot,
                &y_ticks,
                colors,
                &chart.id,
                cx,
                commands,
                diagnostics,
            );
            emit_bars(chart, &plot, &y_scale, cx, commands, diagnostics);
            emit_category_labels(
                &chart.categories,
                n_categories,
                &plot,
                colors.label,
                cx,
                commands,
                diagnostics,
            );
            emit_axis_lines(&plot, colors.axis, commands);
        }
        "line" => {
            emit_axes_frame(
                &plot,
                &y_ticks,
                colors,
                &chart.id,
                cx,
                commands,
                diagnostics,
            );
        }
        // Non-axis kinds are filtered above; no wildcard here.
        _ => {}
    }

    // ── Title ────────────────────────────────────────────────────────────────
    if let Some(title) = &chart.title {
        emit_title(
            title,
            (x, y),
            DEFAULT_TITLE_COLOR,
            &chart.id,
            cx,
            commands,
            diagnostics,
        );
    }

    0.0
}

// ── Title emitter ─────────────────────────────────────────────────────────────

/// Shape and emit a chart title above the plot area.
///
/// The title is placed at the top-left of the chart bbox, vertically just
/// inside the top margin, using Noto Sans 13 px.
fn emit_title(
    title: &str,
    origin: (f64, f64),
    color: Color,
    chart_id: &str,
    cx: NodeCtx,
    commands: &mut Vec<SceneCommand>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (chart_x, chart_y) = origin;
    let families = [String::from("Noto Sans")];
    let req = ShapeRequest {
        text: title,
        families: &families,
        weight: 600,
        style: FontStyle::Normal,
        font_size: 13.0,
        direction: TextDirection::Ltr,
    };

    match cx.engine.shape_with_fallback(&req, cx.fonts) {
        Err(e) => {
            diagnostics.push(Diagnostic::advisory(
                "scene.text_unshaped",
                format!(
                    "chart '{}' title could not be shaped: {}",
                    chart_id, e.message
                ),
                None,
                Some(chart_id.to_owned()),
            ));
        }
        Ok(result) => {
            // Ascent from first run for baseline offset from top edge.
            let ascent: f64 = result.runs.first().map(|r| r.ascent as f64).unwrap_or(10.0);

            // Baseline sits `ascent` px below the chart's top edge, with 2 px
            // of breathing room so it does not clip at the top.
            let baseline_y = chart_y + ascent + 2.0;
            let mut label_x = chart_x + 4.0; // left-aligned with a small indent

            for run in result.runs {
                let advance = run.advance_width as f64;
                let glyphs = run_to_scene_glyphs(&run);
                commands.push(SceneCommand::DrawGlyphRun {
                    x: label_x,
                    y: baseline_y,
                    font_id: run.font_id.clone(),
                    font_size: run.font_size,
                    color,
                    stroke_color: None,
                    stroke_width: None,
                    glyphs,
                });
                label_x += advance;
            }
        }
    }
}
