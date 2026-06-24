//! Standalone parser for a `diagnostics { … }` config block.
//!
//! A Zenith config file (global or local) is a small KDL document whose only
//! meaningful node is a top-level `diagnostics { … }` block, written exactly
//! like the in-document policy block:
//!
//! ```text
//! diagnostics {
//!     allow "layout.off_canvas"
//!     deny  "font.local"
//!     warn  "node.unknown_property"
//! }
//! ```
//!
//! This is NOT a full `.zen` document — there is no `zenith` root node, no
//! `project`, no `tokens`. Only the `diagnostics` block is read; any other
//! top-level node is silently ignored for forward-compatibility, mirroring the
//! lenient posture used throughout the document transform. A source with no
//! `diagnostics` node (including an empty source) yields an empty
//! [`DiagnosticPolicy`], which is an identity pass.

use crate::ast::policy::DiagnosticPolicy;
use crate::error::{ParseError, ParseErrorCode};
use crate::parse::transform::transform_diagnostic_policy;

/// Parse a standalone `diagnostics { … }` KDL config block from raw bytes.
///
/// The bytes are decoded and parsed as KDL using the same UTF-8-then-KDL path
/// as the document parser. The first top-level `diagnostics` node is delegated
/// to the shared [`transform_diagnostic_policy`] transform; other top-level
/// nodes are ignored. A missing `diagnostics` node returns
/// [`DiagnosticPolicy::default`].
///
/// # Errors
///
/// Returns a [`ParseError`] if the bytes are not valid UTF-8, are not valid
/// KDL, or if a recognized `allow`/`deny`/`warn` entry is missing its
/// diagnostic-code string argument.
pub fn parse_diagnostic_policy(source: &[u8]) -> Result<DiagnosticPolicy, ParseError> {
    // Step 1: validate UTF-8 (same contract as `KdlAdapter::parse`).
    let text = std::str::from_utf8(source).map_err(|e| {
        ParseError::spanless(
            ParseErrorCode::NotUtf8,
            format!("config source is not valid UTF-8: {e}"),
        )
    })?;

    // Step 2: parse KDL.
    let kdl_doc: kdl::KdlDocument = text.parse().map_err(|e: kdl::KdlError| {
        ParseError::spanless(
            ParseErrorCode::InvalidKdl,
            format!("config KDL parse error: {e}"),
        )
    })?;

    // Step 3: locate the first top-level `diagnostics` node and transform it.
    // Absent → empty policy (identity pass).
    match kdl_doc
        .nodes()
        .iter()
        .find(|n| n.name().value() == "diagnostics")
    {
        Some(node) => transform_diagnostic_policy(node),
        None => Ok(DiagnosticPolicy::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::policy::PolicyVerb;

    #[test]
    fn parses_allow_deny_warn_block() {
        let src = br#"diagnostics {
            allow "layout.off_canvas"
            deny  "font.local"
            warn  "node.unknown_property"
        }"#;
        let policy = parse_diagnostic_policy(src).expect("must parse");
        assert_eq!(policy.entries.len(), 3);
        assert_eq!(
            policy.verb_for("layout.off_canvas"),
            Some(&PolicyVerb::Allow)
        );
        assert_eq!(policy.verb_for("font.local"), Some(&PolicyVerb::Deny));
        assert_eq!(
            policy.verb_for("node.unknown_property"),
            Some(&PolicyVerb::Warn)
        );
    }

    #[test]
    fn empty_source_is_default_policy() {
        let policy = parse_diagnostic_policy(b"").expect("empty must parse");
        assert!(policy.entries.is_empty());
    }

    #[test]
    fn no_diagnostics_node_is_default_policy() {
        // A valid KDL document with unrelated top-level nodes → empty policy.
        let src = br#"something else=1
        other "node""#;
        let policy = parse_diagnostic_policy(src).expect("must parse");
        assert!(policy.entries.is_empty());
    }

    #[test]
    fn malformed_kdl_is_error() {
        let src = b"diagnostics {{{ not valid kdl";
        let err = parse_diagnostic_policy(src).expect_err("must fail");
        assert_eq!(err.code, ParseErrorCode::InvalidKdl);
    }

    #[test]
    fn entry_missing_code_is_error() {
        // `deny` with no quoted code string is a hard parse error.
        let src = br#"diagnostics {
            deny
        }"#;
        let err = parse_diagnostic_policy(src).expect_err("missing code must fail");
        assert_eq!(err.code, ParseErrorCode::InvalidPropertyValue);
    }

    #[test]
    fn last_wins_across_entries() {
        let src = br#"diagnostics {
            deny "node.unknown_property"
            warn "node.unknown_property"
        }"#;
        let policy = parse_diagnostic_policy(src).expect("must parse");
        assert_eq!(
            policy.verb_for("node.unknown_property"),
            Some(&PolicyVerb::Warn)
        );
    }
}
