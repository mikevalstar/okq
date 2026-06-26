//! Integration tests for `okq search`. Most use `--ephemeral` (in-memory,
//! deterministic, no disk writes / no shared-cache flakiness); one exercises
//! the persisted path via the OKQ_CACHE_DIR override.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        root.join("alpha.md"),
        "---\n\
         type: note\n\
         title: Alpha\n\
         tags: [search]\n\
         ---\n\
         \n\
         # Alpha\n\
         \n\
         Intro about retrieval.\n\
         \n\
         ## Ranking\n\
         \n\
         Documents are ranked by relevance using tantivy and a search index.\n",
    );
    write(
        root.join("beta.md"),
        "---\n\
         type: note\n\
         title: Beta\n\
         tags: [graph]\n\
         ---\n\
         \n\
         # Beta\n\
         \n\
         This concerns graph neighbors and traversal between concepts.\n",
    );
    write(
        root.join("gamma.md"),
        "---\n\
         type: note\n\
         title: Gamma\n\
         tags: [filter]\n\
         ---\n\
         \n\
         # Gamma\n\
         \n\
         Frontmatter tags and predicate filters live here.\n",
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

fn search_json(bundle: &Path, query: &str) -> serde_json::Value {
    let out = stdout(
        okq(bundle)
            .args(["search", query, "--ephemeral", "--json"])
            .assert()
            .success(),
    );
    serde_json::from_str(&out).unwrap()
}

#[test]
fn ranks_the_relevant_concept_first() {
    let dir = fixture();
    let v = search_json(dir.path(), "tantivy");
    assert_eq!(v["schema"], "okq.search/v1");
    assert_eq!(v["results"][0]["id"], "alpha");
}

#[test]
fn hit_locates_the_matching_section() {
    let dir = fixture();
    let v = search_json(dir.path(), "tantivy");
    // "tantivy" only appears under "## Ranking" (line 11 of alpha.md).
    assert_eq!(v["results"][0]["heading"], "Ranking");
    assert_eq!(v["results"][0]["line"], 11);
    assert_eq!(v["results"][0]["slug"], "ranking");
}

#[test]
fn different_query_ranks_different_concept() {
    let dir = fixture();
    let v = search_json(dir.path(), "graph neighbors");
    assert_eq!(v["results"][0]["id"], "beta");
}

#[test]
fn stemming_matches_inflected_forms() {
    let dir = fixture();
    // "rankings" stems to "rank"; alpha contains "ranked"/"Ranking".
    let v = search_json(dir.path(), "rankings");
    let ids: Vec<&str> = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"alpha"), "stemming should match: {ids:?}");
}

#[test]
fn phrase_query_works() {
    let dir = fixture();
    let v = search_json(dir.path(), "\"search index\"");
    assert_eq!(v["results"][0]["id"], "alpha");
}

#[test]
fn limit_bounds_results() {
    let dir = fixture();
    let out = stdout(
        okq(dir.path())
            .args([
                "search",
                "concepts",
                "--ephemeral",
                "--limit",
                "1",
                "--json",
            ])
            .assert()
            .success(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["count"].as_u64().unwrap() <= 1);
}

#[test]
fn no_hits_exits_zero() {
    let dir = fixture();
    let v = search_json(dir.path(), "zzzznotpresentzzzz");
    assert_eq!(v["count"], 0);
}

#[test]
fn empty_query_is_usage_error() {
    let dir = fixture();
    okq(dir.path())
        .args(["search", "   ", "--ephemeral"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn output_is_deterministic() {
    let dir = fixture();
    let once = stdout(
        okq(dir.path())
            .args(["search", "the", "--ephemeral", "--json"])
            .assert()
            .success(),
    );
    let twice = stdout(
        okq(dir.path())
            .args(["search", "the", "--ephemeral", "--json"])
            .assert()
            .success(),
    );
    assert_eq!(once, twice);
}

#[test]
fn no_full_bodies_in_output() {
    // Token-frugal: the snippet is short, never the whole section body.
    let dir = fixture();
    let v = search_json(dir.path(), "graph");
    let snippet = v["results"][0]["snippet"].as_str().unwrap();
    assert!(snippet.len() < 400);
}

#[test]
fn persisted_index_builds_and_reuses_via_cache_override() {
    let dir = fixture();
    let cache = tempfile::tempdir().unwrap();

    // First run builds the on-disk index in the override cache dir.
    okq(dir.path())
        .env("OKQ_CACHE_DIR", cache.path())
        .args(["search", "tantivy", "--json"])
        .assert()
        .success();

    // The cache dir now holds index files (a manifest + Tantivy meta).
    let mut found_manifest = false;
    for entry in walk(cache.path()) {
        if entry.file_name().is_some_and(|n| n == "manifest.json") {
            found_manifest = true;
        }
    }
    assert!(
        found_manifest,
        "expected a manifest.json under the cache dir"
    );

    // Second run reuses it (and --reindex forces a rebuild); both succeed.
    okq(dir.path())
        .env("OKQ_CACHE_DIR", cache.path())
        .args(["search", "tantivy", "--json"])
        .assert()
        .success();
    okq(dir.path())
        .env("OKQ_CACHE_DIR", cache.path())
        .args(["search", "tantivy", "--reindex", "--json"])
        .assert()
        .success();
}

/// Recursively lists files under a directory.
fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}
