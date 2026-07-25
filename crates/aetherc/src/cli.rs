//! Argument parsing and command dispatch for the `aetherc` driver.
//!
//! The CLI is intentionally small and dependency-free while its surface area is
//! minimal. When that surface grows (multiple subcommands, many flags), we will
//! migrate to `clap`; see `TECH_DEBT.md`. Parsing is separated from execution so
//! it can be unit-tested in isolation, while `tests/cli.rs` exercises the built
//! binary end to end.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aether_diagnostics::{Diagnostic, DiagnosticHandler, render};
use aether_source::SourceMap;

/// Program name used in help and diagnostic output.
const PROG: &str = "aetherc";

/// Process exit codes returned by the driver.
///
/// Stable, distinct codes let scripts and integration tests reason about the
/// outcome of an invocation. Values follow common Unix conventions where a
/// suitable one exists.
mod exit {
    /// Everything succeeded.
    pub(super) const SUCCESS: u8 = 0;
    /// The command line was malformed (unknown flag, missing/extra argument).
    pub(super) const USAGE: u8 = 2;
    /// A recognized operation is not yet implemented.
    pub(super) const UNIMPLEMENTED: u8 = 3;
    /// An I/O failure occurred (mirrors `EX_IOERR` from `sysexits.h`).
    pub(super) const IO: u8 = 74;
}

/// A fully parsed CLI invocation.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    /// Print version information and exit.
    Version,
    /// Print usage information and exit.
    Help,
    /// Compile a single source file.
    Compile {
        /// Path to the source file to compile.
        input: PathBuf,
    },
}

/// A failure encountered while parsing command-line arguments.
#[derive(Debug, PartialEq, Eq)]
enum ParseError {
    /// An unrecognized flag/option was supplied.
    UnknownOption(String),
    /// A required input file was not provided.
    MissingInput,
    /// An extra positional argument was supplied.
    UnexpectedArgument(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownOption(opt) => write!(f, "unknown option '{opt}'"),
            ParseError::MissingInput => write!(f, "no input file provided"),
            ParseError::UnexpectedArgument(arg) => write!(f, "unexpected argument '{arg}'"),
        }
    }
}

/// Parse process arguments (excluding the program name) into a [`Command`].
///
/// `--help`/`--version` short-circuit as soon as they are seen, mirroring the
/// behavior of most command-line tools.
fn parse(args: &[String]) -> Result<Command, ParseError> {
    let mut input: Option<PathBuf> = None;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            // Anything else starting with '-' (other than a bare "-") is an
            // option we do not recognize.
            other if other.starts_with('-') && other != "-" => {
                return Err(ParseError::UnknownOption(other.to_string()));
            }
            positional => {
                if input.is_some() {
                    return Err(ParseError::UnexpectedArgument(positional.to_string()));
                }
                input = Some(PathBuf::from(positional));
            }
        }
    }

    match input {
        Some(input) => Ok(Command::Compile { input }),
        None => Err(ParseError::MissingInput),
    }
}

/// Entry point invoked by `main`: parse `args`, execute, and return an exit code.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let command = match parse(args) {
        Ok(command) => command,
        Err(err) => {
            eprintln!("{PROG}: error: {err}");
            eprintln!("{}", usage());
            return ExitCode::from(exit::USAGE);
        }
    };

    match command {
        Command::Version => {
            println!("{}", version_string());
            ExitCode::from(exit::SUCCESS)
        }
        Command::Help => {
            println!("{}", help_string());
            ExitCode::from(exit::SUCCESS)
        }
        Command::Compile { input } => compile(&input),
    }
}

/// Handle the `compile` command.
///
/// The full pipeline (lex → parse → lower → interpret) does not exist yet. This
/// loads the input into a real [`SourceMap`] and reports the current state
/// through the real diagnostics pipeline — exercising the M1 infrastructure end
/// to end — rather than pretending to compile.
fn compile(input: &Path) -> ExitCode {
    let source = match std::fs::read_to_string(input) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("{PROG}: error: cannot read '{}': {err}", input.display());
            return ExitCode::from(exit::IO);
        }
    };

    // Load the source (exercises `aether-source`).
    let mut sources = SourceMap::new();
    let file = sources.add_file(input.display().to_string(), source);
    let (name, bytes, lines) = {
        let loaded = sources.file(file);
        (
            loaded.name().to_string(),
            loaded.len_bytes(),
            loaded.line_count(),
        )
    };

    // Report through the diagnostics pipeline (exercises `aether-diagnostics`).
    let mut handler = DiagnosticHandler::new();
    handler.emit(
        Diagnostic::error("the Aether compilation pipeline is not yet implemented")
            .with_note(format!("loaded '{name}' ({bytes} bytes, {lines} lines)"))
            .with_note("frontend stages (lexer, parser, ...) land in Phase 1; see ROADMAP.md"),
    );

    for diagnostic in handler.diagnostics() {
        eprintln!("{}", render(diagnostic, &sources));
    }

    ExitCode::from(exit::UNIMPLEMENTED)
}

/// The program's version string, e.g. `aetherc 0.0.0`.
fn version_string() -> String {
    format!("{PROG} {}", env!("CARGO_PKG_VERSION"))
}

/// A one-line usage summary.
fn usage() -> String {
    format!("usage: {PROG} [OPTIONS] <INPUT>")
}

/// Full help text shown for `--help`.
fn help_string() -> String {
    format!(
        "{version}\n\
         Aether compiler driver.\n\
         \n\
         {usage}\n\
         \n\
         Arguments:\n  \
         <INPUT>          Path to an Aether source file (.ae) to compile\n\
         \n\
         Options:\n  \
         -h, --help       Print this help message and exit\n  \
         -V, --version    Print version information and exit",
        version = version_string(),
        usage = usage(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn parses_help_flags() {
        assert_eq!(parse(&args(&["--help"])), Ok(Command::Help));
        assert_eq!(parse(&args(&["-h"])), Ok(Command::Help));
    }

    #[test]
    fn parses_version_flags() {
        assert_eq!(parse(&args(&["--version"])), Ok(Command::Version));
        assert_eq!(parse(&args(&["-V"])), Ok(Command::Version));
    }

    #[test]
    fn parses_input_file() {
        assert_eq!(
            parse(&args(&["main.ae"])),
            Ok(Command::Compile {
                input: PathBuf::from("main.ae")
            })
        );
    }

    #[test]
    fn help_takes_precedence_over_input() {
        assert_eq!(parse(&args(&["main.ae", "--help"])), Ok(Command::Help));
    }

    #[test]
    fn rejects_unknown_option() {
        assert_eq!(
            parse(&args(&["--nope"])),
            Err(ParseError::UnknownOption("--nope".to_string()))
        );
    }

    #[test]
    fn rejects_missing_input() {
        assert_eq!(parse(&[]), Err(ParseError::MissingInput));
    }

    #[test]
    fn rejects_extra_positional() {
        assert_eq!(
            parse(&args(&["a.ae", "b.ae"])),
            Err(ParseError::UnexpectedArgument("b.ae".to_string()))
        );
    }
}
