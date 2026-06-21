//! Platform data-directory resolution for zenith-session.
//!
//! The resolved directory is the root under which all `.zen` store data lives.
//!
//! # Platform mapping (via the `dirs` crate)
//!
//! | Platform | Path |
//! |----------|------|
//! | Linux    | `$XDG_DATA_HOME/zenith` (falls back to `~/.local/share/zenith`) |
//! | macOS    | `~/Library/Application Support/zenith` |
//! | Windows  | `%LOCALAPPDATA%\zenith` |
//!
//! # Override
//!
//! Set `ZENITH_DATA_DIR` to a non-empty path to bypass platform detection
//! entirely.  Useful for testing and for portable installations.

use std::path::PathBuf;

use crate::error::SessionError;

/// Resolve the zenith data directory using an injectable environment lookup.
///
/// Priority:
/// 1. `ZENITH_DATA_DIR` environment variable, if non-empty.
/// 2. `dirs::data_dir()` joined with `"zenith"`.
///
/// Returns [`SessionError`] if neither source is available (e.g. on a headless
/// system where the platform data dir cannot be determined).
///
/// The injectable `env` parameter makes this function fully deterministic in
/// tests — pass a closure that returns `Some(...)` for the keys you want to
/// override, `None` otherwise.
pub fn resolve_data_dir_with(
    env: impl Fn(&str) -> Option<String>,
) -> Result<PathBuf, SessionError> {
    if let Some(val) = env("ZENITH_DATA_DIR")
        && !val.is_empty()
    {
        return Ok(PathBuf::from(val));
    }
    dirs::data_dir().map(|d| d.join("zenith")).ok_or_else(|| {
        SessionError::new(
            "cannot determine data directory \
                 (no ZENITH_DATA_DIR and platform data dir unavailable)",
        )
    })
}

/// Resolve the zenith data directory using real environment variables.
///
/// Wraps [`resolve_data_dir_with`] with [`std::env::var`].
pub fn resolve_data_dir() -> Result<PathBuf, SessionError> {
    resolve_data_dir_with(|k| std::env::var(k).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_override_is_used() {
        let result = resolve_data_dir_with(|_| Some("/custom/path".into()));
        assert_eq!(result.unwrap(), PathBuf::from("/custom/path"));
    }

    #[test]
    fn empty_override_falls_through_to_platform() {
        // Empty string must be ignored; result is either Ok (path ending in
        // "zenith") or Err (no platform data dir in CI).
        let result = resolve_data_dir_with(|k| {
            if k == "ZENITH_DATA_DIR" {
                Some(String::new()) // empty — must fall through
            } else {
                None
            }
        });
        match result {
            Ok(path) => {
                assert!(
                    path.ends_with("zenith"),
                    "expected path to end with 'zenith', got: {}",
                    path.display()
                );
            }
            Err(_) => {
                // Acceptable in CI environments with no platform data dir.
            }
        }
    }

    #[test]
    fn no_override_yields_platform_or_err() {
        let result = resolve_data_dir_with(|_| None);
        match result {
            Ok(path) => {
                assert!(
                    path.ends_with("zenith"),
                    "expected path to end with 'zenith', got: {}",
                    path.display()
                );
            }
            Err(e) => {
                assert!(e.message.contains("cannot determine data directory"));
            }
        }
    }
}
