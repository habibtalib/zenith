//! Variant-set block declaration AST types.
//!
//! The top-level `variants` block declares named size/override variants derived
//! from a source page. Each `variant` entry specifies a target page `id`, the
//! `source` page it derives from, the target dimensions `w`/`h`, and an
//! optional list of per-node property overrides. It is a sibling of the
//! `provenance`/`document` blocks. The engine round-trips and validates these
//! records but does NOT act on them in unit V-1; generation is deferred to a
//! later unit.

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
    /// Override for the node's `text` content.
    pub text: Option<String>,
    /// Override for the node's `fill` property (token ref or literal).
    pub fill: Option<PropertyValue>,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
    /// Forward-compat: unrecognized attributes preserved with typed values +
    /// annotations.
    pub unknown_props: BTreeMap<String, UnknownProperty>,
}
