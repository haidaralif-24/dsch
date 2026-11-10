# Prompt and Output Format

Markdown in both directions. `pack` sends the repo as markdown; the model answers in its natural markdown style; `apply` parses that reply.

The design leans on what chat models already do: a path label, then a fenced code block. Showing the model the same shape in the prompt makes it likely to answer in kind, which is better than describing a custom format and hoping. This is a first draft; real model replies saved as fixtures should drive every change.

## Fence rules (CommonMark)

Both the packer and the parser follow the same rules, so files containing triple backticks (READMEs, docs, Markdown-heavy code) don't break anything.

- An opening fence is a run of 3 or more backticks (the parser also accepts `~~~`), followed by an optional info string such as the language.
- A closing fence uses the same character, is **at least as long** as the opening fence, and has nothing after it but whitespace.
- **Packer:** fence length = `max(3, longest backtick run inside the file content + 1)`.
- **Parser:** applies the same closing rule, so a four-backtick fence containing three-backtick blocks is read as one block. Models do this on their own when nesting.
- An unclosed fence at the end of the reply is reported as truncated, never guessed at.

## Outgoing: packed prompt

Context first, instructions last.

`````markdown
## Project: my-repo

### Tree

```text
src/
  main.rs
Cargo.toml
README.md   (not included)
```

### Files

**`src/main.rs`**

```rust
fn main() {}
```

**`docs/GUIDE.md`**

````markdown
# Guide

```bash
cargo run
```
````

### Task

Describe what you want done here.

### Output rules

...the instruction block below...
`````

- Files shown in the tree but not included as file blocks are marked `(not included)` so the model doesn't guess their contents.
- The language tag comes from the file extension, with `text` as the fallback.
- One canonical path-label style: a bold, backticked path on its own line, directly before the fence. The output rules ask the model to answer in that same style.
- Binaries, lockfiles, build output, and anything huge are excluded by default.
- Task and rules go last, where models follow them best.

## Incoming: model reply

The reply is ordinary chat markdown. The parser only acts on labeled fenced blocks and treats everything else (headings, explanations, closing questions) as prose.

### Path labels

A fenced block counts as a file or an edit only if it has a path label. A label is the nearest non-blank line directly before the fence (at most one blank line between). Accepted styles, in priority order:

1. **`path`** — the canonical style we ask for
2. `` `path` `` alone on a line
3. A heading containing a path (`### src/main.js`, or ``### `src/main.js` ``)
4. `File: path`, `**File:** path`, or similar
5. *Low confidence, always flagged:* the first line of the block is a comment containing only a path (`// src/main.js`, `# src/main.py`, `<!-- index.html -->`)

A string is treated as a path only if it has no spaces (unless backticked) and contains a `/` or a file extension. Labels that match a file in the tree are preferred over ones that don't.

### Block kinds

**Full file.** A labeled fence with no SEARCH/REPLACE markers is the complete contents of that file.

- Target doesn't exist: create it.
- Target exists: it's an **overwrite**. Always shown as a diff and requires explicit acceptance.

**Edit.** A labeled fence containing SEARCH/REPLACE markers edits an existing file:

````markdown
**`src/main.js`**

```js
<<<<<<< SEARCH
function old() {}
=======
function renamed() {}
>>>>>>> REPLACE
```
````

- One fence may hold several SEARCH/REPLACE pairs, applied in order.
- Each SEARCH text must match **exactly once** in the current file (after line-ending normalization).
- To insert, search for a nearby unique line and replace it with itself plus the new lines.

**Shell commands.** Fences tagged `bash`, `sh`, `shell`, `zsh`, or `console` are never treated as files and **never executed**. They're collected into a "suggested commands" list for the user to run manually.

- Simple deletions like `rm path/to/file` are recognized and offered as proposed deletions. They are carried out by `dspack` itself, only on confirmation, and only inside the repo root.
- Anything with flags like `-r` or `-f`, globs, pipes, or substitutions is shown as a plain command and never converted into a deletion.

**Everything else.** Unlabeled fences (illustrative snippets, output samples, config examples) are ignored. The apply screen shows a count of ignored blocks so nothing disappears silently.

## Suspicious-content checks

The worst failure mode is an illustrative snippet being treated as a full file and overwriting real code. Defenses:

- A path label is required; unlabeled fences are never applied.
- Overwrites always show a diff and need explicit acceptance, never covered by a blanket "accept all".
- **Short-content warning:** the new content is much smaller than the existing file (configurable ratio).
- **Placeholder warning:** content contains patterns like `rest of file`, `unchanged`, `existing code`, or a line that is only `...` or `// ...`.
- **Duplicate warning:** the same path appears in more than one block of the reply. Blocks are never merged silently; the user chooses.
- **Low-confidence label warning:** the path came from an in-block comment (style 5).

Flagged blocks are deselected by default.

## Block statuses

Each block in the plan carries one status:

| Status | Meaning |
| --- | --- |
| `Create` | New file, ready to write |
| `Overwrite` | Full-file block targeting an existing file; needs explicit acceptance |
| `Edit` | All SEARCH blocks matched exactly once |
| `NoMatch` | SEARCH text not found |
| `MultipleMatches` | SEARCH text found more than once |
| `UnsafePath` | Absolute path, `..`, or escapes the root via symlink |
| `CaseMismatch` | Path differs from the real file only by case |
| `Missing` | Edit or deletion targets a file that doesn't exist |
| `Suspicious` | Placeholder, short-content, duplicate, or low-confidence-label warning |
| `Truncated` | Fence never closed |
| `Command` | Shell block, listed only, never applied |
| `Ignored` | Unlabeled fence, counted only |

## Suggested instruction block

Appended to every packed prompt (overridable per repo in `.dspack.toml`):

```text
First write a short plan in plain prose.
Then give every changed or new file in this exact style: a line with the
relative path in bold backticks, like **`src/main.rs`**, immediately followed
by a fenced code block.
- For a new file or a full rewrite, put the complete file in the block.
- For a small change to an existing file, use SEARCH/REPLACE markers
  (<<<<<<< SEARCH, =======, >>>>>>> REPLACE) inside the block. The SEARCH text
  must match the existing file exactly and appear exactly once.
Never abbreviate with placeholders like "rest of file unchanged".
Paths are relative to the project root. If a file contains triple backticks,
use a longer fence. Put shell commands in separate bash blocks and do not
mix them into file blocks.
```

## Getting the raw markdown out of the chat

The web UI renders markdown. Selecting rendered text with the mouse can drop the fence markers and the bold path labels, which breaks parsing. Use the message's own copy button, which should copy the raw markdown. Verify this on the current DeepSeek UI early, because the whole workflow depends on it. If the copy button strips formatting, per-code-block copy buttons plus a path-in-first-line convention become the fallback.

## Optional strict mode

For models that follow rigid formats well, a config setting can switch to explicit tags (`<edit path="...">`, `<create path="...">`, `<delete path="..." />`) with the same SEARCH/REPLACE markers inside `<edit>`. It's off by default because chat UIs may swallow unknown tags in the rendered view, and markdown-native output avoids that. Not a priority; build it only if markdown parsing proves unreliable.

## Fixtures

Every real reply that parses wrongly becomes a fixture under `tests/fixtures/`, along with the expected plan. The first one is a typical greenfield reply: a scaffold shell block with an `rm` line, three labeled full-file blocks (`index.html`, `src/main.js`, `src/style.css`), a run shell block, and a closing prose section.
