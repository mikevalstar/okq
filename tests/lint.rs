//! Integration tests for `okq lint`. See `docs/features/lint.md`.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

/// A bundle that trips a spread of okf 0.2.7 lint rules: empty sections (L4),
/// orphans (L9), a self-linker (L10), concepts with no `verified` events (L11),
/// and a draft (L12). The frontmatter and link rules this fixture also trips
/// are `validate` warnings now, not lint findings (ADR-0016).
fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        root.join("hub.md"),
        "---\ntype: doc\ntitle: Hub\ndescription: The hub\n---\n\n# Hub\n\n\
         Links to [Old](old.md), [Bare](bare.md), [Draft](draft.md), [Self](self.md).\n",
    );
    write(
        root.join("old.md"),
        "---\ntype: doc\ntitle: Old\ndescription: Deprecated\nstatus: deprecated\n---\n\n# Old\n",
    );
    write(root.join("bare.md"), "---\ntype: doc\n---\n");
    write(
        root.join("draft.md"),
        "---\ntype: doc\ntitle: Draft\ndescription: In progress\nstatus: draft\n---\n\n# Draft\n",
    );
    write(
        root.join("self.md"),
        "---\ntype: doc\ntitle: Self\ndescription: Points at itself\n---\n\n\
         # Self\n\nSee [me](self.md).\n",
    );
    write(
        root.join("orphan.md"),
        "---\ntype: doc\ntitle: Orphan\ndescription: Nothing links here\n---\n\n# Orphan\n",
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

fn rules(v: &serde_json::Value) -> Vec<String> {
    v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["rule"].as_str().unwrap_or("-").to_string())
        .collect()
}

#[test]
fn findings_carry_a_structured_rule_code_and_a_stripped_message() {
    let dir = fixture();
    let out = json(dir.path(), &["lint"]);
    assert_eq!(out["schema"], "okq.lint/v1");

    let found = rules(&out);
    for expected in ["L4", "L9", "L10", "L11", "L12"] {
        assert!(
            found.iter().any(|r| r == expected),
            "missing {expected} in {found:?}"
        );
    }
    // The `[Lnn] ` prefix is lifted into `rule`, never left in the prose.
    for d in out["diagnostics"].as_array().unwrap() {
        let message = d["message"].as_str().unwrap();
        assert!(
            !message.starts_with('['),
            "prefix left in message: {message:?}"
        );
    }
}

#[test]
fn lint_never_reports_an_error_severity() {
    // Hygiene is an opinion; only `validate` can call something an error.
    let dir = fixture();
    let out = json(dir.path(), &["lint"]);
    for d in out["diagnostics"].as_array().unwrap() {
        assert_ne!(d["severity"], "error");
    }
    assert_eq!(
        out["findings"].as_u64().unwrap(),
        out["warnings"].as_u64().unwrap() + out["infos"].as_u64().unwrap()
    );
}

#[test]
fn lint_findings_do_not_change_conformance() {
    let dir = fixture();
    let lint = json(dir.path(), &["lint"]);
    assert!(lint["findings"].as_u64().unwrap() > 0);

    let validate = json(dir.path(), &["validate"]);
    assert_eq!(validate["conformant"], true);
    okq(dir.path())
        .args(["validate", "--check"])
        .assert()
        .success();
}

#[test]
fn rule_filters_narrow_the_report() {
    let dir = fixture();
    let only = json(dir.path(), &["lint", "--rule", "L9"]);
    assert!(only["findings"].as_u64().unwrap() > 0);
    assert!(rules(&only).iter().all(|r| r == "L9"));

    // Case-insensitive.
    let lower = json(dir.path(), &["lint", "--rule", "l9"]);
    assert_eq!(lower["findings"], only["findings"]);
}

#[test]
fn ignore_suppresses_a_rule() {
    let dir = fixture();
    let all = json(dir.path(), &["lint"]);
    let without = json(dir.path(), &["lint", "--ignore", "L12"]);
    assert!(rules(&all).iter().any(|r| r == "L12"));
    assert!(rules(&without).iter().all(|r| r != "L12"));
    assert!(without["findings"].as_u64().unwrap() < all["findings"].as_u64().unwrap());
}

#[test]
fn rule_and_ignore_together_is_a_usage_error() {
    let dir = fixture();
    okq(dir.path())
        .args(["lint", "--rule", "L1", "--ignore", "L2"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn an_unknown_rule_code_is_a_usage_error() {
    // A typo must not silently lint everything.
    let dir = fixture();
    for args in [["lint", "--ignore", "L99"], ["lint", "--rule", "nonsense"]] {
        okq(dir.path()).args(args).assert().failure().code(2);
    }
}

#[test]
fn severity_floor_drops_info_findings() {
    let dir = fixture();
    let warnings_only = json(dir.path(), &["lint", "--severity", "warning"]);
    assert_eq!(warnings_only["infos"], 0);
    for d in warnings_only["diagnostics"].as_array().unwrap() {
        assert_eq!(d["severity"], "warning");
    }
}

#[test]
fn check_exits_3_when_findings_survive_and_0_when_they_do_not() {
    let dir = fixture();
    okq(dir.path())
        .args(["lint", "--check"])
        .assert()
        .failure()
        .code(3);

    // A clean bundle: nothing for lint to say.
    let clean = tempfile::tempdir().unwrap();
    write(clean.path().join("index.md"), "# Index\n\n- [A](a.md)\n");
    write(
        clean.path().join("a.md"),
        "---\ntype: doc\ntitle: A\ndescription: All good\n\
         generated: { by: 'human:mike', at: 2026-01-01 }\n\
         verified: { by: 'human:mike', at: 2026-02-01 }\n---\n\n# A\n\nBody.\n",
    );
    okq(clean.path())
        .args(["lint", "--check"])
        .assert()
        .success();
}

#[test]
fn check_respects_the_filters() {
    // Gate CI on the subset a team agreed to enforce.
    let dir = fixture();
    okq(dir.path())
        .args(["lint", "--check", "--rule", "L10"])
        .assert()
        .failure()
        .code(3);
    // L13 (unquoted `okf_version`) needs a root index.md, which this fixture
    // has none of, so gating on it passes.
    okq(dir.path())
        .args(["lint", "--check", "--rule", "L13"])
        .assert()
        .success();
}

#[test]
fn codes_retired_in_okf_0_2_7_are_a_usage_error() {
    // L14/L15/L16 are gone and several survivors were renumbered. A CI config
    // pinning an old code must fail loudly rather than silently gate on a rule
    // that now means something else (ADR-0016).
    let dir = fixture();
    for code in ["L14", "L15", "L16"] {
        okq(dir.path())
            .args(["lint", "--rule", code])
            .assert()
            .failure()
            .code(2);
    }
}

#[test]
fn staleness_is_not_a_lint_concern() {
    // okf 0.2.7 moved it to `validate --today` (V12); `lint` has no --today.
    let dir = fixture();
    okq(dir.path())
        .args(["lint", "--today", "2026-08-02"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn orphans_and_l9_differ_on_indexed_concepts() {
    // L9 is narrower than `okq orphans`: a concept an index.md lists is not
    // reported, even with no inbound links (docs/features/lint.md).
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path().join("index.md"),
        "# Index\n\n- [Leaf](leaf.md)\n",
    );
    write(
        dir.path().join("leaf.md"),
        "---\ntype: doc\ntitle: Leaf\ndescription: Indexed but unlinked\n---\n\n# Leaf\n\nBody.\n",
    );

    let orphans = json(dir.path(), &["orphans"]);
    assert_eq!(orphans["count"], 1);

    let lint = json(dir.path(), &["lint", "--rule", "L9"]);
    assert_eq!(lint["findings"], 0);
}

#[test]
fn empty_result_is_json_and_exits_zero() {
    let dir = fixture();
    let out = json(dir.path(), &["lint", "--rule", "L13"]);
    assert_eq!(out["findings"], 0);
    assert!(out["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn schema_is_published_for_agents() {
    let dir = fixture();
    let schema = json(dir.path(), &["schema", "lint"]);
    assert!(schema["properties"]["diagnostics"].is_object());
}

#[test]
fn a_malformed_bundle_lints_without_panicking() {
    okq(Path::new("docs/tests")).arg("lint").assert().success();
}
