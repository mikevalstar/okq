//! Command-line surface for okq, parsed with clap.
//!
//! Global flags (`--bundle`, `--json`, `--no-color`) are shared across every
//! command. Help is treated as a feature: each command carries a one-line
//! summary, a longer explanation, and runnable examples.

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Colored help, in the spirit of `gh`: bold green headers/usage, cyan literals.
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

const ABOUT: &str =
    "Query and navigate Open Knowledge Format (OKF) bundles — for humans and agents.";

const LONG_ABOUT: &str = "\
okq is a fast, deterministic, local-first CLI for querying collections of
Markdown-with-frontmatter documents (Open Knowledge Format bundles): ADRs,
design docs, runbooks, wikis.

The same tool serves a person at a terminal and an AI agent assembling context:
every command has a --json mode and script-friendly exit codes. No embeddings,
no network, no API key — same bundle, same answer, every time.";

const MAIN_EXAMPLES: &str = "\
Examples:
  # Rank sections across the bundle by relevance
  okq search \"retrieval latency\"

  # Filter concepts by frontmatter predicate
  okq find --type adr --tag security

  # Expand one concept, or just one section of it
  okq get adrs/0006-agent-runnable-commands --section Decision

  # Everything speaks JSON, for agents and scripts
  okq search \"auth\" --json | jq -r '.results[].path'

Learn more:
  Design & docs:  https://github.com/mikevalstar/okq
  Run 'okq <command> --help' for details and examples on a command.";

const GET_EXAMPLES: &str = "\
Examples:
  # Whole concept (frontmatter + body) — pipe to a pager or glow
  okq get adrs/0002-library-stack

  # Just one section, addressed by heading or slug
  okq get adrs/0002-library-stack --section Decision

  # Only the frontmatter, as JSON
  okq get features/get --frontmatter --json";

const FIND_EXAMPLES: &str = "\
Examples:
  # Concepts carrying a tag
  okq find --tag security

  # ADRs that are accepted (predicates AND together)
  okq find --type adr --where status=accepted

  # Regex over title/body, as JSON
  okq find --match 'BM[0-9]+' --regex --json";

const SEARCH_EXAMPLES: &str = "\
Examples:
  # Rank sections by relevance (terms OR together, BM25)
  okq search \"tantivy index lifecycle\"

  # Exact phrase, top 5 hits
  okq search '\"search backend\"' --limit 5

  # Feed the best hit's location into get, as JSON
  okq search retrieval --json | jq -r '.results[0].path'";

/// okq — query and navigation for Open Knowledge Format (OKF) bundles.
#[derive(Parser, Debug)]
#[command(
    name = "okq",
    version,
    about = ABOUT,
    long_about = LONG_ABOUT,
    after_help = MAIN_EXAMPLES,
    after_long_help = MAIN_EXAMPLES,
    styles = STYLES,
)]
pub struct Cli {
    /// Bundle directory to query [default: current directory].
    #[arg(
        long,
        global = true,
        value_name = "DIR",
        default_value = ".",
        hide_default_value = true
    )]
    pub bundle: PathBuf,

    /// Emit machine-readable JSON on stdout instead of human-readable text.
    #[arg(long, global = true)]
    pub json: bool,

    /// Disable colored output (also honors NO_COLOR).
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// The okq subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Expand one concept: print its frontmatter and/or body, or a single section.
    #[command(after_help = GET_EXAMPLES, after_long_help = GET_EXAMPLES)]
    Get(GetArgs),

    /// Filter concepts by exact predicate (tags, type, frontmatter, text match).
    #[command(after_help = FIND_EXAMPLES, after_long_help = FIND_EXAMPLES)]
    Find(FindArgs),

    /// Search: rank sections by relevance (full-text BM25, via a Tantivy index).
    #[command(after_help = SEARCH_EXAMPLES, after_long_help = SEARCH_EXAMPLES)]
    Search(SearchArgs),
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

/// Arguments for `okq search`.
#[derive(Args, Debug)]
pub struct SearchArgs {
    /// The query. Multiple terms rank by relevance (OR + BM25); `"quote"` for a phrase.
    #[arg(value_name = "QUERY")]
    pub query: String,

    /// Maximum number of ranked hits to return.
    #[arg(long, default_value_t = 10, value_name = "N")]
    pub limit: usize,

    /// Force a full rebuild of the index before searching.
    #[arg(long)]
    pub reindex: bool,

    /// Build a transient in-memory index for this run; write nothing to disk.
    #[arg(long)]
    pub ephemeral: bool,
}
