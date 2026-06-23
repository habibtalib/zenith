//! `pattern` node compilation: deterministic expansion of a single `motif`
//! template into many copies laid out across the pattern's bounds.
//!
//! Two layouts are supported. A `grid` tiles the motif on a fixed `spacing`
//! lattice (with optional positional `jitter`); a `scatter` places `count`
//! copies at seed-derived positions inside the bounds. Every instance is the
//! same motif node, compiled through [`compile_node`] with a translation offset
//! folded into its [`RenderCtx`] — exactly how a group translates its children —
//! so the motif keeps its own authored geometry and gains the instance offset.
//!
//! Only the motif instances render: the pattern's own visual properties
//! (fill/stroke/shadow/…) are inert here; the bounds box is used solely to clip
//! the instances. Placement is fully deterministic (fixed loop order, integer
//! bit-mixing for jitter/scatter) so the same document yields byte-identical
//! commands on every machine.

use zenith_core::{Diagnostic, PatternNode, Severity, dim_to_px, hash_unit};

use crate::ir::SceneCommand;

use super::NodeCtx;
use super::RenderCtx;
use super::anchor::AnchorMap;
use super::compile_node;
use super::util::resolve_anchored_axis;

/// Different seed mix for the vertical jitter axis so x and y jitter are
/// uncorrelated for the same cell.
const JITTER_Y_SEED_MIX: i64 = 0x5555;

/// Compile a `pattern` node by expanding its motif across the resolved bounds.
///
/// Returns `0.0`: patterns are absolute-positioned and do not participate in
/// flow layout.
pub(in crate::compile) fn compile_pattern(
    pattern: &PatternNode,
    cx: NodeCtx,
    commands: &mut Vec<SceneCommand>,
    diagnostics: &mut Vec<Diagnostic>,
    ctx: RenderCtx,
) -> f64 {
    // Entire pattern excluded when visible=false.
    if pattern.visible == Some(false) {
        return 0.0;
    }

    // Resolve the bounds box in the pattern's LOCAL space (pre-ctx.dx/dy). The
    // instance contexts add `ctx.dx`/`ctx.dy` on top, so the box origin must be
    // local here. Validation already emitted diagnostics for bad geometry; when
    // anything fails to resolve to a usable box we render nothing and do NOT
    // re-emit (avoid duplicate diagnostics).
    let Some((bx, by, bw, bh)) = resolve_bounds(pattern, cx.anchors) else {
        return 0.0;
    };

    // Validate the motif ONCE into scratch buffers at an origin instance ctx. A
    // broken motif must not spam one error per instance: if the scratch carries
    // any error, surface the scratch diagnostics once and render nothing.
    let mut scratch_cmds: Vec<SceneCommand> = Vec::new();
    let mut scratch_diags: Vec<Diagnostic> = Vec::new();
    let probe_ctx = RenderCtx {
        dx: ctx.dx + bx,
        dy: ctx.dy + by,
        ..ctx
    };
    compile_node(
        &pattern.motif,
        cx,
        &mut scratch_cmds,
        &mut scratch_diags,
        probe_ctx,
    );
    if scratch_diags.iter().any(|d| d.severity == Severity::Error) {
        diagnostics.extend(scratch_diags);
        return 0.0;
    }
    // Motif is renderable. Surface any non-error scratch diagnostics
    // (warnings / advisories) exactly once. The scratch commands are not used —
    // each instance is recompiled at its own offset below.
    diagnostics.extend(scratch_diags);

    // Clip every instance to the bounds box (in device space).
    commands.push(SceneCommand::PushClip {
        x: ctx.dx + bx,
        y: ctx.dy + by,
        w: bw,
        h: bh,
    });

    let seed = pattern.seed.unwrap_or(0);

    match pattern.kind.as_str() {
        "grid" => emit_grid(pattern, cx, commands, ctx, (bx, by, bw, bh), seed),
        "scatter" => emit_scatter(pattern, cx, commands, ctx, (bx, by, bw, bh), seed),
        // Any other kind was flagged by validation; render nothing.
        _ => {}
    }

    commands.push(SceneCommand::PopClip);

    0.0
}

/// Resolve the pattern's bounds box `(bx, by, bw, bh)` in LOCAL (pre-ctx) space.
///
/// `w`/`h` must resolve to a positive px value; `x`/`y` default to `0.0` when
/// absent (honoring the anchor map like a leaf node). Returns `None` (render
/// nothing) when the box is unusable. No diagnostics are emitted — validation
/// already covered these cases.
fn resolve_bounds(pattern: &PatternNode, anchors: &AnchorMap) -> Option<(f64, f64, f64, f64)> {
    let w_dim = pattern.w.as_ref()?;
    let h_dim = pattern.h.as_ref()?;
    let bw = dim_to_px(w_dim.value, &w_dim.unit)?;
    let bh = dim_to_px(h_dim.value, &h_dim.unit)?;
    if bw <= 0.0 || bh <= 0.0 {
        return None;
    }

    // Anchor-derived (x, y) fallback, mirroring the leaf compilers. A throwaway
    // diagnostics buffer absorbs any push from the helper: we never surface a
    // geometry diagnostic from here (validation owns that), and x/y default to
    // 0 when neither an explicit value nor an anchor is present.
    let anchor_xy = anchors.get(&pattern.id).copied();
    let mut sink: Vec<Diagnostic> = Vec::new();
    let bx = resolve_anchored_axis(
        "pattern",
        &pattern.id,
        "x",
        pattern.x.as_ref(),
        anchor_xy.map(|(ax, _)| ax),
        pattern.source_span,
        &mut sink,
    )
    .unwrap_or(0.0);
    let by = resolve_anchored_axis(
        "pattern",
        &pattern.id,
        "y",
        pattern.y.as_ref(),
        anchor_xy.map(|(_, ay)| ay),
        pattern.source_span,
        &mut sink,
    )
    .unwrap_or(0.0);

    Some((bx, by, bw, bh))
}

/// Compile one motif instance translated by `(ox, oy)` in LOCAL space. The
/// instance context folds `ctx.dx + ox` / `ctx.dy + oy` into the translation so
/// the motif renders at (its own authored x/y + the instance offset).
///
/// Per-instance diagnostics are routed to a local throwaway buffer; the motif
/// was already validated in `compile_pattern` (and any diagnostics surfaced once)
/// so accumulating them here would only produce duplicates and unbounded growth
/// proportional to instance count.
fn emit_instance(
    pattern: &PatternNode,
    cx: NodeCtx,
    commands: &mut Vec<SceneCommand>,
    ctx: RenderCtx,
    ox: f64,
    oy: f64,
) {
    let inst_ctx = RenderCtx {
        dx: ctx.dx + ox,
        dy: ctx.dy + oy,
        ..ctx
    };
    let mut throwaway: Vec<Diagnostic> = Vec::new();
    compile_node(&pattern.motif, cx, commands, &mut throwaway, inst_ctx);
}

/// Emit grid instances in row-major order across the bounds.
fn emit_grid(
    pattern: &PatternNode,
    cx: NodeCtx,
    commands: &mut Vec<SceneCommand>,
    ctx: RenderCtx,
    bounds: (f64, f64, f64, f64),
    seed: i64,
) {
    let (bx, by, bw, bh) = bounds;

    // Spacing is validated > 0; if it does not resolve to a positive value,
    // render nothing rather than loop forever.
    let Some(s) = pattern
        .spacing
        .as_ref()
        .and_then(|d| dim_to_px(d.value, &d.unit))
        .filter(|&s| s > 0.0)
    else {
        return;
    };

    let jitter = pattern.jitter.unwrap_or(0.0);

    // Row-major: `row` advances down, `col` advances across; a cell is emitted
    // while its base origin is still inside the bounds.
    let mut row: i64 = 0;
    while (row as f64) * s < bh {
        let mut col: i64 = 0;
        while (col as f64) * s < bw {
            let base_x = (col as f64) * s;
            let base_y = (row as f64) * s;
            let (jx, jy) = if jitter > 0.0 {
                let jx = (hash_unit(col, row, seed) * 2.0 - 1.0) * jitter * s;
                let jy = (hash_unit(col, row, seed ^ JITTER_Y_SEED_MIX) * 2.0 - 1.0) * jitter * s;
                (jx, jy)
            } else {
                (0.0, 0.0)
            };
            emit_instance(
                pattern,
                cx,
                commands,
                ctx,
                bx + base_x + jx,
                by + base_y + jy,
            );
            col += 1;
        }
        row += 1;
    }
}

/// Emit scatter instances at seed-derived positions inside the bounds.
fn emit_scatter(
    pattern: &PatternNode,
    cx: NodeCtx,
    commands: &mut Vec<SceneCommand>,
    ctx: RenderCtx,
    bounds: (f64, f64, f64, f64),
    seed: i64,
) {
    let (bx, by, bw, bh) = bounds;

    // Count is validated > 0; clamp defensively so a non-positive value renders
    // nothing instead of misbehaving.
    let count = pattern.count.unwrap_or(0);
    if count <= 0 {
        return;
    }

    // Positions are already seed-randomized, so jitter is not applied to scatter.
    for i in 0..count {
        let ix = hash_unit(i, 0, seed) * bw;
        let iy = hash_unit(i, 1, seed) * bh;
        emit_instance(pattern, cx, commands, ctx, bx + ix, by + iy);
    }
}
