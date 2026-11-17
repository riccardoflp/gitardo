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

## M11 — macOS (done)

The platform layer for AppKit. Nothing above `platform` changes; the application tests run on each.

Written without a Mac, and checked on one by the pipeline ([D48](../08-decisions.md#d48--macos-on-github-actions)):
`cargo xtask check` on macOS and both targets of the Mac built, green from the run of 7d01480 (2026-11-15) on
([Q369](../08-decisions.md#open-questions)).

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

## M14 — The window keeps its sides

The parts of the window stay where they are while files are read, a terminal and the Command log share a panel under
the timeline, the icons are the ones other applications use, and the extension can have islands
([D53](../08-decisions.md#d53--pages-beside-the-inspector-a-bottom-panel-the-icons-of-lucide-islands-in-the-extension)).
Built in six phases, each one commit or more, each ending with its cards built and their tests passing; the
extension comes last.

1. **Whole rows dimmed.** The rows a history brought out leaves out are dimmed whole, as in the extension. Cards:
   `graph.hover-history`, `graph.trace-selected`.
2. **Pages beside what opened them.** A file of the inspector opens its Diff page in place of the timeline only, the
   inspector staying beside it and moving the page from file to file; a file of the Files page opens on the right of
   its tree. Cards: `diff.beside-inspector`, `file-view.beside-files`.
3. **The bottom panel.** Its island, its tabs, its height, `` Ctrl+` ``; the Command log moved into it from the
   Activity drawer, and **Terminal** in the bar of views. Cards: `panel.bottom`, `log.show`,
   `activity.what-is-running`, `graph.view-bar`.
4. **The terminal.** A shell in the folder of the repository shown, several at once, in the panel. Cards:
   `terminal.view`.
5. **The icons of Lucide.** Every icon of the window from Lucide's outlines, one meaning each, every button looked at
   again. Cards: `graph.icons`.
6. **The extension.** `gitardo.graph.islands` in the Graph view. Cards: `graph.islands` in the extension.

Exit: selecting a commit dims the message of every row it does not reach; every file of a commit can be opened one after
the other from the inspector, which never closes; a file opened from the Files page leaves its tree on the left;
`git status` typed in the terminal of the panel prints in color, and the pager of `git log` runs there; no icon of the
window means two things; with `gitardo.graph.islands` on, the Graph view of the extension is three islands; every card of the
phase passes its tests, and the snapshots of the window match.

## M15 — The whole history in one read, and reads that cost less

The timeline reads its history with one `git log` as git prints it, rather than one for each page, and lays out only
the rows that come ([D54](../08-decisions.md#d54--the-timeline-is-read-by-one-git-log-as-it-prints)). The repository is
read again as often as before, but each read asks git only for what may have changed, and changes that git ignores ask
for no read at all.

Measured on 2026-11-17 on the repository of Gitardo (499 commits, six worktrees, four of them nested in
`.claude/worktrees` with agents building in them), with the release build of `gitardo-engine --stdio` asked as the
extension asks it: a read after a change costs 1.8 to 2.6 seconds (the median of twelve, the slowest above 5), and
starts some forty git processes, each of which costs 80 to 150 ms to start on that machine before it does anything.
`repo.read-state` takes 0.7 to 1.1 s of it, a page of `graph.read-commits` 0.35 to 0.5 s, and `journal/last`, for the
tooltip of **Undo**, 0.6 to 1.1 s, because it reads the whole state again. In 90 seconds of that work, 144 changes to
files git ignores asked for eight reads, all of them for nothing.

Built in nine phases, each one commit or more, each ending with its cards built and their tests passing; the extension
comes last.

1. **The measure first.** `cargo xtask perf` times the whole history too: from the request to its first page, and to
   its last row, and the layout of the timeline over every row, page after page, as `app` makes it today. It also
   times a read as the extension asks it after a change (`repo.read-state`, the first page of `graph.read-commits` and
   `journal/last`, one after the other) and counts the git processes each starts, on the large repository and on one
   with five other worktrees. Its table in [06-testing.md](06-testing.md) gets the numbers of before. No card.
2. **The read as git prints.** `crates/git` runs a read whose output is taken as it comes, cancelled as any read; the
   read of the commits gives each page to the worker as soon as it is printed, and stops at 100,000 commits; the store
   adds each page under the rows, and starts again on another history. Cards: `graph.read-commits`, `graph.load-more`.
3. **The layout from the last row.** The lanes as they stand under the last row are kept with the layout, and a page
   is laid out from them; the folds, the labels and the date separators of the new rows with it. Cards: `graph.lanes`.
4. **The state read once for each read.** `journal/last` takes the state of the last `repo.read-state` of the
   repository, which the engine keeps, instead of reading it again; it reads only when none was read since the last
   operation. Cards: `undo.last`, `repo.read-state`.
5. **What git ignores asks for no read.** A changed file that git ignores, or that is in the folder of another
   worktree nested in this one, asks for nothing; whether git ignores them is asked once for the files of a burst, with
   `git check-ignore --stdin`, before the read is scheduled. A `.gitignore` that changes is not ignored itself, and
   asks for a read. Cards: `repo.watch-changes`, `repo.schedule-reads`.
6. **The other worktrees counted when they change.** `git status` runs in another worktree on the first read, on
   **Refresh**, and when something in its own git folder (`.git/worktrees/<name>/`) changed since the read before;
   otherwise its count of what is not committed is the one read before. Cards: `repo.read-state`,
   `repo.watch-changes`.
7. **The commits not read again when the history is the same.** A read whose commit checked out, references and stashes
   are those of the read before keeps the rows it has, without `git log`, as a change to the files leaves them;
   **Refresh**, a filter, a search, the branches shown and `graph.load-more` read them as before. A read of
   `graph.read-commits` asked after `repo.read-state` takes what is checked out, the references and the stashes from
   it, instead of reading them again. Cards: `repo.schedule-reads`, `graph.read-commits`.
8. **The focus asks for a read only when one is due.** The window getting the focus asks for a read only when a change
   came while it asked for nothing (the window minimized), or when the last read is more than a minute old. Cards:
   `repo.watch-changes`.
9. **The extension.** Version 1.8 of [the protocol](../transition/02-protocol.md), the pages sent as they are read,
   `EngineGraph` and the controller taking them, and the lanes of `src/shared/graph.ts` laid out from the last row.
   Its watchers of the working tree (`Controller.watch`) ask the engine which files of a burst git ignores and leave
   out the worktrees nested in the repository, `EngineGraph` keeps its rows when the history is the same, and the focus
   follows phase 8. Cards: `graph.read-commits`, `graph.load-more`, `graph.lanes`, `repo.watch-changes`,
   `repo.schedule-reads` and `undo.last` in the extension.

Exit: on the repository that `extension/test/perf/big-repo.mjs` makes (56,120 commits), measured by `cargo xtask perf`
with the release build, the first page comes no later than the first page did before M15, and every commit is read
and laid out in less than 3 seconds, in the window and in the extension; scrolling to the last commit asks for nothing
more; another branch filter shows its first page as soon as the first page did before. On the repository of Gitardo
with its worktrees, a read after a change to a file starts `git status` once in the worktree shown and in no other,
reads the state once, and runs no `git log`; `cargo build` in `rust/target` and an agent writing in a worktree under
`.claude/worktrees` ask for no read of the repository, in either app; `cargo xtask perf` reports a read after a change
in at most half the time it reported in phase 1, on the same machine. Every card of the phase passes its tests, and the
snapshots of the timeline match.

## M16 — The settings in sections, and a side column

The settings page in the middle of the window, in sections, every value changed on it
([D55](../08-decisions.md#d55--the-settings-page-in-sections-in-the-middle-of-the-window)); the repositories and the
branches in one column on the left, the header a bar across the top that says the repository, the worktree and the
branch, what was said in the status bar, the diffs of a commit and of a comparison beside their files, a chooser of the
two sides of a comparison, all commits on one line, and menus that fit the window
([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).
Built in twelve phases, each one commit or more, each ending with its cards built and their tests passing; the extension
comes last. It comes after M14, whose icons of Lucide the buttons of the sections use, and whose Diff page beside the
inspector phase 10 changes.

1. **Menus and pickers longer than the window.** A menu under a button stays under it and scrolls when it is too tall,
   with a bar, the wheel and the arrows; a picker has the same bar and keeps the row lit in view. A fix first, since
   the **…** menu cannot be used whole in a short window. Cards: `ui.menu-scroll`, `ui.picker-scroll`,
   `graph.more-menu`.
2. **Settings: sections, in the middle.** The page centered, the column of the sections and the list under their names,
   the button of the section shown marked as the list scrolls, the menu of a narrow page; a title for each setting, its
   description as long as it needs, its key and the file that sets it. Cards: `settings.sections`, `settings.edit`.
3. **Settings: the search, Changed and Experimental.** The field at the top, the sections with nothing found dimmed, the
   two lists apart, the count and the mark beside each section. Cards: `settings.search`, `settings.sections`.
4. **Settings: every value on the page.** Numbers, strings, lists, `identities`, colors and fonts, written when the
   field loses the focus; the bar of a setting a file sets, and the button that takes it back to its default. Cards:
   `settings.edit`, `settings.reset`, `settings.theme`.
5. **Settings: User and Repository as tabs, and a way in from Quick search.** The two tabs in place of the radio
   buttons, `Set by the repository` in **User**; **Open Settings: <section>** and **Open Settings: <title>**, the page
   scrolled to it. Cards: `settings.edit`, `settings.reveal`.
6. **All commits on one line.** `graph.oneLane`, its switch in the **…** menu, every dot on one line, merges as rings.
   Cards: `graph.one-lane`, `graph.more-menu`.
7. **The side column and the header.** The island of tabs gone; **Repositories** and **Branches** in one island from
   under the header to the status bar, each folding by its heading, the line between them dragged; a row for each
   repository open, its **▾** with its worktrees and submodules; the header a bar on the ground across the top, always
   shown, saying the repository, the worktree and the branch, without the counts, which **Sync** alone shows; the button
   that puts the side column away in the top row of the timeline. Cards: `repo.side-column`, `repo.tabs`,
   `repo.group-tabs`, `graph.toolbar`, `graph.where`, `graph.islands`, `graph.resize-panes`, `graph.toggle-references`,
   `activity.status-bar`.
8. **What each worktree is on.** The branch, the files changed and the commits to pull and to push of each repository
   and of each of its worktrees, read in the background when their git folder changes, under each row and in each
   **▾**. Cards: `repo.worktree-summary`, `repo.tabs`, `repo.group-tabs`.
9. **What was said, in the status bar.** The bars under the header gone; what an operation said and the notices after
   what is running, with their links and **×**, the status bar as high as it is. Cards: `activity.said`,
   `activity.status-bar`.
10. **A diff beside its commit.** The side column put away while the Diff page of a file of the inspector is shown, the
    inspector as the column of the commit's files, a tree, the file shown marked. Cards: `diff.beside-inspector`.
11. **A comparison beside its diffs, and the chooser.** A file of the comparison page opens its diff on the left with
    the comparison as a column on the right; the chooser in place of the two questions, also for **Choose another
    side**. Cards: `compare.beside-diff`, `compare.chooser`, `compare.choose`, `compare.change-side`.
12. **The extension.** `contributes.configuration` as the same sections, in the same order, in VS Code's Settings
    editor; `gitardo.graph.oneLane` and its switch in the Graph view. Cards: `settings.sections` and `graph.one-lane` in
    the extension.

Phases 13 to 17 were asked by the maintainer after trying the window built by the first twelve
([D57](../08-decisions.md#d57--worktrees-in-the-side-column-pages-that-keep-what-opened-them-the-way-back-in-the-title)),
desktop only:

13. **Worktrees in the side column.** A repository of **Repositories** with worktrees or submodules folds out by a
    chevron at its left, its worktrees and its submodules as rows under it, indented, each with the same branch, files
    changed and `↓ ↑`, the one shown marked; a click on one shows it. The **▾** menu of a row goes. Cards:
    `repo.tabs`, `repo.group-tabs`, `repo.worktree-summary`, `repo.side-column`.
14. **A side column that takes less room.** The same things in less height and less width: a repository, a worktree
    and a branch each on one line where they fit, smaller gaps between rows and sections, the heading of **Branches**
    and its buttons on one line. Cards: `repo.side-column`, `repo.tabs` and the cards of the rail it changes.
15. **The side column put away from every page.** The button that puts the side column away and brings it back is in
    the title row of every page as it is in the top row of the timeline, at the same place, with the same key. Cards:
    `repo.side-column`, `graph.toggle-references`.
16. **The Changes page beside what it opens.** A file of the Changes page opens its Diff page or its File page on the
    right of the list of changes, which stays as a column, the file shown marked, staging and discarding still done
    from it; another file of the list opens in the same place. Cards: `changes.beside-file`, `changes.open`,
    `changes.pane`.
17. **The way back, in the title of a page.** The title row of a page says the way it was reached, one name for each
    page it was opened from, each going back to that page as it was left; at most four names, those in between folded
    into **…**, which lists them; `Alt+Left` goes back one. Cards: `ui.page-trail`.

Exit: the **…** menu of a window 400 logical pixels high opens under its button and reaches its last entry; the settings
page opens in the middle of a window 1,600 logical pixels wide with **General** marked, `blame` typed in its search
leaves only **Files and blame** lit, and `graph.pageSize` set to 500 on it writes only that key, comments kept; with
**Show all commits on one line** every dot of a repository with merges is on one line; the header says
`api › login › feature/login` over the settings page as over the timeline; with three repositories open, the
side column lists them under **Repositories** with the branch, the files changed and `↓ ↑` of each, and each worktree in
its **▾** with its own; folding **Branches** gives its height to **Repositories**; a pull that succeeded says so in the
status bar, which is as high as before; a file of a commit opened from the inspector leaves no side column and the
commit's files as a tree on the right; a file of a comparison opened from its page leaves the comparison on the right;
`main` then `feature` chosen in the chooser opens `main ↔ feature`; every card of the phase passes its tests, and the
snapshots of the window, the settings page and the chooser match, in each theme; the worktrees of a repository are rows under it
in **Repositories**, shown by a click; the side column of a repository with three worktrees and two local branches is
less high than before phase 14; the side column is put away and brought back from the settings page, the Changes page
and a Diff page; a file of the Changes page opened in its Diff page leaves the list of changes on its left; a File page
opened from the Diff page of a file of a commit says `Timeline › <commit> › <file>` and its commit goes back to the
Diff page.

## M17 — The window draws its own title bar

The logo and the name of Gitardo at the top left of the window, and no title bar of the system: the header bar is the
title bar, which moves, maximizes and closes the window as the system's did, with everything the system does for its
own ([D58](../08-decisions.md#d58--the-window-draws-its-own-title-bar-with-the-logo-and-the-name-of-gitardo)). Built in
four phases, each one commit or more, each ending with its cards built and their tests passing. The extension has
VS Code's title bar, and nothing changes there.

1. **The logo and the name.** At the top left of the window, above the bar of views, the logo of Gitardo, the one of
   `extension/media/gitardo.svg` drawn as an icon of the window, in each theme; then, at the start of the header bar,
   the name **Gitardo**, before `repository › worktree › branch`. In the narrow layout the name goes and the logo stays.
   Cards: `window.brand`, `graph.where`.
2. **No title bar of the system, on Windows.** The window opens without the system's title bar, keeping what Windows
   gives its windows: the shadow, the rounded corners, Snap Layouts on the maximize button, Aero Snap and its keys. The
   free parts of the header bar are the caption: dragged they move the window, a double click maximizes or restores it,
   a right click or `Alt+Space` opens the system's menu of the window. **Minimize**, **Maximize** or **Restore** and
   **Close** are at the right end of the header bar, drawn by Gitardo as Windows draws them, **Close** red under the
   pointer, named for the screen reader. The edges and the corners resize the window, with their cursors; maximized,
   nothing resizes and the buttons reach the top edge of the screen. Cards: `window.title-bar`,
   `window.caption-buttons`.
3. **Every event of the window.** The header bar dims when the window loses the focus, as the system's title bar does;
   the title of the window, which the taskbar, `Alt+Tab` and the screen reader say, is set as before though it is not
   drawn; a menu or a dialog of Gitardo open while the window is moved or resized closes; a change of screen or of
   scale while dragging lays out the bar again; the size, the place and whether it is maximized are kept for the next
   start, as before. Cards: `window.title-bar`, `window.caption-buttons`.
4. **macOS and Linux.** On macOS the title bar of the system is transparent, the window's content under it, its three
   buttons kept at the top left with the logo after them; on Linux, no decorations, the bar moves, resizes and opens the
   menu of the window through the compositor, and the three buttons are drawn as on Windows. Cards:
   `window.title-bar`, `window.caption-buttons`.

Phases 5 and 6 were added by the maintainer after the plan of the first four ([D59](../08-decisions.md#d59--markdown-files-shown-as-they-read)):

5. **Markdown as it reads.** A Markdown file in the File page can be shown as a preview, laid out as it reads: headings,
   paragraphs, emphasis, lists and task lists, quotes, code blocks in the colors of the code, tables, links that open
   in the browser or open the file of the repository they name, images of the repository; **Preview** and **Source**
   in the title row switch between the two, the preview following the text as it is edited. The setting
   **Open Markdown as a preview** (`markdown.openAsPreview`, in **Files and blame**, off by default) opens every
   Markdown file in its preview. Cards: `file-view.markdown-preview`, `file-view.page`, `settings.edit`.
6. **The extension.** `gitardo.markdown.openAsPreview` opens a Markdown file that Gitardo opens (from the Files view,
   the Changes view, a commit) in VS Code's preview of Markdown rather than its editor. Cards:
   `file-view.markdown-preview` in the extension.

Phases 7 and 8 were added by the maintainer after the plan of the first six ([D60](../08-decisions.md#d60--mermaid-diagrams-drawn-in-the-preview-of-markdown)):

7. **Mermaid diagrams.** A block of code fenced as `mermaid` in the preview of a Markdown file is drawn as its diagram,
   laid out by the library `merman` and drawn into pixels by `resvg`: flowcharts, sequence diagrams and the other kinds
   it draws, in the colors of the window and with its labels in Inter, following the theme, as wide as the page at
   most. It is drawn on a worker and kept for its text and its theme; until it is, its code shows. A diagram with an
   error, or of a kind not drawn, shows its code and one line saying why; nothing in it is followed, run or fetched;
   the screen reader reads its title. Cards: `file-view.mermaid`, `file-view.markdown-preview`.
8. **The extension.** VS Code's preview of Markdown draws a `mermaid` block as the diagram the engine drew for the kind
   of VS Code's theme, through a plugin of its Markdown (`markdown.markdownItPlugins`) and a read of the protocol that
   needs no repository; until the engine answers, and without it, the block shows its code. Cards:
   `file-view.mermaid` in the extension.

Exit: the window opens with no title bar of the system, the logo at its top left and **Gitardo** at the start of the
header bar; dragging the free part of the header bar moves the window and dragging it to the top of the screen
maximizes it; a double click maximizes and restores it; the pointer on **Maximize** on Windows 11 shows Snap Layouts;
the edges and corners resize it, and nothing resizes it while it is maximized; `Alt+Space` opens the system's menu of
the window; the bar dims when another window has the focus; the size and place are kept from one start to the next;
`README.md` opened from the Files page with **Open Markdown as a preview** on shows its headings, lists, a table and
a code block laid out, and **Source** shows its text; a `mermaid` block of a flowchart and one of a sequence diagram
are drawn as their diagrams in each theme, in VS Code's preview too, and one with an error shows its code and why;
every card of the phase passes its tests, and the snapshots of
the header bar match in each theme, focused and not, and those of the preview of Markdown in each theme.

## Before a release

By hand, on each system, with a real account and a real model:

- [ ] Sign in to GitHub with a real token, load the pull requests, open one.
- [ ] Draft a commit message and resolve a conflict with the AI provider configured.
- [ ] Choose a folder, a file to open and a file to save with the system's dialogs.
- [ ] Open a worktree in a new window, in the external editor and in a terminal.
- [ ] Remove a worktree with a linked `node_modules` on Windows, and check that the original `node_modules` is intact.
- [ ] Look at every snapshot that changed since the last release.
- [ ] Read `cargo xtask perf` on the large repository and compare with the last release.

Then the release of both apps, with one version, as the extension's roadmap says in
[Before a release](../extension/07-roadmap.md#before-a-release) and
[D52](../08-decisions.md#d52--a-tag-publishes-the-release-once-the-maintainer-approves).
