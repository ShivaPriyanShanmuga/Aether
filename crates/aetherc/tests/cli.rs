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
