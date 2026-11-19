# History rewriting

What changes the commits of a branch, or brings commits from elsewhere onto it: merging in its four ways, cherry-picking
one commit or several, reverting, resetting, rebasing onto something else, the interactive rebase and the operations
built on it (editing a message, squashing, splitting, fixup), several commits selected at once, dragging commits and
branches onto the timeline, the other branches of a stack that move with a rewrite, and what a rewrite changed, commit
by commit (`range-diff.`). What the banner offers once one of them stops (**Continue**, **Skip**, **Abort**) is in the
conflicts area: `conflicts.continue-operation`, `conflicts.skip-step`, `conflicts.abort-operation`. Copying or saving commits as a patch is in the
patch area.

### `selection.select` — Select several commits

Selects several commits of the timeline at once, to act on them together.

- **Kind**: View
- **Function**: `views::selection::select_view(state, ui, row) -> Option<Intent>`, for a row the list says was
  clicked
- **Entry points**: `Ctrl`+click (`Cmd`+click on macOS) on a row of the timeline adds that commit to the selection, or
  takes it out; `Shift`+click selects every commit between the selected one and the clicked one, both included.
- **Offered when**: in the timeline, wide or narrow. The draft and the rows of stashes are never part of a selection
  of several.
- **Uses**: `selection.bar`, `selection.menu`.

Rules:

- A plain click selects one commit and empties the selection of several.
- `Ctrl`+click starts from the commit selected alone, if any, and adds the clicked one; on a commit already in the
  selection it takes it out. When one commit is left, it is selected alone.
- `Shift`+click takes the rows between the commit selected alone (the anchor) and the clicked one, in the order of the
  timeline, leaving out the draft and the stashes. When that leaves one commit, it is selected alone.
- `Ctrl`+click or `Shift`+click on the draft or on a stash selects it alone.
- The inspector shows the commit clicked last that is in the selection.

Acceptance:

- [ ] With `A` selected, `Ctrl`+click on `C` selects `A` and `C`, and the bar says **2 commits selected**.
- [ ] `Ctrl`+click on `C` again leaves `A` selected alone and the bar goes away.
- [ ] With `A` selected, `Shift`+click on the fourth row below selects five commits, or fewer when stashes are among them.
- [ ] `Shift`+click across the draft does not put the draft in the selection.
- [ ] On macOS, `Cmd`+click does what `Ctrl`+click does elsewhere.

### `selection.bar` — The bar of a selection of several commits

A bar above the commits of the timeline counts the commits selected and offers what is done most with them.

- **Kind**: View
- **Function**: `views::selection::bar_view(state, several, ui, rect) -> Vec<Intent>`
- **Entry points**: shown above the commits of the timeline while two commits or more are selected.
- **Offered when**: two commits or more are selected.
- **Uses**: `squash.commits`, `selection.cherry-pick`, `patch.copy`, `rebase.find-line-before-head`.

What it shows, in order:

- **N commits selected**, then, dimmed, *Ctrl+click adds or removes one, Shift+click selects up to there*.
- **Squash into one**, with the tooltip *Makes them one commit, with a message you give*, when `squash.commits` is
  offered for them.
- **Cherry-pick onto `<current branch>`** (`HEAD` when detached), with the tooltip *Applies them oldest first, as one
  operation*, when `selection.cherry-pick` is offered for them. It empties the selection.
- **Copy as patch**, with the tooltip *Copies them as one patch, oldest first, with their authors and messages, for git
  am to make them again*, when none of them is a merge (`patch.copy`).
- **Clear selection**, which empties it.

The buttons that run something are disabled while an operation runs.

Acceptance:

- [ ] With three commits selected that follow one another before `HEAD`, the bar has **Squash into one**.
- [ ] With a merge commit among the selection, neither **Cherry-pick onto …** nor **Copy as patch** is there.
- [ ] While a rebase is stopped, neither **Squash into one** nor **Cherry-pick onto …** is there.
- [ ] **Clear selection** leaves no commit in the selection of several, and the bar goes away.

### `selection.menu` — The context menu of a selection of several commits

Right-click on a commit of the selection opens the menu of all of them.

- **Kind**: View
- **Function**: `views::selection::menu_view(state, several, ui) -> Vec<Intent>`
- **Entry points**: right-click on any commit of a selection of two or more.
- **Offered when**: two commits or more are selected.
- **Uses**: `squash.commits`, `selection.cherry-pick`, `compare.selected-before`, `patch.copy`, `patch.save`.

Entries, in order, each shown only when it applies:

- **Squash N commits into one…**, when `squash.commits` is offered for them.
- **Cherry-pick N commits onto `<current branch>`**, when `selection.cherry-pick` is offered for them.
- **Compare these two commits**, when exactly two are selected: opens the comparison of the older (the lower in the
  timeline) with the newer, in a Compare page.
- a separator;
- **Copy hashes**: the full hashes, newest first as in the timeline, one per line, to the clipboard.
- **Copy as patch** and **Save as patch…**, when none of them is a merge (`patch.copy`, `patch.save`).

Acceptance:

- [ ] With two commits selected, **Compare these two commits** opens a Compare page whose first side is the lower of
      the two in the timeline.
- [ ] With three commits selected, **Compare these two commits** is not in the menu.
- [ ] **Copy hashes** with three commits selected puts three lines of 40 (or 64) hexadecimal characters in the clipboard,
      in the order of the timeline.

### `selection.cherry-pick` — Cherry-pick several commits

Applies several selected commits onto what is checked out, with one `git cherry-pick`, so that git treats them as one
operation to continue, skip or abort.

- **Kind**: Operation
- **Function**: `features::selection::cherry_pick(context, answers) -> Outcome`
- **Entry points**: **Cherry-pick onto `<current branch>`** in the selection bar; **Cherry-pick N commits onto
  `<current branch>`** in the context menu of the selection.
- **Offered when**: two commits or more are selected, none of them is a merge, and no merge, rebase, cherry-pick,
  revert, patch or bisect is under way.
- **Plan**: `git cherry-pick [-x] <hash>…`, the hashes oldest first whatever the order they were clicked in: the order
  of the timeline, reversed.
- **Danger**: None. The dialog, when shown, says *In this order:* with the first 15 commits as `<short hash> <subject>`
  and *…and N more* past them, then *If one of them conflicts, git stops there: resolve it and continue, skip that
  commit, or abort them all.*
- **Undo**: right after it, all of its commits at once: the journal keeps the commit `HEAD` was at before, and
  `undo.last` runs `git reset --keep <that commit>`. This holds also when it stopped on a conflict and was continued or
  skipped in Gitardo until it ended; **Abort** forgets it. If it was continued from a terminal, or anything else
  happened in between, Undo goes back one commit at a time, by the reflog.
- **After**: the selection is emptied. When git stops on a commit that changes nothing on top of `HEAD`,
  `cherry-pick.skip-empty` gets past it.
- **Refuses**: while an operation is under way, *Finish or abort the `<operation>` first.*; when fewer than two of the
  selected commits are loaded, *Select the commits to cherry-pick in the timeline.*; with a merge among them, *A merge
  commit can only be cherry-picked on its own.*
- **Settings**: `cherryPick.recordOrigin` adds `-x`, so that each new commit ends its message with
  `(cherry picked from commit <hash>)`.
- **Uses**: `cherry-pick.skip-empty`, `conflicts.continue-operation`, `conflicts.skip-step`, `conflicts.abort-operation`, `undo.last`,
  `undo.journal`.
- **Extension**: **Cherry-pick onto `<current>`** in the bar above the graph that counts the selected commits (tooltip
  *Applies them oldest first, as one operation*) and **Cherry-pick N commits onto `<current>`** in the context menu of
  the selection, the commits selected with `Ctrl`+click (`Cmd` on macOS) and `Shift`+click (`selectionMenu` in
  `src/webview/menus.ts`, the bar in `src/webview/App.tsx`). The action `cherryPickMany` runs it with the commits in the
  order of the rows of the graph, newest first (`engineRuns` in `src/engine/actions.ts`). When the engine's binary is
  missing, it says so (D49).

Acceptance:

- [ ] Selecting `C`, then `A`, then `B` (`A` oldest) runs `git cherry-pick A B C`.
- [ ] With `cherryPick.recordOrigin` on, the command is `git cherry-pick -x A B C`.
- [ ] A selection with a merge commit is refused before git runs, with the message above.
- [ ] After three commits picked without a conflict, one Undo runs `git reset --keep <HEAD before>` and the branch is
      where it was.
- [ ] After a stop on the second commit, resolved and continued in Gitardo, one Undo takes back all three.
- [ ] After a stop that was continued from a terminal, Undo takes back only the last commit.
- [ ] Extension: selecting *Pick two*, then *Pick one* (older), and choosing **Cherry-pick onto main** runs `git
      cherry-pick <one> <two>`, and the branch ends with *Pick one*, then *Pick two*.
- [ ] Extension: the bar and the menu offer nothing to cherry-pick when a merge commit is selected, while an operation
      is under way, or during a bisect.
- [ ] Extension: with `gitardo.commandPreview` set to `all`, the question is **Cherry-pick 2 commits onto main?** and
      ends with `git cherry-pick <one> <two>`; right after, **Undo** asks **Undo "Cherry-pick 2 commits onto main"?**
      and runs `git reset --keep <before>`.
- [ ] Extension: **Abort** while the series is stopped leaves `HEAD` where it was before it.

### `cherry-pick.commit` — Cherry-pick a commit

Applies the changes of one commit onto what is checked out, as a new commit, after saying which files will conflict.

- **Kind**: Operation
- **Function**: `features::cherry_pick::commit(context, answers) -> Outcome`
- **Entry points**: **Cherry-pick `<short hash>`** in the context menu of a commit; a commit dropped on the commit
  checked out (`drag.drop`); **Cherry-pick** in the menu of a reflog entry (`reflog.browse`).
- **Offered when**: the commit is not the one checked out.
- **Reads**: for a commit with one parent, `merge.read-predicted-conflicts` with `<hash>^` as the base, `HEAD` as ours
  and `<hash>` as theirs: picking a commit is merging it into `HEAD` from its parent. A merge commit is not predicted.
- **Plan**: `git cherry-pick [-x] [-m 1] <hash>`: `-m 1` for a merge commit, which is picked against its first parent.
- **Danger**: None when no conflict is predicted, and the dialog, when shown, says *No conflicts expected.* Confirm when
  conflicts are predicted, because the operation will stop: *This will stop with conflicts in N files:* and the first 15
  paths, then *…and N more*.
- **Undo**: by the reflog (`undo.last`): the branch goes back to the commit before the pick.
- **After**: when git stops because the commit changes nothing on top of `HEAD`, `cherry-pick.skip-empty`; when it
  stops on conflicts, the conflicts banner and the Changes page show them.
- **Settings**: `cherryPick.recordOrigin` adds `-x`, also for a commit dropped on the one checked out.
- **Uses**: `merge.read-predicted-conflicts`, `cherry-pick.skip-empty`, `conflicts.continue-operation`, `undo.last`.
- **Extension**: **Cherry-pick `<short>`** in the context menu of a commit in the graph and the Gitardo Graph, also on a
  merge and while an operation is under way (`commitMenu` in `src/webview/menus.ts`); **Cherry-pick `<short>` onto
  `<current>`** for a commit dropped on the commit checked out or on the row of the uncommitted changes; **Cherry-pick**
  for a position chosen in the list of the reflog (`Controller.browseReflog`). The action `cherryPick` runs it on the
  commit. The setting is `gitardo.cherryPick.recordOrigin`. When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] The context menu of the commit checked out has no **Cherry-pick** entry.
- [ ] Picking a commit that conflicts in `a.txt` asks first, and the dialog names `a.txt`.
- [ ] Picking a commit that does not conflict runs `git cherry-pick <hash>` without a dialog when `commandPreview` is
      `off`.
- [ ] Picking a merge commit runs `git cherry-pick -m 1 <hash>` and predicts nothing.
- [ ] With Git older than 2.40, the prediction is skipped and the pick runs as with no conflict predicted.
- [ ] Undo after a pick resets the branch to the commit it was at before.
- [ ] Extension: with `gitardo.cherryPick.recordOrigin` on, the new message ends with `(cherry picked from commit
      <hash>)`; off, it is the message picked.
- [ ] Extension: while a merge is stopped, **Cherry-pick** says *Finish or abort the merge first.* and git does not run.
- [ ] Extension: dropping a commit anywhere but on the commit checked out or the uncommitted changes offers nothing.

### `cherry-pick.skip-empty` — Get past a commit that changes nothing

When a cherry-pick or a revert stops on a commit whose changes are already there, or whose conflicts were resolved by
keeping what `HEAD` has, git will not continue until that commit is skipped, and says so in words that read like a
failure. Gitardo skips it, as a rebase would, and says so in the log.

- **Kind**: Operation
- **Function**: `features::cherry_pick::skip_empty(context, answers) -> Outcome`
- **Entry points**: none of its own: it is the follow-up on failure of `cherry-pick.commit`, `selection.cherry-pick`,
  and of `conflicts.continue-operation` and `conflicts.skip-step` during a cherry-pick or a revert.
- **Offered when**: the step that failed was a `git cherry-pick` or `git revert` command.
- **Reads**: `git rev-parse -q --verify CHERRY_PICK_HEAD` (or `REVERT_HEAD`) must succeed; `git ls-files -u` must print
  nothing (no conflict left); `git diff --quiet HEAD` must succeed (the tracked files, in the index and on disk, are as
  in `HEAD`, so that skipping loses nothing). Then `git rev-parse CHERRY_PICK_HEAD` (or `REVERT_HEAD`) for the commit to
  name.
- **Plan**: a warning in the log, *`<short hash>` changes nothing on top of HEAD, so the cherry-pick skips it.* (or *the
  revert*), then `git cherry-pick --skip` (or `git revert --skip`). After each skip the reads are made again: the next
  commit of the series may stop the same way, and is skipped the same way.
- **Danger**: None.
- **Undo**: part of the operation it follows: its Undo takes back the whole.
- **After**: when a skip itself fails for another reason, such as the conflicts of the next commit, that is the failure
  shown; when none of the conditions holds, the original failure is shown.
- **Refuses**: never; it does nothing when the conditions do not hold.

Acceptance:

- [ ] Picking a commit already applied on the branch ends with the commit skipped, a warning in the log, and no
      operation under way.
- [ ] Picking three commits of which the second is already applied ends with two new commits and no stop.
- [ ] With a file changed and not committed, the stop is left as git left it and its error is shown.
- [ ] With a conflict still marked, nothing is skipped.

### `revert.commit` — Revert a commit

Makes a new commit that takes back the changes of a commit.

- **Kind**: Operation
- **Function**: `features::revert::commit(context, answers) -> Outcome`
- **Entry points**: **Revert `<short hash>`** in the context menu of a commit in the timeline, narrowed or not.
- **Offered when**: on every commit, the one checked out included.
- **Plan**: `git revert --no-edit [-m 1] <hash>`: `-m 1` for a merge commit, which is reverted against its first parent.
  `--no-edit` keeps the message git writes, *Revert "<subject>"*, since no editor can open.
- **Danger**: None. A revert is not predicted for conflicts.
- **Undo**: by the reflog (`undo.last`): the branch goes back to the commit before the revert.
- **After**: when it stops on conflicts, the conflicts banner shows **Continue**, **Skip** and **Abort**.
- **Uses**: `conflicts.continue-operation`, `cherry-pick.skip-empty`, `undo.last`.
- **Extension**: **Revert `<short>`** in the context menu of a commit in the graph, the Gitardo Graph and the Commits
  view (`commitMenu`, `logMenu` in `src/webview/menus.ts`), also while an operation is under way. The action `revert`
  runs it on the commit. When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] Reverting a commit with one parent runs `git revert --no-edit <hash>`.
- [ ] Reverting a merge commit runs `git revert --no-edit -m 1 <hash>`.
- [ ] The new commit's subject is `Revert "<subject of the commit>"`.
- [ ] Extension: **Revert 1a2b3c4** runs `git revert --no-edit <hash>` and no editor opens; right after it, **Undo**
      asks, then runs `git reset --keep <previous>`.
- [ ] Extension: while a cherry-pick is stopped, **Revert** says *Finish or abort the cherry-pick first.*
- [ ] Extension: reverting a commit whose changes are taken back already fails with git's *nothing to commit*, and
      nothing is left under way.

### `reset.to-commit` — Reset the current branch to a commit

Moves the branch checked out (or `HEAD` when detached) to a commit, keeping the changes staged, unstaged, or not at all.

- **Kind**: Operation
- **Function**: `features::reset::to_commit(context, answers) -> Outcome`
- **Entry points**: **Reset `<branch>` to here…** (`HEAD` when detached) in the context menu of a commit in the
  timeline, narrowed or not, the one checked out included, where **Hard** discards the changes not committed;
  **Reset `<branch>` to `<short hash>`…** when the current branch is dropped on another commit (`drag.drop`);
  **Reset `<branch>` to here…** in the menu of a reflog entry.
- **Asks**: a pick titled *Reset `<branch>` to `<short hash>`*, with **Soft** (*Keep all changes, staged*), **Mixed**
  (*Keep all changes, unstaged*) and **Hard** (*Discard all changes*). No default: the user picks one.
- **Plan**: `git reset --soft <hash>`, `git reset --mixed <hash>` or `git reset --hard <hash>`.
- **Danger**: None for soft and mixed. Destructive for hard, because *A hard reset throws away all uncommitted changes
  to tracked files.*
- **Undo**: by the reflog (`undo.last`): the branch goes back to the commit it was at. What a hard reset threw away of
  the changes not committed is not brought back.
- **Uses**: `undo.last`.
- **Extension**: **Reset `<current>` to here…** in the context menu of a commit in the graph and the Gitardo Graph
  (every commit, the one checked out included), in the Commits view (every commit but the one checked out) and in the
  list of the reflog; **Reset `<name>` to `<short>`…** for the current branch dropped on another commit. The action
  `reset` runs it on the commit, and the mode is picked in a quick pick. When the engine's binary is missing, it says so
  (D49).

Acceptance:

- [ ] Choosing **Mixed** on `abc1234` runs `git reset --mixed <full hash of abc1234>`.
- [ ] Choosing **Hard** always shows the command and the warning first, whatever `commandPreview` says.
- [ ] Closing the pick runs nothing.
- [ ] Every commit of the timeline, narrowed to the branch checked out or not, has the reset entry in its context
      menu, the commit checked out included.
- [ ] Extension: **Soft** leaves the changes of the commits after it staged and **Mixed** unstaged; right after a hard
      reset, **Undo** brings the commits back with `git reset --keep <previous>`.

### `merge.read-predicted-conflicts` — Predict the conflicts of a merge

Works out, without touching the index or the working tree, which files a merge would leave in conflict.

- **Kind**: Read
- **Function**: `git::read::merge::read_predicted_conflicts(git, request) -> Result<Option<Vec<PathBuf>>, GitError>`
- **Entry points**: none: used by `merge.into-current`, `merge.choose-way` and `cherry-pick.commit`.
- **Reads**: `git merge-tree --write-tree --no-messages --name-only -z [--merge-base=<base>] <ours> <theirs>`, where
  `<ours>` is `HEAD` unless the request says otherwise, and `<base>` is given for a cherry-pick (the parent of the
  commit picked).
- **Extension**: the engine reads it for the merges and the cherry-pick it runs; the extension does not read it.

Rules:

- Exit code 0: no conflict; exit code 1: conflicts. The first NUL-separated record is the tree written; the records
  after it are the paths in conflict, each once, in the order git gives them.
- Any other exit code (Git older than 2.40, histories with no common ancestor) gives "cannot tell" (`None`), and the
  operation runs without a prediction.
- How it is said, by the operation that asked: no path, *No conflicts expected.*; one path or more, *This will stop with
  conflicts in 1 file:* or *in N files:*, then the paths one per line, at most 15, then *…and N more*. Paths in conflict
  make the operation `Confirm`.

Acceptance:

- [ ] Output `<tree>\0a.txt\0a.txt\0b.txt\0` with exit code 1 gives `a.txt` and `b.txt`.
- [ ] Output `<tree>\0` with exit code 0 gives an empty list.
- [ ] Exit code 128 gives "cannot tell".
- [ ] 20 paths are said as 15 lines and *…and 5 more*.
- [ ] Extension: for a branch `other` that changes `a.txt` where `HEAD` changed it too, the prediction is `a.txt`, and
      the working tree, the index and `HEAD` are as they were.

### `merge.into-current` — Merge into the current branch

Merges a commit, branch or remote branch into what is checked out, as the configuration of git says: a fast-forward
when one is possible, otherwise a merge commit.

- **Kind**: Operation
- **Function**: `features::merge::into_current(context, answers) -> Outcome`
- **Entry points**: **Merge into `<current branch>`** in the context menu of a commit, a local branch and a remote
  branch; **Merge `<branch>` into `<current branch>`** in the menu of a branch dropped on the commit checked out
  (`drag.drop`).
- **Offered when**: something is checked out (the repository has a commit), and the commit or the branch's tip is not
  the commit checked out; for a local branch, it is not the one checked out.
- **Reads**: `merge.read-predicted-conflicts` with `HEAD` and the reference.
- **Plan**: `git merge <ref>`. A local branch whose name is also a tag or a remote branch is named in full,
  `refs/heads/<name>`; a commit is named by its hash.
- **Danger**: None when no conflict is predicted (*No conflicts expected.*); Confirm when conflicts are predicted, with
  their list.
- **Undo**: by the reflog (`undo.last`): the branch goes back to where it was before the merge or the fast-forward.
- **After**: when it stops on conflicts, the conflicts banner shows **Continue** and **Abort** (a merge has no step to
  skip).
- **Uses**: `merge.read-predicted-conflicts`, `merge.choose-way`, `conflicts.continue-operation`, `conflicts.abort-operation`, `undo.last`.
- **Extension**: **Merge into `<current>`** in the context menu of a commit, of a local branch (its label in the graph,
  the references, the Branches view) and of a remote branch, also while an operation is under way, and **Merge `<name>`
  into `<current>`** in the menu of a branch dropped on the commit checked out, not while one is (`mergeEntries` and
  `integrate` in `src/webview/menus.ts`). The action `merge` with no way, or the way `default`, runs it (`engineRuns` in
  `src/engine/actions.ts`); when it stops on conflicts, the message is *Merge `<name>` into `<current>` stopped: N files
  have conflicts. Resolve them, then continue or abort the merge.* (`reportEngineFailure` in
  `src/extension/controller.ts`). When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] Merging `feature` into `main` runs `git merge feature`.
- [ ] With a tag also called `feature`, it runs `git merge refs/heads/feature`.
- [ ] Merging the commit checked out is not offered.
- [ ] A merge predicted to conflict in `src/a.rs` asks first and names that file.
- [ ] Extension: **Merge into main** in the menu of the branch `feature` runs `git merge feature`, logged as `> git
      merge feature`; with `gitardo.commandPreview` set to `all`, the question **Merge feature into main?** says *No
      conflicts expected.*
- [ ] Extension: a merge that will conflict in `a.txt` is asked first whatever the setting, and once it stopped, the
      message says *Merge feature into main stopped: 1 file has conflicts.*
- [ ] Extension: right after a merge that moved `main`, **Undo** asks, then runs `git reset --keep <previous>`; while
      the merge is stopped, **Undo** says *Finish or abort the merge first.*

### `merge.choose-way` — Merge into the current branch, choosing how

The entry after **Merge into `<current branch>`** asks which of the four ways to merge, as the menu beside **Pull**
has the ways to pull.

- **Kind**: Operation
- **Function**: `features::merge::choose_way(context, answers) -> Outcome`
- **Entry points**: **Merge into `<current branch>`…** (with the hint *choose how*) wherever `merge.into-current` is
  offered, and **Merge `<branch>` into `<current branch>`…** in the menu of a branch dropped on the commit checked out.
- **Offered when**: as `merge.into-current`.
- **Reads**: `git merge-base --is-ancestor HEAD <ref>` (exit 0: a fast-forward is possible), before asking.
- **Asks**: a pick titled *Merge `<ref>` into `<current branch>`*, each item with the command and what it does as its
  description, *`<command>` · `<what it does>`*:
  - **Merge**, `git merge <ref>`: *A fast-forward when one is possible, otherwise a merge commit*;
  - **Merge (no fast-forward)**, `git merge --no-ff <ref>`: *Always a merge commit, even where a fast-forward would do*;
  - **Merge (fast-forward only)**, `git merge --ff-only <ref>`: *`<current>` moves up to `<ref>`, with no merge
    commit*, or, when `<current>` has commits `<ref>` has not, *Not possible: `<current>` has commits that `<ref>` has
    not*;
  - **Merge (squash)**, `git merge --squash <ref>`: *The changes of `<ref>` staged as one, for you to commit, with no
    merge recorded*.
- **Reads** (after the answer): for every way but the fast-forward only, `merge.read-predicted-conflicts`; for a
  fast-forward only that is not possible, `git rev-list --left-right --count HEAD...<ref> --` to tell why.
- **Plan**: `git merge [--no-ff | --ff-only | --squash] <ref>`.
- **Danger**: None, or Confirm when conflicts are predicted, as `merge.into-current`. A fast-forward only says *A
  fast-forward: no conflicts.* A squash adds first *The changes of `<ref>` are staged in `<current>` as one change, for
  you to commit. No merge is recorded: `<ref>` will not show as merged.*
- **Undo**: by the reflog for the first three. A squash merge moves no branch, so the journal keeps its Undo:
  `merge.undo-squash`, also when it stopped on conflicts, until something is committed.
- **After**: after a squash, the message *The changes are staged, and nothing is committed yet. Commit them to make them
  one commit on `<current>`.* with **Show changes** (opens the Changes page) and **Undo**; and the message of the draft,
  when it is empty, starts from the message git prepared in `.git/SQUASH_MSG`, which lists the commits
  brought in.
- **Refuses**: the fast-forward only when it is not possible, without running git: when `<ref>` has nothing
  `<current>` has not, *`<current>` already has every commit of `<ref>`.*; otherwise *`<current>` cannot be
  fast-forwarded to `<ref>`: it has commits that `<ref>` has not. Merge it another way, or rebase `<current>` onto it.*
- **Uses**: `merge.read-predicted-conflicts`, `merge.undo-squash`, `undo.journal`.
- **Extension**: **Merge into `<current>`…**, with the hint *choose how*, right after **Merge into `<current>`** in the
  same menus, and **Merge `<name>` into `<current>`…** in the drop menu. The action `merge` with the way `ask` runs it,
  and the way is picked in a quick pick; a view that names the way gives it as the answer `way` (`noFastForward` as
  `no-ff`, `fastForwardOnly` as `ff-only`, `squash`). A fast-forward only that is not possible is refused with a
  warning. After a squash, the message has no buttons, and the commit box of the Changes view gets the message git
  prepared (the follow-up `preparedMessage`). When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] The pick lists four ways, in the order above, and the third says *Not possible* when the current branch has a
      commit of its own.
- [ ] Choosing **Merge (fast-forward only)** when it is not possible runs nothing and shows the second refusal.
- [ ] Choosing **Merge (no fast-forward)** runs `git merge --no-ff <ref>`.
- [ ] After **Merge (squash)**, nothing is committed, the changes are staged, and the message of the draft holds the
      text of `.git/SQUASH_MSG` when it was empty.
- [ ] After **Merge (squash)**, a draft whose message already had text keeps it.
- [ ] Closing the pick runs nothing.
- [ ] Extension: the quick pick offers **Merge**, **Merge (no fast-forward)**, **Merge (fast-forward only)** and **Merge
      (squash)** in this order, each with its `git merge` command in its description; `Escape` runs nothing.
- [ ] Extension: **Merge (squash)** of `feature` stages the changes, moves no branch, and an empty commit box gets the
      message git prepared; **Undo** right after, also once a file was staged, asks **Undo "Merge feature into main
      (squash)"?** and runs `git reset --merge`.
- [ ] Extension: **Merge (fast-forward only)** when `main` has a commit of its own says *main cannot be fast-forwarded
      to feature: …* without running git.

### `merge.undo-squash` — Take back a squash merge

A squash merge stages changes and moves no branch, so the reflog cannot take it back: this takes what it staged out of
the index and the working tree.

- **Kind**: Operation
- **Function**: `features::merge::undo_squash(context, answers) -> Outcome`
- **Entry points**: **Undo** in the **…** menu of the header right after a squash merge (`undo.last` finds it in the
  journal); **Undo** in the message that follows the squash merge.
- **Offered when**: the journal's entry for the squash merge still holds: the top of the reflog of `HEAD` is what it was
  when the squash merge ran (nothing committed or moved since). Also after a squash merge that stopped on conflicts.
- **Plan**: `git reset --merge`.
- **Danger**: Confirm, because *What is staged goes back to HEAD, in the index and in the working tree, and so does what
  was staged since. Changes that are not staged stay.* The title is *Undo "Merge `<ref>` into `<current>` (squash)"*.
- **Undo**: none: what it took out can be staged again only by merging again.
- **Uses**: `undo.last`, `undo.journal`.
- **Extension**: **Undo** in the toolbar right after a squash merge, which `undo.last` finds in the journal. When the
  engine's binary is missing, it says so (D49).

Acceptance:

- [ ] Right after a squash merge, Undo runs `git reset --merge` and nothing is staged after it.
- [ ] After a squash merge that stopped on conflicts, Undo runs `git reset --merge` and no file is left in conflict.
- [ ] After a squash merge and a commit, Undo takes back the commit by the reflog instead.
- [ ] Extension: after a squash merge and a file staged, **Undo** runs `git reset --merge`, and nothing is staged after
      it.

### `rebase.onto` — Rebase the current branch onto a commit or a branch

Makes the commits of the branch checked out again on top of another commit, branch or remote branch.

- **Kind**: Operation
- **Function**: `features::rebase::onto(context, answers) -> Outcome`
- **Entry points**: **Rebase `<current branch>` onto this** in the context menu of a commit, a local branch and a remote
  branch; **Rebase `<current branch>` onto `<branch>`** in the menu of a branch dropped on the commit checked out;
  **Rebase `<current branch>` onto `<short hash>`** in the menu of the current branch dropped on another commit
  (`drag.drop`).
- **Offered when**: as `merge.into-current`: something is checked out and the target is not the commit checked out.
- **Plan**: `git rebase <onto>`, with a local branch named in full (`refs/heads/<name>`) when its short name is also a
  tag or a remote branch.
- **Danger**: None.
- **Undo**: by the reflog of the branch (`undo.last`): a rebase moves `HEAD` many times but the branch once, and the
  branch goes back to where it was before the rebase.
- **After**: when it stops on conflicts, the conflicts banner shows **Continue**, **Skip** and **Abort**.
- **Refuses**: nothing of its own; when git refuses (changes not committed), git's words are shown.
- **Uses**: `conflicts.continue-operation`, `conflicts.skip-step`, `conflicts.abort-operation`, `undo.last`.
- **Extension**: **Rebase `<current>` onto this** in the context menu of a commit, a local branch and a remote branch,
  where **Merge into `<current>`** is; **Rebase `<current>` onto `<name>`** and **Rebase `<current>` onto `<short>`** in
  the drop menu (`integrate` and `dropMenu` in `src/webview/menus.ts`). The action `rebase` with no branch runs it.
  When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] **Rebase main onto this** on `origin/main` runs `git rebase origin/main`.
- [ ] Dropping `main` (checked out) on commit `abc1234` and choosing **Rebase main onto abc1234** runs
      `git rebase <full hash>`.
- [ ] Undo after a clean rebase puts the branch back on the commit its reflog had before the rebase.
- [ ] Extension: **Rebase main onto this** on the branch `feature` runs `git rebase feature`, logged as `> git rebase
      feature`, without a question while `gitardo.commandPreview` is `off`.
- [ ] Extension: with changes not committed, git refuses and its failure is shown with **Show Log**; nothing changed.
- [ ] Extension: right after a rebase of three commits, one **Undo** runs `git reset --keep <previous>` and the three
      commits are as they were.

### `rebase.branch-onto` — Rebase another local branch onto a commit

Rebases a local branch that is not checked out onto the commit it is dropped on; git checks it out first.

- **Kind**: Operation
- **Function**: `features::rebase::branch_onto(context, answers) -> Outcome`
- **Entry points**: **Rebase `<branch>` onto `<short hash>`** in the menu of a local branch, not checked out, dropped on
  another commit (`drag.drop`).
- **Offered when**: the branch is local, is not at the commit it is dropped on, and no other worktree has it checked out.
- **Asks**: when no branch is given, which one, from the local branches not checked out here and not at the commit; a
  branch another worktree has checked out is in the list, described *checked out in another worktree*, and refused when
  picked.
- **Plan**: `git rebase <onto> <branch>`.
- **Danger**: None.
- **Undo**: by the reflog (`undo.last`). After it, `<branch>` is the branch checked out, as git leaves it.
- **Refuses**: *`<branch>` is checked out: rebase it onto `<short>` instead.*; *`<branch>` is checked out in another
  worktree: rebase it there.*
- **Uses**: `drag.drop`, `undo.last`.
- **Extension**: **Rebase `<name>` onto `<short>`** in the drop menu, for another local branch dropped on another
  commit, not for one another worktree has checked out (`dropMenu` in `src/webview/menus.ts`). The action `rebase` with
  a branch runs it on the commit, the branch given as the answer `branch`. When the engine's binary is missing, it says
  so (D49).

Acceptance:

- [ ] Dropping `feature` (not checked out) on `abc1234` offers only **Rebase feature onto abc1234**, which runs
      `git rebase <full hash> feature`.
- [ ] A branch that another worktree has checked out is not accepted by any row other than its own.
- [ ] Extension: dropping the local branch `topic`, not checked out, on `1a2b3c4` offers **Rebase topic onto 1a2b3c4**,
      which runs `git rebase <full hash> topic`; `topic` is checked out afterwards.
- [ ] Extension: a branch another worktree has checked out offers no rebase when dropped on a commit; asked to rebase
      it, the engine says *topic is checked out in another worktree: rebase it there.* before git runs.

### `rebase.find-line-before-head` — The line of commits an interactive rebase can rewrite

The commits that an interactive rebase starting at a commit would make again: from `HEAD` down to that commit, when they
form a single line without merges. It decides where the interactive rebase, **Edit message…**, **Split this commit…**,
**Squash…** and the fixup are offered.

- **Kind**: Derivation
- **Function**: `model::rebase::find_line_before_head(input) -> Option<Vec<CommitId>>`
- **Uses**: the commits of the timeline as loaded, each with its parents.
- **Extension**: in the views, `rebaseRange(hash, state)`, `canReword(hash, state)` and `canSquash(rows, state)` of
  `src/webview/menus.ts` decide what the menus offer: `canReword` is false on the row of the uncommitted changes and
  during an operation or a bisect, and true on `HEAD`, a merge or not. The engine works it out again for the feature it
  runs.

Rules:

- Input: the loaded rows, the commit checked out, the commit asked about. Output: the commits from `HEAD` down to the one
  asked about, newest first, both included, or nothing.
- Starting at `HEAD`, each step goes to the first parent. The walk gives nothing when it meets a merge (two parents or
  more), a commit that is not loaded, or the root before reaching the commit asked about.
- With nothing checked out, it gives nothing.

Examples (`HEAD` = `D`, parents `D→C→B→A`, `A` the root):

- `C` gives `D, C`; `A` gives `D, C, B, A`; `D` gives `D`.
- With `C` a merge of `B` and `X`: `D` gives `D`; `B`, `C` and `X` give nothing.
- `X`, on another branch, gives nothing.

Acceptance:

- [ ] Each example above holds.
- [ ] A commit further down than the rows loaded gives nothing.
- [ ] Extension: with the rows `e` → `d` → `c` → `b` (a merge) → `a`, `canSquash` is true for `e, d` and for `d, c`, and
      false for `e, c`, for `c, b`, for `e` alone and during a rebase.

### `rebase.read-pushed` — Where a commit was pushed

The remote branches that have a commit, read before a commit is rewritten, so that the confirmation can say the branch
will have to be force pushed.

- **Kind**: Read
- **Function**: `git::read::rebase::read_pushed(git, request) -> Result<Vec<String>, GitError>`
- **Entry points**: none: used by `message.edit`, `squash.commits`, `squash.into-previous` and `split.commit`.
- **Reads**: `git for-each-ref --contains <hash> --format=%(refname:short) refs/remotes`, leaving out the names that end
  in `/HEAD` and the ones with no `/`.
- **Extension**: the engine reads it for `message.edit`, `squash.commits` and `split.commit`, not for the fixup, the
  rebase editor, `rebase.onto` or the reset; the extension does not read it.

Rules:

- With one remote branch or more, the operation becomes Confirm, and its note starts with *`<short hash>` is pushed
  already, on `<where>`: the branch will have to be force pushed, and whoever has the old commits will have to move to
  the new ones.* `<where>` names up to three, joined by `, `, and with more says *and N more* after the third.

Acceptance:

- [ ] A commit on `origin/main` and `origin/feature` gives both, and the note names both.
- [ ] A commit on five remote branches is said as three names and *and 2 more*.
- [ ] A commit on no remote branch gives nothing and adds no note.
- [ ] Extension: rewording the last commit after it was pushed to `origin/reword` asks, even with
      `gitardo.commandPreview` set to `off`, with the note above.

### `rebase.read-stacked-branches` — The other branches on the commits a rewrite makes again

The local branches, other than the one rewritten, whose tip is one of the commits a rebase is about to make again: the
branches of a stack, each started from the one under it, which the rebase would leave on the old commits.

- **Kind**: Read
- **Function**: `git::read::rebase::read_stacked_branches(git: &Git, request: &StackedRequest) -> Result<StackedBranches, GitError>`
- **Entry points**: none: read by the operations of `rebase.update-refs` before they make their plan.
- **Reads**:
  - the commits made again: `git rev-list <base>..<tip> --`, `<tip>` the branch rewritten and `<base>` what the rebase
    starts from (the commit or branch it is onto for `rebase.onto` and `rebase.branch-onto`, the base of the todo list
    for an interactive rebase); `git rev-list <tip> --` when the interactive rebase starts at the root;
  - the local branches with the folder of the worktree that has each checked out:
    `git for-each-ref --format=%(refname)%00%(objectname)%00%(worktreepath)%00 refs/heads`;
  - git's own setting: `git config --type=bool --get rebase.updateRefs`, which ends with 1 when it is not set.
- **Extension**: the engine reads it for the operations it runs; the extension does not read it.

The result is each branch whose tip is among the commits made again, but the one rewritten, with its commit and the
worktree that has it checked out, if any; and whether git's setting is `true`, `false` or not set.

As built, a feature asks for it with `Query::StackedBranches { base, tip }`. The worktree of each branch is the one
`%(worktreepath)` names, also the worktree the rebase runs in, so that the plan tells the branch of another worktree from
its own.

Acceptance:

- [ ] On `p3`, with `p1` at its second commit and `p2` at its third above `main`, a rebase onto `main` gives `p1` and
      `p2`, each with its commit.
- [ ] A branch at `<base>` itself is not given: that commit is not made again.
- [ ] A branch at the same commit as the tip of the branch rewritten is given.
- [ ] A branch checked out in another worktree is given with the folder of that worktree.
- [ ] git's `rebase.updateRefs` set to `true` is given as set to true; not set, as not set.

### `rebase.update-refs` — Which branches move with a rewrite

Decides whether a rewrite takes the other branches of a stack with it, and says which in its plan, so that a branch on
a commit made again is not left on the old one.

- **Kind**: Derivation
- **Function**: `model::rebase::update_refs(input: &UpdateRefsInput) -> UpdateRefs`
- **Settings**: `rebase.updateRefs` of Gitardo: `true`, `false`, or not set, its default, where git's own
  `rebase.updateRefs` decides, and moving them when that is not set either (Q1130).
- **Uses**: `rebase.read-stacked-branches`, `repo.read-git-version`, `rebase.build-todo-list`; by `rebase.onto`,
  `rebase.branch-onto`, `rebase.interactive` and the operations built on it (`message.edit`, `squash.commits`,
  `squash.into-previous`, `split.commit`), `fixup.commit`, and so by `drag.drop`; `undo.moved-branches`.
- **Extension**: the engine's, in the plans of those operations, which the extension runs on the engine; the setting is
  `gitardo.rebase.updateRefs`.

Rules:

- Input: the version of git, Gitardo's setting and git's, the branches read by `rebase.read-stacked-branches`, and,
  for an interactive rebase, its steps. Output: the option of the rebase, the `update-ref` lines of the todo list, the
  branches that move and those that stay, and the note of the plan.
- The branches move when Gitardo's setting is `true`, or when it is not set and git's is not `false`.
- As built: each rewrite reads the version of git and the branches of the stack before it makes its plan. The note
  that says which branches move is said after the run by `rebase.onto` and `rebase.branch-onto`, whose plan asks
  nothing when they all move; the rewrites whose todo list is Gitardo's, and `fixup.commit`, which ask first anyway,
  end their confirmation with it, and say after the run "`p1` and `p2` moved with the rewrite."
- With no branch on the commits made again, nothing changes: no option, no line, no note.
- A rebase with git's own todo list (`rebase.onto`, `rebase.branch-onto`, `fixup.commit`) gets `--update-refs` when
  the branches move, and `--no-update-refs` when they do not and git's setting is `true`, so that the command says what
  happens whatever the configuration of git.
- An interactive rebase whose todo list is Gitardo's (`rebase.interactive`) replaces git's list, `update-ref` lines
  included, so the option would do nothing: `rebase.build-todo-list` writes `update-ref refs/heads/<branch>` itself
  after the line of the commit each branch is on, and after its `exec` line when it is reworded, so that the branch
  takes the reworded commit. A branch on a commit dropped moves to the commit kept before it; on a commit squashed or
  fixed up, to the commit it becomes.
- A branch checked out in another worktree never moves: git leaves it out of `--update-refs`, and Gitardo writes no
  line for it, since a line moves it all the same and leaves the files and the index of that worktree on the old
  commit (Q1132).
- Git older than 2.38 has neither the option nor the line: no option, no line, and the branches stay.
- The note of the plan, after its own: "These branches move with it: `p1`, `p2`."; for those that stay, "`p2` is
  checked out in `<folder>` and stays on the old commits: rebase it there.", "Git `<version>` cannot move `p1` and `p2`
  with the rewrite (2.38 or later can): they stay on the old commits.", or, with the setting off, "`p1` and `p2` stay on
  the old commits." A plan that leaves a branch behind is Confirm at least; one that moves them all keeps its danger.
- Once it ran, what is said after names the branches that moved: "Rebased `p3` onto `main`; `p1` and `p2` moved with
  it."

Acceptance:

- [ ] Rebasing `p3` onto `main`, with `p1` and `p2` under its tip, runs `git rebase --update-refs main`; `p1` and `p2`
      are on the new commits after, and what is said names them.
- [ ] With Gitardo's `rebase.updateRefs` set to `false` and git's set to `true`, it runs
      `git rebase --no-update-refs main`, asks first, and the note says that `p1` and `p2` stay.
- [ ] With no other branch on the commits made again, the command has neither option.
- [ ] With git 2.37, the command has neither option, and it asks first with the note that says which branches stay
      and why.
- [ ] With `p2` checked out in another worktree, the note says that it stays, and that worktree's files and index are
      as they were after the rebase.
- [ ] Editing the message of the commit `p1` is on (`message.edit`) writes `update-ref refs/heads/p1` after the `exec`
      line of the reword, and `p1` is on the reworded commit after.
- [ ] Squashing the commit `p1` is on into the one before it moves `p1` to the commit they become.
- [ ] Extension: rebasing `p3` onto `main` from the graph runs `git rebase --update-refs main` in the engine, and the
      modal dialog of the plan, when it shows, names `p1` and `p2`.

### `rebase.todo-page` — The interactive rebase page

The list of the commits from a commit up to `HEAD`, to reorder by dragging and to pick, reword, edit, squash, fixup or
drop each one before the rebase starts.

- **Kind**: View
- **Function**: `views::rebase::todo_page_view(state, ui) -> Vec<Intent>`
- **Entry points**: **Interactive rebase of N commits…** in the context menu of a commit, or **Edit this commit
  (interactive rebase)…** when that commit is the one checked out.
- **Offered when**: `rebase.find-line-before-head` gives a line for the commit, and no merge, rebase, cherry-pick,
  revert, patch or bisect is under way.
- **Uses**: `rebase.find-line-before-head`, `rebase.interactive`.
- **Extension**: a dialog over the Graph view, `RebaseEditor` (`src/webview/RebaseEditor.tsx`), opened by **Interactive
  rebase of N commits…** or **Edit this commit (interactive rebase)…** in the context menu of a commit in the graph or
  the Gitardo Graph. **Start rebase** posts the action `rebaseInteractive` with the steps oldest first, the base, and
  each subject reworded with `keepBody`.

What it shows:

- A header, *Interactive rebase of `<branch>`: N commits* (*1 commit*; `HEAD` when detached), and *Newest first. Drag to
  reorder.*
- One row per commit, newest first as in the timeline: a grip to drag it by, the action (**Pick**, **Reword**, **Edit**,
  **Squash**, **Fixup**, **Drop**) with its help as tooltip, the short hash, the subject, and **Move up** and **Move
  down** buttons, disabled at the ends. Every row starts as **Pick**.
- The tooltips of the actions: Pick *Keep the commit as it is*; Reword *Keep the changes, change the message*; Edit
  *Stop right after this commit, to amend it or commit more, then continue*; Squash *Meld into the commit below,
  keeping both messages*; Fixup *Meld into the commit below, discarding this message*; Drop *Remove the commit*.
- With **Reword**, the subject becomes a box with the subject in it, labelled *New subject*; what is typed replaces the
  subject and the body of the message stays.
- After the short hash, the other local branches on the commit, each a label with the icon of a branch: in the color of
  the accent, with the tooltip *`<branch>` moves with the rebase*; muted, with *`<branch>` is checked out in another
  worktree and stays on the old commit*, for one another worktree has checked out (`rebase.update-refs`; Q1131). The
  confirmation of the rebase says which move, as the setting and the version of git decide.
- A footer with what keeps it from running, **Cancel** and **Start rebase**.

What keeps it from running, said in the footer, and **Start rebase** is disabled:

- every row is **Drop**: *Every commit is dropped. To remove them all, reset the branch instead.*
- the lowest row that is not dropped is **Squash** or **Fixup**: *The oldest commit that is kept has nothing below it
  to meld into.*
- a **Reword** row has an empty box: *Enter a message for each reworded commit.*
- nothing changed (every row **Pick**, in the order it started in): **Start rebase** is disabled, with nothing said.

Acceptance:

- [ ] Opening it on the third commit under `HEAD` lists three rows, `HEAD` first.
- [ ] Dragging the lowest row to the top, or pressing **Move up** on it twice, puts it first.
- [ ] Setting every row to **Drop** shows the first message and disables **Start rebase**.
- [ ] **Cancel** closes the page and runs nothing.
- [ ] **Start rebase** closes the page and calls `rebase.interactive` with the rows oldest first.
- [ ] The row of a commit another local branch is on shows that branch, marked as moving with the rebase, and one
      another worktree has checked out as staying.
- [ ] While a bisect is under way, no commit offers the page, nor squashing into the commit before it, nor splitting.
- [ ] Extension: on the third commit below `HEAD` of a line without merges, the menu has **Interactive rebase of 3
      commits…**, and on `HEAD` **Edit this commit (interactive rebase)…**; neither below a merge, while an operation is
      under way, or during a bisect.
- [ ] Extension: the dialog lists the commits newest first, all **Pick**, and **Start rebase** is disabled until
      something changes.

### `rebase.build-todo-list` — The todo list of an interactive rebase

Turns the rows of the interactive rebase page, or the steps another operation prepares, into the todo list git reads,
and the files of the new messages.

- **Kind**: Derivation
- **Function**: `model::rebase::build_todo_list(steps: &[TodoStep], folder: &str, moving: &[StackedBranch]) -> Result<TodoList, Text>`
- **Extension**: `todoRows` (`src/engine/actions.ts`) writes the steps of the rebase editor as the rows of
  `rebase.interactive`: `<action> <full hash>`, and for a reword its new subject, from which the engine builds this
  list.

Rules:

- Input: the steps oldest first, each an action (`pick`, `reword`, `edit`, `squash`, `fixup`, `drop`) and a full hash,
  and for `reword` its new message and whether that message is only a new subject; the original message of each
  reworded commit; the folder the files go in. Output: the text of the todo list, and the files of the messages.
- Each step is one line, `<action> <hash>`, in the order given, except `reword`, which is two lines: `pick <hash>` and
  `exec git commit --amend -F "<folder>/message-<n>.txt"`, `<n>` being the index of the step from 0. Paths are written
  with forward slashes. The text ends with a line break.
- Each branch that moves with the rewrite (`rebase.update-refs`) is a line `update-ref refs/heads/<branch>` after the
  lines of the commit it is on, the `exec` of a reword included, so that it takes the reworded commit; and after the
  squashes, fixups and drops that follow that commit, so that a branch on a commit squashed or fixed up takes the
  commit it becomes, and one on a commit dropped the commit kept before it.
- The message file of a reword holds the new message, trimmed, and a line break. When the message is only a new subject,
  the body of the original message (what follows its first blank line, trimmed) follows it after a blank line.
- A list where every step is `drop` is refused: *Every commit is dropped. To remove them all, reset the branch instead.*
- A list whose first step that is not `drop` is `squash` or `fixup` is refused: *The oldest commit that is kept has
  nothing before it to be squashed into.*
- A hash that is empty or starts with `-` is refused.

Example: steps `pick A`, `reword B` with the subject *Better words* (B's message *Old words* + blank line + *Body.*),
`fixup C`, `drop D` give:

```
pick A
pick B
exec git commit --amend -F "<folder>/message-1.txt"
fixup C
drop D
```

and `message-1.txt` holds *Better words*, a blank line, *Body.* and a line break.

Acceptance:

- [ ] The example above gives exactly that text and that file.
- [ ] A reword whose message is given whole (not only a subject) writes that message alone.
- [ ] `drop A`, `drop B` is refused with the first message; `drop A`, `squash B` with the second.
- [ ] With `p1` on a reworded commit and `p2` and `p3` on a commit and the one squashed into it, `p1` is written after
      the `exec` line, and `p2` and `p3` after the `squash` line.
- [ ] Extension: the steps `pick A`, `reword B` with the subject *Add two, new* and `fixup C` are written as the rows
      `pick A`, `reword B Add two, new` and `fixup C`, one a line.

### `rebase.interactive` — Run an interactive rebase

Runs `git rebase -i` with the todo list prepared on the interactive rebase page, by having git's editor of the todo list
copy the prepared one over it, since no editor can be typed into.

- **Kind**: Operation
- **Function**: `features::rebase::interactive(context, answers) -> Outcome`
- **Entry points**: **Start rebase** on the interactive rebase page (`rebase.todo-page`). The same plan is what
  `message.edit`, `squash.commits`, `squash.into-previous` and `split.commit` build for an older commit.
- **Offered when**: as `rebase.todo-page`.
- **Plan**:
  1. `write <git directory of the worktree>/gitardo-rebase/message-<n>.txt` for each reworded commit;
  2. `write <git directory of the worktree>/gitardo-rebase/todo`, the list built by `rebase.build-todo-list`;
  3. `git -c 'sequence.editor=cp "<folder>/todo"' -c core.editor=true rebase -i [--autostash] <base>`:
     the value of `sequence.editor` is `cp "<path of the todo file>"`, quotes included, so that git's own todo list is
     replaced by the prepared one. `<base>` is the first parent of the oldest commit of the list, or `--root` when the
     oldest has none. `core.editor=true` keeps the messages git prepares for squashes. `--autostash` is added by the
     operations built on it, not by the page.

  Before it, the version of git and the branches on the commits made again are read (`rebase.update-refs`): the
  branches that move are `update-ref` lines of the todo list, and the confirmation ends with the note that names them
  and those that stay.
- **Danger**: Confirm, because it rewrites history: *This rewrites the commits after `<short base>` with this todo
  list:* (or *after the root*) and the todo list. The title is *Interactive rebase of `<branch>`*.
- **Undo**: by the reflog of the branch (`undo.last`): the branch goes back to where it was before the rebase.
- **After**: when the rebase ended (there is no `rebase-merge` folder in the git directory of the worktree), the
  `gitardo-rebase` folder is deleted, and so is it after the plan fails. When it stopped, on a conflict or to edit a commit, the folder stays, since the `exec`
  lines still to run read the message files. A stop to edit shows the banner of `rebase.edit-stop`.
- **Refuses**: as `rebase.build-todo-list`; when git refuses to start (changes not committed and no `--autostash`),
  git's words are shown.
- **Uses**: `rebase.build-todo-list`, `rebase.read-stop`, `rebase.edit-stop`, `conflicts.continue-operation`, `undo.last`.
- **Extension**: **Start rebase** in the dialog of the rebase editor posts `rebaseInteractive`, which runs it on `HEAD`
  with the rows as the answer `todo` (`todoRows` and `engineRuns` in `src/engine/actions.ts`). A reword that gives a
  whole message, which the rows cannot say, runs nothing; the rebase editor gives only a new subject (`keepBody`). When
  the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] Rewording the second of three commits runs a rebase whose todo has `pick`, `pick` and `exec git commit --amend -F`
      for it, and the commit gets the new subject and keeps its body.
- [ ] Reordering two commits makes them in the new order, with new hashes.
- [ ] **Drop** on a commit leaves it out of the branch.
- [ ] **Squash** on a commit melds it into the one below, with both messages; **Fixup** with the message of the one below.
- [ ] A list whose oldest commit is the root runs `git rebase -i --root`.
- [ ] The confirmation shows the todo list before anything runs.
- [ ] After a clean rebase the `gitardo-rebase` folder is gone; after a stop it is there.
- [ ] Extension: the todo list and the message files are written into `<git folder>/gitardo-rebase` once the user
      agrees, each with a `write` line in the log, and the folder is gone once the rebase ends.
- [ ] Extension: while a bisect is under way, the rebase of the rebase editor is refused with *End the bisect first.*
- [ ] Extension: with a file changed and not committed, git refuses and nothing is rewritten.

### `rebase.read-stop` — Where an interactive rebase stopped to change a commit

Tells a stop to edit a commit, or to split one, from a stop on a conflict, and names the commit.

- **Kind**: Read
- **Function**: `git::read::rebase::read_stop(git, request) -> Result<Option<RebaseStop>, GitError>`
- **Entry points**: none: read with the state of the repository while a rebase is under way.
- **Reads**: in the git directory of the worktree, `rebase-merge/amend`, `rebase-merge/done` and
  `rebase-merge/gitardo-split`; `git log -1 --format=%s <hash> --` for the subject.
- **Extension**: the engine reads it with the state of the repository (`repo.read-state`, asked by `Reads.parts` in
  `src/engine/reads.ts`), and the extension shows it as `rebaseStop` of `RepoState`.

Rules:

- Without `rebase-merge/amend`, there is no stop to change a commit.
- Git writes `amend` when it stops on `edit` and leaves it there when the rebase goes on, so the step done last must be
  that `edit`: the last line of `done` that is neither empty nor a comment (`#`) must be `edit <hash>` or `e <hash>`.
  Otherwise the stop is on a conflict further on, and there is none.
- When `gitardo-split` holds a full hash that starts with the hash of that last line, the stop is a split of that
  commit (hash from `gitardo-split`); otherwise it is an edit, of the commit in `amend`.
- The result is the kind (edit or split), the full hash and the subject.

Acceptance:

- [ ] A rebase stopped on `edit abc1234` gives an edit of the commit in `amend`, with its subject.
- [ ] After that edit, a later `pick` that stops on a conflict gives nothing, though `amend` is still there.
- [ ] With `gitardo-split` holding the hash of the commit of the last `edit`, it gives a split.
- [ ] With no rebase under way, it gives nothing.

### `rebase.edit-stop` — The banner of a rebase stopped to edit a commit

While an interactive rebase is stopped right after a commit marked **Edit**, the banner says so and what to do.

- **Kind**: View
- **Function**: `views::rebase::edit_stop_view(state, ui) -> Vec<Intent>`
- **Entry points**: the banner above the timeline, in place of the banner of an operation in progress.
- **Offered when**: `rebase.read-stop` gives an edit.
- **Uses**: `rebase.read-stop`, `conflicts.continue-operation`, `conflicts.abort-operation`.
- **Extension**: the banner of the Graph view, in the panel or an editor tab, and of the Changes view (`OperationBanner`
  in `src/webview/OperationBanner.tsx`). **Continue** is the action `continue`, which runs
  `conflicts.continue-operation`; the engine hands over to this and refuses as above.

What it shows:

- An edit icon and *Stopped at "`<subject>`" to edit it. Amend it or commit more, then Continue.*
- **Continue**, with the tooltip *Go on with the rebase: the commits after it are made again on top*; disabled with the
  tooltip *Resolve the conflicts and stage the files first* while files are in conflict.
- **Abort**, which takes the whole rebase back (`conflicts.abort-operation`).
- No **Skip**: it would throw away what is not committed and keep the commit anyway.

What **Continue** does here:

- With staged changes, or changes to tracked files not staged, it refuses: *Some changes are not committed: amend the
  commit with them, commit them, or stash them, then continue.* Untracked files do not count.
- Otherwise it runs `git -c core.editor=true rebase --continue`, which works as well when nothing was changed. This is
  the operation `features::rebase::continue_edit(context, answers) -> Outcome`, registered as `rebase.edit-stop`.

Acceptance:

- [ ] Stopped at a commit *Add the login page*, the banner says *Stopped at "Add the login page" to edit it. Amend it or
      commit more, then Continue.*
- [ ] The banner has **Continue** and **Abort**, and no **Skip**.
- [ ] **Continue** with a file modified and not committed is refused with the message above, and git is not run.
- [ ] **Continue** after amending the commit makes the commits after it again on top.
- [ ] Extension: **Continue** with nothing changed goes on as well; a commit made at the stop stays between the edited
      one and the ones after.
- [ ] Extension: a later step of the same rebase that stops on a conflict shows `Rebase in progress · N files have
      conflicts`, not the text of the edit.

### `message.edit` — Edit a commit message

Gives a commit a new message: the last one by amending it, an older one by an interactive rebase that makes the commits
after it again.

- **Kind**: Operation
- **Function**: `features::message::edit(context, answers) -> Outcome`
- **Entry points**: the pencil beside the message in the inspector (tooltip *Edit message*); **Edit message…** in the
  context menu of a commit.
- **Offered when**: the commit is not the draft; no merge, rebase, cherry-pick, revert, patch or bisect is
  under way; and the commit is the one checked out (merge or not) or `rebase.find-line-before-head` gives a line for it.
- **Asks**: the whole message, in a box in the inspector, put in place of the message with the cursor at its end;
  *Ctrl+Enter to save* beside **Cancel** and **Save**. `Ctrl`+`Enter` (`Cmd`+`Enter` on macOS) or **Save** gives the
  commit the new message, trimmed; `Escape` or **Cancel** leaves it as it was. **Save** is disabled while the box is
  empty or the message is unchanged.
- **Reads**: `rebase.read-pushed`. For an older commit, the line after it up to `HEAD`:
  `git merge-base --is-ancestor <hash> HEAD`, then `git rev-list --topo-order --parents <hash>..HEAD`, which must be one
  line with no merge, each the parent of the one before, down to `<hash>`, which must not be a merge either; and
  `git rev-parse --verify --quiet <hash>^` for the base. (`squash.commits`, `squash.into-previous` and `split.commit`
  read the same with `git rev-list --parents -n 1 <hash> --`, `git merge-base --is-ancestor <hash> HEAD` and
  `git log -z --topo-order --format=%H%x00%P%x00%B HEAD --not <parents of hash> --`, which gives the messages too.)
- **Plan**:
  - the commit checked out: `git commit --amend --only --allow-empty -F -`, with the message and a line break on
    standard input. `--only` leaves out what is staged.
  - an older commit: `rebase.interactive` with `--autostash` and the steps `reword <hash>` with the whole message, then
    `pick` for each commit after it, oldest first. The title is *Edit the message of `<short hash>`*, and the note *`<short
    hash>` gets the new message, and the commit after it is made again on top of it, with new hashes.* (or *the N commits
    after it are*), with *Changes that are not committed are set aside and put back.* when there are some. Its files
    go into `<git folder>/gitardo-rebase`, as for `rebase.interactive`, deleted once no rebase is under way, also when it
    stopped on a commit made again after it.
- **Danger**: for the last commit, None; for an older one, Confirm (a rewrite of history). Both become Confirm when the
  commit is pushed, with the note of `rebase.read-pushed` first.
- **Undo**: by the reflog (`undo.last`): an amend is taken back with `git reset --soft` to the commit before it; a rebase
  by the reflog of the branch.
- **After**: the commit with its new message is selected: `HEAD`, or the commit as many before `HEAD` as there were
  commits after it.
- **Refuses**: *Finish or abort the `<operation>` first.*; *End the bisect first.*; an empty message, *Enter a
  message.*; an older commit with no line to `HEAD`, *`<short hash>` is not on the line of commits before HEAD without
  merges: its message cannot be changed by a rebase from here.*
- **Uses**: `rebase.find-line-before-head`, `rebase.read-pushed`, `rebase.interactive`, `undo.last`.
- **Extension**: the pencil beside the message in the details of the Graph view, tooltip **Edit message**, and **Edit
  message…** in the context menu of a commit open the box `MessageEditor` (`src/webview/Detail.tsx`), labelled *Commit
  message*, from 3 to 12 lines high as the text grows; saving the text unchanged with the keys closes it as **Cancel**
  does, so the engine's *The message is unchanged.* is not reached from VS Code. The action `reword` runs it on the
  commit with the message as the answer `message`. When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] Saving a new message for the last commit runs `git commit --amend --only --allow-empty -F -` with the message on
      standard input, and what was staged stays staged.
- [ ] Saving for the third commit under `HEAD` runs one interactive rebase with `--autostash`; that commit has the new
      message and the two after it are made again; the new third commit is selected.
- [ ] A commit on `origin/main` asks first and says the branch will have to be force pushed.
- [ ] `Escape` in the box leaves the message as it was and runs nothing.
- [ ] While a bisect is under way, neither the pencil nor **Edit message…** is offered.
- [ ] Saving for an older commit when a commit made again after it stops on a conflict leaves
      `<git folder>/gitardo-rebase` until the rebase is continued to its end or aborted, then deletes it.
- [ ] Extension: saving *Reword three, new* on the last commit runs `git commit --amend --only --allow-empty -F -` with
      that message on standard input, without a question while `gitardo.commandPreview` is `off`; a file staged before
      stays staged and out of the commit.
- [ ] Extension: right after rewording the last commit, **Undo** runs `git reset --soft <previous>` and the old message
      is back; after rewording an older one, **Undo** runs `git reset --keep <previous>`.
- [ ] Extension: the pencil opens a box with the message *Subject*, an empty line and *Body*, the cursor after *Body*;
      **Save** is disabled while the text is the message as it was, and while it is empty.

### `squash.join-messages` — The message of commits squashed into one

Puts together the message of the commit that several become.

- **Kind**: Derivation
- **Function**: `model::squash::join_messages(subject: &str, messages: &[&str]) -> String`
- **Extension**: the engine's, for `squash.commits`; the extension does not work it out.

Rules:

- Input: the subject given, and the messages of the commits, oldest first. Output: the message.
- The subject of a message is its first paragraph, on one line (line breaks inside it become single spaces); its body
  is what follows the first blank line, trimmed.
- The message is the subject given, trimmed, then each message trimmed, joined by blank lines. A message whose subject
  is the subject given adds only its body; an empty part adds nothing.

Example: subject *Add the login page*; messages *Add the login page* + blank line + *With a form.*, then *Fix a typo*.
The result is *Add the login page*, blank line, *With a form.*, blank line, *Fix a typo*.

Acceptance:

- [ ] The example above holds.
- [ ] Three messages with no body and a new subject give the subject and the three messages, separated by blank lines.

### `squash.commits` — Squash commits into one

Makes one commit of several that follow one another before `HEAD`, with the changes of them all.

- **Kind**: Operation
- **Function**: `features::squash::commits(context, answers) -> Outcome`
- **Entry points**: **Squash into one** in the selection bar; **Squash N commits into one…** in the context menu of the
  selection; `gitardo-cli run squash.commits commits:<hash>,<hash>…`. Its target is the commits selected.
- **Offered when**: two commits or more are selected; the oldest of them has a line to `HEAD`
  (`rebase.find-line-before-head`) and the others are the commits right after it on that line, with none missing; and
  no merge, rebase, cherry-pick, revert, patch or bisect is under way.
- **Reads**: the line after the oldest up to `HEAD` with the message of each commit, as `message.edit` says; the
  oldest is not known before it is read, so it is read again from the commit it found below the one asked about until
  the line holds every commit selected; `rebase.read-pushed` for the oldest.
- **Asks**: a box titled *Squash N commits*, *Subject of the commit they become. What their messages say goes under
  it.*, filled with the subject of the oldest; an empty subject is refused while typing, *Enter a subject*.
- **Plan**: `rebase.interactive` with `--autostash` and the steps: `reword <oldest>` with the message of
  `squash.join-messages`, `fixup` for each of the others, then `pick` for each commit after them up to `HEAD`. The
  todo is `pick <oldest>`, `exec git commit --amend -F "<file>"`, the `fixup` lines and the `pick` lines. The title is
  *Squash N commits*, and the note *`<short>, <short>, …` become one commit, with the changes of them all.*, then *The
  commit after them is made again on top of it.* (or *The N commits after them are…*) when there are some, and *Changes
  that are not committed are set aside and put back.* when there are some.
- **Danger**: Confirm, because it rewrites history; with the note of `rebase.read-pushed` first when pushed.
- **Undo**: the whole squash, by the reflog of the branch (`undo.last`).
- **After**: the commit they become is selected: as many before `HEAD` as there were commits after them.
- **Refuses**: *Finish or abort the `<operation>` first.*; *End the bisect first.*; when they do not follow one another
  on that line, *Only commits that follow one another on the line before HEAD, without merges, can be squashed into
  one.*
- **Uses**: `rebase.find-line-before-head`, `squash.join-messages`, `rebase.read-pushed`, `rebase.interactive`,
  `undo.last`.
- **Extension**: **Squash into one** in the bar above the graph that counts the selected commits (tooltip *Makes them
  one commit, with a message you give*), **Squash N commits into one…** in the context menu of the selection, and
  **Squash into the commit before it…** in the context menu of a single commit post the action `squash` with the
  commits, which runs it; the subject is asked in an input box. When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] Squashing `B`, `C` and `D` of `A→B→C→D→E` (`E` = `HEAD`) gives `A→X→E'`, `X` with the changes of the three and
      the message joined, and `X` is selected.
- [ ] Selecting `B` and `D` without `C` does not offer it.
- [ ] Closing the box runs nothing.
- [ ] With a file modified and not committed, the squash runs and the file is modified again after it.
- [ ] One Undo puts the branch back on `E`.
- [ ] While a bisect is under way, it is not offered, and it is refused with *End the bisect first.*
- [ ] Extension: selecting *Add b*, *Add c* and *Add d* and choosing **Squash into one** with the subject *Add b, c and
      d* makes one commit whose message is the subject, then their messages, oldest first; the commit after them is made
      again on top of it, and the commit they became is selected.
- [ ] Extension: pushed commits are asked with the warning of the force push.

### `squash.into-previous` — Squash a commit into the commit before it

The same as squashing two commits, from the context menu of the newer one.

- **Kind**: Operation
- **Function**: `features::squash::into_previous(context, answers) -> Outcome`
- **Entry points**: **Squash into the commit before it…** in the context menu of a commit.
- **Offered when**: the commit has a line to `HEAD`, it has one parent, and that parent has a line to `HEAD` too; no
  operation or bisect is under way.
- **Asks**, **Reads**, **Plan**, **Danger**, **Undo**, **After**, **Refuses**: as `squash.commits` with the commit and
  its parent.
- **Uses**: `squash.commits`.
- **Extension**: **Squash into the commit before it…** posts `squash` with the commit and its parent, which runs
  `squash.commits`, whose plan is the same, rather than this feature.

Acceptance:

- [ ] On `HEAD` whose parent is not a merge, the entry is there and makes one commit of the two.
- [ ] On the root commit, the entry is not there.
- [ ] On a commit whose parent is a merge, the entry is not there.
- [ ] Extension: **Squash into the commit before it…** on `e`, whose parent `d` is on the line, squashes `d` and `e`.

### `split.commit` — Split a commit

Takes the changes of a commit out of it into the working tree, unstaged, to commit them again in as many commits as
wanted.

- **Kind**: Operation
- **Function**: `features::split::commit(context, answers) -> Outcome`
- **Entry points**: **Split this commit…** in the context menu of a commit.
- **Offered when**: where `message.edit` is, and the commit has exactly one parent (not a merge, not the first commit).
- **Reads**: `git rev-list --parents -n 1 <hash>` for its parents; `rebase.read-pushed`; for an older commit, the line
  after it, as `message.edit` reads it.
- **Plan**:
  - the commit checked out: `git reset --mixed HEAD~`. The note says *The changes of `<short>` go back into the working
    tree, unstaged, and the branch goes back to the commit before it: commit them in as many commits as you like.*, and
    *The changes that are not committed stay, among them.* when there are some.
  - an older commit: `rebase.interactive` with `--autostash` and the steps `edit <hash>`, then `pick` for each commit
    after it; then `git reset --mixed HEAD~`; then `write <git directory of the worktree>/rebase-merge/gitardo-split`
    with the full hash and a line break, which goes with the rest of that folder when the rebase ends. The note says
    *The rebase stops at `<short>` with its changes taken out of it, unstaged: commit them in as many commits as you
    like, then Continue, and the commit after it is made again on top.* (or *the N commits after it are*), and *Changes
    that are not committed are set aside until the end.* when there are some.
- **Danger**: Confirm, because it rewrites history; with the note of `rebase.read-pushed` first when pushed.
- **Undo**: the last commit, right after: the journal keeps `git reset --mixed <hash of the commit split>`, since the
  commit may have added files that are untracked now. An older one: **Abort** takes it all back while it is stopped, and
  after the end Undo takes back the whole split by the reflog of the branch.
- **After**: the message of the draft becomes the message of the commit split. For the last commit, the message
  *The changes of "`<subject>`" are in the working tree, unstaged: commit them in as many commits as you like.* with
  **Undo**. For an older one, the banner *Splitting "`<subject>`": commit its changes in as many commits as you like,
  then Continue.*, with **Continue** (`split.continue`) and **Abort**, and no **Skip**.
- **Refuses**: *Finish or abort the `<operation>` first.*; *End the bisect first.*; a hash that is not a commit; a
  merge, *`<short>` is a merge: it cannot be split.*; the first commit, *`<short>` is the first commit, with nothing
  before it to take its changes back to: it cannot be split.*; an older commit with no line to `HEAD`, *`<short>` is not
  on the line of commits before HEAD without merges: it cannot be split by a rebase from here.*
- **Uses**: `rebase.find-line-before-head`, `rebase.read-pushed`, `rebase.interactive`, `rebase.read-stop`,
  `split.continue`, `undo.last`, `undo.journal`.
- **Extension**: **Split this commit…** in the context menu of a commit in the graph and the Gitardo Graph posts
  `split`, which runs it; the commit box of the Changes view gets the message of the commit (the follow-up
  `fillCommitMessage`), and after a split of the last commit the message has no button: **Undo** in the toolbar takes
  the split back. When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] Splitting `HEAD` runs `git reset --mixed HEAD~`; its files are changed and unstaged, and the draft holds its
      message.
- [ ] Undo right after puts `HEAD` back on the commit split, with a file it added tracked again.
- [ ] Splitting the second commit under `HEAD` stops the rebase with that commit's changes unstaged, `gitardo-split` in
      `rebase-merge`, and the banner text above.
- [ ] The entry is not on a merge commit nor on the root commit.
- [ ] While a bisect is under way, the entry is not there, and the split is refused with *End the bisect first.*
- [ ] Extension: **Split this commit…** on the last commit asks **Split `<short>`?**, runs `git reset --mixed HEAD~`,
      and says *The changes of "`<subject>`" are in the working tree, unstaged: commit them in as many commits as you
      like.*; right after, **Undo** runs `git reset --mixed <hash>`, and nothing is left changed.
- [ ] Extension: on an older commit, the question says *the commit after it is made again on top*, the banner says
      *Splitting "`<subject>`": …*, and the commit box holds the whole message of the commit.

### `split.continue` — Continue a split

What **Continue** of the banner does while a rebase is stopped to split a commit: go on once its changes are committed,
or commit what is left of them as one more commit.

- **Kind**: Operation
- **Function**: `features::split::continue_split(context, answers) -> Outcome`
- **Entry points**: **Continue** in the banner while `rebase.read-stop` gives a split (`conflicts.continue-operation` hands over
  to this).
- **Offered when**: a rebase is stopped to split a commit.
- **Reads**: the files of the commit split, both names of a renamed one:
  `git diff-tree -r --no-commit-id --name-only --no-renames -z --root <hash>`; the changes not committed. The files of the
  commit still changed are those, staged or not, whose path or old path is among them; the other changes are the rest of
  the staged and unstaged ones, untracked files left out. A file the commit added is untracked once its changes are
  taken out of it: it counts among the files of the commit still changed.
- **Asks**: when files of the commit are still changed, a dialog *"`<subject>`" is not all committed yet: 1 file of it
  is still changed.* (or *N files of it are*), with the detail *Commit what is left as one more commit, with the message
  the commit had, and continue? Or go back to committing it yourself.*, and **Commit what is left** (the default) and
  **Go back to committing**.
- **Plan**: with nothing of the commit left, `git -c core.editor=true rebase --continue`. With **Commit what is left**:
  `git add --all -- <files left>`, `git commit --quiet -m <message of the commit split>`,
  `git -c core.editor=true rebase --continue`.
- **Danger**: None.
- **Undo**: after the end, the whole split, by the reflog of the branch.
- **After**: as `conflicts.continue-operation`; `cherry-pick.skip-empty` does not apply to a rebase, which drops empty commits
  itself.
- **Refuses**: when other files have changes not committed, *`<file>` has changes that are not committed and are not
  part of the split: commit or stash them, then continue.* (or *N files have changes…*); **Go back to committing** runs
  nothing.
- **Uses**: `rebase.read-stop`, `conflicts.continue-operation`.
- **Extension**: **Continue** in the banner is the action `continue`, which runs `conflicts.continue-operation`; the
  engine hands over to this, and its dialog is asked with VS Code's (`VscodeAsker` in `src/engine/vscode.ts`).

Acceptance:

- [ ] With all the changes of the commit committed in two commits, **Continue** runs `rebase --continue` and the commits
      after it are made again on top.
- [ ] With one file of the commit not committed, the dialog says *1 file of it is still changed*, and **Commit what is
      left** commits it with the commit's message, then continues.
- [ ] With a file not part of the commit modified, **Continue** is refused and names it.
- [ ] An untracked file not part of the commit does not keep it from continuing.
- [ ] Extension: **Commit What Is Left** runs `git add --all -- c.txt`, `git commit --quiet -m <message of the commit>`
      and `git -c core.editor=true rebase --continue`.

### `fixup.commit` — Add changes to an earlier commit

Folds the staged changes, or all the changes when nothing is staged, into an earlier commit, and makes the commits after
it again.

- **Kind**: Operation
- **Function**: `features::fixup::commit(context, answers) -> Outcome`
- **Entry points**: **Add staged changes to this commit (fixup)**, or **Add all changes to this commit (fixup)** when
  nothing is staged, in the context menu of a commit.
- **Offered when**: `rebase.find-line-before-head` gives a line for the commit; it is not the one checked out (**Amend**
  covers that); there are staged changes, or changes not staged; no file is in conflict; and no operation or bisect is
  under way.
- **Plan**: when nothing is staged, `git add --all` first; then `git commit --fixup=<hash>`; then
  `git -c sequence.editor=true rebase -i --autosquash --autostash [--update-refs | --no-update-refs] <hash>^`, or
  `--root` in place of `<hash>^` when the commit is the first; the option as `rebase.update-refs` decides, and the
  confirmation ends with its note. `sequence.editor=true` accepts the todo list git prepares itself, which already moves the
  `fixup!` commit under its target.
- **Danger**: Confirm, because *This rewrites `<short>` and every commit after it. If the changes do not fit there, the
  rebase stops with conflicts.* The title is *Add staged changes to `<short>`* or *Add all changes to `<short>`*.
- **Undo**: by the reflog of the branch (`undo.last`).
- **After**: when the changes do not fit, the rebase stops with conflicts like any other.
- **Refuses**: *Finish or abort the `<operation>` first.*; *End the bisect first.*; with no change, *There are no
  changes to add to that commit.*
- **Uses**: `rebase.find-line-before-head`, `conflicts.continue-operation`, `undo.last`.
- **Extension**: the entries of the context menu of a commit in the graph (`commitMenu` in `src/webview/menus.ts`) post
  `fixup`, which runs it. When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] With `a.txt` staged and `b.txt` modified, the fixup on `HEAD~2` runs `git commit --fixup=<hash>` and the rebase;
      after it `HEAD~2` has the change to `a.txt`, and `b.txt` is still modified.
- [ ] With nothing staged and `b.txt` modified, the entry is **Add all changes to this commit (fixup)** and the plan
      starts with `git add --all`.
- [ ] On the commit checked out, the entry is not there.
- [ ] On the root commit the rebase runs with `--root`.
- [ ] While a bisect is under way, the entry is not there, and the fixup is refused with *End the bisect first.*
- [ ] Extension: with `b.txt` staged and `later.txt` untracked, **Add staged changes to this commit (fixup)** on *Add b*
      asks, even with `gitardo.commandPreview` set to `off`; then *Add b* has the new `b.txt`, and `later.txt` is still
      there.

### `drag.list-drop-entries` — What a drop offers

Decides which rows of the timeline accept what is dragged, and what dropping it there offers.

- **Kind**: Derivation
- **Function**: `model::drag::list_drop_entries(input: &DropInput) -> Vec<DropEntry>`
- **Extension**: `dropMenu(source, target, state)` of `src/webview/menus.ts` gives the same entries; the row of the
  uncommitted changes is a checked-out row too.

Rules (an empty list means the row does not accept the drop):

- Nothing is accepted while nothing is checked out (no commit yet) or a merge, rebase, cherry-pick, revert or patch is
  under way.
- The checked-out row is the row of the commit checked out, and the draft.
- A commit, onto the checked-out row, unless it is the commit checked out: **Cherry-pick `<short>` onto `<current>`**
  (`cherry-pick.commit`). Onto any other row: nothing.
- A branch or remote branch, onto the checked-out row, unless it is the branch checked out or its tip is the commit
  checked out: **Merge `<name>` into `<current>`** (`merge.into-current`), **Merge `<name>` into `<current>`…**
  (`merge.choose-way`), **Rebase `<current>` onto `<name>`** (`rebase.onto`).
- A remote branch, onto any other row: nothing.
- The local branch checked out, onto another commit: **Rebase `<name>` onto `<short>`** (`rebase.onto`), **Reset
  `<name>` to `<short>`…** (`reset.to-commit`).
- Another local branch, onto another commit, unless another worktree has it checked out: **Rebase `<name>` onto
  `<short>`** (`rebase.branch-onto`).
- A branch onto the row of its own tip: nothing.
- `<current>` is the branch checked out, or `HEAD` when detached.

Acceptance:

- [ ] A commit dragged over a row that is not checked out gives an empty list.
- [ ] `feature` dragged onto the commit checked out on `main` gives the three entries, merge first.
- [ ] `main`, checked out, dragged onto `abc1234` gives the rebase and the reset.
- [ ] Any drag while a rebase is stopped gives an empty list.
- [ ] Extension: a branch that another worktree has checked out, dropped on another commit, offers nothing.

### `drag.drop` — Drag a commit or a branch onto the timeline

Drag a commit, or a branch from the timeline or the branch rail, onto a row of the timeline to cherry-pick, merge,
rebase or reset.

- **Kind**: View
- **Function**: `views::drag::drop_view(state, ui) -> Vec<Intent>`
- **Entry points**: dragging a row of the timeline (a commit), a branch label of the timeline, or a branch or remote
  branch of the branch rail, onto a row of the timeline.
- **Uses**: `drag.list-drop-entries`, `cherry-pick.commit`, `merge.into-current`, `merge.choose-way`, `rebase.onto`,
  `rebase.branch-onto`, `reset.to-commit`.
- **Extension**: in the Graph view, a commit row (not the uncommitted changes, not a stash), the label of a local or
  remote branch on a row, or a branch of the references beside the graph, dragged onto a row (`startDrag` and `dragging`
  in `src/webview/drag.ts`); the entries run as the same entries of the context menus.

| Drag | Onto | Offers |
| --- | --- | --- |
| A commit | The checked-out commit | Cherry-pick it onto the current branch |
| A branch | The checked-out commit | Merge it into the current branch, as git does by default or in the way chosen, or rebase the current branch onto it |
| The current branch | Any other commit | Rebase it onto that commit, or reset it there |
| Another local branch | Any other commit | Rebase it onto that commit |

Rules:

- While dragging, each row that accepts the drop (a non-empty list) is outlined as the pointer is over it; a row that
  does not accept it shows that nothing can be dropped there.
- Dropping opens a menu at the pointer with the entries of `drag.list-drop-entries`; choosing one runs its feature, which
  asks and confirms as it does from a context menu. `Escape` or a click elsewhere closes the menu and runs nothing.
- Dropping where nothing is accepted does nothing.

Acceptance:

- [ ] Dragging a commit and dropping it on the commit checked out opens a menu with one entry, **Cherry-pick `<short>`
      onto `<current>`**.
- [ ] Dragging `feature` from the branch rail onto the commit checked out outlines that row, and the menu has the two
      merges and the rebase.
- [ ] Dropping the current branch on another commit and choosing **Reset … to …** asks soft, mixed or hard.
- [ ] Dropping on a row that does not accept the drop opens no menu.
- [ ] Extension: dropping the local branch `feature` on the commit checked out on `main` offers exactly **Merge feature
      into main**, **Merge feature into main…** and **Rebase main onto feature**.
- [ ] Extension: while an operation is under way, no drop offers anything.

### `range-diff.read-last-rewrite` — Read what a branch was before it was rewritten

Where a branch was before its last rebase or amend, and what the rebase was onto, from the reflog of the branch; and
where a remote branch was before a fetch found it force pushed.

- **Kind**: Read
- **Function**: `git::read::range_diff::read_last_rewrite(git: &Git, request: &LastRewriteRequest) -> Result<Option<Rewrite>, GitError>`
- **Entry points**: none: read by `range-diff.compare` and by what offers it.
- **Reads**: `git log -g -z --max-count=50 --format=%H%x00%gs <reference> --`, the last moves of the branch or of the
  remote branch, newest first, each the commit it moved to and what git wrote of the move.
- **Extension**: the engine's, for `range-diff.compare`.

Rules:

- For a local branch, the newest of those moves that is a rewrite:
  - `<what> (finish): refs/heads/<branch> onto <hash>`, as git writes it for `git rebase`, `git rebase -i` and
    `git pull --rebase`, is a rebase: the commit of that move is the tip after it, the commit of the move before it
    the tip before, and `<hash>` the commit it was onto;
  - `commit (amend): <subject>` is an amend: the tip after and the tip before, as for a rebase, each with its parent as
    its base.
- For a remote branch, the newest move that ends in `forced-update`, as a fetch writes it when the remote branch was
  force pushed: the commit of that move and the commit of the move before it.
- A reference with none of these among its 50 last moves gives nothing; so does a move whose commit before is no longer
  in the repository.

As built, a feature asks for it with `Query::LastRewrite { reference }`, the full name of the reference. A reference
with no reflog, or none at all, gives nothing; an amend says whether the commit before has a parent, read with
`git rev-parse --verify --quiet <before>^`, and so does the check that the commit before is still there.

Acceptance:

- [ ] After `git rebase main` on `feature`, it gives the tip before, the tip after and the commit of `main` it was onto.
- [ ] After `git pull --rebase`, it gives the same, onto the commit of the upstream.
- [ ] After an amend, it gives the commit before and the commit after, with their parents.
- [ ] After a commit made on top of a rebase, it still gives the rebase, with the tip it made.
- [ ] After a fetch that found `origin/feature` force pushed, it gives where `origin/feature` was before and after.
- [ ] A branch that was only ever committed to gives nothing.

### `range-diff.ranges` — The two lines of commits compared

Which commits were there before and which are there now, for each way of reaching the page of what a rewrite changed.

- **Kind**: Derivation
- **Function**: `model::range_diff::ranges(input: &RangesInput) -> Option<Ranges>`
- **Uses**: `range-diff.read-last-rewrite`; by `range-diff.compare`.
- **Extension**: the engine's, for `range-diff.compare`.

Rules, each giving the arguments of `git range-diff`:

- After a rebase onto `<base>`: before, the commits from `<base>` to the tip before; now, from `<base>` to the tip
  after: `<base> <tip before> <tip after>`, the form of three that git reads as `<base>..<tip before>` and
  `<base>..<tip after>`.
- After an amend: `<before>^..<before> <after>^..<after>`. An amend of the first commit, which has no parent, gives
  nothing.
- A branch and its upstream: when the last rewrite of the branch started from the commit its upstream is at (it was
  pushed, then rebased), the ranges of that rebase, `<base> <upstream> <branch>`; otherwise the commits each has that
  the other has not, `<upstream>...<branch>`, which git reads as `<branch>..<upstream>` and `<upstream>..<branch>`. In
  that form the commits a rebase onto a newer base brought in are among the commits added (Q1137).
- A remote branch force pushed: `<before>...<after>`.
- Two sides that are the same commit give nothing.

Acceptance:

- [ ] A rebase of `feature` from `a1` to `b2` onto `m3` gives `m3 a1 b2`.
- [ ] An amend of `c4` into `c5` gives `c4^..c4 c5^..c5`.
- [ ] `feature`, rebased after it was pushed to `origin/feature` and not pushed since, gives the ranges of the rebase,
      from `origin/feature`.
- [ ] `feature` and `origin/feature` that each have commits the other has not, with no rebase from the upstream, give
      `origin/feature...feature`.

### `range-diff.read-pairs` — Read what a rewrite changed, commit by commit

The commits of two lines paired, as `git range-diff` pairs them: the same commit made again, a commit changed with
what changed in it, a commit added, a commit dropped.

- **Kind**: Read
- **Function**: `git::read::range_diff::read_pairs(git: &Git, request: &RangeDiffRequest) -> Result<RangeDiff, GitError>`,
  with `git::read::range_diff::parse_pairs(output: &[u8]) -> Result<Vec<Pair>, GitError>`
- **Reads**:
  - how many commits each line has, `git rev-list --count <range> --` for each; more than 1,000 on either side are
    not compared (Q1138);
  - `git range-diff --no-color --no-notes --abbrev=64 <ranges>`, the ranges of `range-diff.ranges`: git takes a length
    past that of its hashes as the whole hash, 40 characters or 64;
  - the subject of each commit named, `git log --no-walk=unsorted --stdin -z --format=%H%x00%s`, the hashes given on its
    standard input, as there may be more than a command line holds: on the line of a pair git writes the subject of
    the commit before, and the page shows the one now.
- **Uses**: `range-diff.ranges`, `activity.track`, which says **Comparing the commits of `<branch>`** while it runs for
  more than half a second.
- **Extension**: the engine's, asked by the tab of `range-diff.page`.

Git has no form of `range-diff` made for programs, so this read parses the one it prints for people, against rule
R9.3 (Q1139). Each pair is a line `<n>:  <hash> <mark> <m>:  <hash> <subject>`, the numbers padded with spaces, a commit
missing on one side written `-` with a line of `-` in place of its hash; the mark is `=` (the same), `!` (changed), `>`
(added, only now) or `<` (dropped, only before). The lines after a pair that start with four spaces are its diff of
diffs: with the four spaces taken off, each starts with the mark of the outer diff (a space, `-` before, `+` now), then
the line of the diff it is in, under headings `## Commit message ##`, `## <path> ##` and `## Metadata ##`.

As built, a heading is written with a space before it, as a line of the diff it is in, and each part also starts a hunk
of the outer diff, `@@ Commit message`, `@@ <path>`. A pair changed in its message, or in what it changes, has a line
of `-` or `+` under that heading or in that hunk; a heading with no line changed under it, there to give the lines
around, does not count.

Acceptance:

- [ ] After a rebase in which one commit was reworded and one dropped, it gives a pair the same, a pair changed whose
      diff of diffs is under `## Commit message ##`, and a pair dropped.
- [ ] A commit only in the second line is a pair added, with no commit before.
- [ ] Every hash is given in full.
- [ ] A recorded output whose subject holds ` ! ` and `:` is read the same.
- [ ] Two lines of 1,200 commits are not compared, and the error says how many there are.

### `range-diff.compare` — See what a rewrite changed

Opens, commit by commit, what a rebase or an amend changed in a branch, what a force push would replace, or what
someone else's force push replaced.

- **Kind**: Operation
- **Function**: `features::range_diff::compare(context, answers) -> Outcome`
- **Entry points**:
  - **See what changed** in what is said after a rebase, an interactive rebase and the operations built on it, an
    amend and a pull that rebased, once they ended and made at least one commit again;
  - **What its last rewrite changed** in the context menu of a local branch;
  - **Compare with `<upstream>` commit by commit** in the context menu of a local branch that has commits its upstream
    has not and the other way round, as of the last fetch, and among the other ways to push in the menu of **Sync**
    while the branch checked out is so;
  - **Compare first** in the warning of `sync.push-force-with-lease` and of `sync.push-force`, which closes it, runs
    nothing and opens the page (Q1140);
  - after a fetch that found the upstream of a local branch force pushed, "`<remote branch>` was force pushed." with
    **See what changed**; and **What the force push changed** in the context menu of that remote branch.
- **Extension**: the same entries in the menus of the graph and the references (`branchMenu` and `remoteBranchMenu` in
  `src/webview/menus.ts`), in **More push options** beside **Push**, and on the messages the extension says after an
  operation (`Controller.afterSync`, `Controller.say`); each sends `feature/run` of `range-diff.compare` to Gitardo's
  engine (`Controller.runOnEngine` in `src/extension/controller.ts`), whose follow-up opens the tab of `range-diff.page`.
  The warning of a force push is VS Code's modal dialog, which gets **Compare First**. When the engine's binary is
  missing, it says so (D49).
- **Reads**: `range-diff.read-last-rewrite` for the branch, or for the remote branch; the upstream, from the state of the
  repository.
- **Plan**: no step; it changes nothing.
- **Danger**: None
- **Undo**: Nothing to undo: it changes nothing.
- **After**: the page of what changed opens (`range-diff.page`), with the ranges of `range-diff.ranges`.
- **Refuses**: "`<branch>` has not been rebased or amended lately." when its reflog has no rewrite;
  "`<branch>` and `<upstream>` are at the same commit." when there is nothing to compare.
- **Uses**: `range-diff.read-last-rewrite`, `range-diff.ranges`, `range-diff.page`, `sync.push-force-with-lease`,
  `sync.push-force`, `sync.read-overwritten`, `sync.fetch`, `sync.pull`, `rebase.onto`, `rebase.interactive`,
  `commit.amend`.

Acceptance:

- [ ] Right after **Rebase feature onto main**, what is said has **See what changed**, which opens the page with the
      ranges of that rebase.
- [ ] **What its last rewrite changed** on a branch rebased in a terminal opens the same page.
- [ ] On a branch that was never rebased or amended, it says so and opens nothing.
- [ ] Before **Push (force with lease)** of a branch rebased after it was pushed, **Compare first** in the warning
      opens the page of the upstream against the branch, and nothing is pushed.
- [ ] After a fetch that found `origin/feature` force pushed, while `feature` follows it, what is said offers **See
      what changed**, which compares `origin/feature` before and after.
- [ ] Extension: **What its last rewrite changed** on a branch rebased by the engine opens the tab of the page.

### `range-diff.page` — What a rewrite changed, commit by commit

The commits of a branch before and after a rewrite, paired: the ones made again unchanged, the ones changed with what
changed in each, the ones added and the ones dropped.

- **Kind**: View
- **Function**: `views::range_diff::page(state: &RewriteState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: `range-diff.compare`.
- **Reads**: `range-diff.read-pairs`.
- **Uses**: `range-diff.read-pairs`, `range-diff.compare`, `commit-page.open`, `file-view.find`.
- **Extension**: a tab of the editor area of its own, as the comparison is (`CompareApp`), with the same rows; a click
  on a hash opens the tab of that commit.

What it shows, from the top:

- The title of what was compared: **What the rebase of `feature` changed**, **What the amend of `feature` changed**,
  **`feature` and `origin/feature`, commit by commit**, or **What the force push of `origin/feature` changed**; the
  page is named `<branch> · what changed` in its tab and its title row.
- The two lines: **Before**, its count of commits and the short hash of the commit it starts from; **Now**, the same;
  then the count of each kind of pair, `2 the same · 1 changed · 1 added · 1 dropped`.
- One row for each pair, in the order of the commits now, a dropped commit where it was: an icon for its kind, the
  short hash before and the short hash now joined by an arrow, the subject now (before, for a dropped one), and for a
  changed one what changed in it, **message**, **changes** or both, from the headings of its diff of diffs. The pairs
  that are the same are dimmed (Q1141).
- A changed row opens by a click, `Enter` or `→`, and shows its diff of diffs under it: each line with its outer mark
  in the colors of a diff (before in the color of lines lost, now in that of lines gained), then the line of the diff
  it is in, its own mark kept; the headings `## Commit message ##` and `## <path> ##` as headings.
- A click on a short hash opens the page of that commit (`commit-page.open`).
- `↑` and `↓` move through the rows; `Ctrl+F` (`Cmd+F` on macOS) finds a word in the page (`file-view.find`).
- While it is read, **Comparing…**; when the read fails, what git said.

Acceptance:

- [ ] After a rebase in which `B` was reworded and `D` dropped, the page lists `A` dimmed as the same, `B` changed with
      **message**, `C` the same and `D` dropped, and the counts say `2 the same · 1 changed · 1 dropped`.
- [ ] A click on the row of `B` shows its diff of diffs, the old subject in the color of lines lost and the new one in
      that of lines gained.
- [ ] A click on the short hash of `B` now opens the page of that commit.
- [ ] A commit added by the rewrite has a row with no hash before.
- [ ] Extension: the tab lists the same rows, and opening a changed row shows its diff of diffs.
