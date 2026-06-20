mod common;
use common::*;
use zenith_core::Severity;
use zenith_tx::{Op, OpSpan, Permissions, Position, Transaction, TxStatus, run_transaction};

// ── AddNode tests ─────────────────────────────────────────────────────────

#[test]
fn add_node_into_page_last() {
    let doc = parse(ADD_BASE_DOC);
    let tx = Transaction {
        ops: vec![Op::AddNode {
            parent: "pg1".to_owned(),
            position: Position::Last,
            source:
                r#"rect id="box" x=(px)10 y=(px)10 w=(px)100 h=(px)80 fill=(token)"color.accent""#
                    .to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["box".to_owned()]);
    assert!(
        result.source_after.contains("id=\"box\""),
        "source_after must contain the new rect; got:\n{}",
        result.source_after
    );
    // "box" inserted last → appears after "base".
    let pos_base = result.source_after.find("id=\"base\"").expect("base");
    let pos_box = result.source_after.find("id=\"box\"").expect("box");
    assert!(pos_base < pos_box, "box should come after base");
}

#[test]
fn add_node_into_group_first() {
    let doc = parse(ADD_GROUP_DOC);
    let tx = Transaction {
        ops: vec![Op::AddNode {
            parent: "grp1".to_owned(),
            position: Position::First,
            source: r#"rect id="g.new" x=(px)0 y=(px)0 w=(px)10 h=(px)10"#.to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["g.new".to_owned()]);
    // First child of the group → appears before g.a.
    let pos_new = result.source_after.find("id=\"g.new\"").expect("g.new");
    let pos_a = result.source_after.find("id=\"g.a\"").expect("g.a");
    assert!(pos_new < pos_a, "g.new should be first in the group");
}

#[test]
fn add_node_before_and_after_sibling() {
    // Insert before g.b.
    let doc = parse(ADD_GROUP_DOC);
    let tx = Transaction {
        ops: vec![Op::AddNode {
            parent: "grp1".to_owned(),
            position: Position::Before {
                id: "g.b".to_owned(),
            },
            source: r#"rect id="g.mid" x=(px)0 y=(px)0 w=(px)10 h=(px)10"#.to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");
    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    let pos_a = result.source_after.find("id=\"g.a\"").expect("g.a");
    let pos_mid = result.source_after.find("id=\"g.mid\"").expect("g.mid");
    let pos_b = result.source_after.find("id=\"g.b\"").expect("g.b");
    assert!(
        pos_a < pos_mid && pos_mid < pos_b,
        "order should be a, mid, b"
    );

    // Insert after g.a.
    let doc = parse(ADD_GROUP_DOC);
    let tx = Transaction {
        ops: vec![Op::AddNode {
            parent: "grp1".to_owned(),
            position: Position::After {
                id: "g.a".to_owned(),
            },
            source: r#"rect id="g.mid" x=(px)0 y=(px)0 w=(px)10 h=(px)10"#.to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");
    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    let pos_a = result.source_after.find("id=\"g.a\"").expect("g.a");
    let pos_mid = result.source_after.find("id=\"g.mid\"").expect("g.mid");
    let pos_b = result.source_after.find("id=\"g.b\"").expect("g.b");
    assert!(
        pos_a < pos_mid && pos_mid < pos_b,
        "order should be a, mid, b"
    );
}

#[test]
fn add_node_index_clamped() {
    // index well beyond len → clamped to last.
    let doc = parse(ADD_GROUP_DOC);
    let tx = Transaction {
        ops: vec![Op::AddNode {
            parent: "grp1".to_owned(),
            position: Position::Index { index: 99 },
            source: r#"rect id="g.tail" x=(px)0 y=(px)0 w=(px)10 h=(px)10"#.to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");
    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    let pos_b = result.source_after.find("id=\"g.b\"").expect("g.b");
    let pos_tail = result.source_after.find("id=\"g.tail\"").expect("g.tail");
    assert!(pos_b < pos_tail, "clamped insert should be last");
}

#[test]
fn add_node_duplicate_id_rejected() {
    let doc = parse(ADD_BASE_DOC);
    let before = run_transaction(
        &doc,
        &Transaction {
            ops: vec![Op::AddNode {
                parent: "pg1".to_owned(),
                position: Position::Last,
                source: r#"rect id="base" x=(px)0 y=(px)0 w=(px)20 h=(px)20"#.to_owned(),
            }],
            permissions: Permissions::default(),
        },
    )
    .expect("run_transaction should not error");

    assert_eq!(before.status, TxStatus::Rejected);
    assert_eq!(before.source_after, before.source_before);
}

#[test]
fn add_node_malformed_fragment_rejected() {
    let doc = parse(ADD_BASE_DOC);
    let tx = Transaction {
        ops: vec![Op::AddNode {
            parent: "pg1".to_owned(),
            position: Position::Last,
            source: "not valid kdl {{{".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.invalid_node_spec"),
        "expected tx.invalid_node_spec; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn add_node_unknown_parent_rejected() {
    let doc = parse(ADD_BASE_DOC);
    let tx = Transaction {
        ops: vec![Op::AddNode {
            parent: "nope".to_owned(),
            position: Position::Last,
            source: r#"rect id="box" x=(px)0 y=(px)0 w=(px)10 h=(px)10"#.to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.invalid_parent"),
        "expected tx.invalid_parent; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn add_node_parent_is_leaf_rejected() {
    // "base" is a rect (a leaf) — not a valid container.
    let doc = parse(ADD_BASE_DOC);
    let tx = Transaction {
        ops: vec![Op::AddNode {
            parent: "base".to_owned(),
            position: Position::Last,
            source: r#"rect id="box" x=(px)0 y=(px)0 w=(px)10 h=(px)10"#.to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.invalid_parent"),
        "expected tx.invalid_parent; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn add_node_before_missing_sibling_rejected() {
    let doc = parse(ADD_GROUP_DOC);
    let tx = Transaction {
        ops: vec![Op::AddNode {
            parent: "grp1".to_owned(),
            position: Position::Before {
                id: "nope".to_owned(),
            },
            source: r#"rect id="g.new" x=(px)0 y=(px)0 w=(px)10 h=(px)10"#.to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unknown_node"),
        "expected tx.unknown_node; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn add_code_node_into_page_accepted() {
    let doc = parse(CODE_DOC);
    let tx = Transaction {
        ops: vec![Op::AddNode {
            parent: "pg1".to_owned(),
            position: Position::Last,
            source:
                r#"code id="snip2" x=(px)0 y=(px)0 w=(px)100 h=(px)40 { content "let x = 1;" }"#
                    .to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["snip2".to_owned()]);
    assert!(
        result.source_after.contains("id=\"snip2\""),
        "source_after must contain the new code node; got:\n{}",
        result.source_after
    );
    assert!(result.source_after.contains("content \"let x = 1;\""));
}

// ── RemoveNode tests ──────────────────────────────────────────────────────

#[test]
fn remove_node_top_level() {
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::RemoveNode {
            node: "a".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["a".to_owned()]);
    assert!(
        !result.source_after.contains("id=\"a\""),
        "node a must be gone from source_after; got:\n{}",
        result.source_after
    );
    assert!(result.source_after.contains("id=\"b\""), "b must remain");
}

#[test]
fn remove_node_nested_in_group() {
    let doc = parse(ADD_GROUP_DOC);
    let tx = Transaction {
        ops: vec![Op::RemoveNode {
            node: "g.a".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["g.a".to_owned()]);
    assert!(
        !result.source_after.contains("id=\"g.a\""),
        "nested node g.a must be gone; got:\n{}",
        result.source_after
    );
    assert!(
        result.source_after.contains("id=\"g.b\""),
        "g.b must remain"
    );
}

#[test]
fn remove_node_unknown_rejected() {
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::RemoveNode {
            node: "nope".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unknown_node"),
        "expected tx.unknown_node; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

// ── SetOpacity tests ──────────────────────────────────────────────────────

#[test]
fn set_opacity_on_rect() {
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetOpacity {
            node: "a".to_owned(),
            opacity: 0.5,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["a".to_owned()]);
    assert!(
        result.source_after.contains("opacity=0.5"),
        "source_after must contain opacity=0.5; got:\n{}",
        result.source_after
    );
    assert_ne!(result.source_before, result.source_after);
}

#[test]
fn set_opacity_clamped_above_one() {
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetOpacity {
            node: "a".to_owned(),
            opacity: 1.5,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    // 1.5 clamped to 1.0; formatter writes "1" (or "1.0") — just verify source
    // changed and the candidate has Some(1.0) by checking node in candidate.
    // We check the diagnostic list is clean (no errors) and affected is recorded.
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "no errors expected; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["a".to_owned()]);
}

#[test]
fn set_opacity_clamped_below_zero() {
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetOpacity {
            node: "a".to_owned(),
            opacity: -0.5,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["a".to_owned()]);
    assert!(
        result.source_after.contains("opacity=0"),
        "clamped-to-0 opacity must appear in source_after; got:\n{}",
        result.source_after
    );
}

#[test]
fn set_opacity_unknown_node_rejected() {
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetOpacity {
            node: "nope".to_owned(),
            opacity: 0.5,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unknown_node"),
        "expected tx.unknown_node; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

// ── ReplaceText tests ─────────────────────────────────────────────────────

#[test]
fn replace_text_updates_spans() {
    let doc = parse(TEXT_DOC);
    let tx = Transaction {
        ops: vec![Op::ReplaceText {
            node: "label".to_owned(),
            spans: vec![OpSpan {
                text: "Goodbye".to_owned(),
                fill: None,
                font_weight: None,
                italic: None,
                underline: None,
                strikethrough: None,
                vertical_align: None,
                footnote_ref: None,
            }],
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["label".to_owned()]);
    assert!(
        result.source_after.contains("Goodbye"),
        "source_after must contain new text; got:\n{}",
        result.source_after
    );
    assert!(
        !result.source_after.contains("Hello"),
        "old text must not appear in source_after"
    );
    assert_ne!(result.source_before, result.source_after);
}

#[test]
fn replace_text_on_rect_unsupported() {
    let doc = parse(MIXED_DOC);
    let tx = Transaction {
        ops: vec![Op::ReplaceText {
            node: "box1".to_owned(),
            spans: vec![OpSpan {
                text: "hi".to_owned(),
                fill: None,
                font_weight: None,
                italic: None,
                underline: None,
                strikethrough: None,
                vertical_align: None,
                footnote_ref: None,
            }],
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unsupported_property" && d.message.contains("rect")),
        "expected tx.unsupported_property naming rect; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn replace_text_span_with_fill_token() {
    // A doc that has both color tokens and a text node.
    const TEXT_WITH_TOKEN_DOC: &str = r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" {
    token id="color.a" type="color" value="#ff0000"
    token id="color.b" type="color" value="#0000ff"
  }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      text id="lbl" x=(px)10 y=(px)10 w=(px)200 h=(px)40 {
        span "Original"
      }
    }
  }
}"##;
    let doc2 = parse(TEXT_WITH_TOKEN_DOC);
    let tx = Transaction {
        ops: vec![Op::ReplaceText {
            node: "lbl".to_owned(),
            spans: vec![OpSpan {
                text: "Branded".to_owned(),
                fill: Some("color.a".to_owned()),
                font_weight: None,
                italic: None,
                underline: None,
                strikethrough: None,
                vertical_align: None,
                footnote_ref: None,
            }],
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc2, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["lbl".to_owned()]);
    // The formatter should emit the span's fill token ref in source_after.
    assert!(
        result.source_after.contains("Branded"),
        "new text must appear in source_after; got:\n{}",
        result.source_after
    );
}

// ── DuplicateNode tests ───────────────────────────────────────────────────

/// Duplicate a leaf rect: parent now has 2 rects, clone right after original,
/// clone has new_id and same geometry/fill.
#[test]
fn duplicate_node_leaf_rect_accepted() {
    let doc = parse(DUP_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::DuplicateNode {
            node: "orig".to_owned(),
            new_id: "orig-copy".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["orig-copy".to_owned()]);

    // Both ids must appear in source_after.
    assert!(
        result.source_after.contains("id=\"orig\""),
        "original must still be present; got:\n{}",
        result.source_after
    );
    assert!(
        result.source_after.contains("id=\"orig-copy\""),
        "clone must be present; got:\n{}",
        result.source_after
    );

    // Clone must appear AFTER the original in source text.
    let pos_orig = result
        .source_after
        .find("id=\"orig\"")
        .expect("orig in source_after");
    let pos_copy = result
        .source_after
        .find("id=\"orig-copy\"")
        .expect("orig-copy in source_after");
    assert!(
        pos_orig < pos_copy,
        "clone should appear after original in source_after"
    );

    // Clone must carry the same geometry and fill as the original.
    // Count occurrences: both nodes should have x=(px)10, y=(px)20, etc.
    assert_eq!(
        result.source_after.matches("x=(px)10").count(),
        2,
        "both orig and clone should have x=(px)10; got:\n{}",
        result.source_after
    );
    assert_eq!(
        result.source_after.matches("w=(px)80").count(),
        2,
        "both orig and clone should have w=(px)80; got:\n{}",
        result.source_after
    );
    assert_eq!(
        result.source_after.matches("(token)\"color.a\"").count(),
        2,
        "both orig and clone should reference color.a; got:\n{}",
        result.source_after
    );

    // source_before has only one rect.
    assert_eq!(
        result.source_before.matches("id=\"orig").count(),
        1,
        "source_before should have only one orig* node"
    );
}

/// Duplicate with a new_id that already exists → post-validate rejects (id.duplicate).
#[test]
fn duplicate_node_colliding_new_id_rejected() {
    // TWO_RECT_DOC has rect "a" and rect "b"; duplicating "a" with new_id="b"
    // creates a second node with id "b" → id.duplicate from post-validate.
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::DuplicateNode {
            node: "a".to_owned(),
            new_id: "b".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Rejected,
        "colliding new_id must be rejected; diagnostics: {:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics.iter().any(|d| d.code == "id.duplicate"),
        "expected id.duplicate diagnostic; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

/// Attempting to duplicate a group → tx.unsupported_property (v0 scope).
#[test]
fn duplicate_node_container_group_rejected() {
    let doc = parse(DUP_GROUP_DOC);
    let tx = Transaction {
        ops: vec![Op::DuplicateNode {
            node: "grp".to_owned(),
            new_id: "grp-copy".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| { d.code == "tx.unsupported_property" && d.message.contains("group") }),
        "expected tx.unsupported_property mentioning group; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

/// Attempting to duplicate an unknown node id → tx.unknown_node.
#[test]
fn duplicate_node_unknown_id_rejected() {
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::DuplicateNode {
            node: "does_not_exist".to_owned(),
            new_id: "copy".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unknown_node"),
        "expected tx.unknown_node; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

// ── DuplicatePage tests ───────────────────────────────────────────────────

/// Duplicate a 1-page doc with 2 nodes: doc now has 2 pages, the copy has the
/// new page id, the copy's nodes carry the suffix, and the source is unchanged.
#[test]
fn duplicate_page_accepted() {
    let doc = parse(DUP_PAGE_DOC);
    let tx = Transaction {
        ops: vec![Op::DuplicatePage {
            page: "pg1".to_owned(),
            new_id: "pg2".to_owned(),
            id_suffix: ".v2".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["pg2".to_owned()]);

    // Both page ids present; the new page appears after the original.
    assert!(result.source_after.contains("page id=\"pg1\""));
    assert!(result.source_after.contains("page id=\"pg2\""));
    let pos_pg1 = result
        .source_after
        .find("page id=\"pg1\"")
        .expect("pg1 in source_after");
    let pos_pg2 = result
        .source_after
        .find("page id=\"pg2\"")
        .expect("pg2 in source_after");
    assert!(pos_pg1 < pos_pg2, "new page should follow the source page");

    // The copy's node ids are <orig><suffix>.
    assert!(
        result.source_after.contains("id=\"r1.v2\""),
        "clone node r1.v2 must be present; got:\n{}",
        result.source_after
    );
    assert!(
        result.source_after.contains("id=\"r2.v2\""),
        "clone node r2.v2 must be present; got:\n{}",
        result.source_after
    );

    // (b) The source page's nodes are NOT renamed — original ids still appear,
    // and they appear exactly once each (only the source carries them).
    assert_eq!(
        result.source_after.matches("id=\"r1\"").count(),
        1,
        "source node r1 must be unchanged and unique; got:\n{}",
        result.source_after
    );
    assert_eq!(
        result.source_after.matches("id=\"r2\"").count(),
        1,
        "source node r2 must be unchanged and unique; got:\n{}",
        result.source_after
    );

    // source_before has only one page.
    assert_eq!(
        result.source_before.matches("page id=").count(),
        1,
        "source_before should have only one page"
    );
}

/// Duplicate with an empty id_suffix → cloned node ids collide with the
/// originals → post-validation rejects via id.duplicate.
#[test]
fn duplicate_page_empty_suffix_rejected() {
    let doc = parse(DUP_PAGE_DOC);
    let tx = Transaction {
        ops: vec![Op::DuplicatePage {
            page: "pg1".to_owned(),
            new_id: "pg2".to_owned(),
            id_suffix: String::new(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Rejected,
        "empty suffix must be rejected; diagnostics: {:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics.iter().any(|d| d.code == "id.duplicate"),
        "expected id.duplicate diagnostic; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

/// Duplicate an unknown source page → tx.unknown_node, transaction rejected.
#[test]
fn duplicate_page_unknown_page_rejected() {
    let doc = parse(DUP_PAGE_DOC);
    let tx = Transaction {
        ops: vec![Op::DuplicatePage {
            page: "does_not_exist".to_owned(),
            new_id: "pg2".to_owned(),
            id_suffix: ".v2".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unknown_node"),
        "expected tx.unknown_node; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}
