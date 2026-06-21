//! Table-node compilation: single-page tables with EXPLICIT and CONTENT-BASED
//! column widths, CONTENT-BASED row heights, and SEPARATE borders only.
//!
//! This unit lays a `table` out as a grid of cells inside its declared
//! `[x, y, w, h]` box, honoring `colspan`/`rowspan` (HTML-table cell flow).
//! AUTO columns (a `column` with no `width`) size to their widest cell's
//! measured natural content; rows size to their tallest cell's wrapped content
//! height at the assigned column width. Both passes reuse the production
//! text-shaping pipeline (`shape_words`/`pack_lines`) via the measurer helpers
//! in [`super::text`]. `border-collapse` is carried but only `"separate"`
//! borders are drawn here.
//!
//! Each cell emits, in order: an optional background `FillRect` (cell.fill or
//! table.fill), then an optional border drawn as four independent `StrokeLine`s
//! (cell or table defaults), then its compiled child content clipped to and
//! translated into the cell content box (cell padding inset), with the cell's
//! `h-align`/`v-align` (overriding the table default) shifting the child within
//! the content box. Opacity cascades (table.opacity × ctx.opacity).

use std::collections::{BTreeMap, BTreeSet};

use zenith_core::{
    Diagnostic, FontProvider, Node, PropertyValue, ResolvedToken, Style, TableNode, dim_to_px,
};
use zenith_layout::RustybuzzEngine;

use crate::ir::SceneCommand;

use super::chain::ChainAssignments;
use super::field::FieldCtx;
use super::paint::resolve_property_color;
use super::text::{
    MeasureEnv, measure_text_natural, measure_text_wrapped_height, resolve_text_families,
};
use super::util::resolve_property_dimension_px;
use super::{ComponentMap, RenderCtx, compile_node};

/// Lower bound (px) a shrunk AUTO column is clamped to, so proportional shrink
/// to fit never collapses a column to zero width (which would hide its border).
const MIN_AUTO_COL_W: f64 = 2.0;

/// One placed cell after the HTML-table occupancy walk: its top-left grid
/// position plus resolved column/row spans. Shared by the width, height, and
/// emission passes so cell placement is byte-identical across all three.
struct PlacedCell<'a> {
    /// 0-based starting row.
    row: usize,
    /// 0-based starting column.
    col: usize,
    /// Column span (≥1, clamped to the grid).
    cs: usize,
    /// Row span (≥1, clamped to the grid).
    rs: usize,
    cell: &'a zenith_core::TableCell,
}

/// Walk the table's rows with a deterministic occupancy grid (HTML-table cell
/// flow honoring `colspan`/`rowspan`) and return every placed cell in emission
/// order. This is the SINGLE placement walk reused by the auto-width pass, the
/// row-height pass, and the emit pass, so a cell occupies byte-identical slots
/// in measurement and rendering.
fn place_cells(table: &TableNode, col_count: usize, row_count: usize) -> Vec<PlacedCell<'_>> {
    let mut placed: Vec<PlacedCell> = Vec::new();
    let mut occupied: BTreeSet<(usize, usize)> = BTreeSet::new();

    for (r, row) in table.rows.iter().enumerate() {
        let mut col_cursor = 0usize;
        for cell in &row.cells {
            while col_cursor < col_count && occupied.contains(&(r, col_cursor)) {
                col_cursor += 1;
            }
            if col_cursor >= col_count {
                break;
            }
            let cs = (cell.colspan.max(1) as usize).min(col_count - col_cursor);
            let rs = (cell.rowspan.max(1) as usize).min(row_count - r);
            for dr in 0..rs {
                for dc in 0..cs {
                    occupied.insert((r + dr, col_cursor + dc));
                }
            }
            placed.push(PlacedCell {
                row: r,
                col: col_cursor,
                cs,
                rs,
                cell,
            });
            col_cursor += cs;
        }
    }
    placed
}

/// Geometry of one placed cell in absolute page pixels (already including the
/// table origin but NOT the cell-padding inset).
struct CellRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn compile_table(
    table: &TableNode,
    resolved: &BTreeMap<String, ResolvedToken>,
    style_map: &BTreeMap<&str, &Style>,
    components: &ComponentMap,
    fonts: &dyn FontProvider,
    engine: &RustybuzzEngine,
    commands: &mut Vec<SceneCommand>,
    diagnostics: &mut Vec<Diagnostic>,
    chains: &ChainAssignments,
    field_ctx: &FieldCtx,
    ctx: RenderCtx,
) {
    // Entire subtree excluded when visible=false (no commands emitted).
    if table.visible == Some(false) {
        return;
    }

    // ── Resolve table geometry ───────────────────────────────────────────
    let (Some(x_dim), Some(y_dim), Some(w_dim), Some(h_dim)) =
        (&table.x, &table.y, &table.w, &table.h)
    else {
        diagnostics.push(Diagnostic::advisory(
            "scene.missing_geometry",
            format!(
                "table '{}' is missing one or more geometry properties (x, y, w, h); skipped",
                table.id
            ),
            table.source_span,
            Some(table.id.clone()),
        ));
        return;
    };
    let (Some(table_x), Some(table_y), Some(table_w), Some(table_h)) = (
        dim_to_px(x_dim.value, &x_dim.unit),
        dim_to_px(y_dim.value, &y_dim.unit),
        dim_to_px(w_dim.value, &w_dim.unit),
        dim_to_px(h_dim.value, &h_dim.unit),
    ) else {
        diagnostics.push(Diagnostic::advisory(
            "scene.missing_geometry",
            format!(
                "table '{}' has an unresolvable geometry unit (x, y, w, h); skipped",
                table.id
            ),
            table.source_span,
            Some(table.id.clone()),
        ));
        return;
    };

    // Absolute page origin (cascade translation applied).
    let origin_x = ctx.dx + table_x;
    let origin_y = ctx.dy + table_y;

    // ── Resolve gap + cell padding (token or literal), default 0 ─────────
    let gap = resolve_property_dimension_px(&table.gap, resolved, 0.0).max(0.0);
    let pad = resolve_property_dimension_px(&table.cell_padding, resolved, 0.0).max(0.0);

    // Opacity cascade.
    let opacity = (table.opacity.unwrap_or(1.0).clamp(0.0, 1.0)) * ctx.opacity;

    // ── Grid dimensions ──────────────────────────────────────────────────
    let col_count = table.columns.len().max(1);
    let row_count = table.rows.len();
    if row_count == 0 {
        // No rows → nothing to draw (the table box itself has no fill in v0).
        return;
    }

    // ── Cell placement (shared occupancy walk) ───────────────────────────
    // Computed ONCE here and reused by the width pass, height pass, and emit
    // pass so a cell occupies byte-identical slots throughout.
    let placed = place_cells(table, col_count, row_count);

    // ── Column widths (CONTENT-BASED auto-sizing) ────────────────────────
    // Explicit `column.width` resolves to its px and is fixed. Each AUTO column
    // (no width) sizes to the widest natural content of the cells that span it
    // (a colspan>1 auto cell divides its natural width across the auto columns
    // it covers). If the natural total overflows the table, only the AUTO
    // columns shrink proportionally (explicit columns stay fixed).
    let mut explicit_w: Vec<Option<f64>> = Vec::with_capacity(col_count);
    for i in 0..col_count {
        let w = table
            .columns
            .get(i)
            .and_then(|c| c.width.as_ref())
            .and_then(|d| dim_to_px(d.value, &d.unit))
            .map(|v| v.max(0.0));
        explicit_w.push(w);
    }
    let sum_explicit: f64 = explicit_w.iter().filter_map(|w| *w).sum();

    // Resolve the node-level font families ONCE per text node we measure (cached
    // by node id) so repeated cells of the same node don't re-probe the provider
    // or re-emit advisories.
    let mut family_cache: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // Shared shaping environment for the per-cell measurers.
    let env = MeasureEnv {
        resolved,
        style_map,
        fonts,
        engine,
    };

    // Natural content width demanded for each AUTO column. Explicit columns keep
    // their resolved px; auto columns accumulate the max demand.
    let mut auto_natural: Vec<f64> = vec![0.0; col_count];
    for pc in &placed {
        // The natural content width this cell needs (widest child + insets).
        let cell_natural = cell_natural_width(pc.cell, pad, &env, diagnostics, &mut family_cache);
        // Which of the columns this cell spans are AUTO? Distribute the cell's
        // natural width equally across them (mirrors the occupancy placement).
        let is_auto = |c: usize| explicit_w.get(c).is_some_and(|w| w.is_none());
        let auto_col_count = (pc.col..pc.col + pc.cs).filter(|&c| is_auto(c)).count();
        if auto_col_count == 0 {
            continue;
        }
        // Subtract the explicit columns the cell also spans from its demand, then
        // split the remainder across the auto columns it covers.
        let explicit_in_span: f64 = (pc.col..pc.col + pc.cs)
            .filter_map(|c| explicit_w.get(c).copied().flatten())
            .sum();
        let span_gaps = gap * (pc.cs.saturating_sub(1)) as f64;
        let auto_demand = (cell_natural - explicit_in_span - span_gaps).max(0.0);
        let per_col = auto_demand / auto_col_count as f64;
        for c in (pc.col..pc.col + pc.cs).filter(|&c| is_auto(c)) {
            if let Some(slot) = auto_natural.get_mut(c) {
                *slot = slot.max(per_col);
            }
        }
    }

    // Assemble natural column widths, then shrink AUTO columns to fit if needed.
    let total_gap_w = gap * (col_count.saturating_sub(1)) as f64;
    let avail_auto = (table_w - sum_explicit - total_gap_w - 2.0 * pad).max(0.0);
    let sum_auto_natural: f64 = explicit_w
        .iter()
        .enumerate()
        .filter(|(_, w)| w.is_none())
        .map(|(i, _)| auto_natural.get(i).copied().unwrap_or(0.0))
        .sum();
    // Shrink factor ≤ 1 applied to AUTO columns only when they overflow.
    let auto_scale = if sum_auto_natural > avail_auto && sum_auto_natural > 0.0 {
        avail_auto / sum_auto_natural
    } else {
        1.0
    };
    let col_widths: Vec<f64> = explicit_w
        .iter()
        .enumerate()
        .map(|(i, w)| match w {
            Some(px) => px.max(0.0),
            None => {
                let nat = auto_natural.get(i).copied().unwrap_or(0.0) * auto_scale;
                // Clamp shrunk columns to a small minimum (never below 0).
                if auto_scale < 1.0 {
                    nat.max(MIN_AUTO_COL_W)
                } else {
                    nat.max(0.0)
                }
            }
        })
        .collect();

    // Left edge of each column (content-box left = origin + pad).
    let mut col_left: Vec<f64> = Vec::with_capacity(col_count);
    let mut cursor = origin_x + pad;
    for w in &col_widths {
        col_left.push(cursor);
        cursor += w + gap;
    }

    // ── Row heights (CONTENT-BASED) ──────────────────────────────────────
    // After column widths are final, each row's height is the max over its
    // cells of the cell's wrapped content height at its assigned column width
    // plus the two padding insets. A rowspan>1 cell distributes its measured
    // height across the rows it covers (max per row), so totals stay
    // consistent. If the natural total overflows `table_h`, rows shrink
    // proportionally; if it underflows, rows stay top-aligned (no stretch).
    let mut row_natural: Vec<f64> = vec![0.0; row_count];
    for pc in &placed {
        // Assigned content width for this cell = summed spanned column widths +
        // interior gaps − the two padding insets.
        let mut span_w = 0.0;
        for c in pc.col..pc.col + pc.cs {
            span_w += col_widths.get(c).copied().unwrap_or(0.0);
        }
        span_w += gap * (pc.cs.saturating_sub(1)) as f64;
        let content_w = (span_w - 2.0 * pad).max(0.0);

        let cell_h = cell_content_height(
            pc.cell,
            content_w,
            pad,
            &env,
            diagnostics,
            &mut family_cache,
        );
        // Distribute across the spanned rows (max per row). `pc.rs` is ≥1 by
        // construction in `place_cells`, so the division is always well-defined.
        let per_row = cell_h / pc.rs as f64;
        for dr in 0..pc.rs {
            if let Some(slot) = row_natural.get_mut(pc.row + dr) {
                *slot = slot.max(per_row);
            }
        }
    }

    let total_gap_h = gap * (row_count.saturating_sub(1)) as f64;
    let avail_h = (table_h - total_gap_h - 2.0 * pad).max(0.0);
    let sum_rows: f64 = row_natural.iter().sum();
    let row_scale = if sum_rows > avail_h && sum_rows > 0.0 {
        avail_h / sum_rows
    } else {
        1.0
    };
    let row_heights: Vec<f64> = row_natural
        .iter()
        .map(|h| (h * row_scale).max(0.0))
        .collect();

    let mut row_top: Vec<f64> = Vec::with_capacity(row_count);
    let mut rcursor = origin_y + pad;
    for h in &row_heights {
        row_top.push(rcursor);
        rcursor += h + gap;
    }

    // ── Cell emission (reusing the shared placement walk) ────────────────
    for pc in &placed {
        // Cell rect: from column `pc.col` left to the right edge of the last
        // spanned column (including interior gaps); similarly for rows.
        let left = col_left.get(pc.col).copied().unwrap_or(origin_x + pad);
        let mut span_w = 0.0;
        for c in pc.col..pc.col + pc.cs {
            span_w += col_widths.get(c).copied().unwrap_or(0.0);
        }
        span_w += gap * (pc.cs.saturating_sub(1)) as f64;

        let top = row_top.get(pc.row).copied().unwrap_or(origin_y + pad);
        let mut span_h = 0.0;
        for dr in 0..pc.rs {
            span_h += row_heights.get(pc.row + dr).copied().unwrap_or(0.0);
        }
        span_h += gap * (pc.rs.saturating_sub(1)) as f64;

        let rect = CellRect {
            x: left,
            y: top,
            w: span_w.max(0.0),
            h: span_h.max(0.0),
        };

        emit_cell(
            table,
            pc.cell,
            &rect,
            pad,
            opacity,
            resolved,
            style_map,
            components,
            fonts,
            engine,
            commands,
            diagnostics,
            chains,
            field_ctx,
            ctx,
        );
    }
}

/// Natural (unwrapped) content width a cell demands, in pixels: the max over the
/// cell's children of the child's natural width, plus the two cell-padding
/// insets. A `Node::Text` child measures via the shared text pipeline; any other
/// kind uses its declared box width (or 0). `family_cache` memoizes per-text-node
/// family resolution so a repeated node id is not re-probed/re-diagnosed.
fn cell_natural_width(
    cell: &zenith_core::TableCell,
    pad: f64,
    env: &MeasureEnv,
    diagnostics: &mut Vec<Diagnostic>,
    family_cache: &mut BTreeMap<String, Vec<String>>,
) -> f64 {
    let mut widest = 0.0_f64;
    for child in &cell.children {
        let w = match child {
            Node::Text(t) => {
                let families = cached_families(
                    t,
                    env.resolved,
                    env.style_map,
                    env.fonts,
                    diagnostics,
                    family_cache,
                );
                measure_text_natural(t, families, env, diagnostics).unwrap_or(0.0)
            }
            other => child_declared_box(other).0.unwrap_or(0.0),
        };
        widest = widest.max(w);
    }
    widest + 2.0 * pad
}

/// Wrapped content height a cell demands at content width `content_w`, in pixels:
/// the max over the cell's children of the child's height, plus the two
/// cell-padding insets. A `Node::Text` child measures its wrapped block height at
/// `content_w` via the shared pipeline; any other kind uses its declared box
/// height (or 0).
fn cell_content_height(
    cell: &zenith_core::TableCell,
    content_w: f64,
    pad: f64,
    env: &MeasureEnv,
    diagnostics: &mut Vec<Diagnostic>,
    family_cache: &mut BTreeMap<String, Vec<String>>,
) -> f64 {
    let mut tallest = 0.0_f64;
    for child in &cell.children {
        let h = match child {
            Node::Text(t) => {
                let families = cached_families(
                    t,
                    env.resolved,
                    env.style_map,
                    env.fonts,
                    diagnostics,
                    family_cache,
                );
                measure_text_wrapped_height(t, content_w, families, env, diagnostics).unwrap_or(0.0)
            }
            other => child_declared_box(other).1.unwrap_or(0.0),
        };
        tallest = tallest.max(h);
    }
    tallest + 2.0 * pad
}

/// Resolve (and memoize) a text node's font families through [`resolve_text_families`].
/// The advisory inside that helper fires at most once per node id because a cache
/// hit skips the resolution entirely.
fn cached_families<'c>(
    text: &zenith_core::TextNode,
    resolved: &BTreeMap<String, ResolvedToken>,
    style_map: &BTreeMap<&str, &Style>,
    fonts: &dyn FontProvider,
    diagnostics: &mut Vec<Diagnostic>,
    family_cache: &'c mut BTreeMap<String, Vec<String>>,
) -> &'c [String] {
    family_cache
        .entry(text.id.clone())
        .or_insert_with(|| resolve_text_families(text, resolved, style_map, fonts, diagnostics))
}

/// Emit one cell: background fill, separate border, and clipped/aligned content.
#[allow(clippy::too_many_arguments)]
fn emit_cell(
    table: &TableNode,
    cell: &zenith_core::TableCell,
    rect: &CellRect,
    pad: f64,
    opacity: f64,
    resolved: &BTreeMap<String, ResolvedToken>,
    style_map: &BTreeMap<&str, &Style>,
    components: &ComponentMap,
    fonts: &dyn FontProvider,
    engine: &RustybuzzEngine,
    commands: &mut Vec<SceneCommand>,
    diagnostics: &mut Vec<Diagnostic>,
    chains: &ChainAssignments,
    field_ctx: &FieldCtx,
    ctx: RenderCtx,
) {
    // ── Background fill: cell.fill else table.fill (token color) ─────────
    let fill_prop: Option<&PropertyValue> = cell.fill.as_ref().or(table.fill.as_ref());
    if let Some(prop) = fill_prop
        && let Some(mut color) = resolve_property_color(prop, resolved, diagnostics, &table.id)
    {
        color.a = (color.a as f64 * opacity).round() as u8;
        commands.push(SceneCommand::FillRect {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
            color,
        });
    }

    // ── Separate border: each cell draws its own four edges independently ─
    let border_prop: Option<&PropertyValue> = cell.border.as_ref().or(table.border.as_ref());
    if let Some(prop) = border_prop
        && let Some(mut color) = resolve_property_color(prop, resolved, diagnostics, &table.id)
    {
        color.a = (color.a as f64 * opacity).round() as u8;
        // Width: cell.border-width else table.border-width else 1px.
        let bw_prop = cell
            .border_width
            .clone()
            .or_else(|| table.border_width.clone());
        let bw = resolve_property_dimension_px(&bw_prop, resolved, 1.0).max(0.0);
        if bw > 0.0 {
            let x0 = rect.x;
            let y0 = rect.y;
            let x1 = rect.x + rect.w;
            let y1 = rect.y + rect.h;
            // Four edges as independent stroke lines (centered stroke).
            for (ax, ay, bx, by) in [
                (x0, y0, x1, y0), // top
                (x0, y1, x1, y1), // bottom
                (x0, y0, x0, y1), // left
                (x1, y0, x1, y1), // right
            ] {
                commands.push(SceneCommand::StrokeLine {
                    x1: ax,
                    y1: ay,
                    x2: bx,
                    y2: by,
                    color,
                    stroke_width: bw,
                    stroke_dash: None,
                    stroke_gap: None,
                    stroke_linecap: None,
                });
            }
        }
    }

    // ── Content box (cell padding inset) ─────────────────────────────────
    let content_x = rect.x + pad;
    let content_y = rect.y + pad;
    let content_w = (rect.w - 2.0 * pad).max(0.0);
    let content_h = (rect.h - 2.0 * pad).max(0.0);

    // Alignment offsets (cell override else table default). Horizontal shifts
    // the child column within the content width; vertical within its height.
    let h_align = cell
        .h_align
        .as_deref()
        .or(table.h_align.as_deref())
        .unwrap_or("start");
    let v_align = cell
        .v_align
        .as_deref()
        .or(table.v_align.as_deref())
        .unwrap_or("top");

    // Clip cell content to the content box, then compile each child with a
    // RenderCtx translated to the content-box origin (plus the alignment
    // offset) so authored coordinate (0,0) lands at the cell's content corner.
    commands.push(SceneCommand::PushClip {
        x: content_x,
        y: content_y,
        w: content_w,
        h: content_h,
    });

    for child in &cell.children {
        // Per-child alignment: shift by the slack between the content box and
        // the child's declared width/height. A child with no declared box (or
        // align="start"/"top") gets a zero offset.
        let (cw, ch) = child_declared_box(child);
        let dx_align = match h_align {
            "center" => ((content_w - cw.unwrap_or(content_w)) / 2.0).max(0.0),
            "end" => (content_w - cw.unwrap_or(content_w)).max(0.0),
            _ => 0.0,
        };
        let dy_align = match v_align {
            "middle" => ((content_h - ch.unwrap_or(content_h)) / 2.0).max(0.0),
            "bottom" => (content_h - ch.unwrap_or(content_h)).max(0.0),
            _ => 0.0,
        };
        let child_ctx = RenderCtx {
            opacity,
            dx: content_x + dx_align,
            dy: content_y + dy_align,
            baseline_grid: ctx.baseline_grid,
        };
        let _ = compile_node(
            child,
            resolved,
            style_map,
            components,
            fonts,
            engine,
            commands,
            diagnostics,
            chains,
            field_ctx,
            child_ctx,
        );
    }

    commands.push(SceneCommand::PopClip);
}

/// The declared `(w, h)` of a cell child in pixels, when the kind carries a
/// box and the dimensions resolve. Used to compute alignment slack. Kinds
/// without a resolvable box yield `(None, None)`.
fn child_declared_box(node: &zenith_core::Node) -> (Option<f64>, Option<f64>) {
    use zenith_core::Node;
    let px =
        |d: &Option<zenith_core::Dimension>| d.as_ref().and_then(|d| dim_to_px(d.value, &d.unit));
    match node {
        Node::Rect(n) => (px(&n.w), px(&n.h)),
        Node::Ellipse(n) => (px(&n.w), px(&n.h)),
        Node::Text(n) => (px(&n.w), px(&n.h)),
        Node::Code(n) => (px(&n.w), px(&n.h)),
        Node::Image(n) => (px(&n.w), px(&n.h)),
        Node::Frame(n) => (px(&n.w), px(&n.h)),
        Node::Group(n) => (px(&n.w), px(&n.h)),
        Node::Field(n) => (px(&n.w), px(&n.h)),
        Node::Toc(n) => (px(&n.w), px(&n.h)),
        Node::Table(n) => (px(&n.w), px(&n.h)),
        Node::Line(_)
        | Node::Polygon(_)
        | Node::Polyline(_)
        | Node::Instance(_)
        | Node::Footnote(_)
        | Node::Unknown(_) => (None, None),
    }
}
