//! `aetherc` — the command-line driver for the Aether compiler platform.
//!
//! This binary is intentionally thin: it collects process arguments and hands
//! off to [`cli::run`], which performs argument parsing and command dispatch.
//! Compiler logic will live in dedicated library crates (see `ARCHITECTURE.md`)
//! and be orchestrated from here as the platform grows.

mod cli;

use std::process::ExitCode;

fn main() -> ExitCode {
    // Skip argv[0] (the program name); `cli::run` only reasons about arguments.
    let args: Vec<String> = std::env::args().skip(1).collect();
    cli::run(&args)
}
