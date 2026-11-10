# Tech Stack

Language: **Rust**. A single static binary, fast tree walking, and strict types that keep the parser honest. Primary platform: **macOS**.

The tool is mostly I/O and string handling, so the stack is deliberately small. There is **no async runtime**; nothing here is network-bound.

## Dependencies

| Concern | Choice | Notes |
| --- | --- | --- |
| CLI | `clap` (derive) | Subcommands and flags |
| File walking | `ignore` | Engine behind ripgrep; handles `.gitignore` plus custom excludes |
| Config | `serde` + `toml` | Default config is a commented template string, so `toml_edit` is not needed |
| Errors | `thiserror` (library), `anyhow` (binary) | Typed errors matter: "block 3 in `src/app.rs` matched 0 times" is the most common message |
| Diff | `similar` | Generates the diffs shown before applying |
| TUI | `ratatui` (pinned to 0.29) | Standard stack, works on macOS, Linux, Windows. Use crossterm through `ratatui::crossterm`, not as a separate dependency |
| Text input | `tui-textarea` 0.7 | For typing the task description inside the TUI. Requires ratatui 0.29; fallback is a hand-rolled input box |
| Fuzzy search | `nucleo` | File picker filtering (unused until M4) |
| Prompts (non-TUI mode) | `dialoguer` | First-run config confirmation, plain CLI confirms |
| Atomic writes | `tempfile` | Write in the target's directory, then persist |
| Clipboard | `arboard` | Native pasteboard on macOS |
| Colors (non-TUI) | `owo-colors` | Plain CLI output |

## Deliberate decisions

- **Git: shell out to the `git` binary** instead of `git2` or `gix`. Only status, stash, and `ls-files` are needed. Avoids a heavy dependency and behaves like the user's own git. Porcelain output is stable.
- **Clipboard: read directly, don't rely on terminal paste.** Terminals deliver paste as bursts of key events unless bracketed paste is enabled. Reading the pasteboard via `arboard` is more reliable. `pbcopy`/`pbpaste` are the fallback. Support `--stdin`/`--stdout` for headless and SSH use. OSC 52 is a later nice-to-have.
- **Parser: hand-written line-based state machine.** No regex, parser-combinator, or Markdown crate. It implements the few CommonMark rules that matter (a closing fence is at least as long as the opening one), detects path labels before fences, and recognizes SEARCH/REPLACE markers. Everything else is prose. Forgiving of chatter around blocks and easy to unit test. If it ever needs full Markdown semantics, `pulldown-cmark` is the fallback.
- **Packer: markdown with dynamic fence length.** Each file's fence is one backtick longer than the longest backtick run inside it (minimum three), so files containing code blocks never break the prompt.
- **Path safety: manual canonicalize-and-check.** Consider `cap-std` later if this feels fragile.
- **Token estimate: bytes / 4.** `tiktoken-rs` uses OpenAI tokenizers, which don't match DeepSeek's.
- **Per-repo config only.** No global config file, no environment-wide state.
- **TUI crate versions are pinned together.** `tui-textarea` 0.7 depends on ratatui 0.29, so ratatui is pinned to 0.29 to avoid two ratatui versions in one build (which causes type mismatches). Crossterm is used via `ratatui::crossterm`, which keeps it on the matching version. Bump them together when `tui-textarea` supports a newer ratatui, or drop it for a hand-rolled input box.
- **Versions come from `cargo add`** at scaffold time (latest compatible), and `Cargo.lock` is committed.

## Testing

- `insta` snapshot tests over real model outputs saved as fixtures. Collect these from day one.
- `proptest` (installed as a dev-dependency) for malformed-input hardening of the parser, once the basic parser works.
- `ratatui::backend::TestBackend` for rendered-screen snapshots.
- Plan logic is pure and testable without any UI.

## macOS notes

- APFS is case-insensitive but case-preserving. Resolve paths through the filesystem instead of comparing strings, and treat case-only mismatches as an error or warning.
- Unicode paths can be stored decomposed; same remedy.
- Default excludes include `.DS_Store`, `.git`, `target`, `node_modules`.
- Terminal.app lacks truecolor; stick to the 256-color palette there. iTerm2, Ghostty, WezTerm, and kitty are better targets.

## Explicitly skipped for now

- `tree-sitter` smart context selection (big scope jump)
- Syntax highlighting in diffs (`syntect` is heavy; plain red/green first)
- Plugin system
- Any browser automation or reverse-engineered web protocol
