//! Pure logic for `zenith library list`.
//!
//! The registry/resolver lives in [`crate::library`]; this module only turns a
//! resolved set of packs into stdout text (human-readable or `--json`). It
//! operates on an already-resolved `&[LibraryPack]`, so it never touches the
//! filesystem itself — the dispatcher resolves the project directory and calls
//! [`crate::library::resolve_packs`].

use crate::commands::serialize_pretty;
use crate::library::LibraryPack;

/// JSON shape for `library list --json`.
#[derive(Debug, serde::Serialize)]
struct LibraryListOutput<'a> {
    schema: &'static str,
    packs: Vec<PackJson<'a>>,
}

/// A single pack entry in the `--json` output.
#[derive(Debug, serde::Serialize)]
struct PackJson<'a> {
    id: &'a str,
    version: Option<&'a str>,
    source: &'static str,
    items: Vec<&'a str>,
}

/// Render the resolved `packs` for `library list`.
///
/// Packs are expected pre-sorted by id (see [`crate::library::resolve_packs`]);
/// item order is preserved from the pack's component order.
///
/// - Human (default): one header line per pack
///   (`<id>  <version>  [preset|project]`) followed by indented `#<item>` lines.
/// - `--json`: a `{"schema":"zenith-library-v1","packs":[…]}` document.
pub fn list(packs: &[LibraryPack], json: bool) -> String {
    if json {
        let out = LibraryListOutput {
            schema: "zenith-library-v1",
            packs: packs
                .iter()
                .map(|p| PackJson {
                    id: &p.id,
                    version: p.version.as_deref(),
                    source: p.source.label(),
                    items: p.items.iter().map(String::as_str).collect(),
                })
                .collect(),
        };
        serialize_pretty(&out)
    } else {
        format_human(packs)
    }
}

/// Human-readable listing.
fn format_human(packs: &[LibraryPack]) -> String {
    if packs.is_empty() {
        return "no libraries found".to_owned();
    }
    let mut lines = Vec::new();
    for pack in packs {
        let version = pack.version.as_deref().unwrap_or("-");
        lines.push(format!(
            "{}  {}  [{}]",
            pack.id,
            version,
            pack.source.label()
        ));
        for item in &pack.items {
            lines.push(format!("  #{}", item));
        }
    }
    lines.join("\n")
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{PackSource, resolve_packs};

    #[test]
    fn human_lists_flowchart_with_items() {
        let packs = resolve_packs(None);
        let out = list(&packs, false);
        assert!(out.contains("@zenith/flowchart"), "got: {}", out);
        assert!(out.contains("[preset]"), "got: {}", out);
        assert!(out.contains("#process"), "got: {}", out);
        assert!(out.contains("#decision"), "got: {}", out);
        assert!(out.contains("#terminator"), "got: {}", out);
    }

    #[test]
    fn json_is_parseable_and_contains_flowchart() {
        let packs = resolve_packs(None);
        let out = list(&packs, true);
        let value: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(value["schema"], "zenith-library-v1");
        let packs_json = value["packs"].as_array().expect("packs array");
        let flow = packs_json
            .iter()
            .find(|p| p["id"] == "@zenith/flowchart")
            .expect("flowchart pack present");
        assert_eq!(flow["version"], "1.0.0");
        assert_eq!(flow["source"], "preset");
        let items: Vec<&str> = flow["items"]
            .as_array()
            .expect("items array")
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(items, vec!["process", "decision", "terminator"]);
    }

    #[test]
    fn empty_packs_human_message() {
        let out = list(&[], false);
        assert_eq!(out, "no libraries found");
    }

    #[test]
    fn version_falls_back_to_dash() {
        let pack = LibraryPack {
            id: "@x/y".to_owned(),
            version: None,
            source: PackSource::Preset,
            items: vec![],
        };
        let out = list(std::slice::from_ref(&pack), false);
        assert!(out.contains("@x/y  -  [preset]"), "got: {}", out);
    }
}
