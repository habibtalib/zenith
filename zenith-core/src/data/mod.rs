//! Runtime data-binding support.
//!
//! Provides [`DataContext`] (a flat `BTreeMap<String, String>` of named field
//! values, plus a `BTreeMap<String, Vec<String>>` of named array columns) and
//! the [`DataFormat`] / [`format_data_value`] formatter that turns raw field
//! strings into locale-styled display strings deterministically.

use std::collections::BTreeMap;

pub mod format;

pub use format::{DataFormat, format_data_value};

/// Named data fields and ordered array columns available at scene-compile time.
///
/// `fields` holds scalar bindings keyed by dot-separated path (e.g.
/// `"revenue.total"`). `arrays` holds ordered sequences of raw element strings
/// keyed by name (e.g. `"sales"` → `["12", "18", "15"]`), populated from JSON
/// array values and CSV column columns.
///
/// Both maps use [`BTreeMap`] for deterministic iteration order on the render
/// path. No `HashMap`, no randomness, no time.
#[derive(Debug, Clone, Default)]
pub struct DataContext {
    /// Scalar field map. Keyed by dotted path, value is the raw string.
    pub fields: BTreeMap<String, String>,
    /// Array column map. Keyed by name, value is the ordered raw element strings.
    pub arrays: BTreeMap<String, Vec<String>>,
}

impl DataContext {
    /// Look up a scalar field value by `path`.
    ///
    /// Returns `None` when the path is not present in this context.
    pub fn get(&self, path: &str) -> Option<&str> {
        self.fields.get(path).map(String::as_str)
    }

    /// Look up an ordered array column by `key`.
    ///
    /// Returns the element slice in source order, or `None` when the key is
    /// not present. Used by the data-binding pre-pass to populate
    /// `ChartSeries.values` from a `data-ref` binding.
    pub fn get_array(&self, key: &str) -> Option<&[String]> {
        self.arrays.get(key).map(Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_array_returns_slice_for_present_key() {
        let mut ctx = DataContext::default();
        ctx.arrays.insert(
            "sales".to_owned(),
            vec!["12".to_owned(), "18".to_owned(), "15".to_owned()],
        );
        let got: Vec<&str> = ctx
            .get_array("sales")
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(got, ["12", "18", "15"]);
    }

    #[test]
    fn get_array_returns_none_for_missing_key() {
        let ctx = DataContext::default();
        assert!(ctx.get_array("missing").is_none());
    }

    #[test]
    fn get_scalar_unaffected_by_arrays() {
        let mut ctx = DataContext::default();
        ctx.fields.insert("x".to_owned(), "hello".to_owned());
        ctx.arrays
            .insert("x".to_owned(), vec!["a".to_owned(), "b".to_owned()]);
        // Scalar and array can coexist under the same key without interference.
        assert_eq!(ctx.get("x"), Some("hello"));
        let got: Vec<&str> = ctx
            .get_array("x")
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(got, ["a", "b"]);
    }

    #[test]
    fn default_has_empty_arrays() {
        let ctx = DataContext::default();
        assert!(ctx.arrays.is_empty());
    }
}
