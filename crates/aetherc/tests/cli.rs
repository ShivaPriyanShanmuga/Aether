//! Integration tests that exercise the built `aetherc` binary end to end.
//!
//! These run the real executable (via `CARGO_BIN_EXE_aetherc`, injected by
//! Cargo) as a subprocess and assert on its exit code and output, giving us
//! confidence in the whole driver, not just the parsing logic unit-tested in
//! `src/cli.rs`.

use std::path::PathBuf;
use std::process::Command;

/// A `Command` pointing at the freshly built `aetherc` binary.
fn aetherc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aetherc"))
}

#[test]
fn version_flag_prints_version_and_succeeds() {
    let output = aetherc()
        .arg("--version")
        .output()
        .expect("failed to run aetherc");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("aetherc"), "stdout was: {stdout}");
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "stdout was: {stdout}"
    );
}

#[test]
fn help_flag_succeeds_and_prints_usage() {
    let output = aetherc()
        .arg("--help")
        .output()
        .expect("failed to run aetherc");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("usage:"), "stdout was: {stdout}");
}

#[test]
fn unknown_option_is_a_usage_error() {
    let output = aetherc()
        .arg("--definitely-not-a-flag")
        .output()
        .expect("failed to run aetherc");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn missing_input_is_a_usage_error() {
    let output = aetherc().output().expect("failed to run aetherc");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn compiling_a_file_reports_unimplemented() {
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("aetherc_it_{}.ae", std::process::id()));
    std::fs::write(&path, b"fn main() -> int { return 0; }\n").expect("write temp source");

    let output = aetherc()
        .arg(&path)
        .output()
        .expect("failed to run aetherc");
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        output.status.code(),
        Some(3),
        "expected UNIMPLEMENTED exit code"
    );
}

#[test]
fn missing_file_is_an_io_error() {
    let output = aetherc()
        .arg("this_file_should_not_exist_12345.ae")
        .output()
        .expect("failed to run aetherc");
    assert_eq!(output.status.code(), Some(74));
}

#[test]
fn dump_tokens_prints_stream_and_succeeds() {
    // A distinct filename prefix avoids collisions with other temp-file tests,
    // which run as parallel threads sharing one process id.
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("aetherc_it_dump_{}.ae", std::process::id()));
    std::fs::write(&path, b"fn main() { return 42; }\n").expect("write temp source");

    let output = aetherc()
        .arg("--dump-tokens")
        .arg(&path)
        .output()
        .expect("failed to run aetherc");
    let _ = std::fs::remove_file(&path);

    assert!(output.status.success(), "expected success exit code");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Fn"), "stdout was:\n{stdout}");
    assert!(stdout.contains("Ident"), "stdout was:\n{stdout}");
    assert!(stdout.contains("Eof"), "stdout was:\n{stdout}");
}

#[test]
fn lexical_error_is_a_compile_error() {
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("aetherc_it_lexerr_{}.ae", std::process::id()));
    // `@` is not a valid token.
    std::fs::write(&path, b"fn main() { @ }\n").expect("write temp source");

    let output = aetherc()
        .arg(&path)
        .output()
        .expect("failed to run aetherc");
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected COMPILE_ERROR exit code"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected character"),
        "stderr was:\n{stderr}"
    );
}

#[test]
fn dump_ast_prints_tree_and_succeeds() {
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("aetherc_it_ast_{}.ae", std::process::id()));
    std::fs::write(&path, b"fn main() -> int { return 1 + 2; }\n").expect("write temp source");

    let output = aetherc()
        .arg("--dump-ast")
        .arg(&path)
        .output()
        .expect("failed to run aetherc");
    let _ = std::fs::remove_file(&path);

    assert!(output.status.success(), "expected success exit code");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Fn \"main\" -> \"int\""),
        "stdout was:\n{stdout}"
    );
    assert!(stdout.contains("Binary +"), "stdout was:\n{stdout}");
}

#[test]
fn syntax_error_is_a_compile_error() {
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("aetherc_it_synerr_{}.ae", std::process::id()));
    // Missing semicolon after the return expression.
    std::fs::write(&path, b"fn main() -> int { return 1 }\n").expect("write temp source");

    let output = aetherc()
        .arg(&path)
        .output()
        .expect("failed to run aetherc");
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected COMPILE_ERROR exit code"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("expected `;`"), "stderr was:\n{stderr}");
}

#[test]
fn dump_air_prints_ir_and_succeeds() {
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("aetherc_it_air_{}.ae", std::process::id()));
    std::fs::write(&path, b"fn main() -> int { return 1 + 2 * 3; }\n").expect("write temp source");

    let output = aetherc()
        .arg("--dump-air")
        .arg(&path)
        .output()
        .expect("failed to run aetherc");
    let _ = std::fs::remove_file(&path);

    assert!(output.status.success(), "expected success exit code");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fn main() -> int {"),
        "stdout was:\n{stdout}"
    );
    assert!(stdout.contains("ret %"), "stdout was:\n{stdout}");
    assert!(stdout.contains("mul %"), "stdout was:\n{stdout}");
}

#[test]
fn missing_return_fails_verification() {
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("aetherc_it_noret_{}.ae", std::process::id()));
    // A function that never returns lowers to an unterminated block.
    std::fs::write(&path, b"fn main() -> int { }\n").expect("write temp source");

    let output = aetherc()
        .arg(&path)
        .output()
        .expect("failed to run aetherc");
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected COMPILE_ERROR exit code"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no terminator"), "stderr was:\n{stderr}");
}
