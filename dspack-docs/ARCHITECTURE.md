# Architecture

## Layout

A Cargo workspace with one library crate and a thin binary.

```
dspack/
├── Cargo.toml              # workspace
├── crates/
│   ├── dspack-core/        # library: all logic, no printing, no prompting
│   │   └── src/
│   │       ├── root.rs     # repo root discovery, config location
│   │       ├── config.rs   # .dspack.toml load/validate/default template
│   │       ├── pack/       # tree + file collection, markdown prompt assembly
│   │       │   ├── collect.rs   # walk, filter, select, token estimate
│   │       │   └── render.rs    # markdown output, dynamic fence length
│   │       ├── parse/      # model reply -> Vec<Block>
│   │       │   ├── fence.rs     # CommonMark fence scanning
│   │       │   ├── label.rs     # path-label detection and confidence
│   │       │   └── block.rs     # classify: file / edit / command / ignored
│   │       ├── plan.rs     # Vec<Block> -> Plan (status per block)
│   │       ├── apply.rs    # Plan -> disk (atomic, all-or-nothing)
│   │       ├── safety.rs   # path validation, symlink checks
│   │       ├── git.rs      # shell-out helpers
│   │       └── history.rs  # last-batch record for undo
│   └── dspack/             # binary: CLI + TUI front ends
│       └── src/
│           ├── main.rs
│           ├── cli.rs      # clap definitions, non-interactive mode
│           └── tui/        # ratatui app: state, events, render
└── tests/fixtures/         # real model replies + expected plans
```

The scaffold starts with `main.rs` only; `cli.rs` and `tui/` are added as the front ends grow.

**Rule:** the core library never prints, prompts, or reads a terminal. It takes data in and returns data out. The TUI and the plain CLI are both front ends over the same functions.

## Root discovery and config

Per-repo only, no global config.

1. Walk up from the current directory looking for `.dspack.toml`. The nearest one wins (handles a subproject in a monorepo).
2. If none is found, walk up looking for `.git`. If found, show the path and ask to create `.dspack.toml` there. Write a commented default only after confirmation.
3. If neither exists, fail with "not inside a repo". Never treat an arbitrary directory (home, `/tmp`) as a root.
4. Prompt only when stdin is a terminal (`std::io::IsTerminal`); otherwise error with a hint to run interactively once or pass `--yes`.
5. Always print the detected root at the top of every command.

If the user declines creation, exit. Behaving differently without a config is a source of confusing bugs.

### Config contents

- extra exclude patterns
- files always included in full
- a custom instruction line appended to every prompt
- token budget warning threshold
- the output-rules text (built-in default, overridable per project)
- suspicious-content thresholds (short-content ratio, placeholder patterns)
- format mode: `markdown` (default) or `tags` (optional strict mode)

## Data flow

### Pack

```
walk repo (ignore crate, .gitignore + excludes)
  -> user selects files (TUI picker or flags)
  -> render markdown: tree, labeled fenced files, task, output rules LAST
       (fence length per file = max(3, longest backtick run + 1))
  -> token estimate (bytes / 4) vs budget
  -> clipboard (or --stdout)
```

### Apply

```
clipboard (or --stdin)
  -> parse: Vec<Block>              (pure, forgiving, fence-aware)
  -> plan:  Plan                    (pure: match against current files, run checks)
  -> review: user accepts/rejects blocks (TUI) or --yes
  -> execute: write atomically, record batch for undo
```

**Plan, then execute** is the core design rule. `plan` computes, for every block, the target path, the match count, the resulting content, and a status (see [FORMAT.md](FORMAT.md)). Nothing is written until `execute`.

## Parser

A hand-written, line-based state machine in three stages.

1. **Fence scan** (`fence.rs`). Walk the reply line by line. An opening fence is 3+ backticks (or `~~~`) plus an optional info string; a closing fence uses the same character, is at least as long, and has only whitespace after it. Content between them is captured verbatim, along with the info string and the line number. An unclosed fence at end of input yields a `Truncated` block instead of a guess. Prose between fences is kept only as potential label text.
2. **Label detection** (`label.rs`). For each fenced block, look at the nearest non-blank line before it (at most one blank line gap) and try the accepted label styles in priority order. Each hit carries a confidence level. Strings must look like paths. Labels matching a file in the tree win ties. The in-block first-line comment style is the lowest confidence.
3. **Classification** (`block.rs`).
   - info string `bash`/`sh`/`shell`/`zsh`/`console`: `Command` block, never applied. Simple `rm path` lines also produce a proposed deletion.
   - labeled, contains SEARCH/REPLACE markers: `Edit`.
   - labeled, no markers: full-file block (`Create` or `Overwrite` after planning).
   - unlabeled: `Ignored`, counted for display.

The parser never touches the filesystem; checking whether files exist belongs to `plan`.

### Suspicious-content checks (in `plan`)

Overwrites of existing files are the dangerous case, so the plan flags: content much shorter than the existing file, placeholder patterns, duplicate paths within one reply, and low-confidence labels. Flagged blocks are deselected by default, and overwrites are excluded from any blanket "accept all".

## Safety model

Model output is untrusted input.

- Reject absolute paths and any path containing `..`.
- Canonicalize the repo root once. Canonicalize each target's parent directory and require it to start with the root. This catches symlinks that point outside the repo.
- Reject writes through symlinks that leave the root.
- Resolve paths through the filesystem, not string comparison (APFS is case-insensitive; Unicode may be decomposed).
- Each search block must match **exactly once**.
- **All-or-nothing:** if any accepted block fails validation, abort the whole batch.
- Write atomically: temp file in the same directory, then rename.
- Before applying, require a clean git state or auto-stash, so undo is always available.
- **Shell commands from the model are never executed.** They are displayed for the user to run. The only command-derived action is a simple `rm path`, converted into a deletion that `dspack` performs itself, with confirmation, inside the root.
- Record each applied batch in `.dspack/last.json` so `dspack undo` reverts exactly what changed, even if git state is messy.

## Matching policy

- Normalize line endings before matching (CRLF files).
- Exact match is the default.
- Optional whitespace-insensitive fallback that still requires a unique match and always shows the diff.
- Case-only path mismatches are an error or warning, never silently accepted.

How forgiving matching should be is a feel decision to settle after real sessions; fixtures will drive it.

## TUI design

State plus events: one `App` struct, an event loop handling key events, and a render function drawing purely from state.

### Screens

- **Pack:** file list with checkboxes (pre-selected from config), live token estimate against the budget, task input box, copy action.
- **Apply:** blocks listed with file, kind, and status; a diff pane for the selected block; per-block accept/reject toggles. Failed and flagged blocks show in red or yellow with the reason. A side section lists suggested shell commands and the ignored-block count.
- **History:** the last applied batch with one-key revert.

### Practical concerns

- Install a panic hook and use a guard type so the terminal is always restored from raw mode.
- Read the clipboard directly in `apply`; don't rely on terminal paste. Enable bracketed paste anyway for the task input box.
- Use crossterm through `ratatui::crossterm` so its version always matches ratatui's (see TECH_STACK.md).
- Render only the visible window of large diffs.
- Test rendered screens with `TestBackend`.
- Keep a non-interactive mode (`--stdin`, `--stdout`, `--yes`) for SSH and scripting. The TUI is the default front end, not the only one.

## Conversation hygiene (prompt side)

- Output rules go at the **end** of the prompt, where models follow them best.
- Ask for a short plan first, then the files.
- On follow-up turns, send only changed files (`git diff --name-only`) plus the tree.
- Start fresh chats often and repeat the instructions; long chats degrade format adherence.
- Always send current file contents; the model's earlier picture goes stale after edits.
- Copy replies with the web UI's own copy button so fence markers and path labels survive. Confirm this behavior early.
