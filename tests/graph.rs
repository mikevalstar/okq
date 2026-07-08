//! Integration tests for the M2 graph commands against a fixture with a known
//! link structure (both inline links and frontmatter relations).
//!
//! Edges in the fixture:
//!   a --related--> b      (frontmatter)
//!   a --link-----> c      (inline)
//!   c --supersedes-> a    (frontmatter)
//!   c --link-----> nope   (dead, inline)
//!   orphan: nothing points at it

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        root.join("a.md"),
        "---\ntype: doc\ntitle: A\nrelated: [b]\n---\n\n# A\n\nLinks to [C](c.md) inline.\n",
    );
    write(
        root.join("b.md"),
        "---\ntype: doc\ntitle: B\n---\n\n# B\n\nLeaf.\n",
    );
    write(
        root.join("c.md"),
        "---\ntype: doc\ntitle: C\nsupersedes: [a]\n---\n\n# C\n\nBroken [link](nope.md).\n",
    );
    write(
        root.join("orphan.md"),
        "---\ntype: doc\ntitle: Orphan\n---\n\n# Orphan\n\nNothing links here.\n",
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

fn json(bundle: &Path, args: &[&str]) -> serde_json::Value {
    let mut full = args.to_vec();
    full.push("--json");
    let out = okq(bundle).args(&full).assert().success();
    serde_json::from_str(&String::from_utf8(out.get_output().stdout.clone()).unwrap()).unwrap()
}

fn ids(v: &serde_json::Value) -> Vec<String> {
    v["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn neighbors_both_directions() {
    let dir = fixture();
    let v = json(dir.path(), &["neighbors", "a"]);
    assert_eq!(v["schema"], "okq.neighbors/v1");
    assert_eq!(ids(&v), vec!["b", "c"]); // sorted by (distance, id)
}

#[test]
fn neighbors_direction_in() {
    let dir = fixture();
    let v = json(dir.path(), &["neighbors", "a", "--direction", "in"]);
    // Only c supersedes a.
    assert_eq!(ids(&v), vec!["c"]);
    assert_eq!(v["results"][0]["edge"], "supersedes");
    assert_eq!(v["results"][0]["direction"], "in");
}

#[test]
fn neighbors_edge_filter() {
    let dir = fixture();
    let v = json(dir.path(), &["neighbors", "a", "--edge", "related"]);
    assert_eq!(ids(&v), vec!["b"]);
}

#[test]
fn neighbors_depth_two_keeps_first_hop_edge() {
    let dir = fixture();
    // From c: c->a (supersedes, d1), a->b (d2). b's first-hop edge is supersedes.
    let v = json(dir.path(), &["neighbors", "c", "--depth", "2"]);
    let b = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "b")
        .unwrap();
    assert_eq!(b["distance"], 2);
    assert_eq!(b["edge"], "supersedes");
}

#[test]
fn backlinks_inbound_only() {
    let dir = fixture();
    let v = json(dir.path(), &["backlinks", "a"]);
    assert_eq!(v["schema"], "okq.backlinks/v1");
    assert_eq!(ids(&v), vec!["c"]);
}

#[test]
fn path_directed() {
    let dir = fixture();
    let v = json(dir.path(), &["path", "c", "b"]);
    assert_eq!(v["found"], true);
    assert_eq!(v["length"], 2); // c -> a -> b
    let nodes: Vec<&str> = v["path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(nodes, vec!["c", "a", "b"]);
}

#[test]
fn path_respects_direction() {
    let dir = fixture();
    // b has no outbound edges → no directed path to a.
    let v = json(dir.path(), &["path", "b", "a"]);
    assert_eq!(v["found"], false);
    // ...but undirected reaches it.
    let v2 = json(dir.path(), &["path", "b", "a", "--undirected"]);
    assert_eq!(v2["found"], true);
}

#[test]
fn path_missing_endpoint_exits_4() {
    let dir = fixture();
    okq(dir.path())
        .args(["path", "a", "nope"])
        .assert()
        .failure()
        .code(4);
}

#[test]
fn orphans_lists_unreferenced() {
    let dir = fixture();
    let v = json(dir.path(), &["orphans"]);
    assert_eq!(v["schema"], "okq.orphans/v1");
    assert_eq!(ids(&v), vec!["orphan"]);
}

#[test]
fn orphans_check_exits_3_when_found() {
    let dir = fixture();
    okq(dir.path())
        .args(["orphans", "--check"])
        .assert()
        .failure()
        .code(3);
}

#[test]
fn deadlinks_reports_broken_target() {
    let dir = fixture();
    let v = json(dir.path(), &["deadlinks"]);
    assert_eq!(v["schema"], "okq.deadlinks/v1");
    assert_eq!(v["count"], 1);
    assert_eq!(v["results"][0]["source_id"], "c");
    assert_eq!(v["results"][0]["raw"], "nope.md");
    assert_eq!(v["results"][0]["edge"], "link");
}

#[test]
fn deadlinks_check_exits_3() {
    let dir = fixture();
    okq(dir.path())
        .args(["deadlinks", "--check"])
        .assert()
        .failure()
        .code(3);
}

#[test]
fn encoded_links_to_an_emoji_concept_resolve_and_broken_ones_are_dead() {
    // A concept whose file name contains an emoji, plus a linker that references
    // it two ways: a working percent-encoded link and a typo'd one. The working
    // link is a real edge; the broken one is a dead link even though it is
    // percent-encoded (ADR-0010 / emoji-filenames — the graph decodes the target).
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("🚀 launch.md"),
        "---\ntype: plan\ntitle: Launch\n---\n\n# Launch\n",
    );
    write(
        root.join("overview.md"),
        "---\ntype: doc\ntitle: Overview\n---\n\n# Overview\n\n\
         Works: [ok](%F0%9F%9A%80%20launch.md).\n\
         Typo: [bad](%F0%9F%9A%80%20launhc.md).\n",
    );

    // The working encoded link is a real outbound edge to the emoji concept.
    let n = json(root, &["neighbors", "overview"]);
    assert_eq!(ids(&n), vec!["🚀 launch".to_string()]);
    assert_eq!(n["results"][0]["edge"], "link");

    // The broken encoded link is reported as a dead link, raw as written.
    let d = json(root, &["deadlinks"]);
    assert_eq!(d["count"], 1);
    assert_eq!(d["results"][0]["source_id"], "overview");
    assert_eq!(d["results"][0]["raw"], "%F0%9F%9A%80%20launhc.md");
    assert_eq!(d["results"][0]["edge"], "link");
}

#[test]
fn neighbors_missing_concept_exits_4() {
    let dir = fixture();
    okq(dir.path())
        .args(["neighbors", "ghost"])
        .assert()
        .failure()
        .code(4);
}

#[test]
fn graph_commands_accept_partial_names() {
    // A nested concept resolved by its bare name flows through the shared
    // resolver into the graph commands.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path().join("adrs/one.md"),
        "---\ntype: adr\ntitle: One\nrelated: [two]\n---\n\n# One\n",
    );
    write(
        dir.path().join("adrs/two.md"),
        "---\ntype: adr\ntitle: Two\n---\n\n# Two\n",
    );
    // `one` resolves to `adrs/one`; its related edge reaches `adrs/two`.
    let v = json(dir.path(), &["neighbors", "one"]);
    assert_eq!(ids(&v), vec!["adrs/two"]);
}

/// A fixture whose only cross-links are Obsidian-style wikilinks, exercising the
/// shapes okq resolves: bare name, alias, `#heading`, path, embed, and a dead
/// bare-name link.
fn wiki_fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        root.join("hub.md"),
        "---\ntype: doc\ntitle: Hub\n---\n\n# Hub\n\n\
         Bare [[Leaf]], aliased [[Leaf|the leaf]], heading [[Leaf#Intro]],\n\
         path [[notes/Deep]], embed ![[Leaf]], a phantom [[Missing]],\n\
         and a broken path [[notes/Gone]].\n",
    );
    write(
        root.join("Leaf.md"),
        "---\ntype: doc\ntitle: Leaf\n---\n\n# Leaf\n\n## Intro\n\nLeaf body.\n",
    );
    write(
        root.join("notes/Deep.md"),
        "---\ntype: doc\ntitle: Deep\n---\n\n# Deep\n\nNested note.\n",
    );

    dir
}

#[test]
fn wikilinks_become_edges() {
    let dir = wiki_fixture();
    // hub reaches Leaf (bare/alias/heading/embed all collapse to one edge) and
    // notes/Deep (path), deduped — the source is excluded.
    let v = json(dir.path(), &["neighbors", "hub", "--direction", "out"]);
    assert_eq!(ids(&v), vec!["Leaf", "notes/Deep"]);
    assert!(
        v["results"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["edge"] == "wikilink")
    );
}

#[test]
fn wikilinks_edge_filter_and_backlinks() {
    let dir = wiki_fixture();
    // Filtering to the wikilink kind keeps them; a made-up kind drops them.
    let v = json(dir.path(), &["neighbors", "hub", "--edge", "wikilink"]);
    assert_eq!(ids(&v), vec!["Leaf", "notes/Deep"]);
    // Leaf's inbound view sees hub via the wikilink.
    let b = json(dir.path(), &["backlinks", "Leaf"]);
    assert_eq!(ids(&b), vec!["hub"]);
    assert_eq!(b["results"][0]["edge"], "wikilink");
}

#[test]
fn dead_wikilink_reported() {
    let dir = wiki_fixture();

    // Default: broken only. `[[notes/Gone]]` is a path that forms a valid id but
    // matches no file → broken. The bare `[[Missing]]` is a phantom, hidden here.
    let v = json(dir.path(), &["deadlinks"]);
    assert_eq!(v["count"], 1);
    assert_eq!(v["results"][0]["source_id"], "hub");
    assert_eq!(v["results"][0]["raw"], "notes/Gone");
    assert_eq!(v["results"][0]["edge"], "wikilink");
    assert_eq!(v["results"][0]["kind"], "broken");

    // `--phantoms` also lists the bare-name phantom.
    let all = json(dir.path(), &["deadlinks", "--phantoms"]);
    assert_eq!(all["count"], 2);
    let phantom = all["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["raw"] == "Missing")
        .expect("phantom listed with --phantoms");
    assert_eq!(phantom["kind"], "phantom");
    assert_eq!(phantom["source_id"], "hub");

    // `--phantoms-only` lists just the phantom, not the broken link.
    let only = json(dir.path(), &["deadlinks", "--phantoms-only"]);
    assert_eq!(only["count"], 1);
    assert_eq!(only["results"][0]["raw"], "Missing");
    assert_eq!(only["results"][0]["kind"], "phantom");
}

#[test]
fn wikilink_resolves_case_insensitively() {
    // Lenient bare-name matching: `[[leaf]]` finds `Leaf`.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path().join("a.md"),
        "---\ntype: doc\ntitle: A\n---\n\n# A\n\nlink [[leaf]].\n",
    );
    write(
        dir.path().join("Leaf.md"),
        "---\ntype: doc\ntitle: Leaf\n---\n\n# Leaf\n",
    );
    let v = json(dir.path(), &["neighbors", "a", "--direction", "out"]);
    assert_eq!(ids(&v), vec!["Leaf"]);
}

#[test]
fn out_of_bundle_links_are_not_dead() {
    // A link escaping the bundle root is out of scope, not dead.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path().join("x.md"),
        "---\ntype: doc\ntitle: X\n---\n\n# X\n\nSee [plan](../../PLAN.md) and [ext](https://example.com).\n",
    );
    let v = json(dir.path(), &["deadlinks"]);
    assert_eq!(v["count"], 0);
}

/// A bundle where `people/Hooman Bahador.md` declares `aliases: [Hooman, HB]`
/// and another note links it three ways: by alias, by real name, and (for the
/// precedence test) a `Report.md` whose alias collides with a real `Report.md`.
fn alias_fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("people/Hooman Bahador.md"),
        "---\ntype: contact\ntitle: Hooman Bahador\naliases: [Hooman, HB]\n---\n\n# Hooman Bahador\n",
    );
    write(
        root.join("daily.md"),
        "---\ntype: doc\ntitle: Daily\n---\n\n# Daily\n\nMet [[Hooman]] and [[HB]] today.\n",
    );
    dir
}

#[test]
fn get_resolves_by_alias() {
    let dir = alias_fixture();
    // `get` accepts an alias and returns the real concept (list-scalar shapes,
    // case-insensitive).
    for needle in ["Hooman", "hooman", "HB"] {
        okq(dir.path())
            .args(["get", needle])
            .assert()
            .success()
            .stdout(predicates::str::contains("Hooman Bahador"));
    }
}

#[test]
fn wikilink_to_alias_forms_edge_not_phantom() {
    let dir = alias_fixture();
    // `[[Hooman]]`/`[[HB]]` resolve to the aliased note → a wikilink edge, and
    // nothing is reported as a dead/phantom link.
    let v = json(dir.path(), &["neighbors", "daily", "--direction", "out"]);
    assert_eq!(ids(&v), vec!["people/Hooman Bahador"]);
    assert_eq!(v["results"][0]["edge"], "wikilink");

    let d = json(dir.path(), &["deadlinks", "--phantoms"]);
    assert_eq!(
        d["count"], 0,
        "alias targets are neither broken nor phantom"
    );
}

#[test]
fn filename_beats_alias() {
    // A real `Report.md` must win over another note's `aliases: [Report]`.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path().join("Report.md"),
        "---\ntype: doc\ntitle: The Real Report\n---\n\n# The Real Report\n",
    );
    write(
        dir.path().join("decoy.md"),
        "---\ntype: doc\ntitle: Decoy\naliases: [Report]\n---\n\n# Decoy\n",
    );
    okq(dir.path())
        .args(["get", "Report"])
        .assert()
        .success()
        .stdout(predicates::str::contains("The Real Report"));
}

#[test]
fn ambiguous_alias_errors_with_candidates() {
    // Two notes claiming the same alias → an ambiguity error (exit 4,
    // not-found/ambiguous), not a silent pick.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path().join("one.md"),
        "---\ntype: doc\ntitle: One\naliases: [Dup]\n---\n\n# One\n",
    );
    write(
        dir.path().join("two.md"),
        "---\ntype: doc\ntitle: Two\naliases: [Dup]\n---\n\n# Two\n",
    );
    okq(dir.path())
        .args(["get", "Dup"])
        .assert()
        .failure()
        .code(4);
}
