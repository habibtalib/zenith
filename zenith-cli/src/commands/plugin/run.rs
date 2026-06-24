//! Orchestration for `zenith plugin {install,uninstall,list}`: resolve targets,
//! perform the work, print a concise human report, and return an exit code.

use std::path::Path;

use super::agent::{ALL_AGENTS, Agent, Scope};
use super::detect::{detect_present, is_installed};
use super::install::{WriteOutcome, install_agent};
use super::path_check::zenith_on_path;
use super::uninstall::{RemoveOutcome, uninstall_agent};

/// Which agents a command should act on.
#[derive(Debug, Clone)]
pub enum Targets {
    /// An explicit set chosen via `--claude`, `--codex`, … flags.
    Agents(Vec<Agent>),
    /// Every supported agent (`--all`).
    All,
    /// Auto-detect agents present on the machine (no flags given).
    Auto,
}

/// `zenith plugin install`.
pub fn run_install(
    project_root: &Path,
    targets: Targets,
    scope: Scope,
    force: bool,
    dry_run: bool,
) -> u8 {
    let agents = match resolve(targets, project_root) {
        Ok(a) => a,
        Err(code) => return code,
    };

    let mut any_overwrite = false;
    let mut any_error = false;
    let verb = if dry_run {
        "would install"
    } else {
        "installed"
    };

    for agent in agents {
        let report = install_agent(agent, scope, project_root, force, dry_run);
        if let Some(reason) = &report.unsupported {
            println!("- {}: skipped ({reason})", agent.display());
            continue;
        }
        let root = report
            .root
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let mut installed = 0usize;
        let mut current = 0usize;
        for f in &report.files {
            match &f.outcome {
                Ok(WriteOutcome::Installed) => installed += 1,
                Ok(WriteOutcome::AlreadyCurrent) => current += 1,
                Ok(WriteOutcome::WouldOverwrite) => {
                    any_overwrite = true;
                    println!("    differs (needs --force): {}", f.path.display());
                }
                Err(e) => {
                    any_error = true;
                    println!("    error: {}: {e}", f.path.display());
                }
            }
        }
        println!(
            "- {} ({}): {verb} {installed}, current {current} → {root}",
            agent.display(),
            scope_label(scope),
        );
    }

    if zenith_on_path().is_none() {
        println!("warning: the installed skill calls `zenith`, but `zenith` is not on your PATH.");
        println!(
            "  Install it: `cargo install --path zenith-cli`, or `./scripts/install.sh`, then ensure its dir is on PATH."
        );
    }

    finish(any_error, any_overwrite, dry_run)
}

/// `zenith plugin uninstall`.
pub fn run_uninstall(project_root: &Path, targets: Targets, scope: Scope, dry_run: bool) -> u8 {
    let agents = match resolve(targets, project_root) {
        Ok(a) => a,
        Err(code) => return code,
    };

    let mut any_error = false;
    let verb = if dry_run { "would remove" } else { "removed" };

    for agent in agents {
        let report = uninstall_agent(agent, scope, project_root, dry_run);
        if let Some(reason) = &report.unsupported {
            println!("- {}: skipped ({reason})", agent.display());
            continue;
        }
        let mut removed = 0usize;
        let mut absent = 0usize;
        for item in &report.items {
            match &item.outcome {
                Ok(RemoveOutcome::Removed) => removed += 1,
                Ok(RemoveOutcome::Absent) => absent += 1,
                Err(e) => {
                    any_error = true;
                    println!("    error: {}: {e}", item.path.display());
                }
            }
        }
        println!(
            "- {} ({}): {verb} {removed}, absent {absent}",
            agent.display(),
            scope_label(scope),
        );
    }

    if any_error { 2 } else { 0 }
}

/// `zenith plugin list` — show install state per agent in both scopes.
pub fn run_list(project_root: &Path) -> u8 {
    println!("Zenith skill install state (agent / project / user):");
    for agent in ALL_AGENTS {
        let proj = mark(is_installed(*agent, Scope::Project, project_root));
        let user = mark(is_installed(*agent, Scope::User, project_root));
        let present = if detect_present(project_root).contains(agent) {
            " (detected)"
        } else {
            ""
        };
        println!(
            "- {:<14} project:{proj}  user:{user}{present}",
            agent.display()
        );
    }
    match zenith_on_path() {
        Some(path) => println!("zenith binary: {}", path.display()),
        None => println!(
            "zenith binary: NOT on PATH — the skill calls `zenith` by name; \
             install it (`cargo install --path zenith-cli` or `./scripts/install.sh`) and put its dir on PATH"
        ),
    }
    0
}

// ── Internal ────────────────────────────────────────────────────────────────

/// Resolve `Targets` to a concrete agent list, printing guidance on empty auto.
fn resolve(targets: Targets, project_root: &Path) -> Result<Vec<Agent>, u8> {
    match targets {
        Targets::Agents(a) if !a.is_empty() => Ok(a),
        Targets::Agents(_) => {
            eprintln!(
                "error: no agent selected — pass --all, an agent flag, or none to auto-detect"
            );
            Err(2)
        }
        Targets::All => Ok(ALL_AGENTS.to_vec()),
        Targets::Auto => {
            let found = detect_present(project_root);
            if found.is_empty() {
                eprintln!(
                    "no agents detected in {} or your home directory.",
                    project_root.display()
                );
                eprintln!("pass an explicit flag (e.g. --claude) or --all to choose targets.");
                return Err(1);
            }
            let names: Vec<&str> = found.iter().map(|a| a.display()).collect();
            eprintln!("detected: {}", names.join(", "));
            Ok(found)
        }
    }
}

fn finish(any_error: bool, any_overwrite: bool, dry_run: bool) -> u8 {
    if any_error {
        return 2;
    }
    if any_overwrite {
        if dry_run {
            eprintln!("some files differ; re-run without --dry-run and with --force to overwrite.");
        } else {
            eprintln!("some files were left unchanged; re-run with --force to overwrite them.");
        }
        return 2;
    }
    0
}

fn scope_label(scope: Scope) -> &'static str {
    match scope {
        Scope::Project => "project",
        Scope::User => "user",
    }
}

fn mark(installed: bool) -> &'static str {
    if installed { "yes" } else { "—" }
}
