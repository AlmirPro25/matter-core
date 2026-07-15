//! File Capabilities v1 — explicit, default-deny filesystem access for program-initiated I/O.
//!
//! Policy is shared by `file.*` and `fileio.*`. CLI `compile -o` and other host-side
//! operations are **not** governed by this policy (they are not program-initiated).
//!
//! No environment variable grants access. Roots must be passed explicitly via CLI flags
//! (or equivalent API). Write never implies delete.

use std::fs;
use std::path::{Component, Path, PathBuf};

/// Distinct filesystem permissions (never implied by each other).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsPermission {
    Read,
    Write,
    Delete,
}

/// Capability policy: empty roots = deny all program FS access.
#[derive(Debug, Clone, Default)]
pub struct FsCapabilityPolicy {
    read_roots: Vec<PathBuf>,
    write_roots: Vec<PathBuf>,
    delete_roots: Vec<PathBuf>,
}

impl FsCapabilityPolicy {
    /// Default: no program FS access.
    pub fn deny_all() -> Self {
        Self::default()
    }

    pub fn is_deny_all(&self) -> bool {
        self.read_roots.is_empty() && self.write_roots.is_empty() && self.delete_roots.is_empty()
    }

    /// Add an explicit read root (directory). Path is canonicalized when possible.
    pub fn allow_read_root(&mut self, dir: impl AsRef<Path>) -> Result<(), String> {
        let root = canonicalize_root(dir.as_ref())?;
        self.read_roots.push(root);
        Ok(())
    }

    pub fn allow_write_root(&mut self, dir: impl AsRef<Path>) -> Result<(), String> {
        let root = canonicalize_root(dir.as_ref())?;
        self.write_roots.push(root);
        Ok(())
    }

    pub fn allow_delete_root(&mut self, dir: impl AsRef<Path>) -> Result<(), String> {
        let root = canonicalize_root(dir.as_ref())?;
        self.delete_roots.push(root);
        Ok(())
    }

    /// Check that `path` may be used for `perm`. Returns a safe resolved path on success.
    ///
    /// Errors always contain the token `capability_denied` and avoid dumping host secrets.
    pub fn check_path(&self, path: &str, perm: FsPermission) -> Result<PathBuf, String> {
        if path.is_empty() {
            return Err("capability_denied: empty path".into());
        }
        if path.contains('\0') {
            return Err("capability_denied: path contains NUL".into());
        }

        // Reject alternate data streams / device-like names early (best-effort).
        if path.contains(':') {
            // Allow Windows drive letter only as "X:..." at start.
            let bytes = path.as_bytes();
            let drive_ok = bytes.len() >= 2
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && (bytes.len() == 2 || bytes[2] == b'\\' || bytes[2] == b'/');
            if !drive_ok {
                return Err("capability_denied: path form not allowed".into());
            }
            // Extra colons (ADS) after the drive letter.
            if path[2..].contains(':') {
                return Err("capability_denied: path form not allowed".into());
            }
        }

        let roots = match perm {
            FsPermission::Read => &self.read_roots,
            FsPermission::Write => &self.write_roots,
            FsPermission::Delete => &self.delete_roots,
        };

        if roots.is_empty() {
            return Err(format!(
                "capability_denied: {} requires explicit authorization \
                 (--allow-fs-{} <dir>); no filesystem access is granted by default",
                perm_label(perm),
                perm_flag(perm)
            ));
        }

        let resolved = resolve_path_for_check(path)?;
        if !is_path_within_any_root(&resolved, roots) {
            return Err(format!(
                "capability_denied: path is outside authorized {} roots \
                 (traversal, absolute escape, and symlink/junction escape are blocked)",
                perm_label(perm)
            ));
        }

        // Extra containment: after resolving, if the path still contains ".." as a
        // remaining component (should not happen post-canonicalize), deny.
        if resolved
            .components()
            .any(|c| matches!(c, Component::ParentDir))
        {
            return Err("capability_denied: unresolved parent-directory component".into());
        }

        Ok(resolved)
    }

    /// Classify a backend method into the permission required for its primary path arg(s).
    pub fn permission_for_method(backend: &str, method: &str) -> Option<FsPermission> {
        let _ = backend;
        match method {
            "read" | "exists" | "lines" | "read_lines" | "size" | "is_file" | "is_dir"
            | "list_dir" => Some(FsPermission::Read),
            "write" | "append" | "write_lines" | "create_dir" => Some(FsPermission::Write),
            "delete" | "remove_dir" => Some(FsPermission::Delete),
            // copy/rename handled specially (two paths).
            "copy" | "rename" => None,
            _ => None,
        }
    }
}

fn perm_label(p: FsPermission) -> &'static str {
    match p {
        FsPermission::Read => "read",
        FsPermission::Write => "write",
        FsPermission::Delete => "delete",
    }
}

fn perm_flag(p: FsPermission) -> &'static str {
    match p {
        FsPermission::Read => "read",
        FsPermission::Write => "write",
        FsPermission::Delete => "delete",
    }
}

fn canonicalize_root(dir: &Path) -> Result<PathBuf, String> {
    if !dir.exists() {
        return Err(
            "capability_denied: allow root does not exist (create the directory first)".into(),
        );
    }
    if !dir.is_dir() {
        return Err("capability_denied: allow root must be a directory".into());
    }
    let canon = fs::canonicalize(dir).map_err(|_| {
        "capability_denied: cannot canonicalize allow root (symlinks/junctions may be unresolvable)"
            .to_string()
    })?;
    Ok(normalize_path(canon))
}

/// Resolve a program path for containment checks.
///
/// Existing paths: full canonicalize (resolves symlinks/junctions when the OS supports it).
/// Non-existing paths (writes): canonicalize parent + join final component.
/// If containment cannot be guaranteed, deny.
fn resolve_path_for_check(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);

    // Soft reject obvious ".." before resolution; final check is still prefix-based.
    // We still resolve because "foo/../secret" can land inside or outside a root.

    if p.exists() {
        let canon = fs::canonicalize(p).map_err(|_| {
            "capability_denied: cannot resolve path (symlink/junction/reparse may be blocked)"
                .to_string()
        })?;
        return Ok(normalize_path(canon));
    }

    // File does not exist: resolve parent.
    let parent = p.parent().filter(|par| !par.as_os_str().is_empty());
    let file_name = p
        .file_name()
        .ok_or_else(|| "capability_denied: path has no file name".to_string())?;

    let parent = match parent {
        Some(par) => par,
        None => {
            // Relative single component like "foo.txt" → current dir
            Path::new(".")
        }
    };

    if !parent.exists() {
        return Err(
            "capability_denied: parent directory does not exist or cannot be resolved".into(),
        );
    }

    // If parent is a symlink/junction that escapes, canonicalize will show the real target.
    let parent_canon = fs::canonicalize(parent).map_err(|_| {
        "capability_denied: cannot resolve parent path (symlink/junction/reparse may be blocked)"
            .to_string()
    })?;

    // Block if the final component is ".." or "." alone.
    if file_name == ".." || file_name == "." {
        return Err("capability_denied: invalid path component".into());
    }

    Ok(normalize_path(parent_canon.join(file_name)))
}

fn is_path_within_any_root(path: &Path, roots: &[PathBuf]) -> bool {
    let path_n = normalize_path(path.to_path_buf());
    for root in roots {
        let root_n = normalize_path(root.clone());
        if path_is_under(&path_n, &root_n) {
            return true;
        }
    }
    false
}

fn path_is_under(path: &Path, root: &Path) -> bool {
    if path == root {
        return true;
    }
    // Component-wise prefix check after normalization (avoids "C:\root_evil" matching "C:\root").
    let root_comps: Vec<_> = root.components().collect();
    let path_comps: Vec<_> = path.components().collect();
    if path_comps.len() < root_comps.len() {
        return false;
    }
    path_comps
        .iter()
        .zip(root_comps.iter())
        .all(|(a, b)| a == b)
}

/// Normalize path for comparison (strip Windows `\\?\` verbatim prefix).
fn normalize_path(p: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            // \\?\UNC\server\share → keep as \\server\share if needed
            if let Some(unc) = rest.strip_prefix("UNC\\") {
                return PathBuf::from(format!(r"\\{}", unc));
            }
            return PathBuf::from(rest);
        }
        p
    }
    #[cfg(not(windows))]
    {
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_root(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("matter_fs_cap_{}", name));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn deny_all_by_default() {
        let p = FsCapabilityPolicy::deny_all();
        let err = p
            .check_path("anything.txt", FsPermission::Read)
            .unwrap_err();
        assert!(err.contains("capability_denied"));
    }

    #[test]
    fn read_does_not_allow_write() {
        let root = temp_root("read_only");
        let mut p = FsCapabilityPolicy::deny_all();
        p.allow_read_root(&root).unwrap();
        let f = root.join("a.txt");
        fs::write(&f, "x").unwrap();
        assert!(p
            .check_path(f.to_str().unwrap(), FsPermission::Read)
            .is_ok());
        let err = p
            .check_path(f.to_str().unwrap(), FsPermission::Write)
            .unwrap_err();
        assert!(err.contains("capability_denied"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn write_does_not_allow_delete() {
        let root = temp_root("write_not_delete");
        let mut p = FsCapabilityPolicy::deny_all();
        p.allow_write_root(&root).unwrap();
        let f = root.join("a.txt");
        fs::write(&f, "x").unwrap();
        assert!(p
            .check_path(f.to_str().unwrap(), FsPermission::Write)
            .is_ok());
        let err = p
            .check_path(f.to_str().unwrap(), FsPermission::Delete)
            .unwrap_err();
        assert!(err.contains("capability_denied"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn blocks_parent_traversal() {
        let root = temp_root("trav");
        let mut p = FsCapabilityPolicy::deny_all();
        p.allow_read_root(&root).unwrap();
        // Escape attempt via ..
        let escape = root.join("..").join("should_not_matter");
        // Create sibling outside root to resolve to something real
        let outside = root.parent().unwrap().join("outside_matter_fs_cap.txt");
        fs::write(&outside, "secret").unwrap();
        let err = p
            .check_path(escape.to_str().unwrap(), FsPermission::Read)
            .unwrap_err();
        assert!(err.contains("capability_denied"));
        let _ = fs::remove_file(outside);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn blocks_absolute_outside_root() {
        let root = temp_root("abs");
        let mut p = FsCapabilityPolicy::deny_all();
        p.allow_read_root(&root).unwrap();
        let outside = std::env::temp_dir().join("matter_fs_cap_outside_abs.txt");
        fs::write(&outside, "x").unwrap();
        let err = p
            .check_path(outside.to_str().unwrap(), FsPermission::Read)
            .unwrap_err();
        assert!(err.contains("capability_denied"));
        let _ = fs::remove_file(outside);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn allows_unicode_and_spaces_inside_root() {
        let root = temp_root("unicode");
        let mut p = FsCapabilityPolicy::deny_all();
        p.allow_read_root(&root).unwrap();
        p.allow_write_root(&root).unwrap();
        let f = root.join("arquivo com espaço áé.txt");
        fs::write(&f, "ok").unwrap();
        assert!(p
            .check_path(f.to_str().unwrap(), FsPermission::Read)
            .is_ok());
        assert!(p
            .check_path(f.to_str().unwrap(), FsPermission::Write)
            .is_ok());
        let _ = fs::remove_dir_all(root);
    }
}
