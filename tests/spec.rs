//! Tests for `okq spec` — the embedded OKF specification, printed verbatim.

use assert_cmd::Command;

/// The same file the binary embeds; equality here means byte-for-byte fidelity.
const SPEC_TEXT: &str = include_str!("../spec/SPEC.md");

fn okq() -> Command {
    Command::cargo_bin("okq").unwrap()
}

fn stdout(args: &[&str]) -> String {
    let out = okq().args(args).assert().success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

#[test]
fn prints_the_exact_spec_text() {
    assert_eq!(stdout(&["spec"]), SPEC_TEXT);
}

#[test]
fn needs_no_bundle() {
    // `spec` must work anywhere — an empty directory is not an error.
    let dir = tempfile::tempdir().unwrap();
    okq().current_dir(dir.path()).arg("spec").assert().success();
}

#[test]
fn json_envelope_carries_version_provenance_and_text() {
    let v: serde_json::Value = serde_json::from_str(&stdout(&["spec", "--json"])).unwrap();
    assert_eq!(v["schema"], "okq.spec/v1");
    assert_eq!(v["okf_version"], "0.2");
    assert_eq!(v["license"], "Apache-2.0");
    assert!(v["source"].as_str().unwrap().contains("knowledge-catalog"));
    assert_eq!(v["text"], SPEC_TEXT);
    assert!(v.get("section").is_none());
}

#[test]
fn section_selects_one_heading() {
    let text = stdout(&["spec", "--section", "11. Conformance"]);
    assert!(text.starts_with("## 11. Conformance"));
    assert!(text.contains("conformant"));
    assert!(!text.contains("## 12."), "must stop at the next sibling");

    // Slug form resolves to the same section.
    assert_eq!(text, stdout(&["spec", "--section", "11-conformance"]));
}

#[test]
fn section_json_reports_heading_and_line() {
    let v: serde_json::Value =
        serde_json::from_str(&stdout(&["spec", "--section", "5.3 Trust tiers", "--json"])).unwrap();
    assert_eq!(v["section"], "5.3 Trust tiers");
    assert!(v["line"].as_u64().unwrap() > 1);
    assert!(v["text"].as_str().unwrap().contains("machine-confirmed"));
}

#[test]
fn unknown_section_exits_5() {
    okq()
        .args(["spec", "--section", "no such heading"])
        .assert()
        .failure()
        .code(5);
}
