//! Integration tests for `okq get`, run against a controlled fixture bundle so
//! snapshots and `path:line` values are stable. Covers the feature spec's
//! acceptance criteria: selectors, identity forms, sections, and exit codes.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

/// Builds a small fixture bundle and returns the temp dir (keep it alive for
/// the duration of the test).
fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Reserved file — must never be concept-addressable.
    write(root.join("index.md"), "# Bundle\n\n- tables/users\n");

    write(
        root.join("tables/users.md"),
        "---\n\
         type: table\n\
         title: Users\n\
         description: The users table.\n\
         tags: [pii, core]\n\
         ---\n\
         \n\
         # Users\n\
         \n\
         The users table.\n\
         \n\
         ## Schema\n\
         \n\
         - id: int\n\
         - email: text\n\
         \n\
         ## Notes\n\
         \n\
         PII lives here.\n",
    );

    // Two identically-named sections → ambiguous selection.
    write(
        root.join("tables/orders.md"),
        "---\n\
         type: table\n\
         title: Orders\n\
         ---\n\
         \n\
         # Orders\n\
         \n\
         ## Notes\n\
         \n\
         first\n\
         \n\
         ## Notes\n\
         \n\
         second\n",
    );

    dir
}

fn write(path: impl AsRef<Path>, contents: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn okq(bundle: &Path) -> Command {
    let mut cmd = Command::cargo_bin("okq").unwrap();
    cmd.arg("--bundle").arg(bundle);
    cmd
}

fn stdout(assert: assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

#[test]
fn default_json_envelope() {
    let dir = fixture();
    let out = stdout(
        okq(dir.path())
            .args(["get", "tables/users", "--json"])
            .assert()
            .success(),
    );
    insta::assert_snapshot!("default_json", out);
}

#[test]
fn section_json_envelope() {
    let dir = fixture();
    let out = stdout(
        okq(dir.path())
            .args(["get", "tables/users", "--section", "Schema", "--json"])
            .assert()
            .success(),
    );
    insta::assert_snapshot!("section_json", out);
}

#[test]
fn frontmatter_only_omits_body() {
    let dir = fixture();
    let out = stdout(
        okq(dir.path())
            .args(["get", "tables/users", "--frontmatter"])
            .assert()
            .success(),
    );
    assert!(out.contains("type: table"), "should show frontmatter");
    assert!(!out.contains("## Schema"), "should not show body");
}

#[test]
fn body_only_omits_frontmatter() {
    let dir = fixture();
    let out = stdout(
        okq(dir.path())
            .args(["get", "tables/users", "--body"])
            .assert()
            .success(),
    );
    assert!(out.contains("## Schema"), "should show body");
    assert!(!out.contains("type: table"), "should not show frontmatter");
}

#[test]
fn section_by_slug() {
    let dir = fixture();
    okq(dir.path())
        .args(["get", "tables/users", "--section", "schema"])
        .assert()
        .success()
        .stdout(predicates::str::contains("email: text"));
}

#[test]
fn md_path_form_resolves() {
    let dir = fixture();
    okq(dir.path())
        .args(["get", "tables/users.md"])
        .assert()
        .success();
}

#[test]
fn missing_concept_exits_4() {
    let dir = fixture();
    okq(dir.path())
        .args(["get", "tables/missing"])
        .assert()
        .failure()
        .code(4);
}

#[test]
fn reserved_index_is_not_a_concept() {
    let dir = fixture();
    okq(dir.path())
        .args(["get", "index"])
        .assert()
        .failure()
        .code(4);
}

#[test]
fn missing_section_exits_5() {
    let dir = fixture();
    okq(dir.path())
        .args(["get", "tables/users", "--section", "Nonexistent"])
        .assert()
        .failure()
        .code(5);
}

#[test]
fn ambiguous_section_exits_5() {
    let dir = fixture();
    okq(dir.path())
        .args(["get", "tables/orders", "--section", "Notes"])
        .assert()
        .failure()
        .code(5);
}

#[test]
fn missing_concept_arg_is_usage_error() {
    let dir = fixture();
    okq(dir.path()).arg("get").assert().failure().code(2);
}
