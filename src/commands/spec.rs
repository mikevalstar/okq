//! `okq spec` — print the OKF specification this build implements, verbatim.
//!
//! The text is an exact copy of the upstream spec, embedded at compile time
//! from `spec/SPEC.md` (provenance and update procedure in `spec/README.md`,
//! decision in ADR-0015). Printing it verbatim is the contract: an agent that
//! wants the authoritative format rules gets them from the binary it already
//! has — offline, pinned to the version okq actually targets. `--section`
//! reuses `get`'s heading-or-slug matcher for expand-on-demand reads.

use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::SpecArgs;
use crate::error::AppError;
use crate::sections;

/// Schema tag stamped on every `spec` JSON document.
pub const SCHEMA: &str = "okq.spec/v1";

/// The embedded spec text — byte-for-byte the upstream `okf/SPEC.md`.
pub const SPEC_TEXT: &str = include_str!("../../spec/SPEC.md");

/// The OKF version the embedded spec (and this okq build) targets.
pub const OKF_VERSION: &str = "0.2";

/// Where the embedded copy came from.
pub const SOURCE_URL: &str =
    "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md";

/// The JSON envelope for `okq spec`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SpecOutput {
    /// Schema tag (`okq.spec/v1`).
    pub schema: &'static str,
    /// The OKF spec version the text specifies.
    pub okf_version: &'static str,
    /// Upstream location of the original document.
    pub source: &'static str,
    /// License of the spec text (upstream's, retained on redistribution).
    pub license: &'static str,
    /// The resolved heading, present only with `--section`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// 1-based line of the resolved heading within the spec, with `--section`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// The spec text: the whole document, or just the selected section.
    pub text: String,
}

/// Runs `spec`: the whole document, or one section by heading text or slug.
pub fn run(args: &SpecArgs) -> Result<SpecOutput, AppError> {
    let (section, line, text) = match &args.section {
        None => (None, None, SPEC_TEXT.to_string()),
        Some(query) => {
            let secs = sections::parse_sections(SPEC_TEXT, sections::body_start_line(SPEC_TEXT));
            let chosen = super::get::select_section(&secs, query, "the OKF spec")?;
            (
                Some(chosen.heading.clone()),
                Some(chosen.line),
                chosen.body.clone(),
            )
        }
    };
    Ok(SpecOutput {
        schema: SCHEMA,
        okf_version: OKF_VERSION,
        source: SOURCE_URL,
        license: "Apache-2.0",
        section,
        line,
        text,
    })
}

/// Serializes the envelope as one pretty JSON document.
pub fn to_json(out: &SpecOutput) -> String {
    serde_json::to_string_pretty(out).expect("SpecOutput is always serializable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::SpecArgs;

    #[test]
    fn whole_spec_is_verbatim() {
        let out = run(&SpecArgs { section: None }).unwrap();
        assert_eq!(out.text, SPEC_TEXT);
        assert_eq!(out.okf_version, "0.2");
        assert!(out.section.is_none());
    }

    #[test]
    fn embedded_spec_looks_like_the_spec() {
        // Guard the vendored copy: right document, right version, unmangled.
        assert!(SPEC_TEXT.starts_with("# Open Knowledge Format (OKF)"));
        assert!(SPEC_TEXT.contains("**Version 0.2**"));
        assert!(SPEC_TEXT.contains("## 11. Conformance"));
    }

    #[test]
    fn section_by_heading_and_slug() {
        for query in ["5.3 Trust tiers", "5-3-trust-tiers"] {
            let out = run(&SpecArgs {
                section: Some(query.into()),
            })
            .unwrap();
            assert_eq!(out.section.as_deref(), Some("5.3 Trust tiers"));
            assert!(out.text.contains("machine-confirmed"));
            assert!(!out.text.contains("## 6."), "section must end at siblings");
        }
    }

    #[test]
    fn missing_section_is_exit_5() {
        let err = run(&SpecArgs {
            section: Some("no such heading".into()),
        })
        .unwrap_err();
        assert_eq!(err.exit_code(), crate::error::exit::SECTION);
    }
}
