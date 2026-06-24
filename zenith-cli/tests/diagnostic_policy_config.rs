//! Integration tests for CLI-layer diagnostic-policy resolution.
//!
//! These exercise the public `zenith_cli::config` loaders/merge plus
//! `zenith_core::validate_with_policy`, and the `validate::run` exit-code path
//! for a malformed local config. All filesystem state is rooted in tempdirs and
//! the injectable loaders are used with explicit paths — no `$HOME`/cwd mutation,
//! so the tests are parallel-safe.

use std::fs;

use tempfile::TempDir;
use zenith_cli::config::{
    CliPolicyFlags, find_local_policy, load_global_policy_in, load_policy_file, merge_policy,
};
use zenith_core::{KdlAdapter, KdlSource as _, Severity, validate_with_policy};

// ── Fixture ─────────────────────────────────────────────────────────────────

/// A document with one unknown property → `node.unknown_property` (Warning by
/// default) and one unused token → `token.unused` (Advisory by default). Both
/// are non-Error, so policy can move them around.
const DOC: &str = r##"zenith version=1 {
  project id="proj.p" name="Policy"
  tokens format="zenith-token-v1" {
    token id="color.unused" type="color" value="#abcdef"
  }
  styles {}
  document id="doc.p" title="Policy" {
    page id="page.p" w=(px)100 h=(px)100 {
      rect id="r.one" x=(px)0 y=(px)0 w=(px)10 h=(px)10 future-prop="x"
    }
  }
}
"##;

/// A document whose in-file policy `allow`s `token.unused`, suppressing it.
const DOC_IN_FILE_ALLOW: &str = r##"zenith version=1 {
  project id="proj.p" name="Policy"
  diagnostics {
    allow "token.unused"
  }
  tokens format="zenith-token-v1" {
    token id="color.unused" type="color" value="#abcdef"
  }
  styles {}
  document id="doc.p" title="Policy" {
    page id="page.p" w=(px)100 h=(px)100 {
      rect id="r.one" x=(px)0 y=(px)0 w=(px)10 h=(px)10
    }
  }
}
"##;

fn parse(src: &str) -> zenith_core::Document {
    KdlAdapter.parse(src.as_bytes()).expect("doc must parse")
}

fn has_error(diags: &[zenith_core::Diagnostic], code: &str) -> bool {
    diags
        .iter()
        .any(|d| d.code == code && d.severity == Severity::Error)
}

fn present(diags: &[zenith_core::Diagnostic], code: &str) -> bool {
    diags.iter().any(|d| d.code == code)
}

// ── Local config walk-up ──────────────────────────────────────────────────────

#[test]
fn local_config_discovered_by_walk_up() {
    let tmp = TempDir::new().expect("tempdir");
    // Config at the root of the temp tree.
    fs::write(
        tmp.path().join(".zenith.kdl"),
        b"diagnostics {\n  deny \"token.unused\"\n}\n",
    )
    .expect("write config");
    // The document lives in a nested subdirectory.
    let nested = tmp.path().join("a").join("b");
    fs::create_dir_all(&nested).expect("mkdir");

    let local = find_local_policy(&nested).expect("walk-up must succeed");
    let doc = parse(DOC);
    let merged = merge_policy(
        &zenith_core::DiagnosticPolicy::default(),
        &local,
        &doc.diagnostic_policy,
        &CliPolicyFlags::default(),
    );
    let report = validate_with_policy(&doc, &merged);
    assert!(
        has_error(&report.diagnostics, "token.unused"),
        "local deny must elevate token.unused to Error"
    );
}

// ── Precedence: CLI > in-file > local > global ─────────────────────────────────

#[test]
fn cli_deny_overrides_global_allow() {
    let tmp = TempDir::new().expect("tempdir");
    // Global config allows token.unused (would suppress it).
    let zenith_dir = tmp.path().join("zenith");
    fs::create_dir_all(&zenith_dir).expect("mkdir");
    fs::write(
        zenith_dir.join("config.kdl"),
        b"diagnostics {\n  allow \"token.unused\"\n}\n",
    )
    .expect("write global");

    let global = load_global_policy_in(tmp.path()).expect("global load");
    let doc = parse(DOC);
    let flags = CliPolicyFlags {
        deny: vec!["token.unused".to_owned()],
        ..Default::default()
    };
    let merged = merge_policy(
        &global,
        &zenith_core::DiagnosticPolicy::default(),
        &doc.diagnostic_policy,
        &flags,
    );
    let report = validate_with_policy(&doc, &merged);
    assert!(
        has_error(&report.diagnostics, "token.unused"),
        "CLI deny must win over global allow"
    );
}

#[test]
fn cli_deny_overrides_in_file_allow() {
    // The document's own policy allows token.unused; --deny must override it,
    // making the suppressed-in-file code reappear as an Error.
    let doc = parse(DOC_IN_FILE_ALLOW);
    // Sanity: in-file allow alone suppresses it.
    let baseline = validate_with_policy(&doc, &doc.diagnostic_policy);
    assert!(
        !present(&baseline.diagnostics, "token.unused"),
        "in-file allow should suppress token.unused"
    );

    let flags = CliPolicyFlags {
        deny: vec!["token.unused".to_owned()],
        ..Default::default()
    };
    let merged = merge_policy(
        &zenith_core::DiagnosticPolicy::default(),
        &zenith_core::DiagnosticPolicy::default(),
        &doc.diagnostic_policy,
        &flags,
    );
    let report = validate_with_policy(&doc, &merged);
    assert!(
        has_error(&report.diagnostics, "token.unused"),
        "CLI deny must override in-file allow and surface token.unused as Error"
    );
}

#[test]
fn in_file_beats_local_and_global() {
    // global=deny, local=deny, in-file=allow → suppressed (in-file wins).
    let global = load_policy_inline("diagnostics {\n  deny \"token.unused\"\n}");
    let local = load_policy_inline("diagnostics {\n  deny \"token.unused\"\n}");
    let doc = parse(DOC_IN_FILE_ALLOW);
    let merged = merge_policy(
        &global,
        &local,
        &doc.diagnostic_policy,
        &CliPolicyFlags::default(),
    );
    let report = validate_with_policy(&doc, &merged);
    assert!(
        !present(&report.diagnostics, "token.unused"),
        "in-file allow must override local+global deny"
    );
}

// ── Malformed config → exit 2 ──────────────────────────────────────────────────

#[test]
fn malformed_local_config_exits_two() {
    let tmp = TempDir::new().expect("tempdir");
    fs::write(tmp.path().join(".zenith.kdl"), b"diagnostics {{{ not kdl")
        .expect("write bad config");

    let out = zenith_cli::commands::validate::run(
        DOC,
        Some(tmp.path()),
        false,
        &CliPolicyFlags::default(),
    );
    assert_eq!(
        out.exit_code, 2,
        "malformed local config must exit 2; stdout: {}",
        out.stdout
    );
    assert!(
        out.stdout.contains("config.error"),
        "error output must name config.error; got: {}",
        out.stdout
    );
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Write `body` to a temp `.zenith.kdl` and load it (exercises `load_policy_file`).
fn load_policy_inline(body: &str) -> zenith_core::DiagnosticPolicy {
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join("inline.kdl");
    fs::write(&path, body.as_bytes()).expect("write");
    load_policy_file(&path).expect("policy must load")
}
