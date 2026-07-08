//! Robustness tests: okq must degrade gracefully on malformed / edge-case docs.
//!
//! These run against the real fixture bundle in `docs/tests/` (see its README).
//! The contract: a bad document is *skipped* (okf collects it into
//! `parse_errors`), never a panic and never a failure of the whole bundle; the
//! good documents alongside it stay queryable.

use std::path::{Path, PathBuf};

use assert_cmd::Command;

/// Absolute path to a subdirectory of the crate, so tests don't depend on cwd.
fn bundle(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(sub)
}

fn okq(sub: &str) -> Command {
    let mut cmd = Command::cargo_bin("okq").unwrap();
    cmd.arg("--bundle").arg(bundle(sub));
    cmd
}

fn ids(json: &str) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    v["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn malformed_bundle_loads_and_skips_bad_docs() {
    let out = okq("docs/tests")
        .args(["find", "--json"])
        .assert()
        .success();
    let json = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let ids = ids(&json);

    // Good / edge-but-valid docs are present — including a concept whose file
    // name begins with an emoji (the widened concept-id rule, ADR-0010).
    for good in [
        "only-frontmatter",
        "no-frontmatter",
        "empty",
        "unicode-emoji",
        "🚀 launch",
    ] {
        assert!(
            ids.iter().any(|id| id == good),
            "expected {good:?} in {ids:?}"
        );
    }
    // Malformed docs are silently skipped, not surfaced as concepts.
    for bad in [
        "unterminated-frontmatter",
        "invalid-yaml-flow",
        "tab-indentation",
        "frontmatter-is-list",
        "frontmatter-is-scalar",
    ] {
        assert!(!ids.iter().any(|id| id == bad), "{bad:?} should be skipped");
    }
}

#[test]
fn whole_docs_tree_loads_despite_malformed_subdir() {
    // The real docs/ bundle contains docs/tests/* junk; it must still load.
    okq("docs")
        .args(["find", "--where", "status=accepted"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "0001-documentation-first-okf-shaped",
        ));
}

#[test]
fn get_on_parse_error_doc_is_graceful_not_found() {
    okq("docs/tests")
        .args(["get", "unterminated-frontmatter"])
        .assert()
        .failure()
        .code(4);
}

#[test]
fn get_on_invalid_concept_id_is_graceful() {
    // A reserved character makes the id unparseable under the widened rule
    // (ADR-0010); `get` fails cleanly (not-found) instead of panicking.
    okq("docs/tests")
        .args(["get", "bad:name"])
        .assert()
        .failure()
        .code(4);
}

#[test]
fn duplicate_headings_section_is_ambiguous() {
    okq("docs/tests")
        .args(["get", "duplicate-headings", "--section", "Notes"])
        .assert()
        .failure()
        .code(5);
}

#[test]
fn unicode_section_resolves_without_panic() {
    okq("docs/tests")
        .args(["get", "unicode-emoji", "--section", "Sección en español"])
        .assert()
        .success()
        .stdout(predicates::str::contains("eñe"));
}

#[test]
fn empty_doc_is_handled() {
    okq("docs/tests").args(["get", "empty"]).assert().success();
}

#[test]
fn no_frontmatter_doc_is_queryable() {
    okq("docs/tests")
        .args(["get", "no-frontmatter", "--body"])
        .assert()
        .success()
        .stdout(predicates::str::contains("OKF-shaped doc"));
}

#[test]
fn no_frontmatter_doc_titles_from_filename() {
    // A file with no frontmatter reports its filename as the `title`, verbatim.
    okq("docs/tests")
        .args(["get", "no-frontmatter", "--json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"title\": \"no-frontmatter\""));
}

#[test]
fn scalar_tags_do_not_break_tag_filter() {
    // tags-not-a-list has a scalar `tags`; it must not crash --tag, and must
    // not match (its tags read as empty). only-frontmatter (tags: [edge]) does.
    let out = okq("docs/tests")
        .args(["find", "--tag", "edge", "--json"])
        .assert()
        .success();
    let ids = ids(&String::from_utf8(out.get_output().stdout.clone()).unwrap());
    assert!(ids.iter().any(|id| id == "only-frontmatter"));
    assert!(!ids.iter().any(|id| id == "tags-not-a-list"));
}

#[test]
fn edge_case_aliases_and_tags_stay_graceful() {
    // The aliases-tags-edge-cases fixture holds a valid alias, an empty alias,
    // real inline tags, and several tag-shaped non-tags. Everything must degrade
    // gracefully and the real signal must be queryable.

    // A declared alias resolves to the concept (empty alias entry is ignored).
    okq("docs/tests")
        .args(["get", "Edge Alias"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Edge-case aliases and inline tags",
        ));

    // Real inline tags are findable; tag-shaped non-tags are not.
    for real in ["fixture-tag", "area/robustness"] {
        let out = okq("docs/tests")
            .args(["find", "--tag", real, "--json"])
            .assert()
            .success();
        let ids = ids(&String::from_utf8(out.get_output().stdout.clone()).unwrap());
        assert!(
            ids.iter().any(|id| id == "aliases-tags-edge-cases"),
            "tag {real:?} should match the fixture"
        );
    }
    for nontag in ["123", "section", "bar"] {
        let out = okq("docs/tests")
            .args(["find", "--tag", nontag, "--json"])
            .assert()
            .success();
        let ids = ids(&String::from_utf8(out.get_output().stdout.clone()).unwrap());
        assert!(
            !ids.iter().any(|id| id == "aliases-tags-edge-cases"),
            "{nontag:?} is not a tag and must not match"
        );
    }
}

#[test]
fn malformed_wikilinks_do_not_break_the_graph() {
    // The wikilinks-malformed fixture holds unterminated / empty / nested /
    // code-fenced `[[…]]`. The graph commands must still run cleanly (exit 0).
    let out = okq("docs/tests")
        .args(["deadlinks", "--json"])
        .assert()
        .success();
    let json = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    // It parses as JSON (no panic, no torn output) and code-fenced / inline-code
    // wikilinks never surface as dead links.
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let raws: Vec<&str> = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["raw"].as_str())
        .collect();
    assert!(!raws.iter().any(|r| r.contains("NotAScannedLink")));
    assert!(!raws.iter().any(|r| r.contains("AlsoIgnoredInFence")));

    // neighbors on the fixture also stays graceful.
    okq("docs/tests")
        .args(["neighbors", "wikilinks-malformed"])
        .assert()
        .success();
}

#[test]
fn headings_inside_code_fence_are_not_sections() {
    // A real heading resolves...
    okq("docs/tests")
        .args([
            "get",
            "headings-in-code-fence",
            "--section",
            "Real Subheading",
        ])
        .assert()
        .success();
    // ...but a "#" line inside a code fence is not a section.
    okq("docs/tests")
        .args(["get", "headings-in-code-fence", "--section", "also fake"])
        .assert()
        .failure()
        .code(5);
}
