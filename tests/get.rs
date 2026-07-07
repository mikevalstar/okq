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
fn bare_name_resolves_to_unique_concept() {
    // `users` (not `tables/users`) uniquely identifies the concept.
    let dir = fixture();
    okq(dir.path())
        .args(["get", "users", "--frontmatter"])
        .assert()
        .success()
        .stdout(predicates::str::contains("title: Users"));
}

#[test]
fn partial_does_not_match_arbitrary_substring() {
    // `ser` is a substring of `users` but not a path-segment suffix.
    let dir = fixture();
    okq(dir.path())
        .args(["get", "ser"])
        .assert()
        .failure()
        .code(4);
}

#[test]
fn ambiguous_partial_exits_4() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path().join("a/notes.md"),
        "---\ntype: note\ntitle: A Notes\n---\n\n# Notes\n",
    );
    write(
        dir.path().join("b/notes.md"),
        "---\ntype: note\ntitle: B Notes\n---\n\n# Notes\n",
    );
    // `notes` matches both a/notes and b/notes.
    okq(dir.path())
        .args(["get", "notes"])
        .assert()
        .failure()
        .code(4)
        .stderr(predicates::str::contains("a/notes"));
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

/// A file with no frontmatter is a valid concept; `get` reports its filename as
/// the `title`, but the `frontmatter` object stays the file's true (empty) one —
/// the inferred title never leaks into the frontmatter surface (issue #6).
#[test]
fn no_frontmatter_infers_title_from_filename() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path().join("plain-note.md"),
        "# A Heading\n\nBody text.\n",
    );

    let out = stdout(
        okq(dir.path())
            .args(["get", "plain-note", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["title"], "plain-note", "title inferred from filename");
    assert_eq!(
        v["frontmatter"],
        serde_json::json!({}),
        "frontmatter untouched"
    );

    // The human `--frontmatter` view shows the real (empty) block, not the title.
    let human = stdout(
        okq(dir.path())
            .args(["get", "plain-note", "--frontmatter"])
            .assert()
            .success(),
    );
    assert!(human.contains("{}"), "empty frontmatter block");
    assert!(
        !human.contains("plain-note\n"),
        "no inferred title in frontmatter"
    );
}

/// An explicit frontmatter `title` still wins over the filename.
#[test]
fn explicit_title_wins_over_filename() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path().join("slug.md"),
        "---\ntype: note\ntitle: Real Title\n---\n\n# Body\n",
    );
    let out = stdout(
        okq(dir.path())
            .args(["get", "slug", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["title"], "Real Title");
}
