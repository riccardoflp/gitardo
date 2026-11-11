# Gitardo Desktop

A desktop application for Git: the commit graph, the history and the everyday operations of Git, in a window of its own.
It does everything the Gitardo extension for VS Code does, and is written from scratch in Rust, with the standard
library and nothing else.

This branch has no code yet. It has the documents to build it from, in a clean room: what every feature does, as a
function, and the rules the code follows.

## The documents

| Document | What it says |
| --- | --- |
| [00 — The clean room](docs/00-clean-room.md) | How the program is written from behavior, never from the extension's code, and who may read what. |
| [01 — The product](docs/01-product.md) | The window, its panes and pages, and what replaces each thing the extension took from VS Code. |
| [02 — Architecture](docs/02-architecture.md) | The crates, which may use which, how a click becomes a git command, threads, state, drawing. |
| [03 — Code rules](docs/03-code-rules.md) | The numbered rules every line follows, and what `cargo xtask check` enforces. |
| [04 — Every feature is a function](docs/04-feature-contract.md) | The four kinds of feature, their signatures, questions, plans, steps, and the template of a card. |
| [05 — The operating systems](docs/05-platform.md) | What is asked of Windows, Linux and macOS, and how; text, fonts, HTTP. |
| [06 — Testing](docs/06-testing.md) | Unit, golden, integration, view snapshots, application tests, performance. |
| [07 — Roadmap](docs/07-roadmap.md) | The milestones, from the empty workspace to three systems, and the checks before a release. |
| [08 — Decisions](docs/08-decisions.md) | Why git is run rather than reimplemented, why no dependencies, why drawing in software; the open questions. |
| [Features](docs/features/README.md) | The catalog: one card per feature, with its function, its plan and its acceptance list. |

Read 00, 01 and 04 first. Then the cards of the area you work on.

## Building

There is nothing to build yet. Once milestone M0 of the roadmap is done:

```bash
cargo build --release
cargo xtask check
```

The only programs needed besides the Rust toolchain are `git` (2.40 or later for the previews of conflicts) and, for
GitHub and the AI provider, `curl`.

## License

MIT, as Gitardo.
