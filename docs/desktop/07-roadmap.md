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

Phase 9 was added by the maintainer after phases 7 and 8
([D61](../08-decisions.md#d61--the-tree-of-the-files-page-shows-what-differs-from-the-last-commit)):

9. **What differs, in the tree of the Files page.** Each file of the tree that differs from `HEAD` is marked as the
   Changes page marks it: its name in the color of its state and its letter at the right end of its row (`A`, `M`,
   `D`, `R`, `C`, `T`, `?`, `!`), staged and unstaged changes taken together, a conflict over everything. A renamed
   file is at its new path, its tooltip naming the old one; a deleted file is at its place, even when its deletion is
   staged, dimmed and struck through, and a click on it opens its Diff page. A folder holding a changed file, at any
   depth, has a dot in the color of the most important state under it, so that a folded folder still tells. The marks
   follow every read of the repository. The extension has no Files view, and VS Code's Explorer marks the changes of
   git itself. Cards: `file-view.files-changes`.

Phase 10 was added by the maintainer after phase 9
([D62](../08-decisions.md#d62--the-icons-of-files-and-folders-are-those-of-symbols-in-the-colors-of-the-theme)):

10. **The icons of files and folders.** Each file and folder of the repository has the icon of its kind, from the icon
    theme of Symbols (MIT), chosen by its name, its extension or its language as the theme chooses them in VS Code: in
    the tree of the Files page, the rows of the Changes page, the files of a commit and of a comparison, the page of new
    worktrees. Each color of an icon is drawn in the color the theme gives its family of hues, the gray of the folders
    in the muted text; the icons are drawn into pixels by `resvg` once for each size, scale and theme. The setting
    **File icons** (`fileIcons`, in **Appearance**) draws them in the colors of the theme (`symbols`, the default), in
    the color of the text (`monochrome`) or as the plain file and folder of before (`simple`). The extension shows the
    file icon theme of VS Code. Cards: `file-view.file-icons`, `settings.edit`.

Phase 11 was added by the maintainer after phase 10
([D63](../08-decisions.md#d63--the-icon-of-each-action-in-the-menus)):

11. **The icon of each action in the menus.** Every item of every context menu and of every menu under a button that
    does something has the icon of its action at the left of its words, the one a button of the same action draws, in
    the color of the words and dimmed with them: the copy icon on every **Copy …**, the bin on every **Delete …**, the
    Timeline on **Show in the timeline**. Eight icons of Lucide are added for the actions the table did not have (copy,
    cut, paste, rename, check out, a new window, reveal, unfold). The items no icon says (the rewrites of the history,
    the values of a list) keep their words lined up with the others; the switches keep their check. The extension's
    own menus are left for later. Cards: `ui.menu-icons`, `graph.icons`.

Exit: the window opens with no title bar of the system, the logo at its top left and **Gitardo** at the start of the
header bar; dragging the free part of the header bar moves the window and dragging it to the top of the screen
maximizes it; a double click maximizes and restores it; the pointer on **Maximize** on Windows 11 shows Snap Layouts;
the edges and corners resize it, and nothing resizes it while it is maximized; `Alt+Space` opens the system's menu of
the window; the bar dims when another window has the focus; the size and place are kept from one start to the next;
`README.md` opened from the Files page with **Open Markdown as a preview** on shows its headings, lists, a table and
a code block laid out, and **Source** shows its text; a `mermaid` block of a flowchart and one of a sequence diagram
are drawn as their diagrams in each theme, in VS Code's preview too, and one with an error shows its code and why; in
the Files page a modified file shows `M`, a file whose deletion is staged is listed struck through and opens its Diff
page, and a folded folder holding a change shows its dot; `src/main.rs` has the icon of Rust and `src` its folder of
code in the Files page, in the orange and the gray of each theme, in the color of the text with **File icons** set to
`monochrome` and as before with `simple`; the menu of a commit shows the copy icon beside **Copy hash**, and the
menu of a branch the bin beside **Delete**; every card of the phase passes its tests, and the snapshots
of the header bar match in each theme, focused and not, those of the preview of Markdown and those of the tree of the
Files page with its marks and with the icons of its files, and those of the menus of a commit and of a file, in each theme.

## M23 — What shows under the pointer stays under it

A fix, asked by the maintainer as urgent and built before the milestones that are not built yet, whatever its number:
the **×** of a row of **Repositories** flickers while the pointer is on it and cannot be clicked, and so does every
control the window shows only on the row under the pointer. In Gitardo Desktop alone: the webviews of the extension
ask the browser, which answers right.

The cause, read in the code: a widget is `hovered` only while it is the topmost one under the pointer
(`ui::Response::hovered`), and a row draws its **×** only while the row is hovered (`views::repo::rows`). With the
pointer on the **×**, the **×** is the topmost: the row is no longer hovered and does not draw it; in the next frame
nothing is over the row, which is hovered again and draws it; and so on, a frame with and a frame without, so that a
press and its release seldom meet the same button. The row shown always draws its **×**, which is why that one works.
The rows of the files of a commit already work around it by asking where the pointer is (`views::details::rows`); the
others do not.

Built in two phases, each one commit or more, each ending with its tests passing.

1. **The toolkit says whether the pointer is within a widget.** Beside `hovered`, what a widget is told says whether
   the pointer is over it or over something drawn inside it, with nothing of another layer covering it; the
   background of a row under the pointer and whatever a row shows only under the pointer ask that. A test of `ui`
   holds, with a row and a button it shows under the pointer, that with the pointer resting on the button for ten
   frames the button is drawn in each, the row keeps its background, and a click presses the button and not the row.
   A rule of the code says which of the two a view asks, and the check of the rules refuses a control drawn only
   while `hovered`. Cards: `repo.tabs`, `repo.close-tab`.
2. **Every place that shows something under the pointer.** Each of the some forty places of `views` that read
   `hovered` is looked at and asks the right one: the **×** of the rows of **Repositories**, the buttons on the titles
   of the lists of the branch rail and of its filters, the rows of branches, stashes, worktrees and submodules, the
   buttons of the rows of the Changes page, of the files of a commit (its workaround replaced) and of a comparison,
   the rows of the tasks of Activity, of the settings and of what new worktrees get, the labels of the timeline and
   their list. For each control shown only under the pointer, a test of the window moves the pointer from the row
   onto the control, rests there ten frames and clicks: every frame draws the same, and the click does what the
   control says. Cards: `repo.tabs`, `repo.close-tab`, `refs.tree`, and those of each place changed.

Exit: with three repositories open, the pointer moved onto the **×** of one not shown finds it still, lit as a button
under the pointer, and a click closes that tab; the same holds for each control of phase 2; while the pointer rests
anywhere in the window, two frames in a row are the same; the check of the rules passes.

## M18 — Activity in the bottom panel, and a list that holds still

What Gitardo is doing is a tab of the bottom panel, beside the terminals and the Command log, rather than a card over the
timeline; its list keeps each row where it is while things start and end, and what ran stays long enough to be read
([D64](../08-decisions.md#d64--activity-a-tab-of-the-bottom-panel-whose-list-holds-still)). Built in three phases, each
one commit or more, each ending with its cards built and their tests passing. Desktop only: the extension says what is
running in VS Code's status bar and in **Gitardo: Show What Is Running** (`activity.track`), and nothing changes there.

1. **What makes the list jump, found.** A test that replays what the engine says while a burst of changes asks for reads
   in a row (forty reads in ten seconds, a fetch running through them, two reads that end within the same frame) and
   writes the list of every frame, so that a row moving, flashing or going while it is read fails it. The causes it
   finds are written under the decision before anything is fixed; reading the code points at five (D64). No card
   changes; the test stays for phase 3.
2. **Activity, a tab of the bottom panel.** The drawer gone; **Activity** a tab of the bottom panel, after **Terminal**
   and before **Command log**, with the height, the gap and the **Hide the panel** of the others; the indicator of the
   status bar, **Activity** in the bar of views and **Show What Is Running** show the panel at it, or hide it when it is
   shown with the focus; `Escape` no longer closes anything there. **Show the commands Gitardo runs** becomes the tab
   beside it. The commands of new worktrees keep their column at the right of the tab until M19 takes them. Cards:
   `activity.what-is-running`, `panel.bottom`, `activity.status-bar`, `graph.view-bar`,
   `worktree-setup.command-steps`.
3. **A list that holds still.** Each activity counted from when the engine started it; what runs less than half a
   second listed neither in the indicator nor in the tab; a row keeps its place while it runs and, once it ended, goes
   under **Ran** with the time it ended and how long it took; reads of the same repository in a row one row that counts
   them; **Ran** keeps the last 50 of the session, the latest first, and the list scrolls when it is taller than the
   tab. Cards: `activity.what-is-running`, `activity.describe-task`, `activity.status-bar`.

Exit: the window has no card over the timeline while something runs; a click on the activity indicator shows the bottom
panel at **Activity**, and the timeline keeps its height above it; during a fetch with forty reads in a row around it,
the replay of phase 1 sees no row move or go before the fetch ends, the fetch counts its seconds from its start, and
the reads are one row; a fetch that ended ten minutes ago is still under **Ran**, with its time; every card of the
phase passes its tests, and the snapshots of the bottom panel at **Activity** match, empty, running and with the commands
of a new worktree, in each theme.

## M19 — Tasks: the tasks of the repository, in the bottom panel

The tasks of the repository listed in a tab of the bottom panel, each run in a terminal beside the list, and the groups
that run several at once: what the extension Run My Tasks does in VS Code, inside Gitardo Desktop
([D65](../08-decisions.md#d65--tasks-a-tab-of-the-bottom-panel-with-the-tasks-of-the-repository-and-their-groups)).
Built in seven phases, each one commit or more, each ending with its cards built and their tests passing. It comes
after M18, whose tab of Activity gives the commands of new worktrees to this one. Desktop only: in VS Code, its tasks
and Run My Tasks do it, and the cards are *Host only* there.

1. **The tasks read.** The tasks of `.vscode/tasks.json` of the worktree shown, shell and process, with their options,
   the overrides of each system and the variables Gitardo can resolve; the scripts of every `package.json` outside
   `node_modules`, run with the package manager its lock file names; read again when one of those files changes. Cards:
   `tasks.read`, `repo.watch-changes`.
2. **The Tasks tab, and a task run.** **Tasks** a tab of the bottom panel, after **Terminal**: the list on the left,
   by source, with its search; on the right, the terminal of the task selected, a pseudo-terminal as those of
   `terminal.view`, which keeps what the task printed once it ended. **Run** and **Stop** on each row, a spinner while
   it runs, its exit code once it ended; a task running asked to run again offers **Restart** or **Leave it running**.
   Nothing runs in a folder that is not trusted. Cards: `tasks.view`, `tasks.run`, `tasks.stop`, `panel.bottom`.
3. **Tasks that wait for others.** `dependsOn`, in sequence or in parallel, a task failed stopping those that wait for
   it; a background task counted as ready when its problem matcher's `endsPattern` is printed, or when it is still
   running after its `beginsPattern` and some time with nothing printed. Cards: `tasks.run`.
4. **Groups.** **Groups** at the top of the list: a group made, renamed, deleted; a task added from its menu or dragged
   onto a group, moved in it by dragging or with **Move up** and **Move down**; **Run all** and **Stop all** on each
   group; the groups of the repository kept in a file of its git folder, the same for every worktree. Cards:
   `tasks.edit-groups`, `tasks.run-group`.
5. **Groups from Run My Tasks.** **Import groups…** reads the file that **Export Groups...** of Run My Tasks saves, and
   **Export groups…** writes one it reads; a group named as one Gitardo has is merged, replaces it or is left out, as
   asked; the launch configurations of the file are left out and said. Cards: `tasks.import-groups`,
   `tasks.export-groups`.
6. **A task from anywhere.** **Run Task…** in Quick search and on a key, a picker of every task by source, the groups
   first, each with its command under its name; a task running shows a count beside **Tasks** in the tabs of the
   panel. Cards: `tasks.quick-run`, `tasks.view`.
7. **The commands of new worktrees among the tasks.** The commands of `worktrees.setupCommands` run as tasks of their
   own source, **New worktrees**, in the Tasks tab, each with its terminal, rather than in the tab of Activity. Cards:
   `worktree-setup.command-steps`, `activity.what-is-running`, `tasks.view`.

Exit: in the repository of Gitardo, the Tasks tab lists the tasks of its `tasks.json` and the scripts of
`extension/package.json`; `npm run build` run from it prints in color in its terminal and ends with its exit code, and
**Stop** on `npm run watch` ends it and its children, on Windows too; a task that depends on a background task starts
once that one prints its `endsPattern`; a group of two tasks runs both with **Run all** and is listed the same in another
worktree of the repository; the file exported by Run My Tasks 0.8.0 is imported with its groups; **Run Task…** runs a
task from the timeline; a new worktree's `npm ci` runs in the Tasks tab; in a folder not trusted nothing runs and the tab
says why; every card of the phase passes its tests, and the snapshots of the Tasks tab match, with a task running and
one ended, in each theme.

## M20 — The timeline in one line a commit, and its two menus

The labels of a commit come first on its row, and the many labels of a busy commit open under the pointer rather than
being cut ([D66](../08-decisions.md#d66--the-labels-of-a-commit-first-on-its-row-the-rest-under-the-pointer)); the
timeline can show each commit on one line, the compact rows
([D67](../08-decisions.md#d67--compact-rows-each-commit-on-one-line)); and the **…** menu of the header, which held
the switches of the timeline and its actions together, becomes two menus
([D68](../08-decisions.md#d68--two-menus-in-the-header-the-view-of-the-timeline-and-its-actions)). Asked by the
maintainer after M17. Built in four phases, each one commit or more, each ending with its cards built and their tests
passing.

1. **The labels first.** On each row of the timeline the labels of the commit (`graph.labels`) come at the start of
   the line of its message, right after the cell of the graph, in their order, and the message after them. They take
   at most a part of the width of the row; the labels that do not fit are gathered in one label `+N`. The pointer
   resting on `+N`, or on any label of the row, opens under it a list of all the labels of the commit, each with its
   icon and its color, each one doing what its label does: a click, a double click (check out, `branch.open`), a
   right click (its menu), the hover that brings out its history. The list closes when the pointer leaves it and its
   labels. Cards: `graph.labels`, `graph.label-overflow`, `graph.view`.
2. **Compact rows.** A setting, `graph.compactRows` (off by default), and its switch **Compact rows** in the menu of
   the view (phase 3): each commit on one line, its labels, then its message, then at the right end the author, the
   date and the hash as `graph.columns` asks, in columns that line up from row to row; the rows lower, the cell of the
   graph following them. The draft of the uncommitted changes, the separators of days and the stashes follow too.
   Cards: `graph.compact-rows`, `graph.view`, `graph.columns`.
3. **Two menus.** In the header, in place of **…**: a menu of the view of the timeline, with every switch of what it
   shows and how (the kinds of branches, the stashes, the first parent, one lane, the history of the selected commit,
   folding, the columns, the separators of dates, the combined upstream, **Compact rows**); and **…**, the menu of the
   actions (refresh, branch, stash, undo, compare, go to commit, reflog, clean up, patches, remotes, submodules,
   identity, pull requests, settings, theme). Cards: `graph.view-menu`, `graph.more-menu`.
4. **The extension.** The labels first with their list under the pointer, the compact rows and the two menus in the
   graph of the extension, where its webview draws them as the desktop does; what VS Code draws itself stays as VS
   Code draws it. Cards: the same, in the extension.

Exit: in a repository with a commit carrying `main`, `origin/main`, three tags and a stash, the row starts with the
labels that fit and `+N`, the pointer on `+N` lists all six and a double click on `main` in that list checks it out;
with **Compact rows** on, every commit is on one line with the author, the date and the hash lined up at the right, and
off, as before; the header has the menu of the view, with every switch, and **…** with the actions alone; every card
of the milestone passes its tests, and the snapshots of the timeline match in each theme, with compact rows and
without, and those of the two menus.

## M21 — The selection in the colors of the theme

A selected commit, a selected row of any list and selected text were tinted teal in the dark and in the light theme:
the color those themes give information, not their accent, so that what is selected looked foreign to the amber of
the rest of the window
([D69](../08-decisions.md#d69--what-is-selected-is-tinted-with-the-accent-of-the-theme)). Asked by the maintainer
after M20. In Gitardo Desktop alone: the extension draws what is selected in the colors of VS Code's theme. Built in
three phases, each one commit or more, each ending with its tests passing.

1. **The colors of the two themes.** In the dark and in the light theme the ground of the selected rows of a list
   that has the focus (`list_selection`) is the accent mixed into the ground of the page, about one part in six: a
   warm brown near `#382D1F` in the dark theme, a sand near `#EADBC8` in the light one; the text on it keeps its
   contrast. The ground of selected text (`text_selection`) loses its teal too (Q671). The selection of a list
   without the focus, the row under the pointer and the row lit in a menu stay the neutral steps they are; the two
   high contrast themes, whose accent is the blue of their selection, are left alone. A test of `render` holds for
   every theme that the selection of a list has the hue of the accent or none, that text on it is read (4.5 to 1, 7
   to 1 in high contrast), and that it is told apart from the ground, from the row under the pointer and from the
   selection without the focus. Card: `settings.theme`.
2. **Every place that shows a selection.** Each is looked at in the four themes and draws what is selected with the
   colors of the theme, none with a color of its own: the rows of the timeline and of its compact rows, the side
   column and its lists, the Changes page, the files of a commit and of a comparison, the tree of the Files page, the
   chooser of a comparison, the pickers and Quick search, the settings page and its row marked for a second, the tabs
   of the bottom panel, the log and the tasks of Activity, the text selected in the editor, in a box, in a diff, in the
   Merge page and in the terminal. Their snapshots are recorded again and looked at. Cards: `settings.theme`,
   `graph.view`, `ui.picker-scroll`, and those of the pages whose snapshots change.
3. **The hues that are left.** Every color of the dark and of the light theme is checked against what the look allows
   ([D17](../08-decisions.md#d17--the-timeline-a-window-of-gitardos-own)): the ground and its steps, one accent, the
   colors of an error, a warning, information and success where they say that, the colors of the lanes and of the
   kinds of labels, the colors of the code. A color used for something else is changed to the one of its meaning, or
   written as a question when the choice is the maintainer's. Card: `settings.theme`.

Exit: in the dark and in the light theme a selected commit, a selected file and a selected setting are tinted with
the amber of the theme, as the buttons and the ring of the focus are, and no place of the window shows a selection in
teal; text selected is told apart from a match of a search; the high contrast themes are as before; the test of the
colors of the themes passes, and the snapshots of every place of phase 2 match in each theme.

## M22 — Repositories, a view of the extension

The repositories of the workspace, with their worktrees and their submodules under them and what each is on, in a
view of Gitardo in VS Code, as the **Repositories** section of the side column lists them in Gitardo Desktop
([D70](../08-decisions.md#d70--the-extension-has-a-repositories-view-of-its-own)). Asked by the maintainer after
M21. In the extension alone: the desktop has it since M16, and its cards are the ones built. Built in three phases,
each one commit or more, each ending with its cards built in the extension and their tests passing.

1. **What each is on, from the engine.** The engine gives the extension what the desktop reads for the rows of
   **Repositories**: for each repository of the workspace and each of its worktrees and submodules, the branch checked
   out or the commit `HEAD` is detached at, the files changed and the commits to pull and to push
   (`repo.worktree-summary`), read once, then again only when its folder or its git folder changes, and said to the
   extension when one changed. A read added to the protocol, a minor version. Cards: `repo.worktree-summary`,
   `repo.watch-hidden`.
2. **The view.** **Repositories**, a view of the Source Control side bar, above Gitardo's other views, drawn by the
   webview as the desktop draws the section: a row for each repository Gitardo found in the workspace
   (`repo.read-repositories`), named as `repo.name-repositories` says, the rows of its worktrees and its submodules
   under it, folding (`repo.group-tabs`); at the right of each row what it is on, `main ● 3 ↓2 ↑1`; the one Gitardo
   shows marked with the bar of the accent, a spinner on a row whose repository runs an operation. A click on a row
   shows it in Gitardo (`repo.select`, `worktree.show-in-gitardo`, `submodule.show-in-gitardo`); the rows take the
   keyboard and say themselves to the screen reader. Which rows are folded is remembered for the workspace. Cards:
   `repo.tabs`, `repo.group-tabs`, `repo.select`.
3. **Its menus, and VS Code's own view.** The menu of a row: **Reveal in File Explorer**, **Copy path**, **Open in
   Integrated Terminal**, and for a worktree and a submodule what their menus have in the Branches view. The view and
   VS Code's Repositories view follow each other as Gitardo and that view already do
   (`repo.follow-source-control`). The picker of the repository in the toolbar of the graph stays. The README of the
   extension and its walkthrough say the view; a step of the end-to-end suite opens it, reads its rows and clicks one.
   Cards: `repo.tabs`, `repo.follow-source-control`.

Exit: in a workspace with two repositories, one with two worktrees and a submodule, the view lists the two with
their rows under them, each with its branch, its files changed and its commits to pull and to push; changing a file
in a worktree not shown updates its count within two seconds; a click on a worktree shows it in the graph and marks
its row, and choosing another repository in the toolbar of the graph marks that one; nothing is read while nothing
changes; every card of the milestone is Built in both apps, and the end-to-end step passes.

## M24 — Changes, a section of the side column

The changes of the repository shown are in the side column, between **Repositories** and **Branches**; the sections
keep the line above their headings when they are folded; the files of the changes have no buttons on their rows, and
what the buttons did is in the menu of the row; a click on a file shows its diff, in the middle of the window
([D71](../08-decisions.md#d71--changes-a-section-of-the-side-column-whose-files-open-their-diffs)). Asked by the
maintainer after M23. In Gitardo Desktop alone: in the extension the Source Control side bar of VS Code is the column
and **Gitardo Changes** a view of it, and whether its rows and its click follow is asked (Q763, Q764). Built in four
phases, each one commit or more, each ending with its tests passing; the pictures of the window, nearly all of which
show the side column, are recorded again once, when the section is as it stays.

1. **A line above every heading.** The heading of every section of the side column but the first has a line above
   it, whether the sections around it are open or folded; before, the line was drawn only between two open sections.
   Only a line between open sections is dragged. Where the sections go and which lines are dragged is one function
   of what is folded, tested for every combination. Cards: `repo.side-column`, `graph.resize-panes`.
2. **The actions of a row, in its menu.** The rows of the changed files and of the files in conflict lose their
   buttons and keep their boxes; the context menu of a row has what the buttons did, each item with its icon, and
   opens by the right button and by the menu key on the row that has the keyboard, which `Tab` reaches. The buttons
   of the heading of the list stay where the width holds them, and the others go into **…** at its right. Cards:
   `changes.row-menu`, `changes.pane`, `ui.menu-icons`.
3. **A click shows the diff.** `changes.openOnClick` is `diff` by default in Gitardo Desktop; the key and its values
   keep their meaning, and the extension its default. Cards: `changes.open`, `settings.read`.
4. **Changes, the third section.** **Changes** between **Repositories** and **Branches**: its heading with the count
   of the changed files, folding as the others; in it the banners of an operation that stopped, the commit box,
   **Conflicts** and the list of the changed files; nothing under its heading while no repository is shown. The two
   lines between the three sections are dragged to share the height, and what is folded and the two heights are
   remembered for the workspace under keys added to those kept (`changesFolded`, `changesSection`). A file of the
   section opens in the middle of the window, its row marked: the Changes page beside a file draws the file alone
   while the side column shows **Changes** open, and its own column of the list otherwise. The pictures of the window
   are recorded again and looked at. Cards: `changes.section`, `repo.side-column`, `changes.beside-file`,
   `graph.resize-panes`.

Built: the four phases, in Gitardo Desktop; questions Q760 to Q776 are open, and the extension is unchanged.

Exit: with `a.txt` changed and the timeline shown, the side column lists **Repositories**, **Changes 1** and
**Branches**, each heading but the first under a line whatever is folded; a click on `a.txt` shows its diff where
the timeline was, the side column staying and the row marked; the row has its box and no button, and its menu, by
the right button or the menu key, stages it, discards it, stashes it and opens the file; folding **Changes**, or
dragging the line under it, is as it was left after the window opens again; `changes.section` and `changes.row-menu`
are Built in the desktop, and the snapshots of the window match in each theme.

## M25 — The graph of the extension loads without stopping

A fix, asked by the maintainer: the graph of the extension seems to load badly, or in blocks, perhaps from a build
left half made. It is not the build. On a clean one, minified, with the release build of the engine, on the repository
`extension/test/perf/big-repo.mjs` makes (56,123 commits), watched in a real VS Code on 2026-11-18 by
`extension/test/perf/graph-load.mjs`:

- **Refresh** with the list at its top: the first page is drawn after 0.3 s and the last row after 3.9 s, in three
  blocks (1,001 rows, then 30,001, then all), and the view draws nothing for 1.1 s and then for 1.9 s. The engine has
  read every commit after less than a second.
- **Refresh** with the list scrolled to row 28,061: 9.3 s to the last row, the view stopped for 1.1 s, 2.3 s and 4.1 s;
  the list is cut to its first page, thrown to row 992, the end of it, and left there. A commit made in a terminal
  does the same, in 10.3 s.
- On a repository of 1,250 commits, after **Refresh** the list has its first 1,000 and its last row says **Loading
  more commits…** for good: the 247 others come only when the list is scrolled to its end.
- The graph is 62 px wide, 48 px with the first page and 62 px again, the messages moving with it.

The causes, read in the code. Until milestone M15 the graph listed a page or a few; since it lists the whole history
(D54), what was written for a thousand rows runs over all of them at each page.

1. The lines between days, weeks and months (`graph.date-separators`) are worked out over every row each time the
   rows change, and `period` of `extension/src/webview/util.ts` writes the name of the month of each commit with
   `toLocaleDateString`, to keep one for each month: some 40 µs a row, 2.2 s for 56,000 rows, at each message with
   rows. The profile of the view gives it 4.9 s of the 6.3 s of a **Refresh**.
2. `Controller.load` of `extension/src/extension/controller.ts` lays out the pages that came, then waits for the
   newest entry of the journal before it gives the state to the views; `pageRead` drops the pages that come meanwhile
   when no state is shown yet, and when the read ends in that wait nothing tells the views after: they keep the rows of
   before the wait, and are told there are more.
3. A read of another history gives its first page as soon as it is read (`EngineGraph.commits`), and
   `Controller.post` sends it in place of the rows the views have; the list is as long as its rows, and the browser
   brings where it is scrolled to back within it. What was read before M15 at once, as many commits as were listed,
   now comes a page at a time.
4. The graph is as wide as the most lanes of the rows listed need (`graph.fit-width`): fewer with the first page.
5. Each message with the rows of a page carries the whole state and goes to every view of Gitardo, those that list no
   commits too, and a search of the whole history is asked of git again at each of them.

Built in seven phases, each one commit or more, each ending with its tests passing. The first five are the
extension's alone; the sixth brings the extension to what the desktop does, and the seventh the desktop to
[D72](../08-decisions.md#d72--the-list-of-commits-keeps-its-length-while-its-commits-are-read-again-proposed).

1. **The measure first.** `extension/test/perf/graph-load.mjs` opens a repository in the throwaway VS Code of
   `npm run test:ui` and records, every 50 ms, how many commits the list has, where it is scrolled to, what says that
   more are loading, how wide the graph is, each message with commits and each frame over 50 ms, while the graph opens,
   on **Refresh**, scrolled to the end, on **Refresh** from the middle and, with `--commit`, after a commit made
   outside; `--profile` says where the view spent a **Refresh**. `test/perf/vscode.mjs` is given the engine, which it
   no longer found ([Q788](../08-decisions.md#of-how-the-graph-of-the-extension-loads-from-q780)). The numbers of
   before go into [the extension's testing](../extension/06-testing.md). No card.
2. **The name of a month is written once.** How far back a commit is, is worked out for each row without its name; the
   name is written for the rows that start a time, a few tens in a long history. A test counts how many names are
   written for 50,000 commits over four years. Cards: `graph.date-separators` in the extension.
3. **The end of a read is told.** The rest of the state is read first, and the pages that came meanwhile are laid out
   under the rows right before the state is given to the views, with whether more follow. A test ends a read while the
   rest of the state is awaited, and finds every row in the state, with no more to come. Cards: `graph.load-more` in
   the extension.
4. **A page goes to who lists commits, and asks for no search.** The rows of a page are sent to the graph and to
   Gitardo Graph alone; what a search of the whole history found is asked again when the commits listed are others, not
   when more come under the same ones (Q787). Cards: `graph.load-more`, `search.history` in the extension.
5. **The list keeps its length, and the graph its width (D72).** While the commits of the same history are read again,
   the list is as long as it was until the read reaches it or ends, and the graph as wide; under the commits read so
   far the rows the list had before are drawn where they were, dimmed, until the read reaches them (Q782), and a list
   that asked for more commits while the last page came is no longer answered with another read of the repository. A
   test of what the list keeps takes the rows as they come after **Refresh**, and the measure of phase 1 finds the list
   at the row it was scrolled to. Cards: `graph.load-more`, `graph.fit-width` in the extension.
6. **The same commits change no row in the extension.** `EngineGraph` compares the pages of a read that starts again
   with the commits of the read before it, and keeps them, laid out as they are, while they are the same, as the
   desktop's `Stream` does: **Refresh**, a branch or a tag made and a fetch with nothing new send the views no row
   (Q781). Cards: `graph.read-commits`, `graph.load-more` in the extension. Tests in `test/graphPages.test.ts` read the
   same history again after a branch made, on **Refresh**, after a commit made on top, and with fewer commits.
7. **The desktop keeps the length of its list.** What D72 says, in the timeline of Gitardo Desktop, with a test of the
   window that scrolls the timeline far down, makes a commit, and finds it where it was at each page (Q780). Cards:
   `graph.load-more`, `graph.fit-width`. The store keeps the rows listed before and their layout aside
   (`Timeline::awaited`), and the timeline draws them under the rows read, dimmed.

Phases 1 to 5 are built, on 2026-11-18. With them, one run after the other on the same machine, a **Refresh** at the
top of the list has its last row drawn after 0.9 s where it took 4.6 s, with no frame of the view over 71 ms where one
took 1.9 s; from row 28,064, after 1.0 s where it took 4.0 s, and after a commit made outside 1.7 s where it took
4.6 s, with the list at row 28,064 where it was left at row 992; the graph is 62 px wide all along; and the repository
of 1,250 commits has every row after **Refresh**. The table is in
[the extension's testing](../extension/06-testing.md#large-repositories). Phases 6 and 7 are built after them, as
Q781 and Q780 recommend until the maintainer answers otherwise.

Exit: on the large repository, with the minified build and the release build of the engine, `graph-load.mjs` reports
that a **Refresh** has the last row drawn within 1.5 s, with no frame of the view longer than 250 ms; that after
**Refresh** from the middle of the list, and after a commit made outside, the list is at the row it was scrolled to;
and that the graph is as wide during the read as before it. On a repository of 1,250 commits, the last row stops
saying **Loading more commits…** by itself after **Refresh**. The tests of each phase pass, and the cards say what the
extension does.

## M26 — Search: files, folders and text across the worktree

A page that finds the files, the folders and the lines of text of the worktree shown, with a preview of the result
under the keyboard: what the extension Search Here does in VS Code, inside Gitardo Desktop
([D73](../08-decisions.md#d73--search-a-page-that-finds-files-folders-and-text-in-the-worktree-shown)). Asked by the
maintainer, who had no way to search across the files of a repository in the window. Built in five phases, each one
commit or more, each ending with its cards built and their tests passing. Desktop only: in VS Code, Search Here and
VS Code's own search do it, and the cards are *Host only* there.

1. **The names read and ranked.** The files of the worktree listed by git, tracked and untracked, the ignored ones
   too when asked, narrowed by the pathspecs that **Include**, **Exclude** and **Hidden** become; a name matched
   against what is typed, every term of it, its letters together or apart, and ranked. Cards:
   `file-search.read-names`, `file-search.pathspecs`, `file-search.rank-names`.
2. **The text searched.** `git grep` read as it prints, every match of a line marked, plain text or a regular
   expression, with match case and whole word, stopped at 5,000 lines; the rows of the list made from what was found,
   for each mode. Cards: `file-search.read-text`, `file-search.list-results`.
3. **The Search page.** **Search in files** in the bar of views, a page with the box, its switches, the modes, the scopes
   **Repository** and **Changes**, **Include** and **Exclude**, **.gitignore** and **Hidden**, and the results, found
   as it is typed, the search before stopped; `↑`, `↓` and `Enter`, which opens the File page at the line with the way
   back in its title row. Cards: `file-search.page`, `graph.view-bar`.
4. **The preview.** Beside the results, the file of the result under the keyboard, drawn as the File page draws it,
   the line of the result lit and every match marked; the content of a folder; a double click on a line opens the
   File page there. Cards: `file-search.preview`, `file-search.page`.
5. **From anywhere, and remembered.** `Ctrl+Shift+F`, **Search in files…** in Quick search, **Search here** in the
   context menu of a file or a folder of the Files page, which narrows the search to it; text selected in the File
   page or in the side of the Diff page in front that is edited becomes what is looked for; what is searched kept for each repository between runs.
   Cards: `file-search.open`, `file-search.remember`, `file-search.page`.

Exit: in the repository of Gitardo, `Ctrl+Shift+F` shows the page with the keyboard in its box; `usock` in **Files**
finds a file named `useSocket.ts`; a word typed in **Text** lists its lines under their files as git finds them, and
typing on stops the search before; `↓` shows each result in the preview with its line lit, and `Enter` opens the File
page at that line, **Search** before it in the title row; **Changes** searches the changed files alone; **Search
here** on a folder of the Files page searches under it; with **.gitignore** off a line of `node_modules` is found;
the page comes back as it was after the window is closed and opened; on the working tree of GIMP a word is found in
under half a second; every card of the milestone passes its tests, and the snapshots of the page match, empty, with
results in each mode and with a match lit in the preview, in the dark and in the light theme.

## M27 — Images, and SVG, drawn where their files are shown

An image in the File page, two versions of it side by side in the Diff page and in the page of a commit, an SVG as its
picture or as its text, in both apps
([D74](../08-decisions.md#d74--images-are-drawn-where-their-files-are-shown)). Asked by the maintainer, who met
*This file is not text.* and **Binary file.** where a picture was. Built in six phases, each one commit or more, each
ending with the cards it names built where it says and their tests passing: the window first, the extension last.

1. **Images read.** `file-view.read-image` tells an image by its first bytes or, for SVG, by its name; reads PNG with
   `tiny-skia` and draws SVG with `resvg` as the diagrams of Mermaid are drawn, nothing outside the file read; says why
   an image is not read; runs on a worker and keeps what it read for the bytes and the scale. The preview of Markdown
   draws through it, so that an SVG of the repository is drawn there; Q581 is answered in its card. Cards:
   `file-view.read-image` (PNG and SVG), `file-view.markdown-preview`.
2. **The File page.** An image drawn in place of *This file is not text.*: fit to the page, on its ground of squares,
   with its size, format and weight under it; the zoom with `Ctrl` and the wheel or the keys, moved by dragging, its
   pixels square above 100%; **Preview** and **Source** for an SVG, the picture drawn again from the text edited; a
   file changed on disk drawn again. Card: `file-view.image`.
3. **The Diff page.** The two versions side by side at one scale, **Empty** for a side with none, the line that says
   what changed, *Only the bytes of the file changed; the pixels are the same.*; **Image** and **Text** for an SVG; the
   buttons of the file kept, `F7` stopping once on an image in a page of several files. Card: `diff.image`.
4. **The page of a commit and a comparison.** The two versions in place of **Binary file.**, read when the diff is
   shown, a click that opens the Diff page; **Show the images** on an SVG. Card: `commit-page.images`.
5. **The other formats.** JPEG, GIF, WebP, BMP and ICO, read by one library weighed as D22 says, its first frame for
   an animation and its largest image for an ICO; every place of the phases before draws them with nothing more.
   Card: `file-view.read-image`, Built once this phase is done.
6. **The extension.** The files of the scheme `gitardo` given to VS Code as bytes by a `FileSystemProvider` that can
   only be read, in place of the `TextDocumentContentProvider`, the URIs as they are, so that an image at a revision,
   of the index or of a conflict opens in VS Code's preview of images, and its diff in its diff editor with two
   pictures; a text file opens as before. The page of a commit and the tab of a comparison draw the two versions as
   `<img>` of `data:` URIs from `read/diff.read-sides`, the policy of content of the webview allowing `data:` for
   images; a side over 10 MB is not sent. The README of each app and its CHANGELOG say what is new; a step of the
   end-to-end suite opens the diff of a PNG from the Gitardo Changes view and the page of a commit that changed one.
   Cards: `file-view.image`, `diff.image`, `commit-page.images`, Built in the extension.

Exit: in a repository with `media/logo.png`, `icons/check.svg` and a photo in JPEG, the Files page opens each as its
picture, the size under it; `Ctrl+=` zooms the PNG with its pixels square; **Source** of the SVG edits its text and
**Preview** draws the edit; changing the PNG in another program and opening its changes shows the two versions side
by side with what changed over them, and **Stage file** stages it; the page of the commit that added the photo shows
it beside **Empty**; a Markdown file that names the SVG draws it; an SVG that names an image of the disk reads no file;
in VS Code, the diff of the PNG opened from the Gitardo Changes view shows the two pictures and the page of that
commit shows its two versions; every card of the milestone is Built in both apps, or Host only in the extension, and
the snapshots of the File page and the Diff page with an image match in the dark and in the light theme.

## M28 — Gitardo, by its name

The window, the taskbar and a release say **Gitardo**, no longer *Gitardo Desktop*
([D75](../08-decisions.md#d75--gitardo-by-its-name-wayland-of-its-own-github-copilot-as-a-provider), Q1). One phase,
one commit:

1. **The name on screen and in a release.** `views::text::PROGRAM` is **Gitardo**, so the title of the window the
   taskbar shows is *\<repository\> — Gitardo*; the window of the widgets of `gitardo-demo` is *Gitardo widgets*; the
   description of `worktrees.openIn` on the settings page says *a new window of Gitardo*. The pipeline of a release
   names its archives `gitardo-<version>-<system>` and the part of the notes of a release about the window
   **Gitardo** (Q865). The README of the desktop is titled **Gitardo** and says, once, that it is the desktop app
   beside the extension. The folder of the settings and the names of the secrets keep their names (Q861); the
   documents keep *Gitardo Desktop* where they tell the two apps apart (Q860). Cards: `window.title-bar`,
   `worktree.open-in-new-window`, whose texts say the name.

Exit: the title of the window in the taskbar of Windows is *\<repository\> — Gitardo*; a test of `views` reads
**Gitardo** in the title; no text of the window says *Gitardo Desktop* (a search of `crates/views/src/text` and of
the texts of `features` finds none); the settings and the secrets of a window of before are read as they were; a
release made by the pipeline on a test tag holds three archives named `gitardo-<version>-…`.

## M29 — Wayland of its own

Gitardo on Wayland as much as on X11, with nothing it needs taken through XWayland
([D75](../08-decisions.md#d75--gitardo-by-its-name-wayland-of-its-own-github-copilot-as-a-provider), Q2). Built in
three phases, each one commit or more, each ending with its tests passing on Linux:

1. **The clipboard of Wayland.** Copy and paste speak to the compositor's clipboard on Wayland (the protocol
   `wlr-data-control` or `ext-data-control`, through the Wayland part of `arboard` or a library weighed as
   [D22](../08-decisions.md#d22--libraries-with-judgment) says), and to X11 as before when `winit` opened the window on
   X11; Gitardo started without XWayland copies and pastes with another Wayland program. `05-platform.md` says so in
   its row of the clipboard, and Q355 is answered in it.
2. **The menu of the window.** A right click on the caption opens the compositor's menu of the window: on Wayland
   `xdg_toplevel.show_window_menu`, on X11 the message `_GTK_SHOW_WINDOW_MENU` to the window manager, through `winit`
   if a version of it does so, otherwise in `crates/platform/src/linux/`; Q570 is answered. Card: `window.title-bar`,
   its line of the menu on Linux.
3. **Checked on Wayland.** The tests of `platform` that open real windows, use the clipboard or watch folders run in
   the pipeline on Linux under a compositor of Wayland started without a screen (Weston with its headless backend), as
   well as under X11; **Before a release** gains a pass by hand on GNOME and on KDE Plasma, both on Wayland: the
   title bar moved, maximized and resized, the menu of the window, a text typed with an input method, copy and paste
   with another program, the dialogs of files through the portal, a scale of 125% and of 200%, the terminal. The
   README of the desktop says that Linux runs on X11 and on Wayland.

Exit: on a Linux without XWayland (`WAYLAND_DISPLAY` set, `DISPLAY` not), Gitardo opens, is moved, maximized and
resized from its title bar, opens the compositor's menu of the window with a right click on the caption, copies a
hash that a terminal of the system pastes and pastes a text copied there; on X11 the same holds; the pipeline runs the
tests of `platform` under Weston and passes; every card of the milestone passes its tests.

## M30 — GitHub Copilot, an AI provider

`ai.provider` set to `copilot` asks GitHub Copilot, with the user's sign-in and subscription, what the other providers
are asked: the conflicts and the draft of a commit message
([D75](../08-decisions.md#d75--gitardo-by-its-name-wayland-of-its-own-github-copilot-as-a-provider), Q3). The
extension asks Copilot already, among the language models of VS Code, and changes nothing. Built in three phases, each
one commit or more, each ending with the cards it names built and their tests passing:

1. **Speaking to the CLI.** `engine::ai::CopilotModel` starts `copilot` in its server mode, once for the window, and
   speaks JSON-RPC to it over its standard input and output as the Copilot SDK does: the models of the subscription, a
   session for each question with its model and its effort, the answer streamed, every permission it asks refused,
   the session ended on cancel and the program when the window closes. A fake `copilot` in `testkit` answers the
   protocol for the tests. The parts of the protocol used are written down in the card, with the version of the CLI
   they were read from. Card: `ai.copilot`.
2. **Choosing it.** `ai.provider` takes `copilot` (**GitHub Copilot** on the settings page), and `ai.endpoint` is not
   shown then; `ai.choose-model` lists the models of Copilot; `ai.ask-conflict` and `commit.draft-message` ask
   through it. The refusals: no CLI on the `PATH`, with **How to install**; not signed in, with **Sign in**, which runs
   `copilot login` in a terminal of the bottom panel; Copilot's own refusal in its words. Cards: `ai.copilot`,
   `ai.choose-model`, `ai.ask-conflict`, `commit.draft-message`, and the row of `ai.provider` in
   `18-settings.md`.
3. **Said.** The README of the desktop, its CHANGELOG under **Unreleased**, the stored data (`ai.provider` gains a
   value, `docs/transition/05-stored-data.md`), and **Before a release** gains: resolve a conflict and draft a
   message with GitHub Copilot signed in. Card: `ai.copilot` Built.

Exit: with the GitHub Copilot CLI installed and signed in, choosing **GitHub Copilot** in the settings, then **Choose
the model that resolves conflicts**, lists the models of the subscription; **Resolve with AI** on a merge in conflict
proposes resolutions named after the model chosen; **Draft the commit message** writes a draft; a question cancelled
leaves the CLI running and the next one is answered; with the CLI signed out, **Sign in** opens a terminal running
`copilot login`, and once it ends the question is answered; without the CLI, the refusal offers **How to install**;
every card of the milestone passes its tests, against the fake CLI.

## M31 — Ask Gitardo how to do something

A question in the user's own words, in any language, about how to do something with Gitardo, answered by the AI
provider from the cards of the catalog, with the words of the controls as they are on screen and a button that does
what the answer names: a page in the window, the participant `@gitardo` of VS Code's Chat in the extension
([D76](../08-decisions.md#d76--ask-gitardo-how-to-do-something-answered-from-the-catalog)). Asked by the maintainer,
who wants a small window where the user asks the AI how to do something with Gitardo's interface. Built in five
phases, each one commit or more, each ending with the cards it names built where it says and their tests passing: the
window first, the extension last.

1. **The guide.** `cargo xtask guide` makes `crates/model/src/help/guide.txt` from the cards of `docs/features/`: for
   each app, the cards Built or Seen there, each with its title, its first paragraph, its entry points and what it
   refuses, or its **Extension** line, and its place; the names of the places of each app; the index of the guide.
   `cargo xtask check` fails when the file is not what the catalog makes. `model::help::guide` reads it once.
   Card: `help.guide`.
2. **The two requests and the answer.** The first request, which chooses at most 12 cards from the index, and its
   answer read; the second, with the cards chosen, the places and the conversation, whose answer is streamed; the
   answer read, `[[id]]` made the title of its card and the features it names listed; both asked of the AI provider
   with `ai.model` and `ai.effort`, cancelled together, nothing of the repository in them, tried against a recorded
   provider. Cards: `help.choose-cards`, `help.question`, `help.ask`, `help.read-answer`.
3. **The Ask Gitardo page.** The page, empty with its three examples, the conversation drawn as the preview of
   Markdown draws, the box that grows, **Ask** and **Stop**, *Looking for the features…*, **New conversation**, an
   error with **Try again**, the page with no provider and **Open the settings**; **Ask Gitardo…** in Quick search and
   the Help menu, and **Ask Gitardo: "…"** as the last entry of Quick search when what is typed finds no command;
   the activity of the status bar, **asking Gitardo**. Card: `help.page`.
4. **Show me.** Under an answer, at most 4 buttons: an operation Quick search runs, run as Quick search runs it; a
   page, opened; a feature done on a commit, a branch, a file or a stash, its place shown with what to choose there
   said in the status bar. Card: `help.show-me`.
5. **The extension.** `read/help.ask` in version 1.11 of the protocol, which asks both requests with
   `host/languageModel`, `purpose` `help`, of the model the Chat chose, with the guide of the extension; the
   participant `@gitardo` of VS Code's Chat (`chatParticipants` in `package.json`), which writes the answer as it
   comes and a button for each feature named that has a command, from a table the tests check against `package.json`;
   **Gitardo: Ask How To…**, which opens the Chat with `@gitardo ` typed, and warns when VS Code has no model.
   The README of each app and its CHANGELOG say what is new. Cards: `help.guide`, `help.choose-cards`,
   `help.question`, `help.ask`, `help.read-answer`, `help.page`, `help.show-me`, Built in the extension.

Exit: with an AI provider set, typing *how do I put my changes aside* in Quick search offers **Ask Gitardo: "how do I
put my changes aside"**, whose page answers with **Stash changes…** in the context menu of the draft and a button
**Stash Changes** that asks what Quick search's asks; *how do I squash my last three commits?*, asked in Italian, is
answered in Italian with the selection bar and **Squash commits**, and **Show where** shows the timeline; a question
about something Gitardo does not do is answered that it does not, with no control made up; **Stop** keeps what came; no
request recorded has the path, a branch or a commit of the repository; with no provider, the page says how to set one
up; in VS Code, `@gitardo how do I put my changes aside` in the Chat answers with a button that runs **Gitardo: Stash
Changes**; every card of the milestone is Built in both apps, and the snapshots of the page match in the dark and in the
light theme.

## M32 — Clone and create a repository, and the history of a shallow clone

The window clones a repository and creates one, where it could only open a folder already on disk, and a shallow clone
says where its history ends and fetches more of it
([D77](../08-decisions.md#d77--clone-and-create-a-repository-and-the-history-of-a-shallow-clone-proposed)). Asked by
the maintainer on 2026-11-19. Built in four phases, each one commit or more, each ending with its cards built and their
tests passing. Desktop first: cloning and creating are VS Code's own in the extension, and those cards are *Host only*
there; the shallow history is *Planned* in the extension, for a later milestone.

1. **Progress in the Activity tab.** The engine reads what git prints on its standard error while a step that asks
   for its progress runs, and the last line of it is the detail of the row of the operation in the **Activity** tab
   and in the tooltip of the activity indicator. Cards: `activity.describe-task`, `activity.what-is-running`,
   `activity.track`.
2. **Clone.** The name of the folder from the URL; the questions of the clone, the folder that holds it remembered as
   `cloneFolder` in `state.json`; `git clone --progress` with its options; the clone opened in a tab, or as the
   workspace. **Clone a repository…** on the Welcome page, in the menu of **+** of **Repositories**, on the timeline
   of a workspace with no repository and in Quick search. Cards: `repo.folder-from-url`, `repo.clone`, `welcome.page`,
   `repo.open-tab`, `repo.no-repository`.
3. **Create.** `git init`, with the first branch of `init.defaultBranch` or the one asked; a folder that is a
   repository refused, one inside a repository asked about; **Create a repository…** where **Clone a repository…**
   is, and **Create a repository here** on the timeline of a workspace with no repository. Cards: `repo.init`,
   `welcome.page`, `repo.no-repository`.
4. **A shallow clone.** Whether the repository is shallow and where its history ends, read with the state and read
   again when the file `shallow` changes; the line that goes on under the dot of those commits, **Not fetched** in the
   inspector, the last row of the timeline with **Load more history** and **Load the whole history**, and the two
   fetches behind them. Cards: `repo.read-shallow`, `repo.read-state`, `repo.watch-changes`, `graph.shallow-end`,
   `sync.deepen`, `sync.unshallow`.

Nothing is built yet.

Exit: on the Welcome page, **Clone a repository…** with the URL of a public repository, into a folder chosen, with
nothing else ticked than the submodules, shows the clone in the **Activity** tab with git's progress under it, then
opens it in a tab with its submodules initialized; a URL that does not exist says git's message and leaves no folder;
**Create a repository…** on an empty folder, with `init.defaultBranch` not set, asks the first branch, `main`, and
shows the new repository with its files untracked; a clone with **Only the last commits** and `2` lists two commits,
the line of the older going on under its dot and the last row saying **The history ends here: this is a shallow
clone.**; **Load more history** with `3` lists five, and **Load the whole history** lists them all and takes the row
away; the cards of the milestone are Built in the desktop, and the snapshots of the end of a shallow history match in
each theme.

## M33 — Stage the lines selected

The Diff page stages, unstages and reverts the lines selected, where it could only take a whole change
([D78](../08-decisions.md#d78--stage-unstage-and-revert-the-lines-selected-in-a-diff-proposed)). Asked by the
maintainer on 2026-11-19. Built in three phases, each one commit or more, each ending with its cards built and their
tests passing. Desktop first: the extension gets the same in its Flow Diff in a later milestone, where VS Code's diff
editor already has **Stage Selected Ranges**.

1. **The patch of the lines.** From the two texts and the rows selected, the patch of the lines taken: added lines not
   selected left out, removed lines not selected left in place, a hunk for each run, the counts of the headers and the
   starts made from what is kept, the line with no line break at the end, the line endings of the index, an untracked
   file created in the index. Tested as a table, and each patch applied by `git apply --cached --unidiff-zero` to a
   repository of the tests. Cards: `diff.lines-patch`.
2. **The selection.** Rows selected on either side, by dragging over the lines or their numbers, by `Shift` with a
   click or the arrows, and by the text selected in the side that is edited; drawn on both sides and in the gutter;
   cleared by `Escape`, a click elsewhere, or an edit that takes away a row of it. Cards: `diff.select-lines`,
   `diff.align`, `diff.edit`.
3. **Stage, unstage and revert.** The three items of the context menu of either side and their keys, each offered
   where its card says; the patch applied, or the edit made and saved for a revert; the page computed again. Cards:
   `diff.stage-lines`, `diff.unstage-lines`, `diff.revert-lines`, `diff.staged-changes`.

Nothing is built yet.

Exit: in a file with a change of five added lines, the second and third selected on the right and **Stage lines**
leave `git diff --cached` with those two lines, drawn as staged; selecting them again and `Alt+U` takes them out of
the index; the row of one line changed into another, selected on the left with `Shift+↓` from it, and **Revert lines**
gives the file its old line, which `Ctrl+Z` takes back; a selection across two changes stages what is selected of
both in one `git apply`, which the Command log shows with its patch; staging two lines of an untracked file adds it to
the index with those two lines; the cards of the milestone are Built in the desktop.

## M34 — The signature of a commit, and the options of the commit box

The inspector and the tab of a commit say whether it is signed and whether the signature holds; the commit box signs
or does not sign a commit, adds `Signed-off-by` and co-authors, and skips the hooks, for one commit
([D79](../08-decisions.md#d79--the-signature-of-a-commit-shown-and-the-options-of-the-commit-box-proposed)). Asked by
the maintainer on 2026-11-19. Built in three phases, each one commit or more, each ending with its cards built and
their tests passing. Desktop first: the extension gets the line of the signature and the options of the commit box of
**Gitardo Changes** in a later milestone.

1. **The signature, read and shown.** `%G?`, the signer, the key, its fingerprint and its trust, read for the commit
   selected once its details are drawn, the read before cancelled; the header of the object read when git says there
   is no signature; the line **Signature** in the inspector and in the tab of a commit. Tested with commits signed by
   an SSH key made for the tests, with and without allowed signers. Cards: `details.read-signature`,
   `details.signature`, `details.show`, `commit-page.view`.
2. **The options of a commit.** How git signs commits, read with the state; the menu beside **Commit** and **Amend**
   with **Sign this commit**, **Add Signed-off-by** and **Skip the hooks**; the arguments they add to `git commit`;
   the line under the box; the new setting `commit.signOff`, in the list of the settings and on the settings page;
   the boxes back to where they start after each commit. Cards: `commit.read-signing`, `commit.options`,
   `commit.option-arguments`, `commit.create`, `commit.amend`, `settings.edit`.
3. **Co-authors.** The authors of the history, read when the picker opens; the picker of many, with an author typed;
   a `--trailer` for each co-author chosen, and none for one the message names already. Cards:
   `commit.read-authors`, `commit.co-authors`, `commit.option-arguments`.

Nothing is built yet.

Exit: in a repository signing with an SSH key whose allowed signers are set, selecting a commit signed shows
**Signature** `Good` with the signer, and one not signed shows no such line; without the allowed signers, the same
commit says **Signed, not checked**; with `commit.gpgSign` on, unticking **Sign this commit** makes a commit without a
signature, and the next commit is signed again; **Add Signed-off-by** and two co-authors chosen in the picker give a
commit whose message ends with the three lines, once each; **Skip the hooks** commits past a `pre-commit` hook that
fails, and the next commit runs it; the cards of the milestone are Built in the desktop, and the snapshots of the
menu and of the line of a signature match in each theme.

## M35 — Upstreams, tags on remotes and new roots

The upstream of a local branch set, changed and taken away; which tags a remote has, the ones it lacks marked, pushed
in one go, and a tag deleted from a remote as well as here; a branch started with no history
([D80](../08-decisions.md#d80--upstreams-set-and-taken-away-the-tags-of-a-remote-checked-branches-with-no-history-proposed)).
Asked by the maintainer among the Git operations Gitardo lacks: today an upstream comes only from a first push, a tag
is deleted only here and pushed one at a time, and nothing starts a branch with no parent. Built in five phases, each
one commit or more, each ending with its cards built and their tests passing; the first four in Gitardo Desktop, the
last in the extension.

1. **Upstreams.** **Set upstream…**, **Change upstream…** and **Stop tracking `<upstream>`** in the context menu of a
   local branch and in Quick search; the pick of the remote branches, those of the branch's name first, the current
   upstream marked; `git branch --set-upstream-to` and `git branch --unset-upstream`, the remote branch named in full
   when it has a namesake. Cards: `branch.set-upstream`, `branch.unset-upstream`.
2. **What a remote has of the tags.** `git ls-remote --tags --refs`, read when asked and never with the reads of the
   repository; the tags here compared with it object by object; **Check tags on `<remote>`** in the **…** of the
   **Tags** list, and the marks of the tags it lacks in the branch rail and on the labels, kept while the window is
   open and updated by Gitardo's own operations on tags. Cards: `tag.read-remote-tags`, `tag.compare-remote`,
   `tag.remote-marks`, `refs.tree`.
3. **Tags pushed and deleted on a remote.** **Push tags…**, the pick of the tags the remote lacks, all ticked,
   `git push <remote> --tags` or each tag named; **Delete from `<remote>`** and **Delete here and from `<remote>`**
   beside **Delete**, `git push <remote> --delete refs/tags/<name>`, destructive. Cards: `tag.push-all`,
   `tag.delete-remote`.
4. **A branch with no history.** **Create branch with no history…** in Quick search and the **…** menu of the header,
   with the question of what the working tree keeps, `git switch --orphan` or `git checkout --orphan`; Undo from the
   journal; the timeline and the header while the branch has no commit. Cards: `branch.create-orphan`, `undo.journal`,
   `graph.where`.
5. **In the extension.** The entries in the menus of a branch and of a tag of the graph, the references and the
   Branches view, **Check tags on `<remote>`** and **Push tags…** in the **…** of the Tags list, the commands of the
   Command Palette, and the marks of the tags drawn in `RefsTree` and on the labels; each card *Built* in the
   extension with its `Extension:` lines tested. Cards: every card above.

Nothing is built yet.

Exit: in a repository whose `feature` follows nothing and whose `origin` has `feature` and `release`, **Set upstream…**
on `feature` lists `origin/feature` first, and once picked the branch rail says how far `feature` is from it;
**Change upstream…** to `origin/release` makes **Pull** pull from it, and **Stop tracking origin/release** leaves
`feature` with no upstream and `origin/release` listed; with `v1` pushed and `v2` made here, **Check tags on origin**
marks `v2` alone, **Push tags…** pushes it and the mark goes; **Delete here and from origin** on `v1` asks with a
warning and `v1` is gone from both; **Create branch with no history…** named `site`, with **Start with no files**,
leaves only the untracked files in the working tree, its first commit has no parent, and Undo before that commit goes
back to `main`; every card of the milestone passes its tests, in the desktop and, after phase 5, in the extension.

## M36 — Stacked branches and what a rewrite changed

The other local branches on the commits a rewrite makes again move with it, the plan says which, and one Undo takes
them all back; and what a rebase, an amend or a force push changed is shown commit by commit, as `git range-diff`
pairs the commits
([D81](../08-decisions.md#d81--the-branches-of-a-stack-move-with-a-rewrite-and-what-a-rewrite-changed-is-shown-commit-by-commit-proposed)).
Asked by the maintainer: rebasing the top of a stack of branches leaves the branches under it on the old commits, and
nothing said what a rewrite changed. Built in six phases, each one commit or more, each ending with its cards built and
their tests passing; the first five in Gitardo Desktop, the last in the extension.

1. **The version of git, and the branches of a stack.** `git --version` read once for each engine; the local branches
   on the commits a rebase makes again, with the worktree that has each, and git's own `rebase.updateRefs`. Cards:
   `repo.read-git-version`, `rebase.read-stacked-branches`.
2. **The branches move with the rewrite.** `--update-refs`, or `--no-update-refs` when git's setting says otherwise, on
   the rebases that let git write their todo list; `update-ref` lines in the todo lists Gitardo writes, after the
   `exec` of a reword; none for a branch of another worktree, nor on a git older than 2.38, each said in the note of the
   plan, which then asks first; what is said after names the branches that moved; the branches of each commit shown on
   the interactive rebase page; Gitardo's setting `rebase.updateRefs`, added to the list of the settings in the section
   **Pull and commits**. Cards: `rebase.update-refs`, `rebase.build-todo-list`, `rebase.onto`, `rebase.branch-onto`,
   `rebase.interactive`, `rebase.todo-page`, `fixup.commit`, `settings.read`.
3. **One Undo for the whole stack.** The journal entry of a rewrite keeps each branch it moved; Undo resets the branch
   rewritten and moves the others back in one `git update-ref --stdin`, leaving out and naming a branch moved since.
   Cards: `undo.moved-branches`, `undo.last`, `undo.journal`.
4. **What a rewrite changed, read.** First the code rules: R9.3 names `git range-diff` as the one read that parses
   what git prints for people (Q1139). Then the last rewrite of a branch, or the last force push of a remote branch,
   from its reflog; the two lines of commits for each case; `git range-diff` read into pairs with their diffs of diffs,
   tested on recorded output. Cards: `range-diff.read-last-rewrite`, `range-diff.ranges`, `range-diff.read-pairs`.
5. **The page, and where it is offered.** The page of the pairs, a changed one opening on its diff of diffs; **See
   what changed** after a rewrite and after a fetch that found an upstream force pushed, **What its last rewrite
   changed** on a branch, **Compare with `<upstream>` commit by commit** on a branch that diverged from its upstream,
   and **Compare first** in the warning of a force push. Cards: `range-diff.page`, `range-diff.compare`,
   `sync.push-force-with-lease`, `sync.push-force`, `preview.confirm`.
6. **In the extension.** The plans already come from the engine: its end-to-end suite rebases a stack and undoes it;
   the setting `gitardo.rebase.updateRefs`; the tab of what a rewrite changed, beside the comparison, with the entries
   of the menus, **Compare First** in the modal dialog of a force push and **See What Changed** on the messages after an
   operation. Cards: every card above.

Nothing is built yet.

Exit: in a repository with `p1`, `p2` on it and `p3` on `p2`, all above an older `main`, **Rebase p3 onto main**
runs `git rebase --update-refs main`, and `p1` and `p2` are on the new commits after, what is said naming them; with
`p2` checked out in another worktree, the plan asks first and says that `p2` stays, and that worktree's files are as
they were; editing the message of the commit `p1` is on moves `p1` to the reworded commit; one Undo puts the three
branches back; **See what changed** after the rebase opens the page with each commit paired, and after a commit was
reworded and one dropped, the reworded one opens on its diff of diffs; before **Push (force with lease)** of `p3`,
**Compare first** opens the page of `origin/p3` against `p3` and pushes nothing; every card of the milestone passes its
tests, in the desktop and, after phase 6, in the extension.

## M37 — Pull requests: check out, and hosting services beyond GitHub

A pull request checked out in a local branch, one from a fork included; and the pull requests of GitLab, Bitbucket,
Azure DevOps, Gitea and Forgejo opened, created and compared on the web as those of GitHub are, those of GitLab read
onto the labels
([D82](../08-decisions.md#d82--pull-requests-on-every-hosting-service-and-checked-out-proposed)). Asked by the
maintainer: `github.detect-remote` gives nothing for any other service, and a pull request could only be opened in the
browser. The cards are a new area, `hosting.`, beside `github.`, whose cards keep their identifiers. Built in five
phases, each one commit or more, each ending with its cards built and their tests passing; the first four in Gitardo
Desktop, the last in the extension.

1. **Which service, and its pages.** The service of the main remote from its URL, with the setting
   `hosting.selfHosted` for a GitLab, a Gitea or a Forgejo of one's own, added to the list of the settings; the
   addresses of a pull request, of the page that creates one and of a comparison on each service, tested on a table of
   URLs. Cards: `hosting.detect-remote`, `hosting.web-links`, `settings.read`.
2. **Opened, created and compared.** **Create merge request…**, **Create pull request…** and **Open merge request
   !`<n>`** in the menus of a branch; the offer after a first push; **Open on `<service>`** in the comparison page.
   Cards: `hosting.open-pull-request`, `hosting.offer-after-push`, `compare.open-on-service`.
3. **The merge requests of GitLab.** The token kept for the host name or given by `git credential fill`; **Load merge
   requests from GitLab**; one GraphQL request for the open merge requests and their pipelines, read again as the pull
   requests of GitHub are, and their numbers on the labels. Cards: `hosting.token`, `hosting.load-merge-requests`,
   `hosting.read-merge-requests`, `graph.pull-request-badge`.
4. **A pull request checked out.** The open pull requests of GitHub and GitLab, those of forks included, to pick from or
   a number typed; one from a branch of the repository checked out as its remote branch; one from a fork fetched by
   the reference of the service into a local branch whose name is asked, then checked out; Undo by the reflog. Cards:
   `hosting.read-pull-request-list`, `hosting.checkout-pull-request`.
5. **In the extension.** The service and the addresses from the engine in place of `parseGitHubRemote` and
   `compareUrl` for the services other than GitHub; the token of GitLab asked in an input box and kept in the secret
   storage of the extension; **Gitardo: Check Out Pull Request…**, the entries of the menus and of the Compare tab; the
   numbers of merge requests on the labels of the graph. Cards: every card above.

Nothing is built yet.

Exit: in a clone of a project on gitlab.com, **Create merge request…** on a pushed branch opens GitLab's page with the
branch filled in, and the first push of a new branch offers it; **Load merge requests from GitLab**, given a token,
shows `!12` on the label of the branch of an open merge request, in the color of its pipeline, and the token is asked
no more after the window opens again; **Open on GitLab** in the comparison of two branches opens GitLab's comparison of
them; in a clone of a project on github.com, **Check out a pull request…** lists the open ones, those of forks with
their author, and picking one from a fork makes `<owner>/<branch>` at its commits and checks it out, a second time
bringing it up to the fork's new commits; with a remote on bitbucket.org, **Create pull request…** opens Bitbucket's
page and nothing offers a checkout; every address of `hosting.web-links` was opened by hand on its service; every card
of the milestone passes its tests, in the desktop and, after phase 5, in the extension.

## M38 — What git leaves alone

Telling git to leave something alone, from the window: ignoring a file or a folder by a pattern written in the
`.gitignore` or the exclude file chosen, stopping tracking a file while it stays on disk, hiding the changes of a
tracked file from git and listing such files so that they are not forgotten, and locking a worktree with a reason
([D83](../08-decisions.md#d83--ignoring-files-files-git-does-not-look-at-and-locked-worktrees-from-the-window-proposed)).
Asked by the maintainer, with M39 and M40, among the Git features Gitardo did not have. Built in four phases, each one
commit or more, each ending with its cards built and their tests passing; the first three are Gitardo Desktop's, and the
fourth brings the milestone to the extension.

1. **Ignore and stop tracking.** **Ignore…** in the menu of a file of the changes and of a file or a folder of the
   Files page, with the patterns and the places offered, the line written as one `write` step, and git asked whether
   a path is ignored already; on a tracked file, the offer to stop tracking it, kept on disk, with its warning. Cards:
   `changes.ignore-patterns`, `changes.ignore`, `changes.stop-tracking`, `changes.row-menu`.
2. **Files hidden from git.** The files marked skip-worktree or assume unchanged read from the index, the sparse
   checkout's left out; **Hide its changes from git…** and **Show its changes to git again**; the group **Hidden from
   git** under the changes, in the Changes page and in the section, and the mark of the Files page. Cards:
   `changes.read-skipped`, `changes.skip`, `changes.unskip`, `changes.skipped-list`, `changes.section`,
   `changes.pane`.
3. **Locked worktrees.** The lock and its reason read from `git worktree list`, the folder of a locked worktree looked
   for on disk; **Lock…** and **Unlock**; the lock in the Worktrees list and in **Repositories**; **Unlock and
   remove**, **Unlock and rename**, and **Prune** leaving a locked worktree. Cards: `worktree.lock`,
   `worktree.unlock`, `worktree.locked`, `worktree.read-list`, `worktree.section`, `worktree.remove`,
   `worktree.rename`, `worktree.prune`.
4. **In the extension.** The entries in the **Gitardo Changes** view, in the context menu of VS Code's Explorer and in
   the Worktrees section, the group **Hidden from git** at the end of the **Gitardo Changes** view, and the lock on the
   rows of the worktrees, all run by the engine. Cards: the cards of the first three phases, in the extension.

Nothing is built yet.

Exit: in a repository with `debug.log` untracked, **Ignore…** with **All `.log` files** writes `*.log` at the end of
the `.gitignore`, `debug.log` leaves the changes and the `.gitignore` comes in; **Ignore…** on the tracked
`config/local.json` with **Stop tracking it** leaves the file on disk and its deletion staged; **Hide its changes from
git…** on a changed tracked file takes it out of the changes and lists it under **Hidden from git 1**, where a click
shows its diff and **Show its changes to git again** brings it back; a worktree locked with a reason shows its lock and
its reason, **Remove…** on it says the reason first, and **Prune** leaves it once its folder is gone; the same entries
do the same in the extension; every card of the milestone passes its tests, and the snapshots of the group and of a
locked worktree match in each theme.

## M39 — Large files and remembered resolutions

Files stored with Git LFS shown by their content, or as not downloaded with **Download**, never as the text of their
pointer; storing a pattern with Git LFS; the **Large files** page with the files and the locks; and git's rerere,
turned on from the window, with the files git resolved as it did before said where conflicts are shown, what it wrote
shown, and a wrong resolution forgotten
([D84](../08-decisions.md#d84--large-files-stored-with-git-lfs-and-resolutions-git-remembers-shown-where-the-files-are-proposed)).
Asked by the maintainer with M38. Built in six phases, each one commit or more, each ending with its cards built and
their tests passing, in Gitardo Desktop alone: the extension follows in a later milestone, and every card is *Planned*
there. The tests of Git LFS need the program, which the runners of the pipeline have (Q1198).

1. **Git LFS read.** Whether it is installed and set up, the patterns from the `.gitattributes` git tracks, the files
   with their sizes and whether they are downloaded, a pointer told apart from a file, and the content read from the
   store of the repository without the server. Cards: `lfs.read-status`, `lfs.read-files`, `lfs.pointer`,
   `lfs.read-content`.
2. **Pointers in the pages, and Download.** The File page, the Diff page and the page of a commit show the content of
   a file stored with Git LFS, or that it is not downloaded, with its size and **Download**, which pulls it for the
   commit checked out and fetches it for another. Cards: `lfs.show-in-pages`, `lfs.download`, `diff.page`,
   `file-view.page`, `commit-page.view`.
3. **The Large files page.** From the menu of the row of the repository and from Quick search: Git LFS and whether it
   is set up, the patterns, the files, **Download all**; **Set up**, **Store with Git LFS…**, **Store them with Git LFS
   in the next commit**, **Stop storing with Git LFS**. Cards: `lfs.page`, `lfs.set-up`, `lfs.track`,
   `lfs.store-committed`, `lfs.untrack`, `repo.tabs`.
4. **Locks.** The locks read when the page opens, a server without them said so; **Lock** and **Unlock**, someone
   else's after a warning. Cards: `lfs.read-locks`, `lfs.lock`, `lfs.unlock`.
5. **Turning rerere on.** Whether git remembers resolutions, read with the repository; **Remember how conflicts are
   resolved…** in the menu of the row of the repository, ticked while it is on, asking whether git stages what it
   resolves. Cards: `rerere.read-state`, `rerere.turn-on`, `rerere.turn-off`, `repo.tabs`.
6. **What git resolved as before.** The files git resolved, read while an operation is stopped; the words in the
   banner, the message of the stop and the rows of the changes; **Show what git wrote**, **Forget this
   resolution…**; the card of such a file in the review. Cards: `rerere.read-resolved`, `rerere.read-marked`,
   `rerere.resolved-as-before`, `rerere.forget`, `review.recorded`, `review.file-list`, `conflicts.banner`,
   `conflicts.stopped`, `changes.row-menu`.

Nothing is built yet.

Exit: in a clone of a repository that stores `*.psd` with Git LFS, made without downloading, the File page of a `.psd`
says it is not downloaded, with its size, and **Download** shows its content after; the page of a commit that changed
one says **Stored with Git LFS** with the two sizes, and no line of a pointer; **Store with Git LFS…** on a `.bin` adds
its pattern to `.gitattributes`; the **Large files** page lists the files, which are downloaded and which not, and the
locks of a server that has them. With **Remember how conflicts are resolved** ticked, a conflict resolved, committed,
and met again by the same merge is said **Resolved as before**, **Show what git wrote** shows the markers beside what
git wrote, and **Forget this resolution…** marks the conflict again; every card of the milestone passes its tests, and
the snapshots of the page and of a file not downloaded match in each theme.

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
