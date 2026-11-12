# Merge, rebase and rewriting history

Bringing the work of one branch into another and changing commits that are made already: merging in the four ways git
has, rebasing what is checked out or another local branch, the interactive rebase and its editor, cherry-picking one
commit or several, reverting, resetting, and the rewrites Gitardo builds on the interactive rebase (giving a commit a
new message, squashing, splitting and fixing up). Also what a drop on a row of the graph offers, the conflicts said
before a merge or a cherry-pick, and the bisect with its banner. What **Continue**, **Skip** and **Abort** do in the
banner of an operation under way is in `operation.continue`, `operation.skip` and `operation.abort`; this file says
what a stop to edit or to split a commit needs from them. **Undo** itself is `undo.last`; each card here says what it
does right after.

### `merge.into-current` — Merge into the current branch

Merges a branch, a remote branch or a commit into what is checked out, as git's own settings say: a fast-forward when
one is possible, otherwise a merge commit.

- **Kind**: Operation
- **Function**: `mergeOp(ref, into, 'default')` in `src/git/ops.ts`, called by `merge(ref, asked, state, repo)` in
  `src/extension/actions.ts`.
- **Entry points**: **Merge into `<current>`** in the context menu of a commit, of a local branch (label in the graph,
  references, Branches view) and of a remote branch; **Merge `<name>` into `<current>`** in the menu of a branch dropped
  on the commit checked out (`rewrite.drag-and-drop`). `<current>` is the branch checked out, or `HEAD` when it is
  detached.
- **Offered when**: a commit is checked out and the commit of the entry is not it (`integrate` in `menus.ts`); a local
  branch only when it is not the one checked out. The context menus do not hide it while an operation is under way;
  the drop menu does.
- **Reads**: the files that will conflict (`merge.preview-conflicts`), and `Repo.canFastForwardTo(ref)`
  (`git merge-base --is-ancestor HEAD <ref>`), which only the fast-forward-only way uses.
- **Plan**: `> git merge <ref>`. `<ref>` is the branch by its name, `refs/heads/<name>` or `refs/remotes/<name>` when
  the short name would mean something else (`refArg`), or the full hash of a commit. Title of the operation:
  `Merge <name> into <current>`, a hash shortened to 7 characters.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`) | Confirm, when the preview finds conflicts: the
  question says `This will stop with conflicts in N files:` and names them.
- **Undo**: `undo.last` from the reflog: the branch goes back to where it was with `git reset --keep <previous>`
  (`--mixed` when the files already are as that commit has them), after asking. While the merge is stopped on
  conflicts, **Undo** refuses with *Finish or abort the merge first.*: `operation.abort` takes it back.
- **After**: on conflicts, the message `Merge <name> into <current> stopped: N files have conflicts. Resolve them, then
  continue or abort the merge.` and the banner `Merge in progress · N files have conflicts`, with **Continue** and
  **Abort** (`conflict.review`, `operation.continue`, `operation.abort`).
- **Refuses**: a ref that starts with `-`: `"<ref>" is not a valid name.`
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `merge.preview-conflicts`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `merge` (`src/shared/protocol.ts:501`); `resolveAction` case `merge`
  (`src/extension/actions.ts:268`) and `merge()` (`actions.ts:615-650`); `MERGE_MODES` and `mergeOp`
  (`src/git/ops.ts:308-318`); `mergeEntries` and `integrate` (`src/webview/menus.ts:34-44`), used by `commitMenu`
  (`menus.ts:195`), `branchMenu` (`menus.ts:323`), `remoteBranchMenu` (`menus.ts:414`) and `dropMenu`
  (`menus.ts:585-589`); `Controller.action` (`src/extension/controller.ts:1628`).
- **Tested by**: `test/integration.test.ts` *creates, renames, merges and deletes branches*; *merges as git does by
  default, always with a merge commit, as a fast-forward only, or as a squash, and undoes each*; *reports a conflicted
  merge, aborts it, then resolves and continues it*; `test/menus.test.ts` *merges with one click as git does by
  default, and asks for the other ways with the entry after it*; `test/e2e/suite.ts` *stops on a conflicting merge,
  says so first, opens the merge editor and aborts cleanly*.

Acceptance:

- [ ] **Merge into main** in the menu of the branch `feature` runs `git merge feature` and nothing else, as an operation
  through `Controller.run`; the log has the line `> git merge feature`.
- [ ] With `gitardo.commandPreview` `off` and no conflict expected it runs without a question; with `all`, the question
  **Merge feature into main?** says `No conflicts expected.` above `git merge feature`.
- [ ] When the merge will conflict in `a.txt`, it asks first whatever the setting, with `This will stop with conflicts
  in 1 file:` and `a.txt`; more than 15 files are cut to 15 and `…and N more`.
- [ ] The command shown is the command that runs. It has no `--no-edit`: git runs without a terminal and takes the
  message it prepares without opening an editor.
- [ ] A branch `v1` that shares its name with a tag is merged as `git merge refs/heads/v1`; a ref starting with `-` is
  refused with `"-x" is not a valid name.` and nothing runs.
- [ ] The entry is not in the menu of the commit checked out, nor of the branch checked out.
- [ ] Right after a merge that moved `main`, **Undo** asks, then runs `git reset --keep <previous>`; while the merge is
  stopped, **Undo** says *Finish or abort the merge first.*

### `merge.choose-way` — Choose the way to merge

The entry after **Merge into `<current>`**, which asks how to merge: with a merge commit always, as a fast-forward
only, or as a squash merge, as the menu beside **Pull** has the ways to pull.

- **Kind**: Operation
- **Function**: `merge(ref, 'ask', state, repo)` in `src/extension/actions.ts`, which builds `mergeOp(ref, into, mode)`
  of `src/git/ops.ts`.
- **Entry points**: **Merge into `<current>`…**, with the hint *choose how*, right after **Merge into `<current>`** in
  the same menus; **Merge `<name>` into `<current>`…** in the drop menu.
- **Offered when**: as `merge.into-current`.
- **Asks**: a list titled `Merge <name> into <current>`, each with its command and what it does:
  **Merge** (`git merge <name>`, *A fast-forward when one is possible, otherwise a merge commit*);
  **Merge (no fast-forward)** (`git merge --no-ff <name>`, *Always a merge commit, even where a fast-forward would
  do*); **Merge (fast-forward only)** (`git merge --ff-only <name>`, *`<current>` moves up to `<name>`, with no merge
  commit*, or *Not possible: `<current>` has commits that `<name>` has not*); **Merge (squash)** (`git merge --squash
  <name>`, see `merge.squash`).
- **Reads**: `Repo.canFastForwardTo(ref)` before the list, to say whether a fast-forward only is possible; for a
  fast-forward only that is not, `Repo.divergence('HEAD', ref)`; for the other ways, `merge.preview-conflicts`.
- **Plan**: `> git merge <ref>`, `> git merge --no-ff <ref>` or `> git merge --ff-only <ref>`, with the title
  `Merge <name> into <current>`, `… (no fast-forward)` or `… (fast-forward only)`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`) | Confirm, when the preview finds conflicts. A
  fast-forward only says `A fast-forward: no conflicts.` and is never previewed.
- **Undo**: as `merge.into-current`: `undo.last` from the reflog, `git reset --keep <previous>`.
- **Refuses**: a fast-forward only when `<current>` has commits that `<ref>` has not, before git runs: *`<current>`
  cannot be fast-forwarded to `<name>`: it has commits that `<name>` has not. Merge it another way, or rebase
  `<current>` onto it.*, or, when `<ref>` has nothing more either, *`<current>` already has every commit of `<name>`.*
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `merge.into-current`, `merge.squash`, `merge.preview-conflicts`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `merge` with `mode: 'ask'` (`src/shared/protocol.ts:501`); `MergeMode`
  (`protocol.ts:346`); `merge()` (`src/extension/actions.ts:615-650`); `MERGE_MODES` and `mergeOp`
  (`src/git/ops.ts:308-318`); `mergeEntries` (`src/webview/menus.ts:34-37`).
- **Tested by**: `test/integration.test.ts` *merges as git does by default, always with a merge commit, as a
  fast-forward only, or as a squash, and undoes each*; `test/menus.test.ts` *merges with one click as git does by
  default, and asks for the other ways with the entry after it*; *offers the same when a branch is dropped on what is
  checked out*; `test/e2e/suite.ts` *merges in the way chosen, refuses a fast-forward only that cannot be one, and
  undoes a squash, which moves no branch*.

Acceptance:

- [ ] The list offers, in this order, **Merge**, **Merge (no fast-forward)**, **Merge (fast-forward only)**,
  **Merge (squash)**, each with its `git merge` command as its description.
- [ ] **Merge (no fast-forward)** of `feature` into `main` runs `git merge --no-ff feature`, and makes a merge commit
  even where a fast-forward would do; it is logged as `> git merge --no-ff feature`.
- [ ] When `main` has a commit that `feature` has not, **Merge (fast-forward only)** says so in the list, and choosing
  it shows *main cannot be fast-forwarded to feature: …* without running git.
- [ ] **Merge (fast-forward only)** that is possible runs `git merge --ff-only feature`: `main` moves to `feature`, no
  merge commit; with `gitardo.commandPreview` `all` the question says `A fast-forward: no conflicts.`
- [ ] Every way but the fast-forward only asks first when the preview finds conflicts, and names the files.
- [ ] Escape on the list runs nothing.
- [ ] Right after any of them that moved the branch, **Undo** runs `git reset --keep <previous>` after asking.

### `merge.squash` — Squash merge

Brings the changes of a branch in as one change, staged and not committed, with no merge recorded; the commit is the
user's to make.

- **Kind**: Operation
- **Function**: `mergeOp(ref, into, 'squash')` in `src/git/ops.ts`, through `merge()` in `src/extension/actions.ts`;
  its Undo is `undoSquashOp(title)` in `src/git/ops.ts`.
- **Entry points**: **Merge (squash)** in the list of `merge.choose-way`.
- **Offered when**: as `merge.into-current`.
- **Reads**: `merge.preview-conflicts`; after it ran, `Repo.squashMessage()`, the message git prepared in `SQUASH_MSG`.
- **Plan**: `> git merge --squash <ref>`, titled `Merge <name> into <current> (squash)`. The question says first
  `The changes of <name> are staged in <current> as one change, for you to commit. No merge is recorded: <name> will
  not show as merged.`
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`) | Confirm, when the preview finds conflicts.
- **Undo**: the reflog cannot take it back, as no branch moves: Gitardo remembers `undoSquashOp`, which asks
  **Undo "Merge `<name>` into `<current>` (squash)"?** with *What is staged goes back to HEAD, in the index and in the
  working tree, and so does what was staged since. Changes that are not staged stay.* and runs `git reset --merge`.
  It is remembered also when the squash stopped on conflicts, and holds until the reflog of `HEAD` moves (a commit) or
  another operation runs in Gitardo.
- **After**: the commit box of the Changes view, when it is empty, gets the message of `SQUASH_MSG`, which lists the
  commits brought in; a message says *Gitardo: the changes are staged, and nothing is committed yet. Commit them to
  make them one commit on `<current>`.* with **Show Changes** (opens the Changes view) and **Undo**. Stopped on
  conflicts, there is no operation under way to continue or abort: the banner says only `N files have conflicts`.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `merge.choose-way`, `merge.preview-conflicts`, `commit.create`, `undo.last`, `preview.ask`, `log.write`.
- **Implemented in**: `merge()` (`src/extension/actions.ts:646-649`); `isSquashMerge` and `undoSquashOp`
  (`src/git/ops.ts:321-334`); `Controller.action` (`src/extension/controller.ts:1675-1682`, `1702-1705`);
  `Controller.reportSquashed` (`controller.ts:1769-1777`); `Repo.squashMessage` (`src/git/repo.ts:232`); the commit
  box keeps what is written (`src/webview/ChangesApp.tsx:85-87`).
- **Tested by**: `test/integration.test.ts` *merges as git does by default, always with a merge commit, as a
  fast-forward only, or as a squash, and undoes each*; *undoes a squash merge that stopped on conflicts, which leaves
  no merge to abort*; `test/e2e/suite.ts` *merges in the way chosen, refuses a fast-forward only that cannot be one,
  and undoes a squash, which moves no branch*.

Acceptance:

- [ ] **Merge (squash)** of `feature` into `main` runs `git merge --squash feature`: the changes are staged, `HEAD`
  does not move, nothing is committed; the log has `> git merge --squash feature`.
- [ ] The question, when it is asked, starts with *The changes of feature are staged in main as one change, for you to
  commit. No merge is recorded: feature will not show as merged.*
- [ ] An empty commit box gets the message git prepared, which names the commits of `feature`; a box with text keeps
  it.
- [ ] Right after it, **Undo** asks **Undo "Merge feature into main (squash)"?**, then runs `git reset --merge`, and
  the staged changes are gone.
- [ ] After a squash merge that stopped on conflicts, **Undo** runs `git reset --merge` too, and no file is left in
  conflict.
- [ ] Once the staged changes are committed, **Undo** takes back that commit from the reflog instead.

### `merge.preview-conflicts` — Say which files will conflict

Before a merge or a cherry-pick, works out which files will stop in conflict, without touching the index or the
working tree, and asks first when there are some.

- **Kind**: Read
- **Function**: `Repo.conflictsWith(theirs, base?, ours = 'HEAD')` in `src/git/repo.ts`, and
  `withConflictPreview(op, conflicts)` in `src/extension/actions.ts`, which puts the result in the question.
- **Entry points**: none of its own: the merges of `merge.into-current`, `merge.choose-way` and `merge.squash`, and
  `cherry-pick.one` for a commit that is not a merge.
- **Plan**: `git merge-tree --write-tree --no-messages --name-only -z [--merge-base=<base>] HEAD <theirs>`, with
  `GIT_OPTIONAL_LOCKS=0`. Exit 0 is no conflict, 1 is conflicts: the first record is the tree, the others the paths,
  each once. Any other exit, as with a git older than 2.40 or histories that never met, is "cannot tell".
- **Rules**: a merge is computed from the commit the two have in common; a cherry-pick of `<hash>` from `<hash>^`. No
  conflict: the operation's note is `No conflicts expected.`, and nothing more is asked. Conflicts: the operation becomes
  `confirm`, with the note `This will stop with conflicts in 1 file:` or `in N files:`, the first 15 names one per line,
  and `…and N more`. Cannot tell: the operation is left as it was, with no note.
- **Uses**: `merge.into-current`, `merge.choose-way`, `merge.squash`, `cherry-pick.one`, `preview.ask`.
- **Implemented in**: `Repo.conflictsWith` (`src/git/repo.ts:1027-1034`) through `Repo.query` (`repo.ts:1012`);
  `withConflictPreview` (`src/extension/actions.ts:120-127`), called by `merge()` (`actions.ts:646`) and the
  `cherryPick` case (`actions.ts:365`).
- **Tested by**: `test/integration.test.ts` *predicts conflicts of a merge and of a cherry-pick without touching
  anything*; `test/e2e/suite.ts` *stops on a conflicting merge, says so first, opens the merge editor and aborts
  cleanly*.

Acceptance:

- [ ] For a branch `other` that changes `a.txt` where `HEAD` changed it too, the merge preview is `['a.txt']`, and the
  working tree, the index and `HEAD` are as they were.
- [ ] For a commit that conflicts, the cherry-pick preview from its parent is `['a.txt']`; for one that does not, `[]`.
- [ ] A merge that will conflict is asked first even with `gitardo.commandPreview` `off`.
- [ ] With 20 files in conflict the question names 15 and ends with `…and 5 more`.
- [ ] With a git that has no `--merge-base` for `merge-tree`, the merge runs as it would with no preview: no note, no
  extra question.
- [ ] The preview runs with `GIT_OPTIONAL_LOCKS=0` and is not written to the log at the info level.

### `rebase.onto` — Rebase onto a commit or a branch

Makes the commits of the current branch again on top of another commit or branch, or does it for another local branch
dropped on a commit.

- **Kind**: Operation
- **Function**: `rebaseOp(onto, current, other?)` in `src/git/ops.ts`.
- **Entry points**: **Rebase `<current>` onto this** in the context menu of a commit, a local branch and a remote
  branch; in the drop menu (`rewrite.drag-and-drop`): **Rebase `<current>` onto `<name>`** for a branch dropped on the
  commit checked out, **Rebase `<name>` onto `<short>`** for the current branch, or another local branch, dropped on
  another commit.
- **Offered when**: in the context menus, as `merge.into-current` (not on the commit checked out, not on the branch
  checked out). In the drop menu, not while an operation is under way, and not for a branch that another worktree has
  checked out or that is at the commit it is dropped on.
- **Plan**: `> git rebase <onto>` for the current branch, `> git rebase <onto> <branch>` for another one, which git
  checks out first and leaves checked out. Title `Rebase <branch> onto <short>`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`). The README's list of what is always asked does
  not name a rebase that is not interactive, and no conflicts are previewed.
- **Undo**: `undo.last` from the reflog of the branch, which a rebase moves once: `git reset --keep <previous>` takes
  every commit back at once. After a rebase of another branch, that branch is moved back and stays checked out. While
  the rebase is stopped, **Undo** says *Finish or abort the rebase first.*
- **After**: on conflicts, the banner `Rebase in progress · N files have conflicts` with **Continue**, **Skip** and
  **Abort** (`operation.continue`, `operation.skip`, `operation.abort`).
- **Refuses**: a name or hash that starts with `-`: `"<name>" is not a valid name.` Git refuses a rebase with changes
  that are not committed, as nothing is set aside: the error says why (`Rebase main onto feature failed. error: cannot
  rebase: You have unstaged changes.`).
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `rewrite.drag-and-drop`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `rebase` (`src/shared/protocol.ts:503`); `resolveAction` case `rebase`
  (`src/extension/actions.ts:271-272`); `rebaseOp` (`src/git/ops.ts:337-341`); `integrate`
  (`src/webview/menus.ts:40-44`); `dropMenu` (`menus.ts:588`, `595-598`).
- **Tested by**: `test/git.test.ts` *rebases the current branch or another one*; `test/integration.test.ts` *reports a
  conflicted rebase and can skip the conflicting commit*; *undoes commits, checkouts, resets and rebases*;
  `test/menus.test.ts` *offers no rebase of a branch another worktree has, by dragging it onto a commit*; *offers the
  same when a branch is dropped on what is checked out*; `test/e2e/ui.mjs` *a branch dragged onto the checked out
  commit offers to merge or rebase*.

Acceptance:

- [ ] **Rebase main onto this** on the branch `feature` runs `git rebase feature`, logged as `> git rebase feature`.
- [ ] Dropping the label of the local branch `topic`, not checked out, on commit `1a2b3c4` offers **Rebase topic onto
  1a2b3c4**, which runs `git rebase <full hash> topic`; `topic` is checked out afterwards.
- [ ] A branch another worktree has checked out offers no rebase when dropped on a commit.
- [ ] It runs without a question while `gitardo.commandPreview` is `off`, as the README's list says; with `all`, the
  question **Rebase main onto feature?** shows `git rebase feature`.
- [ ] A name starting with `-` is refused with `"-x" is not a valid name.`
- [ ] With changes not committed, git refuses and Gitardo shows the failure with **Show Log**; nothing changed.
- [ ] Right after a rebase of three commits, one **Undo** runs `git reset --keep <previous>` and the three commits are
  as they were.

### `rebase.interactive` — Interactive rebase

Rewrites the commits from a commit up to `HEAD` as the plan of `rebase.editor` says: reordered, kept, given new
messages, stopped at, melded or dropped.

- **Kind**: Operation
- **Function**: `rebaseInteractiveOp(base, todoFile, todo, branch, autostash)` in `src/git/ops.ts`, called by
  `interactiveRebase(steps, base, state, repo, autostash = false)` in `src/extension/actions.ts`.
- **Entry points**: **Start rebase** in `rebase.editor`. The same machinery runs `rewrite.reword` of an older commit,
  `rewrite.squash` and `rewrite.split` of an older commit, with `--autostash`.
- **Offered when**: as `rebase.editor`.
- **Asks**: the question **Interactive rebase of `<branch>`?**, always, with *This rewrites the commits after `<short
  base>` with this todo list:* (or *after the root*) and the todo list.
- **Plan**: when the question is built, before it is asked, Gitardo makes a folder `gitardo-rebase-XXXXXX` in the
  temporary folder of the system with the todo list in `todo` (`rebase.todo`) and, for each reworded commit,
  `message-<index>.txt`: the subject typed, then the body the commit has. Then
  `> git -c "sequence.editor=cp \"<temp>/todo\"" -c core.editor=true rebase -i <base>`, or `--root` when the oldest
  commit has no parent. No `--autostash` from the editor. The folder is removed once the rebase ends in one go; after a
  stop it stays.
- **Danger**: Confirm, because it rewrites history: always asked, as the README's list says (*interactive rebase*).
- **Undo**: `undo.last` from the reflog of the branch: `git reset --keep <previous>` takes the whole rebase back at
  once. While it is stopped, **Undo** says *Finish or abort the rebase first.*, and `operation.abort` takes it back.
- **After**: a stop on an `edit` step is `rebase.edit-stop`; a stop on conflicts shows the banner `Rebase in progress ·
  N files have conflicts`.
- **Refuses**: a plan where every commit is dropped (*Every commit is dropped. To remove them all, reset the branch
  instead.*) or whose oldest kept commit is a squash or fixup (*The oldest commit that is kept has nothing before it to
  be squashed into.*); a hash starting with `-`. Git refuses with changes that are not committed (*cannot rebase: You
  have unstaged changes.*) or while another operation is under way, and the failure says so.
- **Settings**: `gitardo.commandPreview` (it is asked anyway).
- **Uses**: `rebase.editor`, `rebase.todo`, `rebase.edit-stop`, `operation.continue`, `operation.skip`,
  `operation.abort`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `rebaseInteractive` (`src/shared/protocol.ts:505`) and `RebaseStep`
  (`protocol.ts:463-472`); `resolveAction` case `rebaseInteractive` (`src/extension/actions.ts:274-275`);
  `interactiveRebase` (`actions.ts:899-921`); `rebaseTodo` and `rebaseInteractiveOp` (`src/git/ops.ts:347-406`);
  `RebaseEditor` (`src/webview/RebaseEditor.tsx`), opened from `commitMenu` (`src/webview/menus.ts:199-202`).
- **Tested by**: `test/integration.test.ts` *reorders, rewords, melds and drops commits in an interactive rebase*;
  *stops to edit a commit, and goes on whether it was amended, followed by more commits or left as it is*;
  `test/git.test.ts` *writes a rebase todo list where a reword is a pick plus an amend from a file*;
  `test/e2e/suite.ts` *changes the message of the last commit, of an older one after asking, and warns when it is
  pushed*; *stops an interactive rebase to edit a commit, and goes on once what was changed is committed*.
- **Differs**: the todo list and the message files are written by `interactiveRebase`
  (`src/extension/actions.ts:900-914`) while the question is built, before the user agrees, and no `write` line goes to
  the log for them; a cancelled question leaves them in the temporary folder. The question shows the todo list but not
  the new messages, which git reads from the message files, while the README (lines 649-653) says the confirmation shows
  the data next to the command.

Acceptance:

- [ ] A plan that moves the newest of three commits under the other two runs one `git rebase -i` whose todo list has the
  three `pick` lines in the new order, oldest first, and the branch ends with the same changes in that order.
- [ ] The command in the question and in the log is `git -c "sequence.editor=cp \"<temp>/todo\"" -c core.editor=true
  rebase -i <base>`, with the path of the folder written for it; nothing else is added.
- [ ] The question is asked even with `gitardo.commandPreview` `off`, and its note lists the todo list.
- [ ] A commit marked **Reword** with the subject *Better* gets *Better* followed by the body it had; the todo list has
  `pick <hash>` and `exec git commit --amend -F "<temp>/message-<index>.txt"` for it.
- [ ] **Squash** keeps both messages and opens no editor, as `core.editor=true` is in the command; **Fixup** keeps the
  message of the commit it melds into; **Drop** removes the commit.
- [ ] With a file changed and not committed, git refuses and nothing is rewritten.
- [ ] A rebase from the first commit of the repository ends with `--root`.
- [ ] The todo list and the message files are written once the user agrees, each with a `write` line in the log, and
  the question shows the new messages. (differs)
- [ ] The temporary folder is removed after a rebase that ended without stopping.
- [ ] Right after it, one **Undo** puts back every commit as it was.

### `rebase.editor` — The interactive rebase editor

The list in which the user plans an interactive rebase: one row per commit from `HEAD` down to the one chosen, newest
first, each with what to do with it.

- **Kind**: View
- **Function**: `RebaseEditor({ range, branch, onClose })` in `src/webview/RebaseEditor.tsx`, with
  `rebaseRange(hash, state)` in `src/webview/menus.ts`, which finds its commits (`rewrite.reach`).
- **Entry points**: **Interactive rebase of N commits…** in the context menu of a commit in the graph or the Gitardo
  Graph, N counting the commit and those above it; on the commit checked out, **Edit this commit (interactive
  rebase)…**.
- **Offered when**: no operation under way, no bisect, and the commits from `HEAD` down to it are loaded and make one
  line without a merge, the commit itself included.
- **Shows**: a dialog over the view, titled *Interactive rebase of `<branch>`: N commits* (*1 commit*), with *Newest
  first. Drag to reorder.* Each row has a grip, a list **Pick**, **Reword**, **Edit**, **Squash**, **Fixup**, **Drop**
  whose tooltip says what the choice does (*Keep the commit as it is*, *Keep the changes, change the message*, *Stop
  right after this commit, to amend it or commit more, then continue*, *Meld into the commit below, keeping both
  messages*, *Meld into the commit below, discarding this message*, *Remove the commit*), the short hash, the subject
  or, for **Reword**, a box with the subject to change, and the buttons **Move up** and **Move down**. Under the list:
  what is wrong with the plan, **Cancel** and **Start rebase**.
- **Rules**: rows can be dragged or moved one place at a time. **Start rebase** is disabled while the plan is the one it
  opened with (every commit **Pick**, in its place) or while it says *Every commit is dropped. To remove them all,
  reset the branch instead.*, *The oldest commit that is kept has nothing below it to meld into.* or *Enter a message
  for each reworded commit.* **Start rebase** posts the steps oldest first, with the parent of the oldest commit as the
  base (none for the first commit), a reworded commit's subject with `keepBody`, and closes the dialog.
- **Uses**: `rebase.interactive`, `rewrite.reach`.
- **Implemented in**: `RebaseEditor` (`src/webview/RebaseEditor.tsx:1-162`); the entry in `commitMenu`
  (`src/webview/menus.ts:176`, `199-202`); `setRebase` and the dialog in `App` (`src/webview/App.tsx:336`, `519`).
- **Tested by**: *none*.

Acceptance:

- [ ] On the third commit below `HEAD` of a line without merges, the menu has **Interactive rebase of 3 commits…**; on
  `HEAD`, **Edit this commit (interactive rebase)…**.
- [ ] Neither entry is offered on a commit below a merge, while an operation is under way, or during a bisect.
- [ ] The dialog lists the commits newest first, all **Pick**, and **Start rebase** is disabled until something
  changes.
- [ ] Setting every row to **Drop** shows *Every commit is dropped. To remove them all, reset the branch instead.* and
  keeps **Start rebase** disabled.
- [ ] Setting the oldest kept row to **Squash** or **Fixup** shows *The oldest commit that is kept has nothing below it
  to meld into.*
- [ ] A **Reword** row with an empty box shows *Enter a message for each reworded commit.*
- [ ] **Cancel** closes the dialog and runs nothing.

### `rebase.todo` — The todo list of an interactive rebase

The text that git reads as the plan of an interactive rebase, written by Gitardo from the steps, since nothing can type
in an editor from an extension.

- **Kind**: Derivation
- **Function**: `rebaseTodo(steps, messageFile)` in `src/git/ops.ts`.
- **Inputs**: the steps, oldest first, each `{ action, hash, message?, keepBody? }` with `action` one of `pick`,
  `reword`, `edit`, `squash`, `fixup`, `drop`; `messageFile(index)`, the path of the file of the message of step
  `index`.
- **Output**: one line per step, `<action> <hash>`, and a newline at the end. A `reword` is two lines: `pick <hash>`
  and `exec git commit --amend -F "<messageFile(index)>"`.
- **Invariants**: refuses when every step is `drop` (*Every commit is dropped. To remove them all, reset the branch
  instead.*), and when the first step that is not `drop` is `squash` or `fixup` (*The oldest commit that is kept has
  nothing before it to be squashed into.*); refuses a hash that starts with `-`. The steps keep their order.
- **Examples**: `pick c1`, `reword c2` (*Better*), `fixup c3`, `edit c5`, `drop c4` with files `/tmp/message-<i>.txt`
  → `pick c1\npick c2\nexec git commit --amend -F "/tmp/message-1.txt"\nfixup c3\nedit c5\ndrop c4\n`; `edit c1` →
  `edit c1\n`; `drop c1`, `squash c2` → refused; `drop c1` → refused.
- **Uses**: `rebase.interactive`, `rewrite.reword`, `rewrite.squash`, `rewrite.split`.
- **Implemented in**: `rebaseTodo` (`src/git/ops.ts:347-360`).
- **Tested by**: `test/git.test.ts` *writes a rebase todo list where a reword is a pick plus an amend from a file*.

Acceptance:

- [ ] The example steps above give exactly the text above.
- [ ] A step with the hash `-x` is refused with `"-x" is not a valid name.`

### `rebase.edit-stop` — Stop to edit a commit

Tells a rebase stopped by an `edit` step, or by a split, from one stopped on a conflict, so that the banner says what
to do and **Continue** checks first that what was changed is committed.

- **Kind**: Read
- **Function**: `Repo.rebaseStop()` in `src/git/repo.ts`; the banner `OperationBanner` in
  `src/webview/OperationBanner.tsx`; `refuseUncommitted(state)` in `src/extension/actions.ts`.
- **Entry points**: a rebase with an **Edit** step (`rebase.interactive`), or the split of an older commit
  (`rewrite.split`).
- **Rules**: a stop to change a commit when the git directory of the worktree has `rebase-merge/amend` and the last
  line of `rebase-merge/done` that is not a comment is `edit <hash>` (or `e <hash>`); otherwise, as on a conflict,
  there is none. It is a split when `rebase-merge/gitardo-split` holds a hash that starts with that `<hash>`, and the
  hash is then the commit being split; otherwise it is an edit, and the hash is the one in `amend`. The subject is read
  with `git log -1 --format=%s <hash> --`, with `GIT_OPTIONAL_LOCKS=0`.
- **Shows**: in the banner of the Graph view (in the panel or an editor tab) and of the Changes view, with the edit
  icon: *Stopped at "`<subject>`" to edit it. Amend it or commit more, then Continue.*, or for a split *Splitting
  "`<subject>`": commit its changes in as many commits as you like, then Continue.* **Continue**, with the tooltip *Go
  on with the rebase: the commits after it are made again on top*, disabled while files are in conflict; **Abort**; no
  **Skip**, which would throw away what is not committed and keep the commit anyway.
- **Continue**: at an edit stop, `operation.continue` first refuses when anything is staged or a tracked file is changed
  (untracked files do not count): *Some changes are not committed: amend the commit with them, commit them, or stash
  them, then continue.* Otherwise it runs `git -c core.editor=true rebase --continue`, also when nothing was changed:
  the commits after it are made again on top. At a split stop, **Continue** is `rewrite.split-rest`.
- **Uses**: `operation.continue`, `operation.abort`, `rewrite.split-rest`, `commit.amend`, `stash.push`.
- **Implemented in**: `Repo.rebaseStop` (`src/git/repo.ts:289-300`), read into `RepoState.rebaseStop` for a rebase
  (`repo.ts:659`); `RebaseStop` (`src/shared/protocol.ts:225-229`); `SPLIT_MARKER` (`src/git/ops.ts:421`);
  `OperationBanner` (`src/webview/OperationBanner.tsx:36-84`); the `continue` case (`src/extension/actions.ts:600-603`)
  and `refuseUncommitted` (`actions.ts:656-661`).
- **Tested by**: `test/integration.test.ts` *stops to edit a commit, and goes on whether it was amended, followed by
  more commits or left as it is*; *splits an older commit in a rebase that stops there, and commits what is left of it
  on the way to the end*; `test/e2e/suite.ts` *stops an interactive rebase to edit a commit, and goes on once what was
  changed is committed*.

Acceptance:

- [ ] A rebase with **Edit** on *Edit one* stops with the banner *Stopped at "Edit one" to edit it. Amend it or commit
  more, then Continue.*, **Continue** and **Abort**, and no **Skip**.
- [ ] With `edit-one.txt` changed and not committed, **Continue** says *Some changes are not committed: amend the commit
  with them, commit them, or stash them, then continue.* and the rebase is still there.
- [ ] Once the change is amended, **Continue** runs `git -c core.editor=true rebase --continue`, the commit after it is
  made again on top, and the commit has the change.
- [ ] **Continue** with nothing changed goes on as well; with a new commit made at the stop, that commit stays between
  the edited one and the ones after.
- [ ] A later step of the same rebase that stops on a conflict shows `Rebase in progress · N files have conflicts`, not
  the edit text.

### `cherry-pick.one` — Cherry-pick a commit

Makes the change of one commit again on what is checked out.

- **Kind**: Operation
- **Function**: `cherryPickOp(hash, isMerge, recordOrigin)` in `src/git/ops.ts`, with `recover: skipEmptyPicks(root)`.
- **Entry points**: **Cherry-pick `<short>`** in the context menu of a commit in the graph and the Gitardo Graph;
  **Cherry-pick `<short>` onto `<current>`** for a commit dropped on the commit checked out or on the row of the
  uncommitted changes (`rewrite.drag-and-drop`); **Cherry-pick** for a position chosen in the list of the reflog.
- **Offered when**: the context menu offers it on every commit but the one checked out, merges included, also while an
  operation is under way; the drop menu not while an operation is under way; the reflog list on any position, always
  as a commit that is not a merge.
- **Reads**: `merge.preview-conflicts` from `<hash>^`, for a commit that is not a merge.
- **Plan**: `> git cherry-pick <hash>`, with `-x` first when `gitardo.cherryPick.recordOrigin` is on and `-m 1` for a
  merge commit (its first parent is the mainline): `> git cherry-pick -x -m 1 <hash>`. Title `Cherry-pick <short>`.
  When git stops on a commit that changes nothing on top of `HEAD`, with no conflict and the files as in `HEAD`, the log
  says `<short> changes nothing on top of HEAD, so the cherry-pick skips it.` and Gitardo runs
  `> git cherry-pick --skip`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`) | Confirm, when the preview finds conflicts.
- **Undo**: `undo.last` from the reflog: `git reset --keep <previous>` takes the new commit away. While the cherry-pick
  is stopped, **Undo** says *Finish or abort the cherry-pick first.*
- **After**: on conflicts, the banner `Cherry-pick in progress · N files have conflicts` with **Continue**, **Skip** and
  **Abort**.
- **Refuses**: a hash starting with `-`. Git refuses a merge commit chosen in the reflog list, which needs `-m`.
- **Settings**: `gitardo.cherryPick.recordOrigin` (default off): each new commit ends its message with `(cherry picked
  from commit <hash>)`; `gitardo.commandPreview`.
- **Uses**: `merge.preview-conflicts`, `operation.continue`, `operation.skip`, `operation.abort`, `preview.ask`,
  `log.write`, `undo.last`.
- **Implemented in**: `Action` case `cherryPick` (`src/shared/protocol.ts:556`); `resolveAction` case `cherryPick`
  (`src/extension/actions.ts:362-366`) and `recordOrigin` (`actions.ts:112`); `cherryPickOp`
  (`src/git/ops.ts:471-473`); `skipEmptyPicks` (`ops.ts:1216-1251`); `commitMenu` (`src/webview/menus.ts:192`);
  `dropMenu` (`menus.ts:577-582`); `Controller.browseReflog` (`src/extension/controller.ts:1464`).
- **Tested by**: `test/integration.test.ts` *cherry-picks, reverts and resets*; *records the commit a cherry-pick was
  picked from only when asked, for one commit and for several*; *predicts conflicts of a merge and of a cherry-pick
  without touching anything*; *does not skip an empty pick while the files have changes of the user*;
  `test/git.test.ts` *picks the first parent as mainline for merge commits*; `test/e2e/suite.ts` *records the commit
  a cherry-pick was picked from when the settings ask*.
- **Differs**: a merge commit gets no conflict preview (`src/extension/actions.ts:365`), so one that will conflict runs
  without asking, while the README's list asks before *a cherry-pick that is going to conflict*. The
  `git cherry-pick --skip` of an empty pick is run by `skipEmptyPicks` (`src/git/ops.ts:1224-1227`) and logged, but it
  is not among the commands of the question.

Acceptance:

- [ ] **Cherry-pick 1a2b3c4** runs `git cherry-pick <hash>`, logged as `> git cherry-pick <hash>`; the branch gets one
  new commit with that change.
- [ ] With `gitardo.cherryPick.recordOrigin` on, the command is `git cherry-pick -x <hash>` and the new message ends
  with `(cherry picked from commit <hash>)`; off, the message is the one picked.
- [ ] A merge commit is picked with `git cherry-pick -m 1 <hash>`.
- [ ] A commit that will conflict in `a.txt` is asked first with `This will stop with conflicts in 1 file:` and
  `a.txt`, even with `gitardo.commandPreview` `off`; a merge commit that will conflict is not. (differs)
- [ ] A commit whose change the branch has already ends with nothing changed and nothing left under way; the log has the
  warning and `> git cherry-pick --skip`, which the question did not show. (differs)
- [ ] The entry is not offered on the commit checked out; dropping a commit anywhere but on the commit checked out or
  the uncommitted changes offers nothing.
- [ ] Right after it, **Undo** asks, then runs `git reset --keep <previous>`.

### `cherry-pick.many` — Cherry-pick several commits

Picks the commits selected in the graph with one `git cherry-pick`, oldest first, as one operation to continue, skip
or abort, and to undo.

- **Kind**: Operation
- **Function**: `cherryPickManyOp(hashes, recordOrigin)` in `src/git/ops.ts`, with `recover: skipEmptyPicks(root)`.
- **Entry points**: **Cherry-pick onto `<current>`** in the bar above the graph that counts the selected commits
  (tooltip *Applies them oldest first, as one operation*); **Cherry-pick N commits onto `<current>`** in the context
  menu of the selection. Commits are selected with `Ctrl`+click (`Cmd` on macOS) and `Shift`+click.
- **Offered when**: two commits or more are selected, none is a merge, no operation is under way and no bisect.
- **Asks**: nothing; with `gitardo.commandPreview` `all`, the question **Cherry-pick N commits?** says *In this order:*,
  then `<short> <subject>` for each, up to 15 and `…and N more`, then *If one of them conflicts, git stops there:
  resolve it and continue, skip that commit, or abort them all.*
- **Plan**: `> git cherry-pick <oldest> … <newest>`, with `-x` first when `gitardo.cherryPick.recordOrigin` is on; the
  order is the order of the graph, oldest first, whatever order they were clicked in. A commit of the series that
  changes nothing on top of `HEAD` is skipped by `> git cherry-pick --skip`, with a warning in the log, also when the
  series is continued.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`). No conflicts are previewed.
- **Undo**: Gitardo remembers where `HEAD` was: **Undo** asks **Undo "Cherry-pick N commits"?** with *`<current>`
  goes back to `<short>`. Git refuses if that would overwrite uncommitted changes.* and runs
  `git reset --keep <before>`, taking every commit of the series back at once. It holds also when the series stopped
  and was continued or skipped in Gitardo to the end, until the reflog of `HEAD` moves again. Continued from a terminal,
  or after something else in between, **Undo** takes back one commit at a time from the reflog. While it is stopped,
  `operation.abort` takes the whole series back.
- **After**: started from the bar, the selection is cleared. On a conflict, the banner `Cherry-pick in progress · N
  files have conflicts`, whose **Continue**, **Skip** and **Abort** apply to the whole series.
- **Refuses**: *Finish or abort the `<operation>` first.* while one is under way; *Select the commits to cherry-pick in
  the graph.* with fewer than two, or some not in the graph; *A merge commit can only be cherry-picked on its own.*; a
  hash starting with `-`.
- **Settings**: `gitardo.cherryPick.recordOrigin`, `gitardo.commandPreview`.
- **Uses**: `cherry-pick.one`, `operation.continue`, `operation.skip`, `operation.abort`, `preview.ask`, `log.write`,
  `undo.last`.
- **Implemented in**: `Action` case `cherryPickMany` (`src/shared/protocol.ts:558`); `resolveAction` case
  `cherryPickMany` (`src/extension/actions.ts:368-382`); `cherryPickManyOp` (`src/git/ops.ts:476-478`);
  `canCherryPick`, `cherryPickMany` and `selectionMenu` (`src/webview/menus.ts:262-276`); the bar in `App`
  (`src/webview/App.tsx:405-418`); the series in `Controller.action` (`src/extension/controller.ts:1650-1651`,
  `1670-1674`, `1714-1722`).
- **Tested by**: `test/integration.test.ts` *cherry-picks several commits as one operation, which stops on a conflict
  and can be continued*; *continues a series of cherry-picks past a conflict resolved by keeping what the branch has*;
  *skips the commits of a series whose changes the branch has already*; *undoes a whole series of cherry-picks by
  aborting it*; *records the commit a cherry-pick was picked from only when asked, for one commit and for several*;
  `test/e2e/suite.ts` *cherry-picks several commits oldest first, whatever order they were selected in*; *continues a
  series of cherry-picks past a conflict resolved by keeping what the branch has*.
- **Differs**: no conflict preview runs for a series (`src/extension/actions.ts:368-381`): a series that will conflict
  starts without asking, while the README's Previews before risky operations and its list in Command preview say a
  cherry-pick that is going to conflict is said and asked first. The `git cherry-pick --skip` of an empty pick
  (`src/git/ops.ts:1224-1227`) runs without being in the question.

Acceptance:

- [ ] Selecting *Pick two* then *Pick one* (older) and choosing **Cherry-pick onto main** runs
  `git cherry-pick <one> <two>`, logged as such, and the branch ends with *Pick one* then *Pick two*.
- [ ] The bar and the menu offer nothing to cherry-pick when a merge commit is selected, while an operation is under
  way, or during a bisect.
- [ ] With `gitardo.commandPreview` `all`, the question is **Cherry-pick 2 commits?**, its note starts with *In this
  order:* and the commits oldest first, and the command at the end is `git cherry-pick <one> <two>`.
- [ ] A series whose second commit conflicts stops there with the banner; **Continue** after resolving makes the rest.
- [ ] A series that will conflict is not asked first while `gitardo.commandPreview` is `off`. (differs)
- [ ] Right after the series, **Undo** asks **Undo "Cherry-pick 2 commits"?** and runs `git reset --keep <before>`:
  both commits are gone; a second **Undo** brings them back.
- [ ] After a series stopped on a conflict and continued in Gitardo to the end, one **Undo** takes back all of it.
- [ ] **Abort** while stopped leaves `HEAD` where it was before the series.

### `revert.commit` — Revert a commit

Makes a new commit that takes back the change of a commit.

- **Kind**: Operation
- **Function**: `revertOp(hash, isMerge)` in `src/git/ops.ts`.
- **Entry points**: **Revert `<short>`** in the context menu of a commit in the graph and the Gitardo Graph, and of a
  commit in the Commits view.
- **Offered when**: on every commit, the one checked out and merges included, also while an operation is under way.
- **Plan**: `> git revert --no-edit <hash>`, and `> git revert --no-edit -m 1 <hash>` for a merge commit, whose first
  parent is the mainline. Title `Revert <short>`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`). No conflicts are previewed.
- **Undo**: `undo.last` from the reflog: `git reset --keep <previous>` takes the new commit away. While the revert is
  stopped, **Undo** says *Finish or abort the revert first.*
- **After**: on conflicts, the banner `Revert in progress · N files have conflicts`; **Continue** runs
  `git -c core.editor=true revert --continue` (`operation.continue`).
- **Refuses**: a hash starting with `-`.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `operation.continue`, `operation.skip`, `operation.abort`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `revert` (`src/shared/protocol.ts:561`); `resolveAction` case `revert`
  (`src/extension/actions.ts:489-490`); `revertOp` (`src/git/ops.ts:522-524`); `commitMenu`
  (`src/webview/menus.ts:193`); `logMenu` (`menus.ts:253`).
- **Tested by**: `test/integration.test.ts` *cherry-picks, reverts and resets*; `test/git.test.ts` *picks the first
  parent as mainline for merge commits*.

Acceptance:

- [ ] **Revert 1a2b3c4** runs `git revert --no-edit <hash>`, logged as such: a new commit takes the change back, and
  no editor opens.
- [ ] A merge commit is reverted with `git revert --no-edit -m 1 <hash>`.
- [ ] It runs without a question while `gitardo.commandPreview` is `off`; the README's list does not name it.
- [ ] Right after it, **Undo** runs `git reset --keep <previous>` after asking, and the revert commit is gone.

### `reset.to` — Reset the current branch to a commit

Moves the branch checked out to another commit, keeping the changes staged, keeping them unstaged, or throwing them
away.

- **Kind**: Operation
- **Function**: `resetOp(hash, mode, branch)` in `src/git/ops.ts`.
- **Entry points**: **Reset `<current>` to here…** in the context menu of a commit in the graph and the Gitardo Graph,
  in the Commits view, and in the list of the reflog; **Reset `<name>` to `<short>`…** for the current branch dropped
  on another commit (`rewrite.drag-and-drop`).
- **Offered when**: the graph offers it on every commit, the one checked out included, also while an operation is
  under way; the Commits view on every commit but the one checked out.
- **Asks**: a list titled `Reset <current> to <short>`: **Soft** (*Keep all changes, staged*), **Mixed** (*Keep all
  changes, unstaged*), **Hard** (*Discard all changes*).
- **Plan**: `> git reset --soft <hash>`, `> git reset --mixed <hash>` or `> git reset --hard <hash>`. Title
  `Reset <current> to <short> (<mode>)`.
- **Danger**: None for **Soft** and **Mixed** (asked only when `gitardo.commandPreview` is `all`) | Destructive for
  **Hard**, because it throws away the changes to tracked files that are not committed: always asked, as a warning, with
  *A hard reset throws away all uncommitted changes to tracked files.*
- **Undo**: `undo.last` from the reflog: the branch goes back to where it was, with `git reset --mixed <previous>` when
  the files already are as that commit has them (after **Soft** or **Mixed**), otherwise `git reset --keep <previous>`.
  The changes a hard reset threw away do not come back.
- **Refuses**: a hash starting with `-`.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `rewrite.drag-and-drop`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `reset` (`src/shared/protocol.ts:562`); `resolveAction` case `reset`
  (`src/extension/actions.ts:492-499`); `resetOp` (`src/git/ops.ts:526-533`); `commitMenu`
  (`src/webview/menus.ts:213`), `logMenu` (`menus.ts:254`), `dropMenu` (`menus.ts:598`); `Controller.browseReflog`
  (`src/extension/controller.ts:1462`).
- **Tested by**: `test/git.test.ts` *only treats a hard reset as dangerous*; `test/integration.test.ts` *cherry-picks,
  reverts and resets*; *undoes commits, checkouts, resets and rebases*.

Acceptance:

- [ ] **Soft** on `1a2b3c4` runs `git reset --soft <hash>` titled `Reset main to 1a2b3c4 (soft)`, and the changes of
  the commits after it are staged.
- [ ] **Mixed** leaves them unstaged in the working tree.
- [ ] **Hard** is asked first, as a warning that says *A hard reset throws away all uncommitted changes to tracked
  files.*, whatever `gitardo.commandPreview` says, and the README's list (*hard reset*) agrees.
- [ ] Escape on the list runs nothing.
- [ ] Right after a hard reset, **Undo** brings the commits back with `git reset --keep <previous>`.

### `rewrite.reword` — Edit the message of a commit

Gives a commit a new message: the last one by amending it, an older one by an interactive rebase that makes the
commits after it again.

- **Kind**: Operation
- **Function**: `rewordHeadOp(message)` in `src/git/ops.ts` for the last commit; for an older one, the `reword` case
  of `resolveAction` builds the steps and calls `interactiveRebase(steps, parent, state, repo, true)` in
  `src/extension/actions.ts`; `warnIfPushed(op, hash, repo)` for both.
- **Entry points**: the pencil beside the message in the details (tooltip **Edit message**), and **Edit message…** in
  the context menu of a commit, which both open `rewrite.message-box`.
- **Offered when**: `canReword` (`rewrite.reach`): on the last commit, merge or not, and on the older ones down to the
  first merge, while no operation is under way and no bisect.
- **Asks**: the new message, in the box. The last commit is not asked further unless it is pushed. An older one is
  asked **Edit the message of `<short>`?** with *`<short>` gets the new message, and the commit after it is made again
  on top of it, with new hashes.* (*the N commits after it are made again*), and *Changes that are not committed are set
  aside and put back.* when there are some. A pushed commit adds the warning of `rewrite.pushed-warning` first.
- **Reads**: `Repo.lineAfter(hash)`, the commits after it; its parent with `git rev-parse --verify --quiet <hash>^`;
  `rewrite.pushed-warning`.
- **Plan**: the last commit: `> git commit --amend --only --allow-empty -F -`, with the message on standard input (the
  log shows it under `standard input:`); what is staged stays staged. An older one: a temporary folder with `todo`
  (`pick <hash>`, `exec git commit --amend -F "<temp>/message-0.txt"`, then `pick` for each commit after it) and
  `message-0.txt` with the whole new message, then `> git -c "sequence.editor=cp \"<temp>/todo\"" -c core.editor=true
  rebase -i --autostash <parent>` (or `--root`).
- **Danger**: None for the last commit when it is not pushed (asked only when `gitardo.commandPreview` is `all`) |
  Confirm, for an older commit, because it rewrites the commits after it (the README's list: *interactive rebase*), and
  for a pushed one, because the branch will have to be force pushed.
- **Undo**: the last commit: `undo.last` from the reflog (`commit (amend)`), `git reset --soft <previous>`, which gives
  back the old message and leaves what is staged. An older one: `undo.last` from the reflog of the branch,
  `git reset --keep <previous>`, which puts back every commit the rebase made again.
- **After**: the commit with its new message is selected in the graph.
- **Refuses**: *Finish or abort the `<operation>` first.*; *End the bisect first.*; *Enter a message.* for an empty
  one; an older commit that is not on one line without merges before `HEAD`: *`<short>` is not on the line of commits
  before HEAD without merges: its message cannot be changed by a rebase from here.*
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `rewrite.message-box`, `rewrite.reach`, `rewrite.pushed-warning`, `rebase.interactive`, `rebase.todo`,
  `commit.amend`, `sync.push`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `reword` (`src/shared/protocol.ts:510`); `resolveAction` case `reword`
  (`src/extension/actions.ts:277-299`); `rewordHeadOp` (`src/git/ops.ts:409-415`); `interactiveRebase` and
  `warnIfPushed` (`actions.ts:899-939`); `canReword` and the menu entry (`src/webview/menus.ts:134-137`, `196`);
  `Controller.action` selects the commit (`src/extension/controller.ts:1657-1662`, `1688`).
- **Tested by**: `test/integration.test.ts` *changes the message of the last commit and leaves what is staged, or of an
  older one with what is not committed set aside*; `test/e2e/suite.ts` *changes the message of the last commit, of an
  older one after asking, and warns when it is pushed*; `test/e2e/ui.mjs` *the message of a commit is changed in the
  details, the last one at once and an older one after a question*; `test/menus.test.ts` *is offered on the commits
  down to the first merge, and not while something else is going on*.
- **Differs**: for an older commit the todo list and the message file are written by `interactiveRebase`
  (`src/extension/actions.ts:900-914`) before the question, with no `write` line in the log, and the question's note is
  replaced (`actions.ts:296`), so it shows neither the todo list nor the message that git reads from the file, while the
  README (lines 649-653) says the confirmation shows them. The reads of the parent (`actions.ts:290`) and of the new
  commit to select (`src/extension/controller.ts:1688`) run `git rev-parse` without `GIT_OPTIONAL_LOCKS=0`.

Acceptance:

- [ ] Saving *Reword three, new* on the last commit runs `git commit --amend --only --allow-empty -F -` with that
  message on standard input, with no question while `gitardo.commandPreview` is `off`; a file staged before stays
  staged and out of the commit.
- [ ] Saving a message on the commit two below `HEAD` asks **Edit the message of `<short>`?** with *the 2 commits after
  it are made again*, then the commit has the message and the two after it have new hashes and their own messages.
- [ ] A file changed and not committed is the same after the rebase as before (`--autostash`).
- [ ] On a commit that `origin/reword` has, the question starts with *`<short>` is pushed already, on origin/reword:
  the branch will have to be force pushed, and whoever has the old commits will have to move to the new ones.*, also for
  the last commit.
- [ ] The entry and the pencil are not offered on a commit below a merge, while an operation is under way, or during a
  bisect.
- [ ] The commit with the new message is selected afterwards.
- [ ] Right after rewording the last commit, **Undo** runs `git reset --soft <previous>` and the old message is back;
  after rewording an older one, **Undo** runs `git reset --keep <previous>` and every commit is as it was.
- [ ] The question for an older commit shows the todo list it runs. (differs)

### `rewrite.message-box` — The box that edits a message

The box that takes the place of the message in the details, to write the new message of a commit.

- **Kind**: View
- **Function**: `MessageEditor({ message, onSave, onCancel })` in `src/webview/Detail.tsx`.
- **Entry points**: the pencil beside the message in the details (tooltip **Edit message**), and **Edit message…** in
  the context menu of a commit, which selects it and opens the box.
- **Offered when**: the pencil, when `canReword` holds for the selected commit (`rewrite.reach`), its details are loaded
  and the box is not open; never on the row of the uncommitted changes.
- **Shows**: a box labelled *Commit message* with the whole message, the subject, an empty line and the body, the
  cursor at its end, from 3 to 12 lines high as the text grows; *Ctrl+Enter to save*; **Cancel** and **Save**.
- **Rules**: `Ctrl`+`Enter` (`Cmd`+`Enter` on macOS) or **Save** posts the action `reword` with the trimmed text;
  `Escape` or **Cancel** closes the box and changes nothing. **Save** is disabled while the text is empty or is the
  message as it was; saving it unchanged with the keys closes the box like **Cancel**.
- **Uses**: `rewrite.reword`.
- **Implemented in**: `MessageEditor` (`src/webview/Detail.tsx:70-115`); the pencil (`Detail.tsx:205-209`) and the box
  (`Detail.tsx:214-223`); `onEditMessage` in `App` (`src/webview/App.tsx:338-341`).
- **Tested by**: `test/e2e/ui.mjs` *the message of a commit is changed in the details, the last one at once and an
  older one after a question*.

Acceptance:

- [ ] The pencil opens a box with the message *Subject*, an empty line and *Body*, and the cursor after *Body*.
- [ ] `Escape` closes it and nothing runs.
- [ ] **Save** is disabled while the text is the message as it was, and while it is empty.
- [ ] `Ctrl`+`Enter` with a new text runs `rewrite.reword` with it, and the box closes.

### `rewrite.squash` — Squash commits into one

Makes one commit of several that follow one another before `HEAD`, with the changes of them all and a message put
together from theirs.

- **Kind**: Operation
- **Function**: the `squash` case of `resolveAction` in `src/extension/actions.ts`, with `squashRange(hashes, state,
  repo)`, `squashMessage(subject, messages)` (`rewrite.squash-message`) and `interactiveRebase(steps, parent, state,
  repo, true)`; `warnIfPushed(op, oldest, repo)`.
- **Entry points**: **Squash into one** in the bar above the graph that counts the selected commits (tooltip *Makes
  them one commit, with a message you give*); **Squash N commits into one…** in the context menu of the selection;
  **Squash into the commit before it…** in the context menu of a single commit, which squashes it with its parent.
- **Offered when**: `canSquash` (`rewrite.reach`): two commits or more, that follow one another on the line before
  `HEAD`, down to the first merge, while no operation is under way and no bisect. **Squash into the commit before it…**
  when the commit and its only parent are both on that line.
- **Asks**: a box titled `Squash N commits` with *Subject of the commit they become. What their messages say goes under
  it.*, filled with the subject of the oldest; an empty one says *Enter a subject*. Then the question **Squash N
  commits?** with *`<short>`, `<short>` become one commit, with the changes of them all.*, *The commit after them is made
  again on top of it.* (or *The N commits after them…*) when there are some, and *Changes that are not committed are set
  aside and put back.* when there are some; pushed, the warning of `rewrite.pushed-warning` first.
- **Reads**: `Repo.lineAfter(oldest)`; the message of each commit; the parent of the oldest with
  `git rev-parse --verify --quiet <oldest>^`; `rewrite.pushed-warning`.
- **Plan**: a temporary folder with `todo` (`pick <oldest>`, `exec git commit --amend -F "<temp>/message-0.txt"`,
  `fixup <each other>`, then `pick <each commit after them>`) and `message-0.txt` with the message put together; then
  `> git -c "sequence.editor=cp \"<temp>/todo\"" -c core.editor=true rebase -i --autostash <parent>` (or `--root`).
- **Danger**: Confirm, because it rewrites the commits and those after them: always asked, as the README's list says
  (*squash*).
- **Undo**: `undo.last` from the reflog of the branch: `git reset --keep <previous>` takes the whole squash back.
- **After**: the commit they became is selected in the graph.
- **Refuses**: *Finish or abort the `<operation>` first.*; *End the bisect first.*; commits that do not follow one
  another before `HEAD`: *Only commits that follow one another on the line before HEAD, without merges, can be squashed
  into one.*; an empty subject.
- **Settings**: `gitardo.commandPreview` (it is asked anyway).
- **Uses**: `rewrite.reach`, `rewrite.squash-message`, `rewrite.pushed-warning`, `rebase.interactive`, `rebase.todo`,
  `sync.push`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `squash` (`src/shared/protocol.ts:515`); `resolveAction` case `squash`
  (`src/extension/actions.ts:301-329`); `squashRange` (`actions.ts:945-955`); `squashMessage` and `splitMessage`
  (`src/git/ops.ts:363-382`); `canSquash`, `squash`, the entries (`src/webview/menus.ts:143-149`, `209-212`, `272`);
  the bar (`src/webview/App.tsx:399-404`); `Controller.action` (`src/extension/controller.ts:1660-1661`, `1688`).
- **Tested by**: `test/integration.test.ts` *squashes commits into one under a new message, makes the ones after them
  again and keeps what is not committed*; `test/menus.test.ts` *is offered for commits that follow one another before
  HEAD, down to the first merge*; *squashes a commit into the one before it when that one is on the same line*;
  `test/git.test.ts` *puts together the message of squashed commits under the subject given*.
- **Differs**: the todo list and the message file are written by `interactiveRebase`
  (`src/extension/actions.ts:900-914`) before the question, with no `write` line in the log, and the note of the
  question is replaced (`actions.ts:327`), so it shows neither the todo list nor the message, while the README (lines
  649-653) says the confirmation shows them. The parent (`actions.ts:316`) and the commit to select
  (`src/extension/controller.ts:1688`) are read with `git rev-parse` without `GIT_OPTIONAL_LOCKS=0`.

Acceptance:

- [ ] Selecting *Add b*, *Add c* and *Add d*, in a line before `HEAD`, and choosing **Squash into one** with the subject
  *Add b, c and d* makes one commit with the changes of the three, whose message is the subject followed by their
  messages, oldest first; the commit after them is made again on top of it.
- [ ] The box starts with the subject of the oldest commit; Escape runs nothing.
- [ ] The question is asked even with `gitardo.commandPreview` `off`, and the command is `git -c "sequence.editor=cp
  \"<temp>/todo\"" -c core.editor=true rebase -i --autostash <parent>`.
- [ ] A file changed and not committed is the same afterwards.
- [ ] **Squash into the commit before it…** on `e`, whose parent `d` is on the line, squashes `d` and `e`; it is not
  offered on a commit whose parent is a merge.
- [ ] Commits with one not selected between them are not offered **Squash into one**.
- [ ] Pushed commits are asked with the force push warning.
- [ ] The commit they became is selected; right after, **Undo** runs `git reset --keep <previous>` and the commits are
  back.
- [ ] The question shows the todo list it runs. (differs)

### `rewrite.squash-message` — The message of squashed commits

The message that the commits of a squash become: the subject given, then what their messages say, oldest first, as git
puts them together.

- **Kind**: Derivation
- **Function**: `squashMessage(subject, messages)` and `splitMessage(message)` in `src/git/ops.ts`.
- **Inputs**: the subject typed; the messages of the commits, oldest first.
- **Output**: the subject trimmed, then each message separated by an empty line; a message whose subject is the one
  given adds only its body; nothing empty is added.
- **Invariants**: `splitMessage` takes the first paragraph, on one line, as the subject, and the rest, trimmed, as the
  body.
- **Examples**: *Add login* with `Add login\n\nWith a form.`, `Fix typo`, `Check the password\n\nAt least 8
  characters.` → `Add login\n\nWith a form.\n\nFix typo\n\nCheck the password\n\nAt least 8 characters.`; *Log in with a
  password* with the same → that subject followed by all three messages whole; *Fix typo* with `Fix typo`, `Fix typo` →
  `Fix typo`; `splitMessage('Subject\nstill subject\n\nBody\n\nMore')` → subject *Subject still subject*, body
  `Body\n\nMore`.
- **Uses**: `rewrite.squash`.
- **Implemented in**: `splitMessage` and `squashMessage` (`src/git/ops.ts:363-382`).
- **Tested by**: `test/git.test.ts` *puts together the message of squashed commits under the subject given*.

Acceptance:

- [ ] The four examples above give exactly the results above.

### `rewrite.split` — Split a commit

Takes the changes of a commit out of it into the working tree, unstaged, to commit them again in as many commits as
the user likes.

- **Kind**: Operation
- **Function**: `splitHeadOp(hash)` in `src/git/ops.ts` for the last commit; for an older one,
  `splitRebaseOp(rebase, hash, gitDir)` in `src/git/ops.ts` around `interactiveRebase(steps, parent, state, repo, true)`;
  `warnIfPushed(op, hash, repo)` for both.
- **Entry points**: **Split this commit…** in the context menu of a commit in the graph and the Gitardo Graph.
- **Offered when**: where **Edit message…** is (`rewrite.reach`), on a commit with exactly one parent: not on a merge,
  not on the first commit.
- **Asks**: always. The last commit: **Split `<short>`?** with *The changes of `<short>` go back into the working tree,
  unstaged, and the branch goes back to the commit before it: commit them in as many commits as you like.*, and *The
  changes that are not committed stay, among them.* when there are some. An older one: **Split `<short>`?** with *The
  rebase stops at `<short>` with its changes taken out of it, unstaged: commit them in as many commits as you like, then
  Continue, and the commit after it is made again on top.* (*the N commits after it are*), and *Changes that are not
  committed are set aside until the end.* when there are some. Pushed, the warning of `rewrite.pushed-warning` first.
- **Reads**: its parents with `git rev-list --parents -n 1 <hash>`; `Repo.lineAfter(hash)`; the git directory of the
  worktree; `rewrite.pushed-warning`.
- **Plan**: the last commit: `> git reset --mixed HEAD~`. An older one: a temporary folder with `todo` (`edit <hash>`,
  then `pick` for each commit after it), then `> git -c "sequence.editor=cp \"<temp>/todo\"" -c core.editor=true rebase
  -i --autostash <parent>`, `> git reset --mixed HEAD~`, and `> write <git dir>/rebase-merge/gitardo-split`, a file with
  the hash of the commit, which git removes with the rest of `rebase-merge` when the rebase ends.
- **Danger**: Confirm, because it takes a commit apart and, for an older one, rewrites the commits after it: always
  asked, as the README's list says (*split*).
- **Undo**: the last commit: Gitardo remembers `git reset --mixed <hash>`, asked as **Undo "Split `<short>`"?** with
  *`<current>` goes back to `<short>`. Your files are not touched.*, which puts the commit back as it was, its added
  files included, until the reflog of `HEAD` moves. An older one: while the rebase is stopped, **Undo** says *Finish or
  abort the rebase first.* and **Abort** takes it all back (`operation.abort`); after the end, `undo.last` from the
  reflog of the branch takes back the whole split.
- **After**: the commit box of the Changes view, when it is empty, gets the message of the commit. The last commit: a
  message *The changes of "`<subject>`" are in the working tree, unstaged: commit them in as many commits as you like.*
  with **Undo**. An older one: the banner *Splitting "`<subject>`": commit its changes in as many commits as you like,
  then Continue.* (`rebase.edit-stop`); **Continue** is `rewrite.split-rest`.
- **Refuses**: *Finish or abort the `<operation>` first.*; *End the bisect first.*; *"`<hash>`" is not a commit.*;
  *`<short>` is a merge: it cannot be split.*; *`<short>` is the first commit, with nothing before it to take its
  changes back to: it cannot be split.*; *`<short>` is not on the line of commits before HEAD without merges: it cannot
  be split by a rebase from here.*
- **Settings**: `gitardo.commandPreview` (it is asked anyway).
- **Uses**: `rewrite.reach`, `rewrite.pushed-warning`, `rewrite.split-rest`, `rebase.interactive`, `rebase.todo`,
  `rebase.edit-stop`, `commit.create`, `operation.abort`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `split` (`src/shared/protocol.ts:521`); `resolveAction` case `split`
  (`src/extension/actions.ts:331-360`); `SPLIT_MARKER`, `splitHeadOp` and `splitRebaseOp` (`src/git/ops.ts:421-443`);
  the menu entry (`src/webview/menus.ts:198`); `Controller.action` (`src/extension/controller.ts:1692-1697`) and
  `Controller.reportSplit` (`controller.ts:1783-1791`).
- **Tested by**: `test/integration.test.ts` *splits the last commit by taking its changes out of it, which undo puts
  back*; *splits an older commit in a rebase that stops there, and commits what is left of it on the way to the end*;
  `test/e2e/suite.ts` *splits the last commit and an older one, commits what is left on Continue, and undoes each split
  whole*.
- **Differs**: for an older commit the todo list is written by `interactiveRebase` (`src/extension/actions.ts:900-914`)
  before the question, with no `write` line in the log, and the note of the question is replaced (`actions.ts:357`), so
  it does not show the todo list, while the README (lines 649-653) says the confirmation shows it. The parents are read
  with `git rev-list` without `GIT_OPTIONAL_LOCKS=0` (`actions.ts:336`).

Acceptance:

- [ ] **Split this commit…** on the last commit asks **Split `<short>`?**, runs `git reset --mixed HEAD~`, and leaves
  `HEAD` at its parent with its changes unstaged; the message says what to do next and the commit box has its message.
- [ ] Right after, **Undo** puts the commit back with `git reset --mixed <hash>`, its new files included, and nothing is
  left changed.
- [ ] On an older commit, it runs the rebase with `edit <hash>` and the `pick` lines after it, then
  `git reset --mixed HEAD~`, and writes `gitardo-split` with the hash; `HEAD` is at its parent, the rebase stopped, and
  the banner says *Splitting "Add b and c": …*.
- [ ] A file changed and not committed before is set aside until the end and is there again after it.
- [ ] It is not offered on a merge, on the first commit, while an operation is under way, or during a bisect.
- [ ] A pushed commit is asked with the force push warning.
- [ ] After the split of an older commit ends, one **Undo** takes back the whole split, and the changes not committed
  stay.
- [ ] The question for an older commit shows the todo list it runs. (differs)

### `rewrite.split-rest` — Commit what is left of a split and continue

What **Continue** does while a rebase is stopped to split a commit: it goes on once the changes of the commit are
committed, or offers to commit what is left of them as one more commit.

- **Kind**: Operation
- **Function**: `continueSplit(stop, state, repo)` in `src/extension/actions.ts`, which builds `continueOp('rebase')`
  or `splitRestOp(paths, message)` of `src/git/ops.ts`.
- **Entry points**: **Continue** in the banner while it says *Splitting "`<subject>`": …* (`operation.continue`).
- **Offered when**: the rebase is stopped to split a commit (`rebase.edit-stop`), and no file is in conflict.
- **Reads**: the files the commit changes (`Repo.changedPaths(hash)`, both names of a renamed file) and the changes
  not committed, compared by `rewrite.split-left`; the message of the commit.
- **Asks**: when files of the commit are still changed, a warning *"`<subject>`" is not all committed yet: 1 file of it
  is still changed.* (*N files of it are*) with *Commit what is left as one more commit, with the message the commit
  had, and continue? Or go back to committing it yourself.* and the buttons **Commit What Is Left** and **Go Back to
  Committing**.
- **Plan**: nothing left: `> git -c core.editor=true rebase --continue`. **Commit What Is Left**:
  `> git add --all -- <files>` (a name that is also a pattern as `:(literal)<name>`), `> git commit --quiet -m
  <message>`, `> git -c core.editor=true rebase --continue`, titled `Commit what is left and continue the rebase`.
- **Danger**: None (the command question only when `gitardo.commandPreview` is `all`); the warning above is asked
  first whenever something is left.
- **Undo**: while the rebase goes on, **Undo** says *Finish or abort the rebase first.*; once it has ended, `undo.last`
  takes back the whole split from the reflog of the branch.
- **Refuses**: changes not committed to files the commit did not change: *`<file>` has changes that are not committed
  and are not part of the split: commit or stash them, then continue.* (*N files have changes…*). **Go Back to
  Committing** runs nothing.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `rewrite.split`, `rewrite.split-left`, `rebase.edit-stop`, `operation.continue`, `commit.create`,
  `stash.push`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: the `continue` case (`src/extension/actions.ts:600-603`); `continueSplit`
  (`actions.ts:667-684`); `splitRestOp` and `continueOp` (`src/git/ops.ts:458-463`, `1206-1208`).
- **Tested by**: `test/integration.test.ts` *splits an older commit in a rebase that stops there, and commits what is
  left of it on the way to the end*; `test/e2e/suite.ts` *splits the last commit and an older one, commits what is left
  on Continue, and undoes each split whole*.

Acceptance:

- [ ] With every change of the commit committed, **Continue** runs `git -c core.editor=true rebase --continue` and the
  commits after it are made again on top.
- [ ] With `c.txt` of the commit still changed, **Continue** says *"Add b and c" is not all committed yet: 1 file of it
  is still changed.*; **Go Back to Committing** leaves the rebase stopped and runs nothing.
- [ ] **Commit What Is Left** runs `git add --all -- c.txt`, `git commit --quiet -m <message of the commit>` and
  `git -c core.editor=true rebase --continue`; the new commit has the whole message the split commit had.
- [ ] With `base.txt`, which the commit did not change, changed and not committed, **Continue** refuses with
  *base.txt has changes that are not committed and are not part of the split: commit or stash them, then continue.*
- [ ] An untracked file that is not part of the commit does not stop it.

### `rewrite.split-left` — What is left of a commit being split

Which changes not committed still belong to the commit being split, and which are in the way of going on.

- **Kind**: Derivation
- **Function**: `splitLeft(paths, working)` in `src/git/ops.ts`.
- **Inputs**: the paths the commit changes, both names of a renamed file; the staged and unstaged changes of the
  working tree, untracked files included.
- **Output**: `left`, the changed files whose path, or old path, the commit changes; `others`, the other changed files
  that are not untracked. Each path once, in the order of the changes.
- **Invariants**: an untracked file that the commit did not add is in neither list: git goes on with it there.
- **Examples**: a commit that added `b.txt` and `c.txt`, both untracked again after the split → `left` `b.txt`,
  `c.txt`, `others` none; `b.txt` committed and `base.txt` changed → `left` `c.txt`, `others` `base.txt`.
- **Uses**: `rewrite.split-rest`.
- **Implemented in**: `splitLeft` (`src/git/ops.ts:449-455`); `Repo.changedPaths` (`src/git/repo.ts:303-307`).
- **Tested by**: `test/integration.test.ts` *splits an older commit in a rebase that stops there, and commits what is
  left of it on the way to the end*.

Acceptance:

- [ ] The two examples above give exactly the lists above.

### `rewrite.fixup` — Add changes to an earlier commit (fixup)

Folds the staged changes, or all changes when nothing is staged, into an earlier commit, rewriting it and the commits
after it.

- **Kind**: Operation
- **Function**: `fixupOp(hash, isRoot, stageAll)` in `src/git/ops.ts`.
- **Entry points**: **Add staged changes to this commit (fixup)** in the context menu of a commit, or **Add all changes
  to this commit (fixup)** when nothing is staged.
- **Offered when**: the commit is on the line from `HEAD` down to the first merge (`rewrite.reach`), it is not `HEAD`
  itself (which **Amend** covers, `commit.amend`), there are changes, no file is in conflict, no operation is under way
  and no bisect.
- **Asks**: always, **Add staged changes to `<short>`?** (or **Add all changes to `<short>`?**) with *This rewrites
  `<short>` and every commit after it. If the changes do not fit there, the rebase stops with conflicts.*
- **Plan**: `> git add --all` when nothing is staged, then `> git commit --fixup=<hash>` and
  `> git -c sequence.editor=true rebase -i --autosquash --autostash <hash>^`, or `--root` for the first commit: git's
  own todo list, accepted as it is.
- **Danger**: Confirm, because it rewrites the commit and those after it: always asked, as the README's list says
  (*fixup*).
- **Undo**: `undo.last` from the reflog of the branch, whose entry before the rebase is the `fixup!` commit: **Undo**
  runs `git reset --keep <that commit>`, which takes back the rebase and leaves the commit `fixup! <subject>` on top,
  with the changes in it; a second **Undo** makes the rebase again. While the rebase is stopped on conflicts, **Undo**
  says *Finish or abort the rebase first.*
- **After**: on conflicts, the banner `Rebase in progress · N files have conflicts` (`operation.continue`,
  `operation.abort`).
- **Refuses**: *Finish or abort the `<operation>` first.*; *There are no changes to add to that commit.*; a hash
  starting with `-`.
- **Settings**: `gitardo.commandPreview` (it is asked anyway).
- **Uses**: `rewrite.reach`, `changes.stage`, `commit.amend`, `operation.continue`, `operation.abort`, `preview.ask`,
  `log.write`, `undo.last`.
- **Implemented in**: `Action` case `fixup` (`src/shared/protocol.ts:582`); `resolveAction` case `fixup`
  (`src/extension/actions.ts:587-592`); `fixupOp` (`src/git/ops.ts:1034-1046`); the menu entry
  (`src/webview/menus.ts:204-207`).
- **Tested by**: `test/integration.test.ts` *folds changes into an earlier commit*; `test/git.test.ts` *folds changes
  into a commit with a fixup commit and an autosquash rebase*.

Acceptance:

- [ ] With `b.txt` staged and `later.txt` untracked, **Add staged changes to this commit (fixup)** on *Add b* asks, then
  runs `git commit --fixup=<hash>` and `git -c sequence.editor=true rebase -i --autosquash --autostash <hash>^`: *Add b*
  has the new `b.txt`, the commits after it are made again, and `later.txt` is still there.
- [ ] With nothing staged, the entry says **Add all changes to this commit (fixup)** and the commands start with
  `git add --all`.
- [ ] On the first commit of the repository the rebase ends with `--root`.
- [ ] It is not offered on `HEAD`, below a merge, with files in conflict, or with no changes.
- [ ] It is asked even with `gitardo.commandPreview` `off`.
- [ ] Changes that do not fit stop the rebase with conflicts and the banner.

### `rewrite.reach` — Which commits a rewrite can reach

Which commits can be given a new message, squashed, split, fixed up or rebased interactively from here: those on the
line from `HEAD` down to the first merge.

- **Kind**: Derivation
- **Function**: `rebaseRange(hash, state)`, `canReword(hash, state)` and `canSquash(rows, state)` in
  `src/webview/menus.ts`, for what the menus offer; `Repo.lineAfter(hash)` in `src/git/repo.ts` and
  `squashRange(hashes, state, repo)` in `src/extension/actions.ts`, for what the host accepts.
- **Inputs**: the commit or the commits; the rows of the graph that are loaded; whether an operation or a bisect is
  under way.
- **Output**: `rebaseRange`: the commits from `HEAD` down to `hash`, newest first, following first parents through the
  loaded rows; null when one of them is not loaded or has more than one parent, `hash` included. `canReword`: false on
  the row of the uncommitted changes, during an operation or a bisect; true on `HEAD`, merge or not; otherwise whether
  `rebaseRange` is not null. `canSquash`: two rows or more, no operation, no bisect, and the rows are the oldest end of
  the range down to the oldest of them, one after the other. `lineAfter(hash)`: the commits after `hash` up to `HEAD`,
  newest first, when `hash` is an ancestor of `HEAD`, each of them has one parent, they make a chain down to `hash`, and
  `hash` is not a merge (`[]` for `HEAD`); null otherwise. `squashRange`: the oldest selected, the others in order, and
  the commits after them; it refuses when the selected commits are not the first ones of the line after the oldest.
- **Invariants**: the reads of `lineAfter` run with `GIT_OPTIONAL_LOCKS=0`.
- **Examples**: rows `d` (`HEAD`) → `c` → `b` (merge of `a` and `x`) → `a`: `canReword` is true for `d` and `c`, false
  for `b` and `a`; with `HEAD` at `b`, true for `b`. Rows `e` → `d` → `c` → `b` (merge) → `a`: `canSquash` is true for
  `e, d` and for `d, c`, false for `e, c` (not one after the other), for `c, b` (a merge), for `e` alone and during a
  rebase. With `base` → `b` → `head`: `lineAfter(base)` is `[head, b]`, `lineAfter(head)` is `[]`, a commit on another
  branch gives null.
- **Uses**: `rebase.editor`, `rewrite.reword`, `rewrite.squash`, `rewrite.split`, `rewrite.fixup`.
- **Implemented in**: `rebaseRange`, `canReword`, `canSquash` (`src/webview/menus.ts:50-61`, `134-147`);
  `Repo.lineAfter` (`src/git/repo.ts:187-198`); `squashRange` (`src/extension/actions.ts:945-955`).
- **Tested by**: `test/menus.test.ts` *is offered on the commits down to the first merge, and not while something else
  is going on*; *is offered for commits that follow one another before HEAD, down to the first merge*;
  `test/integration.test.ts` *changes the message of the last commit and leaves what is staged, or of an older one with
  what is not committed set aside*.

Acceptance:

- [ ] The examples above give exactly the results above.

### `rewrite.pushed-warning` — Warn that a rewritten commit is pushed

Before a commit that a remote branch has already is rewritten, says that the branch will have to be force pushed, and
asks.

- **Kind**: Read
- **Function**: `Repo.remoteBranchesWith(hash)` in `src/git/repo.ts`, and `warnIfPushed(op, hash, repo)` in
  `src/extension/actions.ts`.
- **Entry points**: none of its own: `rewrite.reword` (the commit), `rewrite.squash` (the oldest commit) and
  `rewrite.split` (the commit). Not `rewrite.fixup`, `rebase.interactive`, `rebase.onto` or `reset.to`.
- **Rules**: `git for-each-ref --contains <hash> --format=%(refname:short) refs/remotes`, with
  `GIT_OPTIONAL_LOCKS=0`, leaving out `<remote>/HEAD` and names without `/`; nothing for a value that is not a hash.
  When some remote branch has it, the operation becomes `confirm` and its note starts with *`<short>` is pushed already,
  on `<where>`: the branch will have to be force pushed, and whoever has the old commits will have to move to the new
  ones.* `<where>` names up to three remote branches, then *and N more*.
- **Uses**: `rewrite.reword`, `rewrite.squash`, `rewrite.split`, `sync.push`, `preview.ask`.
- **Implemented in**: `Repo.remoteBranchesWith` (`src/git/repo.ts:201-205`); `warnIfPushed`
  (`src/extension/actions.ts:927-939`).
- **Tested by**: `test/e2e/suite.ts` *changes the message of the last commit, of an older one after asking, and warns
  when it is pushed*; `test/integration.test.ts` *changes the message of the last commit and leaves what is staged, or
  of an older one with what is not committed set aside*.

Acceptance:

- [ ] Rewording the last commit after it was pushed to `origin/reword` asks, even with `gitardo.commandPreview` `off`,
  with *… is pushed already, on origin/reword: the branch will have to be force pushed, and whoever has the old commits
  will have to move to the new ones.*
- [ ] A commit on five remote branches names the first three and *and 2 more*.
- [ ] A commit no remote branch has adds nothing to the question.

### `rewrite.drag-and-drop` — Drag and drop onto a row of the graph

Dragging a commit or a branch onto a row of the graph opens a menu of what can be done with the two: cherry-pick,
merge, rebase or reset.

- **Kind**: View
- **Function**: `dropMenu(source, target, state)` in `src/webview/menus.ts`, with `DragSource`, `startDrag` and
  `dragging` in `src/webview/drag.ts`.
- **Entry points**: a commit row of the graph (not the uncommitted changes, not a stash); the label of a local or
  remote branch on a row; a local or remote branch in the references beside the graph. Rows that accept the drop are
  outlined while dragging.
- **Offered when**: a commit is checked out and no operation is under way; a row with no entry in its menu is no drop
  target, and a stash row never is.
- **Shows**: what each drop offers:

  | Drag | Onto | Offers |
  | --- | --- | --- |
  | A commit other than the one checked out | The commit checked out, or the uncommitted changes | **Cherry-pick `<short>` onto `<current>`** (`cherry-pick.one`) |
  | A branch or remote branch, not the one checked out nor at its commit | The commit checked out, or the uncommitted changes | **Merge `<name>` into `<current>`**, **Merge `<name>` into `<current>`…** (`merge.into-current`, `merge.choose-way`), **Rebase `<current>` onto `<name>`** (`rebase.onto`) |
  | The current branch | Any other commit | **Rebase `<name>` onto `<short>`** (`rebase.onto`), **Reset `<name>` to `<short>`…** (`reset.to`) |
  | Another local branch, which no other worktree has | Any other commit | **Rebase `<name>` onto `<short>`** (`rebase.onto`, with that branch) |

  Anything else offers nothing: a commit dropped elsewhere than on the commit checked out, a remote branch dropped on
  another commit, a branch dropped on its own commit.
- **Uses**: `cherry-pick.one`, `merge.into-current`, `merge.choose-way`, `rebase.onto`, `reset.to`.
- **Implemented in**: `dropMenu` (`src/webview/menus.ts:572-600`); `drag.ts` (`src/webview/drag.ts:1-22`); the sources
  and targets in `App` (`src/webview/App.tsx:467-473`) and in the references (`src/webview/RefsTree.tsx:190`, `272`,
  `286`).
- **Tested by**: `test/menus.test.ts` *offers the same when a branch is dropped on what is checked out*; *offers no
  rebase of a branch another worktree has, by dragging it onto a commit*; `test/e2e/ui.mjs` *a branch dragged onto the
  checked out commit offers to merge or rebase*.

Acceptance:

- [ ] Dropping the local branch `feature` on the commit checked out on `main` offers exactly **Merge feature into
  main**, **Merge feature into main…** and **Rebase main onto feature**.
- [ ] Dropping a commit on the commit checked out offers **Cherry-pick `<short>` onto main**; on any other row, nothing,
  and that row is not outlined.
- [ ] Dropping `main`, checked out, on commit `1a2b3c4` offers **Rebase main onto 1a2b3c4** and **Reset main to
  1a2b3c4…**.
- [ ] Dropping a branch that another worktree has checked out on another commit offers nothing.
- [ ] While an operation is under way, no drop offers anything.

### `bisect.start` — Start a bisect

Starts a `git bisect` from a commit the user knows is bad, or good, to find the commit that broke something.

- **Kind**: Operation
- **Function**: `bisectOp(mark, hash, active = false)` in `src/git/ops.ts`.
- **Entry points**: **Start bisect: this commit is bad** and **Start bisect: this commit is good** in the context menu
  of a commit in the graph and the Gitardo Graph.
- **Offered when**: no operation is under way and no bisect; during a bisect the same entries become `bisect.mark`.
- **Plan**: `> git bisect start`, then `> git bisect bad <hash>` (or `good`). Title `Start bisect: <short> is bad`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: while a bisect is under way, **Undo** refuses with *End the bisect first.*, as each step is a checkout;
  `bisect.end` goes back to where it started.
- **After**: the banner of `bisect.banner`, which asks for the other mark: *Bisect: mark a good commit to begin* (or a
  bad one), with **End** only; the other mark is given from the context menu (**Bisect: mark as good**).
- **Refuses**: a hash starting with `-`.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `bisect.mark`, `bisect.banner`, `bisect.end`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `bisect` (`src/shared/protocol.ts:584`); `resolveAction` case `bisect`
  (`src/extension/actions.ts:594-595`); `bisectOp` (`src/git/ops.ts:1049-1057`); `mark` in `commitMenu`
  (`src/webview/menus.ts:179-182`, `222-223`); the `undo` case (`actions.ts:389-390`).
- **Tested by**: `test/git.test.ts` *starts a bisect with the first mark and only marks afterwards*;
  `test/integration.test.ts` *bisects down to the first bad commit*; `test/e2e/suite.ts` *bisects from the graph and
  reports the first bad commit*.

Acceptance:

- [ ] **Start bisect: this commit is bad** on `1a2b3c4` runs `git bisect start` and `git bisect bad <hash>`, titled
  `Start bisect: 1a2b3c4 is bad`, both in the log.
- [ ] The entries are not offered while an operation is under way.
- [ ] Afterwards the banner says *Bisect: mark a good commit to begin*, and the commit carries the label **bad**.
- [ ] **Undo** during the bisect says *End the bisect first.* and runs nothing.

### `bisect.mark` — Mark the commit tested

Answers git for the commit it checked out, or marks any commit, as good, bad, or one that cannot be tested.

- **Kind**: Operation
- **Function**: `bisectOp(mark, hash?, active = true)` in `src/git/ops.ts`.
- **Entry points**: **Good**, **Bad** and **Skip** in the banner (tooltips *git bisect good*, *git bisect bad*, *git
  bisect skip: this commit cannot be tested*); **Bisect: mark as good** and **Bisect: mark as bad** in the context menu
  of any commit.
- **Offered when**: the banner buttons while a bisect has a good and a bad commit and more than one can still be the
  first bad one, disabled while an operation runs; the menu entries during a bisect while no operation is under way.
- **Plan**: from the banner, `> git bisect good`, `> git bisect bad` or `> git bisect skip`, for the commit checked
  out, titled `Bisect: this commit is good`, `… is bad`, `… is skipped`; from the menu, `> git bisect good <hash>`,
  titled `Bisect: <short> is good`. Git checks out the next commit to test.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: **Undo** refuses with *End the bisect first.*
- **After**: when one commit is left, a message *Bisect: the first bad commit is `<short>` "`<subject>`".* with **Show in
  the graph** and **End bisect**, and the banner says *Bisect: the first bad commit is `<short>`*.
- **Refuses**: **Skip** with no bisect under way: *No bisect is in progress.*; a hash starting with `-`.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `bisect.banner`, `bisect.end`, `preview.ask`, `log.write`.
- **Implemented in**: `bisectOp` (`src/git/ops.ts:1049-1057`); `BisectBanner` (`src/webview/BisectBanner.tsx:13`,
  `29-41`); `mark` in `commitMenu` (`src/webview/menus.ts:179-182`); `Controller.reportBisect`
  (`src/extension/controller.ts:1687`, `1794-1805`).
- **Tested by**: `test/git.test.ts` *starts a bisect with the first mark and only marks afterwards*;
  `test/integration.test.ts` *bisects down to the first bad commit*; `test/e2e/suite.ts` *bisects from the graph and
  reports the first bad commit*.

Acceptance:

- [ ] **Good** in the banner runs `git bisect good` with no hash, titled `Bisect: this commit is good`, and git checks
  out another commit.
- [ ] **Bisect: mark as bad** on a commit runs `git bisect bad <hash>`.
- [ ] Answering until one commit is left shows *Bisect: the first bad commit is `<short>` "`<subject>`".*;
  **Show in the graph** selects it, **End bisect** runs `bisect.end`.
- [ ] A hash `--all` is refused with `"--all" is not a valid name.`

### `bisect.end` — End the bisect

Ends the bisect and goes back to what was checked out before it started.

- **Kind**: Operation
- **Function**: `bisectResetOp()` in `src/git/ops.ts`.
- **Entry points**: **End** in the banner (tooltip *git bisect reset*); **End bisect** in the message that names the
  first bad commit; **Gitardo: End Bisect** in the Command Palette.
- **Offered when**: the banner and the message during a bisect; the command always.
- **Plan**: `> git bisect reset`, titled `End bisect`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: `undo.last` from the reflog, whose last entry is the checkout back: **Undo** checks out again the commit
  that was being tested, detached, with `git checkout --detach <hash>`; the bisect does not come back.
- **After**: the banner and the labels **good**, **bad** and **skipped** go away.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `bisect.banner`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `bisectReset` (`src/shared/protocol.ts:585`); `resolveAction` case `bisectReset`
  (`src/extension/actions.ts:596-597`); `bisectResetOp` (`src/git/ops.ts:1059-1061`); the command
  `gitardo.bisectReset` (`src/extension/extension.ts:182`, `package.json`); `BisectBanner`
  (`src/webview/BisectBanner.tsx:42-44`).
- **Tested by**: `test/e2e/suite.ts` *bisects from the graph and reports the first bad commit*;
  `test/integration.test.ts` *bisects down to the first bad commit*.

Acceptance:

- [ ] **End** runs `git bisect reset`: the branch checked out before the bisect is checked out again, and the banner is
  gone.
- [ ] **Gitardo: End Bisect** does the same from the Command Palette.

### `bisect.banner` — The bisect banner

The bar above the graph while a bisect is under way: what git needs next, how many commits can still be the first bad
one, and the buttons that answer.

- **Kind**: View
- **Function**: `BisectBanner({ state, busy, onShow })` in `src/webview/BisectBanner.tsx`, from `Repo.bisect()` in
  `src/git/repo.ts`.
- **Entry points**: shown on its own during a bisect, in the Graph view (in the panel or an editor tab) and in the
  Changes view; not in the Gitardo Graph.
- **Reads**: the marks, from `git for-each-ref --format=%(refname) %(objectname) refs/bisect`, and, once there is a bad
  commit and a good one, how many can still be the first bad one, counting the bad one:
  `git rev-list --count <bad> --not <good>…`; both with `GIT_OPTIONAL_LOCKS=0`.
- **Shows**: a search icon and *Bisect: mark a bad commit to begin* or *Bisect: mark a good commit to begin* while one
  is missing, with **End** only; then *Bisect: test `<short>` · N commits can still be the first bad one* with **Good**,
  **Bad**, **Skip** and **End**; once one is left, *Bisect: the first bad commit is `<short>`* with **Show**, which
  selects it (in the Graph view only), and **End**. The buttons are disabled while an operation runs. In the graph the
  marked commits carry the labels **good**, **bad** and **skipped**.
- **Uses**: `bisect.start`, `bisect.mark`, `bisect.end`.
- **Implemented in**: `BisectBanner` (`src/webview/BisectBanner.tsx:7-47`), placed in `App`
  (`src/webview/App.tsx:390`) and `ChangesApp` (`src/webview/ChangesApp.tsx:219`); `Repo.bisect`
  (`src/git/repo.ts:444-454`); the labels (`src/webview/labels.ts:42-47`); `Bisect` (`src/shared/protocol.ts:174-180`).
- **Tested by**: `test/git.test.ts` *reads the marks of a bisect from its refs*; `test/e2e/suite.ts` *bisects from the
  graph and reports the first bad commit*.

Acceptance:

- [ ] With only a bad commit marked, the banner says *Bisect: mark a good commit to begin* and has **End** only.
- [ ] With six commits, the fourth broken, the first good and the last bad, the banner says *Bisect: test `<short>` · 5
  commits can still be the first bad one*, and has **Good**, **Bad**, **Skip** and **End**.
- [ ] When one commit is left, it says *Bisect: the first bad commit is `<short>`* with **Show** and **End**.
- [ ] The refs `refs/bisect/bad bbb`, `refs/bisect/good-aaa aaa`, `refs/bisect/good-ccc ccc` and
  `refs/bisect/skip-ddd ddd` read as good `aaa`, `ccc`, bad `bbb`, skipped `ddd`.
