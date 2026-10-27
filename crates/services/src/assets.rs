//! Serving a project's own files to the shells (PLAN_SCRIVENER_UPLOAD slice
//! 3): the picture a document shows as `![alt](research/figure.png)`.
//!
//! Both shells call [`serve_project_asset`] with the project they consider
//! open and the relative path the document names; the web shell gates the
//! project path first, the desktop keeps the open project in shared state
//! and refuses everything once it is closed (plan-4 R5-6). The checks here
//! are the same for both: a relative path of plain components, no hidden
//! entries, no symlinks, resolving inside the project folder.
//!
//! On Unix the walk holds a handle to each folder and opens the next entry
//! relative to it with `O_NOFOLLOW`, so what is read is exactly what was
//! checked: a folder swapped for a symlink between a check and the read
//! (review finding s3-9, repair review) fails to open instead of being
//! followed. Windows has no equivalent in the standard library; there the
//! walk checks each component by path, which closes the static alias but not
//! a concurrent swap (recorded as a known gap in the finding).

use std::fs;
use std::path::{Component, Path};

/// Why a request was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetRefusal {
    /// The path is not a plain relative path of visible components.
    BadPath,
    /// The path resolves outside the project (a symlink, `..`, a link).
    Outside,
    /// No such file.
    NotFound,
    /// Larger than a shell should hold in memory for one picture.
    TooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
}

/// 50 MiB, the same cap the shells use for bodies.
pub const MAX_ASSET_BYTES: u64 = 50 * 1024 * 1024;

/// Validate `relative` and read it from `project_root`.
pub fn serve_project_asset(project_root: &Path, relative: &str) -> Result<Asset, AssetRefusal> {
    let pieces = safe_pieces(relative)?;
    let root = project_root
        .canonicalize()
        .map_err(|_| AssetRefusal::NotFound)?;
    let bytes = read_within(&root, &pieces)?;
    Ok(Asset {
        bytes,
        content_type: content_type_for(relative),
    })
}

/// Open `pieces` under `root` one component at a time, each relative to the
/// handle of the folder before it and never following a symlink, then read
/// the file through the handle that was checked.
#[cfg(unix)]
fn read_within(root: &Path, pieces: &[String]) -> Result<Vec<u8>, AssetRefusal> {
    use std::ffi::CString;
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::io::{AsRawFd, FromRawFd};

    let mut dir = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open(root)
        .map_err(|_| AssetRefusal::NotFound)?;
    for (i, piece) in pieces.iter().enumerate() {
        let name = CString::new(piece.as_bytes()).map_err(|_| AssetRefusal::BadPath)?;
        let last = i + 1 == pieces.len();
        let mut flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        if !last {
            flags |= libc::O_DIRECTORY;
        }
        // SAFETY: `dir` is an open directory handle owned by this function and
        // `name` is a NUL-terminated string that outlives the call.
        let fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            // O_NOFOLLOW refuses a symlink with ELOOP (EMLINK on the BSDs),
            // or with ENOTDIR when O_DIRECTORY was asked for as well (Linux);
            // tell a symlink from a missing or misplaced entry by looking at
            // the entry itself, without following it.
            return Err(match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::ELOOP) | Some(libc::EMLINK) | Some(libc::ENOTDIR)
                    if is_symlink_at(dir.as_raw_fd(), &name) =>
                {
                    AssetRefusal::Outside
                }
                _ => AssetRefusal::NotFound,
            });
        }
        // SAFETY: `fd` was just returned by openat and is owned by nobody else.
        let file = unsafe { fs::File::from_raw_fd(fd) };
        if last {
            let meta = file.metadata().map_err(|_| AssetRefusal::NotFound)?;
            if !meta.is_file() {
                return Err(AssetRefusal::NotFound);
            }
            if meta.len() > MAX_ASSET_BYTES {
                return Err(AssetRefusal::TooLarge);
            }
            let mut bytes = Vec::with_capacity(meta.len() as usize);
            file.take(MAX_ASSET_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| AssetRefusal::NotFound)?;
            if bytes.len() as u64 > MAX_ASSET_BYTES {
                return Err(AssetRefusal::TooLarge);
            }
            return Ok(bytes);
        }
        dir = file;
    }
    Err(AssetRefusal::BadPath)
}

/// Whether `name` inside the open directory `dirfd` is a symlink right now.
#[cfg(unix)]
fn is_symlink_at(dirfd: std::os::unix::io::RawFd, name: &std::ffi::CStr) -> bool {
    // SAFETY: `st` is a plain stat buffer filled by the call; `name` is
    // NUL-terminated and `dirfd` is an open directory handle.
    unsafe {
        let mut st: libc::stat = std::mem::zeroed();
        libc::fstatat(dirfd, name.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) == 0
            && (st.st_mode & libc::S_IFMT) == libc::S_IFLNK
    }
}

/// The path-based walk: each component's own metadata is checked (a symlink
/// anywhere refuses), then the resolved path must still lie inside the root.
/// Not race-free against a concurrent swap; see the module note.
#[cfg(not(unix))]
fn read_within(root: &Path, pieces: &[String]) -> Result<Vec<u8>, AssetRefusal> {
    let mut cur = root.to_path_buf();
    for (i, piece) in pieces.iter().enumerate() {
        cur.push(piece);
        let meta = fs::symlink_metadata(&cur).map_err(|_| AssetRefusal::NotFound)?;
        if meta.file_type().is_symlink() {
            return Err(AssetRefusal::Outside);
        }
        let last = i + 1 == pieces.len();
        if last {
            if !meta.is_file() {
                return Err(AssetRefusal::NotFound);
            }
            if meta.len() > MAX_ASSET_BYTES {
                return Err(AssetRefusal::TooLarge);
            }
        } else if !meta.is_dir() {
            return Err(AssetRefusal::NotFound);
        }
    }
    let canonical = cur.canonicalize().map_err(|_| AssetRefusal::NotFound)?;
    if !canonical.starts_with(root) {
        return Err(AssetRefusal::Outside);
    }
    fs::read(&canonical).map_err(|_| AssetRefusal::NotFound)
}

/// `relative` split on either separator; every component a visible plain
/// name. Returns the components (not yet checked on disk).
fn safe_pieces(relative: &str) -> Result<Vec<String>, AssetRefusal> {
    if relative.is_empty() || relative.chars().any(|c| c.is_control()) {
        return Err(AssetRefusal::BadPath);
    }
    let mut out = Vec::new();
    for piece in relative.split(['/', '\\']) {
        if piece.is_empty() {
            // A leading or doubled separator: refuse an absolute form.
            if out.is_empty() {
                return Err(AssetRefusal::BadPath);
            }
            continue;
        }
        if piece == "." || piece == ".." || piece.starts_with('.') || piece.contains(':') {
            return Err(AssetRefusal::BadPath);
        }
        // Only ordinary names: nothing a path parser would read as a root,
        // prefix or parent.
        let as_path = Path::new(piece);
        let mut comps = as_path.components();
        match (comps.next(), comps.next()) {
            (Some(Component::Normal(_)), None) => {}
            _ => return Err(AssetRefusal::BadPath),
        }
        out.push(piece.to_string());
    }
    if out.is_empty() {
        return Err(AssetRefusal::BadPath);
    }
    Ok(out)
}

/// A content type from the extension; what browsers need to show a picture
/// or open a document, `application/octet-stream` otherwise.
pub fn content_type_for(path: &str) -> &'static str {
    let ext = path
        .rsplit('.')
        .next()
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "pdf" => "application/pdf",
        "txt" | "md" | "markdown" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn project() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("Novel.chikn");
        fs::create_dir_all(root.join("research")).unwrap();
        fs::write(root.join("research").join("figure.png"), b"\x89PNG").unwrap();
        fs::write(dir.path().join("secret.txt"), b"outside").unwrap();
        (dir, root)
    }

    #[test]
    fn serves_a_file_inside_the_project_with_its_type() {
        let (_dir, root) = project();
        let asset = serve_project_asset(&root, "research/figure.png").unwrap();
        assert_eq!(asset.bytes, b"\x89PNG");
        assert_eq!(asset.content_type, "image/png");
        // Either separator, as a document written on Windows might have it.
        assert!(serve_project_asset(&root, "research\\figure.png").is_ok());
    }

    #[test]
    fn refuses_traversal_absolute_hidden_and_odd_paths() {
        let (_dir, root) = project();
        for bad in [
            "../secret.txt",
            "research/../../secret.txt",
            "/etc/passwd",
            "\\\\server\\share",
            "C:secret.txt",
            ".git/config",
            "research/.hidden",
            "",
            "research/fig\u{1}.png",
            "./research/figure.png",
        ] {
            assert_eq!(
                serve_project_asset(&root, bad),
                Err(AssetRefusal::BadPath),
                "{bad}"
            );
        }
    }

    /// A visible folder symlink inside the project aliasing `.git` (or any
    /// folder) must not serve what the direct path refuses (s3-9).
    #[cfg(unix)]
    #[test]
    fn refuses_a_symlinked_folder_on_the_way() {
        let (_dir, root) = project();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git").join("config"), b"[core]").unwrap();
        std::os::unix::fs::symlink(root.join(".git"), root.join("alias")).unwrap();
        assert_eq!(
            serve_project_asset(&root, "alias/config"),
            Err(AssetRefusal::Outside)
        );
        assert_eq!(
            serve_project_asset(&root, ".git/config"),
            Err(AssetRefusal::BadPath)
        );
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_symlink_that_leaves_the_project() {
        let (dir, root) = project();
        std::os::unix::fs::symlink(
            dir.path().join("secret.txt"),
            root.join("research").join("link.txt"),
        )
        .unwrap();
        assert_eq!(
            serve_project_asset(&root, "research/link.txt"),
            Err(AssetRefusal::Outside)
        );
    }

    /// A folder that another process keeps swapping for a symlink to `.git`
    /// never serves the hidden file: every request reads through the handle
    /// it checked (s3-9 repair review, where the path-based walk leaked the
    /// file in 10 of 67 requests).
    #[cfg(unix)]
    #[test]
    fn a_folder_swapped_for_a_symlink_mid_request_never_serves_hidden_files() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let (_dir, root) = project();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git").join("config"), b"[core]").unwrap();
        fs::create_dir_all(root.join("alias")).unwrap();
        fs::write(root.join("alias").join("config"), b"decoy").unwrap();

        let stop = Arc::new(AtomicBool::new(false));
        let flipper = {
            let stop = stop.clone();
            let root = root.clone();
            std::thread::spawn(move || {
                let alias = root.join("alias");
                let parked = root.join("parked");
                while !stop.load(Ordering::Relaxed) {
                    // Directory → symlink → directory, as fast as the
                    // filesystem allows.
                    let _ = fs::rename(&alias, &parked);
                    let _ = std::os::unix::fs::symlink(root.join(".git"), &alias);
                    let _ = fs::remove_file(&alias);
                    let _ = fs::rename(&parked, &alias);
                }
            })
        };

        let mut served = 0;
        for _ in 0..3000 {
            match serve_project_asset(&root, "alias/config") {
                Ok(asset) => {
                    assert_eq!(asset.bytes, b"decoy", "the hidden file leaked");
                    served += 1;
                }
                Err(AssetRefusal::Outside) | Err(AssetRefusal::NotFound) => {}
                Err(other) => panic!("unexpected refusal {other:?}"),
            }
        }
        stop.store(true, Ordering::Relaxed);
        flipper.join().unwrap();
        // The decoy must have been reachable at least sometimes, or the
        // test proved nothing about the race.
        assert!(served > 0, "the decoy was never served");
    }

    #[test]
    fn missing_files_and_folders_are_not_found() {
        let (_dir, root) = project();
        assert_eq!(
            serve_project_asset(&root, "research/missing.png"),
            Err(AssetRefusal::NotFound)
        );
        assert_eq!(
            serve_project_asset(&root, "research"),
            Err(AssetRefusal::NotFound),
            "a folder is not an asset"
        );
    }

    #[test]
    fn content_types_cover_pictures_and_documents() {
        assert_eq!(content_type_for("a/b.JPG"), "image/jpeg");
        assert_eq!(content_type_for("x.svg"), "image/svg+xml");
        assert_eq!(content_type_for("notes.pdf"), "application/pdf");
        assert_eq!(content_type_for("odd.bin"), "application/octet-stream");
    }
}
