//! okq — query and navigation layer for Open Knowledge Format (OKF) bundles.
//!
//! Pre-alpha placeholder. The command surface (search / find / neighbors /
//! backlinks / path / orphans / deadlinks / stats / get / init / new) is
//! designed in PLAN.md and docs/adrs/ but not yet implemented.

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("-V" | "--version") => println!("okq {VERSION}"),
        _ => {
            println!("okq {VERSION} — pre-alpha, no commands implemented yet.");
            println!("Design: https://github.com/mikevalstar/okq (see PLAN.md and docs/adrs/).");
        }
    }
}
