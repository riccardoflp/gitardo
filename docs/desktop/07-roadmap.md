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

A window that draws, on Windows, in the look of the Timeline
([D17](../08-decisions.md#d17--the-timeline-a-window-of-gitardos-own)).

- `platform` on the libraries of [D16](../08-decisions.md#d16--libraries-for-the-window-the-text-and-the-pixels): window,
  input, input methods, frame presentation, HiDPI, clipboard, file dialogs, timers, waking the loop, the screen reader.
- `render`: frame buffer, shapes with `tiny-skia`, text with `cosmic-text` in the fonts of the program, icons, themes.
- `ui`: layout, text, buttons, boxes, lists that draw only the rows in sight, scroll, splitters, menus, tooltips,
  dialogs, pickers, input boxes, focus and keys, find in page, the tree of what is on screen.

Exit: a demo window in the shape of the Timeline (header, branch rail, timeline with a draft, inspector) and every
widget, in every theme, at 100 % and 200 % scale; its snapshots match; a list of a million rows scrolls at 60 frames a
second on the reference machine; a screen reader reads the buttons and the rows.

A first version, on Gitardo's own window of Windows, TrueType rasterizer and shapes, in a look like VS Code's, was built
and replaced by this one.

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

- Areas: `file-history.`, `blame.`, the Files page.

## M8 — Worktrees and submodules

- Areas: `worktree.`, `worktree-setup.`, `submodule.`, the Tasks panel, trusted folders.

## M9 — Over the network

- `tls` with the one library of the exception, `http` written here, proxies, credentials of the system.
- Areas: `github.`, `ai.`, and drafting a commit message with the AI.

## M10 — Linux (done)

The platform layer for X11 and Wayland. Nothing above `platform` changes; the application tests run on each:
`cargo xtask check` passes on Windows and on Linux, with the toolchain of `rust-toolchain.toml`.

## M11 — macOS

The platform layer for AppKit. Nothing above `platform` changes; the application tests run on each.

Written, and checked by clippy for `aarch64-apple-darwin` from Linux: `platform` and every crate but `tls`, `http`,
`engine` and `app`, whose `ring` needs the C compiler of Apple to be built. Nothing of it has run yet. The milestone is
done once `cargo xtask check` passes on a Mac ([Q369](../08-decisions.md#open-questions)).

## M12 — Repository tabs (done)

Several repositories open in one window, each in a tab, each keeping what it showed
([D50](../08-decisions.md#d50--repository-tabs-and-what-each-repository-keeps)). Built in six phases, each one commit
or more, each ending with its cards built and their tests passing; the extension comes last.

1. **The state of each repository.** The store of `app` is split in two: what is the window's (the workspace, the
   settings, the dialogs, the Activity drawer, the Command log, the operation running) and what is a repository's (the
   timeline, the search, the inspector, the pages, the Changes page, the branch rail, the reads). The island of tabs
   replaces the repository menu of the header. Cards: `repo.tabs` (wide only), `repo.switch-tab`, `repo.keep-open`.
2. **Opening and closing tabs.** **+**, the picker of the repositories a workspace opens, the tabs kept with the
   recent workspaces, the workspace files of VS Code. Cards: `repo.open-tab`, `repo.choose-tabs`, `repo.close-tab`,
   `repo.open-code-workspace`.
3. **Worktrees and submodules in their tab.** Cards: `repo.group-tabs`.
4. **The tabs not shown**, watched and read when shown or in the background, and the operations while tabs are
   switched. Cards: `repo.watch-hidden`, and the lines of `repo.switch-tab` about operations.
5. **A narrow window.** The tabs narrowed, **… N**, the island as one button, the keys. Cards: the rest of `repo.tabs`.
6. **The extension.** `gitardo.repositories.keepOpen`, what each webview keeps, the list of repositories grouped.
   Cards: `repo.keep-open`, `repo.group-tabs` and `repo.watch-hidden` in the extension.

The cards built before that change with it, in the phase that changes them, their acceptance lines and tests together:
`repo.select` (its entry points in the desktop are the tabs, and the menu of the header is gone), `repo.open-folder`
and `repo.open-recent` (they ask which repositories to open, and open the tabs a workspace had), `repo.schedule-reads`
(one schedule for each tab), `worktree.show-in-gitardo` and `submodule.show-in-gitardo` (in the tab of their
repository), and the views of the header.

Exit: a workspace of eight repositories opens in eight tabs; showing each one after the first takes no read when
nothing changed in it; a window 400 logical pixels wide still reaches every tab; with
`gitardo.repositories.keepOpen` on, the extension shows a repository chosen again as it was left; every card of the
phase passes its tests, and the snapshots of the island match.

## M13 — The branch rail's own filter (done)

The branch rail lists what its own filter says, and a click on a branch narrows only the timeline
([D51](../08-decisions.md#d51--the-branch-rail-has-a-filter-of-its-own)). Built in three phases, each one commit or
more, each ending with its cards built and their tests passing; the extension comes last.

1. **The rail lists every branch.** A click on a branch narrows the timeline and leaves the rail as it was, the
   branches the timeline leaves out dimmed; `branches.followGraphFilter` is no longer read, and **Filter the list of
   branches too** leaves the timeline's branch filter. Cards: `refs.tree`, `branch.pane`, `filter.list-too`.
2. **The filter of the rail.** Its button at the top of the rail, its picker, the line of what it lists, remembered
   per repository apart from the timeline's. Cards: `refs.list-filter`, `refs.listed`, `refs.scope-line`.
3. **The extension.** The same in the references beside the graph of the extension, and
   `gitardo.branches.followGraphFilter` no longer read. Cards: the same, in the extension.

Exit: a click on any branch of the rail never takes another out of it; the filter of the rail and the timeline's never
change each other, in either app; each repository keeps its own choice of both after the window opens again; every card
of the phase passes its tests, and the snapshots of the rail match.

## Before a release

By hand, on each system, with a real account and a real model:

- [ ] Sign in to GitHub with a real token, load the pull requests, open one.
- [ ] Draft a commit message and resolve a conflict with the AI provider configured.
- [ ] Choose a folder, a file to open and a file to save with the system's dialogs.
- [ ] Open a worktree in a new window, in the external editor and in a terminal.
- [ ] Remove a worktree with a linked `node_modules` on Windows, and check that the original `node_modules` is intact.
- [ ] Look at every snapshot that changed since the last release.
- [ ] Read `cargo xtask perf` on the large repository and compare with the last release.
