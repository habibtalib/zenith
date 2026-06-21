//! Zenith library subsystem: pack format, registry, and resolver.
//!
//! A library "pack" is a `.zen` file whose IDENTITY is declared by a single
//! `library` SELF-entry in its own `libraries` block, for example:
//!
//! ```kdl
//! libraries { library id="@zenith/flowchart" version="1.0.0" }
//! ```
//!
//! That entry's `id` is the package id and `version` is the pack version. A
//! pack's ITEMS are its `components`: item `decision` in pack
//! `@zenith/flowchart` is addressed `@zenith/flowchart#decision`.
//!
//! PRESET packs are embedded in the binary via [`include_str!`] (see
//! [`EMBEDDED_PACKS`]); PROJECT packs live in `<project_dir>/libraries/*.zen`
//! and are scanned at runtime. Resolution order is project packs first, then
//! embedded presets (a project pack shadows an embedded pack of the same id).
//!
//! This module contains pure pack-loading/registry logic only; the CLI command
//! that consumes it lives in [`crate::commands::library`].

use std::path::{Path, PathBuf};

use zenith_core::{KdlAdapter, KdlSource};

/// Embedded preset packs, as `(pack_id, pack_source)` pairs.
///
/// Each `pack_source` is the verbatim `.zen` text of a shipped preset library,
/// bundled into the binary via [`include_str!`] (mirroring how the default
/// fonts are bundled in `zenith-core`). The `pack_id` is the expected package
/// id and is used only for diagnostics/lookup convenience; the authoritative id
/// is parsed from the pack's own `library` self-entry.
pub const EMBEDDED_PACKS: &[(&str, &str)] = &[(
    "@zenith/flowchart",
    include_str!("../../assets/libraries/zenith-flowchart.zen"),
)];

/// Where a [`LibraryPack`] was loaded from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackSource {
    /// A preset pack embedded in the binary.
    Preset,
    /// A project pack read from the given `.zen` file path.
    Project(PathBuf),
}

impl PackSource {
    /// A short, stable label for human/JSON output: `"preset"` or `"project"`.
    pub fn label(&self) -> &'static str {
        match self {
            PackSource::Preset => "preset",
            PackSource::Project(_) => "project",
        }
    }
}

/// A loaded library pack: its identity plus the ids of the items it provides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPack {
    /// The package id, parsed from the pack's `library` self-entry.
    pub id: String,
    /// The pack version, parsed from the pack's `library` self-entry.
    pub version: Option<String>,
    /// Where the pack came from.
    pub source: PackSource,
    /// The item ids (component ids) the pack provides, in source order.
    pub items: Vec<String>,
}

/// An error produced while parsing a pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackError {
    /// Human-readable message describing the failure.
    pub message: String,
}

impl PackError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PackError {}

/// Parse a `.zen` pack `source` into a [`LibraryPack`] tagged with `source_kind`.
///
/// Pack identity is derived from the document's `libraries` block: the library
/// entry whose `id` matches the document's `project` id is the SELF-entry; if no
/// entry matches the project id but there is exactly one library entry, that
/// sole entry is used. A pack with no identifying library self-entry is an error
/// (a pack MUST declare its identity).
///
/// Items are the document's component ids in source order.
///
/// # Errors
///
/// Returns [`PackError`] when the source fails to parse, or when no library
/// self-entry can be determined.
pub fn parse_pack(source: &str, source_kind: PackSource) -> Result<LibraryPack, PackError> {
    let doc = KdlAdapter
        .parse(source.as_bytes())
        .map_err(|e| PackError::new(format!("parse error: {}", e)))?;

    let project_id = doc.project.as_ref().map(|p| p.id.as_str());

    // Prefer the library entry whose id matches the project id; otherwise fall
    // back to the sole library entry when there is exactly one.
    let self_entry = project_id
        .and_then(|pid| doc.libraries.iter().find(|lib| lib.id == pid))
        .or(match doc.libraries.as_slice() {
            [only] => Some(only),
            _ => None,
        });

    let self_entry = self_entry.ok_or_else(|| {
        PackError::new(
            "pack has no identifying library self-entry (declare \
             `libraries { library id=\"…\" version=\"…\" }`)",
        )
    })?;

    let items = doc.components.iter().map(|c| c.id.clone()).collect();

    Ok(LibraryPack {
        id: self_entry.id.clone(),
        version: self_entry.version.clone(),
        source: source_kind,
        items,
    })
}

/// Parse every entry in [`EMBEDDED_PACKS`] into a [`LibraryPack`].
///
/// An embedded pack that fails to parse is skipped (embedded content is shipped
/// and tested, so this should not happen in practice); the returned vector
/// contains only the packs that parsed successfully.
pub fn load_embedded_packs() -> Vec<LibraryPack> {
    EMBEDDED_PACKS
        .iter()
        .filter_map(|(_, src)| parse_pack(src, PackSource::Preset).ok())
        .collect()
}

/// Scan `project_dir/libraries/*.zen` and parse each file into a [`LibraryPack`].
///
/// A missing `libraries/` directory (or a `project_dir` without one) yields an
/// empty vector. Files that fail to read or parse are skipped — a note is
/// written to stderr — so one bad pack never aborts the whole listing.
pub fn load_project_packs(project_dir: &Path) -> Vec<LibraryPack> {
    let libraries_dir = project_dir.join("libraries");
    let entries = match std::fs::read_dir(&libraries_dir) {
        Ok(entries) => entries,
        // Missing directory (or any read error) → no project packs.
        Err(_) => return Vec::new(),
    };

    let mut packs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("zen") {
            continue;
        }
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("note: skipping '{}': {}", path.display(), e);
                continue;
            }
        };
        match parse_pack(&source, PackSource::Project(path.clone())) {
            Ok(pack) => packs.push(pack),
            Err(e) => eprintln!("note: skipping '{}': {}", path.display(), e),
        }
    }
    packs
}

/// Resolve all available packs: project packs first, then embedded presets.
///
/// Project packs take precedence over embedded packs of the same id (a project
/// pack SHADOWS an embedded preset). Both are returned, each tagged with its
/// [`PackSource`], so callers that LIST can show every pack; callers that
/// MATERIALIZE should prefer the first pack for a given id. The result is sorted
/// by id for deterministic output (project before embedded on ties).
pub fn resolve_packs(project_dir: Option<&Path>) -> Vec<LibraryPack> {
    let mut packs = Vec::new();
    if let Some(dir) = project_dir {
        packs.extend(load_project_packs(dir));
    }
    packs.extend(load_embedded_packs());

    // Stable, deterministic order: by id, with project packs ahead of embedded
    // on ties (so the shadowing winner sorts first).
    packs.sort_by(|a, b| {
        a.id.cmp(&b.id)
            .then_with(|| source_rank(&a.source).cmp(&source_rank(&b.source)))
    });
    packs
}

/// Sort rank giving project packs precedence over embedded presets.
fn source_rank(source: &PackSource) -> u8 {
    match source {
        PackSource::Project(_) => 0,
        PackSource::Preset => 1,
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use zenith_core::validate;

    const FLOWCHART_SRC: &str = include_str!("../../assets/libraries/zenith-flowchart.zen");

    #[test]
    fn parse_embedded_flowchart_identity_and_items() {
        let pack = parse_pack(FLOWCHART_SRC, PackSource::Preset).expect("flowchart pack parses");
        assert_eq!(pack.id, "@zenith/flowchart");
        assert_eq!(pack.version.as_deref(), Some("1.0.0"));
        assert_eq!(pack.source, PackSource::Preset);
        assert_eq!(pack.items, vec!["process", "decision", "terminator"]);
    }

    #[test]
    fn embedded_flowchart_parses_and_validates_clean() {
        let doc = KdlAdapter
            .parse(FLOWCHART_SRC.as_bytes())
            .expect("embedded pack must parse");
        let report = validate(&doc);
        let errors: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.severity == zenith_core::Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "embedded pack must validate with no errors; got: {:?}",
            errors
        );
    }

    #[test]
    fn load_embedded_packs_includes_flowchart() {
        let packs = load_embedded_packs();
        assert!(
            packs.iter().any(|p| p.id == "@zenith/flowchart"),
            "embedded packs must include @zenith/flowchart"
        );
    }

    #[test]
    fn pack_without_self_entry_errors() {
        let src = r#"zenith version=1 {
  project id="proj.x" name="No Library"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="d" title="x" {
    page id="pg" w=(px)10 h=(px)10 {}
  }
}
"#;
        let err = parse_pack(src, PackSource::Preset).expect_err("must require a self-entry");
        assert!(err.message.contains("library self-entry"));
    }

    #[test]
    fn load_project_packs_finds_libraries_dir_pack() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lib_dir = dir.path().join("libraries");
        std::fs::create_dir_all(&lib_dir).expect("create libraries dir");
        std::fs::write(lib_dir.join("foo.zen"), FLOWCHART_SRC).expect("write pack");

        let packs = load_project_packs(dir.path());
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].id, "@zenith/flowchart");
        assert!(matches!(packs[0].source, PackSource::Project(_)));
    }

    #[test]
    fn load_project_packs_missing_dir_is_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(load_project_packs(dir.path()).is_empty());
    }

    #[test]
    fn resolve_packs_includes_embedded_when_no_project_dir() {
        let packs = resolve_packs(None);
        assert!(packs.iter().any(|p| p.id == "@zenith/flowchart"));
    }

    #[test]
    fn resolve_packs_is_sorted_by_id() {
        let packs = resolve_packs(None);
        let mut sorted = packs.clone();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        let ids: Vec<_> = packs.iter().map(|p| &p.id).collect();
        let sorted_ids: Vec<_> = sorted.iter().map(|p| &p.id).collect();
        assert_eq!(ids, sorted_ids);
    }
}
