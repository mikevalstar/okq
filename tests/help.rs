//! Golden snapshots of the CLI help, so help is treated as a tested feature.
//! Run with NO_COLOR for stable, plain-text output.

use assert_cmd::Command;

fn help(args: &[&str]) -> String {
    let out = Command::cargo_bin("okq")
        .unwrap()
        .env("NO_COLOR", "1")
        .args(args)
        .assert()
        .success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

#[test]
fn top_level_help() {
    insta::assert_snapshot!("help_main", help(&["--help"]));
}

#[test]
fn get_help() {
    insta::assert_snapshot!("help_get", help(&["get", "--help"]));
}

#[test]
fn find_help() {
    insta::assert_snapshot!("help_find", help(&["find", "--help"]));
}

#[test]
fn search_help() {
    insta::assert_snapshot!("help_search", help(&["search", "--help"]));
}

#[test]
fn index_help() {
    insta::assert_snapshot!("help_index", help(&["index", "--help"]));
}

#[test]
fn validate_help() {
    insta::assert_snapshot!("help_validate", help(&["validate", "--help"]));
}

#[test]
fn skills_help() {
    insta::assert_snapshot!("help_skills", help(&["skills", "--help"]));
}

#[test]
fn skills_install_help() {
    insta::assert_snapshot!(
        "help_skills_install",
        help(&["skills", "install", "--help"])
    );
}

#[test]
fn examples_and_learn_more_present() {
    // The affordances that make help "good" — assert they survive refactors.
    let h = help(&["--help"]);
    assert!(h.contains("Examples:"));
    assert!(h.contains("Learn more:"));
    assert!(h.contains("okq search"));
    assert!(h.contains("--json"));
}
