//! Pure logic for `zenith tx`.
//!
//! The public entry point [`run`] operates entirely on in-memory source text;
//! the caller is responsible for all filesystem I/O and for deciding whether to
//! persist `source_after` (the `--apply` flag lives in `lib.rs`, not here).

use zenith_core::{KdlAdapter, KdlSource};
use zenith_tx::{Transaction, TxResult, TxStatus, run_transaction};

use crate::commands::serialize_pretty;
use crate::json_types::{self, DiagnosticJson, TxOutputJson};

// ── Error type ────────────────────────────────────────────────────────────────

/// An error that prevents a [`TxOutcome`] from being produced.
///
/// Returned for doc-parse failures or transaction-JSON-parse failures.
/// A *rejected* transaction still produces a `TxOutcome` (not a `TxCmdErr`).
#[derive(Debug)]
pub struct TxCmdErr {
    /// Human-readable message.
    pub message: String,
    /// Recommended exit code (always 2 for parse errors).
    pub exit_code: u8,
}

// ── Outcome type ──────────────────────────────────────────────────────────────

/// The computed outcome of a successful transaction run (even a rejected one).
#[derive(Debug)]
pub struct TxOutcome {
    /// The structured result from the engine.
    pub result: TxResult,
    /// Human-readable summary string (ready to print).
    pub human: String,
    /// JSON summary string (ready to print).
    pub json_str: String,
    /// Status-derived exit code: 0 for Accepted/AcceptedWithWarnings, 1 for Rejected.
    pub exit_code: u8,
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Parse the document source and transaction JSON, run the transaction engine,
/// and return a [`TxOutcome`].
///
/// Returns `Err(TxCmdErr { exit_code: 2 })` if either the document or the
/// transaction JSON fails to parse.  A *rejected* transaction is **not** an
/// error at this level — it returns `Ok(TxOutcome { exit_code: 1 })`.
///
/// This function never touches the filesystem.
pub fn run(doc_src: &str, tx_json: &str) -> Result<TxOutcome, TxCmdErr> {
    // Parse document ─────────────────────────────────────────────────────────
    let doc = KdlAdapter.parse(doc_src.as_bytes()).map_err(|e| TxCmdErr {
        message: format!("error[parse.error]: {}", e.message),
        exit_code: 2,
    })?;

    // Parse transaction ──────────────────────────────────────────────────────
    let tx = Transaction::from_json(tx_json).map_err(|e| TxCmdErr {
        message: format!("error[tx.parse]: {}", e.message),
        exit_code: 2,
    })?;

    // Run engine ─────────────────────────────────────────────────────────────
    let result = run_transaction(&doc, &tx).map_err(|e| TxCmdErr {
        message: format!("error[tx.engine]: {}", e.message),
        exit_code: 2,
    })?;

    let exit_code = status_exit_code(&result.status);
    let human = render_human(&result);
    let json_str = render_json(&result);

    Ok(TxOutcome {
        result,
        human,
        json_str,
        exit_code,
    })
}

// ── Output renderers ──────────────────────────────────────────────────────────

/// Render a human-readable summary of the transaction result.
pub fn render_human(result: &TxResult) -> String {
    let status_label = match result.status {
        TxStatus::Accepted => "accepted",
        TxStatus::AcceptedWithWarnings => "accepted (with warnings)",
        TxStatus::Rejected => "rejected",
    };

    let changed = result.source_before != result.source_after;

    let mut out = String::new();
    out.push_str(&format!("status: {}\n", status_label));
    out.push_str(&format!("changed: {}\n", changed));

    if result.affected_node_ids.is_empty() {
        out.push_str("affected: (none)\n");
    } else {
        out.push_str(&format!(
            "affected: {}\n",
            result.affected_node_ids.join(", ")
        ));
    }

    if result.diagnostics.is_empty() {
        out.push_str("diagnostics: (none)");
    } else {
        out.push_str("diagnostics:");
        for d in &result.diagnostics {
            let sev = json_types::severity_str(&d.severity);
            let subject = d
                .subject_id
                .as_deref()
                .map(|s| format!(" ({})", s))
                .unwrap_or_default();
            out.push_str(&format!(
                "\n  {}[{}]{}: {}",
                sev, d.code, subject, d.message
            ));
        }
    }

    out
}

/// Render a JSON summary of the transaction result.
fn render_json(result: &TxResult) -> String {
    let changed = result.source_before != result.source_after;
    let status = match result.status {
        TxStatus::Accepted => "accepted",
        TxStatus::AcceptedWithWarnings => "accepted_with_warnings",
        TxStatus::Rejected => "rejected",
    };
    let out = TxOutputJson {
        schema: "zenith-tx-v1",
        status: status.to_owned(),
        affected: result.affected_node_ids.clone(),
        diagnostics: result
            .diagnostics
            .iter()
            .map(DiagnosticJson::from)
            .collect(),
        changed,
    };
    serialize_pretty(&out)
}

// ── Exit-code helper ──────────────────────────────────────────────────────────

/// Map a `TxStatus` to an exit code.
///
/// `Accepted` and `AcceptedWithWarnings` → 0.  `Rejected` → 1.
fn status_exit_code(status: &TxStatus) -> u8 {
    match status {
        TxStatus::Accepted | TxStatus::AcceptedWithWarnings => 0,
        TxStatus::Rejected => 1,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal document with a text node and a rect node.
    const SMALL_DOC: &str = r##"zenith version=1 {
  project id="proj.tx" name="Tx Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc.tx" title="Tx" {
    page id="pg.tx" w=(px)400 h=(px)300 {
      rect id="box.tx" x=(px)0 y=(px)0 w=(px)400 h=(px)300
      text id="lbl.tx" x=(px)10 y=(px)10 w=(px)200 h=(px)40 {
        span "hello"
      }
    }
  }
}"##;

    // ── 1. Valid set_text_align → Accepted, changed, exit 0 ──────────────────

    #[test]
    fn valid_set_text_align_accepted() {
        let tx_json = r#"{"ops":[{"op":"set_text_align","node":"lbl.tx","align":"center"}]}"#;
        let outcome = run(SMALL_DOC, tx_json).expect("should not be a parse error");

        assert_eq!(outcome.exit_code, 0, "Accepted must yield exit code 0");
        assert_eq!(outcome.result.status, TxStatus::Accepted);

        let changed = outcome.result.source_before != outcome.result.source_after;
        assert!(changed, "source must differ after set_text_align");

        assert!(
            outcome
                .result
                .affected_node_ids
                .contains(&"lbl.tx".to_owned()),
            "affected_node_ids must contain lbl.tx"
        );

        assert!(
            outcome.result.source_after.contains("center"),
            "source_after must contain align=\"center\""
        );
    }

    // ── 2. Unknown node → Rejected, unchanged, exit 1 ────────────────────────

    #[test]
    fn unknown_node_rejected_exit_1() {
        let tx_json = r#"{"ops":[{"op":"set_text_align","node":"no.such.node","align":"center"}]}"#;
        let outcome = run(SMALL_DOC, tx_json).expect("should not be a parse error");

        assert_eq!(outcome.exit_code, 1, "Rejected must yield exit code 1");
        assert_eq!(outcome.result.status, TxStatus::Rejected);

        let changed = outcome.result.source_before != outcome.result.source_after;
        assert!(!changed, "source must not change on rejection");

        assert!(
            outcome.result.affected_node_ids.is_empty(),
            "no nodes should be affected on rejection"
        );
    }

    // ── 3. Malformed tx JSON → Err(exit_code 2) ───────────────────────────────

    #[test]
    fn malformed_tx_json_returns_err_exit_2() {
        let tx_json = r#"{"ops": [THIS IS NOT JSON]}"#;
        let err = run(SMALL_DOC, tx_json).expect_err("malformed JSON must be Err");
        assert_eq!(err.exit_code, 2, "parse error must yield exit code 2");
        assert!(!err.message.is_empty(), "error message must not be empty");
    }

    // ── 4. Malformed doc → Err(exit_code 2) ──────────────────────────────────

    #[test]
    fn malformed_doc_returns_err_exit_2() {
        let tx_json = r#"{"ops":[{"op":"set_text_align","node":"x","align":"center"}]}"#;
        let err = run("not kdl at all {{{", tx_json).expect_err("malformed doc must be Err");
        assert_eq!(err.exit_code, 2, "doc parse error must yield exit code 2");
    }

    // ── 5. JSON output contains schema ───────────────────────────────────────

    #[test]
    fn json_output_contains_schema() {
        let tx_json = r#"{"ops":[{"op":"set_text_align","node":"lbl.tx","align":"center"}]}"#;
        let outcome = run(SMALL_DOC, tx_json).expect("should succeed");
        assert!(
            outcome.json_str.contains("zenith-tx-v1"),
            "JSON output must contain schema field; got: {}",
            outcome.json_str
        );
    }

    // ── 6. Human output contains status line ─────────────────────────────────

    #[test]
    fn human_output_contains_status() {
        let tx_json = r#"{"ops":[{"op":"set_text_align","node":"lbl.tx","align":"center"}]}"#;
        let outcome = run(SMALL_DOC, tx_json).expect("should succeed");
        assert!(
            outcome.human.contains("status:"),
            "human output must contain 'status:'; got: {}",
            outcome.human
        );
    }
}
