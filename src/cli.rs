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

const NEIGHBORS_EXAMPLES: &str = "\
Examples:
  # Concepts one hop away, in or out, any edge type
  okq neighbors adrs/0002-library-stack

  # Two hops, outbound only, following just `supersedes` edges
  okq neighbors adrs/0002-library-stack --depth 2 --direction out --edge supersedes";

const BACKLINKS_EXAMPLES: &str = "\
Examples:
  # What links *to* this concept (the inbound view)
  okq backlinks adrs/0002-library-stack --json";

const PATH_EXAMPLES: &str = "\
Examples:
  # Shortest route between two concepts (follows edge direction)
  okq path adrs/0003-search-index-in-xdg-cache adrs/0001-documentation-first-okf-shaped

  # Ignore direction (treat links as bidirectional)
  okq path features/search features/get --undirected";

const ORPHANS_EXAMPLES: &str = "\
Examples:
  # Concepts nothing links to (stale-doc candidates)
  okq orphans

  # Fail CI if any exist
  okq orphans --check";

const DEADLINKS_EXAMPLES: &str = "\
Examples:
  # Links pointing at missing/renamed concepts
  okq deadlinks

  # Fail CI if any exist
  okq deadlinks --check";

const STATS_EXAMPLES: &str = "\
Examples:
  # One-glance overview of the bundle
  okq stats

  # Machine-readable, with the top 5 hubs/tags
  okq stats --json --top 5";

const SCHEMA_EXAMPLES: &str = "\
Examples:
  # JSON Schema for one command's --json envelope
  okq schema search

  # All schemas, as a committable contract artifact
  okq schema > schemas.json";

const INIT_EXAMPLES: &str = "\
Examples:
  # Scaffold an OKF bundle in the current directory
  okq init

  # ...or in a specific directory (idempotent; safe to re-run)
  okq --bundle docs init";

const NEW_EXAMPLES: &str = "\
Examples:
  # Add an auto-numbered ADR
  okq new adr \"Adopt Tantivy for search\"

  # Add a feature spec; open the file it prints
  $EDITOR \"$(okq new feature 'Saved searches')\"

  # List the available types
  okq new --list";

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

    /// Ignore all .okqignore files; query the full tree, nothing excluded.
    #[arg(long, global = true)]
    pub no_ignore: bool,

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

    /// Concepts adjacent to one concept via the link graph (N-hop, typed edges).
    #[command(after_help = NEIGHBORS_EXAMPLES, after_long_help = NEIGHBORS_EXAMPLES)]
    Neighbors(NeighborsArgs),

    /// Concepts that link *to* a concept (the inbound view).
    #[command(after_help = BACKLINKS_EXAMPLES, after_long_help = BACKLINKS_EXAMPLES)]
    Backlinks(BacklinksArgs),

    /// Shortest link path between two concepts.
    #[command(after_help = PATH_EXAMPLES, after_long_help = PATH_EXAMPLES)]
    Path(PathArgs),

    /// Concepts with no inbound links (stale-doc candidates).
    #[command(after_help = ORPHANS_EXAMPLES, after_long_help = ORPHANS_EXAMPLES)]
    Orphans(OrphansArgs),

    /// Links pointing to missing/renamed concepts (inline + frontmatter).
    #[command(after_help = DEADLINKS_EXAMPLES, after_long_help = DEADLINKS_EXAMPLES)]
    Deadlinks(DeadlinksArgs),

    /// Bundle overview: counts, distributions, link density, and hubs.
    #[command(after_help = STATS_EXAMPLES, after_long_help = STATS_EXAMPLES)]
    Stats(StatsArgs),

    /// Print the JSON Schema for a command's --json output (the agent contract).
    #[command(after_help = SCHEMA_EXAMPLES, after_long_help = SCHEMA_EXAMPLES)]
    Schema(SchemaArgs),

    /// Scaffold a new OKF bundle (adrs/ + features/, seed docs, README).
    #[command(after_help = INIT_EXAMPLES, after_long_help = INIT_EXAMPLES)]
    Init(InitArgs),

    /// Create one concept from a template (adr | feature).
    #[command(after_help = NEW_EXAMPLES, after_long_help = NEW_EXAMPLES)]
    New(NewArgs),
}

/// Edge-traversal direction.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum DirectionArg {
    /// Follow inbound edges.
    In,
    /// Follow outbound edges.
    Out,
    /// Follow both.
    Both,
}

/// Arguments for `okq neighbors`.
#[derive(Args, Debug)]
pub struct NeighborsArgs {
    /// Concept whose neighbors to list (id or `.md` path).
    #[arg(value_name = "CONCEPT")]
    pub concept: String,

    /// How many hops out to traverse.
    #[arg(long, default_value_t = 1, value_name = "N")]
    pub depth: usize,

    /// Which edge directions to follow.
    #[arg(long, value_enum, default_value_t = DirectionArg::Both)]
    pub direction: DirectionArg,

    /// Restrict to these edge types (repeatable; e.g. `link`, `related`, `supersedes`).
    #[arg(long, value_name = "TYPE")]
    pub edge: Vec<String>,
}

/// Arguments for `okq backlinks`.
#[derive(Args, Debug)]
pub struct BacklinksArgs {
    /// Concept whose inbound links to list (id or `.md` path).
    #[arg(value_name = "CONCEPT")]
    pub concept: String,

    /// Restrict to these edge types (repeatable).
    #[arg(long, value_name = "TYPE")]
    pub edge: Vec<String>,
}

/// Arguments for `okq path`.
#[derive(Args, Debug)]
pub struct PathArgs {
    /// Start concept.
    #[arg(value_name = "FROM")]
    pub from: String,

    /// End concept.
    #[arg(value_name = "TO")]
    pub to: String,

    /// Ignore edge direction (treat links as bidirectional).
    #[arg(long)]
    pub undirected: bool,

    /// Restrict to these edge types (repeatable).
    #[arg(long, value_name = "TYPE")]
    pub edge: Vec<String>,
}

/// Arguments for `okq orphans`.
#[derive(Args, Debug)]
pub struct OrphansArgs {
    /// Exit 3 if any orphans are found (for CI gating).
    #[arg(long)]
    pub check: bool,
}

/// Arguments for `okq deadlinks`.
#[derive(Args, Debug)]
pub struct DeadlinksArgs {
    /// Exit 3 if any dead links are found (for CI gating).
    #[arg(long)]
    pub check: bool,
}

/// Arguments for `okq stats`.
#[derive(Args, Debug)]
pub struct StatsArgs {
    /// Cap the hubs and tags lists at this many entries.
    #[arg(long, default_value_t = 10, value_name = "N")]
    pub top: usize,
}

/// Arguments for `okq schema`.
#[derive(Args, Debug)]
pub struct SchemaArgs {
    /// Command whose output schema to print; omit for all commands.
    #[arg(value_name = "COMMAND")]
    pub command: Option<String>,
}

/// Arguments for `okq init`.
#[derive(Args, Debug)]
pub struct InitArgs {}

/// Arguments for `okq new`.
#[derive(Args, Debug)]
pub struct NewArgs {
    /// Concept type to create (`adr` or `feature`).
    #[arg(value_name = "TYPE")]
    pub type_: Option<String>,

    /// Title for the new concept.
    #[arg(value_name = "TITLE")]
    pub title: Option<String>,

    /// List the available types and exit.
    #[arg(long)]
    pub list: bool,
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
