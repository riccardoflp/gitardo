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

## Written in stage 1

| Page | Change |
| --- | --- |
| [desktop/02-architecture.md](../desktop/02-architecture.md) | The picture of the crates gets `system` (Q393), `settings` (Q395), `protocol` and `server`; `engine` no longer uses `platform`. `rust/xtask` checks the new edges (R1.3), lets `system` use `unsafe` (R5.1) and `server` read the clock (R3.2). |
| [desktop/03-code-rules.md](../desktop/03-code-rules.md) | R5.1, R5.2 and R5.4 name `crates/system` beside `crates/platform`; R3.2 names `server`. |
| [extension/02-architecture.md](../extension/02-architecture.md) | `src/engine` among the modules and the engine among the containers; the path of a click through `feature/run` waits for stage 2. |
| [extension/10-security.md](../extension/10-security.md) | The engine as a process: T14 (another binary run as the engine; `gitardo.engine.path` read only from the user's settings) and what it writes, among the input that is not trusted. The host questions that carry a token wait for stage 5. |
| [extension/06-testing.md](../extension/06-testing.md) | Where the tests find the binary (`GITARDO_ENGINE`, else `../rust/target/debug`), and what `test/engine.test.ts` checks. |

## Written in stage 2

| Page | Change |
| --- | --- |
| [transition/02-protocol.md](02-protocol.md) | Version 1.1: `feature/run` with `inputs`, the answers given taken by key, one run at a time on the thread of each repository's engine, every `Validation` an object, each `Step` with its `line`, the follow-ups stage 2 sends, `followUps` in the data of `stepFailed`. |
| [desktop/02-architecture.md](../desktop/02-architecture.md) | `server` uses `features` and `settings`, for `feature/run` (R1.3). |
| [extension/02-architecture.md](../extension/02-architecture.md) | The path of a click through `feature/run`, `feature/ask` and the follow-ups. |
| [extension/11-data.md](../extension/11-data.md) | The settings sent to the engine; `worktreeInputs` written by the follow-up `rememberInputs` and sent as `inputs`. |
| [extension/06-testing.md](../extension/06-testing.md) | What `test/engine.test.ts` checks of `branch.create`, and the scenario run through `gitardo-cli` and the extension. |
| [features/06-branches-tags-remotes.md](../features/06-branches-tags-remotes.md) | The first card merged: `branch.create`, with its **Extension** line, Desktop Built and Extension Built; the extension's card is gone from `features/extension/`. |

## Written in stage 3

| Page | Change |
| --- | --- |
| [transition/02-protocol.md](02-protocol.md) | Version 1.2: `journal/last` with `undoable`, `journal/record` with `before` and `after` and every `UndoRecord` with its fields. |
| [features/16-undo-preview-log.md](../features/16-undo-preview-log.md) | The journal a stack of Gitardo's operations; `undo.last` refuses what it cannot take back and while an operation is under way; the extension's card of `undo.last` merged, with its **Extension** line. |
| [features/06-branches-tags-remotes.md](../features/06-branches-tags-remotes.md) | Undo of a branch created only deletes it. |
| [transition/04-differences.md](04-differences.md) | D7 and D8 fixed; D9 and D10 in the extension's CHANGELOG. |
| [extension/02-architecture.md](../extension/02-architecture.md) | Every `GitOp` recorded with `journal/record`, and **Undo** through `feature/run`. |

## Written in stage 4, area 1 (changes and commit, stashes)

| Page | Change |
| --- | --- |
| [features/05-changes-and-commit.md](../features/05-changes-and-commit.md), [08-stashes.md](../features/08-stashes.md), [02-graph.md](../features/02-graph.md), [17-diff-and-merge-views.md](../features/17-diff-and-merge-views.md) | The extension's cards of the area merged, each with its **Extension** line and acceptance lines; `features/extension/05` and `08` emptied; stage, unstage and discard take the files ticked. |
| [transition/04-differences.md](04-differences.md) | A1, A2, A7, A8, A9 and A12 fixed or in the CHANGELOG; B10 found by the e2e suite with the engine. |
| [extension/02-architecture.md](../extension/02-architecture.md) | A step of the engine logged as the command of a `GitOp`. |
| [extension/06-testing.md](../extension/06-testing.md) | The harness and the scenario test of an area; the e2e and UI suites with the engine and without. |

## Written in stage 4, area 2 (branches, tags, remotes, identity, sync)

| Page | Change |
| --- | --- |
| [transition/02-protocol.md](02-protocol.md) | Version 1.3: the follow-up `offerRun` with its feature, target and answers, the answer `flag`, the flags given to a feature from its first call, and `followUps` written in camel case in the data of `stepFailed`. |
| [features/06-branches-tags-remotes.md](../features/06-branches-tags-remotes.md), [02-graph.md](../features/02-graph.md), [05-changes-and-commit.md](../features/05-changes-and-commit.md) | The extension's cards of the area merged, each with its **Extension** line and acceptance lines, `identity.switch` and `sync.sync` included; `features/extension/06` removed. `remote.delete-branch` names the branch in full when a tag has its name. |
| [features/16-undo-preview-log.md](../features/16-undo-preview-log.md) | The Undo of a deleted branch asks with a confirmation, not a warning. |
| [transition/04-differences.md](04-differences.md) | B2 and B7 fixed; B3 and B8 in the CHANGELOG; B5, B6 and B9 decided as recommended, to be confirmed; the wordings the e2e steps found. |
| [extension/06-testing.md](../extension/06-testing.md) | The scenario test of the area, with a bare repository beside each copy as its remote. |
| [extension/02-architecture.md](../extension/02-architecture.md) | `HostHooks`: choices of the host in a pick of the engine, and what the host does once a plan is confirmed; `offerRun` as a button on the error. |

## Waiting for a stage

| Rule | Stage | What |
| --- | --- | --- |
| The extension's stored data | 4 | [extension/11-data.md](../extension/11-data.md) says where the rebase files go when the rebase area moves. |
| The merge of the cards | 4 | For each area, the extension's cards are merged into the desktop's ones, and the area's file in `features/extension/` is emptied; the card template of the desktop gets an **Extension** line where the interface differs. |
| The check of the catalog | 4 | `cargo xtask docs`, the part of `cargo xtask check` that reads the documents, takes from `docs/catalog.py` the lines a card's kind needs and the words of the glossary, and `npm run check:docs` calls it; `catalog.py` is removed when the last area is merged ([D46](../08-decisions.md#d46--cargo-xtask-check-checks-the-catalog-of-both-apps)). |
| The promises of the cards | 4 | The extension's [promises](../extension/04-feature-contract.md#the-promises-every-card-is-checked-against) are compared with the desktop's contract and the ones it lacks are added there. |
| Security | 5 | [extension/10-security.md](../extension/10-security.md) gets the host questions that carry a token. |
| The vsix | 8 | R1.3 of the extension's code rules: `.vscodeignore` lets in `bin/<target>/`. |

## How the extension is tested with the engine, in short

Desktop first holds for tests too: a behavior is tested in the engine's own tests (table tests of the function, the
plan against a throwaway repository), and the extension tests its interface: that the question is asked as VS Code
asks it, that the follow-ups open what they say, and that the e2e and UI suites pass. What both apps must agree on is
checked once, by the shared scenarios of [desktop/06-testing.md](../desktop/06-testing.md#scenarios-run-by-both-apps),
which compare the plans and the final states of the repository.
