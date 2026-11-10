//! Per-repo config (`.dspack.toml`). No global config.
//!
//! TODO(M0): a `Config` struct with serde, loading, and validation.

/// Written to `.dspack.toml` after the user confirms creation.
pub const DEFAULT_TEMPLATE: &str = r##"# dspack config (per repository). Everything is optional.

# Extra patterns to exclude from packing, on top of .gitignore.
exclude = [".DS_Store", "target", "node_modules", "*.lock"]

# Files that are always included in full.
always_include = []

# Extra instruction line appended to every prompt.
extra_instruction = ""

# Warn when the packed prompt exceeds this many estimated tokens.
token_budget = 60000

# Output format: "markdown" (default) or "tags" (optional strict mode).
format = "markdown"

[checks]
# Warn when new content is smaller than this fraction of the existing file.
short_content_ratio = 0.5
# Warn when content contains any of these (case-insensitive).
placeholder_patterns = ["rest of file", "unchanged", "existing code"]
"##;
