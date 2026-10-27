//! Serving a project's own files to the shells (PLAN_SCRIVENER_UPLOAD slice
//! 3): the picture a document shows as `![alt](research/figure.png)`.
//!
//! Both shells call [`serve_project_asset`] with the project they consider
//! open and the relative path the document names; the web shell gates the
//! project path first, the desktop keeps the open project in shared state
//! and refuses everything once it is closed (plan-4 R5-6). The checks here
//! are the same for both: a relative path of plain components, no hidden
//! entries, no symlinks, resolving inside the project folder.

use std::fs;
use std::path::{Component, Path, PathBuf};

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
    let target = safe_join(project_root, relative)?;
    let root = project_root
        .canonicalize()
        .map_err(|_| AssetRefusal::NotFound)?;
    // Walk down one component at a time and refuse a symlink anywhere on
    // the way, not only at the end: a visible folder link pointing at a
    // hidden folder would otherwise serve its contents through the alias
    // (review finding s3-9).
    let mut cur = root.clone();
    let pieces: Vec<_> = target
        .strip_prefix(project_root)
        .map_err(|_| AssetRefusal::BadPath)?
        .components()
        .collect();
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
    if !canonical.starts_with(&root) {
        return Err(AssetRefusal::Outside);
    }
    let bytes = fs::read(&canonical).map_err(|_| AssetRefusal::NotFound)?;
    Ok(Asset {
        bytes,
        content_type: content_type_for(relative),
    })
}

/// `relative` split on either separator; every component a visible plain
/// name. Returns the joined path (not yet checked on disk).
fn safe_join(project_root: &Path, relative: &str) -> Result<PathBuf, AssetRefusal> {
    if relative.is_empty() || relative.chars().any(|c| c.is_control()) {
        return Err(AssetRefusal::BadPath);
    }
    let mut out = project_root.to_path_buf();
    let mut any = false;
    for piece in relative.split(['/', '\\']) {
        if piece.is_empty() {
            // A leading or doubled separator: refuse an absolute form.
            if !any {
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
        out.push(piece);
        any = true;
    }
    if !any {
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
