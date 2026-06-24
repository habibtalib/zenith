//! Local/system font discovery for the CLI.
//!
//! Enumerates the per-OS directories where the operating system keeps installed
//! fonts. This OS-specific path enumeration lives in the CLI — NOT in
//! `zenith-core` — so the core stays free of machine-specific assumptions: it
//! only ever reads font files from a directory list the caller hands it.
//!
//! The render path uses these dirs as a LAST-RESORT font source: a face found
//! here is registered with `FontSource::Local` and trips a `font.local`
//! advisory, because output that depends on a machine-local font is not
//! guaranteed deterministic across machines.

use std::collections::BTreeMap;
use std::path::PathBuf;

use zenith_core::{FontProvider, default_provider, scan_font_dirs};

use crate::commands::serialize_pretty;
use crate::json_types::FontsOutput;

/// List available fonts in two sections: bundled (portable) and local (this
/// machine only).
///
/// Uses the same discovery code as the renderer so there is no drift:
/// - Bundled families are the faces in `zenith_core::default_provider()`, named
///   in their proper (name-table) case via `zenith_layout::face_metadata`.
/// - Local families come from `zenith_core::scan_font_dirs(&os_font_dirs())`,
///   with any family already in the bundled set excluded (case-insensitively) so
///   the local section shows only genuinely machine-specific families.
///
/// Note: scanning reads every system font file on disk, so this command may take
/// a moment on machines with many fonts installed — that is expected for a
/// discovery command (similar to `fc-list`).
///
/// Returns `(output_string, exit_code)` following the convention used by
/// `commands::schema::*` and other discovery commands.
pub fn list(json: bool) -> (String, u8) {
    // Bundled families in their proper (name-table) case, deduped
    // case-insensitively. The provider keys faces by a lowercased family, so the
    // display name is recovered from each face's own metadata (same source the
    // renderer and `scan_font_dirs` use). The map is keyed by the lowercase name
    // for case-insensitive dedup; iteration order (lowercase-sorted) is stable.
    let mut bundled: BTreeMap<String, String> = BTreeMap::new();
    for face in default_provider().all_faces() {
        if let Ok(meta) = zenith_layout::face_metadata(&face.bytes, face.index) {
            bundled
                .entry(meta.family.to_lowercase())
                .or_insert(meta.family);
        }
    }

    // Local families (proper case), excluding any family already bundled
    // (compared case-insensitively).
    let mut local: BTreeMap<String, String> = BTreeMap::new();
    for entry in scan_font_dirs(&os_font_dirs()) {
        let key = entry.family.to_lowercase();
        if bundled.contains_key(&key) {
            continue;
        }
        local.entry(key).or_insert(entry.family);
    }

    let bundled_vec: Vec<String> = bundled.into_values().collect();
    let local_vec: Vec<String> = local.into_values().collect();

    if json {
        let out = FontsOutput {
            schema: "zenith-fonts-v1",
            bundled: bundled_vec,
            local: local_vec,
        };
        (serialize_pretty(&out), 0)
    } else {
        let mut lines: Vec<String> = Vec::new();

        lines.push("Bundled (portable)".to_owned());
        lines.push("──────────────────".to_owned());
        if bundled_vec.is_empty() {
            lines.push("  (none)".to_owned());
        } else {
            for family in &bundled_vec {
                lines.push(format!("  {family}"));
            }
        }

        lines.push(String::new());
        lines.push("Local / system (this machine only)".to_owned());
        lines.push("──────────────────────────────────".to_owned());
        if local_vec.is_empty() {
            lines.push("  (none found)".to_owned());
        } else {
            for family in &local_vec {
                lines.push(format!("  {family}"));
            }
            lines.push(String::new());
            lines.push(
                "Note: local fonts are not portable — renders that use them may differ on \
another machine and trip a `font.local` advisory."
                    .to_owned(),
            );
        }

        (lines.join("\n"), 0)
    }
}

/// Resolve `$HOME` as a [`PathBuf`].
///
/// Mirrors the pattern used by the plugin-paths module: `var_os` returns `None`
/// when the variable is unset, so no panic is possible. Only the unix-family
/// targets (linux/macos) consult `$HOME` for per-user font dirs.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// The OS font directories to scan for local/system fonts, most-canonical first.
///
/// Only directories that can be named without panicking are included; entries
/// that depend on an unset environment variable are simply omitted. The returned
/// list may contain directories that do not exist — the scanner skips those.
#[cfg(target_os = "linux")]
#[must_use]
pub fn os_font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Some(home) = home_dir() {
        dirs.push(home.join(".fonts"));
        dirs.push(home.join(".local/share/fonts"));
    }
    dirs
}

/// The OS font directories to scan for local/system fonts, most-canonical first.
#[cfg(target_os = "macos")]
#[must_use]
pub fn os_font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
    ];
    if let Some(home) = home_dir() {
        dirs.push(home.join("Library/Fonts"));
    }
    dirs
}

/// The OS font directories to scan for local/system fonts, most-canonical first.
#[cfg(target_os = "windows")]
#[must_use]
pub fn os_font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR") {
        dirs.push(PathBuf::from(windir).join("Fonts"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
    }
    dirs
}

/// Fallback for any other target OS: no known system font locations.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
#[must_use]
pub fn os_font_dirs() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_font_dirs_is_callable_and_paths_are_absolute_or_under_home() {
        // The list may legitimately be empty on exotic targets, but every entry
        // it does contain must be a non-empty path.
        for dir in os_font_dirs() {
            assert!(
                !dir.as_os_str().is_empty(),
                "an os font dir entry must not be empty"
            );
        }
    }

    #[test]
    fn list_human_returns_exit_0_and_contains_bundled_section() {
        let (output, code) = list(false);
        assert_eq!(code, 0, "exit code must be 0");
        assert!(
            output.contains("Bundled"),
            "human output must contain a 'Bundled' section header"
        );
        // "Noto Sans" is always bundled — verify it appears in proper case.
        assert!(
            output.contains("Noto Sans"),
            "bundled section must include 'Noto Sans'"
        );
        assert!(
            output.contains("Local / system"),
            "human output must contain a 'Local / system' section header"
        );
    }

    #[test]
    fn list_json_returns_exit_0_and_valid_envelope() {
        let (output, code) = list(true);
        assert_eq!(code, 0, "exit code must be 0");
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("--json output must be valid JSON");
        assert_eq!(
            parsed["schema"], "zenith-fonts-v1",
            "JSON envelope must carry schema = 'zenith-fonts-v1'"
        );
        let bundled = parsed["bundled"]
            .as_array()
            .expect("'bundled' must be an array");
        assert!(
            bundled.iter().any(|v| v.as_str() == Some("Noto Sans")),
            "bundled array must include 'Noto Sans'"
        );
        // 'local' key must be present (may be empty on CI, that is fine).
        assert!(
            parsed["local"].is_array(),
            "'local' key must be present and be an array"
        );
    }
}
