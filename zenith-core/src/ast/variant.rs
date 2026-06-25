//! Variant-set block declaration AST types.
//!
//! The top-level `variants` block declares named size/override variants derived
//! from a source page. Each `variant` entry specifies a target page `id`, the
//! `source` page it derives from, the target dimensions `w`/`h`, and an
//! optional list of per-node property overrides. It is a sibling of the
//! `provenance`/`document` blocks. Core round-trips and validates these records;
//! variant generation itself is performed by the CLI engine (`zenith variant`).

use std::collections::BTreeMap;

use super::Span;
use super::node::UnknownProperty;
use super::value::{Dimension, PropertyValue};

/// A single variant declaration within a `variants` block.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantDef {
    /// The variant's own stable id. Required.
    pub id: String,
    /// The canonical page id this variant derives from. Required; existence is
    /// validated later, not by the parser.
    pub source: String,
    /// Target page width for this variant. Required.
    pub w: Dimension,
    /// Target page height for this variant. Required.
    pub h: Dimension,
    /// Per-node property overrides; empty when no `override` children are
    /// present.
    pub overrides: Vec<VariantOverride>,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
    /// Forward-compat: unrecognized attributes preserved with typed values +
    /// annotations.
    pub unknown_props: BTreeMap<String, UnknownProperty>,
}

/// A single per-node property override within a [`VariantDef`].
#[derive(Debug, Clone, PartialEq)]
pub struct VariantOverride {
    /// The target node id within the source page. Required.
    pub node: String,
    /// Override for the node's `visible` property.
    pub visible: Option<bool>,
    /// Override for the node's left edge (`x`), in document pixels or points.
    pub x: Option<Dimension>,
    /// Override for the node's top edge (`y`), in document pixels or points.
    pub y: Option<Dimension>,
    /// Override for the node's width (`w`), in document pixels or points.
    pub w: Option<Dimension>,
    /// Override for the node's height (`h`), in document pixels or points.
    pub h: Option<Dimension>,
    /// Override for the node's `fill` property (token ref or literal).
    pub fill: Option<PropertyValue>,
    /// Override for the node's `text` content.
    pub text: Option<String>,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
    /// Forward-compat: unrecognized attributes preserved with typed values +
    /// annotations.
    pub unknown_props: BTreeMap<String, UnknownProperty>,
}
