//! Asset block and asset declaration AST types.

use std::collections::BTreeMap;

use super::Span;
use super::node::UnknownProperty;

/// The kind of an asset — determines how bytes are interpreted by consumers.
///
/// Mirrors `TokenType`: unknown kind strings are preserved as
/// `AssetKind::Unknown(String)` for forward-compat. Parse is infallible;
/// validation emits `asset.invalid_kind` for unrecognized variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetKind {
    /// A raster image (PNG, JPEG, …).
    Image,
    /// An SVG vector graphic.
    Svg,
    /// A font file (TTF, OTF, WOFF2, …).
    Font,
    /// An unrecognized asset kind (forward-compat; version-relative).
    Unknown(String),
}

impl AssetKind {
    /// Parse the asset kind from the `kind` property string. Infallible: an
    /// unrecognized kind is preserved as `AssetKind::Unknown` (forward-compat).
    pub fn from_kind_str(s: &str) -> Self {
        match s {
            "image" => Self::Image,
            "svg" => Self::Svg,
            "font" => Self::Font,
            other => Self::Unknown(other.to_owned()),
        }
    }

    /// Return the canonical string representation of a known kind.
    ///
    /// For `Unknown`, returns the stored string slice.
    pub fn kind_str(&self) -> &str {
        match self {
            Self::Image => "image",
            Self::Svg => "svg",
            Self::Font => "font",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

/// A single asset declaration within an `assets` block.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetDecl {
    /// Globally unique asset ID (e.g. `"asset.logo"`).
    pub id: String,
    /// The asset kind — required.
    pub kind: AssetKind,
    /// Relative path to the asset file — required.
    pub src: String,
    /// Optional SHA-256 hex digest for content integrity.
    pub sha256: Option<String>,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
    /// Forward-compat unknown properties captured during parse.
    pub unknown_props: BTreeMap<String, UnknownProperty>,
}

/// The top-level `assets` block.
///
/// Absent from the document → `AssetBlock::default()` (empty, no error).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AssetBlock {
    /// The ordered list of asset declarations.
    pub assets: Vec<AssetDecl>,
    /// Source span of the `assets { … }` block, when available.
    pub source_span: Option<Span>,
}
