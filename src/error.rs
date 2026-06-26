//! Application errors, each carrying the process exit code it maps to.
//!
//! The exit-code taxonomy is shared across all okq commands (see the `get`
//! feature spec); it is intended to graduate into its own ADR once a second
//! command lands. Codes: `0` success, `2` usage (clap), `4` concept not found,
//! `5` section not found/ambiguous, `1` other (I/O, bad bundle).

use std::fmt;

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
    /// The process exit code this error maps to.
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Usage(_) => 2,
            AppError::Bundle(_) | AppError::Index(_) => 1,
            AppError::InvalidConcept { .. } | AppError::ConceptNotFound { .. } => 4,
            AppError::SectionNotFound { .. } | AppError::SectionAmbiguous { .. } => 5,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Usage(msg) => write!(f, "{msg}"),
            AppError::Bundle(e) => write!(f, "{e}"),
            AppError::Index(msg) => write!(f, "search index error: {msg}"),
            AppError::InvalidConcept { input, reason } => {
                write!(f, "invalid concept id {input:?}: {reason}")
            }
            AppError::ConceptNotFound { input } => {
                write!(f, "no concept found for {input:?}")
            }
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
