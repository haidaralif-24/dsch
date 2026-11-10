//! Repo root discovery.
//!
//! 1. Nearest `.dspack.toml` walking up wins (handles monorepo subprojects).
//! 2. Otherwise the nearest `.git`; the front end then asks before creating
//!    a config there.
//! 3. Otherwise refuse. Never treat an arbitrary directory as a root.

use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = ".dspack.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootLookup {
    /// A config file was found; its directory is the project root.
    Configured(PathBuf),
    /// No config, but inside a git repo. The front end should offer to
    /// create the config at this path.
    GitRepoWithoutConfig(PathBuf),
    /// Not inside a repo. Refuse to run.
    NotARepo,
}

pub fn discover(start: &Path) -> RootLookup {
    for dir in start.ancestors() {
        if dir.join(CONFIG_FILE).is_file() {
            return RootLookup::Configured(dir.to_path_buf());
        }
    }
    for dir in start.ancestors() {
        // `.git` can be a file (worktrees, submodules), so use exists().
        if dir.join(".git").exists() {
            return RootLookup::GitRepoWithoutConfig(dir.to_path_buf());
        }
    }
    RootLookup::NotARepo
}
