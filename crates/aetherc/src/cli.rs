//! Argument parsing and command dispatch for the `aetherc` driver.
//!
//! The CLI is intentionally small and dependency-free while its surface area is
//! minimal. When that surface grows (multiple subcommands, many flags), we will
//! migrate to `clap`; see `TECH_DEBT.md`. Parsing is separated from execution so
//! it can be unit-tested in isolation, while `tests/cli.rs` exercises the built
//! binary end to end.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aether_air_interp::RunError;
use aether_diagnostics::{Diagnostic, DiagnosticHandler, render};
use aether_lexer::{LexResult, Token, tokenize};
use aether_parser::ParseResult;
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
    /// Compilation ran but the source contained errors.
    pub(super) const COMPILE_ERROR: u8 = 1;
    /// The command line was malformed (unknown flag, missing/extra argument).
    pub(super) const USAGE: u8 = 2;
    /// A runtime error occurred while executing the program (mirrors
    /// `EX_SOFTWARE` from `sysexits.h`).
    pub(super) const RUNTIME_ERROR: u8 = 70;
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
        /// Whether `--dump-tokens` was given: print the token stream.
        dump_tokens: bool,
        /// Whether `--dump-ast` was given: print the parsed AST.
        dump_ast: bool,
        /// Whether `--dump-air` was given: print the lowered AIR.
        dump_air: bool,
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
    let mut dump_tokens = false;
    let mut dump_ast = false;
    let mut dump_air = false;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--dump-tokens" => dump_tokens = true,
            "--dump-ast" => dump_ast = true,
            "--dump-air" => dump_air = true,
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
        Some(input) => Ok(Command::Compile {
            input,
            dump_tokens,
            dump_ast,
            dump_air,
        }),
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
        Command::Compile {
            input,
            dump_tokens,
            dump_ast,
            dump_air,
        } => compile(&input, dump_tokens, dump_ast, dump_air),
    }
}

/// Handle the `compile` command.
///
/// Runs the full pipeline — lex, parse, lower to AIR, verify, and interpret —
/// over the input, reporting diagnostics through the real renderer. Each
/// `--dump-*` flag stops the pipeline after its phase and prints that phase's
/// output; otherwise the program is executed and `main`'s result is printed.
fn compile(input: &Path, dump_tokens: bool, dump_ast: bool, dump_air: bool) -> ExitCode {
    let source = match std::fs::read_to_string(input) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("{PROG}: error: cannot read '{}': {err}", input.display());
            return ExitCode::from(exit::IO);
        }
    };

    let mut sources = SourceMap::new();
    let file = sources.add_file(input.display().to_string(), source);
    let mut handler = DiagnosticHandler::new();

    // Lexical analysis.
    let LexResult {
        tokens,
        diagnostics,
    } = tokenize(sources.file(file));
    for diagnostic in diagnostics {
        handler.emit(diagnostic);
    }
    if dump_tokens {
        dump_tokens_to_stdout(&tokens, &sources);
        return finish(&handler, &sources, dump_code(&handler));
    }
    if handler.has_errors() {
        return finish(&handler, &sources, exit::COMPILE_ERROR);
    }

    // Syntactic analysis.
    let ParseResult {
        program,
        diagnostics,
    } = aether_parser::parse(sources.file(file), &tokens);
    for diagnostic in diagnostics {
        handler.emit(diagnostic);
    }
    if dump_ast {
        println!("{}", aether_ast::pretty::print(&program));
        return finish(&handler, &sources, dump_code(&handler));
    }
    if handler.has_errors() {
        return finish(&handler, &sources, exit::COMPILE_ERROR);
    }

    // Lowering to AIR, then structural verification.
    let module = aether_lower::lower(&program);
    for error in aether_air::verify(&module) {
        handler.emit(Diagnostic::error(format!(
            "AIR verification failed: {}",
            error.message
        )));
    }
    if dump_air {
        println!("{}", aether_air::print(&module));
        return finish(&handler, &sources, dump_code(&handler));
    }
    if handler.has_errors() {
        return finish(&handler, &sources, exit::COMPILE_ERROR);
    }

    // Execute `main` and print its result.
    match aether_air_interp::interpret(&module) {
        Ok(result) => {
            println!("{result}");
            finish(&handler, &sources, exit::SUCCESS)
        }
        Err(RunError::NoEntryPoint) => {
            handler.emit(Diagnostic::error("no `main` function to execute"));
            finish(&handler, &sources, exit::COMPILE_ERROR)
        }
        Err(RunError::DivisionByZero { span }) => {
            handler.emit(
                Diagnostic::error("division by zero")
                    .with_primary(span, "the divisor evaluates to zero"),
            );
            finish(&handler, &sources, exit::RUNTIME_ERROR)
        }
    }
}

/// Render all buffered diagnostics to stderr and return `code` as an exit code.
fn finish(handler: &DiagnosticHandler, sources: &SourceMap, code: u8) -> ExitCode {
    for diagnostic in handler.diagnostics() {
        eprintln!("{}", render(diagnostic, sources));
    }
    ExitCode::from(code)
}

/// The exit code for a `--dump-*` request: success unless errors were reported.
fn dump_code(handler: &DiagnosticHandler) -> u8 {
    if handler.has_errors() {
        exit::COMPILE_ERROR
    } else {
        exit::SUCCESS
    }
}

/// Print the token stream to stdout, one token per line as `KIND lo..hi "text"`.
fn dump_tokens_to_stdout(tokens: &[Token], sources: &SourceMap) {
    for token in tokens {
        println!(
            "{kind:<12} {lo:>4}..{hi:<4} {text:?}",
            kind = format!("{:?}", token.kind),
            lo = token.span.lo().to_usize(),
            hi = token.span.hi().to_usize(),
            text = sources.span_text(token.span),
        );
    }
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
         Aether compiler driver. Compiles and runs an Aether source file,\n\
         printing the value returned by `main`.\n\
         \n\
         {usage}\n\
         \n\
         Arguments:\n  \
         <INPUT>          Path to an Aether source file (.ae) to compile\n\
         \n\
         Options:\n  \
         --dump-tokens    Lex the input, print the token stream, then exit\n  \
         --dump-ast       Parse the input, print the AST, then exit\n  \
         --dump-air       Lower the input, print the AIR, then exit\n  \
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
                input: PathBuf::from("main.ae"),
                dump_tokens: false,
                dump_ast: false,
                dump_air: false,
            })
        );
    }

    #[test]
    fn parses_dump_tokens_flag() {
        assert_eq!(
            parse(&args(&["--dump-tokens", "main.ae"])),
            Ok(Command::Compile {
                input: PathBuf::from("main.ae"),
                dump_tokens: true,
                dump_ast: false,
                dump_air: false,
            })
        );
    }

    #[test]
    fn parses_dump_ast_flag() {
        assert_eq!(
            parse(&args(&["--dump-ast", "main.ae"])),
            Ok(Command::Compile {
                input: PathBuf::from("main.ae"),
                dump_tokens: false,
                dump_ast: true,
                dump_air: false,
            })
        );
    }

    #[test]
    fn parses_dump_air_flag() {
        assert_eq!(
            parse(&args(&["--dump-air", "main.ae"])),
            Ok(Command::Compile {
                input: PathBuf::from("main.ae"),
                dump_tokens: false,
                dump_ast: false,
                dump_air: true,
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
