//! CommonMark fence scanning.
//!
//! An opening fence is 3+ backticks (or `~~~`) plus an optional info string.
//! A closing fence uses the same character, is at least as long as the
//! opening one, and has only whitespace after it. An unclosed fence at end of
//! input is reported as truncated, never guessed at.

/// A fenced code block found in a model reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FencedBlock {
    /// Info string after the opening fence (usually the language).
    pub info: String,
    /// Content between the fences, verbatim.
    pub content: String,
    /// 1-based line number of the opening fence.
    pub start_line: usize,
    /// False if the reply ended before the fence was closed.
    pub closed: bool,
}

/// Scan a reply for fenced blocks.
pub fn scan(reply: &str) -> Vec<FencedBlock> {
    let lines: Vec<&str> = reply.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let mut blocks = Vec::new();
    let mut open_char: Option<char> = None;
    let mut open_len = 0usize;
    let mut open_info = String::new();
    let mut open_start = 0usize;
    let mut open_content: Vec<String> = Vec::new();

    for (idx, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        let n = idx + 1;

        if let Some(fence_char) = open_char {
            // Inside a fence: does this line close it?
            if let Some((c, run)) = leading_char_run(line) {
                let is_close = c == fence_char
                    && run >= open_len
                    && line.get(run..).map(|rest| rest.trim().is_empty()).unwrap_or(true);
                if is_close {
                    blocks.push(FencedBlock {
                        info: open_info.clone(),
                        content: open_content.join("\n"),
                        start_line: open_start,
                        closed: true,
                    });
                    open_char = None;
                    continue;
                }
            }
            open_content.push((*raw).to_string());
        } else if let Some((c, run)) = leading_char_run(line) {
            if (c == '`' || c == '~') && run >= 3 {
                open_char = Some(c);
                open_len = run;
                open_info = line[run..].trim().to_string();
                open_start = n;
                open_content = Vec::new();
            }
        }
    }

    if let Some(fence_char) = open_char {
        let _ = fence_char;
        blocks.push(FencedBlock {
            info: open_info,
            content: open_content.join("\n"),
            start_line: open_start,
            closed: false,
        });
    }

    blocks
}

/// Leading character run of `s`, starting at the first char, with its length.
fn leading_char_run(s: &str) -> Option<(char, usize)> {
    let first = s.chars().next()?;
    let mut len = 0;
    for c in s.chars() {
        if c == first {
            len += 1;
        } else {
            break;
        }
    }
    Some((first, len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_empty_for_reply_without_fences() {
        assert!(scan("Just prose, no code here.").is_empty());
    }

    #[test]
    fn scans_single_labeled_block() {
        let reply = "Sure thing!\n```rust\nfn main() {}\n```\n";
        let blocks = scan(reply);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].info, "rust");
        assert_eq!(blocks[0].content, "fn main() {}");
        assert_eq!(blocks[0].start_line, 2);
        assert!(blocks[0].closed);
    }

    #[test]
    fn closing_fence_must_be_at_least_as_long_as_opening() {
        // Opening fence is four backticks; a three-backtick line is content,
        // and only a four-backtick line closes.
        let reply = "````\ncode\n```\nmore\n````\n";
        let blocks = scan(reply);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "code\n```\nmore");
        assert!(blocks[0].closed);
    }

    #[test]
    fn supports_tilde_fences() {
        let reply = "~~~\nplain text\n~~~\n";
        let blocks = scan(reply);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].info, "");
        assert_eq!(blocks[0].content, "plain text");
        assert!(blocks[0].closed);
    }

    #[test]
    fn unclosed_fence_is_reported_truncated() {
        let blocks = scan("```rust\nfn main() {}");
        assert_eq!(blocks.len(), 1);
        assert!(!blocks[0].closed);
        assert_eq!(blocks[0].content, "fn main() {}");
    }

    #[test]
    fn keeps_content_verbatim() {
        let reply = "```text\n  keep   spacing  \n\nsecond line\n```\n";
        let blocks = scan(reply);
        assert_eq!(blocks[0].content, "  keep   spacing  \n\nsecond line");
    }

    #[test]
    fn scans_multiple_blocks() {
        let reply = "```rust\nfn a() {}\n```\n\nProse between.\n\n```bash\ncargo test\n```\n";
        let blocks = scan(reply);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].info, "rust");
        assert_eq!(blocks[1].info, "bash");
        assert_eq!(blocks[1].start_line, 7);
    }
}
