//! Integration tests for trust & lifecycle: the envelope fields, `find`'s
//! filters, `get`'s evidence block, and `stats`' distributions.
//! See `docs/features/trust.md`.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

/// A bundle covering each trust shape: nothing, machine-confirmed, human
/// reviewed, deprecated, and stale.
fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        root.join("plain.md"),
        "---\ntype: doc\ntitle: Plain\n---\n\n# Plain\n\nNo trust frontmatter at all.\n",
    );
    write(
        root.join("machine.md"),
        "---\ntype: doc\ntitle: Machine\ngenerated: { by: 'writer/1.0', at: 2026-05-01 }\n\
         verified:\n  - { by: 'process:nightly', at: 2026-06-01 }\n---\n\n# Machine\n",
    );
    write(
        root.join("human.md"),
        "---\ntype: doc\ntitle: Human\nstatus: draft\n\
         generated: { by: 'writer/1.0', at: 2026-05-01 }\n\
         verified: { by: 'human:mike', at: 2026-07-14 }\n---\n\n# Human\n",
    );
    write(
        root.join("old.md"),
        "---\ntype: doc\ntitle: Old\nstatus: deprecated\nstale_after: 2026-01-01\n---\n\n# Old\n",
    );
    write(
        root.join("odd.md"),
        "---\ntype: doc\ntitle: Odd\nstatus: percolating\n---\n\n# Odd\n",
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

fn record<'a>(v: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    v["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .unwrap_or_else(|| panic!("no record for {id:?}"))
}

#[test]
fn a_concept_without_trust_frontmatter_carries_no_trust_keys() {
    // The omit-at-default rule (ADR-0014): the record must be byte-identical to
    // what okq emitted before trust existed.
    // Asserted on the raw bytes, not a parsed value: serde_json sorts keys on
    // the way in, which would hide a change in what is actually emitted.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path().join("plain.md"),
        "---\ntype: doc\ntitle: Plain\n---\n\n# Plain\n",
    );
    let out = okq(dir.path()).args(["find", "--json"]).assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains(
            "\"id\": \"plain\",\n      \"type\": \"doc\",\n      \"title\": \"Plain\",\n      \
             \"path\": \"plain.md\",\n      \"line\": 1,\n      \"tags\": []\n"
        ),
        "trust keys leaked into a trust-free record:\n{stdout}"
    );
}

#[test]
fn tiers_are_derived_from_the_verified_actors() {
    let dir = fixture();
    let out = json(dir.path(), &["find", "--type", "doc"]);
    assert_eq!(record(&out, "machine")["trust"], "machine-confirmed");
    assert_eq!(record(&out, "human")["trust"], "human-reviewed");
    assert!(record(&out, "plain").get("trust").is_none());
}

#[test]
fn status_is_reported_including_producer_defined_values() {
    let dir = fixture();
    let out = json(dir.path(), &["find", "--type", "doc"]);
    assert_eq!(record(&out, "human")["status"], "draft");
    assert_eq!(record(&out, "old")["status"], "deprecated");
    // §5.4 requires consumers to tolerate an unknown value, not coerce it.
    assert_eq!(record(&out, "odd")["status"], "percolating");
}

#[test]
fn find_filters_by_status_and_trust() {
    let dir = fixture();
    assert_eq!(
        ids(&json(dir.path(), &["find", "--status", "draft"])),
        ["human"]
    );
    assert_eq!(
        ids(&json(dir.path(), &["find", "--trust", "human-reviewed"])),
        ["human"]
    );
    // Repeatable and OR within a flag.
    let mut both = ids(&json(
        dir.path(),
        &[
            "find",
            "--trust",
            "human-reviewed",
            "--trust",
            "machine-confirmed",
        ],
    ));
    both.sort();
    assert_eq!(both, ["human", "machine"]);
    // Case-insensitive, and a producer-defined status is filterable.
    assert_eq!(
        ids(&json(dir.path(), &["find", "--status", "PERCOLATING"])),
        ["odd"]
    );
}

#[test]
fn status_and_trust_filters_and_together() {
    let dir = fixture();
    // draft AND human-reviewed matches; draft AND machine-confirmed does not.
    assert_eq!(
        ids(&json(
            dir.path(),
            &["find", "--status", "draft", "--trust", "human-reviewed"]
        )),
        ["human"]
    );
    let empty = json(
        dir.path(),
        &["find", "--status", "draft", "--trust", "machine-confirmed"],
    );
    assert_eq!(empty["count"], 0);
}

#[test]
fn an_unknown_trust_tier_is_a_usage_error() {
    let dir = fixture();
    okq(dir.path())
        .args(["find", "--trust", "sort-of-verified"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn staleness_is_reproducible_against_today() {
    let dir = fixture();
    // `old.md` has stale_after: 2026-01-01.
    assert_eq!(
        ids(&json(
            dir.path(),
            &["find", "--stale", "--today", "2026-08-02"]
        )),
        ["old"]
    );
    // Before that date, nothing is stale.
    assert_eq!(
        json(dir.path(), &["find", "--stale", "--today", "2025-12-31"])["count"],
        0
    );
    assert_eq!(
        record(
            &json(
                dir.path(),
                &["find", "--type", "doc", "--today", "2026-08-02"]
            ),
            "old"
        )["stale"],
        true
    );
}

#[test]
fn a_malformed_today_is_a_usage_error() {
    let dir = fixture();
    okq(dir.path())
        .args(["find", "--stale", "--today", "last tuesday"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn an_empty_trust_filter_result_exits_zero() {
    // Empty is not an error, per the shared taxonomy.
    let dir = tempfile::tempdir().unwrap();
    write(dir.path().join("a.md"), "---\ntype: doc\n---\n\n# A\n");
    okq(dir.path())
        .args(["find", "--trust", "human-reviewed"])
        .assert()
        .success();
}

#[test]
fn get_reports_the_events_the_tier_was_derived_from() {
    let dir = fixture();
    let out = json(dir.path(), &["get", "human"]);
    assert_eq!(out["trust"], "human-reviewed");
    assert_eq!(out["status"], "draft");
    assert_eq!(out["provenance"]["generated"]["by"], "writer/1.0");
    assert_eq!(out["provenance"]["verified"][0]["by"], "human:mike");
    assert_eq!(out["provenance"]["verified"][0]["at"], "2026-07-14");

    // A concept with no events carries no provenance block at all.
    let plain = json(dir.path(), &["get", "plain"]);
    assert!(plain.get("provenance").is_none());
    assert!(plain.get("trust").is_none());
}

#[test]
fn get_human_output_shows_labels_and_events() {
    let dir = fixture();
    okq(dir.path())
        .args(["get", "human", "--no-color"])
        .assert()
        .success()
        .stdout(predicates::str::contains("[draft, human-reviewed]"))
        .stdout(predicates::str::contains("verified   human:mike"));
}

#[test]
fn field_output_stays_pipe_safe() {
    // `--field` prints the value alone — no header, and no trust block either
    // (the 0.6.1 fix, extended).
    let dir = fixture();
    let out = okq(dir.path())
        .args(["get", "human", "--field", "title", "--no-color"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout, "Human\n");
}

#[test]
fn stats_counts_every_concept_in_both_distributions() {
    let dir = fixture();
    let out = json(dir.path(), &["stats"]);
    let total = out["concepts"].as_u64().unwrap();

    let sum = |v: &serde_json::Value| -> u64 {
        v.as_object()
            .unwrap()
            .values()
            .map(|n| n.as_u64().unwrap())
            .sum()
    };
    assert_eq!(sum(&out["trust"]), total);
    assert_eq!(sum(&out["statuses"]), total);

    assert_eq!(out["trust"]["human-reviewed"], 1);
    assert_eq!(out["trust"]["machine-confirmed"], 1);
    assert_eq!(out["trust"]["unverified"], 3);
    assert_eq!(out["statuses"]["stable"], 2);
    assert_eq!(out["statuses"]["percolating"], 1);
}

#[test]
fn malformed_trust_frontmatter_degrades_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path().join("junk.md"),
        "---\ntype: doc\ntitle: Junk\nverified: yesterday\ngenerated: sometime\n\
         stale_after: whenever\n---\n\n# Junk\n",
    );
    let out = json(dir.path(), &["find"]);
    let junk = record(&out, "junk");
    assert!(junk.get("trust").is_none());
    assert!(junk.get("stale").is_none());
    okq(dir.path()).args(["get", "junk"]).assert().success();
}

#[test]
fn index_listings_never_carry_date_dependent_output() {
    // index.md is written to disk, so nothing clock-dependent may leak in.
    let dir = fixture();
    okq(dir.path()).arg("index").assert().success();
    let index = fs::read_to_string(dir.path().join("index.md")).unwrap();
    assert!(
        !index.contains("stale"),
        "index.md leaked a trust value:\n{index}"
    );
}
