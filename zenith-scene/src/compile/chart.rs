//! `chart` node compilation: draws nothing yet.
//!
//! The chart node carries inline series data and a bounding box, but rendering
//! (bar/line/sparkline/pie/donut drawing) is deferred. A chart with no visual
//! props emits nothing and is byte-identical to before; a chart's fill/stroke
//! background panel will be added when the render pass is implemented.
//!
//! Returns `0.0`: charts are absolute-positioned and do not participate in flow
//! layout (same contract as `compile_pattern`).

use zenith_core::{ChartNode, Diagnostic};

use crate::ir::SceneCommand;

use super::NodeCtx;
use super::RenderCtx;

/// Compile a `chart` node.
///
/// Returns `0.0`: charts are absolute-positioned and do not participate in
/// flow layout.
pub(in crate::compile) fn compile_chart(
    chart: &ChartNode,
    _cx: NodeCtx,
    _commands: &mut Vec<SceneCommand>,
    _diagnostics: &mut Vec<Diagnostic>,
    _ctx: RenderCtx,
) -> f64 {
    // Entire chart excluded when visible=false.
    if chart.visible == Some(false) {
        return 0.0;
    }

    // Chart rendering (axes, bars, lines, wedges, …) is deferred. The node is
    // compiled as a no-op placeholder that holds the declared position and data
    // in the document AST. A document with no chart nodes is byte-identical to
    // before (additive guarantee).
    0.0
}
