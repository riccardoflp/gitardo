# Roadmap

Gitardo is released and works; this roadmap is not the plan of its features, which the maintainer decides one at a
time. It is the order in which the gaps the documents found are dealt with, and in which the documents themselves are
completed. Each milestone ends with something a person can check. The reasons are in [the risks](13-risks.md); the
open questions in [the decisions](../08-decisions.md#open-questions).

## M0 — The documents (done)

The frame (product, architecture, code rules, contract, platform, testing, decisions, glossary, security, stored data,
quality, risks), the cards of the areas where a mistake costs (`changes`, `commit`, `branch`, `tag`, `remote`, `sync`,
`merge`, `rebase`, `cherry-pick`, `revert`, `reset`, `rewrite`, `bisect`, `stash`, `conflict`, `operation`,
`worktree`, `setup`, `submodule`, `undo`, `preview`, `log`, `activity`, `settings`), and `npm run check:docs`.

Exit: `npm run check:docs` says *no problems*; every card's status in the [index](../features/extension/README.md) is *Documented*
or *Differs*.

## M1 — The open questions (partly done by M49)

M49 removed RS17, the README read again against the code, and RS19: `npm test` runs `npm run check:docs` when Python 3
is there, as Q378 proposed. Left: the open questions themselves, Q375 to Q379 and those the cards raised, each built as
it says until the maintainer answers it.

Answer Q375 to Q379 and the questions the cards raised (Q380 onwards). Each answer is a decision, a change to a card, or a
change to the README; where the README and the code disagree (RS17), one of them is fixed. Decide whether
`npm run check:docs` joins `npm test` (Q378, RS19).

Exit: no open question older than this milestone; RS17 and RS19 removed or turned into decisions.

## M2 — Nothing lost without asking (done by M49)

Taken up by [M49](../desktop/07-roadmap.md#m49--the-checks-pass-on-every-system-and-the-risks-tried-again), which tried
each of these against the engine that runs them now: RS2 and RS8 to RS11 are removed, each with a test of its own.

The risks where the user's work can be lost, each fixed with a test that failed before:

- RS2: **Undo** right after an operation it cannot take back says so instead of taking back the one before, and
  staging does not forget a deleted branch (`undo.last`).
- RS8: **Unstage all changes** during an operation under way unstages only, or is not offered (`changes.unstage-all`).
- RS9: **Stage All and Commit** refuses while a file is in conflict (`commit.create`).
- RS10, RS11: **Skip** and taking a side ask with a warning (`operation.skip`, `conflict.take-side`).

Exit: the **Differs** lines of those cards are gone, and their status is *Documented*.

## M3 — What runs is what is shown (done by M49)

Taken up by [M49](../desktop/07-roadmap.md#m49--the-checks-pass-on-every-system-and-the-risks-tried-again), which tried
each of these against the engine that runs them now: RS7 and RS12 to RS14 are removed; RS15 is narrowed to the links a
removal git refuses takes out, which are not put back; RS16 says why the revert of the Flow Diff and **Save** in
**Gitardo Changes** are edits of a file in its editor, not operations, so that `log.record` and `diff.revert-change`
stay *Differs* in the extension.

- RS7: the files of an interactive rebase are written as `writes` of the operation, after the question, and logged; the
  question shows the todo list and the new messages; the reads take no lock.
- RS12: a series of cherry-picks, and a cherry-pick of a merge commit, have the conflict preview.
- RS13: a branch is given to git by its full name where the short one may mean a tag or a remote branch.
- RS14, RS15: a cleanup and a worktree removal that git refuses halfway leave nothing half done.
- RS16: the settings written by the worktree page and the Explorer, and the revert of a change in the Flow Diff, are
  logged; or a decision says they are not operations.

Exit: the **Differs** lines of those cards are gone.

## M4 — The checks (done by M49)

Taken up by [M49](../desktop/07-roadmap.md#m49--the-checks-pass-on-every-system-and-the-risks-tried-again), which
built them, each made to fail once on purpose.

- RS6: every test named in **Tested by** exists, checked by `npm run check:docs` and `cargo xtask check` (R2.6).
- RS3: the imports between folders follow the architecture, `test/imports.test.ts` (R1.4, R1.5).
- RS18: QA1 to QA3 fail above one and a half times their time in `test/perf/vscode.mjs --limits`, which the pipeline
  runs on Linux; QA4 gives no time (Q1590).
- RS19: `npm test` runs `npm run check:docs` when Python 3 is there, and says it skipped it when not (Q378).

Exit: each check runs in a command of [06-testing.md](06-testing.md) and fails on purpose once.

## Along the way — the other areas (done by M49)

The areas without cards of their own (RS20), repositories and identities, the graph, search and filter, the tab of a
commit and the side bar, comparisons, history and blame, patches, GitHub, the Flow Diff and the Flow Merge, have the
desktop's cards, each with a status for the extension (D44); M49 checked them against what the extension does today,
and their rows in [04-feature-contract.md](04-feature-contract.md#the-areas) say where. RS1, the files over 1,000 lines,
is down to the stylesheet of the views (Q1700), and `test/fileLength.test.ts` keeps every file of TypeScript under it.

## Before a release

- [ ] `npm run typecheck`, `npm test`, `npm run check:docs`.
- [ ] `npm run test:e2e` and `npm run test:vsix`.
- [ ] `npm run test:ui`, and the screenshots in `dist/ui` looked at.
- [ ] The checklist of what no test reaches, in [06-testing.md](06-testing.md#what-no-test-reaches).
- [ ] The **Unreleased** section of both CHANGELOGs becomes the version; `package.json` and `../rust/Cargo.toml` have
  it, the same for both apps (D52).
- [ ] Every card changed since the last release has its status in the index.
- [ ] [10-security.md](10-security.md) read again if a service, a secret or a file written was added.
- [ ] The maintainer commits on `main`, pushes it and the tag `vX.Y.Z`; the workflow **Release** checks the commit,
  waits for their approval in the environment `marketplace`, publishes the seven vsix and makes a draft release on
  GitHub with them and the desktop of each system, which the maintainer reads and publishes
  ([D52](../08-decisions.md#d52--a-tag-publishes-the-release-once-the-maintainer-approves)).
