//! Integration tests for `okq skills` (install / list). Covers the embedded,
//! project-local path; `--from-repo` (network) and skills.sh are not exercised.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

fn okq_in(dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("okq").unwrap();
    cmd.current_dir(dir);
    cmd
}

fn stdout(assert: assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

#[test]
fn list_reports_the_embedded_skills() {
    let dir = TempDir::new().unwrap();
    let out = stdout(
        okq_in(dir.path())
            .args(["skills", "list", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["count"], 4);
    let names: Vec<&str> = v["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"okq-explore"));
    assert!(names.contains(&"okq-reference"));
    // Descriptions come along for the listing.
    assert!(v["skills"][0]["description"].as_str().unwrap().len() > 10);
}

#[test]
fn install_writes_agents_copy_and_links_into_claude() {
    let dir = TempDir::new().unwrap();
    okq_in(dir.path())
        .args(["skills", "install"])
        .assert()
        .success();

    for name in [
        "okq-explore",
        "okq-write-okf",
        "okq-maintain",
        "okq-reference",
    ] {
        // Canonical copy exists with real content.
        let canonical = dir
            .path()
            .join(".agents/skills")
            .join(name)
            .join("SKILL.md");
        assert!(canonical.exists(), "missing canonical {name}");
        assert!(
            fs::read_to_string(&canonical).unwrap().contains("---"),
            "{name} has no frontmatter"
        );

        // Linked into .claude/skills and the link resolves to the canonical file.
        let link = dir.path().join(".claude/skills").join(name);
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink(),
            "{name} is not a symlink"
        );
        assert!(
            link.join("SKILL.md").exists(),
            "{name} link does not resolve"
        );
    }
}

#[test]
fn install_is_idempotent_and_reports_updated() {
    let dir = TempDir::new().unwrap();
    okq_in(dir.path())
        .args(["skills", "install"])
        .assert()
        .success();

    // Second run updates in place without error.
    let out = stdout(
        okq_in(dir.path())
            .args(["skills", "install", "--json"])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["scope"], "project");
    assert_eq!(v["source"], "embedded");
    assert!(
        v["skills"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["verb"] == "updated")
    );
}

#[test]
fn install_does_not_clobber_a_real_directory() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join(".claude/skills/okq-explore");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("SKILL.md"), "MINE").unwrap();

    okq_in(dir.path())
        .args(["skills", "install"])
        .assert()
        .success();

    // The user's real directory is left untouched (not replaced by a symlink).
    assert!(
        !fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "MINE");
}

#[test]
fn conflicting_source_flags_are_a_usage_error() {
    let dir = TempDir::new().unwrap();
    okq_in(dir.path())
        .args(["skills", "install", "--from-repo", "--via-skills-sh"])
        .assert()
        .code(2);
}
