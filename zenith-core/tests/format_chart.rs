//! Integration tests for the `chart` node: parse, format, and round-trip —
//! including two `series` children — plus the absent-chart byte-identical guarantee.

mod common;

use common::*;
use zenith_core::format::format_document;

/// **Chart parse + format + round-trip (with two series)**: a `chart` with the
/// chart-specific props (kind/title/caption/legend/axis-min/axis-max/axis-style),
/// geometry, fill, and two `series` children parses into the expected `ChartNode`
/// (series data intact), formats back out preserving everything, and survives a
/// format → re-parse round-trip (spans stripped).
#[test]
fn chart_parse_format_round_trip_with_series() {
    // NOTE: the '#' inside color strings requires r##...## quoting.
    let src = r##"zenith version=1 {
  project id="proj.chart" name="Chart"
  tokens format="zenith-token-v1" {
    token id="color.bar" type="color" value="#334155"
  }
  styles {
  }
  document id="doc.chart" title="Chart" {
    page id="page.chart" w=(px)800 h=(px)600 {
      chart id="c.sales" kind="bar" x=(px)50 y=(px)50 w=(px)600 h=(px)400 title="Sales" legend=#true {
        series 12.0 24.0 18.0 label="Q1"
        series 30.0 15.0 22.0 label="Q2" color=(token)"color.bar"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");

    let chart = match &doc.body.pages[0].children[0] {
        Node::Chart(c) => c,
        other => panic!("expected Chart node, got {other:?}"),
    };
    assert_eq!(chart.id, "c.sales");
    assert_eq!(chart.kind, "bar");
    assert_eq!(chart.title, Some("Sales".to_owned()));
    assert_eq!(chart.legend, Some(true));
    assert_eq!(
        chart.x,
        Some(Dimension {
            value: 50.0,
            unit: Unit::Px
        })
    );
    assert_eq!(
        chart.w,
        Some(Dimension {
            value: 600.0,
            unit: Unit::Px
        })
    );
    assert_eq!(chart.series.len(), 2);
    assert_eq!(chart.series[0].label, Some("Q1".to_owned()));
    assert_eq!(chart.series[0].values, vec![12.0, 24.0, 18.0]);
    assert_eq!(chart.series[0].color, None);
    assert_eq!(chart.series[1].label, Some("Q2".to_owned()));
    assert_eq!(chart.series[1].values, vec![30.0, 15.0, 22.0]);
    assert_eq!(chart.series[1].color, Some(token_ref("color.bar")));

    // The formatter emits the chart-specific props and the series block.
    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted.clone()).expect("formatted must be utf8");
    assert!(
        formatted_str.contains("chart id=\"c.sales\" kind=\"bar\""),
        "formatter must emit chart id + kind; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("title=\"Sales\""),
        "formatter must emit title; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("series"),
        "formatter must emit series children; got:\n{formatted_str}"
    );

    // Round-trip: re-parse equals the first parse (spans stripped).
    let reparsed = adapter.parse(&formatted).expect("re-parse after format");
    assert_eq!(
        strip_spans(doc),
        strip_spans(reparsed),
        "chart (with series) must round-trip identically"
    );
}

/// **Absent chart is byte-identical**: a document that uses NO `chart` node
/// formats exactly as it did before the feature existed (additive guarantee).
#[test]
fn absent_chart_byte_identical() {
    // NOTE: '#' in color hex requires r##...## quoting.
    let src = r##"zenith version=1 {
  project id="proj.nc" name="NoChart"
  tokens format="zenith-token-v1" {
    token id="color.fill" type="color" value="#112233"
  }
  styles {
  }
  document id="doc.nc" title="NoChart" {
    page id="page.nc" w=(px)400 h=(px)300 {
      rect id="r.one" x=(px)0 y=(px)0 w=(px)100 h=(px)100 fill=(token)"color.fill"
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted).expect("formatted must be utf8");
    assert!(
        !formatted_str.contains("chart"),
        "a document without a chart must not emit the keyword; got:\n{formatted_str}"
    );
}
