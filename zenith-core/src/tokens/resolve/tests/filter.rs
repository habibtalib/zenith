//! Filter (incl. duotone) resolution tests.

use super::super::{ResolvedValue, resolve_tokens};
use super::{block, codes, duotone_filter_token, filter_token, has_code, literal_token};
use crate::ast::token::{FilterKind, TokenLiteral, TokenType};

#[test]
fn resolves_filter_with_ops() {
    let b = block(vec![filter_token(
        "filter.photo",
        vec![
            (FilterKind::Grayscale, Some(0.5)),
            (FilterKind::HueRotate, None),
        ],
    )]);
    let r = resolve_tokens(&b);
    assert!(
        r.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        r.diagnostics
    );
    match &r.resolved["filter.photo"].value {
        ResolvedValue::Filter(f) => {
            assert_eq!(f.ops.len(), 2);
            assert_eq!(f.ops[0].kind, FilterKind::Grayscale);
            assert_eq!(f.ops[0].amount, Some(0.5));
            assert_eq!(f.ops[1].kind, FilterKind::HueRotate);
            assert_eq!(f.ops[1].amount, None);
        }
        other => panic!("expected filter, got {other:?}"),
    }
}

#[test]
fn empty_filter_produces_no_ops() {
    let b = block(vec![filter_token("filter.empty", vec![])]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "filter.no_ops"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
    assert!(!r.resolved.contains_key("filter.empty"));
}

#[test]
fn filter_non_finite_amount_produces_invalid_amount() {
    let b = block(vec![filter_token(
        "filter.bad",
        vec![(FilterKind::Saturate, Some(f64::NAN))],
    )]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "filter.invalid_amount"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
    assert!(!r.resolved.contains_key("filter.bad"));
}

#[test]
fn filter_wrong_literal_type_produces_invalid_value() {
    let b = block(vec![literal_token(
        "filter.bad-shape",
        TokenType::Filter,
        TokenLiteral::String("grayscale".to_owned()),
    )]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "token.invalid_value"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
    assert!(!r.resolved.contains_key("filter.bad-shape"));
}

#[test]
fn resolves_duotone_with_both_colors() {
    let b = block(vec![
        literal_token(
            "color.sh",
            TokenType::Color,
            TokenLiteral::String("#000000".to_owned()),
        ),
        literal_token(
            "color.hi",
            TokenType::Color,
            TokenLiteral::String("#ffffff".to_owned()),
        ),
        duotone_filter_token("filter.duo", Some("color.sh"), Some("color.hi"), Some(0.8)),
    ]);
    let r = resolve_tokens(&b);
    assert!(
        r.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        r.diagnostics
    );
    match &r.resolved["filter.duo"].value {
        ResolvedValue::Filter(f) => {
            assert_eq!(f.ops.len(), 1);
            assert_eq!(f.ops[0].kind, FilterKind::Duotone);
            assert_eq!(f.ops[0].amount, Some(0.8));
            assert_eq!(f.ops[0].shadow.as_deref(), Some("color.sh"));
            assert_eq!(f.ops[0].highlight.as_deref(), Some("color.hi"));
        }
        other => panic!("expected filter, got {other:?}"),
    }
}

#[test]
fn duotone_missing_highlight_produces_missing_color() {
    let b = block(vec![
        literal_token(
            "color.sh",
            TokenType::Color,
            TokenLiteral::String("#000000".to_owned()),
        ),
        duotone_filter_token("filter.duo", Some("color.sh"), None, None),
    ]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "filter.duotone_missing_color"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
    assert!(!r.resolved.contains_key("filter.duo"));
}
