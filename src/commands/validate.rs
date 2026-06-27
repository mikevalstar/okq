//! `okq validate` (alias `doctor`) — report OKF conformance diagnostics.
//! See `docs/features/validate.md`. A thin presentation layer over
//! `okf::validate_bundle`: okq adds the envelope, `--json`, severity filtering,
//! and the exit-code contract. The rule set itself lives in okf.

use std::io::Write;
use std::path::Path;

use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::{SeverityArg, ValidateArgs};
use crate::error::AppError;
use crate::view::Corpus;

/// Schema tag stamped on every `validate` JSON document.
pub const SCHEMA: &str = "okq.validate/v1";

/// The `okq.validate/v1` envelope.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidateOutput {
    /// Schema tag (`okq.validate/v1`).
    pub schema: &'static str,
    /// True when the bundle has no error-severity diagnostics (§9 conformant).
    pub conformant: bool,
    /// Total error-severity diagnostics (independent of the `--severity` floor).
    pub errors: usize,
    /// Total warning-severity diagnostics.
    pub warnings: usize,
    /// Total info-severity diagnostics.
    pub infos: usize,
    /// Diagnostics at or above the requested severity floor, sorted
    /// deterministically (severity desc, then path, then message).
    pub diagnostics: Vec<Diagnostic>,
}

/// One conformance finding.
#[derive(Debug, Serialize, JsonSchema)]
pub struct Diagnostic {
    /// `error` | `warning` | `info`.
    pub severity: String,
    /// Bundle-relative path the finding relates to, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Human-readable description of the issue.
    pub message: String,
}

/// Severity rank, for the `--severity` floor and deterministic ordering.
fn rank(severity: okf::Severity) -> u8 {
    match severity {
        okf::Severity::Error => 2,
        okf::Severity::Warning => 1,
        okf::Severity::Info => 0,
    }
}

/// Rank of an already-stringified severity, for sorting the output list.
fn rank_str(severity: &str) -> u8 {
    match severity {
        "error" => 2,
        "warning" => 1,
        _ => 0,
    }
}

fn severity_str(severity: okf::Severity) -> &'static str {
    match severity {
        okf::Severity::Error => "error",
        okf::Severity::Warning => "warning",
        okf::Severity::Info => "info",
    }
}

/// Runs `validate` against the bundle at `bundle_dir`.
pub fn run(
    bundle_dir: &Path,
    args: &ValidateArgs,
    no_ignore: bool,
) -> Result<ValidateOutput, AppError> {
    let corpus = Corpus::load(bundle_dir, no_ignore)?;
    let report = okf::validate_bundle(corpus.bundle());

    // Drop findings for files excluded by .okqignore *first*, so counts and
    // conformance reflect the queryable bundle — not docs okq never loads.
    let kept: Vec<&okf::Diagnostic> = report
        .diagnostics
        .iter()
        .filter(|d| {
            d.path
                .as_deref()
                .map(|p| !corpus.ignore().is_ignored(p))
                .unwrap_or(true)
        })
        .collect();

    let count = |sev: okf::Severity| kept.iter().filter(|d| d.severity == sev).count();
    let errors = count(okf::Severity::Error);

    let floor = match args.severity {
        SeverityArg::Error => 2,
        SeverityArg::Warning => 1,
        SeverityArg::Info => 0,
    };

    // Render paths bundle-relative, like every other command (okf hands back
    // walked, bundle-prefixed paths).
    let root = corpus.bundle().root();
    let mut diagnostics: Vec<Diagnostic> = kept
        .iter()
        .filter(|d| rank(d.severity) >= floor)
        .map(|d| Diagnostic {
            severity: severity_str(d.severity).to_string(),
            path: d
                .path
                .as_ref()
                .map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string()),
            message: d.message.clone(),
        })
        .collect();

    // Deterministic order: severity desc, then path, then message.
    diagnostics.sort_by(|a, b| {
        rank_str(&b.severity)
            .cmp(&rank_str(&a.severity))
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.message.cmp(&b.message))
    });

    Ok(ValidateOutput {
        schema: SCHEMA,
        conformant: errors == 0,
        errors,
        warnings: count(okf::Severity::Warning),
        infos: count(okf::Severity::Info),
        diagnostics,
    })
}

/// Serializes the envelope as pretty JSON.
pub fn to_json(out: &ValidateOutput) -> String {
    serde_json::to_string_pretty(out).expect("ValidateOutput is always serializable")
}

/// Human rendering: one diagnostic per line, `severity  path  message`.
pub fn render_human(
    w: &mut impl Write,
    out: &ValidateOutput,
    no_color: bool,
) -> std::io::Result<()> {
    for d in &out.diagnostics {
        let style = sev_style(&d.severity, no_color);
        match &d.path {
            Some(p) => writeln!(w, "{style}{:>7}{style:#}  {}  {}", d.severity, p, d.message)?,
            None => writeln!(w, "{style}{:>7}{style:#}  {}", d.severity, d.message)?,
        }
    }
    Ok(())
}

/// Color for a severity label (red / yellow / dim), honoring `--no-color`.
fn sev_style(severity: &str, no_color: bool) -> anstyle::Style {
    if no_color {
        return anstyle::Style::new();
    }
    match severity {
        "error" => anstyle::Style::new()
            .fg_color(Some(anstyle::AnsiColor::Red.into()))
            .bold(),
        "warning" => anstyle::Style::new().fg_color(Some(anstyle::AnsiColor::Yellow.into())),
        _ => anstyle::Style::new().dimmed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_orders_error_highest() {
        assert!(rank(okf::Severity::Error) > rank(okf::Severity::Warning));
        assert!(rank(okf::Severity::Warning) > rank(okf::Severity::Info));
        assert_eq!(rank_str("error"), rank(okf::Severity::Error));
        assert_eq!(rank_str("warning"), rank(okf::Severity::Warning));
        assert_eq!(rank_str("info"), rank(okf::Severity::Info));
    }
}
