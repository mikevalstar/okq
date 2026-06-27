//! Integration tests for `okq index`.

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

/// A bundle with two ADRs and one feature, no index.md files yet.
fn bundle() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("adrs")).unwrap();
    fs::create_dir_all(dir.path().join("features")).unwrap();
    fs::write(
        dir.path().join("adrs/0001-first.md"),
        "---\ntype: adr\ntitle: ADR-0001 — First\n---\n# First\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("adrs/0002-second.md"),
        "---\ntype: adr\ntitle: ADR-0002 — Second\n---\n# Second\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("features/search.md"),
        "---\ntype: feature\ntitle: Search\n---\n# Search\n",
    )
    .unwrap();
    dir
}

#[test]
fn generates_root_and_subdir_listings() {
    let dir = bundle();
    okq(dir.path()).arg("index").assert().success();

    // Root lists the folders and carries okf_version.
    let root = fs::read_to_string(dir.path().join("index.md")).unwrap();
    assert!(root.contains("okf_version"));
    assert!(root.contains("[adrs/](adrs/)"));
    assert!(root.contains("[features/](features/)"));

    // adrs/index.md lists its two concepts, with no frontmatter.
    let adrs = fs::read_to_string(dir.path().join("adrs/index.md")).unwrap();
    assert!(
        !adrs.starts_with("---"),
        "subdir index.md must not have frontmatter"
    );
    assert!(adrs.contains("[0001-first.md](0001-first.md)"));
    assert!(adrs.contains("ADR-0002 — Second"));
}

#[test]
fn is_idempotent() {
    let dir = bundle();
    okq(dir.path()).arg("index").assert().success();

    let out = stdout(okq(dir.path()).args(["index", "--json"]).assert().success());
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(
        v["files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["verb"] == "unchanged"),
        "a second run should change nothing"
    );
}

#[test]
fn check_gates_on_stale_listings() {
    let dir = bundle();
    okq(dir.path()).arg("index").assert().success();

    // Up to date -> exit 0.
    okq(dir.path())
        .args(["index", "--check"])
        .assert()
        .success();

    // Add a concept -> the adrs listing is now stale -> exit 3, nothing written.
    fs::write(
        dir.path().join("adrs/0003-third.md"),
        "---\ntype: adr\ntitle: ADR-0003 — Third\n---\n# Third\n",
    )
    .unwrap();
    okq(dir.path()).args(["index", "--check"]).assert().code(3);
    let adrs = fs::read_to_string(dir.path().join("adrs/index.md")).unwrap();
    assert!(!adrs.contains("0003-third"), "--check must not write");
}

#[test]
fn preserves_surrounding_prose() {
    let dir = bundle();
    // A hand-written index.md with prose and no markers.
    fs::write(
        dir.path().join("adrs/index.md"),
        "# Decisions\n\nHand-written intro that must survive.\n",
    )
    .unwrap();
    okq(dir.path()).arg("index").assert().success();

    let adrs = fs::read_to_string(dir.path().join("adrs/index.md")).unwrap();
    assert!(adrs.contains("Hand-written intro that must survive."));
    assert!(adrs.contains("0001-first.md"));
}

#[test]
fn empty_bundle_does_not_panic() {
    let dir = TempDir::new().unwrap();
    okq(dir.path()).arg("index").assert().success();
    // Only the root index is written, listing nothing.
    assert!(dir.path().join("index.md").exists());
}
