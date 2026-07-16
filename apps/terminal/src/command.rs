//! Built-in REPL commands for the `:repl` mode of `oblivion-terminal`.
//!
//! These are intentionally minimal coreutils-style commands — enough to do
//! useful interactive testing without spawning an external shell.
//! Special "meta" commands (`:exit`, `:help`, `:version`) are also handled
//! here.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};

/// Output of evaluating one REPL line.
#[derive(Debug, Default, Clone)]
pub struct CommandOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit: Option<i32>,
}

impl CommandOutput {
    pub fn ok<S: Into<Vec<u8>>>(stdout: S) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: Vec::new(),
            exit: Some(0),
        }
    }

    pub fn err<S: Into<Vec<u8>>>(stderr: S, code: i32) -> Self {
        Self {
            stdout: Vec::new(),
            stderr: stderr.into(),
            exit: Some(code),
        }
    }
}

/// Evaluate a single REPL line and return the user-facing output block.
///
/// `line` is whatever was typed after the prompt (no trailing newline).
/// `cwd` is the directory the terminal was launched from.
pub fn evaluate(line: &str, cwd: &Path) -> Result<CommandOutput> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix(':') {
        return match rest {
            "exit" | "quit" => Ok(CommandOutput {
                stdout: b"bye\n".to_vec(),
                stderr: Vec::new(),
                exit: Some(0),
            }),
            "help" | "?" => Ok(CommandOutput::ok(HELP)),
            "version" | "v" => Ok(CommandOutput::ok(format!(
                "oblivion-terminal {}\n",
                env!("CARGO_PKG_VERSION")
            ))),
            cmd => Err(anyhow!(
                "unknown built-in: `:{}` (type `:help` for the list)",
                cmd
            )),
        };
    }

    let mut parts = line.split_whitespace();
    let Some(cmd) = parts.next() else {
        return Ok(CommandOutput::ok(""));
    };
    let args: Vec<&str> = parts.collect();

    match cmd {
        "echo" => Ok(CommandOutput::ok(format!("{}\n", args.join(" ")))),

        "pwd" => {
            let canonical = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
            Ok(CommandOutput::ok(format!("{}\n", canonical.display())))
        }

        "cd" => {
            // The REPL cannot change the host process's CWD from a library,
            // so we report what is going on rather than pretending. We emit
            // this on stderr so users notice it's not a real shell `cd`.
            if args.is_empty() {
                return Ok(CommandOutput::ok(format!("{}\n", cwd.display())));
            }
            Ok(CommandOutput::err(
                b"obl: cd is informational in oblivion-terminal's built-in REPL; spawn /bin/sh for a real shell\n"
                    .to_vec(),
                1,
            ))
        }

        "ls" => {
            let dir = if args.is_empty() { cwd } else { Path::new(args[0]) };
            let mut out = Vec::new();
            let entries = fs::read_dir(dir)
                .with_context(|| format!("ls: cannot open `{}`", dir.display()))?;
            for entry in entries {
                let entry = entry
                    .with_context(|| format!("ls: error reading `{}`", dir.display()))?;
                let name = entry.file_name();
                out.extend_from_slice(name.as_encoded_bytes());
                out.push(b'\n');
            }
            Ok(CommandOutput::ok(out))
        }

        "cat" => {
            if args.is_empty() {
                return Ok(CommandOutput::err(b"cat: missing operand\n".to_vec(), 1));
            }
            let mut out = Vec::new();
            for arg in &args {
                let bytes = fs::read(arg).with_context(|| format!("cat: `{}`", arg))?;
                out.extend_from_slice(&bytes);
            }
            Ok(CommandOutput::ok(out))
        }

        "date" => {
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Ok(CommandOutput::ok(format!("{}\n", secs)))
        }

        "env" => {
            let mut out = String::new();
            for (k, v) in std::env::vars() {
                out.push_str(&k);
                out.push('=');
                out.push_str(&v);
                out.push('\n');
            }
            Ok(CommandOutput::ok(out))
        }

        "whoami" => match std::env::var("USER") {
            Ok(u) => Ok(CommandOutput::ok(format!("{}\n", u))),
            Err(_) => match Command::new("whoami").output() {
                Ok(o) => Ok(CommandOutput::ok(o.stdout)),
                Err(_) => Ok(CommandOutput::err(b"whoami: not available\n".to_vec(), 1)),
            },
        },

        "uname" => match Command::new("uname").args(&args).output() {
            Ok(o) if !o.stdout.is_empty() => Ok(CommandOutput::ok(o.stdout)),
            _ => Ok(CommandOutput::ok(b"OblivionOS\n".to_vec())),
        },

        "history" => Ok(CommandOutput::ok(
            "(inline REPL: no command history persistence in v0.1.0)\n",
        )),

        "clear" => Ok(CommandOutput::ok("\x1b[2J\x1b[H")),

        _ => Ok(CommandOutput::err(
            format!(
                "obl: command not found: `{}` (try `:help` for built-ins or use a real shell)\n",
                cmd
            ),
            127,
        )),
    }
}

const HELP: &str = "\
built-in commands (run in `--repl` mode or interactively):
  echo <text>   print arguments
  pwd           print current working directory
  ls [path]     list entries in a directory (defaults to cwd)
  cat <files>   print file contents
  date          print seconds since UNIX epoch
  env           print environment variables
  whoami        print the current user
  uname         print the operating system name
  clear         emit a clear-screen ANSI sequence (no effect on plain stdout)
  :help / :?    this list
  :version / :v print the version
  :exit / :quit leave the REPL
";

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn echo_returns_input() {
        let cwd = env::temp_dir();
        let out = evaluate("echo hello world", &cwd).unwrap();
        assert_eq!(out.stdout, b"hello world\n");
        assert_eq!(out.exit, Some(0));
    }

    #[test]
    fn echo_handles_trailing_whitespace() {
        let cwd = env::temp_dir();
        let out = evaluate("echo abc   ", &cwd).unwrap();
        assert_eq!(out.stdout, b"abc\n");
    }

    #[test]
    fn pwd_returns_existing_dir() {
        let cwd = env::temp_dir();
        let out = evaluate("pwd", &cwd).unwrap();
        assert!(!out.stdout.is_empty(), "pwd should produce output");
        assert!(out.stdout.ends_with(b"\n"));
    }

    #[test]
    fn ls_lists_current_dir() {
        // temp_dir always exists and is readable.
        let cwd = env::temp_dir();
        let out = evaluate("ls", &cwd).unwrap();
        assert_eq!(out.exit, Some(0));
    }

    #[test]
    fn unknown_command_returns_127() {
        let cwd = env::temp_dir();
        let out = evaluate("nonsense-cmd", &cwd).unwrap();
        assert_eq!(out.exit, Some(127));
        assert!(!out.stderr.is_empty());
    }

    #[test]
    fn help_mentions_echo() {
        let cwd = env::temp_dir();
        let out = evaluate(":help", &cwd).unwrap();
        assert!(
            out.stdout.windows(b"echo".len()).any(|w| w == b"echo"),
            "help output should mention `echo`"
        );
    }

    #[test]
    fn version_returns_pkg_version() {
        let cwd = env::temp_dir();
        let out = evaluate(":version", &cwd).unwrap();
        assert!(out.stdout.starts_with(b"oblivion-terminal "));
    }

    #[test]
    fn cat_reads_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        std::fs::write(&path, b"sample contents\n").unwrap();
        let cwd = env::temp_dir();
        let out = evaluate(&format!("cat {}", path.display()), &cwd).unwrap();
        assert_eq!(out.stdout, b"sample contents\n");
    }

    #[test]
    fn cat_on_missing_file_returns_err() {
        let cwd = env::temp_dir();
        let out = evaluate("cat /no/such/path/oblivion-test", &cwd);
        // Result::Err from the anyhow context — we don't catch it.
        assert!(out.is_err());
    }

    #[test]
    fn empty_line_is_noop() {
        let cwd = env::temp_dir();
        let out = evaluate("", &cwd).unwrap();
        assert_eq!(out.exit, Some(0));
        assert!(out.stdout.is_empty());
    }
}
