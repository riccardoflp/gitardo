# Graph

The timeline: the commits of every branch, remote branch and tag with their lines and labels, read a page at a time
while scrolling; how the lines are laid out; what stands out under the pointer and around the selected commit; the
merges folded into one row; the draft of the uncommitted changes and the stashes among the commits; the header above
them; and the branch rail beside them, with its lists, its filter box, the recent and favorite branches and the way to a
reference's commit. The search is in [03-search-and-filter.md](03-search-and-filter.md) (`search.`), and so are the
branches the timeline is narrowed to (`filter.`); the selected commit in the inspector is `details.`, several commits
selected together `selection.`, drag and drop `drag.`, fetch, pull and push `sync.`, and what is done to a stash
`stash.`.

### `graph.read-commits` — Read a page of the graph

The commits the timeline lists, newest first, with their parents: as many pages as were asked for.

- **Kind**: Read
- **Function**: `git::read::graph::read_commits(git, request: &CommitsRequest) -> Result<CommitPage, GitError>`
- **Reads**:
  - `git log -z --format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%s --max-count=<count + 1> --date-order <selection>`,
    where `<count>` is the page size times the pages asked for, and `<selection>` the arguments that say which history
    is listed (`filter.log-arguments`), followed by those of a search shown as a list (`search.read-matches`) or of the
    history of a file, a folder or some lines (`file-history.read-names`). One commit more than asked for says whether there
    are more.
  - With the setting `graph.firstParent`, `--first-parent` before the selection, and each commit keeps only its first
    parent.
  - With `graph.showStashes`, stashes in the repository and nothing narrowing the history but the branches shown:
    `git log --no-walk=unsorted -z --format=<the same> <the hash of each stash> --`, for the stashes to be placed among
    the commits by `graph.place-stashes`. `refs/stash` itself is never part of the selection: the commits of a stash
    would clutter the graph.
- **Settings**: `graph.pageSize` (1,000 by default; a number under 100 counts as 100, over 10,000 as 10,000, and what
  is not a number as 1,000); `graph.firstParent`; `graph.showStashes`.
- **Uses**: `filter.log-arguments`, `graph.place-stashes`, `graph.lanes`.

Acceptance:

- [ ] In a repository of 2,500 commits, one page reads 1,000 commits and says there are more; three pages read 2,500
      and say there are none.
- [ ] With `graph.pageSize` set to 50, a page is 100 commits; set to `"abc"`, 1,000.
- [ ] A merge of three parents is read with its three parents, and with `graph.firstParent` with its first only.
- [ ] Commit messages with a tab, a line break in the body, or non-ASCII characters are read whole: the subject is the
      first line.
- [ ] With `graph.showStashes` off, no stash commit is read.

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
- **Uses**: `graph.toolbar`, `refs.tree`, `graph.lanes`, `graph.fit-width`, `graph.labels`, `graph.columns`,
  `graph.load-more`, `graph.uncommitted-row`, `graph.stashes`, `graph.hover-history`, `graph.trace-selected`,
  `graph.fold-merge`, `graph.sticky-branch-names`, `graph.scroll-marks`, `graph.keyboard`, `graph.resize-panes`,
  `filter.bar`, `details.show`, `selection.select`, `drag.drop`, `commit-page.open`, `branch.open`.

- From left to right: the branch rail (`refs.tree`), the commits, the inspector of the selected commit (`details.show`).
  When the window is too narrow to have them side by side, the branch rail is narrowed to its icons and the inspector
  opens over the commits. There, a commit with more than three labels names two of them, what is checked out first, and
  counts the others, `+3`, with their names in the tooltip of the count.
- Above the commits, in this order: the banner of a worktree (`worktree.`), of an operation stopped half way
  (`conflicts.`, `rebase.`), of a bisect (`bisect.`), the bar of what the timeline is narrowed to (`filter.bar`) and
  the bar of several commits selected (`selection.select`). The draft of the uncommitted changes is the first card of
  the commits (`graph.uncommitted-row`).
- Each row has its cell of the graph (`graph.lanes`): a dot for a commit, in the color of its line. Then two lines: its
  labels (`graph.labels`) and its message, then the columns `graph.columns` asks for. The labels are outlined in the
  color of the commit's line; the branch checked out is the one filled in.
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
- [ ] Narrowing the window under the width of the branch rail, the commits and the inspector narrows the branch rail
      to its icons and opens the inspector over the commits.
- [ ] A double click on a commit opens its page; on the label of a branch that is not checked out, checks it out.
- [ ] In a new repository with no commits and no changes, the list says **No commits yet**.
- [ ] Scrolling a list of 50,000 commits draws only the rows in sight.

### `graph.toolbar` — The header

What applies to the whole repository, at the top of the window.

- **Kind**: View
- **Function**: `views::graph::toolbar_view(state: &ToolbarState, ui) -> Vec<Intent>`
- **Entry points**: the top of the window.
- **Uses**: `graph.toggle-references`, `repo.select`, `sync.fetch`, `sync.pull`, `sync.push`, `branch.create`,
  `stash.push`, `undo.last`, `filter.picker`, `search.box`, `repo.schedule-reads`, `graph.more-menu`.

From left to right: the button that puts the branch rail away (`graph.toggle-references`); the repository menu, which
names the repository shown and lists the others of the workspace (`repo.select`); the branch checked out, or `detached
at 3f2a1c9`, or `no commits yet`, with how far it is from its upstream (`activity.status-bar`) and the folder of the
repository in its tooltip; Quick search; the activity indicator (`activity.status-bar`); **Sync** with the number of
commits to pull and to push, `↓3 ↑2`, and its menu: **Fetch**, **Pull** and its other ways, **Push** and its other
ways; **Activity**, which opens the Activity drawer; and **…** (`graph.more-menu`), which has **Refresh**, **Branch**,
which creates a branch at `HEAD`, **Stash**, which stashes all changes, and **Undo**. The button of the branches shown
(`filter.picker`) and the search (`search.box`) are at the top of the timeline. Each entry says in its tooltip what it
runs: **Fetch: git fetch --all --prune**, **Push: git push**, **Stash: git stash push --include-untracked**. When the
window is narrow the header keeps its icons and gives up its words. While an operation runs, the entries that change
something are disabled. **Branch** and **Undo** are disabled before the first commit, and **Stash** also while there is
nothing to stash.

Acceptance:

- [ ] With `main` two ahead of its upstream and three behind, **Sync** shows `↓3 ↑2`.
- [ ] Detached at a commit, the header names `detached at <short hash>`.
- [ ] While a fetch runs, **Pull** and **Push** in the menu of **Sync** cannot be clicked.
- [ ] Narrowing the window hides the words of the header and keeps its icons; **Refresh** stays in **…**.

### `graph.more-menu` — The … menu of the header

The switches of the timeline and the ways to what is not elsewhere in the header.

- **Kind**: View
- **Function**: `views::graph::more_menu_view(state: &MoreMenuState, ui) -> Vec<Intent>`
- **Entry points**: **…** at the end of the header.

In this order, separated in groups:

1. **Refresh** (`repo.schedule-reads`); **Branch** (`branch.create` at `HEAD`), **Stash** (`stash.push`), **Undo**
   (`undo.last`).
2. **Show local branches**, **Show remote branches**, **Show tags** (`filter.kinds`); **Show stashes**
   (`graph.stashes`); **Show only the first parent of merges** (`graph.first-parent`); **Bring out the history of the
   selected commit** (`graph.trace-selected`); **Fold all merges** and **Unfold all merges** (`graph.fold-all`), while
   the graph has its lines.
3. **Show the author**, **Show the date**, **Show the hash** (`graph.columns`); **Separate the days, weeks and months**
   (`graph.date-separators`); **Put a branch and its upstream on one label** (`graph.combine-upstream`).
4. **Compare branches, tags or commits…** (`compare.choose`), **Go to commit…** (`search.go-to-commit`), **History of
   the active file** (`file-history.file`), **Browse reflog…** (`reflog.browse`), **Clean up branches…**
   (`branch.clean-up`), **Apply patch…** (`patch.apply`), **Manage remotes…** (`remote.manage`), **Add submodule…**
   (`submodule.add`) while the repository has no submodule (the Submodules list has it otherwise), **Select
   repository…** (`repo.select`) when it is offered.
5. **Switch identity…** (`identity.switch`).
6. **Load pull requests from GitHub** (`github.load-pull-requests`), when the main remote is on GitHub.

The switches are checked while they are on, and write their setting in the user's settings file, which every window
follows.

Acceptance:

- [ ] Every entry above is in the menu, in that order, with its condition.
- [ ] Unchecking **Show the hash** takes the hashes out of the timeline and sets `graph.columns` to `["author", "date"]`
      in the user's settings.
- [ ] **Add submodule…** is not in the menu of a repository with submodules.

### `graph.load-more` — Load the next commits while scrolling

The commits are read a page at a time, the next page long before the end of those shown, so that scrolling does not
stop to wait for them.

- **Kind**: View
- **Function**: `views::graph::load_more_view(state: &ListState, ui) -> Vec<Intent>`
- **Entry points**: scrolling the list of commits.
- **Settings**: `graph.pageSize`.
- **Uses**: `graph.read-commits`.

- The next page is asked for as soon as the last row drawn is 500 rows or fewer from the end of the commits loaded,
  and once for each number of commits loaded: with small pages, one follows the other until 500 are ahead again.
  Counted in commits loaded, so that the commits of folded merges count too.
- While there are more, the last row says **Loading more commits…** with a spinning icon, and the bar runs along the
  top of the list while the page is read.
- The commits already listed stay where they are: the rows of the page are added under them, and the scroll position,
  the selection and the folds do not move. A read that finds the same commits as before, as after a file is saved,
  changes none of the rows.
- Choosing another repository, other branches, a filter or a search starts again from one page.

Acceptance:

- [ ] With 1,000 commits loaded, scrolling to row 500 asks for the next page; at row 499, not yet.
- [ ] Scrolling on while the page is read does not ask for it again.
- [ ] With `graph.pageSize` at 100, scrolling to the 50th row loads pages until 600 commits are loaded.
- [ ] After a page is added, the selected commit and the scroll position are as they were.
- [ ] The last row says **Loading more commits…** while there are more, and is gone at the end of the history.

### `graph.lanes` — Lay out the lines of the graph

Where each commit's dot goes, and the lines between commits, row by row.

- **Kind**: Derivation
- **Function**: `model::graph::lanes(rows: &[GraphInput]) -> Vec<GraphCell>`

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

Acceptance:

- [ ] Each example above gives exactly the cells it says.
- [ ] A branch merged and deleted leaves its lane free for the next tip, which takes it and a new color.
- [ ] Twenty branches open at once give twenty lanes, and once they end the graph is one lane wide again.
- [ ] A stash on a commit that is also the parent of the next commit of the history: the commit goes on the lane of
      the history, not on the stash's dashed one.

### `graph.fit-width` — How wide the graph is drawn

The graph is as wide as its lines need, within limits, and draws its lines closer before it cuts any.

- **Kind**: Derivation
- **Function**: `model::graph::fit_width(lanes: usize, wanted: Option<f32>, available: f32) -> GraphFit`
- **Uses**: by `graph.view`, `graph.resize-graph` and the commits of what is checked out (`commits.`).

Input: the most lanes any row of the list has, the width the graph was dragged to (none while it chooses its own), and
the width of the list (0 while it is not known). Output: the width the graph is drawn in, the room each lane gets in
it, whether lanes are cut off at its edge, and its natural width.

- A lane has 14 px at its ease and never less than 8 px; the graph has 6 px of margin. Its natural width is
  `lanes × 14 + 6`.
- Choosing its own width, the graph takes its natural width, but no more than 16 lanes at their ease (230 px) and no
  more than a third of the list.
- Dragged, it takes the width it was dragged to, but no more than its natural width and no more than 60% of the list.
- It is never narrower than one lane at its ease (20 px).
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
| Branch rail and commits | 210 px | 120 px | 35% of the window | back to 210 px |
| Commits and inspector, side by side | 320 px for the inspector | 200 px | 55% of the window | back to 320 px |
| Inspector over the commits, in the narrow layout | half the height for the inspector | 80 px | 80% of the height | back to half |

Dragging away from the branch rail or the inspector makes it larger. The arrows move the line by 16 px. A size larger
than the window has room for is drawn at the most the window allows, and comes back when the window is wider. The sizes
are remembered for the workspace.

Acceptance:

- [ ] The branch rail cannot be dragged narrower than 120 px nor wider than 35% of the window.
- [ ] A double click on the line before the inspector gives it 320 px again.
- [ ] The sizes are there after the window is closed and opened again on the same workspace.

### `graph.toggle-references` — Put the branch rail away

The first button of the header puts the branch rail away, to leave its room to the commits, and brings it back.

- **Kind**: View
- **Function**: `views::graph::toggle_references_view(state: &ReferencesShown, ui) -> Vec<Intent>`
- **Entry points**: the first button of the header, pressed while the branch rail is shown.

Its tooltip says **Hide the branches, tags and stashes beside the commits**, or **Show the branches, tags and stashes
beside the commits**. Whether they are put away is remembered for the workspace.

Acceptance:

- [ ] A click hides the branch rail and the commits take its room; another brings it back at the width it had.
- [ ] A hidden branch rail stays hidden after the window is opened again.

### `graph.labels` — The labels of each commit

Which labels a commit has in the timeline, and what each says.

- **Kind**: Derivation
- **Function**: `model::graph::labels(state: &RepositoryState, settings: &GraphSettings) -> HashMap<Oid, Vec<Label>>`
- **Uses**: `graph.combine-upstream`, `graph.pull-request-badge`.

In this order on each commit:

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

### `graph.combine-upstream` — A branch and its upstream on one label

A branch and its upstream of the same name share one label while they are at the same commit; once one of them moves
on, each has its own label again.

- **Kind**: Derivation
- **Function**: `model::graph::combine_upstream(branch: &Branch, remote_branches: &[RemoteBranch]) -> Option<String>`
- **Entry points**: **Put a branch and its upstream on one label** in the **…** menu of the header.
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

Once the pull requests are loaded from GitHub, the label of a branch that has an open one ends with its number, in the
color of its checks.

- **Kind**: View
- **Function**: `views::graph::pull_request_badge_view(state: &PullRequestBadge, ui) -> Vec<Intent>`
- **Entry points**: the labels of the timeline, the rows of the branch rail.
- **Uses**: `github.load-pull-requests`, `github.open-pull-request`.

- The badge has the icon of a pull request, a draft one for a draft, and its number, in the color of its checks:
  passed, failed or running; with no checks, in the color of the label.
- Its tooltip is `Pull request #12: Add the parser · checks passed` (`Draft pull request…`, `checks failed`, `checks
  running`), then **Click to open it on GitHub**.
- A click opens the pull request in the browser, and does not select the commit; a double click on it does nothing
  more.
- A local branch has the pull request of the name it was pushed under (its upstream on a remote on GitHub, not gone). A
  remote branch has it on its own label only when no local branch tracks it.

Acceptance:

- [ ] A branch whose upstream has open pull request #12 with checks failing has `#12` in the color of failure.
- [ ] Clicking the badge opens the pull request and leaves the selection as it was.
- [ ] `origin/feature` with a pull request, tracked by `feature`, has no badge of its own; untracked, it has one.
- [ ] Before the pull requests are loaded, no label has a badge.

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
  the lines between them keep their colors, and the rest of the timeline fades. Going from one label to another changes
  it at once; leaving the labels brings the timeline back 100 ms later, so that passing over labels does not flash it.
- With `Shift` held on a label of the timeline, only the commits it has that the commit checked out has not stand out:
  what merging it, or a pull request of it, would bring. When the commit checked out is not among the rows, the whole
  history stands out.
- Rows of the branch rail bring out their commit's history in the timeline while it is shown.
- While a label is under the pointer, it takes the place of the history of the selected commit.

Acceptance:

- [ ] Resting on `feature` for 150 ms fades every commit `feature` does not reach.
- [ ] Moving the pointer across labels quickly does not fade the timeline.
- [ ] With `Shift` held on `feature`, only the commits of `feature` not in the commit checked out stay in color.
- [ ] Resting on a branch in the branch rail brings out its history in the timeline.

### `graph.trace-selected` — The history of the selected commit

The commit selected brings out its history, less strongly, for as long as it is selected: the commits it comes from and
the ones made on top of it.

- **Kind**: View
- **Function**: `views::graph::trace_selected_view(state: &SelectedTrace, ui) -> Vec<Intent>`
- **Entry points**: selecting a commit; **Bring out the history of the selected commit** in the **…** menu.
- **Settings**: `graph.traceSelected` (on by default).
- **Uses**: `graph.trace-history`.

The commits the selected one reaches and those that reach it, with the lines between them, keep their colors; the rest
is dimmed, less than under a label. It does not apply while several commits are selected, nor to the draft, and
gives way to the history of a label under the pointer until the pointer leaves it. It stays while the
commits are read.

Acceptance:

- [ ] Selecting a commit on `feature` dims the commits of `main` made after `feature` left it.
- [ ] Selecting the draft dims nothing.
- [ ] With two commits selected, nothing is dimmed.
- [ ] Unchecking the entry in **…** sets `graph.traceSelected` to false and nothing is dimmed.

### `graph.columns` — What is shown with each message

Who made each commit and when, on the line under its message, and its hash at the end of the row.

- **Kind**: View
- **Function**: `views::graph::columns_view(state: &ColumnsState, ui) -> Vec<Intent>`
- **Entry points**: **Show the author**, **Show the date** and **Show the hash** in the **…** menu.
- **Settings**: `graph.columns`: a list of `author`, `date` and `hash`, all three by default; not a list, all three.

- The author is the name git has for the author. The date is when the commit was made, said as `now` under a minute,
  `5m ago`, `3h ago`, `2d ago` under a week, and as a date (`Oct 3, 2026`) after; its tooltip has the full date and
  time. The hash is the first seven characters.
- What is left out gives its room to the messages.
- In the narrow layout of the window, the hash is not shown.
- The draft has none of them.

Acceptance:

- [ ] By default each row shows the author, the date and the short hash.
- [ ] Unchecking **Show the date** removes the dates from the line under each message.
- [ ] A commit made 90 minutes ago says `1h ago`, and its tooltip the full date.

### `graph.date-separators` — Lines between days, weeks and months

A thin line before the first commit of yesterday, of this week, of last week and of each month before has its name at
its end, so that you see how far back you are as you scroll.

- **Kind**: Derivation
- **Function**: `model::graph::date_separators(dates: &[Option<i64>], now: DateTime) -> BTreeMap<usize, String>`
- **Entry points**: **Separate the days, weeks and months** in the **…** menu.
- **Settings**: `graph.dateSeparators` (on by default).

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
- **Entry points**: **Show only the first parent of merges** in the **…** menu.
- **Settings**: `graph.firstParent` (off by default).
- **Uses**: `graph.read-commits`, `graph.read-position`.

On, the commits are read with `--first-parent`, and each merge has its first parent only. Each branch still shown has
its line, down to where it started. Changing it reads the timeline again from one page.

Acceptance:

- [ ] On, a merge of `feature` into `main` is one row on the line of `main`, and the commits of `feature` it merged are
      not listed unless `feature` itself is shown and reaches them.
- [ ] A merge with the setting on has no fold button (`graph.fold-merge`): it brought nothing in.

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

### `graph.fold-all` — Fold all merges

Folds every merge, also the ones loaded later, until it is unchecked.

- **Kind**: View
- **Function**: `views::graph::fold_all_view(state: &FoldsState, ui) -> Vec<Intent>`
- **Entry points**: **Fold all merges** and **Unfold all merges** in the **…** menu.
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
  branches that do not have it, the draft is not there.
- **Uses**: `details.show`, `changes.pane`, `stash.push`, `stash.push-staged`, `changes.stage-all`,
  `changes.unstage-all`, `changes.discard-all`.

- The draft has the commit box of the Changes page (`changes.pane`): the message of the next commit and **Commit**.
  Under them it says `<n> files changed`, `<n>` being the number of files with a change of any kind, each once
  (`1 file changed`), followed by `· <m> staged` while some are staged; and **Review**, which opens the Changes page.
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
- **Entry points**: **Show stashes** in the **…** menu; the context menu of a stash row.
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

### `graph.show-in-graph` — Go to a commit in the timeline

Opens the timeline at a commit: loads the history up to it when it is further back, and shows it when the timeline
leaves it out.

- **Kind**: View
- **Function**: `views::graph::show_in_graph_view(state: &GoToState, ui) -> Vec<Intent>`
- **Entry points**: **Show in the timeline** in the menu of a branch, tag or worktree in the branch rail (the
  worktree's commit); a click on a tag, a stash or a worktree in the branch rail; and, from other areas, a step of a
  search, **Go to commit…**, the reflog, the commits of what is checked out, the bisect banner.
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

The local branches with how far each is from its upstream, each in the color of its lane, and the remote branches;
then, kept aside, the stashes, the worktrees, the tags and the submodules: in lists that fold, on the left of the
window.

- **Kind**: View
- **Function**: `views::refs::tree_view(state: &ReferencesState, ui) -> Vec<Intent>`
- **Entry points**: the left of the window; it can be put away (`graph.toggle-references`), and in the narrow layout
  it is narrowed to its icons.
- **Uses**: `refs.filter`, `refs.scope-line`, `refs.sort`, `refs.recent`, `refs.favorite`, `filter.show-only`,
  `filter.show-all`, `graph.show-in-graph`,
  `graph.hover-history`, `graph.pull-request-badge`, `branch.open`, `drag.drop`, `branch.`, `tag.`, `stash.`,
  `worktree.`, `submodule.`.

- Lists, in this order: **Favorites** and **Recent** (each only when it has something), **Local**, **Remotes** (the
  branches of each remote under its name), then **Kept aside**: **Stashes**, **Worktrees**, **Tags**, **Submodules**
  (only when there are some). Each has its icon and the count of what it lists.
- The lists of **Kept aside** are closed when Gitardo starts, and the others open, since the local branches are the
  legend of the lanes; the ones opened or closed stay so until the application is closed.
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
  (`filter.show-only`); a second click on it brings the whole history back (`filter.show-all`). A click on a tag, stash
  or worktree goes to its commit (`graph.show-in-graph`). A double click on a branch
  opens it as `branch.open` says; on a remote branch, checks it out (`branch.open`); on a worktree or a submodule,
  opens it in a new window. Resting the pointer on one brings out its history (`graph.hover-history`). Branches can be
  dragged onto the timeline (`drag.drop`).
- A right click opens the menu of the reference (`branch.`, `remote.`, `tag.`, `stash.`, `worktree.`, `submodule.`); a
  right click on the title of a list, or its **…**, opens the list's own menu. The Worktrees list has **Create
  worktree…** on its title and **…** with what concerns worktrees; the Submodules list **Add submodule…**.

Acceptance:

- [ ] After the application starts, every list of **Kept aside** is closed; one opened stays open when another
      repository is chosen.
- [ ] A list of 250 tags lists 100 and **Show all 250…**.
- [ ] A branch one ahead and two behind its upstream says `↑1 ↓2`; one whose upstream is gone says `gone`.
- [ ] Clicking a stash in the branch rail selects the stash in the timeline.
- [ ] The checked-out branch, 150th by name, is listed after the first 100.

### `refs.filter` — Filter the branch rail by name

From 15 branches and tags on, a filter box appears at the top of the branch rail, which lists the names that match best
first.

- **Kind**: View
- **Function**: `views::refs::filter_view(state: &ReferencesFilter, ui) -> Vec<Intent>`
- **Entry points**: the box at the top of the branch rail.
- **Offered when**: the lists there have 15 names or more together (local branches, remote branches, tags and
  stashes), or something is typed in it.
- **Uses**: `filter.rank-names`.

What is typed leaves in each list the entries whose names have it, ignoring case, the best matches first
(`filter.rank-names`). A list with matches is open while something is typed, whether it was closed or not; its
count says `<matches> of <all>`. `Escape` or the button at the end of the box empties it.

Acceptance:

- [ ] With 14 branches and no tag, there is no box; with 15, there is.
- [ ] Typing `main` lists `main`, then `origin/main`, then `maintenance`, then `release/main-2`, then `domain`.
- [ ] A closed list with a match is shown open while the box has text, and closed again when it is emptied.
- [ ] `Escape` empties the box.

### `refs.scope-line` — The line of the branches shown, at the top of the branch rail

While the timeline is narrowed to some branches, the branch rail lists only those, under a line that says what they are
narrowed to.

- **Kind**: View
- **Function**: `views::refs::scope_line_view(state: &ScopeLine, ui) -> Vec<Intent>`
- **Entry points**: the top of the branch rail, while the timeline is narrowed to some branches.
- **Settings**: `branches.followGraphFilter` (on by default).
- **Uses**: `filter.list-too`, `filter.show-all`, `filter.show-too`.

The line says what the timeline is narrowed to, as the button of the branch filter does (`Current branch`, `main +2`).
Its button lists all the references instead, with those the timeline leaves out dimmed, and back (`filter.list-too`);
its **Show all** shows all the branches again (`filter.show-all`). While all are listed, a dimmed one says **Not shown
in the timeline** in its tooltip, and has an eye at the end of its row, under the pointer, that adds its history to the
timeline (`filter.show-too`). The lists have only the branches, remote branches and tags shown while the line follows
the timeline; stashes, worktrees and submodules are always listed.

Acceptance:

- [ ] Narrowed to `main` and `feature`, the Local list has only those two, under a line saying `main +1`.
- [ ] The button of the line lists every branch, the others dimmed, and sets `branches.followGraphFilter` to false.
- [ ] The eye on a dimmed `fix` adds `fix` to the branches shown.
- [ ] **Show all** takes the line away and lists everything.

### `refs.sort` — Sort the branches and tags

Branches, remote branches and tags are listed by name, or the one committed to last first.

- **Kind**: Derivation
- **Function**: `model::refs::sort(references: &References, order: SortOrder) -> References`
- **Entry points**: **Sort by name** and **Sort by date** in the **…** on the title of the Local, Remotes and Tags
  lists, checked as they are.
- **Settings**: `branches.sortOrder`: `name` or `date`; not set, by name.

- By name is the order git lists them in.
- By date, the latest first: the date of the commit, or for a tag made with a message, when it was made; among the
  same date, by name.
- The order is the same for all three, in the branch rail and in every picker a branch or a tag is chosen from. Choosing
  one writes the setting in the user's settings file.

Acceptance:

- [ ] By date, a branch committed to an hour ago comes before one committed to yesterday.
- [ ] An annotated tag made today on an old commit comes first among the tags by date.
- [ ] **Sort by date** in the Tags list also orders the Local list and the branch picker of `branch.create`.

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
- **Uses**: by `refs.tree`.

The favorites are kept for each repository of the workspace, by the full names of the branches, in Gitardo's storage,
and survive the window being closed. They are listed in the order they were added. A branch that no longer exists is
not listed, and is forgotten the next time the favorites change. Nothing of the repository changes.

Acceptance:

- [ ] **Add to favorites** on `feature` lists it under **Favorites** and puts a star on it under **Local**.
- [ ] **Remove from favorites** takes it out; with none left, the Favorites list is not there.
- [ ] The favorites of one repository are not those of another of the same workspace.
- [ ] A favorite deleted with `git branch -D` is no longer listed.
