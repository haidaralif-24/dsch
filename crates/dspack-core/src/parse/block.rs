//! Classify fenced blocks into what they mean for the plan.

use crate::parse::fence::FencedBlock;
use crate::parse::label::Label;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    /// Labeled fence, no SEARCH/REPLACE markers: complete file contents.
    FullFile,
    /// Labeled fence with SEARCH/REPLACE markers: edit an existing file.
    Edit,
    /// bash/sh/shell/zsh/console fence. Listed only, never executed.
    Command,
    /// Unlabeled fence (illustrative snippet). Counted, never applied.
    Ignored,
    /// Fence was never closed.
    Truncated,
}

/// Info strings that mark a fence as a shell command, never as a file.
pub fn is_command_info(info: &str) -> bool {
    let t = info.trim().to_lowercase();
    matches!(t.as_str(), "bash" | "sh" | "shell" | "zsh" | "console")
}

/// Does the fence content carry SEARCH/REPLACE edit markers?
pub fn has_search_replace(content: &str) -> bool {
    content.contains("<<<<<<<") && content.contains("=======") && content.contains(">>>>>>>")
}

/// Turn a scanned fence plus its detected label into a block kind.
pub fn classify(fence: &FencedBlock, label: Option<&Label>) -> BlockKind {
    if !fence.closed {
        return BlockKind::Truncated;
    }
    if is_command_info(&fence.info) {
        return BlockKind::Command;
    }
    if label.is_none() {
        return BlockKind::Ignored;
    }
    if has_search_replace(&fence.content) {
        BlockKind::Edit
    } else {
        BlockKind::FullFile
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fence::FencedBlock;

    fn fence(info: &str, content: &str, closed: bool) -> FencedBlock {
        FencedBlock {
            info: info.to_string(),
            content: content.to_string(),
            start_line: 1,
            closed,
        }
    }

    fn label(path: &str) -> Label {
        Label { path: path.to_string(), confidence: crate::parse::label::Confidence::High }
    }

    #[test]
    fn full_file_without_markers() {
        let f = fence("rust", "fn main() {}", true);
        assert_eq!(classify(&f, Some(&label("src/main.rs"))), BlockKind::FullFile);
    }

    #[test]
    fn edit_when_search_replace_markers_present() {
        let f = fence("markdown", "<<<<<<< SEARCH\nold\n=======\nnew\n>>>>>>> REPLACE", true);
        assert_eq!(classify(&f, Some(&label("src/main.rs"))), BlockKind::Edit);
    }

    #[test]
    fn command_info_is_never_a_file() {
        let f = fence("bash", "cargo test", true);
        assert_eq!(classify(&f, Some(&label("src/main.rs"))), BlockKind::Command);
        let f = fence("console", "npm run build", true);
        assert_eq!(classify(&f, Some(&label("src/main.rs"))), BlockKind::Command);
    }

    #[test]
    fn unlabeled_fence_is_ignored() {
        let f = fence("rust", "fn main() {}", true);
        assert_eq!(classify(&f, None), BlockKind::Ignored);
    }

    #[test]
    fn unclosed_fence_is_truncated() {
        let f = fence("rust", "fn main() {}", false);
        assert_eq!(classify(&f, Some(&label("src/main.rs"))), BlockKind::Truncated);
    }
}
