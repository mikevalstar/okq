//! Integration tests for `okq validate` (alias `doctor`).

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

fn okq(bundle: &Path) -> Command {
    let mut cmd = Command::cargo_bin("okq").unwrap();
    cmd.arg("--bundle").arg(bundle);
    cmd
}

fn stdout(assert: assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

/// A bundle with one fully-conformant concept and one with broken frontmatter.
fn mixed_bundle() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("good.md"),
        "---\ntype: note\ntitle: Good\ndescription: A fine doc.\ntimestamp: 2026-06-27\n---\n\n# Good\n",
    )
    .unwrap();
    // No frontmatter at all -> missing required `type` (a conformance error).
    fs::write(
        dir.path().join("bad.md"),
        "# Just a heading, no frontmatter\n",
    )
    .unwrap();
    dir
}

#[test]
fn reports_the_silently_dropped_doc_as_an_error() {
    let dir = mixed_bundle();
    let out = stdout(
        okq(dir.path())
            .args(["validate", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["schema"], "okq.validate/v1");
    assert_eq!(v["conformant"], false);
    assert!(v["errors"].as_u64().unwrap() >= 1);
    let mentions_bad = v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["path"].as_str() == Some("bad.md") && d["severity"] == "error");
    assert!(mentions_bad, "bad.md should be flagged as an error");
}

#[test]
fn doctor_is_an_alias() {
    let dir = mixed_bundle();
    // Both spellings produce the same JSON.
    let v = stdout(
        okq(dir.path())
            .args(["validate", "--json"])
            .assert()
            .success(),
    );
    let d = stdout(
        okq(dir.path())
            .args(["doctor", "--json"])
            .assert()
            .success(),
    );
    assert_eq!(v, d);
}

#[test]
fn check_gates_on_conformance() {
    let dir = mixed_bundle();
    // Non-conformant -> exit 3.
    okq(dir.path())
        .args(["validate", "--check"])
        .assert()
        .code(3);

    // Remove the bad doc -> conformant -> exit 0.
    fs::remove_file(dir.path().join("bad.md")).unwrap();
    okq(dir.path())
        .args(["validate", "--check"])
        .assert()
        .success();
}

#[test]
fn severity_floor_filters_the_list() {
    let dir = mixed_bundle();
    // good.md is complete, so at --severity error only the bad.md error shows.
    let out = stdout(
        okq(dir.path())
            .args(["validate", "--severity", "error", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(
        v["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["severity"] == "error"),
        "only errors should show at the error floor"
    );
}

#[test]
fn clean_bundle_is_conformant() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("a.md"),
        "---\ntype: note\ntitle: A\ndescription: x\ntimestamp: 2026-06-27\n---\n\n# A\n",
    )
    .unwrap();
    let out = stdout(
        okq(dir.path())
            .args(["validate", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["conformant"], true);
    assert_eq!(v["errors"], 0);
}

#[test]
fn empty_bundle_does_not_panic() {
    let dir = TempDir::new().unwrap();
    okq(dir.path()).arg("validate").assert().success();
}

#[test]
fn staleness_is_reproducible_against_today() {
    // okf 0.2.7 moved the staleness rule out of lint and into validate (V12),
    // so `--today` lives here now (ADR-0016).
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("a.md"),
        "---\ntype: doc\ntitle: A\ndescription: d\nstale_after: 2026-01-01T00:00:00Z\n---\n\n# A\n\nBody.\n",
    )
    .unwrap();

    let count_stale = |today: &str| {
        let out = stdout(
            okq(dir.path())
                .args(["validate", "--severity", "info", "--today", today, "--json"])
                .assert()
                .success(),
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        v["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["message"].as_str().unwrap().contains("stale"))
            .count()
    };

    assert_eq!(count_stale("2026-08-02"), 1);
    assert_eq!(count_stale("2025-12-31"), 0);
}

#[test]
fn a_malformed_today_is_a_usage_error() {
    let dir = mixed_bundle();
    okq(dir.path())
        .args(["validate", "--today", "whenever"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn a_bare_date_stale_after_still_reports_stale_in_the_envelope() {
    // okq reads bundles its own older releases scaffolded: a `stale_after`
    // without a UTC offset must not silently stop being stale (ADR-0016).
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("a.md"),
        "---\ntype: doc\ntitle: A\ndescription: d\nstale_after: 2026-01-01\n---\n\n# A\n\nBody.\n",
    )
    .unwrap();
    let out = stdout(
        okq(dir.path())
            .args(["find", "--today", "2026-08-02", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["results"][0]["stale"], true);
}
