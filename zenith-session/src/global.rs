//! Global cross-document LRU storage cap.
//!
//! Bounds the TOTAL bytes of all documents' object stores under `<data_dir>/docs`.
//! When over the ceiling, evict the least-recently-used documents (by their
//! `DocMeta.updated_ms`) one whole `docs/<id>/` subtree at a time until under the
//! cap. The most-recently-used document is never evicted. A document's `.zen`
//! source lives outside the store and is untouched — eviction only discards local
//! history, which the next edit re-creates.

use crate::adapter::Fs;
use crate::error::SessionError;
use crate::identity::read_meta;
use crate::layout::StorePaths;

// ── Public types ───────────────────────────────────────────────────────────────

/// Report returned by [`enforce_global_cap`].
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalCapReport {
    /// Doc ids evicted, in eviction order (oldest first).
    pub evicted: Vec<String>,
    /// Total stored object bytes across all docs before eviction.
    pub bytes_before: u64,
    /// Total stored object bytes across all docs after eviction.
    pub bytes_after: u64,
}

// ── Private helpers ────────────────────────────────────────────────────────────

/// Sum the compressed sizes of all objects stored for `doc_id`.
///
/// Walks `<objects_dir>/<shard>/<file>` two levels deep, summing each file's
/// byte length. Returns 0 if the objects directory does not exist.
fn doc_object_bytes(fs: &impl Fs, paths: &StorePaths, doc_id: &str) -> Result<u64, SessionError> {
    let odir = paths.objects_dir(doc_id);
    if !fs.exists(&odir) {
        return Ok(0);
    }
    let mut total: u64 = 0;
    for shard in fs.read_dir(&odir)? {
        for obj in fs.read_dir(&shard)? {
            let bytes = fs.read(&obj)?;
            total = total.saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        }
    }
    Ok(total)
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Evict least-recently-used documents until total stored bytes <= `max_total_bytes`.
///
/// Recency is `DocMeta.updated_ms` (missing meta → treated as oldest, ms 0). The
/// single most-recently-used document is never evicted (so an active doc can never
/// be wiped even if it alone exceeds the cap). Returns what was evicted.
pub fn enforce_global_cap(
    fs: &impl Fs,
    paths: &StorePaths,
    max_total_bytes: u64,
) -> Result<GlobalCapReport, SessionError> {
    let droot = paths.docs_root();
    if !fs.exists(&droot) {
        return Ok(GlobalCapReport {
            evicted: Vec::new(),
            bytes_before: 0,
            bytes_after: 0,
        });
    }

    // Gather (doc_id, bytes, updated_ms) for every doc dir.
    struct DocEntry {
        id: String,
        bytes: u64,
        updated_ms: u128,
    }
    let mut docs: Vec<DocEntry> = Vec::new();
    for dir in fs.read_dir(&droot)? {
        let id = match dir.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_owned(),
            None => continue,
        };
        let bytes = doc_object_bytes(fs, paths, &id)?;
        let updated_ms = read_meta(fs, paths, &id)?
            .map(|m| m.updated_ms)
            .unwrap_or(0);
        docs.push(DocEntry {
            id,
            bytes,
            updated_ms,
        });
    }

    let bytes_before: u64 = docs.iter().fold(0u64, |a, d| a.saturating_add(d.bytes));
    let mut total = bytes_before;
    let mut evicted: Vec<String> = Vec::new();

    if total <= max_total_bytes {
        return Ok(GlobalCapReport {
            evicted,
            bytes_before,
            bytes_after: total,
        });
    }

    // Evict LRU first: sort ascending by updated_ms, tie-break by id for determinism.
    docs.sort_by(|a, b| {
        a.updated_ms
            .cmp(&b.updated_ms)
            .then_with(|| a.id.cmp(&b.id))
    });
    // The most-recently-used doc is the LAST after this sort; never evict it.
    let protected_index = docs.len().saturating_sub(1);
    for (i, d) in docs.iter().enumerate() {
        if total <= max_total_bytes {
            break;
        }
        if i == protected_index {
            continue; // never evict the most-recent doc
        }
        // Evict the whole docs/<id> subtree.
        let dir = paths.doc_dir(&d.id);
        if fs.exists(&dir) {
            fs.remove(&dir)?;
        }
        total = total.saturating_sub(d.bytes);
        evicted.push(d.id.clone());
    }

    Ok(GlobalCapReport {
        evicted,
        bytes_before,
        bytes_after: total,
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{FakeClock, FakeRng, MemFs};
    use crate::{identity, store};
    use std::path::Path;
    use std::time::{Duration, UNIX_EPOCH};

    fn make_paths() -> StorePaths {
        StorePaths::new("/data")
    }

    /// Seed a doc with object bytes (via `put_object`) and a meta timestamp (via
    /// `reconcile` at `updated_ms` milliseconds since epoch). Returns the doc id.
    fn seed_doc(
        fs: &MemFs,
        paths: &StorePaths,
        doc_id: &str,
        content: &[u8],
        updated_ms: u64,
    ) -> String {
        let clock = FakeClock(UNIX_EPOCH + Duration::from_millis(updated_ms));
        let rng = FakeRng(0x01);
        let doc_path = Path::new("/fake/doc.zen");
        // Adopt the given doc_id so we can control it precisely.
        identity::reconcile(fs, paths, &clock, &rng, Some(doc_id), doc_path).unwrap();
        store::put_object(fs, paths, doc_id, content).unwrap();
        doc_id.to_owned()
    }

    #[test]
    fn under_cap_evicts_nothing() {
        let fs = MemFs::new();
        let paths = make_paths();

        seed_doc(&fs, &paths, "doc-a", &[42u8; 200], 1000);
        seed_doc(&fs, &paths, "doc-b", &[99u8; 200], 2000);

        let report = enforce_global_cap(&fs, &paths, 1_000_000).unwrap();

        assert!(report.evicted.is_empty(), "nothing should be evicted");
        assert_eq!(
            report.bytes_after, report.bytes_before,
            "bytes unchanged when under cap"
        );
    }

    #[test]
    fn evicts_lru_first() {
        let fs = MemFs::new();
        let paths = make_paths();

        // "old" doc: reconciled at ms 100, with a large object (~2000 bytes of content).
        seed_doc(&fs, &paths, "old", &vec![0xAAu8; 2000], 100);
        // "new" doc: reconciled at ms 5000, with a smaller object.
        let new_hash = store::put_object(&fs, &paths, "new", &[0xBBu8; 50]).unwrap();
        // Write meta for "new" manually via reconcile.
        {
            let clock = FakeClock(UNIX_EPOCH + Duration::from_millis(5000));
            let rng = FakeRng(0x02);
            identity::reconcile(&fs, &paths, &clock, &rng, Some("new"), Path::new("/n.zen"))
                .unwrap();
        }

        // Measure how many bytes old takes up so we can set cap below combined total
        // but above the new doc alone.
        let old_bytes = doc_object_bytes(&fs, &paths, "old").unwrap();
        let new_bytes = doc_object_bytes(&fs, &paths, "new").unwrap();
        let combined = old_bytes.saturating_add(new_bytes);
        // Cap is below combined but above new_bytes alone.
        let cap = new_bytes.saturating_add(1);
        assert!(
            combined > cap,
            "test requires combined > cap (combined={combined}, cap={cap})"
        );

        let report = enforce_global_cap(&fs, &paths, cap).unwrap();

        assert_eq!(report.evicted, vec!["old"], "older doc should be evicted");
        assert!(
            report.bytes_after <= cap,
            "bytes_after ({}) should be <= cap ({})",
            report.bytes_after,
            cap
        );

        // Old doc's object is gone.
        let old_hash = store::object_hash(&vec![0xAAu8; 2000]);
        assert!(
            store::get_object(&fs, &paths, "old", &old_hash).is_err(),
            "old doc's object must be gone"
        );
        // New doc's object is still readable.
        let got = store::get_object(&fs, &paths, "new", &new_hash).unwrap();
        assert_eq!(got, vec![0xBBu8; 50], "new doc's object must survive");
    }

    #[test]
    fn never_evicts_most_recent() {
        let fs = MemFs::new();
        let paths = make_paths();

        // Single doc whose objects alone exceed the cap.
        seed_doc(&fs, &paths, "solo", &vec![0xCCu8; 2000], 9999);
        let solo_bytes = doc_object_bytes(&fs, &paths, "solo").unwrap();
        assert!(solo_bytes > 0, "solo doc must have some bytes");

        // Cap below the solo doc's size.
        let cap = solo_bytes.saturating_sub(1);

        let report = enforce_global_cap(&fs, &paths, cap).unwrap();

        assert!(
            report.evicted.is_empty(),
            "most-recent (and only) doc must never be evicted"
        );
        assert_eq!(
            report.bytes_after, report.bytes_before,
            "bytes unchanged when protected"
        );
    }

    #[test]
    fn empty_store_noop() {
        let fs = MemFs::new();
        let paths = make_paths();

        // No docs_root exists at all.
        let report = enforce_global_cap(&fs, &paths, 0).unwrap();

        assert!(report.evicted.is_empty());
        assert_eq!(report.bytes_before, 0);
        assert_eq!(report.bytes_after, 0);
    }

    #[test]
    fn missing_meta_treated_as_oldest() {
        let fs = MemFs::new();
        let paths = make_paths();

        // "no-meta" doc: objects only, no reconcile (no meta.json), updated_ms → 0.
        store::put_object(&fs, &paths, "no-meta", &vec![0xDDu8; 2000]).unwrap();

        // "recent" doc: reconciled at ms 9000 with a small object.
        seed_doc(&fs, &paths, "recent", &[0xEEu8; 50], 9000);

        let no_meta_bytes = doc_object_bytes(&fs, &paths, "no-meta").unwrap();
        let recent_bytes = doc_object_bytes(&fs, &paths, "recent").unwrap();
        let combined = no_meta_bytes.saturating_add(recent_bytes);
        let cap = recent_bytes.saturating_add(1);
        assert!(
            combined > cap,
            "test requires combined > cap (combined={combined}, cap={cap})"
        );

        let report = enforce_global_cap(&fs, &paths, cap).unwrap();

        assert_eq!(
            report.evicted,
            vec!["no-meta"],
            "meta-less doc (updated_ms=0) must be evicted first"
        );
        assert!(
            report.bytes_after <= cap,
            "bytes_after ({}) should be <= cap ({})",
            report.bytes_after,
            cap
        );
    }
}
