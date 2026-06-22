//! Shadow resolution and layer cross-check tests.

use super::super::{ResolvedValue, resolve_tokens};
use super::{block, codes, has_code, literal_token, shadow_token};
use crate::ast::token::{TokenLiteral, TokenType};
use crate::ast::value::{Dimension, Unit};

#[test]
fn resolves_shadow_with_clamped_blur() {
    let b = block(vec![
        literal_token(
            "color.shadow.black",
            TokenType::Color,
            TokenLiteral::String("#000000".to_owned()),
        ),
        // Negative blur is clamped to 0; offsets pass through.
        shadow_token(
            "shadow.headline",
            vec![(8.0, 8.0, -4.0, "color.shadow.black")],
        ),
    ]);
    let r = resolve_tokens(&b);
    assert!(
        r.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        r.diagnostics
    );
    match &r.resolved["shadow.headline"].value {
        ResolvedValue::Shadow(s) => {
            assert_eq!(s.layers.len(), 1);
            let layer = &s.layers[0];
            assert_eq!(layer.dx, 8.0);
            assert_eq!(layer.dy, 8.0);
            assert_eq!(layer.blur, 0.0);
            assert_eq!(layer.color_token, "color.shadow.black");
        }
        other => panic!("expected shadow, got {other:?}"),
    }
}

#[test]
fn empty_shadow_produces_no_layers() {
    let b = block(vec![shadow_token("shadow.empty", vec![])]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "shadow.no_layers"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
    assert!(!r.resolved.contains_key("shadow.empty"));
}

#[test]
fn shadow_layer_missing_token_produces_layer_unresolved() {
    let b = block(vec![shadow_token(
        "shadow.bad",
        vec![(0.0, 0.0, 20.0, "color.does.not.exist")],
    )]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "shadow.layer_unresolved"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
}

#[test]
fn shadow_layer_wrong_type_produces_layer_wrong_type() {
    let b = block(vec![
        literal_token(
            "size.not-a-color",
            TokenType::Dimension,
            TokenLiteral::Dimension(Dimension {
                value: 4.0,
                unit: Unit::Px,
            }),
        ),
        shadow_token("shadow.bad", vec![(0.0, 0.0, 20.0, "size.not-a-color")]),
    ]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "shadow.layer_wrong_type"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
}
