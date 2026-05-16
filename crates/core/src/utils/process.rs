//! Bounded subprocess execution helpers.
//!
//! The Pandoc call sites use a 60 second timeout and a 50 MiB combined cap for
//! captured stdout/stderr. If the combined cap is exceeded, the child is killed.

use std::io::{self, Read};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const PANDOC_TIMEOUT: Duration = Duration::from_secs(60);
pub const PANDOC_OUTPUT_LIMIT_BYTES: usize = 50 * 1024 * 1024;

#[derive(Debug)]
pub struct BoundedOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug)]
pub enum BoundedProcessError {
    Io(io::Error),
    TimedOut { timeout: Duration },
    OutputLimitExceeded { stream: &'static str, limit: usize },
}

impl std::fmt::Display for BoundedProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{}", err),
            Self::TimedOut { timeout } => {
                write!(f, "process timed out after {} seconds", timeout.as_secs())
            }
            Self::OutputLimitExceeded { stream, limit } => {
                write!(
                    f,
                    "combined output exceeded {} byte limit while reading {}",
                    limit, stream
                )
            }
        }
    }
}

impl std::error::Error for BoundedProcessError {}

impl From<io::Error> for BoundedProcessError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamKind {
    Stdout,
    Stderr,
}

#[derive(Debug)]
enum StreamEvent {
    Done(StreamKind),
    Capped(StreamKind),
}

/// Runs a command with captured stdout/stderr, a timeout, and combined byte cap.
///
/// If the timeout or output cap is hit, the child is killed promptly and
/// an explicit error is returned instead of waiting for normal process exit.
pub fn output_bounded(
    command: &mut Command,
    timeout: Duration,
    output_limit_bytes: usize,
) -> Result<BoundedOutput, BoundedProcessError> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("failed to capture stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("failed to capture stderr"))?;

    let (tx, rx) = mpsc::channel();
    let shared_bytes_read = Arc::new(Mutex::new(0usize));
    let stdout_handle = read_stream(
        stdout,
        StreamKind::Stdout,
        output_limit_bytes,
        shared_bytes_read.clone(),
        tx.clone(),
    );
    let stderr_handle = read_stream(
        stderr,
        StreamKind::Stderr,
        output_limit_bytes,
        shared_bytes_read,
        tx,
    );

    let deadline = Instant::now() + timeout;
    let mut stdout_done = false;
    let mut stderr_done = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            join_reader(stdout_handle)?;
            join_reader(stderr_handle)?;
            return Err(BoundedProcessError::TimedOut { timeout });
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        let wait_for = remaining.min(Duration::from_millis(20));
        match rx.recv_timeout(wait_for) {
            Ok(StreamEvent::Done(StreamKind::Stdout)) => stdout_done = true,
            Ok(StreamEvent::Done(StreamKind::Stderr)) => stderr_done = true,
            Ok(StreamEvent::Capped(StreamKind::Stdout)) => {
                let _ = child.kill();
                let _ = child.wait();
                join_reader(stdout_handle)?;
                join_reader(stderr_handle)?;
                return Err(BoundedProcessError::OutputLimitExceeded {
                    stream: "stdout",
                    limit: output_limit_bytes,
                });
            }
            Ok(StreamEvent::Capped(StreamKind::Stderr)) => {
                let _ = child.kill();
                let _ = child.wait();
                join_reader(stdout_handle)?;
                join_reader(stderr_handle)?;
                return Err(BoundedProcessError::OutputLimitExceeded {
                    stream: "stderr",
                    limit: output_limit_bytes,
                });
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {}
        }

        if stdout_done && stderr_done {
            if let Some(status) = child.try_wait()? {
                break status;
            }
        }
    };

    let stdout = join_reader(stdout_handle)?;
    let stderr = join_reader(stderr_handle)?;

    Ok(BoundedOutput {
        status,
        stdout,
        stderr,
    })
}

fn read_stream<R: Read + Send + 'static>(
    mut stream: R,
    kind: StreamKind,
    limit: usize,
    shared_bytes_read: Arc<Mutex<usize>>,
    tx: mpsc::Sender<StreamEvent>,
) -> JoinHandle<io::Result<Vec<u8>>> {
    thread::spawn(move || {
        let mut output = Vec::new();
        let mut buf = [0_u8; 8192];

        loop {
            let read = stream.read(&mut buf)?;
            if read == 0 {
                let _ = tx.send(StreamEvent::Done(kind));
                return Ok(output);
            }

            let allowed = {
                let mut total_read = shared_bytes_read
                    .lock()
                    .map_err(|_| io::Error::other("output cap lock poisoned"))?;
                let remaining = limit.saturating_sub(*total_read);
                let allowed = remaining.min(read);
                *total_read = total_read.saturating_add(allowed);
                allowed
            };

            if allowed < read {
                output.extend_from_slice(&buf[..allowed]);
                let _ = tx.send(StreamEvent::Capped(kind));
                return Ok(output);
            }

            output.extend_from_slice(&buf[..allowed]);
        }
    })
}

fn join_reader(handle: JoinHandle<io::Result<Vec<u8>>>) -> Result<Vec<u8>, BoundedProcessError> {
    handle
        .join()
        .map_err(|_| io::Error::other("process output reader panicked"))?
        .map_err(BoundedProcessError::Io)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn process_helper_captures_success_output() {
        let mut command = Command::new("sh");
        command.arg("-c").arg("printf stdout; printf stderr >&2");

        let output = output_bounded(&mut command, Duration::from_secs(1), 1024).unwrap();

        assert!(output.status.success());
        assert_eq!(output.stdout, b"stdout");
        assert_eq!(output.stderr, b"stderr");
    }

    #[test]
    fn process_helper_times_out_and_kills_child() {
        let mut command = Command::new("sh");
        command.arg("-c").arg("sleep 5");

        let err = output_bounded(&mut command, Duration::from_millis(50), 1024).unwrap_err();

        assert!(matches!(err, BoundedProcessError::TimedOut { .. }));
    }

    #[test]
    fn process_helper_caps_stdout_and_kills_child() {
        let mut command = Command::new("sh");
        command.arg("-c").arg("yes stdout");

        let err = output_bounded(&mut command, Duration::from_secs(5), 1024).unwrap_err();

        assert!(matches!(
            err,
            BoundedProcessError::OutputLimitExceeded {
                stream: "stdout",
                limit: 1024
            }
        ));
    }

    #[test]
    fn process_helper_caps_stderr_and_kills_child() {
        let mut command = Command::new("sh");
        command.arg("-c").arg("yes stderr >&2");

        let err = output_bounded(&mut command, Duration::from_secs(5), 1024).unwrap_err();

        assert!(matches!(
            err,
            BoundedProcessError::OutputLimitExceeded {
                stream: "stderr",
                limit: 1024
            }
        ));
    }

    #[test]
    fn process_helper_caps_combined_stdout_and_stderr() {
        let stdout = "a".repeat(700);
        let stderr = "b".repeat(700);
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg(format!("printf '{}'; printf '{}' >&2", stdout, stderr));

        let err = output_bounded(&mut command, Duration::from_secs(1), 1024).unwrap_err();

        assert!(matches!(
            err,
            BoundedProcessError::OutputLimitExceeded { limit: 1024, .. }
        ));
    }
}
