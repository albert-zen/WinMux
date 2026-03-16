//! Minimal PTY host wrapping `portable-pty`.
//!
//! Synchronous-friendly; intentionally decoupled from Tauri / IPC.
//! Spawn a command, write input, collect output, resize, query exit status.

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize as RawSize, PtySystem};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

// ── Public types ─────────────────────────────────────────────────────────────

/// Terminal window dimensions.
#[derive(Debug, Clone, Copy)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

/// Errors produced by [`PtyHost`].
#[derive(Debug)]
pub enum PtyError {
    /// Failed to open the PTY or spawn the child process.
    Spawn(String),
    /// An I/O error occurred (write, wait, …).
    Io(String),
    /// The PTY resize call failed.
    Resize(String),
}

impl std::fmt::Display for PtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PtyError::Spawn(s) => write!(f, "pty spawn: {s}"),
            PtyError::Io(s) => write!(f, "pty io: {s}"),
            PtyError::Resize(s) => write!(f, "pty resize: {s}"),
        }
    }
}

impl std::error::Error for PtyError {}

// ── PtyHost ───────────────────────────────────────────────────────────────────

/// A running process inside a pseudo-terminal.
///
/// A background thread drains PTY output into an internal buffer for
/// [`PtyHost::collected_output`].  All other operations are synchronous.
pub struct PtyHost {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    output: Arc<Mutex<Vec<u8>>>,
}

impl PtyHost {
    /// Spawn `command` (with `args`) inside a PTY of the given `size`.
    ///
    /// Returns immediately after the child process is created.  A detached
    /// thread starts reading PTY output in the background.
    pub fn spawn(command: &str, args: &[&str], size: PtySize) -> Result<Self, PtyError> {
        let pty_system: Box<dyn PtySystem> = native_pty_system();

        let pair = pty_system
            .openpty(RawSize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        // Build the command.
        let mut cmd = CommandBuilder::new(command);
        for arg in args {
            cmd.arg(arg);
        }

        // Spawn – slave is only needed for this call; master is kept for I/O.
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        // Obtain write and read handles from the master side.
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        // Background thread: drain PTY bytes into a shared buffer.
        let output: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let output_bg = Arc::clone(&output);

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        output_bg.lock().unwrap().extend_from_slice(&buf[..n]);
                    }
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer,
            child,
            output,
        })
    }

    /// Write raw bytes into the PTY (simulates keyboard input).
    pub fn write_input(&mut self, data: &[u8]) -> Result<(), PtyError> {
        self.writer
            .write_all(data)
            .map_err(|e| PtyError::Io(e.to_string()))
    }

    /// Resize the PTY window.
    pub fn resize(&self, size: PtySize) -> Result<(), PtyError> {
        self.master
            .resize(RawSize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Resize(e.to_string()))
    }

    /// Non-blocking child-exit query.
    ///
    /// Returns `Some(true)` on clean exit, `Some(false)` on non-zero exit,
    /// or `None` when the child is still running.
    pub fn try_wait(&mut self) -> Result<Option<bool>, PtyError> {
        self.child
            .try_wait()
            .map(|opt| opt.map(|s| s.success()))
            .map_err(|e| PtyError::Io(e.to_string()))
    }

    /// Returns a snapshot of all bytes collected from the PTY so far.
    pub fn collected_output(&self) -> Vec<u8> {
        self.output.lock().unwrap().clone()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn default_size() -> PtySize {
        PtySize { rows: 24, cols: 80 }
    }

    /// Spawning a command that does not exist must return an error immediately.
    #[test]
    fn spawn_invalid_command_returns_error() {
        let result = PtyHost::spawn("__nonexistent_cmux_binary__", &[], default_size());
        assert!(
            result.is_err(),
            "expected an error when spawning a non-existent command"
        );
    }

    /// Smoke test: spawn `cmd.exe /C echo`, wait for exit, verify output.
    #[test]
    #[cfg(target_os = "windows")]
    fn smoke_capture_output_from_echo() {
        let mut host = PtyHost::spawn(
            "cmd.exe",
            &["/C", "echo", "hello_cmux"],
            default_size(),
        )
        .expect("failed to spawn cmd.exe");

        // Wait up to 5 s for the child to exit.
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match host.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {}
                Err(e) => panic!("try_wait error: {e}"),
            }
            assert!(Instant::now() < deadline, "child did not exit within 5 s");
            thread::sleep(Duration::from_millis(50));
        }

        let output_deadline = Instant::now() + Duration::from_secs(2);
        let text = loop {
            let raw = host.collected_output();
            let text = String::from_utf8_lossy(&raw).to_string();
            if text.contains("hello_cmux") {
                break text;
            }

            assert!(
                Instant::now() < output_deadline,
                "expected 'hello_cmux' in PTY output, got: {text:?}"
            );
            thread::sleep(Duration::from_millis(25));
        };

        assert!(
            text.contains("hello_cmux"),
            "expected 'hello_cmux' in PTY output, got: {text:?}"
        );
    }

    /// Resize must not panic; it is valid to call it any time after spawn.
    #[test]
    #[cfg(target_os = "windows")]
    fn resize_after_spawn() {
        let host = PtyHost::spawn("cmd.exe", &["/C", "echo", "resize_ok"], default_size())
            .expect("failed to spawn cmd.exe");

        // The process may already have exited; resize may succeed or fail –
        // what matters is no panic.
        let _ = host.resize(PtySize { rows: 40, cols: 120 });
    }

    /// write_input must not panic on an alive process.
    #[test]
    #[cfg(target_os = "windows")]
    fn write_input_does_not_panic() {
        let mut host = PtyHost::spawn("cmd.exe", &[], default_size())
            .expect("failed to spawn cmd.exe");

        host.write_input(b"exit\r\n").expect("write_input failed");

        // Give the shell time to process the exit command.
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(Some(_)) = host.try_wait() {
                break;
            }
            assert!(Instant::now() < deadline, "shell did not exit after 'exit'");
            thread::sleep(Duration::from_millis(50));
        }
    }
}
