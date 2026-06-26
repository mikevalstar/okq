//! Integration tests for `okq find`, run against a controlled fixture bundle.
//! Covers predicates, boolean combination, output envelope, and exit codes.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(root.join("index.md"), "# Bundle\n");

    write(
        root.join("adrs/0001-pick-rust.md"),
        "---\n\
         type: adr\n\
         title: Pick Rust\n\
         status: accepted\n\
         tags: [rust, lang]\n\
         ---\n\
         \n\
         # Pick Rust\n\nWe chose Rust for speed.\n",
    );
    write(
        root.join("adrs/0002-pick-tantivy.md"),
        "---\n\
         type: adr\n\
         title: Pick Tantivy\n\
         status: draft\n\
         tags: [rust, search]\n\
         ---\n\
         \n\
         # Pick Tantivy\n\nTantivy gives us BM25.\n",
    );
    write(
        root.join("guides/style.md"),
        "---\n\
         type: guide\n\
         title: Style Guide\n\
         status: accepted\n\
         tags: [docs]\n\
         ---\n\
         \n\
         # Style Guide\n\nWrite clearly.\n",
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
fn json_envelope_for_type() {
    let dir = fixture();
    let out = stdout(
        okq(dir.path())
            .args(["find", "--type", "adr", "--json"])
            .assert()
            .success(),
    );
    insta::assert_snapshot!("find_type_adr", out);
}

#[test]
fn tag_filter() {
    let dir = fixture();
    let out = stdout(
        okq(dir.path())
            .args(["find", "--tag", "search"])
            .assert()
            .success(),
    );
    assert!(out.contains("0002-pick-tantivy"));
    assert!(!out.contains("0001-pick-rust"));
    assert!(!out.contains("style"));
}

#[test]
fn repeated_tag_is_and() {
    let dir = fixture();
    // rust AND search → only tantivy
    let out = stdout(
        okq(dir.path())
            .args(["find", "--tag", "rust", "--tag", "search"])
            .assert()
            .success(),
    );
    assert!(out.contains("0002-pick-tantivy"));
    assert!(!out.contains("0001-pick-rust"));
}

#[test]
fn repeated_type_is_or() {
    let dir = fixture();
    let out = stdout(
        okq(dir.path())
            .args(["find", "--type", "adr", "--type", "guide", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["count"], 3);
}

#[test]
fn cross_flag_is_and() {
    let dir = fixture();
    // type adr AND status accepted → only 0001
    let out = stdout(
        okq(dir.path())
            .args(["find", "--type", "adr", "--where", "status=accepted"])
            .assert()
            .success(),
    );
    assert!(out.contains("0001-pick-rust"));
    assert!(!out.contains("0002-pick-tantivy"));
}

#[test]
fn where_sequence_membership() {
    let dir = fixture();
    // --where tags=lang behaves like membership
    okq(dir.path())
        .args(["find", "--where", "tags=lang"])
        .assert()
        .success()
        .stdout(predicates::str::contains("0001-pick-rust"));
}

#[test]
fn match_is_case_insensitive_substring() {
    let dir = fixture();
    okq(dir.path())
        .args(["find", "--match", "bm25"])
        .assert()
        .success()
        .stdout(predicates::str::contains("0002-pick-tantivy"));
}

#[test]
fn match_regex() {
    let dir = fixture();
    okq(dir.path())
        .args(["find", "--match", "BM[0-9]+", "--regex"])
        .assert()
        .success()
        .stdout(predicates::str::contains("0002-pick-tantivy"));
}

#[test]
fn no_predicates_lists_all_concepts() {
    let dir = fixture();
    let out = stdout(okq(dir.path()).args(["find", "--json"]).assert().success());
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["count"], 3); // index.md excluded
}

#[test]
fn empty_result_exits_zero() {
    let dir = fixture();
    let out = stdout(
        okq(dir.path())
            .args(["find", "--tag", "nonexistent", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["count"], 0);
}

#[test]
fn malformed_where_exits_2() {
    let dir = fixture();
    okq(dir.path())
        .args(["find", "--where", "nopredicate"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn invalid_regex_exits_2() {
    let dir = fixture();
    okq(dir.path())
        .args(["find", "--match", "[", "--regex"])
        .assert()
        .failure()
        .code(2);
}
