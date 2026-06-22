//! Gradient (linear + radial) resolution and stop cross-check tests.

use super::super::{ResolvedValue, resolve_tokens};
use super::{block, codes, gradient_token, has_code, literal_token, radial_gradient_token};
use crate::ast::token::{GradientKind, TokenLiteral, TokenType};
use crate::ast::value::{Dimension, Unit};

#[test]
fn resolves_gradient_with_clamped_offsets() {
    let b = block(vec![
        literal_token(
            "color.top",
            TokenType::Color,
            TokenLiteral::String("#001122".to_owned()),
        ),
        literal_token(
            "color.bottom",
            TokenType::Color,
            TokenLiteral::String("#334455".to_owned()),
        ),
        // Offsets out of range get clamped into 0.0..=1.0.
        gradient_token(
            "gradient.bg.hero",
            90.0,
            vec![(-0.5, "color.top"), (1.5, "color.bottom")],
        ),
    ]);
    let r = resolve_tokens(&b);
    assert!(
        r.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        r.diagnostics
    );
    match &r.resolved["gradient.bg.hero"].value {
        ResolvedValue::Gradient(g) => {
            assert_eq!(g.angle_deg, 90.0);
            assert_eq!(
                g.stops,
                vec![
                    (0.0, "color.top".to_owned()),
                    (1.0, "color.bottom".to_owned()),
                ]
            );
        }
        other => panic!("expected gradient, got {other:?}"),
    }
}

#[test]
fn gradient_with_one_stop_produces_too_few_stops() {
    let b = block(vec![
        literal_token(
            "color.top",
            TokenType::Color,
            TokenLiteral::String("#001122".to_owned()),
        ),
        gradient_token("gradient.bad", 90.0, vec![(0.0, "color.top")]),
    ]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "gradient.too_few_stops"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
    assert!(!r.resolved.contains_key("gradient.bad"));
}

#[test]
fn gradient_stop_missing_token_produces_stop_unresolved() {
    let b = block(vec![
        literal_token(
            "color.top",
            TokenType::Color,
            TokenLiteral::String("#001122".to_owned()),
        ),
        gradient_token(
            "gradient.bg",
            90.0,
            vec![(0.0, "color.top"), (1.0, "color.does.not.exist")],
        ),
    ]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "gradient.stop_unresolved"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
}

#[test]
fn gradient_stop_wrong_type_produces_stop_wrong_type() {
    let b = block(vec![
        literal_token(
            "color.top",
            TokenType::Color,
            TokenLiteral::String("#001122".to_owned()),
        ),
        literal_token(
            "size.not-a-color",
            TokenType::Dimension,
            TokenLiteral::Dimension(Dimension {
                value: 4.0,
                unit: Unit::Px,
            }),
        ),
        gradient_token(
            "gradient.bg",
            90.0,
            vec![(0.0, "color.top"), (1.0, "size.not-a-color")],
        ),
    ]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "gradient.stop_wrong_type"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
}

// ── Radial gradient resolution ────────────────────────────────────────

#[test]
fn resolves_radial_gradient_with_params() {
    let b = block(vec![
        literal_token(
            "color.inner",
            TokenType::Color,
            TokenLiteral::String("#ffffff".to_owned()),
        ),
        literal_token(
            "color.outer",
            TokenType::Color,
            TokenLiteral::String("#000000".to_owned()),
        ),
        radial_gradient_token(
            "gradient.radial.hero",
            Some(0.5),
            Some(0.5),
            Some(0.8),
            vec![(0.0, "color.inner"), (1.0, "color.outer")],
        ),
    ]);
    let r = resolve_tokens(&b);
    assert!(
        r.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        r.diagnostics
    );
    match &r.resolved["gradient.radial.hero"].value {
        ResolvedValue::Gradient(g) => {
            assert_eq!(g.kind, GradientKind::Radial);
            assert_eq!(g.center_x, Some(0.5));
            assert_eq!(g.center_y, Some(0.5));
            assert_eq!(g.radius, Some(0.8));
            assert_eq!(
                g.stops,
                vec![
                    (0.0, "color.inner".to_owned()),
                    (1.0, "color.outer".to_owned()),
                ]
            );
        }
        other => panic!("expected gradient, got {other:?}"),
    }
}

#[test]
fn radial_gradient_zero_radius_produces_invalid_radius() {
    let b = block(vec![
        literal_token(
            "color.a",
            TokenType::Color,
            TokenLiteral::String("#aabbcc".to_owned()),
        ),
        literal_token(
            "color.b",
            TokenType::Color,
            TokenLiteral::String("#112233".to_owned()),
        ),
        radial_gradient_token(
            "gradient.bad.radius",
            None,
            None,
            Some(0.0), // zero radius → invalid
            vec![(0.0, "color.a"), (1.0, "color.b")],
        ),
    ]);
    let r = resolve_tokens(&b);
    assert!(
        has_code(&r.diagnostics, "gradient.invalid_radius"),
        "codes: {:?}",
        codes(&r.diagnostics)
    );
    assert!(!r.resolved.contains_key("gradient.bad.radius"));
}
