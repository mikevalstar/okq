//! Tests for `okq schema`. The snapshot locks the output contract so an
//! accidental envelope-shape change fails CI (the agent contract is enforced).

use assert_cmd::Command;

fn okq() -> Command {
    Command::cargo_bin("okq").unwrap()
}

fn stdout(args: &[&str]) -> String {
    let out = okq().args(args).assert().success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

#[test]
fn one_command_schema_is_valid_json_describing_the_envelope() {
    let text = stdout(&["schema", "stats"]);
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    // A JSON Schema document with the stats envelope's fields.
    assert!(v.get("properties").is_some() || v.get("$defs").is_some());
    assert!(text.contains("concepts"));
    assert!(text.contains("link_density"));
    assert!(text.contains("edge_types"));
}

#[test]
fn all_schemas_cover_every_json_command() {
    let text = stdout(&["schema"]);
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    for cmd in [
        "get",
        "find",
        "search",
        "neighbors",
        "backlinks",
        "path",
        "orphans",
        "deadlinks",
        "stats",
    ] {
        assert!(v.get(cmd).is_some(), "schema missing for {cmd}");
    }
}

#[test]
fn neighbors_and_backlinks_share_a_schema() {
    assert_eq!(
        stdout(&["schema", "neighbors"]),
        stdout(&["schema", "backlinks"])
    );
}

#[test]
fn unknown_command_exits_2() {
    okq()
        .args(["schema", "bogus"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicates::str::contains("known:"));
}

#[test]
fn get_schema_is_stable() {
    insta::assert_snapshot!("schema_get", stdout(&["schema", "get"]));
}
