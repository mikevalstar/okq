//! `okq lint` — opinionated bundle hygiene, beside conformance.
//!
//! See `docs/features/lint.md`. A presentation layer over `okf::lint_bundle_at`,
//! shaped like [`validate`](super::validate) on purpose: same severity model,
//! same `--check` gate, same `.okqignore` filtering. What differs is authority —
//! lint is an *opinion* and never reports an error severity, so a bundle with
//! findings is still conformant (ADR-0014).
//!
//! okf tags each finding with a rule code by formatting it into the message as
//! `[L7] …`; okq lifts that into a structured `rule` field so agents and CI can
//! pin a rule without regexing prose. A message that doesn't carry a
//! recognizable prefix passes through unchanged with `rule: null`.

use std::io::Write;
use std::path::Path;

use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::{LintArgs, SeverityArg};
use crate::error::AppError;
use crate::trust;
use crate::view::Corpus;

/// Schema tag stamped on every `lint` JSON document.
pub const SCHEMA: &str = "okq.lint/v1";

/// The rule codes okf 0.2 emits, for validating `--rule` / `--ignore`.
pub const RULES: [&str; 16] = [
    "L1", "L2", "L3", "L4", "L5", "L6", "L7", "L8", "L9", "L10", "L11", "L12", "L13", "L14", "L15",
    "L16",
];

/// The `okq.lint/v1` envelope.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LintOutput {
    /// Schema tag (`okq.lint/v1`).
    pub schema: &'static str,
    /// Total findings that survived every filter.
    pub findings: usize,
    /// Warning-severity findings among them.
    pub warnings: usize,
    /// Info-severity findings among them.
    pub infos: usize,
    /// The findings, sorted deterministically (severity desc, rule, path,
    /// message).
    pub diagnostics: Vec<Finding>,
}

/// One lint finding.
#[derive(Debug, Serialize, JsonSchema)]
pub struct Finding {
    /// `warning` | `info`. Lint never reports `error`.
    pub severity: String,
    /// The okf rule code (`L1`–`L16`), or null if the message carried no
    /// recognizable code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    /// Bundle-relative path the finding relates to, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Human-readable description, with the `[Lnn] ` prefix stripped.
    pub message: String,
}

/// Splits okf's `[Lnn] message` into its code and the message proper. A message
/// with no such prefix yields `(None, message)` unchanged — okq owns this parse,
/// so it degrades rather than breaks if okf's format moves (ADR-0014).
fn split_rule(message: &str) -> (Option<String>, String) {
    let Some(rest) = message.strip_prefix('[') else {
        return (None, message.to_string());
    };
    let Some((code, tail)) = rest.split_once(']') else {
        return (None, message.to_string());
    };
    let is_rule =
        code.len() >= 2 && code.starts_with('L') && code[1..].chars().all(|c| c.is_ascii_digit());
    if !is_rule {
        return (None, message.to_string());
    }
    (Some(code.to_string()), tail.trim_start().to_string())
}

/// Normalizes a `--rule`/`--ignore` value and checks it names a real rule.
fn parse_rule(raw: &str) -> Result<String, AppError> {
    let code = raw.trim().to_uppercase();
    if RULES.contains(&code.as_str()) {
        Ok(code)
    } else {
        Err(AppError::Usage(format!(
            "unknown lint rule {raw:?}; known: {}",
            RULES.join(", ")
        )))
    }
}

fn severity_str(severity: okf::Severity) -> &'static str {
    match severity {
        okf::Severity::Error => "error",
        okf::Severity::Warning => "warning",
        okf::Severity::Info => "info",
    }
}

fn rank_str(severity: &str) -> u8 {
    match severity {
        "error" => 2,
        "warning" => 1,
        _ => 0,
    }
}

/// Sorts `L2` before `L10` — string order would not.
fn rule_key(rule: Option<&str>) -> (u8, u32) {
    match rule.and_then(|r| r.strip_prefix('L')?.parse::<u32>().ok()) {
        Some(n) => (0, n),
        None => (1, 0),
    }
}

/// Runs `lint` against the bundle at `bundle_dir`.
pub fn run(bundle_dir: &Path, args: &LintArgs, no_ignore: bool) -> Result<LintOutput, AppError> {
    if !args.rule.is_empty() && !args.ignore.is_empty() {
        return Err(AppError::Usage(
            "--rule and --ignore are mutually exclusive; pass one or the other".to_string(),
        ));
    }
    let only: Vec<String> = args
        .rule
        .iter()
        .map(|r| parse_rule(r))
        .collect::<Result<_, _>>()?;
    let ignore: Vec<String> = args
        .ignore
        .iter()
        .map(|r| parse_rule(r))
        .collect::<Result<_, _>>()?;

    let corpus = Corpus::load(bundle_dir, no_ignore)?;
    let today = trust::resolve_today(args.today.as_deref())?;
    let report = okf::lint_bundle_at(corpus.bundle(), today);

    let floor = match args.severity {
        SeverityArg::Error => 2,
        SeverityArg::Warning => 1,
        SeverityArg::Info => 0,
    };
    let root = corpus.bundle().root();

    let mut diagnostics: Vec<Finding> = report
        .diagnostics
        .iter()
        // Drop findings for files .okqignore excludes, like `validate` does, so
        // the report reflects the queryable bundle.
        .filter(|d| {
            d.path
                .as_deref()
                .map(|p| !corpus.ignore().is_ignored(p))
                .unwrap_or(true)
        })
        .map(|d| {
            let (rule, message) = split_rule(&d.message);
            Finding {
                severity: severity_str(d.severity).to_string(),
                rule,
                path: d
                    .path
                    .as_ref()
                    .map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string()),
                message,
            }
        })
        .filter(|f| rank_str(&f.severity) >= floor)
        .filter(|f| {
            let code = f.rule.as_deref();
            let kept_by_only = only.is_empty() || code.is_some_and(|c| only.iter().any(|r| r == c));
            let dropped = code.is_some_and(|c| ignore.iter().any(|r| r == c));
            kept_by_only && !dropped
        })
        .collect();

    diagnostics.sort_by(|a, b| {
        rank_str(&b.severity)
            .cmp(&rank_str(&a.severity))
            .then_with(|| rule_key(a.rule.as_deref()).cmp(&rule_key(b.rule.as_deref())))
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.message.cmp(&b.message))
    });

    let count = |sev: &str| diagnostics.iter().filter(|f| f.severity == sev).count();
    Ok(LintOutput {
        schema: SCHEMA,
        findings: diagnostics.len(),
        warnings: count("warning"),
        infos: count("info"),
        diagnostics,
    })
}

/// Serializes the envelope as pretty JSON.
pub fn to_json(out: &LintOutput) -> String {
    serde_json::to_string_pretty(out).expect("LintOutput is always serializable")
}

/// Human rendering: `severity  rule  path  message`, one per line.
pub fn render_human(w: &mut impl Write, out: &LintOutput, no_color: bool) -> std::io::Result<()> {
    for f in &out.diagnostics {
        let style = sev_style(&f.severity, no_color);
        let rule = f.rule.as_deref().unwrap_or("-");
        match &f.path {
            Some(p) => writeln!(
                w,
                "{style}{:>7}{style:#}  {rule:<4}{}  {}",
                f.severity, p, f.message
            )?,
            None => writeln!(
                w,
                "{style}{:>7}{style:#}  {rule:<4}{}",
                f.severity, f.message
            )?,
        }
    }
    Ok(())
}

fn sev_style(severity: &str, no_color: bool) -> anstyle::Style {
    if no_color {
        return anstyle::Style::new();
    }
    match severity {
        "warning" => anstyle::Style::new().fg_color(Some(anstyle::AnsiColor::Yellow.into())),
        _ => anstyle::Style::new().dimmed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_rule_prefix_off_the_message() {
        let (rule, msg) = split_rule("[L15] no inbound links");
        assert_eq!(rule.as_deref(), Some("L15"));
        assert_eq!(msg, "no inbound links");
    }

    #[test]
    fn a_message_without_a_code_degrades_to_none() {
        // If okf ever changes its prefix format, the message must survive
        // intact rather than being mangled or dropped (ADR-0014).
        for message in [
            "no prefix at all",
            "[not-a-rule] something",
            "[L] missing digits",
            "[unterminated message",
        ] {
            let (rule, msg) = split_rule(message);
            assert_eq!(rule, None, "{message:?}");
            assert_eq!(msg, message, "{message:?}");
        }
    }

    #[test]
    fn rules_are_parsed_case_insensitively_and_validated() {
        assert_eq!(parse_rule("l15").unwrap(), "L15");
        assert_eq!(parse_rule(" L4 ").unwrap(), "L4");
        assert!(parse_rule("L99").is_err());
        assert!(parse_rule("nonsense").is_err());
    }

    #[test]
    fn rule_ordering_is_numeric_not_lexical() {
        assert!(rule_key(Some("L2")) < rule_key(Some("L10")));
        // Codeless findings sort last.
        assert!(rule_key(Some("L16")) < rule_key(None));
    }
}
