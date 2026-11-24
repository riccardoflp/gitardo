# Graph

The timeline: the commits of every branch, remote branch and tag with their lines and labels, read a page at a time
while scrolling; how the lines are laid out; what stands out under the pointer and around the selected commit; the
merges folded into one row; the draft of the uncommitted changes and the stashes among the commits; the header above
them; and the branch rail beside them, with its lists, its own filter and its filter box, the recent and favorite branches
and the way to a reference's commit. The search is in [03-search-and-filter.md](03-search-and-filter.md) (`search.`), and so are the
branches the timeline is narrowed to (`filter.`); the selected commit in the inspector is `details.`, several commits
selected together `selection.`, drag and drop `drag.`, fetch, pull and push `sync.`, and what is done to a stash
`stash.`.

### `graph.read-commits` — Read the commits of the graph

The commits the timeline lists, newest first, with their parents, a page at a time as git prints them: one read goes
on to the end of the history, or to 100,000 commits (D54).

- **Kind**: Read
- **Function**: `git::read::graph::read_commits(git, request: &CommitsRequest, page: &mut dyn FnMut(CommitPage)) ->
  Result<ReadEnd, GitError>`, where `ReadEnd` says whether the history has more than the read listed.
- **Reads**:
  - `git log -z --format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%s --max-count=<100,000 + 1> --date-order <selection>`,
    where `<selection>` is the arguments that say which history is listed (`filter.log-arguments`), followed by those
    of a search shown as a list (`search.read-matches`) or of the history of a file, a folder or some lines
    (`file-history.read-names`). Its output is taken as git prints it: each time a page of `graph.pageSize` commits is
    read, it is given to `page`, and the last commits when git ends. One commit more than 100,000 says whether there
    are more.
  - A read from a commit already loaded (`graph.load-more`, past 100,000 commits) adds `--skip=<commits loaded - 1>`
    and goes on the same way; it starts with that commit, or the history changed under it and is read again whole.
  - Cancelled, the read kills its git and gives no more pages.
  - With the setting `graph.firstParent`, `--first-parent` before the selection, and each commit keeps only its first
    parent.
  - With `graph.showStashes`, stashes in the repository and nothing narrowing the history but the branches shown:
    `git log --no-walk=unsorted -z --format=<the same> <the hash of each stash> --`, for the stashes to be placed among
    the commits by `graph.place-stashes`, read before the first page so that each page comes with its stashes.
    `refs/stash` itself is never part of the selection: the commits of a stash would clutter the graph.
  - What is checked out, the references and the stashes that say which history is listed come from the last
    `repo.read-state` of the repository when there is one, and are not read again; a read of the graph with no state
    read before reads them itself.
- **Settings**: `graph.pageSize` (1,000 by default; a number under 100 counts as 100, over 10,000 as 10,000, and what
  is not a number as 1,000); `graph.firstParent`; `graph.showStashes`.
- **Uses**: `filter.log-arguments`, `graph.place-stashes`, `graph.lanes`.
- **Extension**: `read/graph.read-commits` of Gitardo's engine with `stream` (version 1.8 of the protocol), each page
  in a notification `read/graph.rows` as it is read; `EngineGraph` of `src/engine/graph.ts` keeps one read for each
  repository, adds its pages as they come, and stops it when another history is asked for. The same history read
  again, after a commit, a fetch, a branch or a tag made, on **Refresh** (`readAgain`) or after a change of the
  settings, goes over the commits listed: while its pages are the same commits, they stay, laid out as they are, and
  the views are sent none of them; from the first that is not, the commits are cut there and those read follow, laid
  out again; a read that ends with fewer leaves as many (milestone M25, Q781). Tested by `test/graphPages.test.ts`,
  "keeps every row, and gives the views none, when a read after a branch was made finds the same commits", "reads
  every commit again on Refresh, and keeps the rows of those that are the same", "cuts the rows where a read finds
  other commits, as after a commit made on top, and lays them out as a first read would" and "takes out the commits
  the history no longer has when a read of it ends with fewer".

Acceptance:

- [ ] In a repository of 2,500 commits, one read gives three pages, of 1,000, 1,000 and 500 commits, the first before
      git has printed the last commit, and says there are no more; it runs one git for the commits.
- [ ] In a history of 100,050 commits, the read stops at 100,000 and says there are more; the read from the last of
      them gives the 50 left.
- [ ] A read cancelled after its first page gives no other page, and its git is no longer running.
- [ ] With `graph.pageSize` set to 50, a page is 100 commits; set to `"abc"`, 1,000.
- [ ] The stashes are placed among the commits of each page as they would be among all the commits read at once.
- [ ] A merge of three parents is read with its three parents, and with `graph.firstParent` with its first only.
- [ ] Commit messages with a tab, a line break in the body, or non-ASCII characters are read whole: the subject is the
      first line.
- [ ] With `graph.showStashes` off, no stash commit is read.
- [ ] Asked right after `repo.read-state`, a read runs `git log` and nothing else: no `rev-parse`, `symbolic-ref` or
      `stash list`.

### `graph.read-position` — Where a commit is in the history listed

How many commits come before a commit in the history the timeline lists, to know how far it must be loaded for the
commit to be there; or that it is not in it.

- **Kind**: Read
- **Function**: `git::read::graph::read_position(git, request: &PositionRequest) -> Result<Option<usize>, GitError>`
- **Reads**: `git log --format=%H <selection>`, with the same selection, filter and `--first-parent` as
  `graph.read-commits`; the position is the line of the full hash, none when it is not there.
- **Uses**: by `graph.show-in-graph`.

Acceptance:

- [ ] The 2,345th commit of the history listed is at position 2,344.
- [ ] A commit on a branch the timeline is not narrowed to has no position until the branches shown are all of them.
- [ ] A commit no branch or tag reaches has no position.

### `graph.view` — The timeline

The commits of every branch, remote branch and tag with their lines and labels, the branch rail beside them and the
inspector of the selected commit: what the window shows while no page is open, and the first step of every trail.

- **Kind**: View
- **Function**: `views::graph::view(state: &GraphState, ui) -> Vec<Intent>`
- **Entry points**: the window, while no page is open; **Timeline**, the first step of the trail of a page; **Open
  Timeline** in Quick search shows it.
- **Uses**: `graph.toolbar`, `refs.tree`, `graph.lanes`, `graph.fit-width`, `graph.labels`, `graph.label-overflow`,
  `graph.columns`, `graph.compact-rows`, `graph.load-more`, `graph.uncommitted-row`, `graph.stashes`,
  `graph.hover-history`, `graph.trace-selected`, `graph.fold-merge`, `graph.sticky-branch-names`, `graph.scroll-marks`,
  `graph.keyboard`, `graph.resize-panes`, `filter.bar`, `details.show`, `selection.select`, `drag.drop`,
  `commit-page.open`, `branch.open`.

- From left to right: the branch rail (`refs.tree`), the commits, the inspector of the selected commit (`details.show`).
  When the window is too narrow to have them side by side, the side column is put away, its button at the top of the
  timeline opening it over the commits (`repo.side-column`), and the inspector opens over the commits (Q501). There, as
  in any width, the labels that do not fit are counted in `+N` (`graph.label-overflow`, Q644).
- Above the commits, in this order: the banner of a worktree (`worktree.`), of an operation stopped half way
  (`conflicts.`, `rebase.`), of a bisect (`bisect.`), the bar of what the timeline is narrowed to (`filter.bar`) and
  the bar of several commits selected (`selection.select`). The draft of the uncommitted changes is the first card of
  the commits (`graph.uncommitted-row`).
- Each row has its cell of the graph (`graph.lanes`): a dot for a commit, in the color of its line. Then two lines: its
  message, right after the cell of the graph; under it its labels (`graph.labels`) first, then the columns
  `graph.columns` asks for. The labels take the room the author and the date leave them, and those that do not fit are
  one label `+N`, whose list opens under the pointer (`graph.label-overflow`). The labels are outlined in the color of
  the commit's line; the branch checked out is the one filled in. With **Compact rows** on, each row is one line, as
  tall as a row of a list: the labels, the message, then the columns at the right end (`graph.compact-rows`).
- A click selects a commit and shows it in the inspector; `Ctrl`+click (`Cmd` on macOS) and `Shift`+click select several
  (`selection.select`). A double click, or `Enter`, opens it as a page (`commit-page.open`). A double click on the
  label of a branch checks it out instead, or does what `branch.open` does for it, as in the branch rail. A
  right click opens the menu of the commit, of the label under the pointer, of the stash or of the uncommitted changes.
- While no commit is listed it says **No commits yet** in a repository with none, **No commits found** while a filter
  or a search narrows the timeline, and **No commits to show** otherwise. While the commits of a new filter, of the
  branches just chosen or of the first page are read, a bar runs along the top of the list, the commits listed before
  stay, dimmed, until the new ones are there, and with none listed it says **Loading commits…**.
- Only the rows in sight are drawn, and a few more above and under them.

Acceptance:

- [ ] A repository with `main`, `feature` and a tag `v1.0` lists the commits of all three, the newest first, with a
      label at each tip.
- [ ] The label of the branch checked out is filled; the others are outlined in the color of their commit's line.
- [ ] Narrowing the window under the width of the side column, the commits and the inspector puts the side column
      away and opens the inspector over the commits (Q501).
- [ ] A double click on a commit opens its page; on the label of a branch that is not checked out, checks it out.
- [ ] In a new repository with no commits and no changes, the list says **No commits yet**.
- [ ] Scrolling a list of 50,000 commits draws only the rows in sight.

### `graph.toolbar` — The header

Where one is and what applies to the whole repository, in a bar across the top of the window, always shown
([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).

- **Kind**: View
- **Function**: `views::graph::toolbar_view(state: &ToolbarState, ui) -> Vec<Intent>`
- **Entry points**: the top of the window.
- **Uses**: `graph.where`, `repo.tabs`, `repo.group-tabs`, `sync.sync`, `sync.fetch`, `sync.pull`, `sync.pull-with`,
  `sync.push`, `branch.create`, `stash.push`, `undo.last`, `graph.search-button`, `repo.schedule-reads`,
  `graph.view-menu`, `graph.more-menu`.

The header is not an island: it is a bar on the ground of the window, as high as a button and its margins, across the
whole width on the right of the bar of views, above the side column, the timeline, the inspector and the pages, and it
is there whatever is shown under it. From left to right: where one is, as one line, `gitardo › login ›
feature/login-page`: the repository shown, by its name (`repo.name-repositories`), which opens the list of the
repositories open to show another (`repo.tabs`); the worktree or the submodule shown, with its icon, by the name of its
folder or its path, only when it is not the repository's own folder, which lists the repository, its worktrees and its
submodules to show another (`repo.group-tabs`); and the branch checked out, or `detached at 3f2a1c9`, or `no commits yet`, with the folder in its
tooltip and without how far it is from its upstream, which **Sync** alone says. Each part is cut at its start with `…`
before the others when the room is short, the branch last; **Search**, which opens Quick search (`graph.search-button`);
**Sync** with the number of commits to pull and to push, `↓3 ↑2`, which a click runs: a fetch, a pull, then a push of
what is ahead (`sync.sync`), and its menu: **Fetch**, **Pull** and its other ways, **Push** and its other ways; the
menu of the view (`graph.view-menu`), with every switch of the timeline; and **…** (`graph.more-menu`), with the
actions alone, which has **Refresh**, **Branch**, which creates a branch at `HEAD`, **Create branch with no history…**
(`branch.create-orphan`), **Stash**, which stashes
all changes, and **Undo**, none of which is a button of the header. The button of the branches shown (`filter.picker`)
and the search (`search.box`) are at the top of the timeline; what is running is in the status bar, and **Activity** in
the bar of views ([D21](../08-decisions.md#d21--islands-a-bar-of-views-and-a-status-bar)). Each entry says in its
tooltip what it runs: **Sync: git fetch, git pull, git push**, **Fetch: git fetch --all --prune**, **Push: git push**,
**Stash: git stash push --include-untracked**. When the window is narrow the header keeps its icons and gives up its
words. While an operation runs, the entries that change something are disabled. **Branch**, **Create branch with no
history…** and **Undo** are disabled before the first commit, and **Stash** also while there is nothing to stash.

Acceptance:

- [ ] With `main` two ahead of its upstream and three behind, **Sync** shows `↓3 ↑2`.
- [ ] Detached at a commit, the header names `detached at <short hash>`.
- [ ] While a fetch runs, **Sync** itself, and **Pull** and **Push** in its menu, cannot be clicked.
- [ ] Narrowing the window hides the words of the header and keeps its icons; **Refresh** stays in **…**.
- [ ] The header has no box of text: the branch filter and the search box are at the top of the timeline.

### `graph.where` — Where one is, in the header

The repository, the worktree and the branch shown, at the start of the header, which is a bar across the top of the
window, always shown ([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).

- **Kind**: View
- **Function**: `views::graph::where_view(state: &WhereState, ui) -> Vec<Intent>`
- **Entry points**: the start of the header (`graph.toolbar`), above whatever is shown.
- **Uses**: `repo.name-repositories`, `repo.tabs`, `repo.group-tabs`.

As `graph.toolbar` says: the repository, the worktree or the submodule when it is not the repository's own folder, and
the branch checked out, `gitardo › login › feature/login-page`, or `site (no commits yet)` while the branch has no
commit (`branch.create-orphan`), the repository opening the list of the repositories
open and the worktree the list of the repository, its worktrees and its submodules. The header that holds them is a bar on the ground of the window, across its
whole width on the right of the bar of views, above the side column and the pages.

Acceptance:

- [ ] The header goes across the whole window on the right of the bar of views, above the side column, and has no
      border nor background of an island.
- [ ] With the settings page shown, the header is still there, and says the repository and the branch.
- [ ] In the worktree `login` of `api`, on `feature/login`, the header says `api › login › feature/login`; in `api`'s
      own folder, on `main`, `api › main`.
- [ ] A click on `api` in the header lists the repositories open; a click on `login` opens the menu of the worktrees of
      `api`.
- [ ] With `main` two ahead of its upstream, the name of the branch in the header has no `↑2`; **Sync** has it.

### `window.brand` — The logo and the name of Gitardo

The logo of Gitardo at the top left of the window, and its name at the start of the header bar ([D58](../08-decisions.md#d58--the-window-draws-its-own-title-bar-with-the-logo-and-the-name-of-gitardo)).

- **Kind**: View
- **Function**: `views::window::brand_view(state: &BrandState, ui) -> Vec<Intent>`
- **Entry points**: always shown, above the bar of views and at the start of the header bar.

The logo is the one of `extension/media/gitardo.svg`, kept as a copy beside the outlines of Lucide
(`rust/crates/render/icons/gitardo.svg`, which a test compares with the extension's) and drawn as an icon of the
window, as `graph.icons` draws Lucide's: at the size of the icons of the bar of views, 16 logical pixels, with their
weight of line, in the accent of each theme (Q560). It is in the middle of the square above the bar of views, as high
as the header bar and as wide as the bar of views, at the left of the header bar; the bar of views starts under it.
The name **Gitardo** comes first in the header bar, before what `graph.where` says, in the weight of a title (the
strong text of the interface), where the repository started before. In the narrow layout, below 900 logical pixels,
where the header gives up its words, the name goes and the logo stays (Q561). The screen reader is told of both, as
**Logo of Gitardo** and **Gitardo**. Neither does anything when clicked: the logo is part of the caption of
`window.title-bar`, and the name too.

Acceptance:

- [ ] The logo is drawn in the square above the bar of views, in each theme.
- [ ] The header bar says `Gitardo` before `api › login › feature/login`.
- [ ] In a window 700 logical pixels wide the name is not drawn and the logo is.
- [ ] A double click on the logo or on the name does what a double click on the caption does: here it asks nothing
      of Gitardo, and what the window does with it is shown by `window.title-bar`, whose `hit` says Caption there.

### `window.title-bar` — The header bar as the title bar of the window

The window has no title bar of the system: the free parts of the header bar move it, maximize and restore it, and open
the system's menu of the window, and its edges resize it ([D58](../08-decisions.md#d58--the-window-draws-its-own-title-bar-with-the-logo-and-the-name-of-gitardo)).

- **Kind**: Derivation
- **Function**: `views::window::hit(place: Point, size: WindowSize, frame: &FrameState) -> Hit`
- **Entry points**: the pointer on the header bar or on the edges of the window; `Alt+Space`.

`hit` says what a place of the window is, as the system asks it: **Caption** for the parts of the header bar that are
not a button, a field or a link (the logo and the name included), as the last frame drew them; **Minimize**,
**Maximize** and **Close** for the three buttons of `window.caption-buttons`; an **Edge** or a **Corner** for a band of
6 logical pixels along the edges of the window while it is not maximized; and **Client** for the rest. A dialog of
Gitardo open over the window covers the header bar, which is then not the caption, as the system's title bar of a
window under a dialog of its own does not move it.

On Linux the window is opened without decorations, and a press on the caption or on an edge asks the compositor to
move it, to maximize or restore it on a double click, to resize it or to open its menu, as `winit` lets a window ask;
a right click on the caption opens the compositor's menu of the window, asked with `xdg_toplevel.show_window_menu` on
Wayland and with the message `_GTK_SHOW_WINDOW_MENU` to the window manager on X11, at the place of the pointer; on
macOS the system's title bar is transparent over the header bar, which starts after the system's three buttons, and
the system resizes the window from its edges. On Windows the window is opened without decorations, keeping the styles of a window with a frame, so that the system
still gives it its shadow, its rounded corners, Aero Snap and its keys; its inside covers the frame, within the work
area of the screen while it is maximized. `platform` answers `WM_NCHITTEST` with `hit`: the caption, which the system
moves the window from, maximizes and restores with a double click and opens its menu from with a right click, the edges
and corners, which it resizes the window from with their cursors, and the maximize button, which shows Snap Layouts on
Windows 11; the pointer on **Maximize** is then the system's, and `platform` hands it to Gitardo as input. `Alt+Space`
opens the system's menu of the window. The title of the window is set as before, though not drawn, since the taskbar,
`Alt+Tab` and the screen reader say it: *\<repository\> — Gitardo*, or **Gitardo** with no repository open
([D75](../08-decisions.md#d75--gitardo-by-its-name-wayland-of-its-own-github-copilot-as-a-provider)).

The header bar is drawn dimmed while the window has not the focus, as the system dims the title bar of a window behind
another; **Close** under the pointer is still red. When the window starts to be moved or resized (`WM_ENTERSIZEMOVE`
on Windows), what a press outside closes, a menu, a picker, a popover or the side column opened over the timeline,
closes, as the system closes its own menus; a dialog stays, since closing it would lose what it asks, and its backdrop
already keeps the header bar from moving the window. A change of the scale of the screen while the window is dragged
draws it again, and `hit` reads the header bar as that frame drew it. The size of the window as it is restored, where it
is and whether it is maximized are kept when it closes, in `state.json` under `windowSize`, `windowPosition` and
`windowMaximized`, and it opens so at the next start; a place where the window could not be taken by its title bar on
any screen is not used, and the system places the window.

Acceptance:

- [ ] The window opens with no title bar of the system, and keeps the title it is given, which the taskbar says.
- [ ] The title of the window is the repository shown, then **Gitardo**, and **Gitardo** alone with no repository.
- [ ] `hit` says Caption for a free place of the header bar, the logo and the name; Client for **Search**, **Sync**
      and the repository in the header.
- [ ] `hit` says Edge for a place 3 pixels from the left edge, Corner for one 3 pixels from the bottom right corner,
      and Client for both while the window is maximized.
- [ ] On Windows the system is told the caption, which it moves the window from and maximizes or restores it from with
      a double click, the maximize button and each edge and corner.
- [ ] `Alt+Space` opens the system's menu of the window.
- [ ] The header bar is dimmed while the window has not the focus, in each theme.
- [ ] A menu or a picker open when the window starts to be moved or resized closes.
- [ ] After a change of scale the header bar is laid out again, and `hit` reads it as the new frame drew it.
- [ ] The size, the place and whether the window is maximized when it closes are kept for the next start.
- [ ] A place kept is used on a screen, and not when it is off every screen.
- [ ] On Linux and macOS a press on the caption or an edge asks the system to move, maximize or restore, or resize
      the window, or to open its menu.
- [ ] On Linux a right click on the caption opens the compositor's menu of the window where the pointer is:
      `xdg_toplevel.show_window_menu` on Wayland, `_GTK_SHOW_WINDOW_MENU` to the window manager on X11.

### `window.caption-buttons` — Minimize, maximize and close

**Minimize**, **Maximize** or **Restore**, and **Close**, at the right end of the header bar on Windows and Linux,
drawn by Gitardo ([D58](../08-decisions.md#d58--the-window-draws-its-own-title-bar-with-the-logo-and-the-name-of-gitardo)).

- **Kind**: View
- **Function**: `views::window::caption_buttons_view(state: &CaptionState, ui) -> Vec<Intent>`
- **Entry points**: the three buttons at the right end of the header bar, after **…**.

Three buttons 46 logical pixels wide, as Windows draws its own, filling the height of the header bar, with the
outlines Windows uses, one pixel wide; **Close** goes red under the pointer, the other two lighter. **Maximize**
becomes **Restore** while the window is maximized, and on Windows 11 the pointer resting on it shows Snap Layouts,
since `hit` says Maximize there. They reach the top edge and the right edge of the window, and so of the screen while
it is maximized. Each is named for the screen reader, and has its name in its tooltip; they take no keyboard focus,
as the system's do not. **Close** closes the window as the system's close request does. On macOS the system's three
buttons are kept at the top left, the logo after them, and these are not drawn.

Acceptance:

- [ ] On Windows the header bar ends with **Minimize**, **Maximize** and **Close**, in that order, after **…**.
- [ ] **Minimize** minimizes the window; **Maximize** maximizes it and becomes **Restore**, which restores it.
- [ ] **Close** closes the window as the close request of the system does.
- [ ] The pointer on **Close** draws it red.
- [ ] The screen reader names the three buttons `Minimize`, `Maximize` and `Close`, and `Restore` while maximized.
- [ ] On macOS the header bar leaves room for the system's three buttons at the top left and draws none of its own.

### `graph.search-button` — The button of Quick search

**Search** in the header opens Quick search, as `Ctrl+K` does.

- **Kind**: View
- **Function**: `views::search::quick_search_view(state: &QuickSearchState, ui) -> Vec<Intent>`
- **Entry points**: **Search** in the header; `Ctrl+K` (`Cmd+K` on macOS) anywhere in the window.

A button with the icon of a search, the word **Search** and the keys `Ctrl K` beside it, which opens the picker of
Quick search with nothing typed. Quick search only ever opened its picker, so it is a button and not a box
([D21](../08-decisions.md#d21--islands-a-bar-of-views-and-a-status-bar)). When the window is narrow it keeps its icon
alone, with **Search (Ctrl+K)** in its tooltip.

Acceptance:

- [ ] A click on **Search** opens the picker of Quick search with nothing typed.
- [ ] `Ctrl+K` opens it too, from the timeline or a page.
- [ ] In a window narrower than 900 pixels the button shows its icon alone.

### `graph.islands` — The islands of the window

The side column, the timeline, the inspector and the page shown are each a rounded card on the ground of the window,
under the header.

- **Kind**: View
- **Function**: `views::island(ui: &mut Ui, rect: Rect, inside: impl FnOnce(&mut Ui) -> R) -> R`
- **Entry points**: the whole window.
- **Settings**: `gitardo.graph.islands`, in the extension only, off by default.
- **Uses**: `graph.view-bar`, `repo.side-column`, `graph.toolbar`, `activity.status-bar`, `graph.resize-panes`.
- **Extension**: with `gitardo.graph.islands` on, the references, the commits and the details of the Graph view are each
  an island, on a ground one step darker than the panel's background in the dark themes and one step deeper in the
  light ones, with the radius and the border of VS Code's widgets and a gap of 8 pixels between them, 4 in a panel
  narrower than 900 pixels; the gap is the line dragged to resize them, and the details under the commits in a narrow
  panel are an island too. The toolbar stays above them, on the ground. Off, the view is as it was. The other views
  keep their look
  ([D53](../08-decisions.md#d53--pages-beside-the-inspector-a-bottom-panel-the-icons-of-lucide-islands-in-the-extension)).
  The class `islands` of the Graph view's `App` (`src/webview/App.tsx`) and the rules under it in
  `src/webview/styles.css`; the ground is mixed from `--vscode-panel-background` with black. Tested by
  `test/e2e/ui.mjs`, step "with islands on, the references, the commits and the details are rounded islands on a
  darker ground, the gap between two is dragged to resize them, and off the graph is as it was".

At the top, the header, a bar on the ground across the window (`graph.toolbar`). Under it, from the left: the bar of
views, on the ground; the side column, from under the header to the status bar (`repo.side-column`); on its right, the
timeline and the inspector, or the page, and the bottom panel. There is no island above the side column, and the header
is not one
([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).

The ground is one step darker than the islands in the dark themes and one step deeper in the light ones; each island
has the radius of a dialog and a border, with a gap of 8 pixels between islands, 4 when the window is narrow. In the
high contrast themes the ground and the islands are the same and the border tells them apart. The gap between two
islands is the line that is dragged to resize them (`graph.resize-panes`). The inspector opened over the timeline in a
narrow window is an island of its own over it. What is drawn in an island stays inside its rounded corners
([D21](../08-decisions.md#d21--islands-a-bar-of-views-and-a-status-bar)).

Acceptance:

- [ ] In the dark theme, the ground between the rail and the timeline is darker than both.
- [ ] The corners of the inspector are the ground of the window, whatever is drawn in it.
- [ ] In a window narrower than 900 pixels the gap is 4 pixels.
- [ ] Extension: with `gitardo.graph.islands` on, the references, the commits and the details are three islands with
      a gap between them, and dragging the gap between the commits and the details changes their widths.
- [ ] Extension: with `gitardo.graph.islands` off, the Graph view has no gap and no rounded corners.

### `graph.view-bar` — The bar of views

A column of icons on the left edge of the window, each the way to a place of it.

- **Kind**: View
- **Function**: `views::view_bar_view(state: &ViewBarState, ui) -> Vec<Intent>`
- **Entry points**: the left edge of the window.
- **Uses**: `graph.view`, `changes.pane`, `file-search.page`, `compare.choose`, `panel.bottom`,
  `activity.what-is-running`, `settings.edit`.

From the top: **Timeline**, which comes back to the timeline (`graph.view`); **Changes**, with the number of changed
files on it, which opens the Changes page (`changes.pane`); **Files**, which opens the Files page; **Search in
files**, which opens the Search page with the keyboard in its box (`file-search.page`); **Compare**, which asks what
to compare (`compare.choose`); and at its foot **Terminal**, which shows the bottom panel at its terminals
or its Command log and hides it (`panel.bottom`), **Activity**, which shows the bottom panel at its **Activity** tab
and hides it (`activity.what-is-running`), and **Settings**, which opens the settings page (`settings.edit`). The
place shown is marked with the ground of an island; **Terminal** is marked while the bottom panel shows its terminals
or the Command log, **Activity** while it shows its **Activity** tab. Each says its name in its tooltip. Before a
repository is read only **Activity** and **Settings** can be clicked
([D21](../08-decisions.md#d21--islands-a-bar-of-views-and-a-status-bar)).

Acceptance:

- [ ] With three changed files, **Changes** shows `3`; with none, no number.
- [ ] A click on **Changes** opens the Changes page and marks **Changes**; a click on **Timeline** comes back.
- [ ] A click on **Activity** shows the bottom panel at **Activity**, and another hides the panel.
- [ ] A click on **Terminal** shows the bottom panel, and another hides it.
- [ ] With the bottom panel at **Activity**, a click on **Terminal** shows its terminals.
- [ ] With no repository read, **Timeline**, **Changes**, **Files**, **Search in files**, **Compare** and
      **Terminal** cannot be clicked.
- [ ] A click on **Search in files** opens the Search page and marks **Search in files**.

### `panel.bottom` — The bottom panel

An island under the timeline and the inspector, or the page shown, with the terminals, the tasks of the repository,
what is running and the Command log, each in a tab of its own.

- **Kind**: View
- **Function**: `views::panel::bottom_view(state: &BottomPanelState, ui) -> Vec<Intent>`
- **Entry points**: `` Ctrl+` `` (`` Cmd+` `` on macOS), which shows the panel at its terminal, or hides it when it has
  the focus; **Terminal** in the bar of views (`graph.view-bar`); **Show Terminal** and **Show Command Log** in Quick
  search; the activity indicator of the status bar, **Activity** in the bar of views and **Show What Is Running**,
  which show it at **Activity** (`activity.what-is-running`).
- **Uses**: `terminal.view`, `tasks.view`, `activity.what-is-running`, `log.show`, `graph.islands`,
  `graph.resize-panes`.
- **Extension**: VS Code's own panel, with its terminals and the output channel **Gitardo**: nothing to build.

- The panel is an island of its own, as wide as the timeline and the inspector together, or as the page shown, above
  the status bar; the branch rail keeps its whole height beside it. Hidden, it leaves its room to the islands above.
- At its top, on the left, its tabs: **Terminal**, **Tasks**, **Activity** and **Command log**
  ([D64](../08-decisions.md#d64--activity-a-tab-of-the-bottom-panel-whose-list-holds-still),
  [D65](../08-decisions.md#d65--tasks-a-tab-of-the-bottom-panel-with-the-tasks-of-the-repository-and-their-groups));
  the one shown is marked, and **Tasks** has beside its name the number of tasks running, when some are
  (`tasks.view`).
  On the right, with **Terminal** shown: **New terminal**, the list of the terminals when there are more than one, each
  named by its shell and numbered, and **Kill the terminal**; then, always, **Hide the panel**, which hides it and
  stops nothing.
- **Terminal** in the bar of views hides the panel when it shows its terminals or the Command log; otherwise it shows
  it at the tab it showed last, or at **Terminal** when that was **Activity**, which has a button of its own (Q681).
- The gap above it is the line dragged to change its height: it starts at 240 px, never less than 100 px nor more than
  70% of the window, and a double click gives it 240 px again. In the narrow layout the gap is 4 px, as between the
  other islands.
- Whether it is shown, its height and the tab shown are the window's, remembered for the workspace; the terminals are
  each repository tab's (Q418).

Acceptance:

- [ ] `` Ctrl+` `` shows the panel at the terminal, with the focus in it; pressed again there, it hides the panel.
- [ ] The tabs are **Terminal**, **Tasks**, **Activity** and **Command log**, from the left, and a click on
      **Activity** shows what is running, the panel as high as before.
- [ ] The height dragged to 300 px is there after the window is closed and opened again on the same workspace.
- [ ] The panel shown at **Activity** is there, at that tab, after the window is closed and opened again on the same
      workspace.
- [ ] **Hide the panel** while a command runs in the terminal leaves it running: shown again, its output has grown.

### `terminal.view` — A terminal in the bottom panel

A shell in the folder of the repository shown, inside the window, with the colors and the keys of a terminal.

- **Kind**: View
- **Function**: `views::terminal::terminal_view(state: &TerminalState, ui) -> Vec<Intent>`
- **Entry points**: **Terminal** in the bottom panel (`panel.bottom`), which starts one when there is none; **New
  terminal** there; `` Ctrl+Shift+` ``; **New Terminal** in Quick search.
- **Settings**: `terminal.shellPath` (empty: PowerShell 7 on Windows when it is installed, else Windows PowerShell; the
  shell of `SHELL` on Linux and macOS), `terminal.scrollbackLines` (10,000 lines), `editor.fontFamily` and
  `editor.fontSize`.
- **Uses**: `panel.bottom`, `repo.watch-changes`.

- A terminal starts the shell in the folder of the repository tab shown, the worktree shown in it, through a
  pseudo-terminal of the system (ConPTY on Windows), with `TERM=xterm-256color`, so that what git and other programs
  print has its colors and a program that draws the whole screen (an editor, a pager) works there.
- What it prints is drawn in the font of the editor, with the 16 colors of the theme's terminal palette, 256 colors and
  24-bit colors, bold, italic, underline and the cursor; it scrolls back as far as `terminal.scrollbackLines` keeps.
- The keys go to the shell, `Ctrl+C` included, but for those of the window: `` Ctrl+` ``, `Ctrl+Tab`, `Ctrl+K`, and
  `Ctrl+Shift+C` and `Ctrl+Shift+V`, which copy what is selected and paste. With text selected, `Ctrl+C` copies it
  instead. Text is selected with the pointer; a double click selects a word.
- When the panel or the window changes size, the shell is told its new columns and rows.
- What the commands typed there change in the repository shows in the timeline as anything done in another terminal
  does (`repo.watch-changes`).
- When the shell ends, the terminal says **The shell ended with exit code `<n>`** under what it printed, and `Enter`
  starts a new one in its place.
- Each repository tab has its own terminals, kept while another tab is shown; closing a tab or the window ends its
  shells, without asking.

Acceptance:

- [ ] The first terminal of a repository tab starts in the folder of that repository: `pwd` prints it.
- [ ] `git log --oneline --decorate -1` prints its labels in color.
- [ ] `git log` with more commits than the terminal has rows runs its pager, and `q` leaves it.
- [ ] `git commit` typed in the terminal shows the new commit in the timeline.
- [ ] Making the panel narrower changes what `tput cols` prints, on Linux and macOS.
- [ ] Text selected and copied with `Ctrl+Shift+C` is on the clipboard.
- [ ] **New terminal** adds a second one; each keeps what it printed when the other is shown.
- [ ] Showing another repository tab shows its terminals, and coming back shows the first ones as they were.
- [ ] `exit` says that the shell ended with exit code 0, and `Enter` starts a new one.

### `graph.icons` — The icons of the window

Every icon of the window is one of [Lucide](https://lucide.dev)'s, each with one meaning, so that the same action looks
the same everywhere, and as it does in other applications; the logo of Gitardo is not, and is drawn the same way
(`window.brand`), nor are the icons beside the names of files and folders, which are those of Symbols
(`file-view.file-icons`).

- **Kind**: View
- **Function**: `render::Canvas::draw_icon(icon: Icon, origin: Point, size: f32, color: Color)`
- **Entry points**: every button, item of a menu (`ui.menu-icons`), row and mark of the window that has an icon.

The outlines of the icons are Lucide's, kept in the repository with its licence (ISC), drawn in the color and at the
size asked, 16 logical pixels unless said otherwise, with their stroke scaled so that they keep the weight of the text
beside them (Q420): 1.5 logical pixels at 16. An icon is used for the meaning the table gives it, and for no other; a
button whose meaning is not in the table gets a line of it before it gets an icon. A view names the meaning of what it
draws (`views::icons`), never the icon, and the widgets of the toolkit draw only the icons of closing, of a choice, of
the previous and the next match, of a search and of a menu that opens to the side; the half-ticked check box draws its
own dash.

| Meaning | Lucide icon |
| --- | --- |
| Folded, a menu that opens to the side; unfolded | `chevron-right`, `chevron-down` |
| Close a page, the inspector, a box, the bottom panel; empty a box | `x` |
| Chosen in a menu; done; a good commit of a bisect | `check` |
| Add: stage a file or a change, take a side into the result, a new terminal, add a submodule, another tab | `plus` |
| Unstage | `minus` |
| Back to how it was: discard the changes of a file, revert a change, Undo | `undo-2` |
| Delete: a branch, a tag, a stash dropped, a terminal killed, an entry of a list | `trash-2` |
| Save, save all | `save` |
| Search | `search` |
| Filter what is shown or listed | `funnel` |
| Show, show beside; hide | `eye`, `eye-off` |
| Branch; create a branch | `git-branch`, `git-branch-plus` |
| Tag; create a tag | `tag` |
| Remote | `cloud` |
| A tag a remote does not have; one it has under another object | `cloud-off`, `cloud-alert` |
| Commit; open a commit as a page | `git-commit-horizontal`, `maximize-2` |
| Merge; cherry-pick | `git-merge`, `cherry` |
| Compare; swap the two sides | `git-compare-arrows`, `arrow-left-right` |
| Pull request; a draft | `git-pull-request`, `git-pull-request-draft` |
| Fetch; pull; push; Sync | `cloud-download`, `arrow-down-to-line`, `arrow-up-from-line`, `arrow-down-up` |
| Commits to pull and to push, beside their count | `arrow-down`, `arrow-up` |
| Read again | `refresh-cw` |
| The previous and the next change or match | `chevron-up`, `chevron-down`, in the bar of a page |
| Back to the top | `arrow-up-to-line` |
| Move up, move down in a list | `move-up`, `move-down` |
| The bar of views: Timeline, Changes, Files, Compare, Terminal, Activity, Settings | `git-graph`, `file-diff`, `folder-tree`, `git-compare-arrows`, `square-terminal`, `activity`, `settings` |
| Command log | `scroll-text` |
| Folder; file, beside a name only with `fileIcons` set to `simple`; open the file; open its changes; open all the changes | `folder`, `file`, `file-text`, `file-diff`, `files` |
| Worktree, and a repository open in **Repositories** (Q502); submodule | `folder-git-2`, `boxes` |
| Stash; apply; pop | `archive`, `archive-restore`, `package-open` |
| Favorite; recent | `star`, `clock` |
| History of a file, a folder or lines; the reflog | `history` |
| Who changed each line; on every file | `user-pen`, `pin` |
| Drafted or proposed for you: by the AI, for a conflict, for a worktree; resolved by the rules | `sparkles`, `wand-sparkles` |
| Edited, not saved or not committed | `pencil` |
| Waits; did not run, skipped | `hourglass`, `circle-slash` |
| Warning; error, a bad commit of a bisect; information | `triangle-alert`, `circle-x`, `info` |
| What is running | `activity` |
| More | `ellipsis` |
| How the timeline is shown: the menu of the view | `sliders-horizontal` |
| A locked worktree; lock one | `lock` |
| List; tree of folders | `list`, `list-tree` |
| Fold the lines that did not change, the commits a merge brought in; unfold them | `fold-vertical`, `unfold-vertical` |
| A button that opens a list of choices below it | `chevrons-up-down` |
| Accept every change that needs no choice | `check-check` |
| Stop what is running: drafting a message | `circle-stop` |
| Open in the external editor | `external-link` |
| Take the incoming version, or the version of the other side of a comparison; take the current version | `circle-arrow-down`, `circle-arrow-up` |
| Drag to move a row | `grip-vertical` |
| A bisect; start one, mark a commit good or bad (in a menu) | `binoculars` |
| A submodule not initialized | `circle-dashed` |
| The base of a merge, shown or hidden | `git-fork` |
| The look of the window: the section **Appearance** of the settings | `palette` |
| Experimental: the list of the experimental settings | `flask-conical` |
| Copy: a hash, a message, a path, a patch, the text selected; cut; paste | `copy`, `scissors`, `clipboard-paste` |
| Rename a branch or a worktree; edit the message of a commit | `pen-line` |
| Check out a branch, a tag or a commit | `log-in` |
| Open in a new window; reveal a folder in the file manager of the system | `app-window`, `folder-open` |
| The logo of Gitardo, at the top left of the window | the extension's `gitardo.svg`, not Lucide's |
| The signature of a commit, and signing one; the co-authors of a commit | `shield`, `users` |

Acceptance:

- [ ] No icon of the window is drawn for two meanings of the table: a test goes through every `Icon` the views use and
      the meaning each is given.
- [ ] **Discard changes** in the menu of a file of the Changes page and **Revert this change** in a Diff page draw
      the same icon, and **Drop** on a stash another one.
- [ ] Each icon drawn at 16 px and at 32 px matches its snapshot, in the dark and the light theme.
- [ ] The licence of Lucide is in the repository beside its outlines.

### `graph.view-menu` — The menu of the view of the header

Every switch of what the timeline shows and how it draws it, in one menu apart from the actions
([D68](../08-decisions.md#d68--two-menus-in-the-header-the-view-of-the-timeline-and-its-actions)).

- **Kind**: View
- **Function**: `views::graph::view_menu_view(state: &ViewMenuState, ui) -> Vec<Intent>`
- **Entry points**: the button **View**, with the icon of the sliders, right before **…** at the end of the header
  (Q660, Q661).
- **Offered when**: a repository is read; before, the button cannot be clicked.
- **Uses**: `filter.kinds`, `graph.stashes`, `graph.first-parent`, `graph.one-lane`, `graph.compact-rows`,
  `graph.trace-selected`, `graph.date-separators`, `graph.combine-upstream`, `graph.fold-all`, `graph.columns`.
- **Extension**: the button **View**, with the codicon `settings`, the sliders, right before **…** in the toolbar of
  the Graph view and of the Gitardo Graph view, whose menu has the same switches in the same groups (`viewMenu` in
  `src/webview/toolbarMenus.ts`), each ticked while it is on and writing its setting for the user, but **Compact
  rows**, which the extension does not have yet (`graph.compact-rows`, Q655). It opens at the pointer, as the
  extension's menus do, and has no snapshot (Q665).

In this order, separated in groups (Q663):

1. What the timeline shows: **Show local branches**, **Show remote branches**, **Show tags** (`filter.kinds`); **Show
   stashes** (`graph.stashes`); **Show only the first parent of merges** (`graph.first-parent`).
2. How it draws it: **Show all commits on one line** (`graph.one-lane`); **Compact rows** (`graph.compact-rows`);
   **Bring out the history of the selected commit** (`graph.trace-selected`); **Separate the days, weeks and months**
   (`graph.date-separators`); **Put a branch and its upstream on one label** (`graph.combine-upstream`); **Fold all
   merges** and **Unfold all merges** (`graph.fold-all`), while the graph has its lines and is not on one line.
3. What each row shows beside its message: **Show the author**, **Show the date**, **Show the hash**
   (`graph.columns`).

Each switch shows the check in the column of the icons while it is on, and no icon of its own (`ui.menu-icons`, Q610);
**Unfold all merges**, an action, has the icon of unfolding. Each writes its setting in the user's settings file,
which every window follows. The menu opens under its button and scrolls when it is taller than the room
(`ui.menu-scroll`); in the narrow layout of the window the button is the same, an icon alone. It has no key (Q664).

Acceptance:

- [ ] Every entry above is in the menu, in its group and in that order, with its condition.
- [ ] Each switch shows the check while it is on and no other icon.
- [ ] **Fold all merges** and **Unfold all merges** are in the menu only while the graph has its lines and is not on
      one line.
- [ ] The button is right before **…**, with the tooltip **View**, in a wide and in a narrow window, and its menu opens
      under it with the switches alone.
- [ ] Unchecking **Show the hash** takes the hashes out of the timeline and sets `graph.columns` to `["author",
      "date"]` in the user's settings.
- [ ] In a window 400 logical pixels high, the menu opens under its button, and its last entry can be scrolled to and
      chosen (`ui.menu-scroll`).
- [ ] The menu open matches its snapshots in the dark, the light and the two high contrast themes.

### `graph.more-menu` — The … menu of the header

The actions on the repository and the ways to what is not elsewhere in the header, apart from the switches of the
timeline, which are in the menu of the view (`graph.view-menu`,
[D68](../08-decisions.md#d68--two-menus-in-the-header-the-view-of-the-timeline-and-its-actions)).

- **Kind**: View
- **Function**: `views::graph::more_menu_view(state: &MoreMenuState, ui) -> Vec<Intent>`
- **Entry points**: **…** at the end of the header.
- **Extension**: **…** at the end of the toolbar of the graph, with the actions alone (`moreMenu` in
  `src/webview/toolbarMenus.ts`): **Refresh**, but in the side bar, where the header of the view has it; the entries
  of group 2, with **Browse reflog…**; **Switch identity…**; **Load pull requests from GitHub**. It has neither
  **Branch**, **Stash** and **Undo**, which are buttons of the toolbar, nor **Settings** and **Theme…**, which are VS
  Code's.

In this order, separated in groups:

1. **Refresh** (`repo.schedule-reads`); **Branch** (`branch.create` at `HEAD`), **Create branch with no history…**
   (`branch.create-orphan`), **Stash** (`stash.push`), **Undo** (`undo.last`).
2. **Compare branches, tags or commits…** (`compare.choose`), **Go to commit…** (`search.go-to-commit`), **History of
   the active file** (`file-history.file`) of the File page in front, which says to open a file when none is,
   **Reflog** (`reflog.browse`), **Clean up branches…** (`branch.clean-up`), **Apply patch…** (`patch.apply`),
   **Manage remotes…** (`remote.manage`), **Add submodule…** (`submodule.add`) while the repository has no submodule
   (the Submodules list has it otherwise), **Select repository…** (`repo.select`) when it is offered.
3. **Switch identity…** (`identity.switch`).
4. **Load pull requests from GitHub** (`github.load-pull-requests`), when the main remote is on GitHub, or **Load merge
   requests from GitLab** (`hosting.load-merge-requests`) on a GitLab; then **Check out a pull request…** (**Check out
   a merge request…** on GitLab, `hosting.checkout-pull-request`) on GitHub, GitLab, Gitea and Forgejo.
5. **Settings** (`settings.edit`), **Theme…** (`settings.theme`).
6. **Ask Gitardo…** (`help.page`), which a Help menu would hold: the window has no menu bar (Q1351).

**Theme…** stays here, though it changes how the window looks: it opens a picker rather than switching something on
or off, and it is about the whole window, not the timeline (Q662).

Acceptance:

- [ ] Every entry above is in the menu, in that order, with its condition.
- [ ] No switch of the timeline is in the menu: they are in the menu of the view.
- [ ] **Add submodule…** is not in the menu of a repository with submodules.
- [ ] In a window 400 logical pixels high, the menu opens under **…**, and its last entry can be scrolled to and chosen
      (`ui.menu-scroll`).
- [ ] The menu open matches its snapshots in the dark, the light and the two high contrast themes.

### `ui.menu-scroll` — A menu taller than the room under it

A menu, or a level of one, that does not fit the window scrolls inside it, so that every entry can be reached, and it
stays where it opened
([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).

- **Kind**: Derivation
- **Function**: `ui::widget::menu::layout::place(size, placement, window, padding) -> MenuPlace`, which gives where
  the level goes and how much of it is shown.
- **Entry points**: every menu of the window: the menus under a button (the menu of the view, **…**, **Sync**, the
  repository and the worktree of the header),
  the context menus and their submenus.

- A menu under a button opens below it, its left side lined up with the button's, and never above it. When it is
  taller than the room between the button and the foot of the window, less a gap of 8 logical pixels, it is as tall as
  that room and its entries scroll.
- A menu opened at the pointer opens below the pointer when it fits there, else above it when it fits there; when it
  fits neither, it takes the taller of the two rooms and scrolls.
- A submenu opens beside its item, its first entry level with the item, moved up only as much as the window needs, and
  scrolls in the same way when it is taller than the window.
- A level that scrolls has a bar on its right, inside its border, as thin as the bars of the lists, which can be
  dragged; the wheel scrolls it by entries; the arrows move the entry lit and scroll so that it is shown whole; `Home`
  and `End` light the first and the last entry; `PageUp` and `PageDown` move by as many entries as are shown. Its first
  and last shown entries fade into its border when there is more above or below.
- A menu that fits is drawn as before, without a bar.

Acceptance:

- [ ] Opened under **…** in a window 400 logical pixels high, the menu's top is at the button's foot, and its bottom 8
      logical pixels above the foot of the window.
- [ ] The wheel over that menu scrolls it until its last entry is shown whole, and no further.
- [ ] `End` lights the last entry and shows it; `Enter` chooses it.
- [ ] A menu that fits has no bar and opens where it opened before.
- [ ] A context menu opened near the foot of the window opens above the pointer when it fits there.

### `ui.menu-icons` — The icon of each action in the menus

Every item of a menu that does something shows the icon of its action at the left of its words, the one a button of
the same action draws, so that the eye finds an action in a long menu and one meaning is one icon across the window
([D63](../08-decisions.md#d63--the-icon-of-each-action-in-the-menus)).

- **Kind**: Derivation
- **Function**: `views::icons::operation_icon(feature: &str) -> Option<Icon>`, which gives the icon of an item that
  runs an operation of the catalog, by the identifier of its card; the other items name the meaning of their action
  from `views::icons` where their menu is made.
- **Entry points**: every context menu and every menu under a button: the menu of a commit and of its labels in the
  timeline, of the draft, of a stash, of a selection, of a drop; the menus of a branch, a remote branch, a tag, a
  worktree, a stash and a submodule in the side column, and of their lists; the menu of the view, the **…** and the
  **Sync** menus of the header, the repository and the worktree of the header, the menu of a repository tab and the
  **+** of **Repositories**; the menus of a file in the Files page, the inspector, the Commit page and the comparison;
  the menu of a file of the changes and the **…** of their heading (`changes.row-menu`); the menus of the editors of
  the File, Diff and Merge pages; the chooser of the comparison, the sections of the settings.
- **Uses**: `graph.icons`.
- **Extension**: the extension draws its own menus in its webviews, with no icons yet (Q619); VS Code's own menus show
  none.

- The icon is at the left of the words, in the column the toolkit keeps for icons in every menu, drawn at 16 logical
  pixels in the color of the words: lit with them under the pointer or the keys, dimmed with them when the item cannot
  be chosen. No icon has a color of its own.
- An action has the icon the table of `graph.icons` gives its meaning, the one its button draws where it has one:
  every **Copy …** the copy icon, every **Delete …**, **Remove …**, **Drop** and **Prune** the bin, **Show in the
  timeline** the Timeline's, **Compare with…** the icon of Compare, wherever the item is. An item that runs an
  operation takes its icon from `operation_icon`, so that the menu of a branch, of its label and of a drop agree.
- An item that no icon says clearly has none: the operations that rewrite the history (rebase, reset, squash, split,
  fixup, the interactive rebase page), **Apply patch…**, **Switch identity…**, **Synchronize URLs from .gitmodules**,
  **Go to line**, and the items of the menus that list values (the action of a step of a rebase, the value of a
  setting, a font, a page of the way back). Its words start where those of the items with an icon do.
- An item that switches or chooses (the switches of the menu of the view, the kinds of references shown, the order of a
  list, the mode of the search box, the repository and the worktree of the header) shows the check in the column of the
  icons while it is on, and no icon of its own (Q610).
- The views make their items through `views::icons` only, never with `MenuItem::new`, so that an item without an icon
  is one that was asked for by name.

Acceptance:

- [ ] Every item of every menu the views make has the icon of its action, or is on the short list of the items without
      one with why: a test goes through the sources of the views and fails on any other item made without an icon, and
      on an operation offered in a menu with no icon in `operation_icon`.
- [ ] **Discard all** and **Stash changes** of the draft draw the icons of the buttons **Discard changes** and **Stash
      file** of the Changes page; the items of a stash draw the icons of its buttons in the side column.
- [ ] Every **Copy …** item draws the copy icon, and every **Delete …**, **Remove …** and **Drop** item the bin.
- [ ] The kinds of references in the menu of the view show the check while they are shown and no other icon, and none
      while the branch filter chose them.
- [ ] In a menu, the words of an item without an icon start where those of an item with one do.
- [ ] An icon is drawn in the color of the words of its item, and in the dimmed color of an item that cannot be chosen.
- [ ] The menu of a commit of the timeline and the menu of a file of the Files page match their snapshots in the dark,
      the light and the two high contrast themes.

### `ui.picker-scroll` — A picker longer than its rows

The list of a picker scrolls with a bar as well as the wheel, and keeps the row lit in view, so that a long list can be
gone through with the pointer or the keys.

- **Kind**: Derivation
- **Function**: `ui::widget::picker::first_shown(lit: usize, first: usize, shown: usize, count: usize) -> usize`, which
  gives the first row shown so that the lit one is shown.
- **Entry points**: every picker: Quick search, the filters of the timeline and of the branch rail, the choices of an
  operation.

- When there are more rows than the picker shows, a bar on its right says how much is shown and where, and can be
  dragged; a click above or below its thumb moves by a page.
- The arrows move the row lit, scrolling so that it is shown whole; `PageUp` and `PageDown` move by a page, `Home` and
  `End`, while nothing is typed, go to the first and the last row.
- The headings of the groups of rows (**Branches**, **Remote branches**, **Tags**) are drawn above their rows and are
  never lit; the arrows step over them.

Acceptance:

- [ ] With 200 branches in the branch filter, `End` lights the last one and shows it.
- [ ] Dragging the bar to its foot shows the last row.
- [ ] `Down` on the last row shown scrolls the list by one row.
- [ ] The arrows never light a heading.

### `ui.page-trail` — The way back, in the title of a page

The title row of a page names the pages it was reached through, each a way back to that page as it was left, rather
than always `Timeline › <page>`
([D57](../08-decisions.md#d57--worktrees-in-the-side-column-pages-that-keep-what-opened-them-the-way-back-in-the-title)).

- **Kind**: Derivation
- **Function**: `model::trail::page_trail(way: &[P], change: WayChange<P>) -> Vec<P>`, which gives the way after the
  page shown changed; `model::trail::folded(count)` gives the names folded into **…**.
- **Entry points**: the title row of every page, after the button of the side column (`repo.side-column`); `Alt+Left`.
- **Uses**: `repo.side-column`, `diff.beside-inspector`, `compare.beside-diff`, `file-view.beside-files`,
  `changes.beside-file`.
- **Extension**: VS Code has its own way back between its editors: nothing to build.

- The way is the chain of pages each opened from the one before, the timeline first: a page opened from the page
  shown, by one of its links, buttons, menus or keys (`F4` of a Diff page), is added after it.
- A page opened from the bar of views, Quick search, the **…** menu of the header, the side column, or a key of the
  window (`Ctrl+,`, `Ctrl+Tab`) starts the way again from the timeline, with that page alone; so does a page opened
  from the timeline. The timeline shown ends the way. A page that opens once a read ends goes on from the page shown
  then (Q543).
- A page already on the way is gone back to rather than added again, so that the way has no loops.
- The title row says **Timeline**, then the name of each page of the way, each as its own title row names it, then the
  step of the page inside it when it has one (**Files**, **Changes**, the comparison), then the name of the page shown.
  A click on a name shows its page as it was left and drops what came after it from the way: each page keeps what it
  showed while it is open, its scroll, its selection and the file shown beside it (the Changes page shows its file
  beside its list again), so the way keeps the pages and not a copy of them (Q542). **Timeline** shows the timeline.
- At most four names are shown, **Timeline** and the page's own among them; those in between are folded into **…**,
  whose menu lists them, the first opened first. A name before the page's own wider than 200 logical pixels is cut at
  its end with `…`.
- A page closed since is left out of the way; a way that does not end at the page shown is that page alone.
- `Alt+Left`, unless a field of the page uses it, goes back one name: the name before the page's own.

Acceptance:

- [ ] A File page opened by `F4` from the Diff page of a file of a commit, beside the inspector, says
      `Timeline › <the Diff page's name> › <file>`, and that name shows the Diff page beside the inspector again, with
      the same file, and drops the File page from the way.
- [ ] A page opened from the page shown is added after it.
- [ ] A page already on the way is gone back to rather than added again.
- [ ] A page opened from the bar of views, Quick search, the side column or a key starts the way again from the
      timeline.
- [ ] Going back to a name drops what came after it.
- [ ] A page closed is left out of the way, and a way that does not end at the page shown is that page alone.
- [ ] With five names, the second is folded into **…**, whose menu lists it and goes back to its page.
- [ ] `Alt+Left` on the File page shows the Diff page it was opened from, and on that page the timeline.
- [ ] A File page opened from the diff of `a.txt` beside the list of the Changes page says
      `Timeline › <the diff's name> › <file>`, and that name shows the Changes page again with the same diff beside its
      list, which says `Timeline › Changes › <the diff's name>`.

### `graph.load-more` — Load the next commits while scrolling

The commits come a page at a time as they are read, without waiting for the list to be scrolled, so that scrolling does
not stop to wait for them; past the 100,000 commits of one read, the next read starts long before the end of those
shown (D54).

- **Kind**: View
- **Function**: `views::graph::load_more_view(state: &ListState, ui) -> Vec<Intent>`
- **Entry points**: scrolling the list of commits.
- **Settings**: `graph.pageSize`.
- **Uses**: `graph.read-commits`.
- **Extension**: the pages `EngineGraph` adds are laid out and sent by `Controller.showPages` to the views that list
  commits, the graph and Gitardo Graph, those that come within 150 ms together, with only the rows the views do not
  have; the other views, which the rows of a page change nothing of, are sent nothing; `loadMore` of the list of
  commits asks for nothing while a read adds its pages, and past 100,000 commits reads from the last one. The pages
  that come while the rest of the state is read, the newest entry of the journal among it, are laid out under the rows
  right before the views are given the state (`completed` of `src/engine/graph.ts`), so that a read that ends
  meanwhile, as in a repository of a little more than a page, leaves no commit waiting for the list to be scrolled
  (milestone M25). Tested by `test/graphPages.test.ts`, "has every commit, and no more to come, when the read ends
  while the rest of the state is read". While the commits of the history listed are read again from their first page,
  after a commit, a fetch or **Refresh**, the list keeps the length it had until the read reaches it or ends, so that
  it stays at the row it is scrolled to ([D72](../08-decisions.md#d72--the-list-of-commits-keeps-its-length-while-its-commits-are-read-again-proposed)):
  `held` of `src/webview/pages.ts` says how long, and under the commits read so far the rows the list had before are
  drawn where they were, dimmed after 150 ms, without their labels and taking no click, until the read reaches them.
  Another repository, other branches, another filter or other parents followed start from the first page as before
  (`listingOf`), and a read that ends with fewer commits leaves the list as long as they are. `loadMore` asks for
  nothing either once the read ended at the end of the history, which a list asked for while the last page came.
  Tested by `test/pages.test.ts`, "keeps its length and the width of its graph when Refresh reads its commits again
  from the first page, until they are back"; in a window by `test/perf/graph-load.mjs`, which reports the row the list
  is at.

- Each page of a read is added under the rows as soon as it is read, whether the list is scrolled or not, until the
  read ends at the end of the history or at 100,000 commits (`graph.read-commits`).
- Past those, the next read is asked for as soon as the last row drawn is 500 rows or fewer from the end of the
  commits loaded, and once for each number of commits loaded. Counted in commits loaded, so that the commits of folded
  merges count too.
- The bar runs along the top of the list until the first page of a read is there. While there are more, read or still
  to read, the last row says **Loading more commits…** with a spinning icon.
- The commits already listed stay where they are: the rows of the page are added under them, and the scroll position,
  the selection and the folds do not move. A read that finds the same commits as before, as after a file is saved,
  changes none of the rows.
- A read of the same history that finds other commits from some row on, after a commit, a fetch or **Refresh**, takes
  out the rows listed from there, but the list keeps its length and the graph its width until the read reaches them
  or ends ([D72](../08-decisions.md#d72--the-list-of-commits-keeps-its-length-while-its-commits-are-read-again-proposed)):
  under the rows read, the rows of before are drawn where they were, dimmed, without their labels, and a click on one
  selects nothing; the row right under those read says **Loading more commits…**. Where the list is scrolled to does
  not move. A read that ends with fewer commits leaves the list as long as they are.
- Choosing another repository, other branches, a filter or a search cancels the read running and starts a new one;
  the rows listed stay, dimmed, until its first page.

Acceptance:

- [ ] In a repository of 5,000 commits, with the list not scrolled, the five pages are added one after the other and
      the last row no longer says **Loading more commits…** once the fifth is there.
- [ ] With 100,000 commits loaded and more, scrolling to row 99,500 starts the next read; at row 99,499, not yet.
- [ ] Scrolling on while that read runs does not start it again.
- [ ] After a page is added, the selected commit and the scroll position are as they were.
- [ ] Choosing another branch while a read runs stops it: no page of the old read is added after the first of the new.
- [ ] Choosing a branch of 2,500 commits lists its first 1,000 while its read goes on, and the other pages after.
- [ ] The last row says **Loading more commits…** while there are more, and is gone at the end of the history.
- [ ] In a repository of 3,000 commits, the list scrolled to its end: a commit made and **Refresh** leave it at its
      end at each page, with the rows of before under the first page, dimmed, and never the row that says **Loading
      more commits…** in sight; once the last page is there, the 3,001 commits are listed and none of before.

### `graph.shallow-end` — Where the history of a shallow clone ends

In a shallow clone the timeline says where the commits fetched end, and offers to fetch more, so that the last commit
listed is not taken for the first commit of the project
([D77](../08-decisions.md#d77--clone-and-create-a-repository-and-the-history-of-a-shallow-clone-proposed)).

- **Kind**: View
- **Function**: `views::graph::shallow_end_view(state: &ShallowEndState, ui) -> Vec<Intent>`
- **Entry points**: the timeline and the inspector of a shallow repository.
- **Offered when**: `repo.read-shallow` says the repository is shallow.
- **Uses**: `repo.read-shallow`, `graph.lanes`, `graph.load-more`, `details.show`, `sync.deepen`, `sync.unshallow`.
- **Extension**: the same dashed line, with the same tooltip, and the same last row in the **Graph** view, its tabs
  and Gitardo Graph, its two buttons sending `feature/run` of `sync.deepen` and `sync.unshallow` to the engine. The
  details beside the graph do not say **Not fetched** yet.

- The line of a commit whose parents were not fetched does not end at its dot, as the line of a first commit does: it
  goes on under the dot to the bottom of its row, dashed (Q1062). Its tooltip says **The commits before this one were
  not fetched: this repository is a shallow clone.**
- The inspector of such a commit says **Parents** **Not fetched**, with **Load more history**.
- Once the last page is read, the last row of the timeline says **The history ends here: this is a shallow clone.**,
  with **Load more history** and **Load the whole history** after it. While a filter or a search narrows the timeline,
  the row is there too, since what was not fetched was not searched.
- A repository that is not shallow, or no longer is, has none of these.

Acceptance:

- [ ] In a clone with `--depth 2` of a history of five commits, the timeline lists two commits; the line of the older
      goes on under its dot, dashed, and the last row says **The history ends here: this is a shallow clone.** with
      its two buttons.
- [ ] The first commit of a repository cloned whole ends at its dot, and no row is under it.
- [ ] **Load more history** on the last row runs `sync.deepen`, **Load the whole history** runs `sync.unshallow`.
- [ ] Selecting the older commit shows **Not fetched** for its parents in the inspector.
- [ ] The snapshots of the end of a shallow history match in the dark and in the light theme.

### `graph.lanes` — Lay out the lines of the graph

Where each commit's dot goes, and the lines between commits, row by row.

- **Kind**: Derivation
- **Function**: `model::graph::lanes(rows: &[GraphInput]) -> Vec<GraphCell>`
- **Extension**: `GraphLayout` of `src/shared/graph.ts`, whose `page` lays out a page under the rows before it from the
  lanes under the last of them; `LaidOutRows` of `src/engine/graph.ts` keeps the layout with the rows of a read. Folded
  merges are laid out again whole in the view (`foldMerges`).

Input: the rows in the order listed, newest first, each commit before its parents, each with its hash, its parents and
whether its line is dashed (the uncommitted changes and the stashes). Output, for each row: the lane of its dot, its
color, and the lines of its cell, each going from a lane at the top edge to a lane at the middle (upper half), or from
a lane at the middle to a lane at the bottom edge (lower half), with a color and whether it is dashed.

A lane is free, or waits for a commit further down. The rules:

1. A commit takes the lane that waits for it. When several do, the leftmost one that is not dashed, or else the
   leftmost. A commit that no lane waits for is a tip: it takes the leftmost free lane, or a new one at the right,
   with a new color.
2. Every other lane that waits for the same commit ends at its dot, with a line from that lane into the commit's lane
   in the upper half, and is free from there on. Lanes that wait for other commits go straight through the row, in
   both halves.
3. The first parent goes on in the commit's own lane, straight down, even when another lane already waits for it: the
   two meet at the parent, so that a branch is seen to leave its base. The line down is dashed when the commit's is.
4. Each other parent of a merge goes to the lane that already waits for it, or else to a new lane (the leftmost free
   one, or a new one at the right) with a new color, with a line from the dot to that lane in the lower half.
5. Free lanes at the right end are dropped, so that the graph narrows again after a branch ends.
6. Colors are numbered in the order lanes get them, and a lane keeps its color while it waits. They are drawn from a
   palette of eight that starts again after the eighth.
7. A row given without parents has a dot in a lane of its own and no line down: so are the commits found by a search
   shown as a list, which are not each other's history.

Examples (rows newest first, `X: P Q` meaning X with parents P and Q):

- `C: B`, `B: A`, `A:` gives lane 0 for all three, one color, a line down from C and from B, none from A.
- `F2: F1`, `F1: M1`, `M2: M1`, `M1:` gives F2 and F1 lane 0 (color 0); M2 is a tip and takes lane 1 (color 1); at M1
  both lanes wait for it: M1 takes lane 0, and lane 1 ends at it with a line from lane 1 into lane 0.
- `M: A B`, `B: A`, `A:` gives M lane 0, a line from lane 0 to a new lane 1 (color 1) for B; B takes lane 1; at A,
  lane 0 (from M) and lane 1 (from B) both wait: A takes lane 0, lane 1 ends at it, and lane 1 is dropped after.
- An uncommitted-changes row `W: C` above `C:` gives W lane 0 with a dashed line down; C takes lane 0, since it is
  the only lane waiting for it, and its own line is not dashed.

The layout goes on from where it stopped (D54): `model::graph::lanes_after(start: &LaneState, rows: &[GraphInput]) ->
(Vec<GraphCell>, LaneState)` lays out rows under others from the lanes as they stood under the last of them (which
lanes wait, for which commit, in which color, dashed or not, and the next color), and gives them as they stand under
its own last row. `lanes` is `lanes_after` from no lanes. The timeline keeps the state of its last row, and lays out
only the rows of a page added under it; any other change of the rows lays them out again from the first.

Acceptance:

- [ ] Each example above gives exactly the cells it says.
- [ ] 10,000 rows of a real history laid out a page of 1,000 at a time, each from the state of the page before, give
      exactly the cells of the 10,000 laid out at once; so do the same rows cut after a merge, and between a commit and
      its first parent.
- [ ] A branch merged and deleted leaves its lane free for the next tip, which takes it and a new color.
- [ ] Twenty branches open at once give twenty lanes, and once they end the graph is one lane wide again.
- [ ] A stash on a commit that is also the parent of the next commit of the history: the commit goes on the lane of
      the history, not on the stash's dashed one.

### `graph.fit-width` — How wide the graph is drawn

The graph is as wide as its lines need, within limits, and draws its lines closer before it cuts any.

- **Kind**: Derivation
- **Function**: `model::graph::fit_width(lanes: usize, wanted: Option<f32>, available: f32) -> GraphFit`
- **Uses**: by `graph.view` and `graph.resize-graph`.
- **Extension**: `fitGraph` of `src/shared/graph.ts`. While the commits of the history listed are read again from
  their first page, the most lanes are those the list had before the read, or those of the rows read when they are
  more, so that the graph does not get narrower with the first page and wider again
  ([D72](../08-decisions.md#d72--the-list-of-commits-keeps-its-length-while-its-commits-are-read-again-proposed);
  `held` of `src/webview/pages.ts`); the first read of a history widens the graph as its pages come (Q784). Tested by
  `test/pages.test.ts`, "keeps its length and the width of its graph when Refresh reads its commits again from the
  first page, until they are back".

Input: the most lanes any row of the list has, the width the graph was dragged to (none while it chooses its own), and
the width of the list (0 while it is not known). Output: the width the graph is drawn in, the room each lane gets in
it, whether lanes are cut off at its edge, and its natural width.

- A lane has 14 px at its ease and never less than 8 px; the graph has 6 px of margin. Its natural width is
  `lanes × 14 + 6`.
- Choosing its own width, the graph takes its natural width, but no more than 16 lanes at their ease (230 px) and no
  more than a third of the list.
- Dragged, it takes the width it was dragged to, but no more than its natural width and no more than 60% of the list.
- It is never narrower than one lane at its ease (20 px).
- While a read of the same history has not reached the rows listed before it found other commits, the most lanes are
  those of the rows read or of the rows of before, whichever are more (D72), so that the graph does not get narrower
  with the first page and wider again.
- The lanes get `(width − 6) / lanes`, between 8 and 14 px. When even 8 px each do not fit, the lanes past the edge are
  cut off, and the graph fades out at its edge rather than push the messages away.

Acceptance:

- [ ] 3 lanes in a list of 1,000 px: 48 px, 14 px a lane, nothing cut.
- [ ] 20 lanes in a list of 1,000 px: 230 px, 11.2 px a lane, nothing cut.
- [ ] 40 lanes in a list of 1,000 px: 230 px, 8 px a lane, cut.
- [ ] 20 lanes in a list of 600 px: 200 px (a third), 9.7 px a lane, nothing cut.
- [ ] 10 lanes dragged to 400 px in a list of 1,000 px: 146 px, its natural width.

### `graph.resize-graph` — Give the graph less room or more

Drag the edge of the graph to give it less room or more; double-click it to let the graph choose again.

- **Kind**: View
- **Function**: `views::graph::resize_graph_view(state: &GraphEdgeState, ui) -> Vec<Intent>`
- **Entry points**: the edge between the graph and the messages, in the timeline; with the keyboard on it, `←` and
  `→`.
- **Uses**: `graph.fit-width`.

Dragging sets the width the graph is drawn in, from 20 px up to the width at which every lane has its room and no more
than 60% of the list; `←` and `→` move it by 16 px. A double click forgets the width, and the graph chooses its own
again. The edge stays where it is while the commits scroll. The timeline remembers the width, for the workspace, after
the window is closed, also while it is narrowed to a branch.

Acceptance:

- [ ] Dragging the edge past the natural width of the graph stops at it.
- [ ] A double click on the edge brings back the width the graph chooses.
- [ ] The width dragged in the timeline is there after the window is opened again, and while the timeline is narrowed
      to the branch checked out.

### `graph.resize-panes` — Resize the branch rail and the inspector

Drag the line between the branch rail and the commits, or between the commits and the inspector, to change their
widths.

- **Kind**: View
- **Function**: `views::graph::resize_panes_view(state: &PaneSizes, ui) -> Vec<Intent>`
- **Entry points**: the line between the branch rail and the commits, or between the commits and the inspector; with
  the keyboard on it, the arrows.

| Line | Starts at | Least | Most | Double click |
| --- | --- | --- | --- | --- |
| Side column and commits | 210 px | 120 px | 35% of the window | back to 210 px |
| Under **Repositories**, in the side column | 160 px for **Repositories**, or as tall as its rows | 64 px for each section open | the column less 64 px for each open section under it | back to where it starts |
| Under **Changes**, in the side column | 280 px for **Changes**, or as tall as what it shows | 64 px for each section open | the column less 64 px for **Branches** | back to where it starts |
| Commits and inspector, side by side | 320 px for the inspector | 200 px | 55% of the window | back to 320 px |
| Inspector over the commits, in the narrow layout | half the height for the inspector | 80 px | 80% of the height | back to half |

Dragging away from the branch rail or the inspector makes it larger. The arrows move the line by 16 px. A size larger
than the window has room for is drawn at the most the window allows, and comes back when the window is wider. The sizes
are remembered for the workspace.

Acceptance:

- [ ] The side column cannot be dragged narrower than 120 px nor wider than 35% of the window.
- [ ] Dragging the line under **Repositories** down gives it the height **Branches** loses.
- [ ] Dragging the line under **Changes** down gives it the height **Branches** loses, and it is as high after the
      window opens again.
- [ ] A double click on the line before the inspector gives it 320 px again.
- [ ] The sizes are there after the window is closed and opened again on the same workspace.

### `graph.toggle-references` — Put the branch rail away

The first button of the top row of the timeline puts the side column away, its repositories and its branches, to
leave its room to the commits, and brings it back; the first button of the title row of every page does the same,
from the same place (`repo.side-column`).

- **Kind**: View
- **Function**: `views::graph::toggle_references_view(state: &ReferencesShown, ui) -> Vec<Intent>`
- **Entry points**: the first button of the top row of the timeline, before the branch filter (`filter.picker`),
  pressed while the side column is shown; the first button of the title row of every page, at the same place
  (`repo.side-column`); **Toggle the Side Column** in Quick search, from any page.

Its tooltip says **Hide the repositories and the branches**, or **Show the repositories and the branches**, on the
timeline and on every page (Q541). Whether they are put away is remembered for the workspace. In the narrow layout, where
the side column is put away, the button opens it over the timeline, and a click outside it or `Escape` closes it.

Acceptance:

- [ ] A click hides the side column and the commits take its room; another brings it back at the width it had.
- [ ] A hidden branch rail stays hidden after the window is opened again.
- [ ] The first button of the title row of the settings page, of the Changes page and of a Diff page is where the
      first button of the top row of the timeline is, with the same tooltip, whether the side column is shown or put
      away.

### `graph.labels` — The labels of each commit

Which labels a commit has in the timeline, and what each says.

- **Kind**: Derivation
- **Function**: `model::graph::labels(state: &RepositoryState, settings: &GraphSettings) -> HashMap<Oid, Vec<Label>>`
- **Uses**: `graph.combine-upstream`, `graph.pull-request-badge`.

They start the line under the message of the commit, right after its cell of the graph, before its author and its
date, so that they line up from row to row and leave the message its line
([D86](../08-decisions.md#d86--on-two-lines-the-labels-of-a-commit-start-the-line-under-its-message-proposed)); on a
compact row they come first on its one line, before its message (`graph.compact-rows`). Those that do not fit are
counted in `+N` (`graph.label-overflow`). In this order on each commit:

1. `HEAD`, when nothing is checked out but a commit (detached), filled as what is checked out.
2. Each local branch at it, the one checked out filled. A branch that another worktree has checked out has the icon of
   a worktree before its own, and its tooltip says `In the worktree <folder>`. With its upstream it may be one label
   (`graph.combine-upstream`). Its open pull request ends the label (`graph.pull-request-badge`).
3. Each remote branch at it but `<remote>/HEAD` and those a label of a branch stands for, with the icon of a cloud.
4. Each tag at the commit it points at, an annotated tag too.
5. Each stash made there, `stash@{n}` (`graph.stashes`).
6. During a bisect, `good`, `bad` and `skipped` on the commits marked so (`bisect.`).

Acceptance:

- [ ] Detached at a commit, it has a `HEAD` label, filled.
- [ ] `main` checked out and `v1.0` at the same commit give two labels, `main` filled first.
- [ ] A branch checked out in another worktree has the worktree icon, and its tooltip names that worktree's folder.
- [ ] `origin/HEAD` has no label.
- [ ] An annotated tag labels the commit, not the tag object.

### `graph.label-overflow` — All the labels of a commit, under the pointer

The labels of a commit start the line under its message, or the one line of a compact row, and leave the author and
the date or the message their room; those that do not fit are one label `+N`, and the pointer resting on the labels
opens the list of all of them
([D66](../08-decisions.md#d66--the-labels-of-a-commit-first-on-its-row-the-rest-under-the-pointer),
[D86](../08-decisions.md#d86--on-two-lines-the-labels-of-a-commit-start-the-line-under-its-message-proposed)).

- **Kind**: View
- **Function**: `views::graph::label_overflow_view(state: &LabelOverflow, ui) -> Vec<Intent>`
- **Entry points**: the pointer resting on `+N`, or on any label of a row that does not show all its labels whole.
- **Uses**: `graph.labels`, `graph.hover-history`, `graph.pull-request-badge`, `branch.open`, `branch.checkout`,
  `ui.menu-scroll`.
- **Extension**: planned (Q656): the labels come first on the one line of its wide Graph view already, and under the
  message, before the author and the date, on the two lines of its rows in the side bar, all of them, with no list
  under the pointer yet.

- While the rows are of two lines, the labels start the line under the message, right after the cell of the graph, in
  the order of `graph.labels`, then the author and the date. The author and the date keep their width up to a third of
  the line, and the labels take the rest (Q1231). On a compact row (`graph.compact-rows`) the labels come first on its
  line, then the message, which keeps its width up to three fifths of the line, and the labels take the rest (Q640).
  Either way, as many as fit whole, in their order, then `+N` for the others. When not even the first fits whole beside `+N`, it is shortened with an ellipsis, as long as
  it keeps three times its height; else `+N` alone stands for all of them. A label with a pull request keeps room for
  its badge (`graph.pull-request-badge`). The same holds in the narrow layout (Q644).
- When a row does not show all its labels whole, the pointer resting 300 ms (Q642) on `+N` or on any of its labels
  (Q641) opens under it the list of all the labels of the commit, each on its row with its icon and its color as on the
  row, and the badge of its pull request. It never covers the label it opened from: with no room under it and more
  above, it opens above (Q645). Taller than the room, it scrolls (`ui.menu-scroll`).
- Each label of the list does what it does on the row: a click selects the commit; a double click checks out a branch,
  or does what `branch.open` does for it, and opens the commit as a page on a tag; a right click opens its menu; the
  pointer resting on it brings out its history (`graph.hover-history`). A double click or a menu closes the list
  (Q647).
- The list closes when the pointer leaves it and the labels of the row, on `Escape`, or on a press elsewhere; it does
  not open again until the pointer has left the label it rested on. Rows whose labels are all whole open no list.
- `+N` has no tooltip: the list names them (Q646). For the screen reader, the labels of a row are a list named
  **Labels**, and so is the list under the pointer. From the keyboard, the list is not reached (Q643).

Acceptance:

- [ ] A commit with six labels in a window 1000 pixels wide starts the line under its message with the labels that
      fit, then `+5`, then its author and its date, its labels a list for the screen reader.
- [ ] Resting 300 ms on `+5` opens under it the list of the six labels in their order; resting 250 ms does not.
- [ ] A double click on `main` in the list checks it out, and the list closes.
- [ ] A click on a label of the list selects the commit.
- [ ] A right click on `release` in the list opens the menu of `release`.
- [ ] Resting on a label of the list keeps the history of the commit brought out.
- [ ] Moving the pointer from `+5` into the list keeps it open; leaving both closes it; `Escape` closes it, and it does
      not open again while the pointer stays.
- [ ] In a window wide enough for all six labels, resting on one opens no list.
- [ ] Near the foot of the window, the list opens above `+5`.

### `graph.combine-upstream` — A branch and its upstream on one label

A branch and its upstream of the same name share one label while they are at the same commit; once one of them moves
on, each has its own label again.

- **Kind**: Derivation
- **Function**: `model::graph::combine_upstream(branch: &Branch, remote_branches: &[RemoteBranch]) -> Option<String>`
- **Entry points**: **Put a branch and its upstream on one label** in the menu of the view of the header.
- **Settings**: `graph.combineUpstream` (on by default); off, `feature` and `origin/feature` have a label each.
- **Uses**: `refs.favorite`, `remote.delete-branch`.

A local branch and its upstream are one label when the branch has an upstream that is not gone, the upstream has the
same name on its remote as the branch here (`origin/feature` for `feature`), and the remote branch is at the same
commit as the branch. The label is the branch's: its icon, then the cloud of the remote; and the worktree's icon before
them when another worktree has the branch. Its tooltip adds `Up to date with origin/feature`. The remote branch has no
label of its own.

The context menu of that label is the branch's (`branch.`), and ends with what only the upstream has: **Add
`origin/feature` to favorites** (or **Remove `origin/feature` from favorites**), **Copy `origin/feature`**, and
**Delete `feature` from `origin`** (`remote.delete-branch`).

Acceptance:

- [ ] `feature` and `origin/feature` at the same commit are one label with the branch and cloud icons.
- [ ] After a commit on `feature`, `feature` and `origin/feature` have a label each.
- [ ] `feature` tracking `origin/other` at the same commit has two labels.
- [ ] With `graph.combineUpstream` off, they always have a label each.
- [ ] The menu of a shared label ends with **Copy `origin/feature`**, which copies `origin/feature`.

### `graph.pull-request-badge` — Pull request on a label

Once the pull requests are loaded from GitHub, or the merge requests from GitLab (`hosting.load-merge-requests`), the
label of a branch that has an open one ends with its number, in the color of its checks.

- **Kind**: View
- **Function**: `views::graph::pull_request_badge_view(state: &PullRequestBadge, ui) -> Vec<Intent>`
- **Entry points**: the labels of the timeline, the rows of the branch rail.
- **Uses**: `github.load-pull-requests`, `github.open-pull-request`.

- The badge has the icon of a pull request, a draft one for a draft, and its number, in the color of its checks:
  passed, failed or running; with no checks, in the color of the label.
- Its tooltip is `Pull request #12: Add the parser · checks passed` (`Draft pull request…`, `checks failed`, `checks
  running`), then **Click to open it on GitHub**. On GitLab the number is `!12` and the tooltip `Merge request !12: Add
  the parser · pipeline passed` (`Draft merge request…`, `pipeline failed`, `pipeline running`), then **Click to open
  it on GitLab**.
- A click opens the pull request in the browser, and does not select the commit; a double click on it does nothing
  more.
- A local branch has the pull request of the name it was pushed under (its upstream on a remote on GitHub, not gone). A
  remote branch has it on its own label only when no local branch tracks it.

Acceptance:

- [ ] A branch whose upstream has open pull request #12 with checks failing has `#12` in the color of failure.
- [ ] Clicking the badge opens the pull request and leaves the selection as it was.
- [ ] `origin/feature` with a pull request, tracked by `feature`, has no badge of its own; untracked, it has one.
- [ ] Before the pull requests are loaded, no label has a badge.
- [ ] On GitLab, merge request !12 with its pipeline passed is `!12`, with the tooltip `Merge request !12: Add the
      parser · pipeline passed` and **Click to open it on GitLab**.

### `graph.unpushed` — Commits not pushed yet

The commits of a local branch that are not in its upstream yet, what the next push of that branch sends, have a ring
around their dot.

- **Kind**: Derivation
- **Function**: `model::graph::unpushed(rows: &[Row], branches: &[Branch], remote_branches: &[RemoteBranch]) -> HashMap<Oid, String>`
- **Uses**: by `graph.view`.

For each local branch whose upstream is not gone, that git counts ahead of its upstream (as of the last fetch), and
whose tip is among the rows:

- the commits the branch reaches among the rows, less those its upstream reaches when the upstream's commit is among
  the rows, are its commits not pushed, each with the name of its upstream;
- when the upstream's commit is not among the rows, it is further down than all of them, and the commits the branch
  reaches among the rows are the ones not pushed, but only when they are no more than git counts ahead: more would mean
  the timeline leaves the upstream out (as with **Local**), and then which are pushed is not known, and none is marked;
- a commit that two branches have is marked for the first.

The ring is in the color of the commit's line; its tooltip says `Not pushed to origin/main`.

Acceptance:

- [ ] `main` two commits ahead of `origin/main`: its two newest commits have a ring, and their tooltip says `Not pushed
      to origin/main`.
- [ ] A branch with no upstream has no ring.
- [ ] With the timeline narrowed to **Local**, no commit has a ring.
- [ ] After a push and the read that follows it, the rings are gone.

### `graph.trace-history` — The history of a commit among the rows

Which commits and which parts of the lines belong to the history of a commit, for the timeline to bring them out.

- **Kind**: Derivation
- **Function**: `model::graph::trace_history(rows: &[Row], cells: &[GraphCell], wanted: &Trace) -> Vec<CellTrace>`
- **Uses**: by `graph.hover-history` and `graph.trace-selected`.

The commits wanted are one of these, among the rows loaded:

- what a commit reaches: itself and its parents, and theirs, as far as they are loaded;
- what a commit has that another has not: what the first reaches less what the second reaches; none is known when the
  second is not among the rows;
- what a commit reaches together with what reaches it: the commits made on top of it, as far as they are loaded.

For each row the result says whether its dot is in the history, and for each line of its cell whether it is: a line
belongs to it when the commit it comes down from does and so does the parent it goes to; where two lines run into one
lane, the lane belongs to it if either does.

Acceptance:

- [ ] On `A: B`, `B: C`, `X: C`, `C:`, what `A` reaches is `A`, `B`, `C`, and the line from `X` into `C` is not in it.
- [ ] What `feature` has that `main` has not is its own commits, not the base they share.
- [ ] What `B` reaches together with what reaches it is `A`, `B`, `C`.

### `graph.hover-history` — The history of a label under the pointer

Rest the pointer on the label of a branch, remote branch or tag to see its history; hold `Shift` to see only what it
would bring to the branch checked out.

- **Kind**: View
- **Function**: `views::graph::hover_history_view(state: &HoverState, ui) -> Vec<Intent>`
- **Entry points**: the pointer on a label of the timeline; on a branch, tag, stash or worktree of the branch rail; on
  a sticky branch name (`graph.sticky-branch-names`).
- **Uses**: `graph.trace-history`.

- After the pointer has rested 150 ms on a label, the commits it reaches, as far as the timeline has loaded them, and
  the lines between them keep their colors, and the rest of the timeline fades: the lines and dots it leaves out, and
  the whole of each row it leaves out, its message, labels, author, date and hash, to about a third of their strength,
  as the extension does. Going from one label to another changes
  it at once; leaving the labels brings the timeline back 100 ms later, so that passing over labels does not flash it.
- With `Shift` held on a label of the timeline, only the commits it has that the commit checked out has not stand out:
  what merging it, or a pull request of it, would bring. When the commit checked out is not among the rows, the whole
  history stands out.
- Rows of the branch rail bring out their commit's history in the timeline while it is shown.
- While a label is under the pointer, it takes the place of the history of the selected commit.

Acceptance:

- [ ] Resting on `feature` for 150 ms fades every commit `feature` does not reach, its message and author with its dot.
- [ ] Moving the pointer across labels quickly does not fade the timeline.
- [ ] With `Shift` held on `feature`, only the commits of `feature` not in the commit checked out stay in color.
- [ ] Resting on a branch in the branch rail brings out its history in the timeline.

### `graph.trace-selected` — The history of the selected commit

The commit selected brings out its history, less strongly, for as long as it is selected: the commits it comes from and
the ones made on top of it.

- **Kind**: View
- **Function**: `views::graph::trace_selected_view(state: &SelectedTrace, ui) -> Vec<Intent>`
- **Entry points**: selecting a commit; **Bring out the history of the selected commit** in the menu of the view.
- **Settings**: `graph.traceSelected` (on by default).
- **Uses**: `graph.trace-history`.

The commits the selected one reaches and those that reach it, with the lines between them, keep their colors; the rest
is dimmed, less than under a label: the lines and dots left out, and the whole of each row left out, its message,
labels, author, date and hash, to about two thirds of their strength, as the extension does. It does not apply while
several commits are selected, nor to the draft, and gives way to the history of a label under the pointer until the
pointer leaves it. It stays while the commits are read.

Acceptance:

- [ ] Selecting a commit on `feature` dims the commits of `main` made after `feature` left it: their dots, and their
      messages, labels, authors, dates and hashes, while those `feature` reaches keep their full color.
- [ ] Selecting the draft dims nothing.
- [ ] With two commits selected, nothing is dimmed.
- [ ] Unchecking the entry in the menu of the view sets `graph.traceSelected` to false and nothing is dimmed.

### `graph.columns` — What is shown with each message

Who made each commit and when, on the line under its message after its labels, and its hash at the end of the row.

- **Kind**: View
- **Function**: `views::graph::columns_view(state: &ColumnsState, ui) -> Vec<Intent>`
- **Entry points**: **Show the author**, **Show the date** and **Show the hash** in the menu of the view.
- **Settings**: `graph.columns`: a list of `author`, `date` and `hash`, all three by default; not a list, all three.

- The author is the name git has for the author. The date is when the commit was made, said as `now` under a minute,
  `5m ago`, `3h ago`, `2d ago` under a week, and as a date (`Oct 3, 2026`) after; its tooltip has the full date and
  time. The hash is the first seven characters.
- What is left out gives its room to the messages.
- In the narrow layout of the window, the hash is not shown.
- The draft has none of them.
- With **Compact rows** on, the three are columns at the right end of the one line of each row, after the message,
  each as wide on every row (`graph.compact-rows`); the same switches choose them.

Acceptance:

- [ ] By default each row shows the author, the date and the short hash.
- [ ] Unchecking **Show the date** removes the dates from the line under each message.
- [ ] A commit made 90 minutes ago says `1h ago`, and its tooltip the full date.

### `graph.compact-rows` — Each commit on one line

The timeline as a table: each commit on one line, its labels, its message, then its author, its date and its hash in
columns that line up, on rows half as tall, so that twice as many commits are seen at once
([D67](../08-decisions.md#d67--compact-rows-each-commit-on-one-line)).

- **Kind**: View
- **Function**: `views::graph::compact_rows_view(state: &CompactRow, ui) -> Vec<Intent>`
- **Entry points**: **Compact rows** in the menu of the view (`graph.view-menu`); **Compact rows** in the
  **Timeline** section of the settings.
- **Settings**: `graph.compactRows` (off by default), in the user's settings file.
- **Uses**: `graph.columns`, `graph.labels`, `graph.label-overflow`, `graph.lanes`, `graph.date-separators`,
  `graph.uncommitted-row`, `graph.stashes`.
- **Extension**: planned (Q655): the rows of its wide Graph view are one line with the columns at the right end
  already, and two lines in the side bar, by where the view is and not by a setting.

- On, each row of the timeline is as tall as a row of a list, half the two lines (Q648), and the cell of the graph is
  as tall: the dots and the lines keep their sizes and their lanes, closer from row to row.
- A commit is one line: its labels first, by the rules of `graph.label-overflow`, `+N` and its list included; its
  message, shortened with an ellipsis; the arrow of the fold of a merge; then, at the right end, the author, the date
  and the hash that `graph.columns` asks for, in this order, each from the start of its column. The columns are as wide
  on every row (Q649): the author 120 pixels, a fifth of the line at most, a longer name shortened; the date as wide as
  the longest date the timeline says; the hash seven characters. A column that is not shown gives its room to the
  messages. The date keeps its tooltip, the full date.
- Together the columns take half of the line at most: on a line too short for them the hash leaves first, then the
  author, and the date stays (Q650). In the narrow layout of the window the hash is not shown, as on two lines.
- The draft is one line too: its message, what it changes after it, **Review** and **Commit** as tall as the row lets
  them (Q652). A stash and the row of a fold were one line already. The line of a day, a week or a month
  (`graph.date-separators`) has its name where the messages end, before the columns, where neither the labels nor the
  columns are under it (Q651).
- Turning it on or off draws the rows again without reading them, and so does the setting changed in the file (Q654).
  The row at the top of the list stays at the top, and the commit selected stays in sight when it was (Q653). The
  selection, the search, the history brought out, the folds, the next commits loaded while scrolling and the marks
  beside the scroll bar are as before.

Acceptance:

- [ ] Turning it on writes `"graph.compactRows": true` in the user's settings, and asks no read of the repository.
- [ ] On, each commit is a row as tall as a row of a list; off, as tall as before.
- [ ] On, in a window 1400 pixels wide, the author, the date and the hash of three commits by authors and of dates of
      different lengths each start at the same place on every row, at the right end.
- [ ] On, unchecking **Show the author**, **Show the date** or **Show the hash** takes its column out and leaves the
      other two.
- [ ] On, in a window 1000 pixels wide with the side column and the inspector open, the hash and the author leave and
      the date stays.
- [ ] On, a commit with six labels starts its row with those that fit and `+N`, and resting on `+N` opens their list
      under it.
- [ ] On, the draft and a stash are rows as tall as those of the commits, the draft with **Review** and **Commit** on
      its line.
- [ ] **Compact rows** is in the menu of the view, and choosing it turns it on, then off.
- [ ] Turning it on or off keeps the row at the top of the list at the top, and a commit selected in sight stays in
      sight.

### `graph.date-separators` — Lines between days, weeks and months

A thin line before the first commit of yesterday, of this week, of last week and of each month before has its name at
its end, so that you see how far back you are as you scroll.

- **Kind**: Derivation
- **Function**: `model::graph::date_separators(dates: &[Option<i64>], now: DateTime) -> BTreeMap<usize, String>`
- **Entry points**: **Separate the days, weeks and months** in the menu of the view.
- **Settings**: `graph.dateSeparators` (on by default).
- **Extension**: `datePeriods` of `src/webview/util.ts`, over every row each time the rows listed change: how far back
  a commit is, is worked out without its name, which is written for the rows that start a time alone; writing the name
  of the month of each commit took 2.2 s for 56,000 commits, at each page of a read (milestone M25). Tested by
  `test/datePeriods.test.ts`, "writes the name of a month once for each month, not for each commit of a long history".

Each commit's date, in local time, falls in a period, from the nearest: **Today**, **Yesterday**, **This week** (from
Monday, before yesterday), **Last week** (the seven days before this week), then each month by its name and year,
`September 2026`, the current month included for what is before last week. A row gets a separator when its period is
further back than the periods of all the rows above it, and is not the period of the first commit at the top. Rows with
no date (the uncommitted changes) are passed over. A commit older than those under it, as a rebase leaves them, starts
nothing: a period is never named twice.

Acceptance:

- [ ] On Wednesday 11 November 2026, commits of that day, of the 10th, of the 9th, of the 4th and of 20 October give
      separators **Yesterday** before the 10th, **This week** before the 9th, **Last week** before the 4th and
      **October 2026** before 20 October; none before the first.
- [ ] Rows dated May, March and May again, in that order and all in older months (as a rebase leaves them), give
      **March 2026** before the second and nothing before the third.
- [ ] With the setting off, there is no separator.

### `graph.first-parent` — Only the first parent of merges

Follows each branch along its own line only: a merge is one commit on it, without the commits it brought in.

- **Kind**: View
- **Function**: `views::graph::first_parent_view(state: &FirstParentState, ui) -> Vec<Intent>`
- **Entry points**: **Show only the first parent of merges** in the menu of the view.
- **Settings**: `graph.firstParent` (off by default).
- **Uses**: `graph.read-commits`, `graph.read-position`.

On, the commits are read with `--first-parent`, and each merge has its first parent only. Each branch still shown has
its line, down to where it started. Changing it reads the timeline again from one page.

Acceptance:

- [ ] On, a merge of `feature` into `main` is one row on the line of `main`, and the commits of `feature` it merged are
      not listed unless `feature` itself is shown and reaches them.
- [ ] A merge with the setting on has no fold button (`graph.fold-merge`): it brought nothing in.

### `graph.one-lane` — All commits on one line

Every commit of the timeline on one straight line, as a list of the history rather than a picture of its branches
([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).

- **Kind**: Derivation
- **Function**: `model::graph::one_lane(rows: &[GraphInput]) -> Vec<GraphCell>`
- **Entry points**: **Show all commits on one line** in the menu of the view (`graph.view-menu`).
- **Settings**: `graph.oneLane` (off by default).
- **Uses**: in place of `graph.lanes` while it is on.
- **Extension**: the same switch in the **View** menu of the Graph view, `gitardo.graph.oneLane`, which lays out the
  rows of `src/webview/graph` with this function's rules.

On, each row has its dot in the first lane, in the color of the first lane, with a line from the top edge to the dot
when a row is above it and from the dot to the bottom edge when a row is below it; there are no other lanes and no
other lines. The dot of a merge is a ring, as wide as a dot. The line of the uncommitted changes and of a stash is
dashed, as before. The column of the graph is as narrow as one lane. The labels, the search, the selection, the
history brought out by the selected commit and the folds of merges already folded are as before; **Fold all merges**
and **Unfold all merges** are not in the menu while it is on. Turning it on or off draws the rows again without reading
them. A page added under the rows is laid out alone on the same line, as `graph.lanes` lays it out from where the lanes
stopped: the last row above it gets its line down to it (`one_lane_after`).

Acceptance:

- [ ] On, every row of a history with three branches and two merges has its dot in the first lane.
- [ ] On, the dot of a merge is a ring and the dot of any other commit is filled.
- [ ] On, the column of the graph is as wide as one lane.
- [ ] Turning it on writes `"graph.oneLane": true` in the user's settings, and asks no read of the repository.
- [ ] On, **Fold all merges** is not in the menu of the view.
- [ ] On, a page added under the rows is on the one line, as all the rows laid out at once are.

### `graph.fold-rows` — The rows of the timeline with merges folded

The rows with the commits each folded merge brought in put away under one row, and the lines drawn again.

- **Kind**: Derivation
- **Function**: `model::graph::fold_rows(rows: &[Row], folded: &HashSet<Oid>) -> FoldedRows`
- **Uses**: `graph.lanes`; by `graph.fold-merge` and `graph.fold-all`.

- The commits a merge brought in are the commits loaded that its other parents reach and its first parent does not. A
  merge brought nothing in when none is loaded.
- The row that stands for them goes where the first of them was, or under the lowest commit made on top of any of them
  (other than themselves), when that is further down. It has as parents the commits they come from that are listed,
  in the order of the rows, and the lines are drawn again with it in their place (`graph.lanes`).
- A merge whose commits cannot have one row there without being below what they come from stays unfolded.
- A commit put away by a merge folded first is not put away again by another; the result says which merge put away
  each commit.
- The row says how many commits it holds, and of which branch: the branch or remote branch that labels the merge's
  second parent, or else the name in the message of the merge (`Merge branch 'x'`, `Merge remote-tracking branch
  'x'`, `Merge pull request #n from x`): `3 commits of feature`, or `1 commit merged` with neither.

Acceptance:

- [ ] Folding a merge of three commits of `feature` gives one row `3 commits of feature` where the first of them was,
      and the merge's line goes to it.
- [ ] A merge whose branch had a commit made on top of a merged commit further down keeps all of them under that
      commit.
- [ ] A merge whose second parent is already in the first parent's history brought nothing in and cannot be folded.
- [ ] A merge with the message `Merge pull request #7 from octo/fix` and no label at its second parent says `of
      octo/fix`.

### `graph.fold-merge` — Fold the commits a merge brought in

Puts the commits that a merge brought in under one row, and shows them again.

- **Kind**: View
- **Function**: `views::graph::fold_merge_view(state: &FoldsState, ui) -> Vec<Intent>`
- **Entry points**: **Fold the commits it merged into one row** in the context menu of a merge; the arrow after its
  message, shown while the pointer is on the row or the merge is selected; `←` on a selected merge. To unfold: a click
  on the folded row, on the arrow of the merge, `→` on the merge, or going to one of its commits from elsewhere
  (`graph.show-in-graph`, a search step).
- **Offered when**: the merge brought in commits among those loaded (`graph.fold-rows`), and the graph has its lines:
  not among what a search finds shown as a list.
- **Uses**: `graph.fold-rows`.

- The folded row has a hollow dashed dot, the icon of unfolding and its text; its tooltip says **Click to show them
  again**. It cannot be selected; the arrows pass over it; it has no context menu.
- The arrow of the merge says **Fold the commits it merged into one row**, or **Show the commits it merged again**.
- The timeline keeps the folds of each repository, wide or narrow, after a page took its place and after the window
  is closed: for the 20 repositories shown last, and the 200 merges of each folded or unfolded last.
- A merge folded that is no longer in the history, after a rebase or a reset, is forgotten once the whole history is
  loaded with nothing narrowing it.

Acceptance:

- [ ] **Fold the commits it merged into one row** on a merge of three commits gives a row `3 commits of feature`.
- [ ] A click on that row shows them again.
- [ ] Going to one of the folded commits from the reflog unfolds the merge and selects the commit.
- [ ] The folds are there after the window is closed and opened again.
- [ ] A merge that brought nothing in has no arrow and no entry.
- [ ] With the pointer moved from the row of a merge not selected onto its arrow and resting there ten frames, each
      frame draws the arrow and the row lit, none differing from the one before; a click then folds the merge.

### `graph.fold-all` — Fold all merges

Folds every merge, also the ones loaded later, until it is unchecked.

- **Kind**: View
- **Function**: `views::graph::fold_all_view(state: &FoldsState, ui) -> Vec<Intent>`
- **Entry points**: **Fold all merges** and **Unfold all merges** in the menu of the view.
- **Offered when**: the graph has its lines. **Fold all merges** is greyed out while no merge is loaded and it is not
  checked; **Unfold all merges** while no merge is folded.
- **Uses**: `graph.fold-rows`, `graph.fold-merge`.

Checked, it folds every merge, also those loaded later as the timeline is scrolled or more of the history is read; a
merge unfolded by hand stays unfolded. **Unfold all merges**, or **Fold all merges** again, shows all their commits
again and forgets every fold. Whether all are folded is kept with the folds of the repository.

Acceptance:

- [ ] **Fold all merges** folds every merge loaded, and the merges of the next page are folded when it comes.
- [ ] A merge unfolded by hand while all are folded stays unfolded after the next page.
- [ ] **Unfold all merges** shows every commit again.

### `graph.lane-tips` — The branch each line comes from

For each row, and each line that comes down into it, the row of the tip of the branch that line is the history of.

- **Kind**: Derivation
- **Function**: `model::graph::lane_tips(cells: &[GraphCell], named: &[bool]) -> Vec<Vec<Option<usize>>>`
- **Uses**: by `graph.sticky-branch-names`.

Going down the rows, a line carries the tip it comes from: the first commit up that line that has a name (a branch, a
remote branch, a tag, `HEAD` or a stash; not the marks of a bisect), or a commit nothing comes down to. A commit passes
its own line's tip, or itself when it is named or nothing comes down to it, to its first parent. A line to another
parent of a merge has no tip until a named commit is on it: which branch was merged is not known.

Acceptance:

- [ ] Under the tip of `feature`, every row on its line has the row of `feature` as the tip of that line.
- [ ] The line of a merged branch with no label has no tip until a labeled commit is on it.

### `graph.sticky-branch-names` — Branch names that stay at the top

Scrolled down past the tip of a branch whose line goes on down the graph, its name stays at the top of the list.

- **Kind**: View
- **Function**: `views::graph::sticky_branch_names_view(state: &StickyNames, ui) -> Vec<Intent>`
- **Entry points**: scrolling the list of commits.
- **Uses**: `graph.lane-tips`, `graph.hover-history`.

For each line that comes down into the first row in sight from a tip above it, a name with an arrow up, in the color of
the line, at the top of the list beside the graph, each tip once. Resting the pointer on it brings out the history of
that tip; a click scrolls back up to it, centered, and selects it; its tooltip is `Go up to <name>`. A line with no
tip, as the one of a merged branch, has no name. Nothing is shown at the very top of the list.

Acceptance:

- [ ] Scrolled past the tip of `feature` while its line goes on, `feature` stays at the top in its line's color.
- [ ] Clicking it scrolls to the tip and selects it.
- [ ] At the top of the list there is no sticky name.

### `graph.scroll-marks` — Marks beside the scroll bar

Beside the scroll bar of the timeline, marks say where in the commits loaded the one checked out is, where the one
selected is, and where what a search found is.

- **Kind**: View
- **Function**: `views::graph::scroll_marks_view(state: &ScrollMarks, ui) -> Vec<Intent>`
- **Entry points**: the right edge of the list of commits.

A mark for row `i` of `n` (with one more row while there are more to load) is at `(i + 0.5) / n` of the height of the
list. The commit checked out has a mark in the color of its line, the selected one a mark of its own, and each commit a
search found a third kind; marks of the same kind closer than a pixel are one. They take no clicks.

Acceptance:

- [ ] With 1,000 commits loaded and the one checked out at row 0, its mark is at the top.
- [ ] A search with 14 matches shows their marks, merged where they fall on the same pixel.
- [ ] Selecting a commit moves the mark of the selection to it.

### `graph.keyboard` — Move through the timeline with the keyboard

With the keyboard on the list of commits, the arrows go from commit to commit.

- **Kind**: View
- **Function**: `views::graph::keyboard_view(state: &ListState, ui) -> Vec<Intent>`
- **Entry points**: the list of commits, with the keyboard on it.
- **Uses**: `graph.fold-merge`, `commit-page.open`.

| Key | Does |
| --- | --- |
| `↓`, `↑` | Selects the next or previous commit, passing over folded rows; with nothing selected, the first |
| `Home` | Selects the commit checked out |
| `Alt+↓` | Selects the first parent of the commit selected |
| `Alt+↑` | Selects the commit made on top of it on its line (whose first parent it is), or else the merge that took it in |
| `Shift+Alt+↓`, `Shift+Alt+↑` | Selects the next merge down, or up |
| `←` on a merge | Folds the commits it merged |
| `→` on a folded merge | Shows them again |
| `Enter` | Opens the commit selected as a page, as a double click does |

The row selected is scrolled into sight. `Ctrl+Home` and `Cmd+Home` are left to the list.

Acceptance:

- [ ] `Home` selects the commit checked out from anywhere in the list.
- [ ] `Alt+↓` on a merge selects its first parent, not the commit under it.
- [ ] `Alt+↑` on the last commit of a merged branch selects the merge.
- [ ] `Shift+Alt+↓` selects the next merge down.
- [ ] `Enter` on the draft opens nothing.

### `graph.uncommitted-row` — The draft at the top of the timeline

The changes not committed are the draft, the first card of the timeline, joined to the commit they are made on, also
before the first commit of a new repository.

- **Kind**: View
- **Function**: `views::graph::uncommitted_row_view(state: &UncommittedRow, ui) -> Vec<Intent>`
- **Entry points**: the first card of the timeline; its context menu.
- **Offered when**: there are changes (staged, not staged, untracked or in conflict), nothing narrows the timeline but
  the branches shown, and the commit checked out is among the commits listed (or there is none yet). Narrowed to
  branches that do not have it, the draft is not there, and the Changes page is where one commits.
- **Uses**: `details.show`, `changes.pane`, `commit.create`, `stash.push`, `stash.push-staged`, `changes.stage-all`,
  `changes.unstage-all`, `changes.discard-all`.

- The draft shows the message of the commit box of the Changes page (`changes.pane`) and **Commit**
  (`commit.create`), and nothing else of that box: **Amend last commit**, the button that drafts the message with the
  AI provider and the identity are on the Changes page only. Under them it says `<n> files changed`, `<n>` being the
  number of files with a change of any kind, each once (`1 file changed`), followed by `· <m> staged` while some are
  staged; and **Review**, which opens the Changes page.
  Its dot is a hollow ring joined to the commit checked out by a dashed line; it has no author, date or hash.
- Selected, it lists its files in the inspector, as a commit does (`details.show`). It cannot be selected together with
  commits, opened as a page, dragged or folded.
- Its context menu:
  - **Stash changes…** (`stash.push`), when there is a commit to stash onto;
  - **Stash staged changes…** (`stash.push-staged`), when there is a commit and something is staged;
  - **Stage all changes** (`changes.stage-all`), when something is not staged and no file is in conflict: staging it
    would mark the conflicts resolved, markers and all;
  - **Unstage all changes** (`changes.unstage-all`), when something is staged;
  - **Discard all changes…** (`changes.discard-all`), in the color of danger, but not during a merge, a rebase, a
    cherry-pick, a revert or a patch, which **Abort** takes back.
- When the changes are all committed or discarded, the draft goes and the inspector no longer shows it.

Acceptance:

- [ ] Changing a file adds the draft above the commit checked out, saying `1 file changed`, with a dashed line to it.
- [ ] In a new repository with one untracked file, the draft is the only card, with no line down.
- [ ] Narrowed to a branch that is not checked out, the draft is not there.
- [ ] With a file in conflict, the menu has no **Stage all changes**; during a merge, no **Discard all changes…**.
- [ ] Before the first commit, the menu has no stash entry.

### `graph.place-stashes` — Where the stashes go among the commits

Each stash goes above the commit it was made on, where its date puts it.

- **Kind**: Derivation
- **Function**: `model::graph::place_stashes(commits: &[Row], stashes: &[StashCommit]) -> Vec<Row>`
- **Uses**: by `graph.read-commits`.

- A stash is a commit whose first parent is the commit it was made on; its other parents (the index, and the untracked
  files it took) are left out, as the stash stands for them.
- A stash whose base is not among the commits listed is not shown.
- A stash goes before the first commit listed that is not newer than it, but never under its base. Stashes are placed
  from the newest, so that several made on the same commit are listed as they were made, the newest first.

Acceptance:

- [ ] A stash made yesterday on a commit of last week goes above the commits of today and under the newer ones made
      since, above its base.
- [ ] A stash made on a commit made after it in date order (its clock was behind) goes right above its base.
- [ ] Two stashes on the same commit are listed `stash@{0}` above `stash@{1}`.
- [ ] A stash on a branch the timeline is not narrowed to is not listed.

### `graph.stashes` — Stashes in the timeline

The stashes are in the timeline too, each above the commit it was made on, as long as that commit is in the timeline.

- **Kind**: View
- **Function**: `views::graph::stashes_view(state: &StashRows, ui) -> Vec<Intent>`
- **Entry points**: **Show stashes** in the menu of the view; the context menu of a stash row.
- **Extension**: the stashes in the graph, placed by the engine's read of the graph (`graphState` in
  `src/engine/graph.ts`), each above the commit it was made on, labeled `stash@{N}`; the **Stashes** section of the
  references beside the graph, one entry per stash named by its message, with the tooltip `stash@{N}: <message>`,
  where a click selects its row (the Branches view has no such section); **Show stashes** in the **View** menu of the
  toolbar, ticked while they are shown, which writes `gitardo.graph.showStashes` for the user. The context menu of a
  stash, there and on its row (`stashMenu` in `src/webview/menus.ts`), sends the operations to Gitardo's engine as
  their cards say.
- **Offered when**: the commit the stash was made on is in the timeline: on the branches shown, and not among what a
  search finds, in the history of a file or of a commit on no branch.
- **Settings**: `graph.showStashes` (on by default).
- **Uses**: `graph.place-stashes`, `stash.apply`, `stash.pop`, `stash.drop`, `stash.branch`, `details.show`.

A stash is a square with no fill rather than a dot, joined to its commit by a dashed line; its label, `stash@{0}`, has a
dotted border, and its message is in italics. Selected, the inspector shows what it holds, the untracked files it took
included. Its context menu applies, pops, drops it or makes a branch of it, as in the Stashes list of the branch rail.
It cannot be selected together with commits, dragged, or be where a commit is dropped.

Acceptance:

- [ ] `git stash` in a terminal adds a square above the commit checked out, labeled `stash@{0}`.
- [ ] Unchecking **Show stashes** takes them out and sets `graph.showStashes` to false.
- [ ] The context menu of a stash row has apply, pop, drop and create branch, and nothing of a commit.
- [ ] While the timeline shows the history of a file, no stash is in it.
- [ ] Extension: a stash made on `main` is a row above the commit of `main`, with `stash@{0}` and a single parent, that
      commit, joined to it by a dashed line.
- [ ] Extension: it is not in the graph narrowed to a branch that does not have its commit, nor among what a search for
      its message finds, nor with `gitardo.graph.showStashes` set to `false`.
- [ ] Extension: the **Stashes** section of the references lists each stash by its message, and a click on one selects
      its row in the graph.

### `graph.show-in-graph` — Go to a commit in the timeline

Opens the timeline at a commit: loads the history up to it when it is further back, and shows it when the timeline
leaves it out.

- **Kind**: View
- **Function**: `app::store::search::go_to(state, commit) -> Vec<Effect>`, which every way to a commit calls: it draws
  nothing of its own, and the bar it leaves is the one of `filter.bar`
- **Entry points**: **Show in the timeline** in the menu of a branch, tag or worktree in the branch rail (the
  worktree's commit); a click on a tag, a stash or a worktree in the branch rail; and, from other areas, a step of a
  search, **Go to commit…**, the reflog, the page of a commit (`commit-page.show-in-graph`), the bisect banner.
- **Reads**: `graph.read-position`.
- **Uses**: `graph.read-commits`, `graph.fold-merge`, `filter.log-arguments`.

- A commit among the rows loaded is selected and scrolled to the middle of the list. A commit inside a folded merge
  unfolds it first.
- Otherwise its position in the history listed is read; when the timeline is narrowed by a search shown as a list, the
  history of a file or of some lines, and the commit is not in it, that is taken away; when it is still not there and
  the timeline is narrowed to some branches, all of them are shown again; when it is still not there (no branch or tag
  reaches it, such as a commit lost to a reset), the timeline shows the history of that commit alone, and the bar says
  `Only the history of 3f2a1c9 · on no branch or tag` with **Show all commits**.
- The history is loaded up to the page that has it, `position / page size + 1` pages, and the commit is selected and
  scrolled to the middle.
- The timeline comes back in place of the page that asked, and selects it.

Acceptance:

- [ ] **Show in the timeline** on a tag 3,500 commits back loads four pages and selects the tag's commit.
- [ ] **Show in the timeline** on a branch the timeline is not narrowed to shows all branches again and selects its
      commit.
- [ ] Going to a commit found in the reflog that no branch reaches shows its history alone, until **Show all
      commits**.
- [ ] Going to a commit of the history listed while the timeline shows the history of a file keeps that history when
      the commit is in it.

### `refs.tree` — The branch rail

The legend and the filter of the timeline: the local branches with how far each is from its upstream, each in the
color of its lane, and the branches of each remote under a heading of its own; then, kept aside, the stashes, the
worktrees, the tags and the submodules: in lists that fold, on the left of the window.

- **Kind**: View
- **Function**: `views::refs::tree_view(state: &ReferencesState, ui) -> Vec<Intent>`
- **Entry points**: the left of the window; it can be put away (`graph.toggle-references`), and in the narrow layout
  it is narrowed to its icons.
- **Uses**: `refs.filter`, `refs.scope-line`, `refs.sort`, `refs.recent`, `refs.favorite`, `filter.show-only`,
  `filter.show-all`, `graph.show-in-graph`,
  `graph.hover-history`, `graph.pull-request-badge`, `branch.open`, `drag.drop`, `tag.remote-marks`, `branch.`, `tag.`,
  `stash.`, `worktree.`, `submodule.`.
- **Extension**: the references beside the graph and in the Branches view (`RefsTree` in `src/webview/RefsTree.tsx`).
  They list what their own filter lists (`refs.listed`), whatever the graph is narrowed to, the branches it leaves out
  dimmed (`filter.list-too`). A click on a branch selects its tip in the graph and narrows nothing, as it did before;
  **Show only this history** in its menu narrows the graph, and the references stay as they were. Tested by
  `test/e2e/ui.mjs`, step "the branches list the favorites and the ones checked out last first, all of them whatever
  the graph is narrowed to, what their own filter chooses, and can be put away; the status bar opens the graph".

- Lists, in this order: **Favorites** and **Recent** (each only when it has something), **Local**, then one list for
  each remote, under a heading that is the remote's name (**ORIGIN**, **UPSTREAM**), with its branches; then **Kept
  aside**: **Stashes**, **Worktrees**, **Tags**, **Submodules** (only when there are some). Each has its icon and the
  count of what it lists.
- The lists of **Kept aside** are closed when Gitardo starts, and the others (the local branches, the favorites, the
  recent branches and the list of each remote) open, since the local branches are the legend of the lanes; the ones
  opened or closed stay so until the application is closed.
- A list with more than 100 entries lists the first 100 and **Show all `<n>`…**, which lists them all until the
  list is closed. The branch checked out is listed even when it comes after the first 100.
- A local branch says how far it is from its upstream, `↑1 ↓2`, or `gone` when its upstream was deleted; its tooltip
  says `main → origin/main` or `main (no upstream)`, the worktree that has it with what is not committed there, and its
  pull request. A branch that another worktree has has the worktree icon, and the number of files not committed there.
  A favorite has a star.
- A stash is listed by its message, with `stash@{n}: <message>` in its tooltip. A worktree, by its folder (its path
  from where new worktrees go, or the name of its folder), with its branch and how far it is from its upstream, or
  `detached`, or `missing`; a submodule, by its path, with a badge of how it stands against its recorded commit (see
  `submodule.`).
- The branch checked out, and the worktree of this window, are marked as current; the references at the selected
  commit are marked as selected.
- A click on a branch or a remote branch narrows the timeline to its history and selects its tip
  (`filter.show-only`); a second click on it brings the whole history back (`filter.show-all`). The rail itself does
  not change: it keeps listing what its own filter lists (`refs.list-filter`), the branches the timeline leaves out
  dimmed (`filter.list-too`). A click on a tag, stash or worktree still goes to its commit (`graph.show-in-graph`),
  without narrowing anything. A double click on a branch
  opens it as `branch.open` says; on a remote branch, checks it out (`branch.open`); on a worktree or a submodule,
  opens it in a new window. Resting the pointer on one brings out its history (`graph.hover-history`). Branches can be
  dragged onto the timeline (`drag.drop`).
- A right click opens the menu of the reference (`branch.`, `remote.`, `tag.`, `stash.`, `worktree.`, `submodule.`); a
  right click on the title of a list, or its **…**, opens the list's own menu. The Worktrees list has **Create
  worktree…** on its title and **…** with what concerns worktrees; the Submodules list **Add submodule…**; the Tags
  list, in a repository with a remote, **Check tags on `<remote>`** and **Push tags…** after the order of its tags.
- Once a remote was checked, a tag it lacks has the mark of `tag.remote-marks` after its name and in its tooltip, and
  the count of the Tags list says how many it lacks, `24 · 2 not on origin`.

Acceptance:

- [ ] After the application starts, every list of **Kept aside** is closed and the others are open; one opened stays
      open when another repository is chosen.
- [ ] A list of 250 tags lists 100 and **Show all 250…**.
- [ ] A branch one ahead and two behind its upstream says `↑1 ↓2`; one whose upstream is gone says `gone`.
- [ ] Clicking a stash in the branch rail selects the stash in the timeline.
- [ ] The checked-out branch, 150th by name, is listed after the first 100.
- [ ] A click on `feature` narrows the timeline to its history, and the rail still lists `main` and every other local
      branch, `main` dimmed.
- [ ] With the pointer moved from the title of **Local** onto its **…** and resting there ten frames, each frame draws
      the **…** and the title lit, none differing from the one before; a click then opens the menu of the list.

### `refs.filter` — Filter the branch rail by name

From 15 branches and tags on, a filter box appears at the top of the branch rail, which lists the names that match best
first.

- **Kind**: View
- **Function**: `views::refs::filter_view(state: &ReferencesFilter, ui) -> Vec<Intent>`
- **Entry points**: the box at the top of the branch rail.
- **Offered when**: the lists there have 15 names or more together (local branches, remote branches, tags and
  stashes), or something is typed in it.
- **Uses**: `filter.rank-names`.

What is typed shows, in place of the lists, one list of the entries of every list whose names have it, ignoring case,
the best matches first (`filter.rank-names`), under the title **MATCHES** with the count `<matches> of <all>`; among
names that match as well, they keep the order of the lists. **Favorites** and **Recent**, whose branches are among the
local ones, give none. An entry keeps the row and the menu of its list, so that a match of a list closed, such as
**Tags**, is shown too; once the box is emptied, the lists are as they were. `Escape` or the button at the end of the
box empties it.

Acceptance:

- [ ] With 14 branches and no tag, there is no box; with 15, there is.
- [ ] Typing `main` lists `main`, then `origin/main`, then `maintenance`, then `release/main-2`, then `domain`.
- [ ] A match of a closed list is shown while the box has text, and the list is closed again when it is emptied.
- [ ] `Escape` empties the box.

### `refs.scope-line` — The line of what the branch rail lists

While the filter of the branch rail lists less than everything, a line at the top of the rail says what it lists.

- **Kind**: View
- **Function**: `views::refs::scope_line_view(state: &ScopeLine, ui) -> Vec<Intent>`
- **Entry points**: the top of the branch rail, while its own filter lists less than everything (`refs.list-filter`).
- **Uses**: `refs.list-filter`, `filter.label`.
- **Extension**: the line above the names of the references, with the funnel's label (`scopeLabel` of
  `src/shared/scope.ts`) and **Show all**, which sends `setListed` with nothing; what the graph is narrowed to is said in
  the bar above its commits. Tested by `test/e2e/ui.mjs`, step "the branches list the favorites and the ones checked out last first, all of them whatever
  the graph is narrowed to, what their own filter chooses, and can be put away; the status bar opens the graph".

The line says what the rail lists, as the button of its filter does (`Local`, `main +2`), and its **Show all** lists
everything again: the filter of the rail goes back to all. What the timeline is narrowed to is never said here: the
bar above the timeline says it (`filter.bar`). Stashes, worktrees and submodules are always listed.

Acceptance:

- [ ] With the filter of the rail on `main` and `feature`, the Local list has only those two, under a line saying
      `main +1`.
- [ ] **Show all** on the line takes it away and lists everything, and the timeline shows what it showed.
- [ ] Narrowing the timeline to `main` shows no line, and the rail lists every branch.

### `refs.list-filter` — Choose what the branch rail lists

The branch rail's own filter: which branches, remote branches and tags it lists, apart from what the timeline shows.

- **Kind**: View
- **Function**: `views::refs::list_filter_view(state: &ListFilterState, ui) -> Vec<Intent>`
- **Entry points**: the button with the funnel at the end of the heading of **Branches** in the side column
  (`branch.pane`), and in the narrow layout at the top of the icons of the rail; **Filter the Branch Rail** in Quick search.
- **Uses**: `filter.picker`, `filter.preset`, `filter.tick`, `filter.kinds`, `filter.rank-names`, `filter.label`,
  `refs.listed`, `refs.scope-line`.
- **Extension**: the funnel at the end of the top row of the references, after the box that finds a name, beside the
  graph and in the Branches view (`ListFilterButton` in `src/webview/ScopeButton.tsx`). It opens the list of the
  branches shown in the graph, titled **Branches listed**, over the references alone, with the three check boxes at its
  bottom, greyed out with **Chosen in the filter** as Q416 says. Its tooltip is `Branches listed here: <label>`. No
  command, as the branches shown have none. Tested by `test/e2e/ui.mjs`, step "the branches list the favorites and the ones checked out last first, all of them whatever
  the graph is narrowed to, what their own filter chooses, and can be put away; the status bar opens the graph"; `test/e2e/suite.ts`, step "lists in the references what their own filter chooses, kept for each repository, and
  the branches shown in the graph and it never change each other"; `test/scope.test.ts`, "refs.list-filter: the
  kinds listed are switched one at a time, all three are everything, and the last one stays".

The button opens the picker of the timeline's branch filter (`filter.picker`) titled **Branches listed**, over the rail
alone: the presets **All**, **Current**, **Upstream**, **Local** and **Remotes**, the box that finds a branch, the
branches ticked one by one, and **Show local branches**, **Show remote branches** and **Show tags**. What is chosen
changes only what the rail lists, as `refs.listed` says: the timeline keeps what it shows, and what is chosen in the
timeline's filter changes nothing here. The button is pressed in while the rail lists less than everything, its tooltip
says what it lists (`filter.label`), and so does the line at the top of the rail (`refs.scope-line`).

Acceptance:

- [ ] **Local** in the filter of the rail lists no remote branch in the rail, and the timeline still shows every branch.
- [ ] Ticking `main` and `feature` lists only those two among the local branches, and the timeline is not narrowed.
- [ ] Narrowing the timeline to `main` from its branch filter leaves the filter of the rail as it was, and the rail
      lists every branch.
- [ ] The button is pressed in while the rail lists less than everything, and its tooltip says what it lists.

### `refs.listed` — Which references the branch rail lists

What the branch rail lists from the choice of its own filter, and that choice kept for each repository.

- **Kind**: Derivation
- **Function**: `model::filter::rail::listed(choice: &Choice, references: &References) -> Listed`
- **Uses**: `filter.preset`, `filter.tick`, `filter.kinds`, `filter.remember`.
- **Extension**: `listedRefs` of `src/shared/scope.ts`, the rules of `model::filter::rail::listed` ported to
  TypeScript, over the `Scope` the branches shown are made of. The choice is kept for each repository in the workspace
  state under `gitardo.branchesListed` (`Controller.setListed`), and is sent to the views as `RepoState.listed` without
  the references gone since (`Controller.listedIn`), which is not written again then (Q414). Tested by
  `test/scope.test.ts`, the tests that start "refs.listed:"; `test/e2e/suite.ts`, step "lists in the references what their own filter chooses, kept for each repository, and
  the branches shown in the graph and it never change each other".

- **All** lists every branch, remote branch and tag; **Current** the branch checked out and its upstream; **Upstream**
  the branch checked out, its upstream and the branch it was made from when git says so, as `filter.preset` says;
  **Local** the local branches; **Remotes** the remote branches; the branches ticked, those alone, among the kinds shown.
- A branch ticked that is gone is no longer listed, and the others still are.
- The choice is remembered for each repository, apart from the timeline's (`filter.remember`), in the window's stored
  state, and is the one in use when the repository is shown again or the window opens again.
- The stashes, the worktrees and the submodules are always listed; **Favorites** and **Recent** list only the branches
  the choice lists.

Acceptance:

- [ ] **All** lists every branch, remote branch and tag.
- [ ] **Current** lists the branch checked out and its upstream.
- [ ] With `main` and `old` ticked and `old` deleted, the rail lists `main`.
- [ ] The choice of `api` is in use when the window opens again, and `web` keeps its own.

### `refs.sort` — Sort the branches and tags

Branches, remote branches and tags are listed by name, or the one committed to last first.

- **Kind**: Derivation
- **Function**: `model::refs::sort(references: &References, order: SortOrder) -> References`
- **Entry points**: **Sort by name** and **Sort by date** in the **…** on the title of the Local list, of the list of
  each remote and of the Tags list, checked as they are.
- **Extension**: **Sort by name** and **Sort by date** in the **…** on the title of the **Local**, **Remotes** and
  **Tags** sections of the references tree and of the Branches view, the one in use ticked. The order is the setting
  `gitardo.branches.sortOrder`, written to the user settings; while it is not set, VS Code's `git.branchSortOrder` set
  to `committerdate` means `date`. `sortRefs` in `src/shared/refs.ts` sorts, in TypeScript, the references the engine
  read.
- **Settings**: `branches.sortOrder`: `name` or `date`; not set, by name, or by date when Git's own `branch.sort` is
  `-committerdate` or `committerdate` (`model::refs::order_in_force`).
- **Reads**: Git's `branch.sort`, with the state of the repository (`repo.read-state`).

- By name is the order git lists them in.
- By date, the latest first: the date of the commit, or for a tag made with a message, when it was made; among the
  same date, by name.
- The order is the same for all three, in the branch rail and in every picker a branch or a tag is chosen from. Choosing
  one writes the setting in the user's settings file.

Acceptance:

- [ ] By date, a branch committed to an hour ago comes before one committed to yesterday.
- [ ] An annotated tag made today on an old commit comes first among the tags by date.
- [ ] **Sort by date** in the Tags list also orders the Local list and the branch picker of `branch.create`.
- [ ] Extension: **Sort by date** in the menu of **Local** sets `gitardo.branches.sortOrder` to `date` in the user
      settings, and the Local, Remotes and Tags sections, the Branches view and the pickers of branches list the latest
      first.
- [ ] Extension: with the setting not set and `git.branchSortOrder` set to `alphabetically`, they are listed by name.

### `refs.recent` — The branches checked out last

**Recent** lists the local branches that were checked out last, the latest first and up to five, without the one that
is checked out now: where you were before is one double click away.

- **Kind**: Derivation
- **Function**: `model::refs::recent(moves: &[String], branches: &[String], current: Option<&str>) -> Vec<String>`
- **Reads**: the last 300 moves of `HEAD`, from `repo.read-state`.
- **Uses**: by `refs.tree`.

Each move `checkout: moving from <a> to <b>`, the latest first, gives `<b>` then `<a>`: arrived at before it was left
for the other. A name is kept when it is a local branch that still exists, is not the one checked out, and is not
already kept; other moves are passed over. It stops at five.

Acceptance:

- [ ] After checking out `a`, then `b`, then `c`, **Recent** lists `b`, then `a`.
- [ ] A branch deleted since it was checked out is not listed.
- [ ] Moves to a commit (detached) are passed over.
- [ ] After eight branches checked out one after the other, five are listed.

### `refs.favorite` — Add a branch to favorites

**Favorites** has the branches marked with **Add to favorites**, local or remote; they carry a star where they are
listed with the others.

- **Kind**: View
- **Function**: `views::refs::favorite_view(state: &Favorites, ui) -> Vec<Intent>`
- **Entry points**: **Add to favorites** and **Remove from favorites** in the context menu of a branch or a remote
  branch, in the branch rail and on labels of the timeline; on a label shared with its upstream,
  **Add `origin/feature` to favorites** too (`graph.combine-upstream`).
- **Extension**: **Add to favorites** and **Remove from favorites** in the context menu of a local or a remote branch,
  and **Add `<remote branch>` to favorites** under the entries of a label that stands for its upstream too; the full
  names of each repository are kept in the workspace state under `gitardo.favorites` (`Controller.setFavorite`), and the
  **Favorites** sections of the references tree and of the Branches view list them. No git command runs.
- **Uses**: by `refs.tree`.

The favorites are kept for each repository of the workspace, by the full names of the branches, in Gitardo's storage,
and survive the window being closed. They are listed in the order they were added. A branch that no longer exists is
not listed, and is forgotten the next time the favorites change. Nothing of the repository changes.

Acceptance:

- [ ] **Add to favorites** on `feature` lists it under **Favorites** and puts a star on it under **Local**.
- [ ] **Remove from favorites** takes it out; with none left, the Favorites list is not there.
- [ ] The favorites of one repository are not those of another of the same workspace.
- [ ] A favorite deleted with `git branch -D` is no longer listed.
- [ ] Extension: **Add to favorites** on `feature` and on `origin/main` makes the favorites `refs/heads/feature`,
      `refs/remotes/origin/main`, in that order, and the views are told without the repository being read again.
