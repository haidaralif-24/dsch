//! Model output is untrusted input. Path validation lives here.
//!
//! Checks, in order of increasing trust: the string must be a safe relative
//! path, it must resolve inside the repo root, and (for existing files) it must
//! not differ from the real file only by case.

use std::path::{Component, Path, PathBuf};

/// First-line check for a path from the model: non-empty, relative, and made
/// only of normal components. Rejects absolute paths and any `..`.
/// This is necessary but not sufficient; symlinks need a resolved check too.
pub fn is_safe_relative(path: &str) -> bool {
    let p = Path::new(path);
    !path.is_empty()
        && p.is_relative()
        && p.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

/// Resolved-path check: the fully-resolved target must live inside `root`.
/// Absolute paths and `..` escapes are rejected earlier by `is_safe_relative`;
/// this catches symlink escapes and canonicalization surprises.
pub fn resolves_inside(root: &Path, rel: &str) -> bool {
    let root_c = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let target = root_c.join(rel);
    let existing = match target.canonicalize() {
        Ok(c) => Some(c),
        Err(_) => ancestor_canonical(&target),
    };
    match existing {
        Some(c) => c.starts_with(&root_c),
        None => false,
    }
}

/// Deepest existing ancestor of `target` (the target itself if it exists).
fn ancestor_canonical(target: &Path) -> Option<PathBuf> {
    let parent = target.parent()?;
    if parent == target {
        return None;
    }
    parent.canonicalize().ok().or_else(|| ancestor_canonical(parent))
}

/// True if an existing file at `rel` differs from the real file only by case.
pub fn is_case_mismatch(root: &Path, rel: &str) -> bool {
    let target = root.join(rel);
    if !target.exists() {
        return false;
    }
    let file_name = Path::new(rel)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if file_name.is_empty() {
        return false;
    }
    let dir = target.parent().unwrap_or(root);
    match find_case_insensitive(dir, &file_name) {
        Some(real) => real != file_name,
        None => false,
    }
}

/// Case-insensitive lookup of `name` in `dir`; returns the real entry name.
fn find_case_insensitive(dir: &Path, name: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let entry_name = entry.file_name().to_string_lossy().to_string();
        if entry_name.eq_ignore_ascii_case(&name) {
            return Some(entry_name);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_relative_paths() {
        assert!(is_safe_relative("src/main.rs"));
        assert!(is_safe_relative("./src/main.rs"));
        assert!(is_safe_relative("Cargo.toml"));
    }

    #[test]
    fn rejects_absolute_and_parent_paths() {
        assert!(!is_safe_relative(""));
        assert!(!is_safe_relative("/etc/passwd"));
        assert!(!is_safe_relative("../outside"));
        assert!(!is_safe_relative("a/../../b"));
        assert!(!is_safe_relative("a/../b"));
    }

    #[test]
    fn resolves_inside_accepts_paths_in_root() {
        let dir = std::env::temp_dir().join("dspack_safety_ok");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(resolves_inside(&dir, "src/main.rs"));
        assert!(resolves_inside(&dir, "Cargo.toml"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolves_inside_rejects_escape() {
        let dir = std::env::temp_dir().join("dspack_safety_escape");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!resolves_inside(&dir, "../outside"));
        assert!(!resolves_inside(&dir, "/etc/passwd"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn case_mismatch_detected_for_wrong_case() {
        let dir = std::env::temp_dir().join("dspack_safety_case");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src").join("Main.rs"), "fn main() {}").unwrap();
        assert!(is_case_mismatch(&dir, "src/main.rs"));
        assert!(!is_case_mismatch(&dir, "src/Main.rs"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
