//! Command-line interface library for Zenith.
//!
//! Owns all command dispatch, argument parsing (via clap), JSON I/O shaping,
//! and human-readable stdout/stderr formatting.
//!
//! `src/main.rs` is kept thin — it only calls [`run`].
//! `zenith-layout` is reached transitively through `zenith-scene`; the CLI
//! never constructs layout types directly.
//!
//! # Module layout
//!
//! - `cli` — clap `#[derive(Parser)]` types.
//! - `commands/` — one module per subcommand; all business logic is here,
//!   operating on in-memory bytes, never touching the FS.
//! - `json_types` — serialisable DTOs for JSON output.
//! - `lib.rs` — this file: wiring + `run()` dispatcher + file I/O edge.

pub mod cli;
pub mod commands;
pub mod json_types;

use std::io::Write as _;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};
use crate::commands::serialize_pretty;
use crate::json_types::RenderOutput;

/// Main entry point: parse CLI arguments, dispatch to the appropriate command,
/// handle all file I/O, and return the appropriate exit code.
///
/// All business logic lives in `commands/`; this function is I/O only.
pub fn run() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Validate(args) => {
            let src = match read_file(&args.path) {
                Ok(s) => s,
                Err(msg) => {
                    eprintln!("{}", msg);
                    return ExitCode::from(2);
                }
            };
            let out = commands::validate::run(&src, args.json);
            println!("{}", out.stdout);
            ExitCode::from(out.exit_code)
        }

        Command::Fmt(args) => {
            let src = match read_file(&args.path) {
                Ok(s) => s,
                Err(msg) => {
                    eprintln!("{}", msg);
                    return ExitCode::from(2);
                }
            };
            match commands::fmt::run(&src) {
                Ok(result) => {
                    // Write formatted content back to disk.
                    if let Err(e) = std::fs::write(&args.path, &result.formatted) {
                        eprintln!("error writing '{}': {}", args.path.display(), e);
                        return ExitCode::from(2);
                    }
                    println!("{}", commands::fmt::render_stdout(&result, args.json));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", e.message);
                    ExitCode::from(e.exit_code)
                }
            }
        }

        Command::Tokens(args) => {
            let src = match read_file(&args.path) {
                Ok(s) => s,
                Err(msg) => {
                    eprintln!("{}", msg);
                    return ExitCode::from(2);
                }
            };
            match commands::tokens::list(&src, args.json) {
                Ok(out) => {
                    println!("{}", out);
                    ExitCode::SUCCESS
                }
                Err((msg, code)) => {
                    eprintln!("{}", msg);
                    ExitCode::from(code)
                }
            }
        }

        Command::Render(args) => {
            // Require at least one of --scene / --png.
            if args.scene.is_none() && args.png.is_none() {
                eprintln!("error: at least one of --scene <OUT> or --png <OUT> is required");
                return ExitCode::from(2);
            }

            let src = match read_file(&args.path) {
                Ok(s) => s,
                Err(msg) => {
                    eprintln!("{}", msg);
                    return ExitCode::from(2);
                }
            };

            // --scene ─────────────────────────────────────────────────────────
            if let Some(scene_out) = &args.scene {
                match commands::render::to_scene_json(&src) {
                    Ok(json) => {
                        if let Err(e) = std::fs::write(scene_out, json.as_bytes()) {
                            eprintln!("error writing scene to '{}': {}", scene_out.display(), e);
                            return ExitCode::from(2);
                        }
                        if args.json {
                            let out = RenderOutput {
                                schema: "zenith-render-v1",
                                diagnostics: vec![],
                            };
                            println!("{}", serialize_pretty(&out));
                        } else {
                            println!("scene written to '{}'", scene_out.display());
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e.message);
                        return ExitCode::from(e.exit_code);
                    }
                }
            }

            // --png ───────────────────────────────────────────────────────────
            if let Some(png_out) = &args.png {
                match commands::render::to_png(&src) {
                    Ok(bytes) => {
                        if let Err(e) = write_bytes(png_out, &bytes) {
                            eprintln!("error writing PNG to '{}': {}", png_out.display(), e);
                            return ExitCode::from(2);
                        }
                        if args.json {
                            let out = RenderOutput {
                                schema: "zenith-render-v1",
                                diagnostics: vec![],
                            };
                            println!("{}", serialize_pretty(&out));
                        } else {
                            println!("PNG written to '{}'", png_out.display());
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e.message);
                        return ExitCode::from(e.exit_code);
                    }
                }
            }

            ExitCode::SUCCESS
        }

        Command::Tx(args) => {
            // Read document source.
            let doc_src = match read_file(&args.path) {
                Ok(s) => s,
                Err(msg) => {
                    eprintln!("{}", msg);
                    return ExitCode::from(2);
                }
            };

            // Read transaction JSON.
            let tx_json = match read_file(&args.tx_file) {
                Ok(s) => s,
                Err(msg) => {
                    eprintln!("{}", msg);
                    return ExitCode::from(2);
                }
            };

            // Run the pure transaction logic.
            let outcome = match commands::tx::run(&doc_src, &tx_json) {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("{}", e.message);
                    return ExitCode::from(e.exit_code);
                }
            };

            // Print output.
            if args.json {
                println!("{}", outcome.json_str);
            } else {
                println!("{}", outcome.human);
            }

            // Apply: persist source_after if requested and not rejected.
            if args.apply
                && outcome.exit_code != 1
                && let Err(e) = std::fs::write(&args.path, outcome.result.source_after.as_bytes())
            {
                eprintln!("error writing '{}': {}", args.path.display(), e);
                return ExitCode::from(2);
            }

            ExitCode::from(outcome.exit_code)
        }
    }
}

// ── I/O helpers ───────────────────────────────────────────────────────────────

/// Read a file to a UTF-8 string.
///
/// Returns a human-readable error message on failure (never panics).
fn read_file(path: &std::path::Path) -> Result<String, String> {
    std::fs::read(path)
        .map_err(|e| format!("error reading '{}': {}", path.display(), e))
        .and_then(|bytes| {
            String::from_utf8(bytes)
                .map_err(|_| format!("error: '{}' is not valid UTF-8", path.display()))
        })
}

/// Write raw bytes to a file.
///
/// Returns a `std::io::Error` on failure.
fn write_bytes(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)
}
