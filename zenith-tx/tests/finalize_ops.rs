mod common;
use common::*;
use zenith_tx::{Op, Permissions, Transaction, TxStatus, run_transaction};

/// Multi-page doc with candidate pages carrying various statuses and policies.
///
/// Pages:
/// - pg.rejected.delete   — candidate-status="rejected", cleanup-policy="delete"
/// - pg.rejected.archive  — candidate-status="rejected", cleanup-policy="archive"
/// - pg.rejected.nopolicy — candidate-status="rejected", no cleanup-policy
/// - pg.rejected.weird    — candidate-status="rejected", cleanup-policy="weird"
/// - pg.selected          — candidate-status="selected"
/// - pg.draft             — candidate-status="draft"
const FINALIZE_DOC: &str = r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" {
    token id="color.a" type="color" value="#ff0000"
  }
  styles { }
  document id="doc1" title="T" {
    page id="pg.rejected.delete" w=(px)400 h=(px)300 candidate-status="rejected" cleanup-policy="delete" {
      rect id="r1" x=(px)10 y=(px)20 w=(px)80 h=(px)60 fill=(token)"color.a"
    }
    page id="pg.rejected.archive" w=(px)400 h=(px)300 candidate-status="rejected" cleanup-policy="archive" {
      rect id="r2" x=(px)10 y=(px)20 w=(px)80 h=(px)60 fill=(token)"color.a"
    }
    page id="pg.rejected.nopolicy" w=(px)400 h=(px)300 candidate-status="rejected" {
      rect id="r3" x=(px)10 y=(px)20 w=(px)80 h=(px)60 fill=(token)"color.a"
    }
    page id="pg.rejected.weird" w=(px)400 h=(px)300 candidate-status="rejected" cleanup-policy="weird" {
      rect id="r4" x=(px)10 y=(px)20 w=(px)80 h=(px)60 fill=(token)"color.a"
    }
    page id="pg.selected" w=(px)400 h=(px)300 candidate-status="selected" {
      rect id="r5" x=(px)10 y=(px)20 w=(px)80 h=(px)60 fill=(token)"color.a"
    }
    page id="pg.draft" w=(px)400 h=(px)300 candidate-status="draft" {
      rect id="r6" x=(px)10 y=(px)20 w=(px)80 h=(px)60 fill=(token)"color.a"
    }
  }
}"##;

// ── finalize_run: delete policy ───────────────────────────────────────────────

/// A rejected page with cleanup-policy="delete" is removed from source_after.
#[test]
fn finalize_run_deletes_rejected_page_with_delete_policy() {
    let doc = parse(FINALIZE_DOC);
    let tx = Transaction {
        ops: vec![Op::FinalizeRun {
            run_pages: vec!["pg.rejected.delete".to_owned()],
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; diagnostics: {:?}",
        result.diagnostics
    );

    // The deleted page must not appear in source_after.
    assert!(
        !result.source_after.contains("pg.rejected.delete"),
        "deleted page must not be in source_after:\n{}",
        result.source_after
    );

    // affected contains the page id.
    assert!(
        result
            .affected_node_ids
            .contains(&"pg.rejected.delete".to_owned()),
        "pg.rejected.delete must be in affected_node_ids: {:?}",
        result.affected_node_ids
    );
}

// ── finalize_run: archive policy ─────────────────────────────────────────────

/// A rejected page with cleanup-policy="archive" gets workspace-role="archived".
#[test]
fn finalize_run_archives_rejected_page_with_archive_policy() {
    let doc = parse(FINALIZE_DOC);
    let tx = Transaction {
        ops: vec![Op::FinalizeRun {
            run_pages: vec!["pg.rejected.archive".to_owned()],
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; diagnostics: {:?}",
        result.diagnostics
    );

    // Page must still be present (not deleted).
    assert!(
        result.source_after.contains("pg.rejected.archive"),
        "archived page must remain in source_after:\n{}",
        result.source_after
    );

    // workspace-role="archived" must appear on that page.
    assert!(
        result.source_after.contains("workspace-role=\"archived\""),
        "workspace-role=archived must be present in source_after:\n{}",
        result.source_after
    );

    // affected contains the page id.
    assert!(
        result
            .affected_node_ids
            .contains(&"pg.rejected.archive".to_owned()),
        "pg.rejected.archive must be in affected_node_ids: {:?}",
        result.affected_node_ids
    );
}

// ── finalize_run: absent policy defaults to archive ───────────────────────────

/// A rejected page with no cleanup-policy defaults to archived (not deleted).
#[test]
fn finalize_run_archives_rejected_page_when_policy_absent() {
    let doc = parse(FINALIZE_DOC);
    let tx = Transaction {
        ops: vec![Op::FinalizeRun {
            run_pages: vec!["pg.rejected.nopolicy".to_owned()],
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; diagnostics: {:?}",
        result.diagnostics
    );

    // Page must still be present (not deleted).
    assert!(
        result.source_after.contains("pg.rejected.nopolicy"),
        "page must remain in source_after:\n{}",
        result.source_after
    );

    // workspace-role="archived" must be set.
    assert!(
        result.source_after.contains("workspace-role=\"archived\""),
        "workspace-role=archived must be present in source_after:\n{}",
        result.source_after
    );

    // affected contains the page id.
    assert!(
        result
            .affected_node_ids
            .contains(&"pg.rejected.nopolicy".to_owned()),
        "pg.rejected.nopolicy must be in affected_node_ids: {:?}",
        result.affected_node_ids
    );
}

// ── finalize_run: non-rejected pages untouched ────────────────────────────────

/// A "selected" or "draft" page listed in run_pages is left unchanged.
#[test]
fn finalize_run_leaves_non_rejected_pages_untouched() {
    let doc = parse(FINALIZE_DOC);
    let tx = Transaction {
        ops: vec![Op::FinalizeRun {
            run_pages: vec!["pg.selected".to_owned(), "pg.draft".to_owned()],
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; diagnostics: {:?}",
        result.diagnostics
    );

    // Both pages must still be present and unchanged.
    assert!(
        result.source_after.contains("pg.selected"),
        "pg.selected must remain: {}",
        result.source_after
    );
    assert!(
        result.source_after.contains("pg.draft"),
        "pg.draft must remain: {}",
        result.source_after
    );

    // Neither page should be in affected (they were skipped).
    assert!(
        !result.affected_node_ids.contains(&"pg.selected".to_owned()),
        "pg.selected must NOT be in affected_node_ids: {:?}",
        result.affected_node_ids
    );
    assert!(
        !result.affected_node_ids.contains(&"pg.draft".to_owned()),
        "pg.draft must NOT be in affected_node_ids: {:?}",
        result.affected_node_ids
    );

    // source_after must equal source_before (no mutation occurred).
    assert_eq!(
        result.source_after, result.source_before,
        "source_after must equal source_before when no page is acted on"
    );
}

// ── finalize_run: unrecognized policy is advisory ─────────────────────────────

/// A rejected page with an unrecognized cleanup-policy gets an advisory and
/// is left untouched.
#[test]
fn finalize_run_unrecognized_policy_is_advisory() {
    let doc = parse(FINALIZE_DOC);
    let tx = Transaction {
        ops: vec![Op::FinalizeRun {
            run_pages: vec!["pg.rejected.weird".to_owned()],
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    // An advisory (tx.noop) must not elevate the status to Rejected.
    assert!(
        result.status == TxStatus::Accepted || result.status == TxStatus::AcceptedWithWarnings,
        "expected Accepted or AcceptedWithWarnings; got {:?}; diagnostics: {:?}",
        result.status,
        result.diagnostics
    );

    // Advisory diagnostic with code tx.noop must be present.
    assert!(
        result.diagnostics.iter().any(|d| d.code == "tx.noop"),
        "expected tx.noop advisory; got: {:?}",
        result.diagnostics
    );

    // Page must be untouched (not deleted, no workspace-role added for this page).
    assert!(
        result.source_after.contains("pg.rejected.weird"),
        "page must remain in source_after:\n{}",
        result.source_after
    );

    // The weird page must not appear in affected (it was left untouched).
    assert!(
        !result
            .affected_node_ids
            .contains(&"pg.rejected.weird".to_owned()),
        "pg.rejected.weird must NOT be in affected_node_ids: {:?}",
        result.affected_node_ids
    );
}

// ── finalize_run: unknown page id errors ─────────────────────────────────────

/// An id in run_pages that doesn't exist produces tx.unknown_node; the
/// transaction is rejected.
#[test]
fn finalize_run_unknown_page_id_errors() {
    let doc = parse(FINALIZE_DOC);
    let tx = Transaction {
        ops: vec![Op::FinalizeRun {
            run_pages: vec!["pg.nonexistent".to_owned()],
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Rejected,
        "expected Rejected; diagnostics: {:?}",
        result.diagnostics
    );

    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unknown_node"),
        "expected tx.unknown_node; got: {:?}",
        result.diagnostics
    );

    // source_after == source_before when rejected.
    assert_eq!(result.source_after, result.source_before);
}
