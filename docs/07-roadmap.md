# Roadmap

The order in which Gitardo Desktop is built, in milestones. Each ends with something that runs and that a person can
try, and with its exit criteria met. A milestone does not start until the one before it is done; the cards it builds
are listed by area, and the index of the catalog marks each card as it is built.

## M0 — The workshop

The workspace, with nothing in it but what checks it.

- The Cargo workspace with every crate of [02-architecture.md](02-architecture.md), empty, and the lints of
  [03-code-rules.md](03-code-rules.md).
- `xtask` with `check`: no outside packages, allowed edges between crates, `unsafe` only in `platform`, file sizes,
  catalog against code and tests.
- `testkit`: throwaway repositories, golden files, the random source, PNG reading and writing.
- `json`: parse, write, and edit in place with comments.

Exit: `cargo xtask check` passes on a clean clone, and fails on purpose when an outside crate is added.

## M1 — Git, without a window

Everything Gitardo reads, and the engine that runs plans, tested without any UI.

- `git`: running git, repository discovery, every Read card (state, log with paging, details, status, stashes,
  worktrees, submodules, reflog, blame, diff, merge-tree), with golden tests on the awkward repositories.
- `model`: commits, references, lanes of the graph, file trees, names of references with namesakes, messages.
- `engine`: plans, steps, log, undo journal, activity, cancellation.
- `gitardo-cli`, a small binary that runs a feature from the command line with answers given as arguments, and prints
  its plan or runs it. It is how features are tried before there is a window.

Exit: `gitardo-cli state` on a repository of 50,000 commits prints its state in under a second; `branch.create`,
`commit.create`, `sync.fetch` and `undo.last` run from the CLI and pass their integration tests.

## M2 — Pixels

A window that draws, on Windows.

- `platform` for Windows: window, input, frame presentation, HiDPI, clipboard, timers, waking the loop.
- `render`: frame buffer, shapes with anti-aliasing, the TrueType reader and rasterizer, the glyph cache, icons, themes.
- `ui`: layout, text, buttons, boxes, lists that draw only the rows in sight, scroll, splitters, menus, tooltips,
  dialogs, pickers, input boxes, focus and keys, find in page.

Exit: a demo window with every widget, in every theme, at 100 % and 200 % scale; its snapshots match; a list of a
million rows scrolls at 60 frames a second on the reference machine.

## M3 — The graph

The first useful program: read-only history.

- Areas: `repo.`, `activity.`, `graph.`, `refs.`, `search.`, `filter.`, `details.`, `commits.`, `commit-page.`,
  `welcome.`, `settings.`, `log.`.

Exit: the graph of a 56,000-commit repository opens in under 1.5 s and pages in under 300 ms per 1,000 commits; every
card of these areas passes its tests.

## M4 — Everyday operations

- Areas: `changes.`, `commit.`, `identity.`, `branch.`, `tag.`, `remote.`, `sync.`, `stash.`, `undo.`, `preview.`, and
  the Diff page for one file (`diff.` without staging of changes).

Exit: a day of ordinary work (stage, commit, branch, pull, push, stash, undo) can be done in Gitardo Desktop alone.

## M5 — History and rewriting

- Areas: `merge.`, `cherry-pick.`, `revert.`, `reset.`, `rebase.`, `message.`, `squash.`, `split.`, `fixup.`,
  `drag.`, `selection.`, `compare.`, `patch.`, `reflog.`, `bisect.`, and the rest of `diff.`.

## M6 — Conflicts

- Areas: `conflicts.`, `review.` (without the AI), `merge-view.`, the File page and its editor (`file-view.`).

Exit: the fuzz tests of the conflict rules pass thousands of cases against git.

## M7 — Files and blame

- Areas: `file-history.`, `blame.`, the Files pane.

## M8 — Worktrees and submodules

- Areas: `worktree.`, `worktree-setup.`, `submodule.`, the Tasks panel, trusted folders.

## M9 — Over the network

- `http` through curl, credentials of the system.
- Areas: `github.`, `ai.`, and drafting a commit message with the AI.

## M10 — Linux, then M11 — macOS

The platform layer for X11, then for AppKit. Nothing above `platform` changes; the application tests run on each.

## Before a release

By hand, on each system, with a real account and a real model:

- [ ] Sign in to GitHub with a real token, load the pull requests, open one.
- [ ] Draft a commit message and resolve a conflict with the AI provider configured.
- [ ] Choose a folder, a file to open and a file to save with the system's dialogs.
- [ ] Open a worktree in a new window, in the external editor and in a terminal.
- [ ] Remove a worktree with a linked `node_modules` on Windows, and check that the original `node_modules` is intact.
- [ ] Look at every snapshot that changed since the last release.
- [ ] Read `cargo xtask perf` on the large repository and compare with the last release.
