//! Command implementations for the Zenith CLI.
//!
//! Each submodule exposes a pure function whose core logic operates on
//! in-memory source bytes/strings — never touching the filesystem.  File I/O
//! (reading the document, writing formatted source or rendered output) is the
//! responsibility of the dispatcher in `lib.rs`.

pub mod fmt;
pub mod render;
pub mod tokens;
pub mod tx;
pub mod validate;

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Serialise `value` to pretty-printed JSON, falling back to the error
/// message string if serialisation itself fails (which cannot happen for
/// these well-typed DTOs, but is kept as a safe fallback).
pub(crate) fn serialize_pretty<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| e.to_string())
}
