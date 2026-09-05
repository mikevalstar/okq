//! Trust & lifecycle values, derived from OKF v0.2 frontmatter.
//!
//! okf owns the semantics (`§5.2`–`§5.5`): `status()` reads the lifecycle
//! value, and the `verified` / `stale_after` keys carry the rest. okq's job
//! here is to turn those into the small, serializable shape the shared concept
//! envelope carries, and to decide what counts as a default worth omitting.
//!
//! **Two predicates okq computes itself.** okf 0.2.7 tightened
//! `Frontmatter::trust_tier()` and `is_stale_on()` to require a full ISO-8601
//! timestamp with a time of day *and* an explicit UTC offset, so a bare
//! `at: 2026-07-14` stops counting. okq reads bundles it did not write —
//! including ones its own older releases scaffolded with bare dates — and
//! silently reclassifying those as `unverified` is the wrong failure mode for a
//! trust signal. So [`tier_of`] counts any named verifier and [`is_stale`]
//! accepts a date *or* a datetime — which is what okf 0.2.1 did. `okq validate`
//! still reports the imprecise value as a V6/V7/V11 warning, so an author is
//! told to tighten it (ADR-0016).
//!
//! Every field is **omitted at its spec-defined default** (ADR-0014): absent
//! `status` means `stable`, absent `trust` means `unverified`, absent `stale`
//! means not stale. A concept with no trust frontmatter therefore serializes
//! exactly as it did before this existed. See `docs/features/trust.md`.

use okf::{Actor, Concept, Date, Frontmatter, Status, TrustTier};
use schemars::JsonSchema;
use serde::Serialize;

/// A concept's trust and lifecycle values, as they appear in an envelope.
///
/// `None` on any field means "at its default" — the field is skipped entirely
/// during serialization rather than emitted as null.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ConceptTrust {
    /// Lifecycle `status`, omitted when `stable` (the default when the key is
    /// absent). A producer-defined value outside draft/stable/deprecated is
    /// reported verbatim — §5.4 requires consumers to tolerate it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Derived trust tier, omitted when `unverified` (the default when there is
    /// no `verified` key). Never stored in frontmatter: it is computed from the
    /// `verified` actors — any `human:` verifier yields `human-reviewed`,
    /// non-human verifiers alone yield `machine-confirmed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust: Option<String>,
    /// `true` when `stale_after` has passed, omitted otherwise. Evaluated
    /// against `--today` when given, else the system date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale: Option<bool>,
}

impl ConceptTrust {
    /// Derives the envelope values for a concept, evaluating staleness against
    /// `today` (`None` skips the staleness check entirely).
    pub fn of(c: &Concept, today: Option<Date>) -> Self {
        Self::from_frontmatter(&c.document.frontmatter, today)
    }

    /// Same, from frontmatter alone — `get` already holds one.
    pub fn from_frontmatter(fm: &Frontmatter, today: Option<Date>) -> Self {
        let status = fm.status();
        let tier = tier_of(fm);
        ConceptTrust {
            status: (status != Status::Stable).then(|| status.to_string()),
            trust: (tier != TrustTier::Unverified).then(|| tier_name(tier).to_string()),
            stale: today.and_then(|d| is_stale(fm, d).then_some(true)),
        }
    }

    /// `true` when every value is at its default, so the whole block can be
    /// left out of human output.
    pub fn is_default(&self) -> bool {
        self.status.is_none() && self.trust.is_none() && self.stale.is_none()
    }

    /// The non-default values, for the `[draft, human-reviewed, stale]` suffix
    /// human output appends to a concept line. Empty when all are default.
    pub fn labels(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(s) = &self.status {
            out.push(s.clone());
        }
        if let Some(t) = &self.trust {
            out.push(t.clone());
        }
        if self.stale == Some(true) {
            out.push("stale".to_string());
        }
        out
    }
}

/// The trust tier a concept's `verified` events derive to.
///
/// An event counts when it **names a verifier**. The tier answers *who* has
/// reviewed this, not *when*, so a missing or imprecise `at` does not change
/// the answer — which is also what okf 0.2.1 did. Any human verifier wins;
/// non-human verifiers alone yield `machine-confirmed`; no named verifier is
/// `unverified`. Ordering events by time is a separate question, and one that
/// does need a parseable timestamp.
///
/// See the module docs for why okq derives this rather than calling
/// `Frontmatter::trust_tier()`.
pub fn tier_of(fm: &Frontmatter) -> TrustTier {
    let mut usable = fm
        .verified()
        .into_iter()
        .filter(|v| {
            v.by.as_ref()
                .is_some_and(|by| !by.as_str().trim().is_empty())
        })
        .peekable();
    if usable.peek().is_none() {
        TrustTier::Unverified
    } else if usable.any(|v| v.by.as_ref().is_some_and(Actor::is_human)) {
        TrustTier::HumanReviewed
    } else {
        TrustTier::MachineConfirmed
    }
}

/// Whether `stale_after` has passed on `today`, accepting a bare date where okf
/// 0.2.7 requires a full offset-bearing timestamp.
///
/// A datetime is compared on its date part, without shifting by any offset —
/// matching the reference implementation's truncation. Missing or unparseable
/// means never stale.
pub fn is_stale(fm: &Frontmatter, today: Date) -> bool {
    fm.stale_after()
        .and_then(|f| f.datetime)
        .is_some_and(|dt| today >= dt.date)
}

/// Canonical spelling of a trust tier, for filters and distributions.
pub fn tier_name(tier: TrustTier) -> &'static str {
    match tier {
        TrustTier::Unverified => "unverified",
        TrustTier::MachineConfirmed => "machine-confirmed",
        TrustTier::HumanReviewed => "human-reviewed",
    }
}

/// The tier names a `--trust` filter accepts, in ascending order of confidence.
pub const TIER_NAMES: [&str; 3] = ["unverified", "machine-confirmed", "human-reviewed"];

/// Parses a `--today YYYY-MM-DD` value, falling back to the system date when no
/// flag was given.
///
/// Staleness is the one okq predicate whose answer depends on the clock, so the
/// flag exists to make it reproducible (ADR-0014): same bundle plus same
/// `--today` gives the same answer.
pub fn resolve_today(flag: Option<&str>) -> Result<Option<Date>, crate::error::AppError> {
    match flag {
        Some(raw) => Date::parse(raw.trim()).map(Some).ok_or_else(|| {
            crate::error::AppError::Usage(format!(
                "--today must be an ISO date (YYYY-MM-DD), got {raw:?}"
            ))
        }),
        None => Ok(Date::today_utc()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use okf::Document;

    fn fm(yaml: &str) -> Frontmatter {
        let text = format!("---\n{yaml}\n---\n\n# T\n");
        Document::parse(&text).unwrap().frontmatter
    }

    fn today() -> Option<Date> {
        Date::parse("2026-08-02")
    }

    #[test]
    fn no_trust_frontmatter_is_all_default() {
        let t = ConceptTrust::from_frontmatter(&fm("type: doc"), today());
        assert!(t.is_default());
        assert_eq!(serde_json::to_string(&t).unwrap(), "{}");
    }

    #[test]
    fn bare_verified_mapping_counts_as_one_event() {
        // §5.2: a bare `{by, at}` mapping MUST read as a one-element list.
        let t = ConceptTrust::from_frontmatter(
            &fm("type: doc\nverified: { by: 'human:mike', at: 2026-07-14 }"),
            today(),
        );
        assert_eq!(t.trust.as_deref(), Some("human-reviewed"));
    }

    #[test]
    fn human_verifier_outranks_machine() {
        let machine = ConceptTrust::from_frontmatter(
            &fm("type: doc\nverified:\n  - { by: 'process:nightly', at: 2026-07-01 }"),
            today(),
        );
        assert_eq!(machine.trust.as_deref(), Some("machine-confirmed"));

        let mixed = ConceptTrust::from_frontmatter(
            &fm(
                "type: doc\nverified:\n  - { by: 'process:nightly', at: 2026-07-01 }\n  - { by: 'human:mike', at: 2026-07-02 }",
            ),
            today(),
        );
        assert_eq!(mixed.trust.as_deref(), Some("human-reviewed"));
    }

    #[test]
    fn unknown_status_is_reported_verbatim() {
        let t = ConceptTrust::from_frontmatter(&fm("type: doc\nstatus: active"), today());
        assert_eq!(t.status.as_deref(), Some("active"));
    }

    #[test]
    fn stable_status_is_omitted() {
        let t = ConceptTrust::from_frontmatter(&fm("type: doc\nstatus: stable"), today());
        assert_eq!(t.status, None);
    }

    #[test]
    fn staleness_compares_against_today() {
        let past = fm("type: doc\nstale_after: 2026-01-01");
        let future = fm("type: doc\nstale_after: 2099-01-01");
        assert_eq!(
            ConceptTrust::from_frontmatter(&past, today()).stale,
            Some(true)
        );
        assert_eq!(ConceptTrust::from_frontmatter(&future, today()).stale, None);
        // No `today` at all: staleness is not evaluated.
        assert_eq!(ConceptTrust::from_frontmatter(&past, None).stale, None);
    }

    #[test]
    fn malformed_trust_frontmatter_degrades_to_defaults() {
        for yaml in [
            "type: doc\nverified: yesterday",
            "type: doc\nverified: [1, 2, 3]",
            "type: doc\ngenerated: sometime",
            "type: doc\nstale_after: not-a-date",
        ] {
            let t = ConceptTrust::from_frontmatter(&fm(yaml), today());
            assert!(t.is_default(), "{yaml:?} should degrade to defaults");
        }
    }

    #[test]
    fn labels_list_non_defaults_in_order() {
        let t = ConceptTrust::from_frontmatter(
            &fm(
                "type: doc\nstatus: draft\nverified: { by: 'human:mike' }\nstale_after: 2026-01-01",
            ),
            today(),
        );
        assert_eq!(t.labels(), ["draft", "human-reviewed", "stale"]);
    }

    #[test]
    fn today_flag_must_be_an_iso_date() {
        assert!(resolve_today(Some("2026-08-02")).unwrap().is_some());
        assert!(resolve_today(Some("last tuesday")).is_err());
    }
}
