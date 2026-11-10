//! Render the packed prompt as markdown: tree, labeled fenced files, task,
//! output rules last. See FORMAT.md.
//!
//! TODO(M2): full prompt rendering.

/// The fence to wrap `content` in: one backtick longer than the longest
/// backtick run inside it, minimum three. Files containing code blocks
/// (READMEs, docs) therefore never break the prompt.
pub fn fence_for(content: &str) -> String {
    let mut longest = 0usize;
    let mut run = 0usize;
    for ch in content.chars() {
        if ch == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    "`".repeat((longest + 1).max(3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_content_gets_three_backticks() {
        assert_eq!(fence_for("fn main() {}"), "```");
        assert_eq!(fence_for(""), "```");
    }

    #[test]
    fn content_with_a_fence_gets_a_longer_one() {
        assert_eq!(fence_for("```bash\nls\n```"), "````");
    }

    #[test]
    fn longest_run_wins() {
        assert_eq!(fence_for("`a` ````` b ``"), "``````");
    }
}
