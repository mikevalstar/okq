//! okq — the query and navigation layer for Open Knowledge Format (OKF)
//! bundles. See PLAN.md and docs/ for the design.
//!
//! This crate is structured library-first: all logic lives here and the thin
//! `okq` binary just calls [`run`], so commands are testable without a process.

pub mod cli;
pub mod commands;
pub mod error;
pub mod graph;
pub mod index;
pub mod model;
pub mod sections;
pub mod yaml_json;

use clap::Parser;

use cli::{Cli, Command};
use error::{AppError, exit};

/// Parses argv, runs the requested command, and returns the process exit code.
/// On success a command yields its own exit code (`0`, or `3` for a `--check`
/// that found issues — ADR-0004); errors map through [`AppError::exit_code`].
pub fn run() -> i32 {
    let cli = Cli::parse();
    match dispatch(&cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("okq: error: {e}");
            e.exit_code()
        }
    }
}

/// Runs the chosen command and returns its success exit code.
fn dispatch(cli: &Cli) -> Result<i32, AppError> {
    match &cli.command {
        Command::Get(args) => {
            let got = commands::get::run(&cli.bundle, args)?;
            if cli.json {
                println!("{}", commands::get::to_json(&got));
            } else {
                let mut out = anstream::stdout().lock();
                commands::get::render_human(&mut out, &got, cli.no_color)?;
            }
            Ok(exit::SUCCESS)
        }
        Command::Find(args) => {
            let found = commands::find::run(&cli.bundle, args)?;
            if cli.json {
                println!("{}", commands::find::to_json(&found));
            } else if found.results.is_empty() {
                eprintln!("No concepts match.");
            } else {
                let mut out = anstream::stdout().lock();
                commands::find::render_human(&mut out, &found, cli.no_color)?;
            }
            Ok(exit::SUCCESS)
        }
        Command::Search(args) => {
            let found = commands::search::run(&cli.bundle, args)?;
            if cli.json {
                println!("{}", commands::search::to_json(&found));
            } else if found.results.is_empty() {
                eprintln!("No matches for {:?}.", found.query);
            } else {
                let mut out = anstream::stdout().lock();
                commands::search::render_human(&mut out, &found, cli.no_color)?;
            }
            Ok(exit::SUCCESS)
        }
        Command::Neighbors(args) => {
            let out = commands::graph::neighbors(&cli.bundle, args)?;
            if cli.json {
                println!("{}", commands::graph::to_json(&out));
            } else if out.results.is_empty() {
                eprintln!("No neighbors of {:?}.", out.concept);
            } else {
                let mut w = anstream::stdout().lock();
                commands::graph::render_nodes(&mut w, &out, cli.no_color)?;
            }
            Ok(exit::SUCCESS)
        }
        Command::Backlinks(args) => {
            let out = commands::graph::backlinks(&cli.bundle, args)?;
            if cli.json {
                println!("{}", commands::graph::to_json(&out));
            } else if out.results.is_empty() {
                eprintln!("No backlinks to {:?}.", out.concept);
            } else {
                let mut w = anstream::stdout().lock();
                commands::graph::render_nodes(&mut w, &out, cli.no_color)?;
            }
            Ok(exit::SUCCESS)
        }
        Command::Path(args) => {
            let out = commands::graph::path(&cli.bundle, args)?;
            if cli.json {
                println!("{}", commands::graph::to_json(&out));
            } else if !out.found {
                eprintln!("No path from {:?} to {:?}.", out.from, out.to);
            } else {
                let mut w = anstream::stdout().lock();
                commands::graph::render_path(&mut w, &out, cli.no_color)?;
            }
            Ok(exit::SUCCESS)
        }
        Command::Orphans(args) => {
            let out = commands::graph::orphans(&cli.bundle, args)?;
            if cli.json {
                println!("{}", commands::graph::to_json(&out));
            } else if out.results.is_empty() {
                eprintln!("No orphans.");
            } else {
                let mut w = anstream::stdout().lock();
                commands::graph::render_orphans(&mut w, &out, cli.no_color)?;
            }
            Ok(check_code(args.check, out.count))
        }
        Command::Deadlinks(args) => {
            let out = commands::graph::deadlinks(&cli.bundle, args)?;
            if cli.json {
                println!("{}", commands::graph::to_json(&out));
            } else if out.results.is_empty() {
                eprintln!("No dead links.");
            } else {
                let mut w = anstream::stdout().lock();
                commands::graph::render_deadlinks(&mut w, &out, cli.no_color)?;
            }
            Ok(check_code(args.check, out.count))
        }
    }
}

/// Maps a health-check outcome to an exit code: `3` when `--check` and issues
/// were found, else `0` (ADR-0004).
fn check_code(check: bool, count: usize) -> i32 {
    if check && count > 0 {
        exit::CHECK_FAILED
    } else {
        exit::SUCCESS
    }
}
