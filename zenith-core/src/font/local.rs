//! Local/system font scanning.
//!
//! Reads font files from a caller-supplied list of directories and extracts the
//! family / weight / style of every face, so the CLI can register machine-local
//! fonts as a LAST-RESORT resolution source (after bundled + project fonts).
//!
//! ## Determinism boundary
//!
//! This module performs filesystem reads of font FILES (the same kind of read
//! the bundled `include_bytes!` faces already are at compile time), but it does
//! NOT enumerate OS font locations itself: the directory list is passed in by
//! the caller. OS-directory discovery lives in the CLI (`os_font_dirs`) so the
//! core never reaches into machine-specific paths on its own.
//!
//! Output is fully deterministic for a given set of directory contents: files
//! are collected and sorted by path before parsing, and the returned entries are
//! sorted by `(family, weight, style, path, index)`.
//!
//! No `unwrap`/`expect`/`panic!`: every IO or parse failure is skipped via
//! `match … continue`.

use std::path::{Path, PathBuf};

use ttf_parser::name_id;

use super::FontStyle;

/// Upper bound on faces probed inside a single font collection (`.ttc`). Guards
/// against a malformed collection header advertising an unbounded face count.
const MAX_COLLECTION_FACES: u32 = 64;

/// Maximum subdirectory depth walked under each font root. Font directories nest
/// only a few levels; this cap bounds the walk and terminates symlink cycles.
const MAX_SCAN_DEPTH: u32 = 8;

/// A single local/system font face discovered by [`scan_font_dirs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFontEntry {
    /// Absolute or caller-relative path to the font file on disk.
    pub path: PathBuf,
    /// Typographic family name (e.g. `"Inter"`), preferring name ID 16 over 1.
    pub family: String,
    /// Numeric weight (e.g. 400, 700).
    pub weight: u16,
    /// Normal or italic style.
    pub style: FontStyle,
    /// Face index within the file (0 for single-face files; >0 for `.ttc`).
    pub index: u32,
}

/// Scan each directory in `dirs` for font files and return every readable face.
///
/// Each directory is read with `std::fs::read_dir`; directories that do not
/// exist or cannot be read are silently skipped (no error). Files whose
/// extension is `ttf`, `otf`, or `ttc` (case-insensitive) are collected, sorted
/// by path for determinism, and parsed. For a font collection, faces are probed
/// from index 0 upward until parsing fails or `MAX_COLLECTION_FACES` is
/// reached. Faces with no readable family name are skipped.
///
/// The returned `Vec` is sorted by `(family, weight, style, path, index)`, so it
/// is stable for a given set of directory contents.
#[must_use]
pub fn scan_font_dirs(dirs: &[PathBuf]) -> Vec<LocalFontEntry> {
    let mut files: Vec<PathBuf> = Vec::new();
    // OS font directories are hierarchical (e.g. `/usr/share/fonts/TTF/…`,
    // `~/Library/Fonts/…`), so each root is walked recursively. A depth-capped
    // worklist bounds the walk and terminates even on symlink cycles without
    // needing a visited set.
    let mut worklist: Vec<(PathBuf, u32)> = dirs.iter().map(|d| (d.clone(), 0u32)).collect();
    while let Some((dir, depth)) = worklist.pop() {
        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for entry in read {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            // Use `file_type()` from the DirEntry rather than `path.is_dir()` to
            // avoid an extra `stat` syscall per entry — most platforms populate
            // the type during `readdir`.
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let path = entry.path();
            if is_dir {
                if depth < MAX_SCAN_DEPTH {
                    worklist.push((path, depth + 1));
                }
            } else if has_font_extension(&path) {
                files.push(path);
            }
        }
    }

    // Sort files by path so parse order (and thus output order) is deterministic
    // regardless of the directory-iteration order the OS returns.
    files.sort();

    let mut entries: Vec<LocalFontEntry> = Vec::new();
    for path in files {
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        for index in 0..MAX_COLLECTION_FACES {
            let face = match ttf_parser::Face::parse(&bytes, index) {
                Ok(f) => f,
                // A failed parse at index 0 means the file is unreadable; at a
                // higher index it means the collection has no further faces.
                // Either way, stop probing this file.
                Err(_) => break,
            };
            let family = match best_family_name(&face) {
                Some(f) => f,
                None => continue,
            };
            let weight = face.weight().to_number();
            let style = if face.is_italic() {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            };
            entries.push(LocalFontEntry {
                path: path.clone(),
                family,
                weight,
                style,
                index,
            });
        }
    }

    entries.sort_by(|a, b| {
        a.family
            .cmp(&b.family)
            .then(a.weight.cmp(&b.weight))
            .then(a.style.cmp(&b.style))
            .then(a.path.cmp(&b.path))
            .then(a.index.cmp(&b.index))
    });
    entries
}

/// True when `path` has a `ttf`, `otf`, or `ttc` extension (case-insensitive).
fn has_font_extension(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => {
            let ext = ext.to_ascii_lowercase();
            ext == "ttf" || ext == "otf" || ext == "ttc"
        }
        None => false,
    }
}

/// Return the best available family name from a face's name table.
///
/// Prefers name ID 16 (Typographic Family) over name ID 1 (Family). Mirrors the
/// strategy in `zenith-layout`'s `font_meta::best_family_name` so a local face
/// registers under the same family string a project asset would.
fn best_family_name(face: &ttf_parser::Face<'_>) -> Option<String> {
    let mut typo_family: Option<String> = None;
    let mut family: Option<String> = None;

    for name in face.names() {
        if name.name_id == name_id::TYPOGRAPHIC_FAMILY
            && typo_family.is_none()
            && let Some(s) = name.to_string()
        {
            typo_family = Some(s);
        } else if name.name_id == name_id::FAMILY
            && family.is_none()
            && let Some(s) = name.to_string()
        {
            family = Some(s);
        }
    }

    typo_family.or(family)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The workspace bundled-fonts directory, used as a real, committed fixture.
    fn bundled_fonts_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts")
    }

    #[test]
    fn scans_bundled_fonts_dir_for_noto_sans() {
        let entries = scan_font_dirs(&[bundled_fonts_dir()]);
        assert!(
            !entries.is_empty(),
            "scanning the bundled fonts dir must yield faces"
        );
        assert!(
            entries.iter().any(|e| e.family.contains("Noto Sans")),
            "expected a 'Noto Sans' family among scanned faces, got: {:?}",
            entries.iter().map(|e| &e.family).collect::<Vec<_>>()
        );
        // A regular face (weight 400, Normal) must be present and parsed.
        assert!(
            entries
                .iter()
                .any(|e| e.weight == 400 && e.style == FontStyle::Normal),
            "expected at least one 400/Normal face"
        );
    }

    #[test]
    fn extracts_weight_and_style_variants() {
        let entries = scan_font_dirs(&[bundled_fonts_dir()]);
        // The bundle ships bold (700) and italic faces; the scanner must read
        // their weight/style from the font tables.
        assert!(
            entries.iter().any(|e| e.weight == 700),
            "expected a 700-weight face among scanned bundled fonts"
        );
        assert!(
            entries.iter().any(|e| e.style == FontStyle::Italic),
            "expected an italic face among scanned bundled fonts"
        );
    }

    #[test]
    fn nonexistent_dir_yields_empty() {
        let entries = scan_font_dirs(&[PathBuf::from("/this/path/does/not/exist/zenith")]);
        assert!(entries.is_empty(), "a missing dir must yield no entries");
    }

    #[test]
    fn empty_dir_list_yields_empty() {
        let entries = scan_font_dirs(&[]);
        assert!(
            entries.is_empty(),
            "an empty dir list must yield no entries"
        );
    }

    #[test]
    fn output_is_sorted_and_deterministic() {
        let a = scan_font_dirs(&[bundled_fonts_dir()]);
        let b = scan_font_dirs(&[bundled_fonts_dir()]);
        assert_eq!(a, b, "two scans of the same dir must be identical");
        // Verify the documented sort order holds.
        let mut sorted = a.clone();
        sorted.sort_by(|x, y| {
            x.family
                .cmp(&y.family)
                .then(x.weight.cmp(&y.weight))
                .then(x.style.cmp(&y.style))
                .then(x.path.cmp(&y.path))
                .then(x.index.cmp(&y.index))
        });
        assert_eq!(a, sorted, "scan output must already be sorted");
    }

    #[test]
    fn non_font_extensions_are_skipped() {
        // The bundled fonts dir also contains ABOUT.txt and LICENSE.txt; none of
        // the scanned entries may point at a non-font file.
        let entries = scan_font_dirs(&[bundled_fonts_dir()]);
        for e in &entries {
            assert!(
                has_font_extension(&e.path),
                "scanned a non-font file: {}",
                e.path.display()
            );
        }
    }
}
