//! PTY-backed pseudo-terminal session built on `portable-pty`.
//!
//! A `Pty` owns the master fd, the spawned child, and a child-killer.
//! The destructor is responsible for cleanup: it always sends
//! `SIGKILL` to the child (best-effort) and then `wait`s on it so the
//! process never becomes a zombie, even if `drain_to_end` was never called
//! or was interrupted by a panic / error on the read path.
//!
//! During `Pty::spawn` we additionally wrap the child in an owned
//! [`ChildReaper`] *before* taking the master writer/reader, so that any
//! failure in those calls — or any panic — still reaps the spawned child.

use std::io::{BufReader, Read, Write};

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};

/// A spawned pseudo-terminal session.
///
/// Both the explicit `Drop` impl on `Pty` and the `ChildReaper` used
/// inside [`Pty::spawn`] exist so the child process is always reaped.
pub struct Pty {
    writer: Box<dyn Write + Send>,
    killer: Box<dyn portable_pty::ChildKiller + Send + Sync>,
    reader: Option<Box<dyn Read + Send>>,
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
}

/// RAII guard that owns the spawned child and reaps it on `Drop`.
/// Owned `Option<Box<dyn Child + Send + Sync>>` so the borrow checker
/// has nothing to track — callers can `take()` the value out before the
/// guard's destructor runs.
struct ChildReaper(Option<Box<dyn portable_pty::Child + Send + Sync>>);

impl Drop for ChildReaper {
    fn drop(&mut self) {
        if let Some(mut c) = self.0.take() {
            // Best-effort wait. We deliberately ignore any error here:
            // `Drop` cannot return one and a zombie that survives until
            // process exit is preferable to aborting the program.
            let _ = c.wait();
        }
    }
}

impl Pty {
    /// Spawn `shell` connected to a fresh pty of the requested size.
    pub fn spawn(shell: &str, cols: u16, rows: u16) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("openpty failed")?;

        let mut cmd = CommandBuilder::new(shell);
        // Most shells look at TERM to enable colors / line editing.
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("spawn_command failed")?;
        let killer = child.clone_killer();
        // Closing our copy of the slave fd means EOF on the master read
        // side once the child itself closes its end.
        drop(pair.slave);

        // Wrap the child in a reaper *before* any further fallible call
        // so an error or panic below still waits on the child.
        let mut reaper = ChildReaper(Some(child));

        let writer = pair
            .master
            .take_writer()
            .context("take_writer failed")?;
        let reader = pair
            .master
            .try_clone_reader()
            .context("try_clone_reader failed")?;

        // Hand ownership of the child box to the Pty struct; the reaper
        // becomes a no-op because its `Option` is now `None`.
        Ok(Self {
            writer,
            killer,
            reader: Some(reader),
            child: reaper.0.take(),
        })
    }

    /// Write `buf` to the master side of the pty.
    pub fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.writer
            .write_all(buf)
            .context("pty write failed")?;
        self.writer.flush().ok();
        Ok(())
    }

    /// Read the pty to EOF and return the captured bytes. EOF is observed
    /// once the spawned child closes its end of the slave (typically
    /// because the command finished and the shell exited).
    ///
    /// Performs a blocking drain — not for use on tight-frame paths.
    /// Also reaps the child eagerly so the caller doesn't have to
    /// depend on `Drop` running.
    pub fn drain_to_end(&mut self) -> Result<Vec<u8>> {
        let reader = self
            .reader
            .take()
            .context("drain_to_end called twice")?;
        let mut reader = BufReader::new(reader);
        let mut captured = Vec::with_capacity(4096);
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => captured.extend_from_slice(&chunk[..n]),
                Err(e) => return Err(anyhow::anyhow!("pty read failed: {}", e)),
            }
        }
        if let Some(mut c) = self.child.take() {
            let _ = c.wait();
        }
        Ok(captured)
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        // Always kill (best-effort) and reap. This stops zombies
        // when a caller drops a `Pty` without ever calling
        // `drain_to_end` (or after a panic on the read path).
        let _ = self.killer.kill();
        if let Some(mut c) = self.child.take() {
            let _ = c.wait();
        }
    }
}
