//! Integration tests for `okq stats` against a fixture with known metrics.
//!
//! Concepts: a, b, c, orphan.  Edges: a--related-->b, a--link-->c (2).
//! c--link-->nope is dead. Inbound: b<-a, c<-a; a and orphan have none.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("a.md"),
        "---\ntype: adr\ntitle: A\ntags: [x, y]\nrelated: [b]\n---\n\n# A\n\nSee [C](c.md).\n",
    );
    write(
        root.join("b.md"),
        "---\ntype: adr\ntitle: B\ntags: [x]\n---\n\n# B\n",
    );
    write(
        root.join("c.md"),
        "---\ntype: guide\ntitle: C\n---\n\n# C\n\nBroken [missing](nope.md).\n",
    );
    write(
        root.join("orphan.md"),
        "---\ntype: note\ntitle: Orphan\n---\n\n# Orphan\n",
    );
    dir
}

fn write(path: impl AsRef<Path>, contents: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn stats_json(bundle: &Path) -> serde_json::Value {
    let out = Command::cargo_bin("okq")
        .unwrap()
        .args(["--bundle"])
        .arg(bundle)
        .args(["stats", "--json"])
        .assert()
        .success();
    serde_json::from_str(&String::from_utf8(out.get_output().stdout.clone()).unwrap()).unwrap()
}

#[test]
fn totals_and_density() {
    let v = stats_json(fixture().path());
    assert_eq!(v["schema"], "okq.stats/v1");
    assert_eq!(v["concepts"], 4);
    assert_eq!(v["edges"], 2);
    assert_eq!(v["link_density"], 0.5);
    assert_eq!(v["dead_links"], 1);
    assert_eq!(v["parse_errors"], 0);
}

#[test]
fn orphans_count_matches() {
    // a (no inbound) and orphan have no backlinks.
    let v = stats_json(fixture().path());
    assert_eq!(v["orphans"], 2);
}

#[test]
fn distributions() {
    let v = stats_json(fixture().path());
    assert_eq!(v["types"]["adr"], 2);
    assert_eq!(v["types"]["guide"], 1);
    assert_eq!(v["types"]["note"], 1);
    assert_eq!(v["tags"]["x"], 2);
    assert_eq!(v["tags"]["y"], 1);
    assert_eq!(v["edge_types"]["link"], 1);
    assert_eq!(v["edge_types"]["related"], 1);
}

#[test]
fn hubs_are_linked_to_only() {
    let v = stats_json(fixture().path());
    let hubs: Vec<&str> = v["hubs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["id"].as_str().unwrap())
        .collect();
    // Only b and c have inbound edges; a and orphan never appear.
    assert!(hubs.contains(&"b"));
    assert!(hubs.contains(&"c"));
    assert!(!hubs.contains(&"a"));
    assert!(!hubs.contains(&"orphan"));
    assert_eq!(v["hubs"][0]["in_degree"], 1);
}

#[test]
fn untyped_concepts_bucket() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path().join("plain.md"), "# Plain\n\nNo frontmatter.\n");
    let v = stats_json(dir.path());
    assert_eq!(v["types"]["(untyped)"], 1);
}

#[test]
fn stats_exits_zero_even_when_empty() {
    let dir = tempfile::tempdir().unwrap();
    let v = stats_json(dir.path());
    assert_eq!(v["concepts"], 0);
    assert_eq!(v["link_density"], 0.0);
}

#[test]
fn top_caps_hubs() {
    let dir = fixture();
    let out = Command::cargo_bin("okq")
        .unwrap()
        .args(["--bundle"])
        .arg(dir.path())
        .args(["stats", "--json", "--top", "1"])
        .assert()
        .success();
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.get_output().stdout.clone()).unwrap()).unwrap();
    assert_eq!(v["hubs"].as_array().unwrap().len(), 1);
}
