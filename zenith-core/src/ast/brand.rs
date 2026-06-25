//! Brand-contract AST type.
//!
//! A `brand { … }` block (sibling of `diagnostics { … }` inside `zenith { … }`)
//! declares the set of approved colors, font families, and font weights for the
//! document. Absent child nodes mean "unconstrained" for that category; an absent
//! `brand` block altogether is an identity pass (no brand checks).

use super::Span;

/// The brand contract parsed from a root `brand { … }` block.
///
/// Each field is an `Option<Vec<_>>`:
/// - `None` means the child node was absent → that category is unconstrained.
/// - `Some(vec)` means the child node was present → the vec lists the approved
///   values for that category (may be empty if the author wrote, e.g., `colors`
///   with no arguments, which means NO color is approved).
///
/// An absent `brand { … }` block is represented as the `Default` (all `None`),
/// which is identical to [`BrandContract::is_empty`] returning `true`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BrandContract {
    /// Approved color hex strings (case-normalised to lowercase at parse time),
    /// or `None` when the `colors` child was absent (unconstrained).
    pub allowed_colors: Option<Vec<String>>,
    /// Approved font-family names, or `None` when the `fonts` child was absent
    /// (unconstrained).
    pub allowed_fonts: Option<Vec<String>>,
    /// Approved font weights (integers in 100..=900), or `None` when the
    /// `weights` child was absent (unconstrained).
    pub allowed_weights: Option<Vec<u32>>,
    /// Source span of the `brand { … }` node, when available.
    pub source_span: Option<Span>,
}

impl BrandContract {
    /// True when no category is constrained — i.e. the `brand` block was absent
    /// or declared with no children that the engine recognises.
    ///
    /// The formatter uses this to decide whether to emit the block at all:
    /// when `is_empty()` returns `true`, nothing is emitted, preserving
    /// byte-identical output for documents that have no brand contract.
    pub fn is_empty(&self) -> bool {
        self.allowed_colors.is_none()
            && self.allowed_fonts.is_none()
            && self.allowed_weights.is_none()
    }
}

/// Merge two [`BrandContract`]s with per-category override semantics.
///
/// For each category (`allowed_colors`, `allowed_fonts`, `allowed_weights`),
/// `over`'s value wins when `Some`; `base`'s value is used as the fallback
/// when `over`'s value is `None`. This mirrors the per-entry last-wins
/// precedence used by the diagnostic policy, applied at the category level:
/// a config layer that declares a category replaces the same category from a
/// lower-precedence layer, while absent categories in `over` do not
/// "erase" those in `base`.
///
/// `source_span` is taken from `over` when `Some`, otherwise from `base`.
///
/// The function is pure (no I/O, no mutation) and can be chained:
/// ```ignore
/// let effective = merge_brand_contract(
///     &merge_brand_contract(&global, &local),
///     &doc_brand,
/// );
/// ```
pub fn merge_brand_contract(base: &BrandContract, over: &BrandContract) -> BrandContract {
    BrandContract {
        allowed_colors: over
            .allowed_colors
            .clone()
            .or_else(|| base.allowed_colors.clone()),
        allowed_fonts: over
            .allowed_fonts
            .clone()
            .or_else(|| base.allowed_fonts.clone()),
        allowed_weights: over
            .allowed_weights
            .clone()
            .or_else(|| base.allowed_weights.clone()),
        source_span: over.source_span.or(base.source_span),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colors(hexes: &[&str]) -> BrandContract {
        BrandContract {
            allowed_colors: Some(hexes.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }

    fn fonts(names: &[&str]) -> BrandContract {
        BrandContract {
            allowed_fonts: Some(names.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }

    fn full(colors: &[&str], fonts: &[&str], weights: &[u32]) -> BrandContract {
        BrandContract {
            allowed_colors: Some(colors.iter().map(|s| s.to_string()).collect()),
            allowed_fonts: Some(fonts.iter().map(|s| s.to_string()).collect()),
            allowed_weights: Some(weights.to_vec()),
            source_span: None,
        }
    }

    #[test]
    fn both_empty_yields_empty() {
        let result = merge_brand_contract(&BrandContract::default(), &BrandContract::default());
        assert!(result.is_empty());
    }

    #[test]
    fn over_wins_per_category() {
        let base = full(&["#000000"], &["Arial"], &[400]);
        let over = full(&["#ffffff"], &["Roboto"], &[700]);
        let result = merge_brand_contract(&base, &over);
        assert_eq!(result.allowed_colors, Some(vec!["#ffffff".to_owned()]));
        assert_eq!(result.allowed_fonts, Some(vec!["Roboto".to_owned()]));
        assert_eq!(result.allowed_weights, Some(vec![700u32]));
    }

    #[test]
    fn absent_category_in_over_falls_back_to_base() {
        // over has only colors; fonts/weights fall back from base.
        let base = full(&["#000000"], &["Arial"], &[400]);
        let over = colors(&["#ffffff"]);
        let result = merge_brand_contract(&base, &over);
        assert_eq!(result.allowed_colors, Some(vec!["#ffffff".to_owned()]));
        assert_eq!(result.allowed_fonts, Some(vec!["Arial".to_owned()]));
        assert_eq!(result.allowed_weights, Some(vec![400u32]));
    }

    #[test]
    fn empty_over_is_identity() {
        let base = full(&["#abc"], &["Noto Sans"], &[300, 400]);
        let result = merge_brand_contract(&base, &BrandContract::default());
        assert_eq!(result.allowed_colors, base.allowed_colors);
        assert_eq!(result.allowed_fonts, base.allowed_fonts);
        assert_eq!(result.allowed_weights, base.allowed_weights);
    }

    #[test]
    fn empty_base_with_over_yields_over() {
        let over = fonts(&["Helvetica"]);
        let result = merge_brand_contract(&BrandContract::default(), &over);
        assert_eq!(result.allowed_fonts, Some(vec!["Helvetica".to_owned()]));
        assert!(result.allowed_colors.is_none());
        assert!(result.allowed_weights.is_none());
    }

    #[test]
    fn chained_merge_is_transitive() {
        // global = colors, local = fonts, doc = weights → all three in effective.
        let global = colors(&["#111111"]);
        let local = fonts(&["Roboto"]);
        let doc = BrandContract {
            allowed_weights: Some(vec![400u32]),
            ..Default::default()
        };
        let effective = merge_brand_contract(&merge_brand_contract(&global, &local), &doc);
        assert_eq!(effective.allowed_colors, Some(vec!["#111111".to_owned()]));
        assert_eq!(effective.allowed_fonts, Some(vec!["Roboto".to_owned()]));
        assert_eq!(effective.allowed_weights, Some(vec![400u32]));
    }
}
