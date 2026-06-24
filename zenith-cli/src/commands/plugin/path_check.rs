//! Non-invasive PATH reachability check for the `zenith` binary.
//!
//! The skill installed by `zenith plugin install` instructs AI agents to call
//! `zenith` by name. If the binary is not resolvable on the user's `PATH`, the
//! agent hits `command not found` with no guidance. These helpers detect that
//! situation so the command can print a warning — they never modify `PATH` or
//! any shell file.

use std::ffi::OsStr;
use std::path::PathBuf;

/// Resolve `bin` on a `PATH`-style value, returning the first matching file.
///
/// `path_var` is the raw `PATH` value (e.g. from [`std::env::var_os`]); it is
/// split with [`std::env::split_paths`], which honours the platform separator.
/// For each directory we test `dir/<bin><EXE_SUFFIX>` and return the first entry
/// that exists and is a file. A `None` / empty `PATH` resolves to `None`.
pub fn resolve_on_path(path_var: Option<&OsStr>, bin: &str) -> Option<PathBuf> {
    let path_var = path_var?;
    let file_name = format!("{bin}{}", std::env::consts::EXE_SUFFIX);
    for dir in std::env::split_paths(path_var) {
        let candidate = dir.join(&file_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Resolve the `zenith` binary on the current process's `PATH`.
pub fn zenith_on_path() -> Option<PathBuf> {
    resolve_on_path(std::env::var_os("PATH").as_deref(), "zenith")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn resolves_binary_present_in_path_dir() {
        let dir = tempfile::tempdir().unwrap();
        let bin_name = format!("zenith{}", std::env::consts::EXE_SUFFIX);
        let bin_path = dir.path().join(&bin_name);
        File::create(&bin_path).unwrap();

        let path_var = std::env::join_paths([dir.path()]).unwrap();
        let found = resolve_on_path(Some(path_var.as_os_str()), "zenith");
        assert_eq!(found.as_deref(), Some(bin_path.as_path()));
    }

    #[test]
    fn none_when_path_missing() {
        assert!(resolve_on_path(None, "zenith").is_none());
    }

    #[test]
    fn none_when_path_empty() {
        assert!(resolve_on_path(Some(OsStr::new("")), "zenith").is_none());
    }

    #[test]
    fn none_when_dir_lacks_binary() {
        let dir = tempfile::tempdir().unwrap();
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        assert!(resolve_on_path(Some(path_var.as_os_str()), "zenith").is_none());
    }
}
