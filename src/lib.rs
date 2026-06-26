//! okq — the query and navigation layer for Open Knowledge Format (OKF)
//! bundles. See PLAN.md and docs/ for the design.
//!
//! This crate is structured library-first: all logic lives here and the thin
//! `okq` binary just calls [`run`], so commands are testable without a process.

pub mod cli;
pub mod commands;
pub mod error;
pub mod model;
pub mod sections;
pub mod yaml_json;

use clap::Parser;

use cli::{Cli, Command};
use error::AppError;

/// Parses argv, runs the requested command, and returns the process exit code.
/// Usage errors are handled by clap (exit 2); everything else maps through
/// [`AppError::exit_code`].
pub fn run() -> i32 {
    let cli = Cli::parse();
    match dispatch(&cli) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("okq: error: {e}");
            e.exit_code()
        }
    }
}

fn dispatch(cli: &Cli) -> Result<(), AppError> {
    match &cli.command {
        Command::Get(args) => {
            let got = commands::get::run(&cli.bundle, args)?;
            if cli.json {
                println!("{}", commands::get::to_json(&got));
            } else {
                let mut out = anstream::stdout().lock();
                commands::get::render_human(&mut out, &got, cli.no_color)?;
            }
            Ok(())
        }
        Command::Find(args) => {
            let found = commands::find::run(&cli.bundle, args)?;
            if cli.json {
                println!("{}", commands::find::to_json(&found));
            } else {
                let mut out = anstream::stdout().lock();
                commands::find::render_human(&mut out, &found, cli.no_color)?;
            }
            Ok(())
        }
    }
}
