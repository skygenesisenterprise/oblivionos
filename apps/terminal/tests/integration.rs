//! End-to-end smoke test for `oblivion-terminal`.
//!
//! These tests spawn the actual compiled binary as a subprocess and verify
//! the on-disk behavior using the standard `CARGO_BIN_EXE_<name>` env var,
//! which cargo sets automatically for integration tests of the same package.

use std::io::Write;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_oblivion-terminal");

#[test]
fn cli_runs_command_and_strips_ansi() {
    let child = Command::new(BIN)
        .args(["--command", "echo hello-oblivion"])
        .arg("--no-color")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn oblivion-terminal");

    let out = child
        .wait_with_output()
        .expect("wait_with_output");
    assert!(
        out.status.success(),
        "oblivion-terminal exited non-zero: {:?}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("hello-oblivion"),
        "--command output did not contain expected text: {:?}",
        stdout,
    );
    assert!(
        !stdout.contains('\x1b'),
        "--no-color output still contained ESC: {:?}",
        stdout,
    );
}

#[test]
fn repl_quits_on_exit_command() {
    let mut child = Command::new(BIN)
        .args(["--repl"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn oblivion-terminal");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(b":help\n:exit\n")
            .expect("write to stdin");
    }
    let out = child.wait_with_output().expect("wait_with_output");
    assert!(
        out.status.success(),
        "repl exited non-zero: {:?}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("echo"),
        "repl help did not mention `echo`: {:?}",
        stdout
    );
    assert!(
        stdout.contains("bye"),
        "repl did not print bye on :exit: {:?}",
        stdout
    );
}

#[test]
fn repl_handles_echo_builtin() {
    let mut child = Command::new(BIN)
        .args(["--repl"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn oblivion-terminal");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(b"echo from-the-builtins\n:exit\n")
            .expect("write to stdin");
    }
    let out = child.wait_with_output().expect("wait_with_output");
    assert!(out.status.success(), "repl exited non-zero: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("from-the-builtins"),
        "repl did not echo back builtin result: {:?}",
        stdout
    );
}
