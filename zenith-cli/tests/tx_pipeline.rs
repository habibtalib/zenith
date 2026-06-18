//! End-to-end integration test: transaction → compile → rasterize.
//!
//! Reads `examples/hello.zen` from the workspace root, applies a
//! `set_text_align` transaction that sets `text.hello` to `center`, asserts
//! the transaction is accepted and the source changes, then compiles and
//! renders the post-transaction source to PNG and checks determinism.

use zenith_cli::commands::tx::run as tx_run;
use zenith_core::{KdlAdapter, KdlSource, default_provider};
use zenith_render::render_png;
use zenith_scene::compile;
use zenith_tx::TxStatus;

/// Transaction JSON targeting the `text.hello` node in `hello.zen`.
const TX_JSON: &str = r#"{"ops":[{"op":"set_text_align","node":"text.hello","align":"center"}]}"#;

fn hello_src() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir)
        .join("..")
        .join("examples")
        .join("hello.zen");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e))
}

// ── Transaction correctness ────────────────────────────────────────────────────

#[test]
fn tx_hello_accepted_and_changed() {
    let src = hello_src();
    let outcome = tx_run(&src, TX_JSON)
        .unwrap_or_else(|e| panic!("tx_run returned Err (exit {}): {}", e.exit_code, e.message));

    // Status must be Accepted.
    assert_eq!(
        outcome.result.status,
        TxStatus::Accepted,
        "transaction must be Accepted"
    );

    // Exit code must be 0.
    assert_eq!(outcome.exit_code, 0, "Accepted must yield exit code 0");

    // Source must have changed.
    let changed = outcome.result.source_before != outcome.result.source_after;
    assert!(changed, "source_after must differ from source_before");

    // `text.hello` must be in the affected ids.
    assert!(
        outcome
            .result
            .affected_node_ids
            .contains(&"text.hello".to_owned()),
        "text.hello must appear in affected_node_ids; got: {:?}",
        outcome.result.affected_node_ids
    );

    // `source_after` must contain the new alignment value.
    assert!(
        outcome.result.source_after.contains("center"),
        "source_after must contain align=\"center\"; snippet: {}",
        &outcome.result.source_after[..outcome.result.source_after.len().min(500)]
    );
}

// ── Post-transaction render ────────────────────────────────────────────────────

#[test]
fn tx_then_render_produces_valid_png() {
    let src = hello_src();
    let outcome = tx_run(&src, TX_JSON)
        .unwrap_or_else(|e| panic!("tx_run failed (exit {}): {}", e.exit_code, e.message));

    let post_src = &outcome.result.source_after;

    // Parse the post-transaction source.
    let provider = default_provider();
    let doc = KdlAdapter
        .parse(post_src.as_bytes())
        .unwrap_or_else(|e| panic!("post-tx source failed to parse: {}", e.message));

    // Compile.
    let compiled = compile(&doc, &provider);

    // Render first time.
    let png1 = render_png(&compiled.scene, &provider)
        .unwrap_or_else(|e| panic!("first render failed: {}", e));

    // Non-empty.
    assert!(!png1.is_empty(), "PNG must not be empty");

    // PNG magic bytes (first 8 bytes).
    assert!(
        png1.len() >= 8,
        "PNG must have at least 8 bytes; got {}",
        png1.len()
    );
    assert_eq!(
        &png1[0..8],
        &[0x89u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        "first 8 bytes must be the PNG signature"
    );

    // Render second time — must be byte-identical (determinism gate).
    let png2 = render_png(&compiled.scene, &provider)
        .unwrap_or_else(|e| panic!("second render failed: {}", e));

    assert_eq!(
        png1, png2,
        "two renders of post-transaction source must be byte-identical"
    );
}

// ── Full pipeline: parse → tx → compile → render ─────────────────────────────

#[test]
fn full_pipeline_parse_tx_compile_render() {
    let src = hello_src();

    // Step 1: transaction.
    let outcome = tx_run(&src, TX_JSON)
        .unwrap_or_else(|e| panic!("tx_run failed (exit {}): {}", e.exit_code, e.message));

    assert_eq!(
        outcome.result.status,
        TxStatus::Accepted,
        "pipeline: transaction must be Accepted"
    );

    // Step 2: parse the resulting source.
    let provider = default_provider();
    let doc = KdlAdapter
        .parse(outcome.result.source_after.as_bytes())
        .unwrap_or_else(|e| panic!("pipeline: post-tx parse failed: {}", e.message));

    // Step 3: compile.
    let compiled = compile(&doc, &provider);
    assert!(
        !compiled.scene.commands.is_empty(),
        "pipeline: compiled scene must have at least one draw command"
    );

    // Step 4: rasterize.
    let png = render_png(&compiled.scene, &provider)
        .unwrap_or_else(|e| panic!("pipeline: render failed: {}", e));

    assert!(!png.is_empty(), "pipeline: PNG must not be empty");
    assert_eq!(
        &png[0..4],
        &[0x89u8, 0x50, 0x4E, 0x47],
        "pipeline: output must be a valid PNG"
    );
}
