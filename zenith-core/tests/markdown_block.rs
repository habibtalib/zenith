//! Integration tests for the block-level markdown parser.
//!
//! Exercises `parse_block_markdown` via the public API only.
//! Raw strings containing `#` use `r##"..."##` to avoid compile errors.

use zenith_core::{ListKind, MdBlock, parse_block_markdown};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn texts(blocks: &[MdBlock]) -> Vec<String> {
    blocks
        .iter()
        .map(|b| match b {
            MdBlock::Heading { spans, .. } => spans.iter().map(|s| s.text.as_str()).collect(),
            MdBlock::Paragraph { spans } => spans.iter().map(|s| s.text.as_str()).collect(),
            MdBlock::Blockquote { spans } => spans.iter().map(|s| s.text.as_str()).collect(),
            MdBlock::ListItem { spans, .. } => spans.iter().map(|s| s.text.as_str()).collect(),
            MdBlock::CodeBlock { content, .. } => content.clone(),
            MdBlock::HorizontalRule => String::new(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Basic structure
// ---------------------------------------------------------------------------

#[test]
fn empty_input_yields_no_blocks() {
    assert!(parse_block_markdown("").is_empty());
}

#[test]
fn blank_lines_only_yield_no_blocks() {
    assert!(parse_block_markdown("\n\n\n").is_empty());
}

#[test]
fn single_paragraph() {
    let blocks = parse_block_markdown("Hello world");
    assert_eq!(blocks.len(), 1);
    assert!(matches!(&blocks[0], MdBlock::Paragraph { .. }));
    assert_eq!(texts(&blocks)[0], "Hello world");
}

#[test]
fn two_paragraphs_split_by_blank_line() {
    let blocks = parse_block_markdown("First\n\nSecond");
    assert_eq!(blocks.len(), 2);
    assert!(matches!(&blocks[0], MdBlock::Paragraph { .. }));
    assert!(matches!(&blocks[1], MdBlock::Paragraph { .. }));
    assert_eq!(texts(&blocks)[0], "First");
    assert_eq!(texts(&blocks)[1], "Second");
}

#[test]
fn leading_and_trailing_blank_lines_ignored() {
    let blocks = parse_block_markdown("\n\nHello\n\n");
    assert_eq!(blocks.len(), 1);
    assert_eq!(texts(&blocks)[0], "Hello");
}

#[test]
fn paragraph_lines_joined_with_space() {
    let blocks = parse_block_markdown("line one\nline two\nline three");
    assert_eq!(blocks.len(), 1);
    assert_eq!(texts(&blocks)[0], "line one line two line three");
}

// ---------------------------------------------------------------------------
// Headings
// ---------------------------------------------------------------------------

#[test]
fn h1_through_h6() {
    let src = r##"# H1
## H2
### H3
#### H4
##### H5
###### H6"##;
    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 6);
    for (i, block) in blocks.iter().enumerate() {
        match block {
            MdBlock::Heading { level, .. } => assert_eq!(*level, (i + 1) as u8),
            other => panic!("expected Heading, got {other:?}"),
        }
    }
    let t = texts(&blocks);
    assert_eq!(t[0], "H1");
    assert_eq!(t[5], "H6");
}

#[test]
fn heading_immediately_after_paragraph_no_blank_line() {
    let src = r##"Paragraph text
## Heading"##;
    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 2);
    assert!(matches!(&blocks[0], MdBlock::Paragraph { .. }));
    assert!(matches!(&blocks[1], MdBlock::Heading { level: 2, .. }));
}

#[test]
fn heading_with_inline_marks() {
    let src = r##"## **bold** x"##;
    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 1);
    let MdBlock::Heading { level, spans } = &blocks[0] else {
        panic!("expected Heading")
    };
    assert_eq!(*level, 2);
    let joined: String = spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(joined, "bold x");
    assert!(spans[0].font_weight.is_some(), "first span should be bold");
}

#[test]
fn heading_trailing_hashes_stripped() {
    let src = r##"## title ##"##;
    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 1);
    assert_eq!(texts(&blocks)[0], "title");
}

#[test]
fn hash_with_no_space_is_paragraph() {
    // `#text` (no space after #) → paragraph, not heading.
    let src = r##"#text"##;
    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 1);
    assert!(matches!(&blocks[0], MdBlock::Paragraph { .. }));
}

#[test]
fn seven_hashes_is_paragraph() {
    let src = r##"####### not a heading"##;
    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 1);
    assert!(matches!(&blocks[0], MdBlock::Paragraph { .. }));
}

// ---------------------------------------------------------------------------
// Blockquotes
// ---------------------------------------------------------------------------

#[test]
fn single_line_blockquote() {
    let blocks = parse_block_markdown("> hello");
    assert_eq!(blocks.len(), 1);
    assert!(matches!(&blocks[0], MdBlock::Blockquote { .. }));
    assert_eq!(texts(&blocks)[0], "hello");
}

#[test]
fn multi_line_blockquote_merged() {
    let blocks = parse_block_markdown("> line one\n> line two");
    assert_eq!(blocks.len(), 1);
    assert!(matches!(&blocks[0], MdBlock::Blockquote { .. }));
    assert_eq!(texts(&blocks)[0], "line one line two");
}

#[test]
fn blockquote_with_inline_marks() {
    let src = "> **bold** text";
    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 1);
    let MdBlock::Blockquote { spans } = &blocks[0] else {
        panic!("expected Blockquote")
    };
    let joined: String = spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(joined, "bold text");
    assert!(spans[0].font_weight.is_some());
}

// ---------------------------------------------------------------------------
// Fenced code blocks
// ---------------------------------------------------------------------------

#[test]
fn fenced_code_no_lang() {
    let src = "```\nfn main() {}\n```";
    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 1);
    let MdBlock::CodeBlock { lang, content } = &blocks[0] else {
        panic!("expected CodeBlock")
    };
    assert!(lang.is_none());
    assert_eq!(content, "fn main() {}");
}

#[test]
fn fenced_code_with_lang() {
    let src = "```rust\nlet x = 1;\n```";
    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 1);
    let MdBlock::CodeBlock { lang, content } = &blocks[0] else {
        panic!("expected CodeBlock")
    };
    assert_eq!(lang.as_deref(), Some("rust"));
    assert_eq!(content, "let x = 1;");
}

#[test]
fn fenced_code_content_is_raw_no_inline_parsing() {
    // `**not bold**` inside a code fence must NOT be parsed as bold.
    let src = "```\n**not bold**\n```";
    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 1);
    let MdBlock::CodeBlock { content, .. } = &blocks[0] else {
        panic!("expected CodeBlock")
    };
    assert_eq!(content, "**not bold**");
}

#[test]
fn unclosed_fence_at_eof_flushes_as_code_block() {
    let src = "```rust\nfn main() {}";
    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 1);
    let MdBlock::CodeBlock { lang, content } = &blocks[0] else {
        panic!("expected CodeBlock")
    };
    assert_eq!(lang.as_deref(), Some("rust"));
    assert_eq!(content, "fn main() {}");
}

// ---------------------------------------------------------------------------
// Horizontal rules
// ---------------------------------------------------------------------------

#[test]
fn hr_triple_dash() {
    let blocks = parse_block_markdown("---");
    assert_eq!(blocks.len(), 1);
    assert!(matches!(&blocks[0], MdBlock::HorizontalRule));
}

#[test]
fn hr_triple_star() {
    let blocks = parse_block_markdown("***");
    assert_eq!(blocks.len(), 1);
    assert!(matches!(&blocks[0], MdBlock::HorizontalRule));
}

#[test]
fn hr_triple_underscore() {
    let blocks = parse_block_markdown("___");
    assert_eq!(blocks.len(), 1);
    assert!(matches!(&blocks[0], MdBlock::HorizontalRule));
}

#[test]
fn hr_spaced_dashes() {
    let blocks = parse_block_markdown("- - -");
    assert_eq!(blocks.len(), 1);
    assert!(matches!(&blocks[0], MdBlock::HorizontalRule));
}

// ---------------------------------------------------------------------------
// List items
// ---------------------------------------------------------------------------

#[test]
fn unordered_list_dash() {
    let blocks = parse_block_markdown("- item");
    assert_eq!(blocks.len(), 1);
    let MdBlock::ListItem {
        kind,
        depth,
        ordinal,
        ..
    } = &blocks[0]
    else {
        panic!("expected ListItem")
    };
    assert_eq!(*kind, ListKind::Unordered);
    assert_eq!(*depth, 0);
    assert!(ordinal.is_none());
    assert_eq!(texts(&blocks)[0], "item");
}

#[test]
fn unordered_list_star() {
    let blocks = parse_block_markdown("* item");
    assert_eq!(blocks.len(), 1);
    assert!(matches!(
        &blocks[0],
        MdBlock::ListItem {
            kind: ListKind::Unordered,
            ..
        }
    ));
}

#[test]
fn unordered_list_plus() {
    let blocks = parse_block_markdown("+ item");
    assert_eq!(blocks.len(), 1);
    assert!(matches!(
        &blocks[0],
        MdBlock::ListItem {
            kind: ListKind::Unordered,
            ..
        }
    ));
}

#[test]
fn ordered_list_first_item() {
    let blocks = parse_block_markdown("1. first");
    assert_eq!(blocks.len(), 1);
    let MdBlock::ListItem { kind, ordinal, .. } = &blocks[0] else {
        panic!("expected ListItem")
    };
    assert_eq!(*kind, ListKind::Ordered);
    assert_eq!(*ordinal, Some(1));
    assert_eq!(texts(&blocks)[0], "first");
}

#[test]
fn ordered_list_second_item_ordinal_captured() {
    let blocks = parse_block_markdown("2. second");
    assert_eq!(blocks.len(), 1);
    let MdBlock::ListItem { ordinal, .. } = &blocks[0] else {
        panic!()
    };
    assert_eq!(*ordinal, Some(2));
}

#[test]
fn list_item_with_inline_marks() {
    let blocks = parse_block_markdown("- **bold** item");
    assert_eq!(blocks.len(), 1);
    let MdBlock::ListItem { spans, .. } = &blocks[0] else {
        panic!("expected ListItem")
    };
    let joined: String = spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(joined, "bold item");
    assert!(spans[0].font_weight.is_some());
}

#[test]
fn nested_list_two_space_indent_depth_one() {
    let blocks = parse_block_markdown("  - nested");
    assert_eq!(blocks.len(), 1);
    let MdBlock::ListItem { depth, .. } = &blocks[0] else {
        panic!("expected ListItem")
    };
    assert_eq!(*depth, 1);
}

// ---------------------------------------------------------------------------
// Mixed document
// ---------------------------------------------------------------------------

#[test]
fn mixed_document_all_roles_in_order() {
    let src = r##"# Title

Paragraph here.

> A quote.

- List item

1. Ordered

---

```
code
```"##;

    let blocks = parse_block_markdown(src);
    assert_eq!(blocks.len(), 7);
    assert!(matches!(&blocks[0], MdBlock::Heading { level: 1, .. }));
    assert!(matches!(&blocks[1], MdBlock::Paragraph { .. }));
    assert!(matches!(&blocks[2], MdBlock::Blockquote { .. }));
    assert!(matches!(
        &blocks[3],
        MdBlock::ListItem {
            kind: ListKind::Unordered,
            ..
        }
    ));
    assert!(matches!(
        &blocks[4],
        MdBlock::ListItem {
            kind: ListKind::Ordered,
            ..
        }
    ));
    assert!(matches!(&blocks[5], MdBlock::HorizontalRule));
    assert!(matches!(&blocks[6], MdBlock::CodeBlock { .. }));
}
