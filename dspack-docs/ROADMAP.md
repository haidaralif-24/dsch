# Roadmap

Build order: **core library and CLI first, TUI on top.** Starting with the TUI tangles parsing and apply logic into UI code, which is hard to undo.

## M0: Skeleton and format check

- [x] Cargo workspace (`dspack-core`, `dspack`), module stubs, all dependencies, and `git init` via `scaffold.sh`
- [ ] Root discovery: nearest `.dspack.toml`, else `.git` with a create-config prompt, else refuse (`root::discover` exists; the prompt and wiring remain)
- [ ] Config load with a commented default template (the template constant exists; the `Config` struct and loading remain)
- [ ] Always print the detected root
- [ ] **Verify the web UI's copy button yields raw markdown** (fences and bold path labels intact). The whole workflow depends on this.
- [ ] Save the first fixture: a typical greenfield reply (scaffold shell block with `rm`, three labeled full-file blocks, run commands, closing prose)

## M1: Apply core (the risky part)

- [ ] Fence scanner with CommonMark rules (closing fence at least as long, tilde support, truncation detection)
- [ ] Path-label detection with confidence levels
- [ ] Block classification: full file, edit (SEARCH/REPLACE), shell command, ignored
- [ ] Fixtures from real DeepSeek replies, snapshot-tested with `insta`
- [ ] Plan builder with per-block statuses and suspicious-content checks (short content, placeholders, duplicates, low-confidence labels)
- [ ] Safety checks: path validation, symlink escape, case mismatch (`safety::is_safe_relative` covers absolute and `..` paths; resolved-path and symlink checks remain)
- [ ] Atomic, all-or-nothing execute
- [ ] CLI `apply --stdin` with a dry-run diff
- [ ] Clean-git or auto-stash check
- [ ] Suggested-commands list, with simple `rm` offered as a confirmed deletion

**Milestone goal:** run `apply` on real model output and trust it.

## M2: Pack

- [ ] Tree + file walking with `ignore` and config excludes
- [ ] Markdown rendering with dynamic fence length (longest backtick run + 1, minimum 3)
- [ ] `(not included)` markers for omitted files in the tree
- [ ] Output-rules block appended last, overridable per repo
- [ ] Token estimate and budget warning
- [ ] CLI `pack` with flags for file selection, output to clipboard or `--stdout`
- [ ] Round-trip test: pack a repo, feed it through a fake "model" that echoes labeled blocks, parse it back

## M3: History and undo

- [ ] Record applied batches in `.dspack/last.json`
- [ ] `dspack undo`

## M4: TUI

- [ ] Terminal guard and panic hook
- [ ] Keep `ratatui` pinned to 0.29 while `tui-textarea` 0.7 is in use; use crossterm via `ratatui::crossterm`
- [ ] Apply screen: block list, diff pane, per-block accept/reject, commands panel
- [ ] Pack screen: file picker with checkboxes, live token count, task input
- [ ] History screen
- [ ] `TestBackend` snapshots

## Later / maybe

- Whitespace-insensitive matching fallback (decide after real sessions)
- Follow-up mode: send only changed files plus the tree
- Optional strict tag mode (`<edit>`, `<create>`, `<delete>`) if markdown parsing proves unreliable
- OSC 52 clipboard for SSH
- `cap-std` for path enforcement if manual checks feel fragile
- Syntax highlighting in diffs
- `proptest` fuzzing of the parser

## Open questions

- How forgiving should matching and label detection be? Fixtures will decide.
- Which label styles does DeepSeek actually use most? Collect real replies before trimming the list.
- Should `.dspack.toml` be committed or git-ignored by default? Currently offered as a prompt on first run.
- Per-block accept/reject versus all-or-nothing in the default TUI flow.
- Keep `tui-textarea` (which pins `ratatui` to 0.29) or hand-roll the task input and move to the latest `ratatui`?
- Final project name.
