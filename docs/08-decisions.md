# Decisions

The choices Gitardo is built on, each with why and what was rejected. A decision is never edited away: when it changes,
a new one replaces it, and the old one says *Replaced by*. Most of these were made while the code was written and are
reconstructed here from the history and from the maintainer's words; they say so. Where the reason is not known, it is
an open question at the end, not a guess.

*Source: Michael Nygard, "Documenting Architecture Decisions" (2011).*

## D1 — A VS Code extension, with its views in the panel and the Source Control side bar

Gitardo lives inside VS Code: the graph in the bottom panel next to the terminal, its other views in the Source Control
side bar under VS Code's own, its pages as editor tabs. It has no icon of its own in the activity bar: git already has
its place there, and a second one would split it. The views were first in a container of their own; they moved to the
Source Control side bar in `3d5998b`. A standalone application is a separate project, not a mode of this one.

*Reconstructed from `d790289` and `3d5998b`, and from the maintainer's choice of the three zones of the graph.*

## D2 — Run the git executable, never a library

Gitardo runs `git` from the `PATH` for everything it reads and does, with the arguments as a list and no shell. What it
shows is then what git would do for the user in a terminal, with the user's configuration, hooks, credentials and
version, and the user can run the same commands. A library (isomorphic-git, libgit2 bindings) was not used: it would
behave differently from the user's git in the corners (hooks, filters, configuration) and add a native or large
dependency. The cost: a process per command, and parsing what git prints.

*Reconstructed from `d790289` and the README's Command preview.*

## D3 — React for the views, Monaco only where text is edited

Every view and tab is one React bundle, picked by the `data-view` of the page. Monaco, which is heavy, is loaded only by
the Flow Diff and the Flow Merge, where text is compared and edited side by side; the colors of code elsewhere come
from shiki, loaded on demand. *Reconstructed from the maintainer's choice of React, `137bfd1` and `9e97d6e`.*

## D4 — An operation is a GitOp built before it runs

Everything that changes the repository is first data: a title, the exact commands, whether it can lose work, what
Gitardo writes itself, and what to offer if git refuses. One function, `Controller.run`, asks, runs, logs and reads
again. That gives the command preview, the log and Undo to every operation for nothing, and lets the tests run an
operation's commands on a real repository without VS Code. The questions an operation asks are still asked by its case
of `resolveAction`, with VS Code's dialogs, rather than returned as data: moving them would make every operation
testable without VS Code, but costs a rewrite of `actions.ts` for features that work; it is not planned. See
[the feature contract](04-feature-contract.md#an-operation-as-it-is-built-today).

*Reconstructed from `src/git/ops.ts` and AGENTS.md, "From a click to git".*

## D5 — Show the exact commands, and always ask before losing work

`gitardo.commandPreview` set to `all` shows the commands of every operation before they run; `off`, the default, asks
only before what can lose work or rewrite history, which is asked whatever the setting. The commands shown are the ones
that run, with no flag added after. The alternative, asking before everything always, was rejected as noise; asking
before nothing, as unsafe.

*Reconstructed from the maintainer's choice at the start of the project (then `gitar.commandPreview`).*

## D6 — Undo reads the reflog, and remembers only what it cannot tell

**Undo** takes back the last operation by reading the reflog of `HEAD`, so it also undoes what was done in a terminal,
and undoing twice redoes. What leaves no trace there (a deleted branch, a dropped stash, conflicts resolved, a squash
merge, a series of commits made in one operation) Gitardo remembers itself, in memory, for as long as the top of the
reflog stays what it was. It is not kept across a reload of the window: a stale undo that moves a branch is worse than
none. *Reconstructed from `Controller.action` and the README's Undo.*

## D7 — Resolving conflicts automatically, and with AI, is experimental and off

The rules for the conflicts that need no choice, and the language model, only **propose**; nothing is written until the
user accepts, in the review or in the Flow Merge, and Undo marks the conflicts again. Both are behind settings that are
off by default (`gitardo.conflicts.autoResolve`, `gitardo.conflicts.ai`) and marked experimental in the Settings
editor: a wrong resolution can be committed without being noticed. *Reconstructed from `11c06c4`.*

## D8 — Everything in English, and the former names nowhere

Code, comments, UI text, the README, the CHANGELOG, these documents and commit messages are in English. The extension
had other names before 0.2.0; they are written nowhere but the 0.2.0 entry of the CHANGELOG, which explains the change.
The identifier changed with the name (`gitardo.*`), knowing that it made a new listing on the Marketplace.
*Reconstructed from `eda8186` and the maintainer's instructions.*

## D9 — Settings are `gitardo.*`, and VS Code's Git settings are read when Gitardo's are not set

Every setting is a VS Code setting named `gitardo.<area>.<name>`. Where VS Code's Git extension has a setting for the
same thing, Gitardo reads it while its own is not set (`git.branchSortOrder`, `git.worktreeSymlinkFolders`,
`git.worktreeIncludeFiles`), so that a user of both configures it once. *Reconstructed from the settings in
`package.json`.*

## D10 — The README is for users; `docs/` is for whoever changes the code

The README is the extension's page on the Marketplace and its **Details** tab in VS Code, so it says only what a user
needs. What a developer needs (the scripts, the tests, the layout of the source, the architecture, the rules) is in
`docs/`, and `AGENTS.md` is the way in. The CHANGELOG and the walkthrough stay as they are: they are part of the
product. Before this, the README had a **Development** section and `AGENTS.md` held the architecture and the rules;
both moved into `docs/` when it was written, so that nothing is said twice.

## D11 — Cards first for the areas where a mistake costs

The catalog has cards for the areas whose operations can lose work, rewrite history or leave the repository in a state
that is hard to read: changes and commits, branches and sync, merging and rewriting, stashes, conflicts, worktrees,
submodules, and Undo with the preview and the log. The other areas (the graph, search, the tab of a commit,
comparisons, history and blame, patches, GitHub, the Flow Diff and the Flow Merge, repositories) are described for
users in the README and get their cards the first time one of their features is changed, in that commit.

Why: the README already describes every feature at the level of a card, so cards add most where they add the function,
the tests, the danger and the undo of an operation; writing all of them at once would cost much and age before it was
read. What it costs: a change to a feature of an area without cards first writes the cards of that area, or at least of
the features it touches.

To change this choice, replace this decision: to write the whole catalog at once, list the areas in
[04-feature-contract.md](04-feature-contract.md#the-areas) as *Written* as their files come, and run
`npm run check:docs`; to write fewer, mark areas *Later* and delete their files.

## D12 — No linter, no formatter

The project has `tsc --noEmit` in strict mode and no ESLint or Prettier: the code is laid out as Prettier would lay it
out with a width of 160, by hand, matching the file. A linter would enforce rules that review catches today (R5, R6)
but adds configuration and dependencies to keep. *Reconstructed from AGENTS.md; the reason for leaving Prettier out is
Q3.*

## D13 — Tests are named by what the user sees

Tests are named with a sentence of what the user sees (`it('discards a file taken out of the index and left on disk back
to HEAD, rather than deleting it')`), not with the identifier of a card. The card's **Tested by** line names the test by
its file and title, so that the link goes from the documents to the tests. The cost: a test renamed or removed leaves a
**Tested by** line that no tool finds (see [RS6](13-risks.md)). The alternative, starting each test's name with its
card's identifier, would rename hundreds of tests for a link the card already gives.

## D14 — The maintainer publishes by hand

No agent pushes, tags or publishes. A release is a commit with the version and the CHANGELOG, a tag, and a package made
with `npm run package` that the maintainer uploads to the Marketplace. *Reconstructed from AGENTS.md.*

## Open questions

- **Q1** — The glossary's **branches shown** is `Scope` in the code and "filter by branch" in the README, while `Filter`
  in the code is the search. Which word wins, in the code and on screen? Nothing is renamed until this is answered.
- **Q2** — **Operation** means both what Gitardo runs (`GitOp`, **Undo Last Operation**) and what git has under way
  (`Operation`, the banner). Keep both, with *operation under way* for the second, or rename one?
- **Q3** — Why is there no Prettier in the project, when the code is laid out as Prettier lays it out? (D12)
- **Q4** — `npm run check:docs` needs Python, which the project does not otherwise need, so it is not part of
  `npm test`. Keep it apart, port the check to a Vitest test, or add it to `npm test` and require Python?
- **Q5** — The private channel for reporting a vulnerability in [SECURITY.md](../SECURITY.md): an email address, or
  GitHub's private vulnerability reporting on the repository?

The questions the cards raised, each naming the cards it would change:

- **Q6** — Staging, unstaging and discarding one hunk are built (`changes.stage-hunk`) but no view offers them. Offer
  them, or remove the code?
- **Q7** — Amending a commit already pushed asks nothing and says nothing, while rewording, squashing and splitting one
  warn. Should the amend warn too? `commit.amend`, `rewrite.pushed-warning`.
- **Q8** — Is reverting one change in the Flow Diff a discard, to be asked first as the README's list asks before
  discarding? And should discarding there take the file back to `HEAD`, as the Changes view does, rather than to the
  index? `changes.discard`, RS16.
- **Q9** — Deleting a branch whose upstream is another local branch: should it offer to delete the upstream too, as
  it does for a remote branch? `branch.delete`.
- **Q10** — A rebase onto a branch, and a pull with `--rebase`, rewrite the commits not yet pushed and ask nothing. Is
  that the intended line between `confirm` and nothing? `rebase.onto`, `sync.pull`.
- **Q11** — Undo after a fixup leaves the `fixup!` commit when the autosquash stopped, and a fixup of a pushed commit
  does not warn. `rewrite.fixup`, `undo.last`.
- **Q12** — The rebase editor runs without `--autostash` and without the warning for pushed commits that the other
  rewrites have. On purpose? `rebase.editor`, `rewrite.pushed-warning`.
- **Q13** — The menus offer merge, rebase, cherry-pick and revert while an operation is under way, and git refuses.
  Hide them instead? `merge.into-current`, `operation.banner`.
- **Q14** — What should **Undo** do right after **Abort**? Today it takes back whatever the reflog shows last.
  `operation.abort`, `undo.last`.
- **Q15** — The review reads the conflicts of at most 200 files; what does the user see of the others?
  `conflict.files-read`, `conflict.review-files`.
- **Q16** — **Sync** of submodules acts on all of them, while the other submodule entries act on one. `submodule.sync`.
- **Q17** — Stashes are named to git by position (`stash@{n}`); one pushed in a terminal between reading and running
  moves the others. Name them by their commit instead? `stash.apply`, `stash.pop`, `stash.drop`.
- **Q18** — The message drafted by the language model is not shown as running, and a message typed in the box is kept
  only once the Changes view has been opened. `commit.draft-message`, `commit.box`, `activity.track`.
