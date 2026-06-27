//! `okq init` and `okq new` — scaffold an OKF bundle and author concepts into it.
//! See `docs/features/scaffold.md`. These are the only commands that *write* into
//! a bundle (the search index lives in the XDG cache, not here — ADR-0003).

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::sections::slugify;
use crate::templates;

/// Concept types `okq new` knows how to create.
pub const TYPES: [&str; 2] = ["adr", "feature"];

/// One scaffolding action, for the `init` report.
pub struct Action {
    /// Path relative to the bundle.
    pub path: String,
    /// What happened: `created`, `exists`, or `updated`.
    pub verb: &'static str,
}

/// `okq new`: create one concept from a template; returns the path written.
pub fn new(bundle_dir: &Path, type_: &str, title: &str) -> Result<PathBuf, AppError> {
    let date = templates::today_iso();
    let (folder, filename, content) = match type_ {
        "adr" => {
            let number = next_adr_number(&bundle_dir.join("adrs"));
            (
                "adrs",
                format!("{number:04}-{}.md", slugify(title)),
                templates::adr(title, &date),
            )
        }
        "feature" => (
            "features",
            format!("{}.md", slugify(title)),
            templates::feature(title, &date),
        ),
        other => {
            return Err(AppError::Usage(format!(
                "unknown type {other:?}; known types: {}",
                TYPES.join(", ")
            )));
        }
    };

    let dir = bundle_dir.join(folder);
    fs::create_dir_all(&dir)?;
    let path = dir.join(&filename);
    if path.exists() {
        return Err(AppError::Io(format!("{} already exists", path.display())));
    }
    fs::write(&path, content)?;
    Ok(path)
}

/// `okq init`: scaffold a Full-OKF skeleton; returns a per-file report. Creates
/// only absent files; the README is updated non-destructively.
pub fn init(bundle_dir: &Path) -> Result<Vec<Action>, AppError> {
    let name = bundle_name(bundle_dir);
    let date = templates::today_iso();
    let mut report = Vec::new();

    ensure_file(
        bundle_dir,
        "index.md",
        &templates::root_index(&name),
        &mut report,
    )?;
    ensure_file(
        bundle_dir,
        "adrs/index.md",
        &templates::adrs_index(),
        &mut report,
    )?;
    ensure_file(
        bundle_dir,
        "features/index.md",
        &templates::features_index(),
        &mut report,
    )?;

    // Seed ADR-0001 only if the bundle has no ADRs yet.
    if !has_adrs(&bundle_dir.join("adrs")) {
        ensure_file(
            bundle_dir,
            "adrs/0001-record-architecture-decisions.md",
            &templates::seed_adr(&date),
            &mut report,
        )?;
    }

    report.push(ensure_readme(bundle_dir, &name)?);
    Ok(report)
}

/// Writes `rel` with `content` if absent; records the outcome.
fn ensure_file(
    bundle_dir: &Path,
    rel: &str,
    content: &str,
    report: &mut Vec<Action>,
) -> Result<(), AppError> {
    let path = bundle_dir.join(rel);
    if path.exists() {
        report.push(Action {
            path: rel.to_string(),
            verb: "exists",
        });
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, content)?;
    report.push(Action {
        path: rel.to_string(),
        verb: "created",
    });
    Ok(())
}

/// Creates a base README, or injects the okq block into an existing one.
fn ensure_readme(bundle_dir: &Path, name: &str) -> Result<Action, AppError> {
    let path = bundle_dir.join("README.md");
    if !path.exists() {
        fs::write(&path, templates::base_readme(name))?;
        return Ok(Action {
            path: "README.md".to_string(),
            verb: "created",
        });
    }
    let original = fs::read_to_string(&path)?;
    let updated = templates::inject_okq_block(&templates::ensure_type_readme(&original));
    if updated != original {
        fs::write(&path, updated)?;
    }
    Ok(Action {
        path: "README.md".to_string(),
        verb: "updated",
    })
}

/// The next ADR number = highest existing numeric prefix + 1 (1 if none).
fn next_adr_number(adrs_dir: &Path) -> u32 {
    let mut max = 0;
    if let Ok(entries) = fs::read_dir(adrs_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") {
                if let Some(number) = name.split('-').next().and_then(|p| p.parse::<u32>().ok()) {
                    max = max.max(number);
                }
            }
        }
    }
    max + 1
}

/// `true` if the directory already holds a numbered ADR.
fn has_adrs(adrs_dir: &Path) -> bool {
    fs::read_dir(adrs_dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.ends_with(".md")
                    && name
                        .split('-')
                        .next()
                        .is_some_and(|p| p.parse::<u32>().is_ok())
            })
        })
        .unwrap_or(false)
}

/// A human name for the bundle, from its canonical directory name.
fn bundle_name(bundle_dir: &Path) -> String {
    fs::canonicalize(bundle_dir)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Knowledge base".to_string())
}
