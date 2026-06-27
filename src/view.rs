//! The filtered bundle view — the single seam where `.okqignore` takes effect.
//!
//! Every command loads its bundle through [`Corpus::load`] instead of
//! [`okf::Bundle::load`] directly. A `Corpus` pairs the loaded bundle with the
//! [`IgnoreSet`] for its tree and the set of *hidden* concept ids (those whose
//! file is excluded). Commands then enumerate concepts and resolve ids through
//! the corpus, so ignore filtering is applied once, here, rather than
//! re-implemented per command. okf stays unaware of exclusion (ADR-0006).

use std::collections::HashSet;
use std::path::Path;

use okf::{Bundle, Concept, ConceptId};

use crate::error::AppError;
use crate::ignore::IgnoreSet;

/// A loaded bundle plus its `.okqignore` filter.
pub struct Corpus {
    bundle: Bundle,
    ignore: IgnoreSet,
    /// Ids of concepts excluded by `.okqignore` — *not in the bundle* for query
    /// purposes (they 404 on `get`, and links into them are dead).
    hidden: HashSet<ConceptId>,
}

impl Corpus {
    /// Loads the bundle at `dir`, then its `.okqignore` rules. With `no_ignore`,
    /// the ignore set is disabled and nothing is hidden — the full tree.
    pub fn load(dir: &Path, no_ignore: bool) -> Result<Corpus, AppError> {
        let bundle = Bundle::load(dir)?;
        let ignore = IgnoreSet::load(bundle.root(), no_ignore);
        let hidden = bundle
            .concepts()
            .iter()
            .filter(|c| ignore.is_ignored(&c.path))
            .map(|c| c.id.clone())
            .collect();
        Ok(Corpus {
            bundle,
            ignore,
            hidden,
        })
    }

    /// The underlying bundle (full, unfiltered) — for root, links, and metadata.
    pub fn bundle(&self) -> &Bundle {
        &self.bundle
    }

    /// The `.okqignore` set for this bundle.
    pub fn ignore(&self) -> &IgnoreSet {
        &self.ignore
    }

    /// The hidden concept ids, for graph building (edges into these are dead).
    pub fn hidden(&self) -> &HashSet<ConceptId> {
        &self.hidden
    }

    /// `true` if a concept is excluded by `.okqignore`.
    pub fn is_hidden(&self, id: &ConceptId) -> bool {
        self.hidden.contains(id)
    }

    /// The visible concepts, in okf's path order (ignored ones removed).
    pub fn concepts(&self) -> impl Iterator<Item = &Concept> {
        self.bundle
            .concepts()
            .iter()
            .filter(|c| !self.hidden.contains(&c.id))
    }

    /// `true` if a visible concept with this id exists.
    pub fn contains(&self, id: &ConceptId) -> bool {
        !self.hidden.contains(id) && self.bundle.contains(id)
    }

    /// Looks up a visible concept by id; `None` if absent or hidden.
    pub fn get(&self, id: &ConceptId) -> Option<&Concept> {
        if self.hidden.contains(id) {
            return None;
        }
        self.bundle.get(id)
    }
}
