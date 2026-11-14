# One engine for both apps

How Gitardo moves from two engines, the TypeScript one of the extension and the Rust one of Gitardo Desktop, to one:
the Rust one, with the extension as an interface of VS Code over it
([D37](../08-decisions.md#d37--one-engine-the-rust-one-for-both-apps)). These pages are written before any code of the
move; the stage in progress is the first one of [06-roadmap.md](06-roadmap.md) not marked done.

| File | Says |
| --- | --- |
| [01-engine.md](01-engine.md) | Why one engine, what it is as a program, how the extension starts it, talks to it and copes when it fails, and what stays in TypeScript. |
| [02-protocol.md](02-protocol.md) | Every request, answer and notification between the extension and `gitardo-engine`, with their JSON, and how the engine's `Asker` becomes VS Code's dialogs. |
| [03-map.md](03-map.md) | Every action and read of the extension, and the desktop feature or read that replaces it, with the gaps on both sides. |
| [04-differences.md](04-differences.md) | Where the two apps behave differently today, operation by operation, each a bug, a choice or a line of the CHANGELOG. |
| [05-stored-data.md](05-stored-data.md) | What each app keeps between runs, and how the names stay as they are. |
| [06-roadmap.md](06-roadmap.md) | The stages, each desktop first, with their acceptance lines, and the first step described for the session that starts it. |
| [07-rules.md](07-rules.md) | What changes in AGENTS.md, the code rules, the contracts, the glossary, the testing pages and the catalog. |

The decisions are D37 to D49 and the questions Q393 to Q406 in [08-decisions.md](../08-decisions.md). Paths in these
pages are from the root of the repository, or from `rust/` and `extension/` where they name crates (`crates/engine`)
or the extension's sources (`src/git/ops.ts`).
