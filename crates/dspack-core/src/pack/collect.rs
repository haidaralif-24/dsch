//! Walk the repo (the `ignore` crate), apply excludes, select files.
//!
//! TODO(M2): tree + file collection.

/// Rough token estimate: bytes / 4. Good enough for a budget warning;
/// a real tokenizer for DeepSeek isn't worth the dependency.
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_by_bytes() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }
}
