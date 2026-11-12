# Changes and commit

What is not committed yet, and making a commit of it: the **Gitardo Changes** view of the side bar with its one list
of files, staging and unstaging them whole or a change at a time, discarding them, opening them and their diffs, the
files with edits not saved in an editor, and the commit box with its message, amend and the message drafted by a
language model. Discarding is also offered on the uncommitted changes row of the graph and its files in the details, and
staging and discarding in the title of the Flow Diff: those entry points are on the cards of what they run. Stashing
from this view is `stash.push`; the buttons of the files in conflict are the cards of the conflicts area.

### `changes.view` — The Changes view

The view of the side bar that lists every file not committed, staged or not, in one list with a box each, and the box
to commit them.

- **Kind**: View
- **Function**: `ChangesApp()` in `src/webview/ChangesApp.tsx`, view kind `changes`, registered as `gitardo.changes`
  in `src/extension/extension.ts`.
- **Entry points**: **Gitardo: Show Changes** in the Command Palette (`gitardo.openChanges`, which focuses the view);
  ticking **Gitardo Changes** in the **…** menu of the Source Control side bar; **Show Changes** in the message that
  follows a squash merge.
- **Offered when**: always, once asked for: the view is declared with `"visibility": "hidden"` in `package.json`, so
  VS Code does not show it until the user does, and keeps it until the user hides it again.
- **Reads**: the state of the repository (`state` messages, without the commits of the graph, since the view does not
  show the graph), and the files with edits not saved (`unsaved` messages).
- **Uses**: `changes.stage`, `changes.unstage`, `changes.stage-all`, `changes.unstage-all`, `changes.discard`,
  `changes.discard-all`, `changes.open`, `changes.open-all`, `changes.layout`, `changes.unsaved`, `changes.save`,
  `changes.compare-with-saved`, `commit.box`, `stash.push`, `conflict.review`, `conflict.resolve-ai`,
  `conflict.take-side`, `operation.continue`, `operation.abort`; the box that chooses the repository and the banners of
  a worktree, an operation under way and a bisect, as the graph has them. `workingChanges(working)` in
  `src/git/parse.ts` puts the staged and the unstaged files in one list, each with how much of it is staged.
- **Implemented in**: `ChangesApp`, `Group` and `CommitBox` in `src/webview/ChangesApp.tsx`; `FileRow`, `Check` and
  `ActionButtons` in `src/webview/FileRow.tsx`; `useCapped` and `FILES_LIMIT` (300) in `src/webview/Capped.tsx`;
  `sideView('changes')` and `gitardo.openChanges` in `src/extension/extension.ts`; the `gitardo.changes` view in
  `package.json`.
- **Tested by**: `test/e2e/ui.mjs`, steps "the views are in the Source Control side bar of VS Code, folded under its
  own, and the one of the changes is hidden until asked for" and "the changes view of Gitardo shows when it is asked
  for, and lists what changed"; `test/git.test.ts`, "lists staged and unstaged files together, each with how much of it
  is staged and what it is next to HEAD".

Acceptance:

- [ ] After installing, the side bar does not show **Gitardo Changes**; **Gitardo: Show Changes** shows it, open.
- [ ] With `b.txt` changed and `new.txt` untracked, the view lists the two under one header **Changes**, which counts
  `2`, each with a box, its status letter, its file icon and its name, its directory beside the name.
- [ ] A file with some of its changes staged and some not is listed once, with its box half ticked; the header box is
  ticked when every file is wholly staged, half ticked when some is staged, and empty when nothing is.
- [ ] Files in conflict are listed apart, under **Conflicts** above **Changes**, with no box, and with the buttons
  **Mark as resolved**, **Take the current side for the whole file** and **Take the incoming side for the whole file**,
  plus the wand and the sparkle of the review where they apply.
- [ ] Each changed file has, in order, **Open File** (not on a deleted file), **Open Changes**, **Save** (only while an
  editor has edits of it not saved), **Stash this file…** and **Discard changes**.
- [ ] The header of **Changes** has, in order, **View as tree** or **View as list**, **Open all changes in one
  editor**, **Stash only the staged changes…** (only while something is staged), **Stash all changes…**, **Save all**
  (only while some file has edits not saved) and **Discard all changes**.
- [ ] With 301 changed files the list shows the first 300 and **Show all 301 files…**; the header still counts `301`
  and its box and buttons act on all 301.
- [ ] With nothing to commit and no edits not saved the view says *No changes*.
- [ ] Before the repository is read the view says *Loading repository…*; in a workspace with no repository it says
  *No Git repository in this workspace* with a **Check again** button, which reads the workspace again.
- [ ] While an operation runs, the **Commit** button and the box that chooses the repository are disabled.

### `changes.stage` — Stage a file

Puts the changes of a file in the index, to be committed, by ticking its box.

- **Kind**: Operation
- **Function**: `stageOp(paths)` in `src/git/ops.ts`.
- **Entry points**: ticking the box of a file in the Changes view (title *Click to stage*, or *Partly staged: click to
  stage all of it* on a half ticked one); **Mark as resolved** on a file in conflict in the Changes view; **Stage
  File** in the title of the Flow Diff; ticking the header box of the Changes view while files are in conflict (see
  `changes.stage-all`).
- **Offered when**: the box is on every changed file; it is greyed out on a file that only an editor has changed
  (*Not saved: save it to stage it*). **Stage File** shows in the Flow Diff when its right side is the working tree
  (`gitardo.flowDiffSide == worktree`).
- **Plan**: `> git add -- <file>`; several files in one command, `git add -- a.ts b.ts`. A file whose name is also a
  pattern goes as `:(literal)<file>` (`pathspec(file)`).
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: not taken back. Staging moves no reference, so **Undo** right after takes back what the reflog has last,
  such as the commit before, and shows it first. Staged from the Changes view, it also makes Gitardo forget what it
  remembered to take back an earlier operation (a branch deleted, a stash dropped); staged from the Flow Diff, it does
  not.
- **After**: the box is ticked once the repository is read again; the file stays where it was in the list.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` `{ kind: 'stage'; paths }` in `src/shared/protocol.ts`; `case 'stage'` of
  `resolveAction` in `src/extension/actions.ts`; `stageOp` and `pathspec` in `src/git/ops.ts`; `onPick` of the row and
  **Mark as resolved** in `src/webview/ChangesApp.tsx`; `FlowDiff.fileAction('stage')` in `src/flow/flowDiff.ts`, run
  through `Controller.runIn`; `Controller.action`, which clears `undoable` after it.
- **Tested by**: `test/git.test.ts`, "marks a file name that is also a pattern to be taken as it is written, and leaves
  plain names alone"; `test/integration.test.ts`, "stages, unstages and commits in a repository with no commits yet";
  `test/awkward.test.ts`, "stages, unstages, cuts into hunks, discards and stashes files whatever their names" and
  "takes a name with brackets as that file, not as a pattern that other files match too"; `test/e2e/suite.ts`, step
  "shows the exact commands and waits when the preview is on"; `test/e2e/ui.mjs`, steps "stages a file by ticking its
  box, in the one list of changes, and commits it with a typed message" and "with the command preview on, a real dialog
  shows the exact command and Cancel runs nothing".

Acceptance:

- [ ] Ticking the box of `b.txt` runs `git add -- b.txt` through `Controller.run`, and the log has `> git add -- b.txt`.
- [ ] With `gitardo.commandPreview` set to `off`, nothing is asked; set to `all`, a question *Stage b.txt?* shows
  `git add -- b.txt` and the folder it runs in, with **Run**; **Cancel** stages nothing and leaves the box unticked.
- [ ] The command in the question is the one in the log, with nothing added.
- [ ] Ticking `pages/[id].tsx` runs `git add -- :(literal)pages/[id].tsx`, and `pages/i.tsx` stays unstaged.
- [ ] A file named `-dash.txt` is staged with `git add -- -dash.txt`: it comes after `--`, so it is never an option.
- [ ] Ticking a half ticked box stages the rest of the file: the box is then ticked.
- [ ] **Mark as resolved** on `clash.ts` in conflict runs `git add -- clash.ts`, and the file leaves **Conflicts**.
- [ ] **Stage File** in the Flow Diff of `a.txt` runs `git add -- a.txt` through `Controller.runIn`, logged the same
  way, and the Flow Diff is drawn again.
- [ ] Before the first commit, ticking `a.txt` runs `git add -- a.txt` and lists it as added.
- [ ] **Undo** right after staging does not unstage the file.

### `changes.unstage` — Unstage a file

Takes a file out of the index, leaving its changes in the working tree, by unticking its box.

- **Kind**: Operation
- **Function**: `unstageOp(paths, hasHead)` in `src/git/ops.ts`.
- **Entry points**: unticking the ticked box of a file in the Changes view (title *Staged: click to unstage*);
  **Unstage File** in the title of the Flow Diff; unticking the header box of the Changes view while files are in
  conflict (see `changes.unstage-all`).
- **Offered when**: the box is ticked when all of the file is staged. **Unstage File** shows in the Flow Diff when its
  right side is the index (`gitardo.flowDiffSide == index`).
- **Plan**: `> git restore --staged -- <file>`; a staged rename with both of its names,
  `git restore --staged -- new.txt old.txt`; before the first commit, when there is no `HEAD` to restore from,
  `git rm --cached -r -- <file>`. Names that are patterns go as `:(literal)<file>`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: not taken back: it moves no reference, so **Undo** right after takes back what the reflog has last, and
  shows it first.
- **After**: the box is empty once the repository is read again. A rename unstaged is listed as git sees it: the old
  name deleted, the new one untracked; ticking both stages it as a rename again.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` `{ kind: 'unstage'; paths }` in `src/shared/protocol.ts`; `case 'unstage'` of
  `resolveAction` in `src/extension/actions.ts`; `unstageOp` in `src/git/ops.ts`; `onPick` with `pathsOf(file)` in
  `src/webview/ChangesApp.tsx`; `FlowDiff.fileAction('unstage')` in `src/flow/flowDiff.ts`.
- **Tested by**: `test/git.test.ts`, "unstages without HEAD by removing from the index"; `test/integration.test.ts`,
  "stages, unstages and commits in a repository with no commits yet" and "unstages both sides of a staged rename";
  `test/awkward.test.ts`, "stages, unstages, cuts into hunks, discards and stashes files whatever their names".

Acceptance:

- [ ] Unticking `a.ts` runs `git restore --staged -- a.ts` through `Controller.run`, logged as
  `> git restore --staged -- a.ts`.
- [ ] Unticking the staged rename of `old.txt` to `new.txt` runs `git restore --staged -- new.txt old.txt`, and
  nothing is left staged.
- [ ] In a repository with no commit yet, unticking `a.txt` runs `git rm --cached -r -- a.txt`, and `a.txt` is
  listed as untracked again, still on disk.
- [ ] Unticking `a*.ts` runs `git restore --staged -- :(literal)a*.ts`.
- [ ] Nothing is asked with `gitardo.commandPreview` set to `off`; with `all`, *Unstage a.ts?* shows the command first.
- [ ] **Unstage File** in the Flow Diff of the index runs the same command through `Controller.runIn`.

### `changes.stage-all` — Stage all changes

Puts every change in the index at once.

- **Kind**: Operation
- **Function**: `stageAllOp()` in `src/git/ops.ts`; with files in conflict, the Changes view uses `stageOp(paths)`.
- **Entry points**: ticking the header box of **Changes** in the Changes view (*Click to stage all changes*); **Stage
  all changes** in the context menu of the uncommitted changes row of the graph.
- **Offered when**: the header box is enabled while git sees a change (not only edits not saved). The menu entry shows
  while something is not staged and no file is in conflict: staging everything would take the conflicts as resolved,
  markers and all.
- **Plan**: `> git add --all`. With files in conflict, the header box names the other files one by one instead,
  `git add -- a.ts b.ts`, the ones not wholly staged.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: not taken back: it moves no reference, so **Undo** right after takes back what the reflog has last, and
  shows it first.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `changes.stage`, `preview.ask`, `log.write`.
- **Implemented in**: `Action` `{ kind: 'stageAll' }` in `src/shared/protocol.ts`; `case 'stageAll'` of
  `resolveAction` in `src/extension/actions.ts`; `stageAllOp` in `src/git/ops.ts`; `stageAll` in
  `src/webview/ChangesApp.tsx`; `changesMenu(state)` in `src/webview/menus.ts`.
- **Tested by**: `test/menus.test.ts`, "stashes, stages, unstages and discards them all, as far as there is something
  to" and "leaves out stashing before the first commit, staging over conflicts, and discarding during a merge";
  `test/e2e/suite.ts`, step "stages and commits without asking when the preview is off".

Acceptance:

- [ ] With `a.txt` changed and `outside.txt` untracked, ticking the header box runs `git add --all`; both are then
  staged and the header box is ticked.
- [ ] With `clash.ts` in conflict and `a.ts`, `b.ts` changed, the header box runs `git add -- a.ts b.ts`, titled
  *Stage 2 files*, and `clash.ts` stays in conflict.
- [ ] The context menu of the uncommitted changes row has **Stage all changes** while something is not staged, and not
  while a file is in conflict.
- [ ] Nothing is asked with `gitardo.commandPreview` set to `off`; with `all`, *Stage all changes?* shows
  `git add --all` first.

### `changes.unstage-all` — Unstage all changes

Takes everything out of the index at once, leaving the changes in the working tree.

- **Kind**: Operation
- **Function**: `unstageAllOp(hasHead)` in `src/git/ops.ts`; with files in conflict, the Changes view uses
  `unstageOp(paths, hasHead)`.
- **Entry points**: unticking the ticked header box of **Changes** in the Changes view (*All staged: click to unstage
  all changes*); **Unstage all changes** in the context menu of the uncommitted changes row of the graph.
- **Offered when**: the header box is ticked when every file is wholly staged. The menu entry shows while something is
  staged, also while files are in conflict.
- **Plan**: `> git reset`; before the first commit, `git rm --cached -r -- .`. With files in conflict, the header box
  names the staged files one by one instead, `git restore --staged -- a.ts b.ts`, both names of a rename.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: not taken back. `git reset` writes `reset: moving to HEAD` in the reflog of `HEAD` but not in that of the
  branch, so **Undo** right after takes back the last move of the branch, such as the commit before, and shows it first.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `changes.unstage`, `preview.ask`, `log.write`.
- **Implemented in**: `Action` `{ kind: 'unstageAll' }` in `src/shared/protocol.ts`; `case 'unstageAll'` of
  `resolveAction` in `src/extension/actions.ts`; `unstageAllOp` in `src/git/ops.ts`; `unstageAll` in
  `src/webview/ChangesApp.tsx`; `changesMenu(state)` in `src/webview/menus.ts`.
- **Tested by**: `test/git.test.ts`, "unstages without HEAD by removing from the index"; `test/menus.test.ts`,
  "stashes, stages, unstages and discards them all, as far as there is something to".
- **Differs**: **Unstage all changes** in the menu of the uncommitted changes row is offered while files are in
  conflict (`src/webview/menus.ts:161`), and the header box of the Changes view runs `git reset` whenever no file is in
  conflict (`src/webview/ChangesApp.tsx:207-212`). During a merge or a cherry-pick, `git reset` ends it: it deletes
  `MERGE_HEAD` or `CHERRY_PICK_HEAD` and forgets the conflicts, leaving their markers in the files, with nothing asked.
  The Changes view avoids this only while files are in conflict.

Acceptance:

- [ ] With `a.txt` and `b.txt` staged and nothing in conflict, unticking the header box runs `git reset`, and both
  are listed unstaged.
- [ ] Before the first commit it runs `git rm --cached -r -- .`, and every file is untracked again, still on disk.
- [ ] With `clash.ts` in conflict and `a.ts` staged, unticking from the Changes view runs
  `git restore --staged -- a.ts`, and `clash.ts` stays in conflict.
- [ ] During a merge, unstaging all asks first and keeps the merge under way, or is not offered. (differs)
- [ ] Nothing is asked with `gitardo.commandPreview` set to `off`; with `all`, *Unstage all changes?* shows the
  command first.

### `changes.stage-hunk` — Stage or unstage one change of a file

Stages or unstages one change of a file rather than the whole file, from the buttons on the change in the Flow Diff.

- **Kind**: Operation
- **Function**: `chunkOp(verb, file, patch)` in `src/flow/chunks.ts`, which the Flow Diff runs. The actions
  `stageHunk`, `unstageHunk` and `discardHunk` and their builder `hunkOp(kind, file, patch)` in `src/git/ops.ts` do the
  same with a hunk of `git diff`, but no view posts them today.
- **Entry points**: the stage button in the gutter of the Flow Diff on a change, and the unstage button on a change
  already staged (drawn dimmed). Ranges staged in VS Code's own diff editor (**Stage Selected Ranges**) are VS Code's,
  not Gitardo's: the Changes view shows their result.
- **Offered when**: the Flow Diff shows a file against `HEAD` or the index, without a rename (`canStage(descriptor)` in
  `src/flow/sides.ts`).
- **Reads**: the file in the index, in `HEAD` and on disk, to build a patch of that change alone, and checks that what
  the Flow Diff shows is still what is there.
- **Plan**: `> git apply --cached --unidiff-zero --whitespace=nowarn -`, with the patch on standard input, which the
  log writes after `standard input:`. An untracked file is staged whole, `git add -- <file>`; a change of a file that
  `HEAD` does not have is unstaged whole, `git restore --staged -- <file>`. `hunkOp` would run
  `git apply --cached -`, `git apply --cached --reverse -` and, to discard, `git apply --reverse -`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`), with the note *The patch on standard input
  changes this part of a.txt in the index:* and the `@@` line of the change. `hunkOp`'s discard is destructive:
  *The change will be removed from the file on disk.*
- **Undo**: not taken back: it moves no reference, so **Undo** right after takes back what the reflog has last.
- **Refuses**: *Flow Diff: stage chunk failed — chunk staging needs UTF-8 text, this file is <encoding> — stage the
  whole file* when the file is not UTF-8 and the change has characters past ASCII; *… the first line of a file with a
  BOM cannot be staged per chunk — stage the whole file* for the first line of a file that starts with a BOM.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `changes.stage`, `changes.unstage`, `preview.ask`, `log.write`; the Flow Diff, which shows the change.
- **Implemented in**: `ChunkActions.stageChunk`, `unstageChunk`, `applyToIndex` and `chunkOp` in `src/flow/chunks.ts`,
  run through `Controller.runIn`; `Action` `{ kind: 'stageHunk' | 'unstageHunk' | 'discardHunk'; path; patch }` in
  `src/shared/protocol.ts`, `case 'stageHunk'` of `resolveAction` in `src/extension/actions.ts` and `hunkOp` in
  `src/git/ops.ts`, reached only by tests.
- **Tested by**: `test/e2e/suite.ts`, step "switches a diff between the diff editor and the Flow Diff, which stages and
  unstages a change of it"; `test/e2e/ui.mjs`, step "a diff opens in the Flow Diff from the button of the diff editor,
  stages a change there, and goes back"; `test/flow/chunkPatch.test.ts`, "stage one chunk out of two" and the tests
  after it; `test/integration.test.ts`, "stages, unstages and discards single hunks" and "stages, unstages and discards
  single lines of a hunk" (for `hunkOp`).

Acceptance:

- [ ] In the Flow Diff of `a.txt` against `HEAD`, the stage button of its first change runs
  `git apply --cached --unidiff-zero --whitespace=nowarn -` through `Controller.runIn`, and the log has that line and
  the patch after `standard input:`.
- [ ] After it, the index has that change and not the others; the Changes view shows `a.txt` half ticked, and ticking
  it stages the rest.
- [ ] The unstage button on the change staged takes it out of the index, and the file on disk is unchanged.
- [ ] With `gitardo.commandPreview` set to `all`, *Stage change in a.txt?* shows the command and the note with the
  `@@` line of the change before it runs.
- [ ] On a file whose encoding is not UTF-8, a change with a character past ASCII is refused with the message above,
  and the index is unchanged.

### `changes.discard` — Discard the changes of a file

Takes a file back to what was committed, or to what is staged, losing its changes; an untracked file is deleted.

- **Kind**: Operation
- **Function**: `discardOp(paths, working)` in `src/git/ops.ts` for the changes not staged;
  `discardChangesOp(paths, working, hasHead)` in `src/git/ops.ts` for all of a file, staged changes included.
- **Entry points**: **Discard changes** on a file of the Changes view; **Discard all changes in this file, staged too…**
  on a file of the uncommitted changes in the details of the graph; **Discard all changes in 1 selected file…** or
  **Discard all changes in N selected files…** above the files of the uncommitted changes in the details, once some are
  ticked; **Discard Changes** in the title of the Flow Diff.
- **Offered when**: on every changed file of the Changes view and of the details, also during an operation under way.
  **Discard Changes** shows in the Flow Diff when its right side is the working tree.
- **Plan**:
  - Nothing of the file staged (Changes view), or the Flow Diff: `> git restore -- a.ts` for a tracked file, back to
    what the index has; `> git clean -f -- new.ts` for an untracked one.
  - Some or all of it staged (Changes view), and every entry of the details: `> git restore --source=HEAD --staged
    --worktree -- a.ts`, back to `HEAD`, which deletes a file added since; both names of a rename; an untracked file
    with `git clean -f -- <file>`. Before the first commit, `git rm -r --force --quiet -- <file>`.
  - A name that is also a pattern goes as `:(literal)<file>`.
- **Danger**: Destructive, because the changes are in no commit and no stash. Always asked, with a warning:
  *Discard changes in a.ts?* with *Unstaged changes to tracked files will be lost.* and *One untracked file will be
  deleted from disk.*; or *Discard all changes in a.ts?* with *One file goes back to the last commit, staged changes
  included: they will be lost, and a file added since is deleted from disk.*
- **Undo**: cannot bring the changes back. **Undo** right after takes back what the reflog has last instead, such as
  the commit before, and shows it first.
- **Refuses**: *There is nothing to discard.* when none of the files given has a change.
- **Settings**: none: it asks whatever `gitardo.commandPreview` says.
- **Uses**: `preview.ask`, `log.write`.
- **Implemented in**: `Action` `{ kind: 'discard'; paths }` and `{ kind: 'discardChanges'; paths? }` in
  `src/shared/protocol.ts`; `case 'discard'` and `case 'discardChanges'` of `resolveAction` in
  `src/extension/actions.ts`; `discardOp` and `discardChangesOp` in `src/git/ops.ts`; the **Discard changes** button
  in `src/webview/ChangesApp.tsx`; `fileActions` and `changesActions` in `src/webview/Detail.tsx`;
  `FlowDiff.fileAction('discard')` in `src/flow/flowDiff.ts`, run through `Controller.runIn`.
- **Tested by**: `test/git.test.ts`, "restores tracked files and deletes untracked ones when discarding" and "marks a
  file name that is also a pattern to be taken as it is written, and leaves plain names alone";
  `test/integration.test.ts`, "shows uncommitted changes as the first row and discards them", "discards changes staged
  or not, one file at a time or all at once, and what was added since" and "discards a file taken out of the index and
  left on disk back to HEAD, rather than deleting it"; `test/awkward.test.ts`, "takes a name with brackets as that file,
  not as a pattern that other files match too"; `test/e2e/ui.mjs`, step "the changes that are not committed are stashed
  or discarded from their row in the graph, a file at a time, or the ones ticked in the details".

Acceptance:

- [ ] **Discard changes** on `a.ts`, changed and not staged, asks *Discard changes in a.ts?* as a warning with
  `git restore -- a.ts`, whatever `gitardo.commandPreview` says; **Run** runs it through `Controller.run` and logs it.
- [ ] On an untracked `new.ts` it runs `git clean -f -- new.ts`, says *One untracked file will be deleted from disk.*,
  and `new.ts` is gone from disk.
- [ ] On `a.txt` partly staged it runs `git restore --source=HEAD --staged --worktree -- a.txt`, and `a.txt` is as
  `HEAD` has it, nothing staged.
- [ ] On the staged rename of `old.txt` to `new.txt`, the file goes back to `old.txt` and `new.txt` is deleted.
- [ ] A file taken out of the index and left on disk goes back to what `HEAD` has, and is not deleted.
- [ ] In the details, ticking `one.txt` and `two.txt`, both untracked, and choosing **Discard all changes in 2
  selected files…** asks *Discard all changes in 2 files?* with `git clean -f -- one.txt two.txt` and *2 untracked
  files will be deleted from disk.*
- [ ] Discarding `pages/[id].txt` runs `git restore -- :(literal)pages/[id].txt`, and `pages/i.txt` and
  `pages/d.txt` keep their changes.
- [ ] **Discard Changes** in the Flow Diff of `a.ts` against `HEAD` runs `git restore -- a.ts`, asked first: what is
  staged of `a.ts` stays.
- [ ] **Cancel** in the question leaves every file as it was.

### `changes.discard-all` — Discard all changes

Takes every file back to the last commit, staged changes included, and deletes the untracked files and folders.

- **Kind**: Operation
- **Function**: `discardChangesOp(null, working, hasHead)` in `src/git/ops.ts`; the header of the Changes view uses
  `discardChangesOp(paths, …)` with every file listed, or `discardOp` of the files not staged when nothing is staged.
- **Entry points**: **Discard all changes…** in the context menu of the uncommitted changes row of the graph;
  **Discard all changes…** above the files of the uncommitted changes in the details, while none is ticked; **Discard
  all changes** on the header of **Changes** in the Changes view.
- **Offered when**: the entries of the graph and of the details only while no operation is under way, which **Abort**
  takes back instead. The button of the Changes view always.
- **Plan**:
  - From the graph and the details: `> git restore --source=HEAD --staged --worktree -- .` and
    `> git clean -f -d -- .`, each only when there is something for it. Before the first commit,
    `git rm -r --force --quiet -- .` in place of the first.
  - From the Changes view with something staged: `git restore --source=HEAD --staged --worktree -- <every file>` and
    `git clean -f -- <every untracked file>`. With nothing staged: `git restore -- <every changed file>` and
    `git clean -f -- <every untracked file>`. Files in conflict are left out.
- **Danger**: Destructive, because nothing of it is in a commit or a stash. Always asked, with a warning:
  *Discard all changes?* with, for example, *2 files go back to the last commit, staged changes included: they will be
  lost, and a file added since is deleted from disk. One untracked file will be deleted from disk.*
- **Undo**: cannot bring the changes back. **Undo** right after takes back what the reflog has last instead, and shows
  it first.
- **Refuses**: *Abort the merge to discard all of it.* (or the rebase, cherry-pick, revert) when asked without files
  during an operation under way; *There is nothing to discard.* with no change.
- **Settings**: none: it asks whatever `gitardo.commandPreview` says.
- **Uses**: `changes.discard`, `operation.abort`, `preview.ask`, `log.write`.
- **Implemented in**: `Action` `{ kind: 'discardChanges' }` and `{ kind: 'discardAll' }` in `src/shared/protocol.ts`;
  `case 'discardChanges'` and `case 'discardAll'` of `resolveAction` in `src/extension/actions.ts`;
  `discardChangesOp` and `discardOp` in `src/git/ops.ts`; `changesMenu(state)` in `src/webview/menus.ts`;
  `changesActions` in `src/webview/Detail.tsx`, with `discardAll: !state.operation` from `src/webview/App.tsx`; the
  **Discard all changes** button in `src/webview/ChangesApp.tsx`.
- **Tested by**: `test/integration.test.ts`, "discards changes staged or not, one file at a time or all at once, and
  what was added since" and "discards everything before the first commit, staged files too"; `test/menus.test.ts`,
  "leaves out stashing before the first commit, staging over conflicts, and discarding during a merge";
  `test/e2e/suite.ts`, step "always asks before an operation that loses work"; `test/e2e/ui.mjs`, step "the changes
  that are not committed are stashed or discarded from their row in the graph, a file at a time, or the ones ticked in
  the details".

Acceptance:

- [ ] **Discard all changes…** in the menu of the uncommitted changes row asks *Discard all changes?* as a warning with
  `git restore --source=HEAD --staged --worktree -- .` and `git clean -f -d -- .`, whatever
  `gitardo.commandPreview` says, and runs them through `Controller.run` on **Run**.
- [ ] After it, `git status --porcelain` prints nothing: a file added to the index since is gone from disk, and so is
  an untracked folder.
- [ ] Before the first commit, it runs `git rm -r --force --quiet -- .`, and the staged files are deleted from disk.
- [ ] During a merge, the menu of the uncommitted changes row has no **Discard all changes…**, and neither do the
  details.
- [ ] With only `a.txt` changed and nothing staged, **Discard all changes** of the Changes view asks with
  `git restore -- a.txt` and the note *Unstaged changes to tracked files will be lost.*
- [ ] **Cancel** leaves every file as it was.

### `changes.open` — Open a file or its diff

Opens a changed file in its editor, or its diff, from the Changes view.

- **Kind**: View
- **Function**: `Controller.openFile(file)` and `Controller.openDiff({ kind: 'change' }, file)` in
  `src/extension/controller.ts`, chosen by the `openChange` message.
- **Entry points**: a click on a file of the Changes view; the buttons **Open File** and **Open Changes** on its row.
- **Offered when**: **Open File** is on every file but a deleted one; **Open Changes** on every changed file.
- **Reads**: whether the file has changes not staged, to choose the two sides of the diff.
- **Plan**: the diff opens in VS Code's diff editor as the list of changes of VS Code opens it: the index against the
  file on disk while some of it is not staged, titled `a.txt (Working Tree)`; `HEAD` against the index once all of it
  is, titled `a.txt (Index)`; an added file against nothing. When VS Code's Git extension has the repository open, the
  two sides are its own `git:` documents, so that **Stage Selected Ranges**, **Unstage Selected Ranges** and **Revert
  Selected Ranges** work in them; otherwise Gitardo serves the same versions.
- **Settings**: `gitardo.changes.openOnClick`: `file` (the default) opens the file, `diff` opens its diff. It changes
  only the click in this view, not the buttons and not the list of changes of VS Code.
- **Implemented in**: `WebviewMessage` `openChange`, `openFile` and `openDiff` in `src/shared/protocol.ts`; their cases
  in `Controller.onMessage`, and `openFile`, `openDiff` and `sides` (case `change`) in `src/extension/controller.ts`;
  `onOpen` and the row buttons in `src/webview/ChangesApp.tsx`; `gitardo.changes.openOnClick` in `package.json`.
- **Tested by**: `test/e2e/suite.ts`, step "serves file contents at a commit and in the index, and opens diffs".

Acceptance:

- [ ] With `gitardo.changes.openOnClick` left at `file`, a click on `a.txt` opens `a.txt` from disk in a text editor.
- [ ] With it set to `diff`, the same click opens a diff editor; **Open File** still opens the file.
- [ ] A click on a deleted file opens its diff, whatever the setting says.
- [ ] **Open Changes** on `a.txt` with changes not staged opens the index against the file on disk; on `a.txt` wholly
  staged it opens `HEAD` against the index.
- [ ] A click on a file in conflict that has its base and both sides opens it in the merge editor, whatever the
  setting says; a click on a file only an editor has changed opens that editor.

### `changes.open-all` — Open all changes in one editor

Opens the diffs of every changed file one under the other, in VS Code's editor of several diffs.

- **Kind**: View
- **Function**: `Controller.openChanges(title, source, files)` in `src/extension/controller.ts`, which runs
  `vscode.changes`.
- **Entry points**: **Open all changes in one editor** on the header of **Changes** in the Changes view.
- **Offered when**: while git sees a change; files that only an editor has changed are left out of it.
- **Plan**: one editor titled **Changes**, each file with the same two sides **Open Changes** gives it.
- **Implemented in**: `WebviewMessage` `openChanges` in `src/shared/protocol.ts`; its case in `Controller.onMessage`
  and `openChanges` in `src/extension/controller.ts`; the header button in `src/webview/ChangesApp.tsx`.
- **Tested by**: *none* for the Changes view; `test/e2e/suite.ts`, step "opens all the changes between two commits in
  one editor", covers the same function for a comparison.

Acceptance:

- [ ] With `a.txt` and `b.txt` changed, the button opens one editor titled **Changes** with the two diffs.
- [ ] A file wholly staged shows `HEAD` against the index there, and a file with changes not staged the index against
  the file on disk.

### `changes.layout` — List the files or show them as a tree

Shows the changed files one after the other, each with its directory beside its name, or under their directories.

- **Kind**: View
- **Function**: `switchLayout` in `ChangesApp()` in `src/webview/ChangesApp.tsx`, with `FileTree` in
  `src/webview/FileTree.tsx` and `fileTree` in `src/shared/tree.ts`.
- **Entry points**: **View as tree** and **View as list** on the header of **Changes**.
- **Implemented in**: `layout` kept as `changesFiles` through `loadUiState` and `saveUiState` in
  `src/webview/vscode.ts`; the header button in `src/webview/ChangesApp.tsx`.
- **Tested by**: *none*.

Acceptance:

- [ ] The view starts as a list; **View as tree** shows `src/a.ts` under a directory row `src`, and the button becomes
  **View as list**.
- [ ] A click on a directory row closes it and hides its files; another click opens it.
- [ ] The choice is kept when the view is hidden and shown again.
- [ ] The directory rows have no box: staging stays per file and for all.

### `changes.unsaved` — Files with edits not saved

Shows which files have edits not saved in an editor, so that what is committed is not taken for what is on screen.

- **Kind**: View
- **Function**: `Controller.checkUnsaved()` and `Controller.unsavedIn(repo, state, files)` in
  `src/extension/controller.ts`, which tell the Changes views with an `unsaved` message.
- **Entry points**: the Changes view.
- **Offered when**: an editor of VS Code has a file of the repository shown with edits not saved.
- **Reads**: the text documents of VS Code that are dirty; the submodules and the worktrees inside the repository, whose
  files are of other repositories; and `git check-ignore -z --stdin`, without taking locks, to leave out what git
  ignores. It is worked out again at each edit, at each editor closed or saved and at each read of the repository, and
  sent only when the list changed.
- **Implemented in**: `HostMessage` `unsaved` in `src/shared/protocol.ts`; `checkUnsaved`, `unsavedIn` and the
  listeners of `onDidChangeTextDocument`, `onDidCloseTextDocument` and `onDidSaveTextDocument` in
  `src/extension/controller.ts`; `Repo.ignored(paths)` in `src/git/repo.ts`; `listed` and the rows with `staged: null`
  in `src/webview/ChangesApp.tsx`; the dot of `FileRow` in `src/webview/FileRow.tsx`.
- **Tested by**: `test/e2e/suite.ts`, step "tells the Changes view which files are not saved in an editor, and saves
  them"; `test/integration.test.ts`, "says which files git ignores, never a tracked one".

Acceptance:

- [ ] A changed file with edits not saved has a dot after its name, titled *Not saved*, and a **Save** button on its row.
- [ ] A file git sees no change in, edited in an editor and not saved, is listed among the others in the order of the
  paths, with its box greyed out (*Not saved: save it to stage it*) and the buttons **Compare with the saved file** and
  **Save**; a click on it opens its editor.
- [ ] A file git ignores, a file of a submodule and a file of a worktree inside the repository are not listed, edited
  or not.
- [ ] Reverting the editor takes the file out of the list, or its dot away, without a read of the repository.
- [ ] The view does not say *No changes* while a file has edits not saved.

### `changes.save` — Save a file from the Changes view

Saves the edits an editor has of a file, or of all of them, so that git sees them.

- **Kind**: View
- **Function**: `Controller.save(paths)` in `src/extension/controller.ts`, which saves the dirty text documents of
  those files with VS Code.
- **Entry points**: **Save** on the row of a file with edits not saved; **Save all** on the header of **Changes**.
- **Offered when**: **Save** while that file has edits not saved; **Save all** while some file has.
- **Plan**: VS Code saves the editors, as `Ctrl`+`S` would; no git command runs, and nothing is written to the log.
- **Implemented in**: `WebviewMessage` `save` in `src/shared/protocol.ts`; its case in `Controller.onMessage` and
  `save` in `src/extension/controller.ts`; the buttons in `src/webview/ChangesApp.tsx`.
- **Tested by**: `test/e2e/suite.ts`, step "tells the Changes view which files are not saved in an editor, and saves
  them".

Acceptance:

- [ ] **Save** on `first.txt` writes the edits of its editor to disk; the dot goes, and `first.txt` is listed as
  changed once the repository is read again.
- [ ] **Save all** saves every file of the repository listed with a dot, and no other editor.

### `changes.compare-with-saved` — Compare with the saved file

Shows what saving a file would change, for a file that git sees no change in yet.

- **Kind**: View
- **Function**: the `compareWithSaved` case of `Controller.onMessage` in `src/extension/controller.ts`, which runs VS
  Code's `workbench.files.action.compareWithSaved` on the file.
- **Entry points**: **Compare with the saved file** on the row of a file only an editor has changed.
- **Offered when**: the file has edits not saved and git sees no change in it.
- **Implemented in**: `WebviewMessage` `compareWithSaved` in `src/shared/protocol.ts`; its case in
  `src/extension/controller.ts`; the button in `src/webview/ChangesApp.tsx`.
- **Tested by**: *none*.

Acceptance:

- [ ] The button opens VS Code's comparison of the file on disk with what its editor has, not saved.

### `commit.box` — The commit box

The message of the next commit, the button that commits it, the choice to amend, and who the commit will be made as.

- **Kind**: View
- **Function**: `CommitBox({ state, busy })` in `src/webview/ChangesApp.tsx`.
- **Entry points**: the top of the Changes view, under the banners.
- **Plan**: a box of three lines, with the placeholder `Message (Ctrl+Enter to commit on main)`, or `on detached
  HEAD`; the button **Commit**, which reads **Amend** while **Amend last commit** is ticked; the sparkle button that
  drafts a message; the box **Amend last commit**; and under them the identity of the repository.
- **Settings**: `gitardo.identities`, through the switch of identity.
- **Uses**: `commit.create`, `commit.amend`, `commit.draft-message`, `commit.start-message`; switching identity, which
  runs `git config user.name` and `git config user.email` in the repository.
- **Implemented in**: `CommitBox` in `src/webview/ChangesApp.tsx`, which keeps the message as `message` through
  `loadUiState` and `saveUiState` in `src/webview/vscode.ts`; `UiCommand` `switchIdentity` and
  `Controller.switchIdentity` in `src/extension/controller.ts`.
- **Tested by**: `test/e2e/ui.mjs`, step "stages a file by ticking its box, in the one list of changes, and commits it
  with a typed message".

Acceptance:

- [ ] On `main` the box reads `Message (Ctrl+Enter to commit on main)` while empty; on a detached `HEAD`,
  `Message (Ctrl+Enter to commit on detached HEAD)`.
- [ ] **Commit** is disabled while the message is empty or only spaces, while nothing is changed, and while an
  operation runs; with **Amend last commit** ticked it is enabled with an empty message.
- [ ] `Ctrl`+`Enter`, or `Cmd`+`Enter`, in the box does what the button does, and nothing while it is disabled.
- [ ] A message typed and left is still there when the view is hidden and shown again.
- [ ] Under the button is `Ada Lovelace <ada@work.example>`, the identity commits are made with here, or *No identity
  configured*; a click on it switches identity.
- [ ] Once a commit or an amend succeeds, the box is empty and **Amend last commit** unticked; when it fails, the
  message stays.

### `commit.create` — Commit

Makes a commit of what is staged with the message typed, or of everything once the user agrees.

- **Kind**: Operation
- **Function**: `commitOp(message, amend, stageAll)` in `src/git/ops.ts`, with `amend` false.
- **Entry points**: **Commit** in the commit box of the Changes view; `Ctrl`+`Enter` in its message.
- **Offered when**: the message is not empty, git sees a change, and no operation runs (see `commit.box`).
- **Asks**: when nothing is staged and something is not, a modal warning *There are no staged changes. Stage all
  changes and commit them?* with **Stage All and Commit**; closing it commits nothing.
- **Plan**: `> git commit -m "<message>"`, the message trimmed and given whole, its body included, so that no editor
  opens. After **Stage All and Commit**, `> git add --all` first, in the same operation.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: right after, **Undo** asks *Undo "commit: <subject>"?* with `git reset --soft <commit before>` and *main
  goes back to <hash>. The changes of the undone commit stay staged.*: the commit goes and its changes are staged again,
  from the reflog of the branch. The first commit of a repository cannot be undone: *There is nothing to undo.*
- **After**: the box is emptied (`committed`), the new commit is the first of the graph after the uncommitted changes,
  if any are left.
- **Refuses**: *There are no changes to commit.* when nothing is staged or changed; *Enter a commit message.* with an
  empty message. When git refuses, such as a hook that fails, *Commit failed.* with the line of git that says why, and
  **Show Log**.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `changes.stage-all`, `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` `{ kind: 'commit'; message; amend }` in `src/shared/protocol.ts`; `case 'commit'` of
  `resolveAction` in `src/extension/actions.ts`; `commitOp` in `src/git/ops.ts`; `Controller.action`, which posts
  `committed` after it, in `src/extension/controller.ts`; `Repo.undoInfo` in `src/git/repo.ts`, which takes a
  `commit:` entry back with a soft reset.
- **Tested by**: `test/git.test.ts`, "commits, amends, and stages everything first when asked";
  `test/integration.test.ts`, "stages, unstages and commits in a repository with no commits yet" and "undoes commits,
  checkouts, resets and rebases"; `test/awkward.test.ts`, "keeps messages whole: long ones, empty ones, and ones with
  characters of every kind"; `test/e2e/suite.ts`, steps "stages and commits without asking when the preview is off" and
  "undoes the last commit, and restores a deleted branch"; `test/e2e/ui.mjs`, step "stages a file by ticking its box,
  in the one list of changes, and commits it with a typed message".

Acceptance:

- [ ] With `b.txt` staged, typing `Commit from the UI` and pressing **Commit** runs `git commit -m "Commit from the
  UI"` through `Controller.run`; the log has `> git commit -m "Commit from the UI"`, and `git log -1 --format=%s`
  prints `Commit from the UI`.
- [ ] What is not staged stays out of the commit and stays listed.
- [ ] Nothing is asked with `gitardo.commandPreview` set to `off`; with `all`, *Commit?* shows the command first.
- [ ] With nothing staged and `a.txt` changed, **Commit** asks *There are no staged changes. Stage all changes and
  commit them?*; **Stage All and Commit** runs `git add --all` and then `git commit -m "<message>"`, one operation.
- [ ] A message with a subject, a blank line and a body of several lines is committed exactly as typed, trimmed at its
  ends.
- [ ] Right after, **Undo** asks first and then runs `git reset --soft` to the commit before: the commit is gone and its
  changes are staged.

### `commit.amend` — Amend the last commit

Replaces the last commit with one that has what is staged too, and the new message if one is typed.

- **Kind**: Operation
- **Function**: `commitOp(message, amend, stageAll)` in `src/git/ops.ts`, with `amend` true and `stageAll` false.
- **Entry points**: ticking **Amend last commit** in the commit box of the Changes view, then **Amend** or
  `Ctrl`+`Enter`. The box says under the pointer *Replace the last commit instead of adding a new one. Leave the message
  empty to keep it.*
- **Offered when**: the repository has a commit: the box is disabled before the first one.
- **Plan**: with a message, `> git commit --amend -m "<message>"`; with the message empty,
  `> git commit --amend --no-edit`, which keeps the message the commit has. What is staged goes into the commit; what is
  not staged stays out, and staging everything is not offered.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`). Amend is not in the README's list of what is
  always asked.
- **Undo**: right after, **Undo** asks *Undo "commit (amend): <subject>"?* with `git reset --soft` to the commit as it
  was before the amend: its old message is back, and what the amend added is staged.
- **After**: the box is emptied and **Amend last commit** unticked.
- **Refuses**: when git refuses, such as during a merge, *Amend last commit failed.* with the line of git that says
  why, and **Show Log**.
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `preview.ask`, `log.write`, `undo.last`; `rewrite.reword` changes the message of the last commit from the
  details, with `git commit --amend --only`, which leaves out what is staged.
- **Implemented in**: `Action` `{ kind: 'commit'; message; amend }` in `src/shared/protocol.ts`; `case 'commit'` of
  `resolveAction` in `src/extension/actions.ts`, which asks nothing for an amend; `commitOp` in `src/git/ops.ts`; the
  `amend` box of `CommitBox` in `src/webview/ChangesApp.tsx`; `Repo.undoInfo` in `src/git/repo.ts`.
- **Tested by**: `test/git.test.ts`, "commits, amends, and stages everything first when asked".

Acceptance:

- [ ] With **Amend last commit** ticked, `a.txt` staged and the message empty, **Amend** runs
  `git commit --amend --no-edit`: the last commit has the change of `a.txt` and keeps its message and its parent.
- [ ] With the message `Reword`, it runs `git commit --amend -m Reword`, and the last commit is titled `Reword`.
- [ ] With nothing staged and the message empty, **Amend** is enabled and runs `git commit --amend --no-edit`.
- [ ] When the last commit is on its upstream already, nothing is asked and nothing says the branch will need a force
  push.
- [ ] Nothing is asked with `gitardo.commandPreview` set to `off`.
- [ ] Right after, **Undo** asks first and then puts back the commit as it was before the amend.

### `commit.start-message` — The message the box starts from

Gives the commit box a message to start from when Gitardo knows what the next commit is: the commits a squash merge
brought, or the message of a commit being split.

- **Kind**: View
- **Function**: `Controller.reportSquashed(message)` and `Controller.reportSplit(repo, hash, wasHead)` in
  `src/extension/controller.ts`, which post a `commitMessage` message; `CommitBox` takes it in
  `src/webview/ChangesApp.tsx`.
- **Entry points**: a squash merge, from `merge.into-current`; **Split this commit…**, from `rewrite.split`.
- **Offered when**: the squash merge ended without conflicts and git left `.git/SQUASH_MSG`; or the split succeeded,
  the last commit or an older one.
- **Reads**: `.git/SQUASH_MSG` (`Repo.squashMessage()`), or the message of the commit split with
  `git log -1 --format=%B <hash>` (`Repo.commitMessage(hash)`), without taking locks.
- **After**: after a squash merge, the message *Gitardo: the changes are staged, and nothing is committed yet. Commit
  them to make them one commit on main.* with **Show Changes** and **Undo**. After a split of the last commit, *The
  changes of "<subject>" are in the working tree, unstaged: commit them in as many commits as you like.* with **Undo**.
- **Uses**: `merge.into-current`, `rewrite.split`, `commit.box`.
- **Implemented in**: `HostMessage` `commitMessage` in `src/shared/protocol.ts`; `reportSquashed`, `reportSplit` and
  the lines of `Controller.action` that call them in `src/extension/controller.ts`; `Repo.squashMessage` and
  `Repo.commitMessage` in `src/git/repo.ts`; the `commitMessage` case in `CommitBox` in `src/webview/ChangesApp.tsx`.
- **Tested by**: `test/e2e/suite.ts`, steps "merges in the way chosen, refuses a fast-forward only that cannot be one,
  and undoes a squash, which moves no branch" and "splits the last commit and an older one, commits what is left on
  Continue, and undoes each split whole".

Acceptance:

- [ ] After a squash merge of `feature` without conflicts, the empty commit box holds what `.git/SQUASH_MSG` says,
  trimmed, which names the commits it brought.
- [ ] After **Split this commit…** on `Add the login page`, the empty box holds its whole message.
- [ ] When the box already has text other than spaces, that text stays and the message is not used.

### `commit.draft-message` — Draft the message with the language model

Asks a language model available in VS Code to write the commit message from the changes, for the user to edit.

- **Kind**: Read
- **Function**: `draftCommitMessage(repo, state, token)` in `src/extension/ai.ts`, with `buildPrompt(diff, untracked,
  recentSubjects)` and `cleanMessage(text)` in `src/shared/commitPrompt.ts`; `Controller.draftMessage()` in
  `src/extension/controller.ts`.
- **Entry points**: the sparkle button of the commit box, *Draft a message from the changes with the language model
  set up in VS Code*.
- **Offered when**: git sees a change, and no draft is being made: the button spins and is disabled meanwhile.
- **Reads**: the first model `vscode.lm.selectChatModels()` gives; the staged changes with
  `git diff --cached --no-color --no-ext-diff`, or, when nothing is staged, the changes not staged with
  `git diff --no-color --no-ext-diff` and the names of the untracked files; the subjects of up to the last 10 commits of
  the graph. All of it without taking locks.
- **Plan**: one request to the model, with the justification *Gitardo sends your uncommitted changes to the model to
  draft a commit message.*: instructions for a summary line of at most 72 characters in the imperative and an optional
  body, the subjects as the style to follow, the untracked files under *New files that are not in the diff:*, and the
  diff cut to 20,000 characters, under *Diff (truncated):* when it was cut. A code fence around the answer is taken
  away. The status bar shows *Gitardo: drafting commit message* while it runs.
- **After**: the draft takes the place of what the box had.
- **Refuses**: *Gitardo: There are no changes to describe.*; *Gitardo: No language model is available in VS Code.
  Install an extension that provides one, such as GitHub Copilot.*
- **Uses**: `commit.box`.
- **Implemented in**: `UiCommand` `draftMessage` and `HostMessage` `drafting` in `src/shared/protocol.ts`;
  `Controller.command` and `Controller.draftMessage` in `src/extension/controller.ts`; `draftCommitMessage` in
  `src/extension/ai.ts`; `buildPrompt`, `cleanMessage` and `MAX_DIFF_CHARS` in `src/shared/commitPrompt.ts`;
  `Repo.diffText(staged)` in `src/git/repo.ts`; the sparkle button and the `drafting` case of `CommitBox` in
  `src/webview/ChangesApp.tsx`.
- **Tested by**: `test/git.test.ts`, "includes the diff, the new files and the style to follow", "cuts a diff that is
  too long to send, and says so" and "unwraps an answer that came in a code fence". The request itself needs a
  signed-in account, which no test has.

Acceptance:

- [ ] Nothing is sent to a model until the sparkle button is pressed.
- [ ] With `a.txt` staged and `b.txt` not, the diff sent is that of `git diff --cached`, and `b.txt` is not in it.
- [ ] With nothing staged and `notes.md` untracked, the diff sent is that of `git diff`, and the request lists
  `- notes.md` under *New files that are not in the diff:*.
- [ ] A diff of 50,000 characters is sent cut to its first 20,000, under *Diff (truncated):*.
- [ ] An answer `` ```text\nAdd parser\n\nBody.\n``` `` puts `Add parser`, a blank line and `Body.` in the box.
- [ ] With no model in VS Code, the error above is shown and the box keeps what it had.
- [ ] Nothing in the repository changes, and nothing is written to the log of operations.
