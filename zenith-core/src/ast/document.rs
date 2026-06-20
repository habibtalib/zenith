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
    /// Optional uniform print-bleed margin applied to all four sides. When this
    /// resolves to a positive pixel value `b`, the rendered media box expands to
    /// `(width + 2b) × (height + 2b)`, all page content shifts into the inner
    /// trim box `[b, b, width, height]`, the background fills the entire media
    /// box (bleeding off the trim edge), and crop/trim marks are auto-drawn in
    /// the bleed margin at the four trim corners. `None` or a non-positive value
    /// renders byte-identically to a page with no bleed.
    pub bleed: Option<Dimension>,
    /// Book live-area margin (gutter side). With document `mirror_margins=true`
    /// this is the BINDING-side margin: it sits on the LEFT for a recto (odd,
    /// 1-based) page and on the RIGHT for a verso (even) page. Without mirroring
    /// it is treated uniformly as the left margin. `None` → no inner margin.
    ///
    /// Margins are v0 METADATA + VALIDATION ONLY: they describe the intended
    /// live area and drive the `margin.violation` advisory, but they do NOT
    /// auto-reposition arbitrary page nodes (that is the job of master pages /
    /// flow frames). See [`super::super::validate`]'s margin check.
    pub margin_inner: Option<Dimension>,
    /// Book live-area margin (fore-edge side). The mirror of [`Page::margin_inner`]:
    /// with `mirror_margins=true` it sits on the RIGHT for a recto page and on
    /// the LEFT for a verso page; without mirroring it is the right margin.
    /// `None` → no outer margin. Metadata + validation only (see `margin_inner`).
    pub margin_outer: Option<Dimension>,
    /// Book live-area top margin. `None` → no top margin. Metadata + validation
    /// only (see [`Page::margin_inner`]).
    pub margin_top: Option<Dimension>,
    /// Book live-area bottom margin. `None` → no bottom margin. Metadata +
    /// validation only (see [`Page::margin_inner`]).
    pub margin_bottom: Option<Dimension>,
    /// Author-declared safe/dead zones for this page. These are not rendering
    /// nodes; the validator checks page children against them.
    pub safe_zones: Vec<SafeZone>,
    /// Author-declared fold-line positions for this page (tri-fold/bi-fold
    /// print). These are non-printing page metadata, not rendering nodes; the
    /// validator advises when content crosses a fold line.
    pub folds: Vec<Fold>,
    /// Optional master-page reference. When `Some(id)` names a declared
    /// [`MasterDef`], the master's nodes (running heads, folios, TOC refs) are
    /// projected UNDER this page's own children at compile time — the master's
    /// [`Field`](super::node::Node::Field) nodes are resolved against this page's
    /// index/parity. An unknown reference is a hard `master.unknown_reference`
    /// validation error. `None` → the page has no master (renders as before).
    pub master: Option<String>,
    /// Child content nodes in z-order (first = bottommost, last = topmost).
    pub children: Vec<Node>,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
}

/// The kind of a [`SafeZone`].
#[derive(Debug, Clone, PartialEq)]
pub enum SafeZoneType {
    /// Content must NOT overlap this zone (e.g. a platform UI dead zone).
    Exclusion,
    /// Content must overlap this zone (e.g. a guaranteed-visible region).
    Required,
}

/// A named safe/dead zone declared on a [`Page`].
///
/// Declared as a `safe-zone` child of a `page`; it is a sibling of rendering
/// nodes but is itself not rendered.
#[derive(Debug, Clone, PartialEq)]
pub struct SafeZone {
    pub id: String,
    pub zone_type: SafeZoneType,
    pub x: Dimension,
    pub y: Dimension,
    pub w: Dimension,
    pub h: Dimension,
    pub label: Option<String>,
    pub source_span: Option<Span>,
}

/// A non-printing fold-line position declared on a [`Page`].
///
/// Declared as a `fold` child of a `page`; it is a sibling of rendering nodes
/// but is itself never rendered. A vertical fold has an `x` position; a
/// horizontal fold has a `y` position. Used for tri-fold / bi-fold print
/// layouts so the validator can advise when content crosses a fold line.
#[derive(Debug, Clone, PartialEq)]
pub struct Fold {
    pub id: String,
    /// `"vertical"` (position is an x coordinate) or `"horizontal"` (position
    /// is a y coordinate). Any other / absent value defaults to `"vertical"`.
    pub orientation: String,
    /// The fold-line position: x for a vertical fold, y for a horizontal fold.
    /// `None` when the author omitted `position`.
    pub position: Option<Dimension>,
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

/// A reusable component definition: a named child-node subtree declared once
/// (in the document-level `components` block) and instanced into multiple places
/// via [`Node::Instance`](super::node::Node::Instance).
///
/// Declared as `component id="logo.block" { <any child nodes> }`. The component's
/// child node ids are LOCAL to the component: they are validated for uniqueness
/// only WITHIN the component, not globally, and they are prefixed with the
/// instance id when an instance is expanded at compile time. The `component` id
/// itself participates in the global id-uniqueness set.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentDef {
    pub id: String,
    /// The component's child nodes in source order (the reusable subtree).
    pub children: Vec<super::node::Node>,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
}

/// A reusable master-page definition: a named child-node subtree declared once
/// (in the document-level `masters` block) and projected onto every [`Page`]
/// whose `master` attribute names it.
///
/// Declared as `master id="m.body" { <any child nodes, incl. field nodes> }`.
/// Structurally mirrors [`ComponentDef`]: the master's child node ids are LOCAL
/// to the master (validated for uniqueness only WITHIN the master) and are
/// prefixed with the page id when the master is projected at compile time. The
/// `master` id itself participates in the global id-uniqueness set.
///
/// Unlike a component, a master is not instanced explicitly: a page opts in via
/// `page ... master="m.body"`, and the master's [`Field`](super::node::Node::Field)
/// nodes are resolved against that page's index/parity/live-area at compile time.
#[derive(Debug, Clone, PartialEq)]
pub struct MasterDef {
    pub id: String,
    /// The master's child nodes in source order (the projected subtree).
    pub children: Vec<super::node::Node>,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
}

/// The root `zenith` node — the complete parsed `.zen` document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// Must be `1` in v0.
    pub version: u32,
    /// Declared export color space: `Some("srgb")` (default) or `Some("cmyk")`.
    /// `None` when the author omitted the `colorspace` attribute. In v0 this is
    /// informational export metadata only — it does NOT change PNG output (the
    /// PNG is always sRGB); a future PDF backend consults it. An invalid value
    /// is preserved here verbatim and surfaced as a validation warning.
    pub colorspace: Option<String>,
    /// Mirrored book margins toggle. `Some(true)` → page margins mirror by page
    /// parity (recto = odd 1-based page → inner margin on LEFT; verso = even →
    /// inner margin on RIGHT). `Some(false)` or `None` (default) → margins are
    /// uniform (inner = left, outer = right on every page). This only affects
    /// how [`Page::margin_inner`]/[`Page::margin_outer`] are interpreted by the
    /// `margin.violation` validation advisory; it is metadata, not layout.
    pub mirror_margins: Option<bool>,
    /// Declared page progression for export: `Some("ltr")` (default) or
    /// `Some("rtl")` (right-to-left book page order). `None` when the author
    /// omitted the attribute. v0: metadata for export (e.g. a PDF
    /// `/ViewerPreferences /Direction /R2L`); it does NOT change page render
    /// order or PNG output. An invalid value is preserved verbatim and surfaced
    /// as a validation warning.
    pub page_progression: Option<String>,
    pub project: Option<Project>,
    /// Asset declarations; empty when the `assets` block is absent.
    pub assets: AssetBlock,
    pub tokens: TokenBlock,
    pub styles: StyleBlock,
    /// Reusable component definitions; empty when the `components` block is
    /// absent. Instanced via [`Node::Instance`](super::node::Node::Instance).
    pub components: Vec<ComponentDef>,
    /// Reusable master-page definitions; empty when the `masters` block is
    /// absent. Projected onto pages via [`Page::master`].
    pub masters: Vec<MasterDef>,
    pub body: DocumentBody,
}
