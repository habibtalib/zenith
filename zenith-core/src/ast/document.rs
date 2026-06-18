//! Top-level document AST types.

use super::Span;
use super::asset::AssetBlock;
use super::node::Node;
use super::style::StyleBlock;
use super::token::TokenBlock;
use super::value::Dimension;
use super::value::PropertyValue;

/// Metadata for the project.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub author: Option<String>,
}

/// A single page within a document body.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub id: String,
    pub name: Option<String>,
    /// Page width — required.
    pub width: Dimension,
    /// Page height — required.
    pub height: Dimension,
    pub background: Option<PropertyValue>,
    /// Child content nodes in z-order (first = bottommost, last = topmost).
    pub children: Vec<Node>,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
}

/// The `document` child of the root `zenith` node.
///
/// Named `DocumentBody` to avoid clashing with the root `Document` type.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentBody {
    pub id: String,
    pub title: Option<String>,
    pub pages: Vec<Page>,
}

/// The root `zenith` node — the complete parsed `.zen` document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// Must be `1` in v0.
    pub version: u32,
    pub project: Option<Project>,
    /// Asset declarations; empty when the `assets` block is absent.
    pub assets: AssetBlock,
    pub tokens: TokenBlock,
    pub styles: StyleBlock,
    pub body: DocumentBody,
}
