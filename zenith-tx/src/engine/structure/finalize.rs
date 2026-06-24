//! `FinalizeRun` application: for each listed page whose candidate-status is
//! "rejected", apply its cleanup-policy ("delete" or "archive"/absent).

use zenith_core::{Diagnostic, Document};

use super::super::record_affected;

pub(in crate::engine) fn apply_finalize_run(
    run_pages: &[String],
    doc: &mut Document,
    diagnostics: &mut Vec<Diagnostic>,
    affected: &mut Vec<String>,
) {
    for id in run_pages {
        // Re-find the page by id on each iteration: deletes shift indices.
        let pos = doc.body.pages.iter().position(|p| p.id == id.as_str());

        let Some(pos) = pos else {
            diagnostics.push(Diagnostic::error(
                "tx.unknown_node",
                format!("finalize_run: page {:?} not found", id),
                None,
                Some(id.clone()),
            ));
            continue;
        };

        // Only rejected pages are acted on; all others are silently skipped.
        let candidate_status = doc.body.pages[pos].candidate_status.clone();
        if candidate_status.as_deref() != Some("rejected") {
            continue;
        }

        let cleanup_policy = doc.body.pages[pos].cleanup_policy.clone();
        match cleanup_policy.as_deref() {
            Some("delete") => {
                doc.body.pages.remove(pos);
                record_affected(id, affected);
            }
            Some("archive") | None => {
                doc.body.pages[pos].workspace_role = Some("archived".to_owned());
                record_affected(id, affected);
            }
            Some(other) => {
                diagnostics.push(Diagnostic::advisory(
                    "tx.noop",
                    format!(
                        "finalize_run: page {:?} has unrecognized cleanup-policy {:?}; \
                         leaving it untouched",
                        id, other
                    ),
                    None,
                    Some(id.clone()),
                ));
            }
        }
    }
}
