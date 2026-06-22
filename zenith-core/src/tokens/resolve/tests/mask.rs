//! Mask resolution tests.

use super::super::{ResolvedMask, ResolvedValue, resolve_tokens};
use super::{block, codes, has_code, literal_token, mask_token};
use crate::ast::token::{MaskShape, TokenLiteral, TokenType};

#[test]
fn resolves_mask_literal() {
    let b = block(vec![mask_token(
        "mask.vignette",
        MaskShape::RoundedRect,
        Some(40.0),
        60.0,
        true,
    )]);
    let r = resolve_tokens(&b);
    assert!(
        r.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        r.diagnostics
    );
    assert_eq!(
        r.resolved["mask.vignette"].value,
        ResolvedValue::Mask(ResolvedMask {
            shape: MaskShape::RoundedRect,
            radius: Some(40.0),
            feather: 60.0,
            invert: true,
        })
    );
}

#[test]
fn mask_negative_feather_produces_invalid_feather() {
    let b = block(vec![mask_token(
        "mask.bad",
        MaskShape::Rect,
        None,
        -5.0,
        false,
    )]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "mask.invalid_feather"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
    assert!(!r.resolved.contains_key("mask.bad"));
}

#[test]
fn mask_wrong_literal_type_produces_invalid_value() {
    let b = block(vec![literal_token(
        "mask.bad-shape",
        TokenType::Mask,
        TokenLiteral::String("rounded".to_owned()),
    )]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "token.invalid_value"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
    assert!(!r.resolved.contains_key("mask.bad-shape"));
}
