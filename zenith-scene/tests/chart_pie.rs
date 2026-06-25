//! Integration tests for pie and donut chart scene emission.
//!
//! Covers:
//! - `slice-colors` child node: declared fill colors must appear on the emitted
//!   `FillPolygon` paints (not the palette).
//! - Palette fallback: a chart WITHOUT `slice-colors` emits the same palette
//!   colors as before (byte-identical additive guarantee).

mod common;
use common::*;
use zenith_core::default_provider;
use zenith_scene::compile_page;
use zenith_scene::ir::{Color, Paint, SceneCommand};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Collect the solid-fill colors of every `FillPolygon` command in order.
fn fill_polygon_colors(result: &zenith_scene::CompileResult) -> Vec<Color> {
    result
        .scene
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::FillPolygon {
                paint: Paint::Solid { color },
                ..
            } => Some(*color),
            _ => None,
        })
        .collect()
}

// Palette slot 0 (blue) and slot 1 (red) — match SERIES_PALETTE in palette.rs.
const PALETTE_0: Color = Color::srgb(66, 133, 244, 255);
const PALETTE_1: Color = Color::srgb(234, 67, 53, 255);

// ── Palette-fallback (absent slice-colors → byte-identical) ──────────────────

/// Pie without `slice-colors` must use palette colors, unchanged from before.
#[test]
fn pie_without_slice_colors_uses_palette() {
    let src = r##"zenith version=1 {
  project id="proj.pf" name="PF"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.pf" title="PF" {
    page id="page.pf" w=(px)400 h=(px)400 {
      chart id="c.pf" kind="pie" x=(px)0 y=(px)0 w=(px)400 h=(px)400 {
        series 50.0 50.0
      }
    }
  }
}"##;

    let doc = parse(src);
    let result = compile_page(&doc, &default_provider(), 0, None);

    let colors = fill_polygon_colors(&result);
    assert_eq!(
        colors.len(),
        2,
        "two equal slices → two FillPolygon commands"
    );
    assert_eq!(
        colors[0], PALETTE_0,
        "slice 0 should use palette slot 0 (blue)"
    );
    assert_eq!(
        colors[1], PALETTE_1,
        "slice 1 should use palette slot 1 (red)"
    );
}

/// Donut without `slice-colors` must also use palette colors.
#[test]
fn donut_without_slice_colors_uses_palette() {
    let src = r##"zenith version=1 {
  project id="proj.df" name="DF"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.df" title="DF" {
    page id="page.df" w=(px)400 h=(px)400 {
      chart id="c.df" kind="donut" x=(px)0 y=(px)0 w=(px)400 h=(px)400 {
        series 50.0 50.0
      }
    }
  }
}"##;

    let doc = parse(src);
    let result = compile_page(&doc, &default_provider(), 0, None);

    let colors = fill_polygon_colors(&result);
    assert_eq!(
        colors.len(),
        2,
        "two equal slices → two FillPolygon commands"
    );
    assert_eq!(
        colors[0], PALETTE_0,
        "donut slice 0 should use palette slot 0"
    );
    assert_eq!(
        colors[1], PALETTE_1,
        "donut slice 1 should use palette slot 1"
    );
}

// ── slice-colors with token refs resolves to token values ────────────────────

/// A pie with `slice-colors` whose token values are distinct colors must emit
/// those exact colors on the FillPolygon paints — not the palette.
#[test]
fn pie_slice_colors_tokens_override_palette() {
    // Two tokens with distinctive red-channel values: 0xAA and 0x11.
    // Using r##"..."## because the hex strings contain '#'.
    let src = r##"zenith version=1 {
  project id="proj.sc" name="SC"
  tokens format="zenith-token-v1" {
    token id="color.s0" type="color" value="#aa2233"
    token id="color.s1" type="color" value="#115566"
  }
  styles {}
  document id="doc.sc" title="SC" {
    page id="page.sc" w=(px)400 h=(px)400 {
      chart id="c.sc" kind="pie" x=(px)0 y=(px)0 w=(px)400 h=(px)400 {
        slice-colors (token)"color.s0" (token)"color.s1"
        series 50.0 50.0
      }
    }
  }
}"##;

    let doc = parse(src);
    let result = compile_page(&doc, &default_provider(), 0, None);

    // No unexpected diagnostics from slice-colors.
    let sc_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("chart.") || d.code.starts_with("token."))
        .collect();
    assert!(
        sc_diags.is_empty(),
        "no chart/token diagnostics expected; got: {sc_diags:?}"
    );

    let colors = fill_polygon_colors(&result);
    assert_eq!(colors.len(), 2, "two slices → two FillPolygon commands");

    // color.s0 = #aa2233 → r=0xAA=170, g=0x22=34, b=0x33=51.
    assert_eq!(
        colors[0],
        Color::srgb(0xAA, 0x22, 0x33, 0xFF),
        "slice 0 fill must be color.s0 (#aa2233), not palette"
    );
    // color.s1 = #115566 → r=0x11=17, g=0x55=85, b=0x66=102.
    assert_eq!(
        colors[1],
        Color::srgb(0x11, 0x55, 0x66, 0xFF),
        "slice 1 fill must be color.s1 (#115566), not palette"
    );
}

/// A donut with `slice-colors` applies the same token override on annulus rings.
#[test]
fn donut_slice_colors_tokens_override_palette() {
    let src = r##"zenith version=1 {
  project id="proj.dsc" name="DSC"
  tokens format="zenith-token-v1" {
    token id="color.d0" type="color" value="#cc3344"
    token id="color.d1" type="color" value="#225577"
  }
  styles {}
  document id="doc.dsc" title="DSC" {
    page id="page.dsc" w=(px)400 h=(px)400 {
      chart id="c.dsc" kind="donut" x=(px)0 y=(px)0 w=(px)400 h=(px)400 {
        slice-colors (token)"color.d0" (token)"color.d1"
        series 60.0 40.0
      }
    }
  }
}"##;

    let doc = parse(src);
    let result = compile_page(&doc, &default_provider(), 0, None);

    let colors = fill_polygon_colors(&result);
    assert_eq!(colors.len(), 2, "two slices → two FillPolygon commands");

    assert_eq!(
        colors[0],
        Color::srgb(0xCC, 0x33, 0x44, 0xFF),
        "donut slice 0 fill must be color.d0 (#cc3344)"
    );
    assert_eq!(
        colors[1],
        Color::srgb(0x22, 0x55, 0x77, 0xFF),
        "donut slice 1 fill must be color.d1 (#225577)"
    );
}

/// Partial `slice-colors` (fewer entries than slices): declared slices use the
/// token, undeclared slices fall back to the palette.
#[test]
fn pie_partial_slice_colors_falls_back_for_undeclared_slices() {
    let src = r##"zenith version=1 {
  project id="proj.psc" name="PSC"
  tokens format="zenith-token-v1" {
    token id="color.only" type="color" value="#ff8800"
  }
  styles {}
  document id="doc.psc" title="PSC" {
    page id="page.psc" w=(px)400 h=(px)400 {
      chart id="c.psc" kind="pie" x=(px)0 y=(px)0 w=(px)400 h=(px)400 {
        slice-colors (token)"color.only"
        series 33.0 33.0 34.0
      }
    }
  }
}"##;

    let doc = parse(src);
    let result = compile_page(&doc, &default_provider(), 0, None);

    let colors = fill_polygon_colors(&result);
    assert_eq!(colors.len(), 3, "three slices → three FillPolygon commands");

    // Slice 0: declared → #ff8800 = r=0xFF, g=0x88, b=0x00.
    assert_eq!(
        colors[0],
        Color::srgb(0xFF, 0x88, 0x00, 0xFF),
        "slice 0 must use the declared token color"
    );
    // Slices 1 and 2: undeclared → palette slots 1 and 2.
    assert_eq!(
        colors[1], PALETTE_1,
        "slice 1 must fall back to palette slot 1"
    );
    // Palette slot 2 = green.
    assert_eq!(
        colors[2],
        Color::srgb(52, 168, 83, 255),
        "slice 2 must fall back to palette slot 2 (green)"
    );
}
