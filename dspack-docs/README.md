# dspack

> Working name. A terminal tool that makes a plain web chatbot (like chat.deepseek.com) usable as a coding assistant, using nothing but copy and paste.

DeepSeek's web chat is powerful and free, but it can't see your files and can't edit them. `dspack` closes that gap without automating the browser:

1. **Pack**: build a well-structured markdown prompt from your repo (directory tree, selected files as labeled code blocks, your task, and output-format instructions) and copy it to the clipboard.
2. **Chat**: paste it into the web chat by hand.
3. **Apply**: copy the model's reply, and `dspack` parses its labeled code blocks into file changes, shows you a diff, and applies the ones you accept. Shell commands in the reply are listed for you to run yourself, never executed.

Everything stays manual on the browser side. No scraping, no reverse-engineered APIs, no terms-of-service risk.

## Principles

- **Markdown both ways.** The prompt shows files as a path label plus a fenced block, and the model answers in the same natural shape. Strict input, forgiving output parsing.
- **One repo, one config.** No global state. Each project has its own `.dspack.toml` at the repo root.
- **Model output is untrusted input.** Paths are validated, edits must match exactly once, and writes are atomic.
- **Plan, then execute.** Parsing produces a plan you can review; nothing touches disk until you confirm.
- **Terminal-first, TUI by default.** A `ratatui` interface for picking files and reviewing diffs, with a non-interactive mode for scripting and SSH.
- **Library first.** All logic lives in a UI-free library; the TUI and CLI are thin front ends.

## Workflow

```
dspack pack     # pick files, write the task, copy the prompt
                # ...paste into the web chat, copy the reply...
dspack apply    # read clipboard, review the diff, accept or reject blocks
dspack undo     # revert the last applied batch
```

On first run inside a repo with no config, `dspack` asks before creating `.dspack.toml` at the repo root. Outside a git repo it refuses to run.

## Status

Scaffolded. The core library is mostly stubs; the working pieces so far are the fence-length helper, the first path-safety check, and the token estimate. See [ROADMAP.md](ROADMAP.md) for milestones.

## Documentation

| File | Contents |
| --- | --- |
| [TECH_STACK.md](TECH_STACK.md) | Crates and the reasoning behind each choice |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Crate layout, data flow, safety model, TUI design |
| [FORMAT.md](FORMAT.md) | The prompt and output format the model must follow |
| [ROADMAP.md](ROADMAP.md) | Milestones and open questions |

## Development

```bash
./scaffold.sh dspack   # generate the workspace, add dependencies, git init
cd dspack
cargo test
cargo run -p dspack -- --help
```

## Platform

Developed on macOS. Linux should work; Windows is untested.
