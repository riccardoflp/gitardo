# Roadmap

Gitardo is released and works; this roadmap is not the plan of its features, which the maintainer decides one at a
time. It is the order in which the gaps the documents found are dealt with, and in which the documents themselves are
completed. Each milestone ends with something a person can check. The reasons are in [the risks](13-risks.md); the
open questions in [the decisions](08-decisions.md#open-questions).

## M0 — The documents (done)

The frame (product, architecture, code rules, contract, platform, testing, decisions, glossary, security, stored data,
quality, risks), the cards of the areas where a mistake costs (`changes`, `commit`, `branch`, `tag`, `remote`, `sync`,
`merge`, `rebase`, `cherry-pick`, `revert`, `reset`, `rewrite`, `bisect`, `stash`, `conflict`, `operation`,
`worktree`, `setup`, `submodule`, `undo`, `preview`, `log`, `activity`, `settings`), and `npm run check:docs`.

Exit: `npm run check:docs` says *no problems*; every card's status in the [index](features/README.md) is *Documented*
or *Differs*.

## M1 — The open questions

Answer Q1 to Q5 and the questions the cards raised (Q6 onwards). Each answer is a decision, a change to a card, or a
change to the README; where the README and the code disagree (RS17), one of them is fixed. Decide whether
`npm run check:docs` joins `npm test` (Q4, RS19).

Exit: no open question older than this milestone; RS17 and RS19 removed or turned into decisions.

## M2 — Nothing lost without asking

The risks where the user's work can be lost, each fixed with a test that failed before:

- RS2: **Undo** right after an operation it cannot take back says so instead of taking back the one before, and
  staging does not forget a deleted branch (`undo.last`).
- RS8: **Unstage all changes** during an operation under way unstages only, or is not offered (`changes.unstage-all`).
- RS9: **Stage All and Commit** refuses while a file is in conflict (`commit.create`).
- RS10, RS11: **Skip** and taking a side ask with a warning (`operation.skip`, `conflict.take-side`).

Exit: the **Differs** lines of those cards are gone, and their status is *Documented*.

## M3 — What runs is what is shown

- RS7: the files of an interactive rebase are written as `writes` of the operation, after the question, and logged; the
  question shows the todo list and the new messages; the reads take no lock.
- RS12: a series of cherry-picks, and a cherry-pick of a merge commit, have the conflict preview.
- RS13: a branch is given to git by its full name where the short one may mean a tag or a remote branch.
- RS14, RS15: a cleanup and a worktree removal that git refuses halfway leave nothing half done.
- RS16: the settings written by the worktree page and the Explorer, and the revert of a change in the Flow Diff, are
  logged; or a decision says they are not operations.

Exit: the **Differs** lines of those cards are gone.

## M4 — The checks

- RS6: a check that every test named in **Tested by** exists.
- RS3: a check of the imports between folders, as the architecture draws them (R1.4).
- RS18: thresholds for QA1 to QA4, on the large repository, in a script that fails.

Exit: each check runs in a command of [06-testing.md](06-testing.md) and fails on purpose once.

## Along the way — the other areas

The areas without cards (RS20) get theirs the first time one of their features changes, in that commit: repositories
and identities, the graph, search and filter, the tab of a commit and the side bar, comparisons, history and blame,
patches, GitHub, the Flow Diff and the Flow Merge. When an area is written, its row in
[04-feature-contract.md](04-feature-contract.md#the-areas) says *Written*.

## Before a release

- [ ] `npm run typecheck`, `npm test`, `npm run check:docs`.
- [ ] `npm run test:e2e` and `npm run test:vsix`.
- [ ] `npm run test:ui`, and the screenshots in `dist/ui` looked at.
- [ ] The checklist of what no test reaches, in [06-testing.md](06-testing.md#what-no-test-reaches).
- [ ] The CHANGELOG's **Unreleased** section becomes the version; `package.json` has the version.
- [ ] Every card changed since the last release has its status in the index.
- [ ] [10-security.md](10-security.md) read again if a service, a secret or a file written was added.
- [ ] The maintainer packs with `npm run package`, tags and publishes by hand.
