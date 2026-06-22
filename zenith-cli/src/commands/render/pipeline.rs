//! Shared parse/validate, page-resolution, and hash-verification helpers.

use sha2::{Digest, Sha256};

use zenith_core::{Document, KdlAdapter, KdlSource, validate};

use super::entry::RenderCmdErr;

/// Verify that `bytes` match the `sha256` field declared on an asset.
///
/// `id` is the asset identifier (for error messages); `kind` is a short noun
/// used in error messages (`"asset"` or `"font asset"`).
///
/// Returns `Err` (exit code 2) when:
/// - `sha256` is `None` (no hash declared).
/// - The computed SHA-256 hex digest does not match `sha256` (case-insensitive,
///   trimmed).
pub(super) fn verify_locked_sha256(
    id: &str,
    kind: &str,
    sha256: Option<&str>,
    bytes: &[u8],
) -> Result<(), RenderCmdErr> {
    let declared = sha256.ok_or_else(|| {
        RenderCmdErr::new(format!("--locked: {kind} '{id}' has no declared sha256"), 2)
    })?;
    let hex = format!("{:x}", Sha256::digest(bytes));
    if declared.trim().to_lowercase() != hex {
        return Err(RenderCmdErr::new(
            format!("--locked: {kind} '{id}' sha256 mismatch (declared {declared}, actual {hex})"),
            2,
        ));
    }
    Ok(())
}

/// Parse → validate, returning the parsed [`Document`].
///
/// Returns early with an error if parse fails (exit code 2) or if validation
/// has errors (exit code 1).
pub(super) fn parse_validate(src: &str) -> Result<Document, RenderCmdErr> {
    // Parse ─────────────────────────────────────────────────────────────────
    let doc = KdlAdapter
        .parse(src.as_bytes())
        .map_err(|e| RenderCmdErr::new(format!("error[parse.error]: {}", e.message), 2))?;

    // Validate ───────────────────────────────────────────────────────────────
    let report = validate(&doc);
    if report.has_errors() {
        let msgs: Vec<String> = report
            .diagnostics
            .iter()
            .filter(|d| d.severity == zenith_core::Severity::Error)
            .map(crate::commands::format_error_diag)
            .collect();
        return Err(RenderCmdErr::new(msgs.join("\n"), 1));
    }

    Ok(doc)
}

/// Resolve a 1-based `page` number to a 0-based page index within `doc`.
///
/// Returns `Err` (exit code 2) when the document has no pages or when `page`
/// is outside `1..=pages.len()`.
pub(super) fn resolve_page_index(doc: &Document, page: usize) -> Result<usize, RenderCmdErr> {
    let n = doc.body.pages.len();
    if doc.body.pages.is_empty() || page < 1 || page > n {
        return Err(RenderCmdErr::new(
            format!("page {page} out of range; document has {n} page(s)"),
            2,
        ));
    }
    Ok(page - 1)
}
