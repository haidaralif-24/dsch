//! Parse a model reply into blocks. Hand-written, line-based, fence-aware.
//!
//! Three stages: fence scan -> label detection -> classification.
//! The parser never touches the filesystem.

pub mod block;
pub mod fence;
pub mod label;

use crate::parse::block::BlockKind;
use crate::parse::fence::FencedBlock;
use crate::parse::label::Label;

/// One fenced block from a reply, with its label and kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedBlock {
    /// The scanned fence (content, position, closed-ness).
    pub fence: FencedBlock,
    /// The path label detected before the fence, if any.
    pub label: Option<Label>,
    /// What the block means: file, edit, command, ignored, or truncated.
    pub kind: BlockKind,
}

/// Parse a model reply into blocks.
///
/// Stages, in order: fence scan, then label detection, then classification.
/// The parser never touches the filesystem.
pub fn parse(reply: &str) -> Vec<ParsedBlock> {
    let blocks = fence::scan(reply);
    let lines: Vec<&str> = reply.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    blocks
        .into_iter()
        .map(|fence| {
            let content_first_line = fence.content.split('\n').next().unwrap_or("").to_string();
            let label = label::detect(&lines, fence.start_line, &content_first_line);
            let kind = block::classify(&fence, label.as_ref());
            ParsedBlock { fence, label, kind }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_single_full_file_block() {
        let reply = "Sure!\n\n**`src/main.rs`**\n\n```rust\nfn main() {}\n```\n";
        let blocks = parse(reply);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, block::BlockKind::FullFile);
        assert_eq!(blocks[0].label.as_ref().unwrap().path, "src/main.rs");
        assert_eq!(blocks[0].fence.content, "fn main() {}");
    }

    #[test]
    fn parses_edit_block_from_markers() {
        let reply = "I'll make the change.\n\n**`src/app.rs`**\n\n```rust\n<<<<<<< SEARCH\nold\n=======\nnew\n>>>>>>> REPLACE\n```\n";
        let blocks = parse(reply);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, block::BlockKind::Edit);
    }

    #[test]
    fn parses_command_block_ignored_by_default() {
        let reply = "Run this:\n\n```bash\ncargo test\n```\n";
        let blocks = parse(reply);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, block::BlockKind::Command);
        assert!(blocks[0].label.is_none());
    }

    #[test]
    fn parses_unlabeled_prose_block_as_ignored() {
        let reply = "```rust\nfn main() {}\n```\n";
        let blocks = parse(reply);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, block::BlockKind::Ignored);
    }
}
