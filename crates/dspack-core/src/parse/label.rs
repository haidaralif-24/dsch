//! Path-label detection: the nearest non-blank line before a fence
//! (at most one blank line gap). Styles in priority order are listed in
//! FORMAT.md; the canonical one is **`path`**.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// Path came from an in-block first-line comment. Always flagged.
    Low,
    /// A heading or "File: path" style label.
    Medium,
    /// The canonical bold-backticked path, or a backticked path alone.
    High,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub path: String,
    pub confidence: Confidence,
}

/// Detect the path label for the fence that opens at 1-based `fence_line`,
/// scanning the reply `lines` (as produced by splitting the reply on newlines)
/// and using the block's first content line for the in-block comment style.
pub fn detect<'a>(lines: &[&'a str], fence_line: usize, content_first_line: &str) -> Option<Label> {
    let fence_idx = fence_line.saturating_sub(1);
    let src = nearest_line(lines, fence_idx)?;

    let styles: [Option<(String, Confidence)>; 4] = [
        bold_backticked(src).map(|p| (p, Confidence::High)),
        bare_backticked(src).map(|p| (p, Confidence::High)),
        heading(src).map(|p| (p, Confidence::Medium)),
        file_style(src).map(|p| (p, Confidence::Medium)),
    ];

    for style in styles {
        if let Some((path, confidence)) = style {
            if is_valid_path(&path) {
                return Some(Label { path, confidence });
            }
        }
    }

    // Style 5: the first content line is a comment that names the path.
    comment_path(content_first_line).map(|p| Label { path: p, confidence: Confidence::Low })
}

/// Nearest non-blank line before the fence, allowing at most one blank line
/// between it and the fence. `fence_idx` is the 0-based index of the opening
/// fence line itself; we look strictly before it.
fn nearest_line<'a>(lines: &[&'a str], fence_idx: usize) -> Option<&'a str> {
    let mut i = fence_idx;
    let mut gap = 0usize;
    while i > 0 {
        i -= 1;
        if lines[i].trim().is_empty() {
            gap += 1;
            if gap > 1 {
                return None;
            }
        } else {
            return Some(lines[i]);
        }
    }
    None
}

/// ``**`src/main.rs`**`` — the canonical bold-backticked path.
fn bold_backticked(src: &str) -> Option<String> {
    let t = src.trim();
    if t.starts_with("**`") && t.ends_with("`**") && t.len() > 4 {
        let path = t[3..t.len() - 3].trim().to_string();
        if is_valid_path(&path) {
            return Some(path);
        }
    }
    None
}

/// `` `src/main.rs` `` alone on a line.
fn bare_backticked(src: &str) -> Option<String> {
    let t = src.trim();
    if t.len() >= 2 && t.starts_with('`') && t.ends_with('`') {
        let path = t[1..t.len() - 1].trim().to_string();
        if is_valid_path(&path) {
            return Some(path);
        }
    }
    None
}

/// A heading that contains a path: `### src/main.js` or `### \`src/main.js\``.
fn heading(src: &str) -> Option<String> {
    let t = src.trim();
    if !t.starts_with('#') {
        return None;
    }
    let path = t.trim_start_matches('#').trim().trim_start_matches('`').trim().to_string();
    if is_valid_path(&path) {
        return Some(path);
    }
    None
}

/// A `File: path` (or `**File:** path`) style label.
fn file_style(src: &str) -> Option<String> {
    let lower = src.trim().to_lowercase();
    let pos = lower.find("file:")?;
    let path = lower[pos + "file:".len()..].trim().to_string();
    let path = path.trim_start_matches('`').trim().to_string();
    if is_valid_path(&path) {
        return Some(path);
    }
    None
}

/// Style 5: the first content line is a comment that names the path.
fn comment_path(content_first_line: &str) -> Option<String> {
    let t = content_first_line.trim();
    let marker_len = ["//", "#", "<!--"]
        .iter()
        .filter(|m| t.starts_with(*m))
        .map(|m| m.len())
        .min()
        .unwrap_or(0);
    let path = t[marker_len..].trim().trim_start_matches('`').trim().to_string();
    if is_valid_path(&path) {
        return Some(path);
    }
    None
}

/// A string is treated as a path only if it is non-empty, has no whitespace,
/// and contains a `/` or a file extension.
fn is_valid_path(p: &str) -> bool {
    let t = p.trim();
    !t.is_empty() && !t.chars().any(char::is_whitespace) && (t.contains('/') || t.contains('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(reply: &str) -> Vec<&str> {
        reply.split('\n').map(|l| l.trim_end_matches('\r')).collect()
    }

    #[test]
    fn accepts_canonical_bold_backticked_path() {
        let ls = lines("**`src/main.rs`**\n```rust\nfn main() {}\n```");
        let label = detect(&ls, 2, "fn main() {}").unwrap();
        assert_eq!(label.path, "src/main.rs");
        assert_eq!(label.confidence, Confidence::High);
    }

    #[test]
    fn accepts_bare_backticked_path() {
        let ls = lines("Sure, do it.\n\n`src/app.rs`\n```rust\nfn app() {}");
        let label = detect(&ls, 4, "fn app() {}").unwrap();
        assert_eq!(label.path, "src/app.rs");
        assert_eq!(label.confidence, Confidence::High);
    }

    #[test]
    fn accepts_heading_with_path() {
        let ls = lines("### src/main.js\n```js\nfunction main() {}\n```");
        let label = detect(&ls, 2, "function main() {}").unwrap();
        assert_eq!(label.path, "src/main.js");
        assert_eq!(label.confidence, Confidence::Medium);
    }

    #[test]
    fn accepts_file_style_label() {
        let ls = lines("File: src/app.rs\n```rust\nfn app() {}");
        let label = detect(&ls, 2, "fn app() {}").unwrap();
        assert_eq!(label.path, "src/app.rs");
        assert_eq!(label.confidence, Confidence::Medium);
    }

    #[test]
    fn accepts_in_block_comment_as_low_confidence() {
        let ls = lines("Sure thing\n```js\n// src/main.js\nfunction main() {}");
        let label = detect(&ls, 2, "// src/main.js").unwrap();
        assert_eq!(label.path, "src/main.js");
        assert_eq!(label.confidence, Confidence::Low);
    }

    #[test]
    fn allows_one_blank_line_before_fence() {
        let ls = lines("Sure!\n\n**`src/main.rs`**\n```rust\nfn main() {}");
        let label = detect(&ls, 4, "fn main() {}").unwrap();
        assert_eq!(label.path, "src/main.rs");
        assert_eq!(label.confidence, Confidence::High);
    }

    #[test]
    fn rejects_gap_of_two_blank_lines() {
        let ls = lines("Sure!\n\n\n```rust\nfn main() {}");
        assert!(detect(&ls, 4, "fn main() {}").is_none());
    }

    #[test]
    fn rejects_paths_with_spaces() {
        let ls = lines("**`src/my file.rs`**\n```rust\nfn main() {}");
        assert!(detect(&ls, 2, "fn main() {}").is_none());
    }
}
