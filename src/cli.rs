//! Command-line surface for okq, parsed with clap.
//!
//! Global flags (`--bundle`, `--json`, `--no-color`) are shared across every
//! command. Today the only subcommand is `get`; more land in later milestones.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// okq — query and navigation for Open Knowledge Format (OKF) bundles.
#[derive(Parser, Debug)]
#[command(name = "okq", version, about, long_about = None)]
pub struct Cli {
    /// Bundle directory to query.
    #[arg(long, global = true, value_name = "DIR", default_value = ".")]
    pub bundle: PathBuf,

    /// Emit machine-readable JSON instead of human-readable text.
    #[arg(long, global = true)]
    pub json: bool,

    /// Disable colored output.
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// The okq subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print one concept's frontmatter and/or body (optionally a single section).
    Get(GetArgs),

    /// Filter concepts by frontmatter/content predicates (set membership, not ranking).
    Find(FindArgs),
}

/// Arguments for `okq get`.
#[derive(Args, Debug)]
pub struct GetArgs {
    /// Concept to expand: a concept id (`tables/users`) or `.md` path
    /// (`tables/users.md`).
    #[arg(value_name = "CONCEPT")]
    pub concept: String,

    /// Print only the frontmatter.
    #[arg(long)]
    pub frontmatter: bool,

    /// Print only the body.
    #[arg(long)]
    pub body: bool,

    /// Print only the named section (matched by heading text or slug).
    #[arg(long, value_name = "HEADING")]
    pub section: Option<String>,
}

/// Arguments for `okq find`.
#[derive(Args, Debug)]
pub struct FindArgs {
    /// Require this tag (repeatable; all required — AND).
    #[arg(long, value_name = "TAG")]
    pub tag: Vec<String>,

    /// Require this `type` (repeatable; any matches — OR).
    #[arg(long = "type", value_name = "TYPE")]
    pub type_: Vec<String>,

    /// Require a frontmatter predicate `field=value` (repeatable; all required — AND).
    #[arg(long = "where", value_name = "FIELD=VALUE")]
    pub where_: Vec<String>,

    /// Require the title or body to contain this pattern.
    #[arg(long = "match", value_name = "PATTERN")]
    pub match_: Option<String>,

    /// Treat `--match` as a regular expression instead of a literal substring.
    #[arg(long, requires = "match_")]
    pub regex: bool,
}
