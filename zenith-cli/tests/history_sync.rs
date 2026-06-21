//! Integration tests for `sync_external_in` in `zenith-cli`.
//!
//! Exercises round-trip-safe co-session sync: after a simulated external edit
//! (GUI, hand-edit, or `git checkout`), `sync_external_in` captures the new
//! on-disk state into Tier-1 history as an "external" change so it is undoable.

use std::path::PathBuf;

use tempfile::TempDir;
use zenith_cli::history::{
    NavOutcome, SyncOutcome, record_edit_in, sync_external_in, undo_edit_in,
};
use zenith_core::{KdlAdapter, KdlSource as _};
use zenith_session::StorePaths;
use zenith_session::adapter::OsFs;

// ── Fixture ───────────────────────────────────────────────────────────────────

/// A minimal valid `.zen` document (no `doc-id` attribute yet).
const MINIMAL_NO_ID: &str = r##"zenith version=1 {
  project id="proj.sync" name="Sync Test"
  tokens format="zenith-token-v1" {
    token id="color.bg" type="color" value="#f8fafc"
  }
  styles {
  }
  document id="doc.sync" title="Sync Test" {
    page id="page.one" w=(px)480 h=(px)160 {
      rect id="rect.bg" x=(px)0 y=(px)0 w=(px)480 h=(px)160 fill=(token)"color.bg"
    }
  }
}
"##;

fn store_in(tmp: &TempDir) -> StorePaths {
    StorePaths::new(tmp.path())
}

fn doc_path_in(tmp: &TempDir) -> PathBuf {
    tmp.path().join("sync-test.zen")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// After a normal edit (state A'), simulate an external edit (write state B to
/// disk directly). `sync_external_in` must return `Captured{..}`, advance HEAD
/// to B, and make the external change undoable (undo restores A').
#[test]
fn sync_captures_external_change() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let doc_path = doc_path_in(&tmp);

    // First edit through history pipeline — mints doc-id, HEAD = A'.
    let recorded = record_edit_in(&paths, MINIMAL_NO_ID.as_bytes(), &doc_path, "tx.apply");
    assert!(
        recorded.warning.is_none(),
        "first edit must have no warning"
    );
    let bytes_a_prime = recorded.bytes;

    // Write the stamped bytes to disk so the doc-id is present on disk.
    std::fs::write(&doc_path, &bytes_a_prime).unwrap();

    // Simulate an external edit: derive B from A' by changing a pixel value.
    let bytes_b = String::from_utf8(bytes_a_prime.clone())
        .unwrap()
        .replace("w=(px)480", "w=(px)500")
        .into_bytes();
    // Write B directly to disk (no history pipeline — this is the external edit).
    std::fs::write(&doc_path, &bytes_b).unwrap();

    // Sync must capture the external change.
    let outcome = sync_external_in(&paths, &doc_path).expect("sync_external_in must succeed");
    assert!(
        matches!(outcome, SyncOutcome::Captured { .. }),
        "sync must return Captured when the on-disk content differs from HEAD; got {outcome:?}"
    );

    // HEAD must now equal B (the external state).
    let fs = OsFs;
    // Read the doc-id from the file so we can query the session.
    let raw = std::fs::read(&doc_path).unwrap();
    let doc = KdlAdapter
        .parse(&raw)
        .expect("doc must be parseable after sync");
    let doc_id = doc.doc_id.expect("doc must have a doc-id after sync");
    let current = zenith_session::current_content(&fs, &paths, &doc_id)
        .expect("current_content must succeed")
        .expect("session must have a HEAD after sync");
    assert_eq!(
        current, bytes_b,
        "HEAD must equal the externally-written bytes B after sync"
    );

    // The external change must be undoable: undo must restore A'.
    let nav = undo_edit_in(&paths, &doc_path).expect("undo_edit_in must succeed after sync");
    assert!(
        matches!(nav, NavOutcome::Moved),
        "undo after sync must return Moved (external change is undoable)"
    );
    let on_disk_after_undo = std::fs::read(&doc_path).unwrap();
    assert_eq!(
        on_disk_after_undo, bytes_a_prime,
        "undo after sync must restore A' — round-trip safe"
    );
}

/// When the on-disk content already matches the session HEAD, `sync_external_in`
/// must return `AlreadyInSync` (dedup — no spurious record).
#[test]
fn sync_noop_when_unchanged() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let doc_path = doc_path_in(&tmp);

    // Single edit — mints doc-id, HEAD = A'.
    let recorded = record_edit_in(&paths, MINIMAL_NO_ID.as_bytes(), &doc_path, "tx.apply");
    assert!(
        recorded.warning.is_none(),
        "first edit must have no warning"
    );
    let bytes_a_prime = recorded.bytes;

    // Write the stamped (HEAD) bytes to disk so file == HEAD.
    std::fs::write(&doc_path, &bytes_a_prime).unwrap();

    // Sync must detect no change.
    let outcome = sync_external_in(&paths, &doc_path).expect("sync_external_in must succeed");
    assert_eq!(
        outcome,
        SyncOutcome::AlreadyInSync,
        "sync must return AlreadyInSync when the file matches HEAD"
    );
}

/// A file that has never been recorded through the history pipeline has no
/// `doc-id`; `sync_external_in` must return an `Err` whose message mentions
/// "no history".
#[test]
fn sync_no_doc_id_errors() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let doc_path = doc_path_in(&tmp);

    // Write the fixture directly without going through record_edit_in (no doc-id).
    std::fs::write(&doc_path, MINIMAL_NO_ID.as_bytes()).unwrap();

    let result = sync_external_in(&paths, &doc_path);
    assert!(
        result.is_err(),
        "sync_external_in must error for a doc with no doc-id"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("no history"),
        "error message must mention 'no history'; got: {msg:?}"
    );
}
