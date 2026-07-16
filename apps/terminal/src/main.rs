//! `oblivion-terminal` CLI entrypoint.
//!
//! Argparse is handled by `clap`. After parsing we build a `TerminalConfig`,
//! instantiate `Terminal`, and delegate to `Terminal::run`. `--no-color` is
//! honored only in `--command` mode (it has no effect on interactive REPLs).

use std::io::{stdout, Write};

use anyhow::{Context, Result};
use clap::Parser;
use oblivion_terminal::{Mode, Terminal, TerminalConfig};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "oblivion-terminal",
    version,
    about = "Minimal terminal and REPL for OblivionOS"
)]
struct Cli {
    /// Path to the shell used by `--command`. Defaults to /bin/sh.
    #[arg(long, short = 's', default_value = "/bin/sh", env = "SHELL")]
    shell: String,

    /// Run a single command through `--shell` and exit.
    #[arg(long, short = 'c')]
    command: Option<String>,

    /// Run the built-in REPL instead of an external shell.
    #[arg(long, short = 'r')]
    repl: bool,

    /// Strip ANSI escape sequences from `--command` output.
    #[arg(long)]
    no_color: bool,

    /// Terminal columns for the pty (used in `--command` mode).
    #[arg(long, default_value_t = 80)]
    cols: u16,

    /// Terminal rows for the pty (used in `--command` mode).
    #[arg(long, default_value_t = 24)]
    rows: u16,

    /// Verbosity (`-v`, `-vv`, `-vvv`).
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = match cli.verbose {
        0 => "oblivion_terminal=warn",
        1 => "oblivion_terminal=info",
        2 => "oblivion_terminal=debug",
        _ => "oblivion_terminal=trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_writer(std::io::stderr)
        .init();

    let mode = if let Some(cmd) = cli.command.clone() {
        Mode::Command {
            shell: cli.shell.clone(),
            command: cmd,
        }
    } else if cli.repl {
        Mode::Repl
    } else {
        // No mode flag and no command → print help and exit cleanly.
        use clap::CommandFactory;
        let mut cmd = Cli::command();
        cmd.print_help().ok();
        println!();
        return Ok(());
    };

    let cfg = TerminalConfig {
        mode,
        cwd: std::env::current_dir().unwrap_or_default(),
        cols: cli.cols,
        rows: cli.rows,
    };
    let terminal = Terminal::new(cfg);

    let mut input = std::io::stdin().lock();
    let mut output = stdout().lock();

    if cli.no_color && matches!(terminal.config().mode, Mode::Command { .. }) {
        // Capture, strip, then write. Avoids touching the writer's escape
        // state mid-stream which would cause TTYs to render unpredictably.
        let mut buf = Vec::new();
        terminal.run(&mut input, &mut buf)?;
        let stripped = oblivion_terminal::strip_ansi(&buf);
        output.write_all(&stripped).context("write stdout")?;
        output.flush().ok();
        return Ok(());
    }

    terminal.run(&mut input, &mut output)
}
