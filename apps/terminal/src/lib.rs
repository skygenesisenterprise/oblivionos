//! `oblivion-terminal` — terminal utilities for OblivionOS.
//!
//! Provides:
//! * [`Pty`] — a portable-pty-backed pseudo-terminal session.
//! * [`Terminal`] — high-level runtime that wires stdin/stdout to either a
//!   [`Pty`] running an external shell, or a built-in [`crate::command`] REPL.
//!
//! This is the public API surface. The CLI consumes it from `main.rs`.

pub mod command;
pub mod parser;
pub mod pty;

pub use command::{CommandOutput, evaluate as evaluate_builtin};
pub use parser::strip_ansi;
pub use pty::Pty;

use std::io::{BufRead, Write};
use std::path::PathBuf;

/// What the terminal should do when [`Terminal::run`] is called.
#[derive(Debug, Clone)]
pub enum Mode {
    /// Spawn `shell`, write `command`, drain everything the pty produces, and
    /// forward that into the caller's writer exactly once. Stdin is not touched.
    Command { shell: String, command: String },
    /// Read lines from the caller's reader, evaluate each line through the
    /// built-in command set, and write responses into the caller's writer
    /// until the user types `:exit`/`:quit` (or EOF on stdin).
    Repl,
}

/// All knobs the caller may want to tune before creating a [`Terminal`].
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    pub mode: Mode,
    pub cwd: PathBuf,
    pub cols: u16,
    pub rows: u16,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            mode: Mode::Repl,
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            cols: 80,
            rows: 24,
        }
    }
}

/// A `Terminal` orchestrates read/write between user-facing I/O, an optional
/// pseudo-terminal, and a built-in REPL.
///
/// Construct with [`Terminal::new`], then call [`Terminal::run`]. The
/// `input` and `output` arguments are user-replaceable so tests can pin
/// stdin/stdout without touching the process.
pub struct Terminal {
    cfg: TerminalConfig,
}

impl Terminal {
    pub fn new(cfg: TerminalConfig) -> Self {
        Self { cfg }
    }

    pub fn config(&self) -> &TerminalConfig {
        &self.cfg
    }

    /// Run the configured mode against the provided I/O pair.
    pub fn run<R: BufRead, W: Write>(&self, input: &mut R, output: &mut W) -> anyhow::Result<()> {
        match &self.cfg.mode {
            Mode::Command { shell, command } => self.run_command(shell, command, output),
            Mode::Repl => self.run_repl(input, output),
        }
    }

    fn run_command<W: Write>(
        &self,
        shell: &str,
        command: &str,
        output: &mut W,
    ) -> anyhow::Result<()> {
        let mut pty = Pty::spawn(shell, self.cfg.cols, self.cfg.rows)?;
        tracing::info!(shell, command, "running through pty");
        pty.write_all(format!("{}\n", command).as_bytes())?;
        let captured = pty.drain_to_end()?;
        output.write_all(&captured)?;
        Ok(())
    }

    fn run_repl<R: BufRead, W: Write>(
        &self,
        input: &mut R,
        output: &mut W,
    ) -> anyhow::Result<()> {
        writeln!(output, "oblivion-terminal v{}", env!("CARGO_PKG_VERSION"))?;
        writeln!(output, "type `:help` for the command list, `:exit` to quit")?;

        let mut buf = String::new();
        loop {
            write!(output, "obl> ")?;
            output.flush()?;
            buf.clear();
            let n = input.read_line(&mut buf)?;
            if n == 0 {
                // EOF on stdin — graceful shutdown.
                writeln!(output)?;
                return Ok(());
            }
            let trimmed = buf.trim_end_matches(['\n', '\r']);
            if trimmed.is_empty() {
                continue;
            }
            match command::evaluate(trimmed, &self.cfg.cwd)? {
                CommandOutput { stdout, stderr, exit } => {
                    if !stdout.is_empty() {
                        output.write_all(&stdout)?;
                    }
                    if !stderr.is_empty() {
                        output.write_all(&stderr)?;
                    }
                    if let Some(code) = exit {
                        if code != 0 {
                            writeln!(output, "[exit {}]", code)?;
                        }
                        if matches!(trimmed, ":exit" | ":quit") {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}
