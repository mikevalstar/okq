//! Integration tests for `.okqignore` (ADR-0006 / features/okqignore.md).
//! Covers global filtering, nested precedence, negation, `--no-ignore`, the
//! get-404 and dead-link consequences, search, and graceful degradation.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

/// A bundle with a real ADR linking to two notes; `notes/` is a candidate for
/// ignoring. Callers drop `.okqignore` files in before running.
fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(root.join("index.md"), "# Bundle\n");
    write(
        root.join("adrs/0001-real.md"),
        "---\n\
         type: adr\n\
         title: Real ADR\n\
         tags: [real]\n\
         related: [\"../notes/scratch.md\", \"../notes/keep.md\"]\n\
         ---\n\
         \n\
         # Real ADR\n\n\
         A [scratch note](../notes/scratch.md) and a [keeper](../notes/keep.md).\n",
    );
    write(
        root.join("notes/scratch.md"),
        "---\ntype: note\ntitle: Scratch\ntags: [draft]\n---\n\n# Scratch\n\nThrowaway.\n",
    );
    write(
        root.join("notes/keep.md"),
        "---\ntype: note\ntitle: Keep\ntags: [draft]\n---\n\n# Keep\n\nKeep this one.\n",
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
    // Isolate the search index cache per invocation tree.
    cmd.env("OKQ_CACHE_DIR", bundle.join(".okq-cache"));
    cmd.arg("--bundle").arg(bundle);
    cmd
}

fn stdout(assert: assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

#[test]
fn directory_ignore_hides_from_find() {
    let dir = fixture();
    write(dir.path().join(".okqignore"), "notes/\n");

    let out = stdout(okq(dir.path()).args(["find", "--json"]).assert().success());
    assert!(out.contains("adrs/0001-real"));
    assert!(!out.contains("notes/scratch"));
    assert!(!out.contains("notes/keep"));
}

#[test]
fn no_ignore_reveals_everything() {
    let dir = fixture();
    write(dir.path().join(".okqignore"), "notes/\n");

    let out = stdout(
        okq(dir.path())
            .args(["--no-ignore", "find", "--json"])
            .assert()
            .success(),
    );
    assert!(out.contains("notes/scratch"));
    assert!(out.contains("notes/keep"));
}

#[test]
fn nested_negation_overrides_root() {
    let dir = fixture();
    // Root ignores notes/; a nested file re-includes keep.md.
    write(dir.path().join(".okqignore"), "notes/\n");
    write(dir.path().join("notes/.okqignore"), "!keep.md\n");

    let out = stdout(okq(dir.path()).args(["find", "--json"]).assert().success());
    assert!(out.contains("notes/keep"));
    assert!(!out.contains("notes/scratch"));
}

#[test]
fn negation_in_single_file() {
    let dir = fixture();
    write(dir.path().join(".okqignore"), "notes/\n!notes/keep.md\n");

    let out = stdout(okq(dir.path()).args(["find", "--json"]).assert().success());
    assert!(out.contains("notes/keep"));
    assert!(!out.contains("notes/scratch"));
}

#[test]
fn get_ignored_concept_is_not_found() {
    let dir = fixture();
    write(dir.path().join(".okqignore"), "notes/\n");

    // Exit 4 = concept not found (the shared taxonomy).
    okq(dir.path())
        .args(["get", "notes/scratch"])
        .assert()
        .code(4);

    // ...but --no-ignore can still reach it.
    okq(dir.path())
        .args(["--no-ignore", "get", "notes/scratch"])
        .assert()
        .success();
}

#[test]
fn link_into_ignored_concept_is_dead() {
    let dir = fixture();
    // Ignore scratch only; the ADR's link/relation to it becomes dead, while
    // its link to keep.md stays healthy.
    write(dir.path().join(".okqignore"), "notes/scratch.md\n");

    let out = stdout(
        okq(dir.path())
            .args(["deadlinks", "--json"])
            .assert()
            .success(),
    );
    assert!(out.contains("scratch"));
    assert!(!out.contains("keep"));
}

#[test]
fn stats_counts_only_visible_concepts() {
    let dir = fixture();
    write(dir.path().join(".okqignore"), "notes/\n");

    let out = stdout(okq(dir.path()).args(["stats", "--json"]).assert().success());
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    // Only the real ADR remains a concept (index.md is reserved).
    assert_eq!(v["concepts"], 1);
}

#[test]
fn search_skips_ignored_concepts() {
    let dir = fixture();
    write(dir.path().join(".okqignore"), "notes/\n");

    // "Throwaway" only appears in the ignored scratch note.
    let out = stdout(
        okq(dir.path())
            .args(["search", "throwaway", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["count"], 0);

    // With --no-ignore it surfaces again (separate cache, so no bleed-through).
    let out = stdout(
        okq(dir.path())
            .args(["--no-ignore", "search", "throwaway", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["count"], 1);
}

#[test]
fn comment_only_ignore_file_is_harmless() {
    let dir = fixture();
    write(dir.path().join(".okqignore"), "# just a comment\n\n");

    // Nothing excluded; all three concepts visible, no panic.
    let out = stdout(okq(dir.path()).args(["find", "--json"]).assert().success());
    assert!(out.contains("notes/scratch"));
    assert!(out.contains("notes/keep"));
    assert!(out.contains("adrs/0001-real"));
}

#[test]
fn editing_ignore_rebuilds_search_index() {
    let dir = fixture();
    write(dir.path().join(".okqignore"), "notes/\n");

    // First search builds the index over the filtered set: no hit.
    let out = stdout(
        okq(dir.path())
            .args(["search", "throwaway", "--json"])
            .assert()
            .success(),
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&out).unwrap()["count"],
        0
    );

    // Loosen the rules; the index must rebuild (ignore file is stamped).
    write(dir.path().join(".okqignore"), "# nothing ignored now\n");
    let out = stdout(
        okq(dir.path())
            .args(["search", "throwaway", "--json"])
            .assert()
            .success(),
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&out).unwrap()["count"],
        1
    );
}
