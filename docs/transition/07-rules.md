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

## Written in stage 4, area 3 (merge, rebase, cherry-pick, revert, reset, rewriting, bisect, patches)

| Page | Change |
| --- | --- |
| [features/07-history-rewriting.md](../features/07-history-rewriting.md), [11-history-and-blame.md](../features/11-history-and-blame.md), [14-patches.md](../features/14-patches.md) | The extension's cards of the area merged, each with its **Extension** line and acceptance lines; `features/extension/07-merge-and-rewrite.md` emptied; `message.edit` deletes its rebase files once the rebase ends; `rebase.branch-onto` lists a branch held by another worktree; `bisect.answer` asks when no button answered. |
| [extension/11-data.md](../extension/11-data.md) | Where the engine leaves the files of a rebase: `<git folder>/gitardo-rebase`, deleted once no rebase is under way; `gitardo-split` in `rebase-merge` as before; the patch of the clipboard written by the engine from the answer `clipboard`. |
| [transition/02-protocol.md](02-protocol.md) | The answer `clipboard` given with `feature/run`, which answers the engine's read of the clipboard. |
| [transition/04-differences.md](04-differences.md) | C1 and C7 checked and found not seen; C4 and C6 fixed; C2, C3, C5, C8, C9 and A6 in the CHANGELOG; the pairs of the area run both ways. |
| [extension/06-testing.md](../extension/06-testing.md) | `test/engineHistory.test.ts`. |

## Written in stage 4, area 4 (conflicts)

| Page | Change |
| --- | --- |
| [features/09-conflicts.md](../features/09-conflicts.md), [17-diff-and-merge-views.md](../features/17-diff-and-merge-views.md), [16-undo-preview-log.md](../features/16-undo-preview-log.md) | The extension's cards of the conflicts, the review and the questions to the AI merged, each with its **Extension** line and acceptance lines; `features/extension/09` emptied; Skip asks first, taking a side warns when typed lines are lost, and the `preview.confirm` paragraph agrees with the cards. |
| [desktop/04-feature-contract.md](../desktop/04-feature-contract.md) | **Danger and confirmation**: `Danger::Agreed`, a plan agreed to in the feature's own question, never shown again. |
| [transition/02-protocol.md](02-protocol.md) | Version 1.3: the targets `resolutions` and `mergeResult`, and `Danger::Agreed` sent as `none`. |
| [transition/04-differences.md](04-differences.md) | D2, D3 and D4 fixed, D6 decided, D1 and D5 in the CHANGELOG; the pairs of the area not reached before, checked. |
| [extension/02-architecture.md](../extension/02-architecture.md) | `src/engine/resolutions.ts`, and the Flow Diff and Flow Merge handed the runs of the engine with their `GitOp`. |

## Written in stage 4, area 5 (worktrees and their setup, submodules)

| Page | Change |
| --- | --- |
| [features/12-worktrees.md](../features/12-worktrees.md), [13-submodules.md](../features/13-submodules.md) | The extension's cards of the area merged, each with its **Extension** line and acceptance lines; `features/extension/12` and `13` emptied, and the identifiers of the area in `features/extension/18-settings.md` and `01-repositories.md` changed to the desktop's. |
| [transition/02-protocol.md](02-protocol.md) | Version 1.3: `offerRun` for **Remove anyway**, given `force` as a `flag`, and for **Choose What It Gets…**; `host.terminal`, the setup commands of a new worktree handed over in `waiting`. |
| [transition/04-differences.md](04-differences.md) | W1–W3, W9, W10, S3, S4 and S6 fixed; W4 and W5 decided as recommended, to be confirmed; W6–W8, S1 and S5 in the CHANGELOG; W11 left to D7; W12 waiting for `WriteSetting` (Q399). |
## Written in stage 4, area 6 (AI and GitHub)

| Page | Change |
| --- | --- |
| [transition/02-protocol.md](02-protocol.md) | Version 1.6: `read/commit.draft-message` and `read/ai.ask-conflict`; the host questions `host/languageModel` and `host/languageModels`, said at `initialize` as `languageModel`, and `host/githubToken` with `ask` for **Sign in to GitHub**; `$/cancel` sent by the engine for a host question it no longer waits for; the follow-up `writeSetting` and the target `comparison`. |
| [features/05-changes-and-commit.md](../features/05-changes-and-commit.md), [09-conflicts.md](../features/09-conflicts.md), [10-compare.md](../features/10-compare.md), [15-github.md](../features/15-github.md) | The card `commit.read-draft-diff`, desktop first (Q398); the **Extension** lines of the cards of the AI and of GitHub, run or asked through the engine, with the TypeScript as fallback; no card of the area is left to the extension alone. |
| [08-decisions.md](../08-decisions.md) | Q398 answered; Q399 built for `ai.choose-model` (`WriteSetting`). |
| [transition/04-differences.md](04-differences.md) | G1 the engine's; G2 and G3 decided, to be confirmed; G4 and G5 in the CHANGELOG; G6 open. |
| [extension/06-testing.md](../extension/06-testing.md) | `test/engineAi.test.ts`, with a fake language model. |
| [extension/10-security.md](../extension/10-security.md) | T2, T3 and the secrets: what reaches the model through the engine, and the token asked with `ask` only to sign in. |

## Written in stage 5 (the engine's reads)

| Page | Change |
| --- | --- |
| [transition/02-protocol.md](02-protocol.md) | Version 1.4: every read but the graph's with its method, parameters and answer, the shapes of the reads, the paragraph on parameters that changed from 1.2, and how the server answers them: on the thread of the request, cancelled by `$/cancel`, logged as a read with how long it took; `host/githubToken` asked by `read/github.read-pull-requests`. |
| [desktop/02-architecture.md](../desktop/02-architecture.md), `rust/xtask` | `protocol` uses `text`; `server` uses `text` and `http` (R1.3). |
| [features/06-branches-tags-remotes.md](../features/06-branches-tags-remotes.md), [01-repositories.md](../features/01-repositories.md), [05-changes-and-commit.md](../features/05-changes-and-commit.md) | The cards `branch.read-log`, `repo.common-folder` and `commit.prepared-message`, desktop first, which answer Q397 and Q398 ([08-decisions.md](../08-decisions.md)). |
| [features/03-search-and-filter.md](../features/03-search-and-filter.md), `rust/CHANGELOG.md` | `search.read-revision` finds no commit for a place past the end of a reflog instead of failing (R1). |
| [transition/04-differences.md](04-differences.md) | **Reads (stage 5)**: R1 fixed, R2 to R5 the same answer in another shape, and the three reads with no read of the extension to compare with. |
| [extension/06-testing.md](../extension/06-testing.md) | `test/engineReads.test.ts`, each read of the engine against the extension's. |
| [extension/10-security.md](../extension/10-security.md) | T2 and the secrets: the GitHub token asked by the engine with `host/githubToken`, done from **Waiting for a stage**. |
| [extension/02-architecture.md](../extension/02-architecture.md) | **Where the reads come from**: the views read through `src/engine/reads.ts`, the engine's read turned into the view's shape by `shapes.ts`, `Repo` when the engine is not there, fails, or cannot answer what the view asks; the state's parts from `repo.read-state`, the commits of the graph still from `Repo.withCommits`; `src/flow` uses `src/engine/reads.ts`, and `src/engine` uses `src/shared`. |
| [extension/06-testing.md](../extension/06-testing.md) | `test/reads.test.ts`, the reads as the views ask them, with the engine and without. |
| [transition/04-differences.md](04-differences.md), [08-decisions.md](../08-decisions.md), `extension/CHANGELOG.md` | R6 in the CHANGELOG; R7 a choice, Q407, read in TypeScript until it is decided. |

## Written in stage 6 (the webviews on the engine's shapes)

| Page | Change |
| --- | --- |
| [extension/02-architecture.md](../extension/02-architecture.md) | **The views' shapes**: the state and the details of a commit are the engine's types in `src/shared/state.ts`, read by the extension, the webviews and the preview server, and produced by `Repo` too; what the views show that the engine does not write is worked out from its fields, with the helpers beside the types; what the extension adds is in `RepoState`; `shapes.ts` converts only the other reads. |
| [extension/06-testing.md](../extension/06-testing.md) | **The engine's shapes**: the golden files of `rust/crates/protocol/tests/shapes.rs`, every optional part filled, and `test/engineShapes.ts`, which `npm run typecheck` checks both ways against them; what it cannot see, and which tests compare those values. |
| [transition/06-roadmap.md](06-roadmap.md) | Stage 6: the check is against the engine's JSON, not a schema. |
| [transition/04-differences.md](04-differences.md) | **The views on the engine's shapes (stage 6)**: V1, the order of the uncommitted changes, the view's kept. |
| `extension/tsconfig.json` | `resolveJsonModule`, so that `test/engineShapes.ts` reads the engine's golden files as types. |

## Written in stage 7 (the graph reads)

| Page | Change |
| --- | --- |
| [transition/02-protocol.md](02-protocol.md) | Version 1.5: `read/graph.read-commits`, `read/graph.read-position` and `read/search.read-matches`, with `BranchesShown`, `Narrowing` (a search shown as a list among them), `SearchMode` and `Row` written out, the matches with `hasMore`, and how the extension asks the next pages with `skip`. |
| [desktop/06-testing.md](../desktop/06-testing.md), `rust/xtask` | `cargo xtask perf <folder>`, the reads of the graph timed in the release engine, and the numbers of stage 7's own run. |
| [transition/04-differences.md](04-differences.md) | **Reads of the graph (stage 7)**: R8, two stashes of the same second, in the CHANGELOG. |
| [extension/02-architecture.md](../extension/02-architecture.md) | `src/engine/graph.ts` and `CommitsReader`: the graph read by the engine, `git log` as the fallback. |
| [extension/06-testing.md](../extension/06-testing.md) | `test/engineGraph.test.ts`, the rows, pages, lines, positions and matches of the engine against `Repo`'s. |

## Written in stages 8 and 9

| Page | Change |
| --- | --- |
| [extension/03-code-rules.md](../extension/03-code-rules.md) | R1.3: `.vscodeignore` lets in `bin/<target>/`. |
| [extension/06-testing.md](../extension/06-testing.md) | `npm run package` makes the universal vsix and one per target whose engine it finds; `npm run test:vsix` tests the vsix of this computer with its engine. |
| `.github/workflows/check.yml` | The pipeline: `cargo xtask check` and the desktop on Windows, Linux and macOS (both targets of the Mac), the engine of the seven targets, the extension's checks and its e2e suite with the engine and without on the three systems, and the vsix of every target, all as artifacts; it publishes nothing. |

## Waiting for a stage

| Rule | Stage | What |
| --- | --- | --- |
| The merge of the cards | 4 | For each area, the extension's cards are merged into the desktop's ones, and the area's file in `features/extension/` is emptied; the card template of the desktop gets an **Extension** line where the interface differs. |
| The check of the catalog | 4 | `cargo xtask docs`, the part of `cargo xtask check` that reads the documents, takes from `docs/catalog.py` the lines a card's kind needs and the words of the glossary, and `npm run check:docs` calls it; `catalog.py` is removed when the last area is merged ([D46](../08-decisions.md#d46--cargo-xtask-check-checks-the-catalog-of-both-apps)). |
| The promises of the cards | 4 | The extension's [promises](../extension/04-feature-contract.md#the-promises-every-card-is-checked-against) are compared with the desktop's contract and the ones it lacks are added there. |

## How the extension is tested with the engine, in short

Desktop first holds for tests too: a behavior is tested in the engine's own tests (table tests of the function, the
plan against a throwaway repository), and the extension tests its interface: that the question is asked as VS Code
asks it, that the follow-ups open what they say, and that the e2e and UI suites pass. What both apps must agree on is
checked once, by the shared scenarios of [desktop/06-testing.md](../desktop/06-testing.md#scenarios-run-by-both-apps),
which compare the plans and the final states of the repository.
