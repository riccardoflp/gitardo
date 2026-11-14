# The rules that change

What one engine and *desktop first* change in the rules of the repository, page by page: what is already written on
`main-merge` with these documents, and what waits for a stage of the [roadmap](06-roadmap.md). A rule that a stage
needs is written in the commit of that stage, not before.

## Written with these documents

| Page | Change |
| --- | --- |
| [AGENTS.md](../../AGENTS.md) | Rewritten for both apps: desktop first as the first ground rule, a change seen by users goes into the card with each app's status and into the CHANGELOG of each app it reaches, the transition as a place to start, the commands of each part in short. |
| [rust/AGENTS.md](../../rust/AGENTS.md) | The clean room is replaced by D40: the extension's code may be read, never translated line by line. Desktop first, and a change to the engine the extension uses keeps the protocol working. Paths from `rust/`. |
| [extension/AGENTS.md](../../extension/AGENTS.md), `extension/CLAUDE.md` | New, for the folder: a moved area sends `feature/run`, its `GitOp` is only the fallback and gets no new behavior; a new feature starts in the engine; the Marketplace's README and CHANGELOG; the settings sent under the desktop's names; the commands. |
| [desktop/00-clean-room.md](../desktop/00-clean-room.md) | Marked as replaced by D40, kept as the record. |
| [desktop/03-code-rules.md](../desktop/03-code-rules.md) | **R1.7**: `serde` only in `protocol` and `server`, and how a change to a message raises the version. **R2.7**: a card's status in the extension is a known one and never ahead of the desktop's, checked by `cargo xtask check`. |
| [extension/03-code-rules.md](../extension/03-code-rules.md) | **R1.5**: `gitardo-engine` is the one program besides `git`, spoken to by `src/engine` only. **R8.6**: in a moved area nothing outside `src/git` runs git or builds a `GitOp`, and the area's `GitOp` gets no new behavior. |
| [desktop/04-feature-contract.md](../desktop/04-feature-contract.md) | **Its status in each app**: the two columns of the index, desktop first, one function for both apps. |
| [extension/04-feature-contract.md](../extension/04-feature-contract.md) | The page holds for the areas not moved; in a moved one the desktop's contract holds. |
| [09-glossary.md](../09-glossary.md) | **The two apps and the engine**: Gitardo Desktop, the extension, engine, host, host question, protocol, desktop first, status of a card, fallback. |
| [desktop/06-testing.md](../desktop/06-testing.md) | **Scenarios run by both apps**: one JSON per operation, run through `gitardo-cli` and through the extension, comparing the plans and the repositories after; the protocol's round-trip and golden tests; a change to the engine also runs the extension's e2e. |
| [extension/06-testing.md](../extension/06-testing.md) | **With the engine**: how the tests find the binary, `test/engine.test.ts`, the integration tests through `feature/run` with a fake host, the e2e and UI suites run with and without the binary. |
| [features/README.md](../features/README.md) | The column **Status** becomes **Desktop**, and a column **Extension** is added to every row, filled from [03-map.md](03-map.md) and [04-differences.md](04-differences.md); the extension's identifiers are listed at the end with the desktop card each is an alias of ([D44](../08-decisions.md#d44--one-catalog-the-desktops-with-a-status-for-each-app)). |
| `rust/xtask` | The index's sixth cell is read as the extension's status and checked by R2.7, with three tests (`check/catalog/tests.rs`). |
| [08-decisions.md](../08-decisions.md) | D37 to D49, and the open questions Q393 to Q406. |

## Waiting for a stage

| Rule | Stage | What |
| --- | --- | --- |
| The edges of the crates | 1 | The picture of [desktop/02-architecture.md](../desktop/02-architecture.md) gets `protocol`, `server` and the crate Q393 decides, before the code (R1.3). |
| The extension's architecture | 1 | [extension/02-architecture.md](../extension/02-architecture.md) gets `src/engine` and the path of a click through `feature/run`. |
| The extension's stored data | 2 | [extension/11-data.md](../extension/11-data.md) says that the settings are sent to the engine, and where the rebase files go when the rebase area moves. |
| The merge of the cards | 4 | For each area, the extension's cards are merged into the desktop's ones, and the area's file in `features/extension/` is emptied; the card template of the desktop gets an **Extension** line where the interface differs. |
| The check of the catalog | 4 | `cargo xtask docs`, the part of `cargo xtask check` that reads the documents, takes from `docs/catalog.py` the lines a card's kind needs and the words of the glossary, and `npm run check:docs` calls it; `catalog.py` is removed when the last area is merged ([D46](../08-decisions.md#d46--cargo-xtask-check-checks-the-catalog-of-both-apps)). |
| The promises of the cards | 4 | The extension's [promises](../extension/04-feature-contract.md#the-promises-every-card-is-checked-against) are compared with the desktop's contract and the ones it lacks are added there. |
| Security | 1, 5 | [extension/10-security.md](../extension/10-security.md) gets the engine as a process: what it trusts in its standard input, the binary's place in the package, the host questions that carry a token. |
| The vsix | 8 | R1.3 of the extension's code rules: `.vscodeignore` lets in `bin/<target>/`. |

## How the extension is tested with the engine, in short

Desktop first holds for tests too: a behavior is tested in the engine's own tests (table tests of the function, the
plan against a throwaway repository), and the extension tests its interface: that the question is asked as VS Code
asks it, that the follow-ups open what they say, and that the e2e and UI suites pass. What both apps must agree on is
checked once, by the shared scenarios of [desktop/06-testing.md](../desktop/06-testing.md#scenarios-run-by-both-apps),
which compare the plans and the final states of the repository.
