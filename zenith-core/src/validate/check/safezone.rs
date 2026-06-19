//! Page-level safe-zone validation.
//!
//! Authors declare named safe/dead zones on a page (`safe-zone` children). This
//! check compares each direct page child node's bounding box against every
//! declared zone and emits a `safe_zone.violation` ADVISORY when a zone is
//! violated:
//!
//! - **Exclusion** zone → violated when a node OVERLAPS the zone.
//! - **Required** zone → violated when a node has ZERO overlap with the zone.
//!
//! Full-bleed background nodes (whose bbox covers ~the whole page) are exempt,
//! so a page background never trips an exclusion zone.

use crate::ast::document::{Page, SafeZone, SafeZoneType};
use crate::ast::value::dim_to_px;
use crate::diagnostics::Diagnostic;

use super::nodes::{node_bbox, node_id_and_span};

/// Resolve a zone's authored rect to pixels.
///
/// Returns `None` when any dimension is unresolvable; the caller then skips the
/// zone (no panic, no diagnostic).
fn zone_rect_px(zone: &SafeZone) -> Option<(f64, f64, f64, f64)> {
    let x = dim_to_px(zone.x.value, &zone.x.unit)?;
    let y = dim_to_px(zone.y.value, &zone.y.unit)?;
    let w = dim_to_px(zone.w.value, &zone.w.unit)?;
    let h = dim_to_px(zone.h.value, &zone.h.unit)?;
    Some((x, y, w, h))
}

/// Standard AABB overlap test for two `(x, y, w, h)` rects.
fn intersects(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

/// Validate every direct page child against every declared safe-zone.
///
/// Deterministic: zones are iterated in declared order, nodes in child order.
pub(super) fn check_safe_zones(
    page: &Page,
    page_w: f64,
    page_h: f64,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for zone in &page.safe_zones {
        let Some(zone_rect) = zone_rect_px(zone) else {
            // Unresolvable zone dimension — skip this zone entirely.
            continue;
        };

        let label_suffix = match &zone.label {
            Some(label) => format!(" (\"{label}\")"),
            None => String::new(),
        };

        for node in &page.children {
            let Some(node_rect) = node_bbox(node, page_w, page_h) else {
                continue;
            };
            let (nx, ny, nw, nh) = node_rect;

            // Exempt full-bleed background nodes that cover ~the whole page.
            if nx <= 0.0 && ny <= 0.0 && nx + nw >= page_w && ny + nh >= page_h {
                continue;
            }

            let overlaps = intersects(node_rect, zone_rect);
            let (node_id, node_span) = node_id_and_span(node);

            match zone.zone_type {
                SafeZoneType::Exclusion => {
                    if overlaps {
                        diagnostics.push(Diagnostic::advisory(
                            "safe_zone.violation",
                            format!(
                                "node '{}' overlaps exclusion safe-zone '{}'{}",
                                node_id, zone.id, label_suffix
                            ),
                            node_span,
                            Some(node_id.to_owned()),
                        ));
                    }
                }
                SafeZoneType::Required => {
                    if !overlaps {
                        diagnostics.push(Diagnostic::advisory(
                            "safe_zone.violation",
                            format!(
                                "node '{}' falls entirely outside required safe-zone '{}'{}",
                                node_id, zone.id, label_suffix
                            ),
                            node_span,
                            Some(node_id.to_owned()),
                        ));
                    }
                }
            }
        }
    }
}
