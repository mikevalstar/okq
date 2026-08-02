//! Integration tests for `okq init` and `okq new`.

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

#[test]
fn init_creates_a_conformant_queryable_bundle() {
    let dir = TempDir::new().unwrap();
    okq(dir.path()).arg("init").assert().success();

    for f in [
        "index.md",
        "README.md",
        "adrs/index.md",
        "features/index.md",
        "adrs/0001-record-architecture-decisions.md",
    ] {
        assert!(dir.path().join(f).exists(), "missing {f}");
    }
    // okf_version marker on the root index.
    assert!(
        fs::read_to_string(dir.path().join("index.md"))
            .unwrap()
            .contains("okf_version")
    );

    // Every concept is typed (conformant).
    let out = stdout(okq(dir.path()).args(["find", "--json"]).assert().success());
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let untyped = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r.get("type").is_none())
        .count();
    assert_eq!(untyped, 0, "scaffolded bundle has untyped concepts");

    // No dead links.
    okq(dir.path()).arg("deadlinks").assert().success();
}

#[test]
fn init_is_idempotent() {
    let dir = TempDir::new().unwrap();
    okq(dir.path()).arg("init").assert().success();
    let readme_once = fs::read_to_string(dir.path().join("README.md")).unwrap();
    okq(dir.path()).arg("init").assert().success();
    let readme_twice = fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert_eq!(readme_once, readme_twice);
    // Only one okq block.
    assert_eq!(readme_twice.matches("<!-- okq:begin -->").count(), 1);
}

#[test]
fn init_injects_into_existing_readme_non_destructively() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("README.md"),
        "# My Project\n\nImportant prose.\n",
    )
    .unwrap();
    okq(dir.path()).arg("init").assert().success();
    let readme = fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert!(
        readme.contains("# My Project"),
        "original content preserved"
    );
    assert!(readme.contains("Important prose."));
    assert!(readme.contains("<!-- okq:begin -->"), "okq block injected");
    assert!(
        readme.contains("type: readme"),
        "type added for conformance"
    );
}

#[test]
fn init_does_not_overwrite_an_existing_concept() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("adrs")).unwrap();
    fs::write(
        dir.path().join("adrs/0001-mine.md"),
        "---\ntype: adr\ntitle: Mine\n---\n\n# Mine\n",
    )
    .unwrap();
    okq(dir.path()).arg("init").assert().success();
    // The seed ADR is not added (a numbered ADR already exists)...
    assert!(
        !dir.path()
            .join("adrs/0001-record-architecture-decisions.md")
            .exists()
    );
    // ...and the user's ADR is untouched.
    assert!(
        fs::read_to_string(dir.path().join("adrs/0001-mine.md"))
            .unwrap()
            .contains("# Mine")
    );
}

#[test]
fn new_adr_auto_numbers_and_prints_path() {
    let dir = TempDir::new().unwrap();
    okq(dir.path()).arg("init").assert().success(); // seeds 0001
    let path = stdout(
        okq(dir.path())
            .args(["new", "adr", "Adopt Tantivy"])
            .assert()
            .success(),
    );
    assert!(
        path.trim().ends_with("adrs/0002-adopt-tantivy.md"),
        "got {path}"
    );
    let content = fs::read_to_string(path.trim()).unwrap();
    assert!(content.contains("type: adr"));
    assert!(content.contains("title: Adopt Tantivy"));
    // v0.2 trust keys, not the superseded v0.1 `timestamp` that lint flags
    // (L5) — a scaffolded doc must pass okq's own lint (features/trust.md).
    assert!(content.contains("status: draft"));
    assert!(content.contains("generated: { by: okq/"));
    assert!(!content.contains("timestamp:"));

    // The next one increments.
    let path3 = stdout(
        okq(dir.path())
            .args(["new", "adr", "Another"])
            .assert()
            .success(),
    );
    assert!(path3.trim().ends_with("adrs/0003-another.md"));
}

#[test]
fn new_feature_slugifies() {
    let dir = TempDir::new().unwrap();
    let path = stdout(
        okq(dir.path())
            .args(["new", "feature", "Saved Searches!"])
            .assert()
            .success(),
    );
    assert!(
        path.trim().ends_with("features/saved-searches.md"),
        "got {path}"
    );
}

#[test]
fn new_unknown_type_exits_2() {
    let dir = TempDir::new().unwrap();
    okq(dir.path())
        .args(["new", "widget", "X"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicates::str::contains("known types"));
}

#[test]
fn new_missing_title_exits_2() {
    let dir = TempDir::new().unwrap();
    okq(dir.path())
        .args(["new", "adr"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn new_refuses_to_overwrite() {
    let dir = TempDir::new().unwrap();
    okq(dir.path())
        .args(["new", "feature", "Dup"])
        .assert()
        .success();
    okq(dir.path())
        .args(["new", "feature", "Dup"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicates::str::contains("already exists"));
}

#[test]
fn new_list_shows_types() {
    let dir = TempDir::new().unwrap();
    let out = stdout(okq(dir.path()).args(["new", "--list"]).assert().success());
    assert!(out.contains("adr"));
    assert!(out.contains("feature"));
}

#[test]
fn init_leaves_a_bundle_that_passes_its_own_lint() {
    // A scaffolded bundle used to trip L16 (index.md out of sync) and, downstream
    // of it, L15 (the seeded concepts read as orphans because nothing links to
    // them and no index listed them). `init` now generates the listings itself.
    let dir = tempfile::tempdir().unwrap();
    okq(dir.path()).arg("init").assert().success();
    okq(dir.path())
        .args(["lint", "--check", "--rule", "L15", "--rule", "L16"])
        .assert()
        .success();
}

#[test]
fn init_reports_each_path_once() {
    // A seeded index.md is written twice (template, then listing); reporting it
    // twice reads like a bug.
    let dir = tempfile::tempdir().unwrap();
    let out = okq(dir.path()).arg("init").assert().success();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    let paths: Vec<&str> = stderr
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1))
        .filter(|p| p.ends_with(".md"))
        .collect();
    let mut unique = paths.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(paths.len(), unique.len(), "duplicate paths in {paths:?}");
}

#[test]
fn init_listings_include_the_seed_adr() {
    let dir = tempfile::tempdir().unwrap();
    okq(dir.path()).arg("init").assert().success();
    let index = fs::read_to_string(dir.path().join("adrs/index.md")).unwrap();
    assert!(
        index.contains("<!-- okq:index:begin -->"),
        "no listing block"
    );
    assert!(
        index.contains("0001-record-architecture-decisions.md"),
        "seed ADR missing from its listing:\n{index}"
    );
}

#[test]
fn init_stays_idempotent_with_listing_generation() {
    let dir = tempfile::tempdir().unwrap();
    okq(dir.path()).arg("init").assert().success();
    let first = fs::read_to_string(dir.path().join("adrs/index.md")).unwrap();
    okq(dir.path()).arg("init").assert().success();
    let second = fs::read_to_string(dir.path().join("adrs/index.md")).unwrap();
    assert_eq!(first, second);
    assert_eq!(second.matches("<!-- okq:index:begin -->").count(), 1);
}

#[test]
fn new_points_at_okq_index_without_writing_it() {
    // `new` keeps stdout to the path alone (pipeable) and doesn't rewrite files
    // the caller didn't name; the nudge goes to stderr.
    let dir = tempfile::tempdir().unwrap();
    okq(dir.path()).arg("init").assert().success();
    let before = fs::read_to_string(dir.path().join("adrs/index.md")).unwrap();

    let out = okq(dir.path())
        .args(["new", "adr", "Something New"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();

    assert_eq!(stdout.lines().count(), 1, "stdout must stay pipe-safe");
    assert!(stdout.trim().ends_with("0002-something-new.md"));
    assert!(
        stderr.contains("okq index"),
        "no nudge on stderr: {stderr:?}"
    );
    assert_eq!(
        before,
        fs::read_to_string(dir.path().join("adrs/index.md")).unwrap(),
        "new should not rewrite the listing"
    );
}
