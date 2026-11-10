//! Plan: match parsed blocks against the current files and run the checks.
//! Pure: nothing is written until `apply::execute`.

use std::path::Path;

use crate::parse::ParsedBlock;
use crate::parse::block::BlockKind;
use crate::parse::label::Label;
use crate::safety;

/// Per-block status. See the table in FORMAT.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Create,
    Overwrite,
    Edit,
    NoMatch,
    MultipleMatches,
    UnsafePath,
    CaseMismatch,
    Missing,
    Suspicious,
    Truncated,
    Command,
    Ignored,
}

/// One entry in the plan: what a block means for the repo, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedBlock {
    /// Target path (empty for entries that don't touch the filesystem).
    pub path: String,
    pub kind: BlockKind,
    pub status: Status,
    pub detail: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub blocks: Vec<PlannedBlock>,
}

impl Plan {
    pub fn new() -> Self {
        Plan { blocks: Vec::new() }
    }

    /// Turn a model reply into a plan against the repo rooted at `root`.
    pub fn build(reply: &str, root: &Path) -> Plan {
        let parsed = crate::parse::parse(reply);
        let mut plan = Plan::new();
        let mut seen: Vec<String> = Vec::new();
        for pb in parsed {
            plan.blocks.push(plan_outcome(&pb, root, &mut seen));
        }
        plan
    }

    /// Indices of blocks that may be executed once accepted by the user.
    pub fn actionable(&self) -> Vec<usize> {
        self.blocks
            .iter()
            .enumerate()
            .filter_map(|(i, b)| {
                if b.status == Status::Create || b.status == Status::Edit {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }
}

fn plan_outcome(pb: &ParsedBlock, root: &Path, seen: &mut Vec<String>) -> PlannedBlock {
    let kind = pb.kind.clone();
    let path = pb.label.as_ref().map(|l| l.path.clone()).unwrap_or_default();

    if kind == BlockKind::Command {
        let cmd = pb.fence.content.trim().to_string();
        return PlannedBlock {
            path: cmd.clone(),
            kind,
            status: Status::Command,
            detail: format!("shell block collected, not executed: `{}`", cmd),
        };
    }
    if kind == BlockKind::Ignored {
        return PlannedBlock { path: String::new(), kind, status: Status::Ignored, detail: "unlabeled fence ignored".into() };
    }
    if kind == BlockKind::Truncated {
        return PlannedBlock { path, kind, status: Status::Truncated, detail: "fence was never closed".into() };
    }

    if pb.label.is_none() || !safety::is_safe_relative(&path) {
        return PlannedBlock { path: path.clone(), kind, status: Status::UnsafePath, detail: format!("unsafe or missing path label: {:?}", path) };
    }
    if !safety::resolves_inside(root, &path) {
        return PlannedBlock { path: path.clone(), kind, status: Status::UnsafePath, detail: format!("path escapes repo root: {}", path) };
    }
    if !seen.iter().any(|p| *p == path) {
        seen.push(path.clone());
    } else {
        return PlannedBlock { path, kind, status: Status::Suspicious, detail: "duplicate path in reply".into() };
    }

    if kind == BlockKind::FullFile {
        let content = &pb.fence.content;
        if let Some((status, detail)) = content_suspicion(content, pb.label.as_ref().unwrap()) {
            return PlannedBlock { path, kind, status, detail };
        }
        let exists = root.join(&path).exists();
        return PlannedBlock {
            path,
            kind,
            status: if exists { Status::Overwrite } else { Status::Create },
            detail: if exists { "full-file overwrite; diff shown, needs acceptance".into() } else { "new file".into() },
        };
    }

    // Edit.
    if !root.join(&path).exists() {
        return PlannedBlock { path: path.clone(), kind, status: Status::Missing, detail: format!("edit target missing: {}", path) };
    }
    if safety::is_case_mismatch(root, &path) {
        return PlannedBlock { path: path.clone(), kind, status: Status::CaseMismatch, detail: format!("case mismatch: {}", path) };
    }
    let content = std::fs::read_to_string(root.join(&path)).unwrap_or_default();
    let (search, _replace) = match edit_markers(&pb.fence.content) {
        Some(pair) => pair,
        None => return PlannedBlock { path, kind, status: Status::NoMatch, detail: "edit markers missing or malformed".into() },
    };
    let occurrences = count_occurrences(&content, &search);
    if occurrences == 0 {
        return PlannedBlock { path, kind, status: Status::NoMatch, detail: "search text not found".into() };
    }
    if occurrences > 1 {
        return PlannedBlock { path, kind, status: Status::MultipleMatches, detail: format!("search text matched {} times", occurrences) };
    }
    PlannedBlock { path, kind, status: Status::Edit, detail: "search matched exactly once".into() }
}

/// Suspicious-content checks: placeholder text, very short full-file content,
/// and low-confidence labels. Returns the flagged status when content is
/// suspect.
fn content_suspicion(content: &str, label: &Label) -> Option<(Status, String)> {
    let lower = content.to_lowercase();
    let placeholder = ["rest of file", "unchanged", "existing code", "todo", "fixme"]
        .iter()
        .any(|p| lower.contains(p));
    let placeholder = placeholder
        || content.lines().any(|l| {
            let t = l.trim();
            (t == "..." || t == "// .." || t == "# ..") && !t.is_empty()
        });
    if placeholder {
        return Some((Status::Suspicious, "placeholder content in block".into()));
    }
    if label.confidence == crate::parse::label::Confidence::Low {
        return Some((Status::Suspicious, "low-confidence path label".into()));
    }
    None
}

/// Extract the (SEARCH, REPLACE) pair from an edit fence's content.
fn edit_markers(content: &str) -> Option<(String, String)> {
    let lines: Vec<&str> = content.split('\n').collect();
    let s_idx = lines.iter().position(|l| l.trim_start().starts_with("<<<<<<< SEARCH"))?;
    let eq_idx = lines.iter().position(|l| l.trim_start() == "=======")?;
    let e_idx = lines.iter().position(|l| l.trim_start().starts_with(">>>>>>> REPLACE"))?;
    if s_idx > eq_idx || eq_idx > e_idx {
        return None;
    }
    let search = lines[s_idx + 1..eq_idx].join("\n");
    let replace = lines[eq_idx + 1..e_idx].join("\n");
    Some((search, replace))
}

/// Count occurrences of `needle` in `haystack` (line-ending normalized).
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    let hay = haystack.replace("\r\n", "\n");
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0usize;
    let mut start = 0usize;
    while let Some(pos) = hay[start..].find(needle) {
        count += 1;
        start += pos + needle.len();
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_file(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, content).unwrap();
    }

    #[test]
    fn full_file_create_when_missing() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let plan = Plan::build("**`src/new.rs`**\n\n```rust\nfn new() {}\n```\n", root);
        assert_eq!(plan.blocks.len(), 1);
        assert_eq!(plan.blocks[0].status, Status::Create);
        assert_eq!(plan.blocks[0].kind, BlockKind::FullFile);
    }

    #[test]
    fn full_file_overwrite_when_exists() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "src/main.rs", "fn old() {}\n");
        let plan = Plan::build("**`src/main.rs`**\n\n```rust\nfn new() {}\n```\n", root);
        assert_eq!(plan.blocks[0].status, Status::Overwrite);
    }

    #[test]
    fn full_file_suspicious_on_placeholder() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let plan = Plan::build("Sure!\n\n**`src/main.rs`**\n\n```rust\nrest of file\n```\n", root);
        assert_eq!(plan.blocks[0].status, Status::Suspicious);
    }

    #[test]
    fn full_file_unsafe_path() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let plan = Plan::build("Bad path:\n\n**`../evil.rs`**\n\n```rust\nfn evil() {}\n```\n", root);
        assert_eq!(plan.blocks[0].status, Status::UnsafePath);
    }

    #[test]
    fn edit_resolves_when_search_matches_once() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "src/app.rs", "fn old() {}\n");
        let reply = "Change it.\n\n**`src/app.rs`**\n\n```rust\n<<<<<<< SEARCH\nfn old() {}\n=======\nfn new() {}\n>>>>>>> REPLACE\n```\n";
        let plan = Plan::build(reply, root);
        assert_eq!(plan.blocks[0].status, Status::Edit);
    }

    #[test]
    fn edit_no_match_when_search_missing() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "src/app.rs", "fn old() {}\n");
        let reply = "Change it.\n\n**`src/app.rs`**\n\n```rust\n<<<<<<< SEARCH\nfn missing() {}\n=======\nfn new() {}\n>>>>>>> REPLACE\n```\n";
        let plan = Plan::build(reply, root);
        assert_eq!(plan.blocks[0].status, Status::NoMatch);
    }

    #[test]
    fn edit_missing_target() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let reply = "Change it.\n\n**`src/gone.rs`**\n\n```rust\n<<<<<<< SEARCH\nfn old() {}\n=======\nfn new() {}\n>>>>>>> REPLACE\n```\n";
        let plan = Plan::build(reply, root);
        assert_eq!(plan.blocks[0].status, Status::Missing);
    }

    #[test]
    fn command_and_ignored_blocks() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let reply = "Run:\n\n```bash\ncargo test\n```\n\n```rust\nfn main() {}\n```\n";
        let plan = Plan::build(reply, root);
        assert_eq!(plan.blocks[0].status, Status::Command);
        assert_eq!(plan.blocks[1].status, Status::Ignored);
    }
}
