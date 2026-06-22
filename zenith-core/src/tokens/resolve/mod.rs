//! Token graph resolution: validates literals, follows alias chains, detects
//! cycles, and collects diagnostics — never hard-fails.
//!
//! This module root is wiring only. The driver (entry point, alias-chain walk,
//! cycle detection, and gradient/shadow cross-checks) lives in [`driver`]; the
//! per-type literal validators live in [`validate`]; the public resolved value
//! types live in [`types`].

mod driver;
mod types;
mod validate;

#[cfg(test)]
mod tests;

pub use driver::resolve_tokens;
pub use types::{
    ResolvedFilter, ResolvedFilterOp, ResolvedGradient, ResolvedMask, ResolvedShadow,
    ResolvedShadowLayer, ResolvedToken, ResolvedValue, TokenResolution,
};
