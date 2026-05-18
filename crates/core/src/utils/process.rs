//! Bounded subprocess execution helpers.
//!
//! The Pandoc call sites use a 60 second timeout and a 50 MiB combined cap for
//! captured stdout/stderr. If the combined cap is exceeded, the child is killed.

use std::io::{self, Read};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub const PANDOC_TIMEOUT: Duration = Duration::from_secs(60);
pub const PANDOC_OUTPUT_LIMIT_BYTES: usize = 50 * 1024 * 1024;
const POST_KILL_DRAIN_TIMEOUT: Duration = Duration::from_millis(500);

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
    Finished(StreamKind, io::Result<Vec<u8>>),
    Capped(StreamKind, Vec<u8>),
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
    let mut child = ChildProcessTree::spawn(command.stdout(Stdio::piped()).stderr(Stdio::piped()))?;

    let stdout = child
        .stdout()
        .ok_or_else(|| io::Error::other("failed to capture stdout"))?;
    let stderr = child
        .stderr()
        .ok_or_else(|| io::Error::other("failed to capture stderr"))?;

    let (tx, rx) = mpsc::channel();
    let shared_bytes_read = Arc::new(Mutex::new(0usize));
    read_stream(
        stdout,
        StreamKind::Stdout,
        output_limit_bytes,
        shared_bytes_read.clone(),
        tx.clone(),
    );
    read_stream(
        stderr,
        StreamKind::Stderr,
        output_limit_bytes,
        shared_bytes_read,
        tx,
    );

    let deadline = Instant::now() + timeout;
    let mut stdout_done = false;
    let mut stderr_done = false;
    let mut child_status = None;
    let mut killed_after_exit = false;
    let mut stdout = None;
    let mut stderr = None;
    let status = loop {
        if child_status.is_none() {
            if let Some(status) = child.try_wait()? {
                child_status = Some(status);
            }
        }

        if child_status.is_some() && stdout_done && stderr_done {
            break child_status.take().expect("child status set");
        }

        if child_status.is_some() && !killed_after_exit {
            child.kill_tree()?;
            killed_after_exit = true;
        }

        if Instant::now() >= deadline {
            child.kill_tree()?;
            let _ = child.wait();
            drain_reader_events(
                &rx,
                &mut stdout_done,
                &mut stderr_done,
                &mut stdout,
                &mut stderr,
                POST_KILL_DRAIN_TIMEOUT,
            )?;
            return Err(BoundedProcessError::TimedOut { timeout });
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        let wait_for = remaining.min(Duration::from_millis(20));
        match rx.recv_timeout(wait_for) {
            Ok(event) => {
                if let Some(stream) = handle_stream_event(
                    event,
                    &mut stdout_done,
                    &mut stderr_done,
                    &mut stdout,
                    &mut stderr,
                )? {
                    child.kill_tree()?;
                    let _ = child.wait();
                    drain_reader_events(
                        &rx,
                        &mut stdout_done,
                        &mut stderr_done,
                        &mut stdout,
                        &mut stderr,
                        POST_KILL_DRAIN_TIMEOUT,
                    )?;
                    return Err(BoundedProcessError::OutputLimitExceeded {
                        stream,
                        limit: output_limit_bytes,
                    });
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                if !(stdout_done && stderr_done) {
                    child.kill_tree()?;
                    let _ = child.wait();
                    return Err(io::Error::other("process output reader disconnected").into());
                }
            }
        }

        if stdout_done && stderr_done && child_status.is_none() {
            if let Some(status) = child.try_wait()? {
                child_status = Some(status);
            }
        }
    };

    Ok(BoundedOutput {
        status,
        stdout: stdout.unwrap_or_default(),
        stderr: stderr.unwrap_or_default(),
    })
}

fn handle_stream_event(
    event: StreamEvent,
    stdout_done: &mut bool,
    stderr_done: &mut bool,
    stdout: &mut Option<Vec<u8>>,
    stderr: &mut Option<Vec<u8>>,
) -> Result<Option<&'static str>, BoundedProcessError> {
    match event {
        StreamEvent::Finished(StreamKind::Stdout, result) => {
            *stdout = Some(result?);
            *stdout_done = true;
            Ok(None)
        }
        StreamEvent::Finished(StreamKind::Stderr, result) => {
            *stderr = Some(result?);
            *stderr_done = true;
            Ok(None)
        }
        StreamEvent::Capped(StreamKind::Stdout, output) => {
            *stdout = Some(output);
            *stdout_done = true;
            Ok(Some("stdout"))
        }
        StreamEvent::Capped(StreamKind::Stderr, output) => {
            *stderr = Some(output);
            *stderr_done = true;
            Ok(Some("stderr"))
        }
    }
}

fn drain_reader_events(
    rx: &mpsc::Receiver<StreamEvent>,
    stdout_done: &mut bool,
    stderr_done: &mut bool,
    stdout: &mut Option<Vec<u8>>,
    stderr: &mut Option<Vec<u8>>,
    timeout: Duration,
) -> Result<(), BoundedProcessError> {
    let deadline = Instant::now() + timeout;
    while !(*stdout_done && *stderr_done) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }

        match rx.recv_timeout(remaining.min(Duration::from_millis(20))) {
            Ok(event) => {
                let _ = handle_stream_event(event, stdout_done, stderr_done, stdout, stderr)?;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

#[cfg(any(unix, windows))]
struct ChildProcessTree {
    inner: platform::ChildProcessTree,
}

#[cfg(any(unix, windows))]
impl ChildProcessTree {
    fn spawn(command: &mut Command) -> io::Result<Self> {
        platform::spawn(command).map(|inner| Self { inner })
    }

    fn stdout(&mut self) -> Option<platform::ChildStdout> {
        self.inner.stdout()
    }

    fn stderr(&mut self) -> Option<platform::ChildStderr> {
        self.inner.stderr()
    }

    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.inner.try_wait()
    }

    fn wait(&mut self) -> io::Result<ExitStatus> {
        self.inner.wait()
    }

    fn kill_tree(&mut self) -> io::Result<()> {
        self.inner.kill_tree()
    }
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::os::unix::process::CommandExt;

    pub(super) type ChildStdout = std::process::ChildStdout;
    pub(super) type ChildStderr = std::process::ChildStderr;

    pub(super) struct ChildProcessTree {
        child: std::process::Child,
        process_group: ProcessGroup,
    }

    #[derive(Clone, Copy)]
    struct ProcessGroup {
        pgid: libc::pid_t,
    }

    impl ChildProcessTree {
        pub(super) fn stdout(&mut self) -> Option<ChildStdout> {
            self.child.stdout.take()
        }

        pub(super) fn stderr(&mut self) -> Option<ChildStderr> {
            self.child.stderr.take()
        }

        pub(super) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            self.child.try_wait()
        }

        pub(super) fn wait(&mut self) -> io::Result<ExitStatus> {
            self.child.wait()
        }

        pub(super) fn kill_tree(&mut self) -> io::Result<()> {
            let result = unsafe { libc::kill(-self.process_group.pgid, libc::SIGKILL) };
            if result == 0 {
                return Ok(());
            }

            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                Ok(())
            } else {
                Err(err)
            }
        }
    }

    pub(super) fn spawn(command: &mut Command) -> io::Result<ChildProcessTree> {
        command.process_group(0);
        let child = command.spawn()?;
        let process_group = ProcessGroup {
            pgid: child.id() as libc::pid_t,
        };
        Ok(ChildProcessTree {
            child,
            process_group,
        })
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::collections::BTreeMap;
    use std::env;
    use std::ffi::{OsStr, OsString};
    use std::fs::File;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{FromRawHandle, RawHandle};
    use windows_sys::Win32::Foundation::{
        CloseHandle, HANDLE, HANDLE_FLAG_INHERIT, STILL_ACTIVE, WAIT_FAILED, WAIT_OBJECT_0,
        WAIT_TIMEOUT,
    };
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_INPUT_HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Pipes::CreatePipe;
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, GetExitCodeProcess, ResumeThread, TerminateProcess, WaitForSingleObject,
        CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, INFINITE, PROCESS_INFORMATION,
        STARTF_USESTDHANDLES, STARTUPINFOW,
    };

    pub(super) type ChildStdout = File;
    pub(super) type ChildStderr = File;

    pub(super) struct ChildProcessTree {
        process: ProcessHandle,
        job: JobHandle,
        stdout: Option<File>,
        stderr: Option<File>,
    }

    impl ChildProcessTree {
        pub(super) fn stdout(&mut self) -> Option<ChildStdout> {
            self.stdout.take()
        }

        pub(super) fn stderr(&mut self) -> Option<ChildStderr> {
            self.stderr.take()
        }

        pub(super) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            self.process.try_wait()
        }

        pub(super) fn wait(&mut self) -> io::Result<ExitStatus> {
            self.process.wait()
        }

        pub(super) fn kill_tree(&mut self) -> io::Result<()> {
            match self.job.terminate() {
                Ok(()) => Ok(()),
                Err(job_err) => match self.process.terminate() {
                    Ok(()) => Ok(()),
                    Err(_) => Err(job_err),
                },
            }
        }
    }

    struct Handle {
        handle: HANDLE,
    }

    impl Handle {
        fn new(handle: HANDLE) -> io::Result<Self> {
            if handle.is_null() {
                Err(io::Error::last_os_error())
            } else {
                Ok(Self { handle })
            }
        }

        fn raw(&self) -> HANDLE {
            self.handle
        }

        fn into_file(mut self) -> File {
            let handle = self.handle;
            self.handle = std::ptr::null_mut();
            unsafe { File::from_raw_handle(handle as RawHandle) }
        }
    }

    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.handle.is_null() {
                unsafe {
                    CloseHandle(self.handle);
                }
            }
        }
    }

    struct Pipe {
        read: Handle,
        write: Handle,
    }

    impl Pipe {
        fn new() -> io::Result<Self> {
            let mut read = std::ptr::null_mut();
            let mut write = std::ptr::null_mut();
            let mut attrs = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: std::ptr::null_mut(),
                bInheritHandle: 1,
            };

            let ok = unsafe { CreatePipe(&mut read, &mut write, &mut attrs, 0) };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }

            let read = Handle::new(read)?;
            let write = Handle::new(write)?;
            let ok = unsafe {
                windows_sys::Win32::Foundation::SetHandleInformation(
                    read.raw(),
                    HANDLE_FLAG_INHERIT,
                    0,
                )
            };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(Self { read, write })
        }
    }

    pub(super) struct JobHandle {
        handle: HANDLE,
    }

    impl JobHandle {
        fn create() -> io::Result<Self> {
            let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }

            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = unsafe {
                SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            if ok == 0 {
                let err = io::Error::last_os_error();
                unsafe {
                    CloseHandle(handle);
                }
                return Err(err);
            }

            Ok(Self { handle })
        }

        fn assign(&self, process: HANDLE) -> io::Result<()> {
            let ok = unsafe { AssignProcessToJobObject(self.handle, process) };
            if ok == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }

        fn terminate(&self) -> io::Result<()> {
            let ok = unsafe { TerminateJobObject(self.handle, 1) };
            if ok == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    impl Drop for JobHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    struct ProcessHandle {
        handle: HANDLE,
    }

    impl ProcessHandle {
        fn try_wait(&self) -> io::Result<Option<ExitStatus>> {
            match unsafe { WaitForSingleObject(self.handle, 0) } {
                WAIT_TIMEOUT => Ok(None),
                WAIT_OBJECT_0 => self.exit_status().map(Some),
                WAIT_FAILED => Err(io::Error::last_os_error()),
                _ => Err(io::Error::last_os_error()),
            }
        }

        fn wait(&self) -> io::Result<ExitStatus> {
            match unsafe { WaitForSingleObject(self.handle, INFINITE) } {
                WAIT_OBJECT_0 => self.exit_status(),
                WAIT_FAILED => Err(io::Error::last_os_error()),
                _ => Err(io::Error::last_os_error()),
            }
        }

        fn exit_status(&self) -> io::Result<ExitStatus> {
            let mut code = 0;
            let ok = unsafe { GetExitCodeProcess(self.handle, &mut code) };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            if code == STILL_ACTIVE as u32 {
                return Err(io::Error::other("process is still active"));
            }
            Ok(<ExitStatus as std::os::windows::process::ExitStatusExt>::from_raw(code))
        }

        fn terminate(&self) -> io::Result<()> {
            let ok = unsafe { TerminateProcess(self.handle, 1) };
            if ok == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    struct ThreadHandle {
        handle: HANDLE,
    }

    impl ThreadHandle {
        fn resume(self) -> io::Result<()> {
            let result = unsafe { ResumeThread(self.handle) };
            if result == u32::MAX {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    impl Drop for ThreadHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    pub(super) fn spawn(command: &mut Command) -> io::Result<ChildProcessTree> {
        let job = JobHandle::create()?;
        let stdout_pipe = Pipe::new()?;
        let stderr_pipe = Pipe::new()?;
        let mut command_line = command_line(command);
        let current_dir = command
            .get_current_dir()
            .map(|dir| wide_null(dir.as_os_str()));
        let environment = environment_block(command);

        let mut startup = STARTUPINFOW::default();
        startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        startup.dwFlags = STARTF_USESTDHANDLES;
        startup.hStdInput = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        startup.hStdOutput = stdout_pipe.write.raw();
        startup.hStdError = stderr_pipe.write.raw();

        let mut process_info = PROCESS_INFORMATION::default();
        let ok = unsafe {
            CreateProcessW(
                std::ptr::null(),
                command_line.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
                CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
                environment
                    .as_ref()
                    .map_or(std::ptr::null(), |block| block.as_ptr() as *const _),
                current_dir
                    .as_ref()
                    .map_or(std::ptr::null(), |dir| dir.as_ptr()),
                &startup,
                &mut process_info,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }

        let process = ProcessHandle {
            handle: process_info.hProcess,
        };
        let thread = ThreadHandle {
            handle: process_info.hThread,
        };

        if let Err(err) = job.assign(process.handle) {
            let _ = process.terminate();
            let _ = process.wait();
            return Err(err);
        }

        if let Err(err) = thread.resume() {
            let _ = job.terminate();
            let _ = process.wait();
            return Err(err);
        }
        drop(stdout_pipe.write);
        drop(stderr_pipe.write);
        Ok(ChildProcessTree {
            process,
            job,
            stdout: Some(stdout_pipe.read.into_file()),
            stderr: Some(stderr_pipe.read.into_file()),
        })
    }

    fn command_line(command: &Command) -> Vec<u16> {
        let mut line = Vec::new();
        append_quoted_arg(&mut line, command.get_program());
        for arg in command.get_args() {
            line.push(' ' as u16);
            append_quoted_arg(&mut line, arg);
        }
        line.push(0);
        line
    }

    fn append_quoted_arg(line: &mut Vec<u16>, arg: &OsStr) {
        let wide: Vec<u16> = arg.encode_wide().collect();
        let needs_quotes = wide.is_empty()
            || wide
                .iter()
                .any(|ch| matches!(*ch, 0x20 | 0x09 | b'"' as u16));

        if !needs_quotes {
            line.extend(wide);
            return;
        }

        line.push(b'"' as u16);
        let mut backslashes = 0;
        for ch in wide {
            if ch == b'\\' as u16 {
                backslashes += 1;
            } else if ch == b'"' as u16 {
                for _ in 0..(backslashes * 2 + 1) {
                    line.push(b'\\' as u16);
                }
                line.push(ch);
                backslashes = 0;
            } else {
                for _ in 0..backslashes {
                    line.push(b'\\' as u16);
                }
                line.push(ch);
                backslashes = 0;
            }
        }
        for _ in 0..(backslashes * 2) {
            line.push(b'\\' as u16);
        }
        line.push(b'"' as u16);
    }

    fn wide_null(value: &OsStr) -> Vec<u16> {
        let mut wide: Vec<u16> = value.encode_wide().collect();
        wide.push(0);
        wide
    }

    fn environment_block(command: &Command) -> Option<Vec<u16>> {
        let envs: Vec<_> = command.get_envs().collect();
        if envs.is_empty() {
            return None;
        }

        let mut merged: BTreeMap<OsString, OsString> = env::vars_os().collect();
        for (key, value) in envs {
            if let Some(value) = value {
                merged.insert(key.to_os_string(), value.to_os_string());
            } else {
                merged.remove(key);
            }
        }

        let mut block = Vec::new();
        for (key, value) in merged {
            block.extend(key.encode_wide());
            block.push(b'=' as u16);
            block.extend(value.encode_wide());
            block.push(0);
        }
        block.push(0);
        Some(block)
    }
}

#[cfg(not(any(unix, windows)))]
compile_error!("output_bounded process-tree management is only implemented for Unix and Windows");

fn read_stream<R: Read + Send + 'static>(
    mut stream: R,
    kind: StreamKind,
    limit: usize,
    shared_bytes_read: Arc<Mutex<usize>>,
    tx: mpsc::Sender<StreamEvent>,
) {
    thread::spawn(move || {
        let result = read_stream_to_end(&mut stream, kind, limit, shared_bytes_read, &tx);
        if let Some(event) = result {
            let _ = tx.send(event);
        }
    });
}

fn read_stream_to_end<R: Read>(
    stream: &mut R,
    kind: StreamKind,
    limit: usize,
    shared_bytes_read: Arc<Mutex<usize>>,
    tx: &mpsc::Sender<StreamEvent>,
) -> Option<StreamEvent> {
    let mut output = Vec::new();
    let mut buf = [0_u8; 8192];

    loop {
        let read = match stream.read(&mut buf) {
            Ok(read) => read,
            Err(err) => return Some(StreamEvent::Finished(kind, Err(err))),
        };
        if read == 0 {
            return Some(StreamEvent::Finished(kind, Ok(output)));
        }

        let allowed = {
            let mut total_read = match shared_bytes_read.lock() {
                Ok(total_read) => total_read,
                Err(_) => {
                    return Some(StreamEvent::Finished(
                        kind,
                        Err(io::Error::other("output cap lock poisoned")),
                    ));
                }
            };
            let remaining = limit.saturating_sub(*total_read);
            let allowed = remaining.min(read);
            *total_read = total_read.saturating_add(allowed);
            allowed
        };

        if allowed < read {
            output.extend_from_slice(&buf[..allowed]);
            let _ = tx.send(StreamEvent::Capped(kind, output));
            return None;
        }

        output.extend_from_slice(&buf[..allowed]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    const HELPER_MODE_ENV: &str = "CHIKN_PROCESS_HELPER_MODE";
    const HELPER_MARKER_ENV: &str = "CHIKN_PROCESS_HELPER_MARKER";
    const HELPER_TEST_NAME: &str = "utils::process::tests::process_helper_fixture";

    fn helper_command(mode: &str) -> Command {
        let mut command = Command::new(env::current_exe().unwrap());
        command
            .arg("--exact")
            .arg(HELPER_TEST_NAME)
            .arg("--nocapture")
            .env(HELPER_MODE_ENV, mode);
        command
    }

    fn helper_command_with_marker(mode: &str, marker: &Path) -> Command {
        let mut command = helper_command(mode);
        command.env(HELPER_MARKER_ENV, marker);
        command
    }

    fn marker_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "chikn-process-{label}-{}-{nanos}.marker",
            std::process::id()
        ))
    }

    fn assert_marker_not_written(marker: &Path) {
        thread::sleep(Duration::from_millis(800));
        assert!(
            !marker.exists(),
            "process-tree child survived and wrote marker at {}",
            marker.display()
        );
        let _ = fs::remove_file(marker);
    }

    fn spawn_pipe_holder() {
        let mut command = Command::new(env::current_exe().unwrap());
        command
            .arg("--exact")
            .arg(HELPER_TEST_NAME)
            .arg("--nocapture")
            .env(HELPER_MODE_ENV, "hold-pipe");
        if let Ok(marker) = env::var(HELPER_MARKER_ENV) {
            command.env(HELPER_MARKER_ENV, marker);
        }
        let mut child = command.spawn().unwrap();

        thread::spawn(move || {
            let _ = child.wait();
        });
    }

    #[test]
    fn process_helper_fixture() {
        match env::var(HELPER_MODE_ENV).as_deref() {
            Ok("success") => {
                print!("stdout");
                eprint!("stderr");
            }
            Ok("sleep") => thread::sleep(Duration::from_secs(5)),
            Ok("hold-pipe") => {
                if let Ok(marker) = env::var(HELPER_MARKER_ENV) {
                    thread::sleep(Duration::from_millis(500));
                    let _ = fs::write(marker, "alive");
                }
                thread::sleep(Duration::from_secs(5));
            }
            Ok("spawn-holder-exit") => spawn_pipe_holder(),
            Ok("sleep-with-holder") => {
                spawn_pipe_holder();
                thread::sleep(Duration::from_secs(5));
            }
            Ok("spam-stdout") => {
                let mut stdout = io::stdout().lock();
                let chunk = vec![b'a'; 8192];
                for _ in 0..128 {
                    stdout.write_all(&chunk).unwrap();
                    stdout.flush().unwrap();
                }
            }
            Ok("spam-stdout-with-holder") => {
                spawn_pipe_holder();
                let mut stdout = io::stdout().lock();
                let chunk = vec![b'a'; 8192];
                loop {
                    stdout.write_all(&chunk).unwrap();
                    stdout.flush().unwrap();
                }
            }
            Ok("spam-stderr") => {
                let mut stderr = io::stderr().lock();
                let chunk = vec![b'b'; 8192];
                for _ in 0..128 {
                    stderr.write_all(&chunk).unwrap();
                    stderr.flush().unwrap();
                }
            }
            Ok("combined-output") => {
                let stdout = "a".repeat(2048);
                let stderr = "b".repeat(2048);
                print!("{stdout}");
                eprint!("{stderr}");
            }
            _ => {}
        }
    }

    #[test]
    fn process_helper_captures_success_output() {
        let mut command = helper_command("success");

        let output = output_bounded(&mut command, Duration::from_secs(1), 1024).unwrap();

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("stdout"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("stderr"));
    }

    #[test]
    fn process_helper_times_out_and_kills_child() {
        let mut command = helper_command("sleep");

        let err = output_bounded(&mut command, Duration::from_millis(50), 1024).unwrap_err();

        assert!(matches!(err, BoundedProcessError::TimedOut { .. }));
    }

    #[test]
    fn process_helper_timeout_kills_process_tree() {
        let marker = marker_path("timeout");
        let mut command = helper_command_with_marker("sleep-with-holder", &marker);

        let started = Instant::now();
        let err = output_bounded(&mut command, Duration::from_millis(50), 1024).unwrap_err();

        assert!(matches!(err, BoundedProcessError::TimedOut { .. }));
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "output_bounded waited for process-tree child after timeout"
        );
        assert_marker_not_written(&marker);
    }

    #[test]
    fn process_helper_returns_when_grandchild_keeps_pipe_open_after_parent_exit() {
        let marker = marker_path("parent-exit");
        let mut command = helper_command_with_marker("spawn-holder-exit", &marker);

        let started = Instant::now();
        let output = output_bounded(&mut command, Duration::from_secs(2), 1024).unwrap();

        assert!(output.status.success());
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "output_bounded waited for inherited pipe grandchild"
        );
        assert_marker_not_written(&marker);
    }

    #[test]
    fn process_helper_caps_stdout_and_kills_child() {
        let mut command = helper_command("spam-stdout");

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
    fn process_helper_output_cap_kills_process_tree() {
        let marker = marker_path("output-cap");
        let mut command = helper_command_with_marker("spam-stdout-with-holder", &marker);

        let started = Instant::now();
        let err = output_bounded(&mut command, Duration::from_secs(5), 1024).unwrap_err();

        assert!(matches!(
            err,
            BoundedProcessError::OutputLimitExceeded {
                stream: "stdout",
                limit: 1024
            }
        ));
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "output_bounded waited for process-tree child after output cap"
        );
        assert_marker_not_written(&marker);
    }

    #[test]
    fn process_helper_caps_stderr_and_kills_child() {
        let mut command = helper_command("spam-stderr");

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
        let mut command = helper_command("combined-output");

        let err = output_bounded(&mut command, Duration::from_secs(1), 1024).unwrap_err();

        assert!(
            matches!(
                err,
                BoundedProcessError::OutputLimitExceeded { limit: 1024, .. }
            ),
            "unexpected combined-output error: {err:?}"
        );
    }
}
