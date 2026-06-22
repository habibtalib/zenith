//! Recipe block declaration AST types.
//!
//! The top-level `recipes` block declares named generative recipes — each
//! `recipe` entry specifies an `id`, a generator `kind`, an optional integer
//! `seed`, optional `generator` version/hash, an optional `bounds` frame id,
//! an optional `detached` link state, typed `param` children, `palette` token
//! children, and `expanded` materialized-node children. It is a sibling of the
//! `variants`/`provenance`/`document` blocks. The engine round-trips and
//! validates these records but does NOT act on them in unit RC-1; expansion is
//! deferred to a later unit.

use std::collections::BTreeMap;

use super::Span;
use super::node::UnknownProperty;
use super::value::PropertyValue;

/// A single recipe declaration within a `recipes` block.
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeDef {
    /// The recipe's own stable id. Required.
    pub id: String,
    /// Generator kind (freeform string, e.g. `"aurora"`, `"scatter"`). Required.
    pub kind: String,
    /// Optional integer seed for deterministic generation.
    pub seed: Option<i64>,
    /// Optional generator version/hash string (e.g. `"aurora@1"`).
    pub generator: Option<String>,
    /// Optional frame/page id this recipe applies within.
    pub bounds: Option<String>,
    /// Optional link/detach state: `Some(false)` = linked (default), `Some(true)` = detached.
    pub detached: Option<bool>,
    /// Typed generation parameters; empty when no `param` children are present.
    pub params: Vec<RecipeParam>,
    /// Palette token ids; each comes from a `palette token="<id>"` child node.
    pub palette: Vec<String>,
    /// Materialized node ids; each comes from an `expanded node="<id>"` child node.
    pub expanded: Vec<String>,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
    /// Forward-compat: unrecognized attributes preserved with typed values +
    /// annotations.
    pub unknown_props: BTreeMap<String, UnknownProperty>,
}

/// A single typed generation parameter within a [`RecipeDef`].
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeParam {
    /// Parameter name. Required.
    pub name: String,
    /// Parameter value (number dimension, token ref, or string literal). Required.
    pub value: PropertyValue,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
    /// Forward-compat: unrecognized attributes preserved with typed values +
    /// annotations.
    pub unknown_props: BTreeMap<String, UnknownProperty>,
}
