//! Embedded templates and content generators for `okq init` / `okq new`.
//!
//! Frontmatter follows the Google OKF well-known keys (`type`/`title`/
//! `description`/`tags`/`timestamp`); we don't impose this repo's own extensions
//! (status/created/updated) on adopters. See `docs/features/scaffold.md`.

use std::time::{SystemTime, UNIX_EPOCH};

/// Markers that fence the okq-owned block in a README.
pub const README_BEGIN: &str = "<!-- okq:begin -->";
/// Closing marker for the okq-owned README block.
pub const README_END: &str = "<!-- okq:end -->";

/// Today's date as ISO-8601 `YYYY-MM-DD` (UTC), for frontmatter `timestamp`.
/// Authoring may read the clock — the determinism principle binds queries, not writes.
pub fn today_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Howard Hinnant's days-from-civil inverse: days since 1970-01-01 → (year, month, day).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Frontmatter + body for a new ADR (`okq new adr`).
pub fn adr(title: &str, date: &str) -> String {
    format!(
        "---\n\
         type: adr\n\
         title: {title}\n\
         description: One-line summary of the decision.\n\
         tags: []\n\
         timestamp: {date}\n\
         ---\n\
         \n\
         # {title}\n\
         \n\
         ## Status\n\
         \n\
         Proposed.\n\
         \n\
         ## Context\n\
         \n\
         What is the situation that forces a decision?\n\
         \n\
         ## Decision\n\
         \n\
         What we decided, and the main reasons it won.\n\
         \n\
         ## Consequences\n\
         \n\
         What becomes easier, what becomes harder.\n"
    )
}

/// Frontmatter + body for a new feature (`okq new feature`).
pub fn feature(title: &str, date: &str) -> String {
    format!(
        "---\n\
         type: feature\n\
         title: {title}\n\
         description: One-line summary of the capability.\n\
         tags: []\n\
         timestamp: {date}\n\
         ---\n\
         \n\
         # {title}\n\
         \n\
         ## Summary\n\
         \n\
         One or two sentences: what this lets the user do.\n\
         \n\
         ## Motivation\n\
         \n\
         Why this exists — the problem it removes.\n\
         \n\
         ## Behavior\n\
         \n\
         How it works from the user's perspective.\n\
         \n\
         ## Acceptance criteria\n\
         \n\
         - [ ] Concrete, checkable statements that mean \"done\".\n\
         \n\
         ## Open questions\n\
         \n\
         - Anything unresolved.\n"
    )
}

/// The canonical seed ADR (Michael Nygard's "Record architecture decisions").
pub fn seed_adr(date: &str) -> String {
    format!(
        "---\n\
         type: adr\n\
         title: Record architecture decisions\n\
         description: Use ADRs to capture significant, hard-to-reverse decisions.\n\
         tags: [process]\n\
         timestamp: {date}\n\
         ---\n\
         \n\
         # Record architecture decisions\n\
         \n\
         ## Status\n\
         \n\
         Accepted.\n\
         \n\
         ## Context\n\
         \n\
         We need to record the architectural decisions made on this project — the\n\
         significant, hard-to-reverse ones — so the reasoning survives turnover and time.\n\
         \n\
         ## Decision\n\
         \n\
         We will use Architecture Decision Records, as [described by Michael\n\
         Nygard](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions).\n\
         Each record is one Markdown file in `adrs/`, numbered, with Status / Context /\n\
         Decision / Consequences.\n\
         \n\
         ## Consequences\n\
         \n\
         The rationale behind decisions is preserved and queryable\n\
         (`okq find --type adr`). One lightweight step is added when making a\n\
         significant decision.\n"
    )
}

/// The root `index.md` — carries `okf_version` (OKF §11) and a short listing.
pub fn root_index(name: &str) -> String {
    format!(
        "---\n\
         okf_version: \"0.1\"\n\
         ---\n\
         \n\
         # {name}\n\
         \n\
         An [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf)\n\
         (OKF) bundle — Markdown + YAML frontmatter, one concept per file. Query it with okq:\n\
         \n\
         {indent_cmds}\n\
         \n\
         Folders: `adrs/` (decisions), `features/` (specs).\n",
        indent_cmds = "    okq find --type adr\n    okq search \"<topic>\"\n    okq stats"
    )
}

/// `adrs/index.md` directory listing.
pub fn adrs_index() -> String {
    "# Architecture Decision Records\n\
     \n\
     Numbered records of significant, hard-to-reverse decisions. List them:\n\
     \n\
     \x20\x20\x20\x20okq find --type adr\n\
     \n\
     Add one with `okq new adr \"<title>\"`.\n"
        .to_string()
}

/// `features/index.md` directory listing.
pub fn features_index() -> String {
    "# Features\n\
     \n\
     One spec per capability. List them:\n\
     \n\
     \x20\x20\x20\x20okq find --type feature\n\
     \n\
     Add one with `okq new feature \"<title>\"`.\n"
        .to_string()
}

/// The okq-owned README block (between the markers), describing how to query the bundle.
pub fn okq_block() -> String {
    format!(
        "{README_BEGIN}\n\
         ## Knowledge base\n\
         \n\
         This directory is an [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf)\n\
         (OKF) bundle. Explore it with [okq](https://github.com/mikevalstar/okq):\n\
         \n\
         \x20\x20\x20\x20okq search \"<topic>\"     # ranked full-text\n\
         \x20\x20\x20\x20okq find --type adr      # filter by frontmatter\n\
         \x20\x20\x20\x20okq stats                # overview\n\
         \x20\x20\x20\x20okq new adr \"<title>\"    # add a decision\n\
         {README_END}"
    )
}

/// A base README for a bundle that has none.
pub fn base_readme(name: &str) -> String {
    format!(
        "---\ntype: readme\ntitle: {name}\n---\n\n# {name}\n\n{}\n",
        okq_block()
    )
}

/// Ensures a README's frontmatter has a `type` (adds `type: readme` if missing),
/// so the README stays a conformant concept.
pub fn ensure_type_readme(content: &str) -> String {
    if let Some(rest) = content.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            let frontmatter = &rest[..end];
            if frontmatter
                .lines()
                .any(|l| l.trim_start().starts_with("type:"))
            {
                return content.to_string();
            }
            return format!("---\ntype: readme\n{}", &content[4..]);
        }
    }
    format!("---\ntype: readme\n---\n\n{content}")
}

/// Injects (or replaces) the okq block in a README, between the markers.
pub fn inject_okq_block(content: &str) -> String {
    let block = okq_block();
    match (content.find(README_BEGIN), content.find(README_END)) {
        (Some(begin), Some(end)) if end >= begin => {
            let end = end + README_END.len();
            format!("{}{}{}", &content[..begin], block, &content[end..])
        }
        _ => format!("{}\n\n{}\n", content.trim_end(), block),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn today_is_iso_shaped() {
        let d = today_iso();
        assert_eq!(d.len(), 10);
        assert_eq!(d.as_bytes()[4], b'-');
        assert_eq!(d.as_bytes()[7], b'-');
    }

    #[test]
    fn civil_epoch_is_1970() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(31), (1970, 2, 1));
    }

    #[test]
    fn ensure_type_adds_when_missing() {
        assert!(ensure_type_readme("# Hi\n").starts_with("---\ntype: readme\n---"));
        assert!(ensure_type_readme("---\ntitle: X\n---\n\nBody\n").contains("type: readme"));
        // idempotent when already typed
        let typed = "---\ntype: readme\n---\n\nBody\n";
        assert_eq!(ensure_type_readme(typed), typed);
    }

    #[test]
    fn inject_replaces_between_markers() {
        let existing = format!("# Doc\n\n{README_BEGIN}\nold\n{README_END}\n\n## Keep me\n");
        let out = inject_okq_block(&existing);
        assert!(out.contains("## Keep me"));
        assert!(!out.contains("old"));
        assert_eq!(out.matches(README_BEGIN).count(), 1);
    }
}
