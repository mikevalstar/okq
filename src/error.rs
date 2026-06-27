//! Application errors, each carrying the process exit code it maps to.
//!
//! The exit-code taxonomy is the canonical agent/script contract defined in
//! ADR-0004 (`docs/adrs/0004-exit-code-taxonomy.md`); this module is its single
//! source of truth. See [`exit`] for the codes and [`AppError::exit_code`] for
//! the error→code mapping. Code `3` (a health check that *found issues*) is not
//! an error — a command returns it from a successful run via `dispatch`.

use std::fmt;

/// The okq exit-code taxonomy (ADR-0004). The canonical numbers; never renumber.
pub mod exit {
    /// Success — including a valid empty answer (no matches, no path, no neighbors).
    pub const SUCCESS: i32 = 0;
    /// Other / internal error: bad bundle, I/O, search-index failure.
    pub const ERROR: i32 = 1;
    /// Usage error: bad flags/args, malformed `--where`, invalid regex/query.
    pub const USAGE: i32 = 2;
    /// A health check ran cleanly but found issues (opt-in, e.g. `--check`).
    pub const CHECK_FAILED: i32 = 3;
    /// Concept not found / not resolvable.
    pub const NOT_FOUND: i32 = 4;
    /// Section not found / ambiguous within a resolved concept.
    pub const SECTION: i32 = 5;
}

/// A top-level okq error.
#[derive(Debug)]
pub enum AppError {
    /// A predicate or argument was invalid at runtime (bad `--where`, invalid
    /// `--regex`). Maps to the same exit code clap uses for usage errors.
    Usage(String),
    /// The bundle could not be loaded (bad `--bundle` directory, I/O failure).
    Bundle(okf::BundleError),
    /// A search-index operation failed (build, open, writer-lock contention).
    Index(String),
    /// A write/scaffold operation failed or would overwrite an existing file.
    Io(String),
    /// The concept identity was syntactically invalid.
    InvalidConcept {
        /// The identity as the caller typed it.
        input: String,
        /// Why it was rejected.
        reason: String,
    },
    /// No concept resolved for the given identity.
    ConceptNotFound {
        /// The identity as the caller typed it.
        input: String,
    },
    /// A partial identity matched more than one concept.
    ConceptAmbiguous {
        /// The identity as the caller typed it.
        input: String,
        /// The matching concept ids.
        candidates: Vec<String>,
    },
    /// `--section` matched no heading in the resolved concept.
    SectionNotFound {
        /// The concept that was searched.
        concept: String,
        /// The requested heading/slug.
        query: String,
    },
    /// `--section` matched more than one heading.
    SectionAmbiguous {
        /// The concept that was searched.
        concept: String,
        /// The requested heading/slug.
        query: String,
        /// The matching headings, with their locations.
        candidates: Vec<String>,
    },
}

impl AppError {
    /// The process exit code this error maps to (ADR-0004).
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Usage(_) => exit::USAGE,
            AppError::Bundle(_) | AppError::Index(_) | AppError::Io(_) => exit::ERROR,
            AppError::InvalidConcept { .. }
            | AppError::ConceptNotFound { .. }
            | AppError::ConceptAmbiguous { .. } => exit::NOT_FOUND,
            AppError::SectionNotFound { .. } | AppError::SectionAmbiguous { .. } => exit::SECTION,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Usage(msg) => write!(f, "{msg}"),
            AppError::Bundle(e) => write!(f, "{e}"),
            AppError::Index(msg) => write!(f, "search index error: {msg}"),
            AppError::Io(msg) => write!(f, "{msg}"),
            AppError::InvalidConcept { input, reason } => {
                write!(f, "invalid concept id {input:?}: {reason}")
            }
            AppError::ConceptNotFound { input } => {
                write!(f, "no concept found for {input:?}")
            }
            AppError::ConceptAmbiguous { input, candidates } => write!(
                f,
                "{input:?} matches multiple concepts; disambiguate: {}",
                candidates.join(", ")
            ),
            AppError::SectionNotFound { concept, query } => {
                write!(f, "no section {query:?} in concept {concept:?}")
            }
            AppError::SectionAmbiguous {
                concept,
                query,
                candidates,
            } => write!(
                f,
                "section {query:?} is ambiguous in concept {concept:?}; candidates: {}",
                candidates.join("; ")
            ),
        }
    }
}

impl std::error::Error for AppError {}

impl From<okf::BundleError> for AppError {
    fn from(e: okf::BundleError) -> Self {
        AppError::Bundle(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Bundle(okf::BundleError::Io(e))
    }
}
