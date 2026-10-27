//! Publication of an imported project (PLAN_SCRIVENER_UPLOAD slice 3).
//!
//! An importer builds a `.chikn` project under a temporary sibling name
//! (`<final>.importing-<random>`) and asks the engine to publish it. Moving
//! and removing a `.chikn` folder is `.chikn` I/O, so the step lives here
//! (review finding plan-4 R3-3). The contract (R1-1, R2-1, R3-4, R3-5):
//!
//! - every fallible read of the finished project happens *before* the
//!   commit point;
//! - the commit point is a rename that cannot replace an existing entry
//!   (`renameat2(RENAME_NOREPLACE)` on Linux, `renamex_np(RENAME_EXCL)` on
//!   macOS, `MoveFileExW` without `REPLACE_EXISTING` on Windows), so a
//!   folder created between the caller's check and the rename is never
//!   replaced; where the storage refuses the exclusive form the import fails
//!   safely instead of falling back to a plain rename;
//! - after the commit nothing is undone: if the write token cannot be
//!   acquired at the final path, the result says so truthfully
//!   ([`Published::NotOpened`]) rather than reporting a failure that would
//!   leave a published project behind a "failed" message.

use crate::core::project::fidelity::{acquire_write_token, WriteToken};
use crate::core::project::reader;
use crate::models::Project;
use crate::utils::error::ChiknError;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The outcome of a successful publication.
#[derive(Debug)]
pub enum Published {
    /// The project is in place and open for writing.
    Opened {
        project: Box<Project>,
        token: WriteToken,
    },
    /// The project is in place at `path` but could not be opened for
    /// writing afterwards; it appears on the project screen like any other.
    NotOpened { path: PathBuf, reason: String },
}

/// Publish the finished project at `temp` as `final_path`.
///
/// On an error before the commit point the temporary project is removed,
/// except when the destination came into existence meanwhile: then the
/// temporary project is left for the caller (nothing was replaced) and the
/// error names the destination.
pub fn publish_imported(temp: &Path, final_path: &Path) -> Result<Published, ChiknError> {
    publish_with(temp, final_path, exclusive_rename, acquire_write_token)
}

/// The publication with its two platform-facing steps injectable, so tests
/// can stage the destination race, an unsupported storage and a token
/// failure after the commit.
pub(crate) fn publish_with(
    temp: &Path,
    final_path: &Path,
    rename: impl Fn(&Path, &Path) -> io::Result<()>,
    acquire: impl Fn(&Path) -> Result<WriteToken, ChiknError>,
) -> Result<Published, ChiknError> {
    // Read everything before the commit: a project that cannot be read is
    // never published.
    let mut project = match reader::read_project_readonly(temp) {
        Ok(project) => project,
        Err(err) => {
            discard(temp);
            return Err(err);
        }
    };

    if final_path.symlink_metadata().is_ok() {
        return Err(already_exists(final_path));
    }

    // The commit point.
    if let Err(err) = rename(temp, final_path) {
        if is_exists(&err) {
            return Err(already_exists(final_path));
        }
        discard(temp);
        if is_unsupported(&err) {
            return Err(ChiknError::InvalidProject(format!(
                "This projects folder's storage cannot publish a project safely \
                 (the exclusive rename into {} was refused: {err}); choose another folder",
                final_path.display()
            )));
        }
        return Err(ChiknError::Io(err));
    }

    // After the commit: rebind to the final path. Nothing below may undo
    // the publication.
    project.path = final_path.to_string_lossy().into_owned();
    match acquire(final_path) {
        Ok(token) => Ok(Published::Opened {
            project: Box::new(project),
            token,
        }),
        Err(err) => Ok(Published::NotOpened {
            path: final_path.to_path_buf(),
            reason: err.to_string(),
        }),
    }
}

fn already_exists(final_path: &Path) -> ChiknError {
    ChiknError::InvalidFormat(format!("Project already exists: {}", final_path.display()))
}

fn discard(temp: &Path) {
    let _ = fs::remove_dir_all(temp);
}

fn is_exists(err: &io::Error) -> bool {
    if err.kind() == io::ErrorKind::AlreadyExists {
        return true;
    }
    #[cfg(unix)]
    {
        matches!(
            err.raw_os_error(),
            Some(libc::EEXIST) | Some(libc::ENOTEMPTY)
        )
    }
    #[cfg(windows)]
    {
        // ERROR_FILE_EXISTS, ERROR_ALREADY_EXISTS
        matches!(err.raw_os_error(), Some(80) | Some(183))
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

fn is_unsupported(err: &io::Error) -> bool {
    if err.kind() == io::ErrorKind::Unsupported {
        return true;
    }
    #[cfg(unix)]
    {
        matches!(
            err.raw_os_error(),
            Some(libc::EINVAL) | Some(libc::ENOSYS) | Some(libc::ENOTSUP)
        )
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Rename that fails when the destination exists, whatever it is.
#[cfg(target_os = "linux")]
fn exclusive_rename(from: &Path, to: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let from = CString::new(from.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path holds a NUL byte"))?;
    let to = CString::new(to.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path holds a NUL byte"))?;
    // SAFETY: both pointers are valid NUL-terminated C strings for the
    // duration of the call; renameat2 reads them and touches nothing else.
    let rc = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            libc::RENAME_NOREPLACE as libc::c_ulong,
        )
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
fn exclusive_rename(from: &Path, to: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let from = CString::new(from.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path holds a NUL byte"))?;
    let to = CString::new(to.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path holds a NUL byte"))?;
    // SAFETY: as above; renamex_np reads both strings.
    let rc = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn exclusive_rename(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
    let wide = |p: &Path| -> Vec<u16> { p.as_os_str().encode_wide().chain(Some(0)).collect() };
    let from = wide(from);
    let to = wide(to);
    // SAFETY: both buffers are NUL-terminated UTF-16 strings that outlive
    // the call; no MOVEFILE_REPLACE_EXISTING, so an existing destination
    // makes the call fail.
    let ok = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) };
    if ok != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// No exclusive rename is known for this platform: refuse rather than fall
/// back to a rename that could replace an existing folder.
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn exclusive_rename(_from: &Path, _to: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no exclusive rename on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::project::writer::create_project;
    use tempfile::TempDir;

    fn staged(root: &Path, name: &str) -> (PathBuf, PathBuf) {
        let final_path = root.join(format!("{name}.chikn"));
        let temp = root.join(format!("{name}.chikn.importing-test"));
        create_project(&temp, name).unwrap();
        (temp, final_path)
    }

    #[test]
    fn publishes_and_rebinds_to_the_final_path() {
        let dir = TempDir::new().unwrap();
        let (temp, final_path) = staged(dir.path(), "Novel");
        match publish_imported(&temp, &final_path).unwrap() {
            Published::Opened { project, token } => {
                assert_eq!(project.path, final_path.to_string_lossy());
                assert_eq!(token.root(), final_path.canonicalize().unwrap());
            }
            other => panic!("expected Opened, got {other:?}"),
        }
        assert!(final_path.join("project.yaml").exists());
        assert!(!temp.exists(), "the temporary name is gone");
        // The published project reads back.
        reader::read_project(&final_path).unwrap();
    }

    /// A destination that comes into existence between the caller's check
    /// and the rename is never replaced: the exclusive rename refuses it and
    /// the temporary project stays.
    #[test]
    fn refuses_a_destination_created_between_check_and_rename() {
        let dir = TempDir::new().unwrap();
        let (temp, final_path) = staged(dir.path(), "Race");
        let racing_rename = |from: &Path, to: &Path| {
            fs::create_dir(to).unwrap(); // someone else wins the race
            exclusive_rename(from, to)
        };
        let err = publish_with(&temp, &final_path, racing_rename, acquire_write_token).unwrap_err();
        assert!(err.to_string().contains("already exists"), "{err}");
        assert!(temp.join("project.yaml").exists(), "temporary project kept");
        assert!(
            fs::read_dir(&final_path).unwrap().next().is_none(),
            "the empty folder was not replaced"
        );
    }

    /// Storage without an exclusive rename fails safely: no plain-rename
    /// fallback, temporary project removed, nothing at the destination.
    #[test]
    fn unsupported_storage_fails_safely() {
        let dir = TempDir::new().unwrap();
        let (temp, final_path) = staged(dir.path(), "Nas");
        let unsupported = |_: &Path, _: &Path| -> io::Result<()> {
            Err(io::Error::from_raw_os_error(libc::EINVAL))
        };
        let err = publish_with(&temp, &final_path, unsupported, acquire_write_token).unwrap_err();
        assert!(
            err.to_string().contains("cannot publish a project safely"),
            "{err}"
        );
        assert!(!temp.exists(), "temporary project removed");
        assert!(!final_path.exists(), "nothing published");
    }

    /// A project that cannot be read is never published and its temporary
    /// folder is removed.
    #[test]
    fn a_read_error_before_the_commit_removes_the_temporary_project() {
        let dir = TempDir::new().unwrap();
        let (temp, final_path) = staged(dir.path(), "Broken");
        fs::remove_file(temp.join("project.yaml")).unwrap();
        publish_imported(&temp, &final_path).unwrap_err();
        assert!(!temp.exists());
        assert!(!final_path.exists());
    }

    /// After the rename nothing is undone: a token failure yields a truthful
    /// NotOpened with the final path, and the project is intact there.
    #[test]
    fn a_token_failure_after_the_commit_reports_not_opened() {
        let dir = TempDir::new().unwrap();
        let (temp, final_path) = staged(dir.path(), "Locked");
        let no_token = |_: &Path| -> Result<WriteToken, ChiknError> {
            Err(ChiknError::ReadOnly("simulated".to_string()))
        };
        match publish_with(&temp, &final_path, exclusive_rename, no_token).unwrap() {
            Published::NotOpened { path, reason } => {
                assert_eq!(path, final_path);
                assert!(reason.contains("simulated"));
            }
            other => panic!("expected NotOpened, got {other:?}"),
        }
        assert!(!temp.exists());
        reader::read_project(&final_path).unwrap();
    }

    /// The real platform primitive refuses an existing destination of every
    /// kind: an empty folder, a file and a dangling symlink.
    #[cfg(unix)]
    #[test]
    fn exclusive_rename_refuses_every_existing_destination() {
        let dir = TempDir::new().unwrap();
        for (name, make) in [
            (
                "folder",
                Box::new(|p: &Path| fs::create_dir(p).unwrap()) as Box<dyn Fn(&Path)>,
            ),
            ("file", Box::new(|p: &Path| fs::write(p, b"x").unwrap())),
            (
                "dangling",
                Box::new(|p: &Path| std::os::unix::fs::symlink("/nonexistent/target", p).unwrap()),
            ),
        ] {
            let src = dir.path().join(format!("src-{name}"));
            fs::create_dir(&src).unwrap();
            let dst = dir.path().join(format!("dst-{name}"));
            make(&dst);
            let err = exclusive_rename(&src, &dst).unwrap_err();
            assert!(is_exists(&err), "{name}: {err}");
            assert!(src.exists(), "{name}: source untouched");
        }
    }
}
