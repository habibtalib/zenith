//! Validation tests for the `chart` node.
//!
//! Covers valid chart kinds, `chart.invalid_kind` on bogus kinds, geometry
//! requirements, id-uniqueness participation, and the absent-chart additive
//! guarantee.

mod common;

use common::*;

/// A valid `bar` chart with geometry and series produces no chart-specific
/// diagnostics.
#[test]
fn chart_valid_bar_no_diagnostics() {
    // NOTE: '#' inside color hex requires r##...## quoting.
    let src = r##"zenith version=1 {
  project id="proj.vb" name="ValidBar"
  tokens format="zenith-token-v1" {
    token id="color.bg" type="color" value="#ffffff"
  }
  styles {
  }
  document id="doc.vb" title="ValidBar" {
    page id="page.vb" w=(px)800 h=(px)600 {
      chart id="c.bar" kind="bar" x=(px)50 y=(px)50 w=(px)600 h=(px)400 {
        series 10.0 20.0 30.0 label="A"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let report = validate(&doc);
    let chart_diags: Vec<&str> = codes(&report)
        .into_iter()
        .filter(|c| c.starts_with("chart."))
        .collect();
    assert!(
        chart_diags.is_empty(),
        "valid bar chart must produce no chart.* diagnostics; got: {:?}",
        chart_diags
    );
}

/// A bogus `kind` value fires `chart.invalid_kind`.
#[test]
fn chart_invalid_kind_fires_diagnostic() {
    let src = r##"zenith version=1 {
  project id="proj.ik" name="InvalidKind"
  styles {
  }
  document id="doc.ik" title="InvalidKind" {
    page id="page.ik" w=(px)800 h=(px)600 {
      chart id="c.bad" kind="histogram" x=(px)0 y=(px)0 w=(px)400 h=(px)300 {
        series 1.0 2.0 label="X"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let report = validate(&doc);
    assert!(
        has_code(&report, "chart.invalid_kind"),
        "bogus chart kind must fire chart.invalid_kind; got: {:?}",
        codes(&report)
    );
}

/// All five valid kind values ("bar", "line", "sparkline", "pie", "donut")
/// produce no `chart.invalid_kind`.
#[test]
fn chart_all_valid_kinds() {
    for kind in ["bar", "line", "sparkline", "pie", "donut"] {
        let src = format!(
            r##"zenith version=1 {{
  project id="proj.vk" name="ValidKind"
  styles {{
  }}
  document id="doc.vk" title="ValidKind" {{
    page id="page.vk" w=(px)800 h=(px)600 {{
      chart id="c.kind" kind="{kind}" x=(px)0 y=(px)0 w=(px)400 h=(px)300 {{
        series 1.0 2.0 label="S"
      }}
    }}
  }}
}}
"##
        );
        let adapter = KdlAdapter;
        let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
        let report = validate(&doc);
        assert!(
            !has_code(&report, "chart.invalid_kind"),
            "kind={kind:?} must not fire chart.invalid_kind; got: {:?}",
            codes(&report)
        );
    }
}

/// A chart whose id duplicates another node's id fires `id.duplicate` — proving
/// the chart participates in id-uniqueness checking.
#[test]
fn chart_duplicate_id_fires_id_duplicate() {
    let src = r##"zenith version=1 {
  project id="proj.dup" name="Dup"
  styles {
  }
  document id="doc.dup" title="Dup" {
    page id="page.dup" w=(px)800 h=(px)600 {
      rect id="node.dup" x=(px)0 y=(px)0 w=(px)10 h=(px)10
      chart id="node.dup" kind="bar" x=(px)0 y=(px)0 w=(px)400 h=(px)300 {
        series 1.0 label="A"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let report = validate(&doc);
    assert!(
        has_code(&report, "id.duplicate"),
        "chart id duplicate must fire id.duplicate; got: {:?}",
        codes(&report)
    );
}

/// A chart missing geometry (x/y/w/h absent) outside a flow parent fires
/// `node.missing_geometry`, proving that geometry validation runs on chart nodes.
#[test]
fn chart_missing_geometry_fires_geometry_diagnostic() {
    let src = r##"zenith version=1 {
  project id="proj.mg" name="MissingGeom"
  styles {
  }
  document id="doc.mg" title="MissingGeom" {
    page id="page.mg" w=(px)800 h=(px)600 {
      chart id="c.nogeom" kind="line" {
        series 1.0 2.0 label="S"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let report = validate(&doc);
    let geom_diags: Vec<&str> = codes(&report)
        .into_iter()
        .filter(|c| *c == "node.missing_geometry")
        .collect();
    assert!(
        !geom_diags.is_empty(),
        "chart missing geometry must fire node.missing_geometry; got: {:?}",
        codes(&report)
    );
}
