//! Resolver unit tests, grouped by concern. Shared builder helpers live here;
//! each submodule exercises one token family.

use crate::ast::token::{
    FilterKind, FilterLiteral, FilterOp, GradientKind, GradientLiteral, GradientStopRef,
    MaskLiteral, MaskShape, ShadowLayerRef, ShadowLiteral, Token, TokenBlock, TokenLiteral,
    TokenType, TokenValue,
};
use crate::diagnostics::Diagnostic;

mod filter;
mod gradient;
mod literals;
mod mask;
mod shadow;

// ── Builder helpers ───────────────────────────────────────────────────

pub(super) fn literal_token(id: &str, token_type: TokenType, literal: TokenLiteral) -> Token {
    Token {
        id: id.to_owned(),
        token_type,
        value: TokenValue::Literal(literal),
        source_span: None,
    }
}

pub(super) fn alias_token(id: &str, token_type: TokenType, target: &str) -> Token {
    Token {
        id: id.to_owned(),
        token_type,
        value: TokenValue::Reference {
            token_id: target.to_owned(),
        },
        source_span: None,
    }
}

pub(super) fn block(tokens: Vec<Token>) -> TokenBlock {
    TokenBlock {
        format: "zenith-token-v1".to_owned(),
        tokens,
    }
}

pub(super) fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
    diagnostics.iter().any(|d| d.code == code)
}

pub(super) fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(|d| d.code.as_str()).collect()
}

pub(super) fn gradient_token(id: &str, angle_deg: f64, stops: Vec<(f64, &str)>) -> Token {
    Token {
        id: id.to_owned(),
        token_type: TokenType::Gradient,
        value: TokenValue::Literal(TokenLiteral::Gradient(GradientLiteral {
            kind: GradientKind::Linear,
            angle_deg,
            center_x: None,
            center_y: None,
            radius: None,
            stops: stops
                .into_iter()
                .map(|(offset, color)| GradientStopRef {
                    offset,
                    color_token: color.to_owned(),
                })
                .collect(),
        })),
        source_span: None,
    }
}

pub(super) fn radial_gradient_token(
    id: &str,
    center_x: Option<f64>,
    center_y: Option<f64>,
    radius: Option<f64>,
    stops: Vec<(f64, &str)>,
) -> Token {
    Token {
        id: id.to_owned(),
        token_type: TokenType::Gradient,
        value: TokenValue::Literal(TokenLiteral::Gradient(GradientLiteral {
            kind: GradientKind::Radial,
            angle_deg: 90.0,
            center_x,
            center_y,
            radius,
            stops: stops
                .into_iter()
                .map(|(offset, color)| GradientStopRef {
                    offset,
                    color_token: color.to_owned(),
                })
                .collect(),
        })),
        source_span: None,
    }
}

pub(super) fn shadow_token(id: &str, layers: Vec<(f64, f64, f64, &str)>) -> Token {
    Token {
        id: id.to_owned(),
        token_type: TokenType::Shadow,
        value: TokenValue::Literal(TokenLiteral::Shadow(ShadowLiteral {
            layers: layers
                .into_iter()
                .map(|(dx, dy, blur, color)| ShadowLayerRef {
                    dx,
                    dy,
                    blur,
                    color_token: color.to_owned(),
                })
                .collect(),
        })),
        source_span: None,
    }
}

pub(super) fn filter_token(id: &str, ops: Vec<(FilterKind, Option<f64>)>) -> Token {
    Token {
        id: id.to_owned(),
        token_type: TokenType::Filter,
        value: TokenValue::Literal(TokenLiteral::Filter(FilterLiteral {
            ops: ops
                .into_iter()
                .map(|(kind, amount)| FilterOp {
                    kind,
                    amount,
                    shadow: None,
                    highlight: None,
                })
                .collect(),
        })),
        source_span: None,
    }
}

/// Build a filter token with a single `duotone` op carrying the given
/// shadow/highlight color token ids (either may be `None` to exercise the
/// missing-color diagnostic).
pub(super) fn duotone_filter_token(
    id: &str,
    shadow: Option<&str>,
    highlight: Option<&str>,
    amount: Option<f64>,
) -> Token {
    Token {
        id: id.to_owned(),
        token_type: TokenType::Filter,
        value: TokenValue::Literal(TokenLiteral::Filter(FilterLiteral {
            ops: vec![FilterOp {
                kind: FilterKind::Duotone,
                amount,
                shadow: shadow.map(str::to_owned),
                highlight: highlight.map(str::to_owned),
            }],
        })),
        source_span: None,
    }
}

pub(super) fn mask_token(
    id: &str,
    shape: MaskShape,
    radius: Option<f64>,
    feather: f64,
    invert: bool,
) -> Token {
    Token {
        id: id.to_owned(),
        token_type: TokenType::Mask,
        value: TokenValue::Literal(TokenLiteral::Mask(MaskLiteral {
            shape,
            radius,
            feather,
            invert,
        })),
        source_span: None,
    }
}
