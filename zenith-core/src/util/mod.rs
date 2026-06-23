//! Pure, platform-independent helper functions shared across Zenith.
//!
//! Everything here is deterministic and dependency-free: no time, no
//! randomness, no platform-specific behavior. The functions are reusable by
//! any backend (scene compilation, render, future backends) so that the same
//! inputs always yield the same bytes on every machine.

mod hash;

pub use hash::hash_unit;
