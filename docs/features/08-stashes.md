# Stashes

Setting changes aside with `git stash` and bringing them back: stashing every change, only what is staged, or some
files; applying, popping and dropping a stash, with what it would stop on said before it runs; making a branch of one;
and where the stashes are shown: the Stashes view of the side bar, the graph and its references, the details. Pulling
with the changes set aside, `--autostash`, is `sync.pull`.

Every operation here names a stash by its place, `stash@{<n>}`, as git lists it. The log shows it in quotes,
`"stash@{0}"`, as it shows every argument with characters a shell would read.

### `stash.push` — Stash all changes

Sets every change aside, staged or not and untracked files included, and leaves the working tree as the last commit has
it.

- **Kind**: Operation
- **Function**: `stashPushOp(message)` in `src/git/ops.ts`, with no `paths` and no `staged`.
- **Entry points**: **Stash changes…** in the context menu of the uncommitted changes in the graph; the **Stash** button
  of the toolbar of the graph, whose tooltip is **Stash: git stash push --include-untracked**; **Stash all changes…**
  above the files of the uncommitted changes in the details, while no file is ticked; **Stash all changes…** at the top
  of the Changes view; the **Stash Changes** button on the title of the Stashes view; **Gitardo: Stash Changes** in the
  Command Palette; **Stash All Changes…** on the **Changes** group of the list of changes of VS Code, in the Source
  Control view, as a button and in its context menu.
- **Offered when**: the menu of the uncommitted changes and the details offer it once there is a commit; the toolbar
  button is disabled while an operation runs, while nothing is uncommitted and before the first commit; the Changes
  view, the Stashes view and the command offer it always. The list of changes of VS Code has it while
  `gitardo.scm.stashButtons` is on, for a repository of git.
- **Asks**: an input box titled **Stash changes**, with **Optional message. Untracked files are stashed too.**, empty
  at first. The message is trimmed; empty means none. Escape cancels, and nothing runs.
- **Plan**: `> git stash push --include-untracked`, or with a message
  `> git stash push --include-untracked -m "<message>"`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`). The changes are kept in the stash.
- **Undo**: Gitardo remembers nothing for it, and git writes nothing to the reflog of the branch for it: **Undo** right
  after it offers to take back the operation before it, such as the last commit. The changes come back with
  `stash.pop`.
- **After**: the new stash is `stash@{0}`, which git names `On <branch>: <message>`, or `WIP on <branch>: <short hash>
  <subject>` without a message; the others move down one place. Ignored files stay where they are. From the list of
  changes of VS Code, in a workspace with several repositories, it runs in the repository of the first file of the
  group, which the views then show (`Controller.stashFromScm`).
- **Refuses**: git refuses before the first commit, which Gitardo says as **Stash changes failed. You do not have the
  initial commit yet**. With nothing to set aside git writes *No local changes to save* to the log and changes nothing.
- **Settings**: `gitardo.scm.stashButtons` (the entries in the list of changes of VS Code), `gitardo.commandPreview`.
- **Uses**: `preview.ask`, `log.write`, `stash.view`.
- **Implemented in**: the `stashPush` case of `Action` (`src/shared/protocol.ts`); the `stashPush` case of
  `resolveAction` (`src/extension/actions.ts`); `stashPushOp` (`src/git/ops.ts`); `changesMenu` in
  `src/webview/menus.ts`; the **Stash** button in `src/webview/Toolbar.tsx`; `changesActions` in
  `src/webview/Detail.tsx`; the header of `src/webview/ChangesApp.tsx`; the commands `gitardo.stash` and
  `gitardo.scmStashAll` in `src/extension/extension.ts`, the second through `Controller.stashFromScm(uris, 'all')`;
  `view/title` and `scm/resourceGroup/context` in `package.json`.
- **Tested by**: `test/git.test.ts` "builds stash commands and rejects anything that is not a stash";
  `test/integration.test.ts` "stashes tracked and untracked changes and pops them back"; `test/e2e/suite.ts` "the
  buttons added to the list of changes of VS Code stash a file, what is staged or everything, in the repository of the
  file"; `test/menus.test.ts` "stashes, stages, unstages and discards them all, as far as there is something to" and
  "leaves out stashing before the first commit, staging over conflicts, and discarding during a merge".
- **Differs**: `Repo.undoInfo` (`src/git/repo.ts` line 1053) reads the reflog of the branch, which `git stash push`
  does not write, and `Controller.action` (`src/extension/controller.ts` line 1689) records nothing for a stash, so
  **Undo** right after it offers to take back the operation before it.

Acceptance:

- [ ] With `a.txt` changed and `new.txt` untracked on `main`, stashing with the message `work in progress` runs
  `git stash push --include-untracked -m "work in progress"`, leaves nothing uncommitted, and lists `stash@{0}` as
  `On main: work in progress`.
- [ ] An empty message runs `git stash push --include-untracked`, with no `-m`.
- [ ] Escape in the input box runs nothing.
- [ ] It runs as a `GitOp` through `Controller.run`: `> git stash push --include-untracked` is written to the
  **Gitardo** output channel, and the command is shown first only when `gitardo.commandPreview` is `all`.
- [ ] The command shown is the command that runs, with no flag added after.
- [ ] With `gitardo.commandPreview` at `off` it does not ask: it loses no work, and the README's list of what asks does
  not name it.
- [ ] Before the first commit the menu of the uncommitted changes has no **Stash changes…** and the **Stash** button of
  the toolbar is disabled.
- [ ] **Stash All Changes…** on the **Changes** group of the list of changes of VS Code runs
  `git stash push --include-untracked`, untracked files included; with `gitardo.scm.stashButtons` off it is not there.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `stash.push-staged` — Stash only the staged changes

Sets aside what is staged, and leaves what is not staged, untracked files included, where it is.

- **Kind**: Operation
- **Function**: `stashPushOp(message, { staged: true })` in `src/git/ops.ts`.
- **Entry points**: **Stash staged changes…** in the context menu of the uncommitted changes in the graph; **Stash only
  the staged changes…** at the top of the Changes view; **Stash Only the Staged Changes…** on the **Staged Changes**
  group of the list of changes of VS Code, as a button and in its context menu.
- **Offered when**: something is staged, and in the menu of the graph once there is a commit; in the list of changes of
  VS Code while `gitardo.scm.stashButtons` is on.
- **Asks**: an input box titled **Stash staged changes**, with **Optional message. Only what is staged is stashed; the
  rest stays where it is.** Escape cancels.
- **Plan**: `> git stash push --staged`, or `> git stash push --staged -m "<message>"`. Untracked files are never
  staged, so `--include-untracked` is not there.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: as for `stash.push`: Gitardo remembers nothing, and **Undo** right after it offers to take back the
  operation before it. The changes come back with `stash.pop`.
- **After**: the index is as `HEAD` has it, and the staged files are back to the last commit in the working tree; the
  changes that were not staged stay.
- **Refuses**: a git older than 2.35 does not know `--staged` and refuses.
- **Settings**: `gitardo.scm.stashButtons`, `gitardo.commandPreview`.
- **Uses**: `preview.ask`, `log.write`.
- **Implemented in**: the `stashPush` case of `Action` with `staged`; the `stashPush` case of `resolveAction`;
  `stashPushOp`; `changesMenu` in `src/webview/menus.ts`; the header of `src/webview/ChangesApp.tsx`; the command
  `gitardo.scmStashStaged` in `src/extension/extension.ts` through `Controller.stashFromScm(uris, 'staged')`;
  `scm/resourceGroup/context` in `package.json`.
- **Tested by**: `test/integration.test.ts` "stashes only some files, or only what is staged"; `test/e2e/suite.ts`
  "the buttons added to the list of changes of VS Code stash a file, what is staged or everything, in the repository of
  the file"; `test/menus.test.ts` "stashes, stages, unstages and discards them all, as far as there is something to".
- **Differs**: as for `stash.push`: `src/git/repo.ts` line 1053 and `src/extension/controller.ts` line 1689.

Acceptance:

- [ ] With `a.txt` staged and `b.txt` changed and not staged, it runs `git stash push --staged`, leaves `a.txt` as the
  last commit has it, and leaves `b.txt` and the untracked files changed.
- [ ] The message `staged` runs `git stash push --staged -m staged`.
- [ ] With nothing staged, the menu of the uncommitted changes and the Changes view do not offer it.
- [ ] It runs as a `GitOp` through `Controller.run`, logged, and is shown first only when `gitardo.commandPreview` is
  `all`; the command shown is the one that runs.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `stash.push-files` — Stash some files

Sets aside every change of one file, or of the files ticked, staged or not, and leaves the other files alone.

- **Kind**: Operation
- **Function**: `stashPushOp(message, { paths })` in `src/git/ops.ts`.
- **Entry points**: **Stash this file…** on each file of the Changes view; **Stash this file…** on each file of the
  uncommitted changes in the details; **Stash 1 selected file…** or **Stash <n> selected files…** above the files of
  the uncommitted changes in the details, while files are ticked; **Stash This File…** on each file of the **Staged
  Changes**, **Changes** and **Untracked Changes** groups of the list of changes of VS Code, as a button and in the
  context menu, for every file selected there.
- **Offered when**: in the details, once there is a commit; in the Changes view always; in the list of changes of VS
  Code while `gitardo.scm.stashButtons` is on.
- **Asks**: an input box titled **Stash <file>** for one path, **Stash <n> files** for more, with **Optional message.
  Every change to the file is stashed, staged or not.** Escape cancels.
- **Plan**: `> git stash push --include-untracked -- <file>…`, or with a message
  `> git stash push --include-untracked -m "<message>" -- <file>…`. A renamed file is passed with its new and its old
  name, so that git does not leave half of the rename behind; its title then counts two files.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: as for `stash.push`: Gitardo remembers nothing, and **Undo** right after it offers to take back the
  operation before it. The changes come back with `stash.pop`.
- **After**: the files named are as the last commit has them, an untracked one is gone; the other files are untouched.
  From the list of changes of VS Code, files outside the repository are left out, and with none left nothing runs.
- **Settings**: `gitardo.scm.stashButtons`, `gitardo.commandPreview`.
- **Uses**: `preview.ask`, `log.write`.
- **Implemented in**: the `stashPush` case of `Action` with `paths`; the `stashPush` case of `resolveAction`;
  `stashPushOp` and `pathspec` (`src/git/ops.ts`); `fileActions` and `changesActions` in `src/webview/Detail.tsx`; the
  file buttons of `src/webview/ChangesApp.tsx`; the command `gitardo.scmStashFile` in `src/extension/extension.ts`
  through `Controller.stashFromScm(uris, 'files')`; `scm/resourceState/context` in `package.json`.
- **Tested by**: `test/integration.test.ts` "stashes only some files, or only what is staged"; `test/awkward.test.ts`
  "stages, unstages, cuts into hunks, discards and stashes files whatever their names" and "takes a name with brackets
  as that file, not as a pattern that other files match too"; `test/e2e/suite.ts` "stashes a single file and leaves
  the rest alone" and "the buttons added to the list of changes of VS Code stash a file, what is staged or everything,
  in the repository of the file"; `test/e2e/ui.mjs` "the changes that are not committed are stashed or discarded from
  their row in the graph, a file at a time, or the ones ticked in the details".
- **Differs**: as for `stash.push`: `src/git/repo.ts` line 1053 and `src/extension/controller.ts` line 1689.

Acceptance:

- [ ] **Stash This File…** on `a.txt` in the list of changes of VS Code, with the message `from the list`, runs
  `git stash push --include-untracked -m "from the list" -- a.txt`, and `keep.txt`, untracked, stays.
- [ ] Stashing `a.txt` and the untracked `new.txt` together leaves `b.txt` changed and `new.txt` gone, and popping the
  stash brings `new.txt` back.
- [ ] A file whose name is also a pattern is passed as `:(literal)<name>`: stashing `pages/[id].txt` leaves
  `pages/i.txt` changed.
- [ ] A file of another repository of the workspace is stashed in that repository, which the views then show.
- [ ] It runs as a `GitOp` through `Controller.run`, logged, and is shown first only when `gitardo.commandPreview` is
  `all`; the command shown is the one that runs.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `stash.preview` — What applying a stash would stop on

Says, in the question before a stash is applied or popped, what git would stop on, so that nothing is tried in vain.

- **Kind**: Read
- **Function**: `withStashPreview(op, ref, state, repo)` in `src/extension/actions.ts`, with `Repo.stashFiles(ref)` and
  `Repo.conflictsWith(theirs, base)` in `src/git/repo.ts`.
- **Reads**: the files the stash changes, `git diff --name-only -z stash@{<n>}^1 stash@{<n>}`; the untracked files it
  took, `git ls-tree -r -z --name-only stash@{<n>}^3`, none when it has no third parent; and the conflicts of its
  changes with what is committed, from the commit it was made on,
  `git merge-tree --write-tree --no-messages --name-only -z --merge-base=stash@{<n>}^1 HEAD stash@{<n>}`. Whether an
  untracked file is there again is asked of the disk. Every read runs with `GIT_OPTIONAL_LOCKS=0`.
- **Uses**: the uncommitted files of the state of the repository.
- **Implemented in**: `withStashPreview` (`src/extension/actions.ts`), called by the `stashApply` and `stashPop` cases
  of `resolveAction`; `Repo.stashFiles` and `Repo.conflictsWith` (`src/git/repo.ts`).
- **Tested by**: `test/e2e/suite.ts` "asks before applying a stash, saying what it would stop on, and before deleting a
  tag or a merged branch"; `test/integration.test.ts` "lists what a stash changes and took along, and knows which
  branches delete without forcing".

Rules of its result:

- Its result is the note of the question, after the note of the operation, in this order, each said only when it
  applies:
  1. Files the stash changes that have changes not committed, staged or not (untracked files do not count):
     **Git will refuse it and change nothing, as 1 file it changes has changes that are not committed: commit or stash
     them first.**, or **…as <n> files it changes have changes…**, followed by the files.
  2. Untracked files the stash took that are there again: **An untracked file it took along is there again: git leaves
     it as it is and keeps the stash.**, or **<n> untracked files it took along are there again: git leaves them as it
     is and keeps the stash.**, followed by the files.
  3. Files that would be in conflict: **This will stop with conflicts in 1 file:** or **…in <n> files:**, followed by
     the files.
  4. **No conflicts expected.** when the conflicts could be computed, there are none, and no file of rule 1 is there.
- A list shows at most 15 files, then **…and <n> more**.
- When git cannot compute the conflicts (older than 2.40), rules 3 and 4 say nothing.
- A name that is not `stash@{<n>}` is refused with *"<name>" is not a stash.*

Worked examples:

- A stash of `preview.txt`, after a commit that changed the same line → **This will stop with conflicts in 1
  file:** and `preview.txt`.
- The same stash with `preview.txt` changed and not committed → **Git will refuse it and change nothing, as 1 file it
  changes has changes that are not committed: commit or stash them first.**
- A stash that took `along.txt`, with `along.txt` there again → **An untracked file it took along is there again**,
  and `along.txt`.
- Nothing in the way → **No conflicts expected.**

### `stash.apply` — Apply a stash

Brings the changes of a stash back into the files and keeps the stash.

- **Kind**: Operation
- **Function**: `stashOp('apply', ref)` in `src/git/ops.ts`, with `withStashPreview` in `src/extension/actions.ts`.
- **Entry points**: **Apply** in the context menu of a stash: in the Stashes view, in the Stashes section of the
  references beside the graph, and on its row in the graph; the first button on the row of a stash in the Stashes view,
  whose tooltip is **Apply: bring the changes back and keep the stash**.
- **Offered when**: always; the buttons of the Stashes view are hidden while an operation runs.
- **Reads**: `stash.preview`.
- **Plan**: `> git stash apply "stash@{<n>}"`.
- **Danger**: Confirm, because it changes the files and can stop on conflicts: always asked as a question, whatever
  `gitardo.commandPreview` says, as **Apply stash@{<n>}?** with **Run**, the note **Its changes go into the files, and
  it stays in the list.**, then what `stash.preview` found, then the command.
- **Undo**: Gitardo remembers nothing, and git writes nothing to the reflog for it: **Undo** right after it offers to
  take back the operation before it. The stash is still there.
- **After**: the stash stays in the list. When git stops on conflicts the files have them marked, nothing is under way
  to continue, and Gitardo says **Apply stash@{<n>} stopped: <n> files have conflicts. Resolve them, then stage the
  files.**, with what can resolve them (`conflict.resolve-rules`, `conflict.resolve-ai`, `conflict.review`).
- **Refuses**: a name that is not `stash@{<n>}`: *"<name>" is not a stash.* Git refuses, changing nothing, when a file
  it changes has changes not committed. With an untracked file it took there again, git applies the rest, leaves that
  file and fails, which Gitardo says as **Apply stash@{<n>} failed. error: could not restore untracked files from
  stash**.
- **Uses**: `stash.preview`, `preview.ask`, `log.write`.
- **Implemented in**: the `stashApply` case of `Action`; the `stashApply` case of `resolveAction`; `stashOp`;
  `stashMenu` in `src/webview/menus.ts`; the row buttons of `src/webview/StashesApp.tsx`; `onRowMenu` in
  `src/webview/App.tsx`; the Stashes section of `src/webview/RefsTree.tsx`; `Controller.reportFailure`.
- **Tested by**: `test/git.test.ts` "builds stash commands and rejects anything that is not a stash";
  `test/e2e/suite.ts` "asks before applying a stash, saying what it would stop on, and before deleting a tag or a
  merged branch".
- **Differs**: as for `stash.push`: `Repo.undoInfo` (`src/git/repo.ts` line 1053) finds nothing of it in the reflog,
  and Gitardo records nothing for it (`src/extension/controller.ts` line 1689).

Acceptance:

- [ ] Applying `stash@{0}` asks **Apply stash@{0}?** with `git stash apply "stash@{0}"`, even with
  `gitardo.commandPreview` at `off`, and with nothing in the way the question says **No conflicts expected.** and **it
  stays in the list**.
- [ ] Run, it brings `preview.txt` back as the stash has it, and the stash is still listed.
- [ ] Cancelled, it changes nothing.
- [ ] It runs as a `GitOp` through `Controller.run`, and `> git stash apply "stash@{0}"` is written to the **Gitardo**
  output channel; the command shown is the one that runs.
- [ ] It asks first, which agrees with the README's list of what asks (applying a stash).
- [ ] `stash.preview`'s three cases each show in the question before anything runs.
- [ ] `--all` in place of a stash is refused, and nothing runs.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `stash.pop` — Pop a stash

Brings the changes of a stash back into the files and drops the stash, once they went in without conflicts.

- **Kind**: Operation
- **Function**: `stashOp('pop', ref)` in `src/git/ops.ts`, with `withStashPreview` in `src/extension/actions.ts`.
- **Entry points**: **Pop** in the context menu of a stash: in the Stashes view, in the Stashes section of the
  references beside the graph, and on its row in the graph; the second button on the row of a stash in the Stashes
  view, whose tooltip is **Pop: bring the changes back and drop the stash**.
- **Offered when**: always; the buttons of the Stashes view are hidden while an operation runs.
- **Reads**: `stash.preview`.
- **Plan**: `> git stash pop "stash@{<n>}"`.
- **Danger**: Confirm, because it changes the files and can stop on conflicts: always asked as a question, as **Pop
  stash@{<n>}?** with **Run**, the note **Its changes go into the files, and it is dropped once they are in without
  conflicts: with conflicts it stays.**, then what `stash.preview` found, then the command.
- **Undo**: Gitardo remembers nothing: **Undo** right after it offers to take back the operation before it, and a stash
  that git dropped is not made again.
- **After**: without conflicts the stash is gone from the list and the ones after it move up one place. With conflicts
  git keeps the stash and leaves the conflicts marked in the files, and Gitardo says **Pop stash@{<n>} stopped: <n>
  files have conflicts. Resolve them, then stage the files.** Once they are resolved, the stash is dropped by hand
  (`stash.drop`).
- **Refuses**: as `stash.apply`: a name that is not a stash; git refusing over changes not committed; an untracked
  file there again, where git applies the rest, keeps the stash and fails with **Pop stash@{<n>} failed. error: could
  not restore untracked files from stash**.
- **Uses**: `stash.preview`, `preview.ask`, `log.write`, `conflict.review`.
- **Implemented in**: the `stashPop` case of `Action`; the `stashPop` case of `resolveAction`; `stashOp`; `stashMenu`
  in `src/webview/menus.ts`; the row buttons of `src/webview/StashesApp.tsx`; `onRowMenu` in `src/webview/App.tsx`;
  `Controller.reportFailure`.
- **Tested by**: `test/git.test.ts` "builds stash commands and rejects anything that is not a stash";
  `test/integration.test.ts` "stashes tracked and untracked changes and pops them back"; `test/e2e/suite.ts` "stashes
  a single file and leaves the rest alone"; `test/resolve.test.ts` "does the same for a rebase, a cherry-pick and a
  stash that stopped".
- **Differs**: as for `stash.push`: `src/git/repo.ts` line 1053 and `src/extension/controller.ts` line 1689.

Acceptance:

- [ ] Popping `stash@{0}` asks **Pop stash@{0}?**, even with `gitardo.commandPreview` at `off`, and with nothing in
  the way the question says **No conflicts expected.**
- [ ] Run without conflicts, its changes are in the files and the list of stashes is one shorter.
- [ ] Run against a commit that changed the same line, git stops, the file has the conflict marked, and the stash is
  still listed: it is dropped only when its changes went in without conflicts.
- [ ] With an untracked file it took there again, the tracked changes go in, the file is left as it was, and the
  stash is still listed.
- [ ] It runs as a `GitOp` through `Controller.run`, and `> git stash pop "stash@{0}"` is written to the **Gitardo**
  output channel; the command shown is the one that runs.
- [ ] It asks first, which agrees with the README's list of what asks (popping a stash).
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `stash.drop` — Drop a stash

Deletes a stash, after a warning; **Undo** brings it back.

- **Kind**: Operation
- **Function**: `stashOp('drop', ref)` in `src/git/ops.ts`.
- **Entry points**: **Drop**, marked as an entry that loses work, in the context menu of a stash: in the Stashes view,
  in the Stashes section of the references beside the graph, and on its row in the graph; the **Drop…** button on the
  row of a stash in the Stashes view.
- **Offered when**: always; the buttons of the Stashes view are hidden while an operation runs.
- **Plan**: `> git stash drop "stash@{<n>}"`.
- **Danger**: Destructive, because the stashed changes are deleted: always asked with a warning, **Drop
  stash@{<n>}?**, with the note **The stashed changes will be deleted.** and the command.
- **Undo**: Gitardo remembers the hash and the message of the stash (`restoreStashOp`). **Undo** right after it asks
  **Undo: restore dropped stash?** and runs `> git stash store -m "<message>" <hash>`: the stash comes back as
  `stash@{0}`, with the message it had, whatever its place was. It is remembered while the last entry of the reflog of
  `HEAD` is the one there was at the drop, no other operation has succeeded through `Controller.action`, and the same
  repository is shown.
- **After**: the stashes after it move up one place.
- **Refuses**: a name that is not `stash@{<n>}`: *"<name>" is not a stash.*
- **Uses**: `undo.last`, `preview.ask`, `log.write`.
- **Implemented in**: the `stashDrop` case of `Action`; the `stashDrop` case of `resolveAction`; `stashOp`;
  `stashMenu` in `src/webview/menus.ts`; the row buttons of `src/webview/StashesApp.tsx`; in `Controller.action`, the
  `undoable` it records with `restoreStashOp` (`src/git/ops.ts`) once the drop succeeded, and the `undo` case that runs
  it.
- **Tested by**: `test/git.test.ts` "builds stash commands and rejects anything that is not a stash";
  `test/integration.test.ts` "restores a deleted branch and a dropped stash".

Acceptance:

- [ ] Dropping `stash@{0}` warns **Drop stash@{0}?** with **The stashed changes will be deleted.** and
  `git stash drop "stash@{0}"`, whatever `gitardo.commandPreview` says, which agrees with the README's list of what
  asks (dropping a stash).
- [ ] Cancelled, the stash is still listed.
- [ ] It runs as a `GitOp` through `Controller.run`, and the command is written to the **Gitardo** output channel; the
  command shown is the one that runs.
- [ ] Right after dropping the stash `On main: keep me`, **Undo** asks **Undo: restore dropped stash?** and runs
  `git stash store -m "On main: keep me" <hash>`, and the stash is listed again with the same hash and message.
- [ ] Dropping `stash@{1}` and undoing it lists it again as `stash@{0}`.
- [ ] After a commit made since the drop, **Undo** no longer restores the stash: it takes back the commit.
- [ ] `--all` in place of a stash is refused, and nothing runs.

### `stash.branch` — Create a branch from a stash

Makes a branch at the commit a stash was made on, checks it out, applies the stash there and drops it: the way to a
stash that no longer fits where the branch is now.

- **Kind**: Operation
- **Function**: `stashBranchOp(name, ref)` in `src/git/ops.ts`, with `updatingSubmodules(op, ref^, state, repo)` in
  `src/extension/actions.ts`.
- **Entry points**: **Create branch from stash…** in the context menu of a stash: in the Stashes view, in the Stashes
  section of the references beside the graph, and on its row in the graph.
- **Offered when**: always.
- **Asks**: an input box titled **Create branch from stash@{<n>}**, with the placeholder **feature/my-change**, which
  refuses as it is typed: **Enter a name** when empty; **That is not a valid Git ref name** for a name that starts with
  `-`, `.` or `/`, has a space or one of `~ ^ : ? * [ \`, `..` or `@{`, or ends with `/`, `.` or `.lock`; **A branch
  with that name already exists**. Escape cancels.
- **Plan**: `> git stash branch <name> "stash@{<n>}"`, and once it succeeded, when the repository has submodules or the
  commit the stash was made on has, `> git submodule update --init --recursive` (`submodule.update-on-checkout`).
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`), unless a submodule has commits on a detached
  `HEAD` that no branch would have then: Confirm, as `submodule.update-on-checkout` says.
- **Undo**: the reflog has the checkout: **Undo** right after it asks **Undo checkout: go back to <branch>?** and runs
  `git checkout <branch>`, then updates the submodules the same way. The new branch stays, the changes of the stash
  stay in the working tree when git can carry them over, and the stash, which git dropped, is not made again.
- **After**: the new branch is checked out at the commit the stash was made on, with the changes of the stash, and its
  staged changes staged again; git drops the stash once it applied without conflicts.
- **Refuses**: the names above, in the input box; a name starting with `-` that reaches `stashBranchOp` is refused with
  *"<name>" is not a valid name.*; a name that is not `stash@{<n>}`.
- **Settings**: `gitardo.submodules.updateOnCheckout`, `gitardo.commandPreview`.
- **Uses**: `submodule.update-on-checkout`, `undo.last`, `log.write`.
- **Implemented in**: the `stashBranch` case of `Action`; the `stashBranch` case of `resolveAction`, with `askName`;
  `stashBranchOp` (`src/git/ops.ts`); `stashMenu` in `src/webview/menus.ts`; the `then` run by `Controller.action`.
- **Tested by**: *none*.

Acceptance:

- [ ] Creating `rescue` from `stash@{0}` runs `git stash branch rescue "stash@{0}"`: `rescue` is checked out at the
  commit the stash was made on, has its changes, and the stash is gone from the list.
- [ ] `-x`, `a..b` and the name of an existing branch are refused in the input box, and nothing runs.
- [ ] It runs as a `GitOp` through `Controller.run`, logged, and is shown first only when `gitardo.commandPreview` is
  `all` or a submodule would leave commits behind; the commands shown, the submodule update included, are the ones
  that run.
- [ ] In a repository with submodules, `git submodule update --init --recursive` runs after it; with
  `gitardo.submodules.updateOnCheckout` off, it does not.
- [ ] Right after it, **Undo** offers to go back to the branch checked out before.

### `stash.list` — The stashes of a repository

Reads the stashes, newest first, each time the repository is read.

- **Kind**: Read
- **Function**: `Repo.state` in `src/git/repo.ts`, which runs
  `git stash list -z --format=%H%x1f%gd%x1f%gs%x1f%at`, and `parseStashes(out)` in `src/git/parse.ts`.
- **Reads**: for each stash its hash, its name `stash@{<n>}`, its message as git wrote it (`On main: …`, `WIP on
  main: …`) and when it was made, into `Stash` (`src/shared/protocol.ts`), and `RepoState.stashes`. Runs with
  `GIT_OPTIONAL_LOCKS=0`. The repository is read again when `logs/refs/stash` changes in the git directory, also from a
  terminal.
- **Implemented in**: `Repo.state` and `STASH_FORMAT`, `parseStashes` (`src/git/parse.ts`); the watcher of
  `Controller`.
- **Tested by**: `test/integration.test.ts` "stashes tracked and untracked changes and pops them back".

Acceptance:

- [ ] After `git stash push -m "work in progress"` on `main`, the list is one stash, `stash@{0}`, with the message
  `On main: work in progress`.
- [ ] A stash made in a terminal shows in the Stashes view without a refresh.
- [ ] While the graph does not show the stashes (`stash.graph`), none of the commits git makes for a stash is a row of
  it.

### `stash.contents` — What a stash holds

Reads the files a stash changed and, apart from them, the untracked files it took along, which git keeps in a commit of
their own.

- **Kind**: Read
- **Function**: `Repo.commitDetail(hash)` in `src/git/repo.ts`, given the hash of the stash.
- **Reads**: the files it changed against the commit it was made on, its first parent:
  `git diff --name-status -z -M <first parent> <stash>`. When it has a third parent with no parent of its own whose
  subject starts with `untracked files on`, the files of that commit:
  `git diff-tree --root --no-commit-id -r --name-status -z <third parent>`, into `CommitDetail.untracked` with the hash
  of that commit; none when it took nothing, although git still makes that commit. Runs with `GIT_OPTIONAL_LOCKS=0`.
- **Shown in**: the details of the graph and the tab of a commit, under a heading **1 untracked file** or **<n>
  untracked files**, after the files it changed; the Stashes view, after the files it changed, with no heading
  (`stash.view`). An untracked file opens as a diff against nothing.
- **Implemented in**: `Repo.commitDetail`; the untracked part of `src/webview/Detail.tsx`, `src/webview/CommitApp.tsx`
  and `src/webview/StashesApp.tsx`.
- **Tested by**: `test/integration.test.ts` "lists what a stash holds, untracked files included".

Acceptance:

- [ ] A stash of `a.txt` changed and `new.txt` untracked lists `a.txt` as `M` among its files, and `new.txt` as `A`
  among its untracked files, whose content is `new.txt` as it was.
- [ ] A stash made with tracked changes only has no untracked files, and no heading for them.
- [ ] Selecting a stash in the graph shows its files and **1 untracked file** with `new.txt` under it.

### `stash.view` — The Stashes view

The view of the side bar that lists the stashes, unfolds each to its files, and applies, pops or drops them.

- **Kind**: View
- **Function**: `StashesApp` in `src/webview/StashesApp.tsx`; the count on its header from `sideView('stashes', …)` in
  `src/extension/extension.ts`.
- **Entry points**: the **Stashes** view of the Source Control side bar, folded at first; **Gitardo: Show Stashes**;
  **Show Stashes** in the warning that a pull with `--autostash` kept the changes in a stash (`sync.pull`).
- **Shows**:
  - On its header, how many stashes there are, nothing when there are none.
  - With none, **No stashes. Put your changes aside with the button above, or from the Changes view: one file, what
    is staged, or everything.**
  - One row per stash, `stash@{0}` first, named by its message, with how long ago it was made beside it; its tooltip
    is `stash@{<n>}: <message>` and the full date.
  - A click on the row or its arrow unfolds it: **Loading…**, then the files it changed, then the untracked files it
    took (`stash.contents`), or **No files**. A file opens its diff in the diff editor, against the commit the stash was
    made on, and an untracked one against nothing. What was read of a stash is kept while it is in the list.
  - On each row, while no operation runs, three buttons: apply (`stash.apply`), pop (`stash.pop`) and **Drop…**
    (`stash.drop`).
  - Its context menu: **Apply**, **Pop**, **Create branch from stash…** (`stash.branch`), **Drop**.
  - On its title, **Stash Changes** (`stash.push`).
- **Implemented in**: `StashesApp`, `stashMenu` (`src/webview/menus.ts`), the view `gitardo.stashes` and the command
  `gitardo.openStashes` in `src/extension/extension.ts` and `package.json`.
- **Tested by**: `test/e2e/ui.mjs` "the stashes in the side bar unfold to their files, and their header counts them";
  `test/e2e/suite.ts` "the five views of the side bar load too, and the graph opens as an editor tab".

Acceptance:

- [ ] With the stash `Put aside`, made with the untracked file `new.txt`, the view lists one row whose name has `Put
  aside`, its header says `1`, and unfolded it lists `new.txt`.
- [ ] The context menu of a row has **Apply**, **Pop**, **Create branch from stash…** and **Drop**.
- [ ] With no stash, the header has no count and the view says **No stashes…**.
- [ ] While an operation runs, the buttons of the rows are not there.

### `stash.graph` — Stashes in the graph and its references

Where the graph shows the stashes, and what their menus there do.

- **Kind**: View
- **Function**: `Repo.withStashes(commits, stashes)` in `src/git/repo.ts`, and `onRowMenu` in `src/webview/App.tsx`,
  which opens `stashMenu` (`src/webview/menus.ts`) on the row of a stash.
- **Shows**:
  - Each stash as a row above the commit it was made on, where its date puts it, while that commit is in the graph:
    on the branches shown, and not while a search or the history of a file narrows it. The commits git makes inside the
    stash for the index and the untracked files are left out. Its node is a square with no fill, joined to its commit
    by a dashed line; its label is `stash@{<n>}`.
  - Its context menu, as in the Stashes view: **Apply**, **Pop**, **Create branch from stash…**, **Drop**. Selected,
    it shows what it holds in the details (`stash.contents`).
  - The **Stashes** section of the references beside the graph, one entry per stash named by its message, with the
    tooltip `stash@{<n>}: <message>` and the same context menu; a click on one selects its row. The Branches view has
    no such section.
  - **Show stashes** in the **…** menu of the toolbar, ticked while they are shown, writes `gitardo.graph.showStashes`
    for the user.
- **Settings**: `gitardo.graph.showStashes`, on unless set to `false`.
- **Implemented in**: `Repo.withStashes` and `Repo.state(…, stashes)`; `onRowMenu` in `src/webview/App.tsx`; the
  Stashes section of `src/webview/RefsTree.tsx`; **Show stashes** in `src/webview/Toolbar.tsx`; `setShowStashes` in
  `Controller.onMessage`.
- **Tested by**: `test/integration.test.ts` "shows the stashes in the graph, each above the commit it was made on, when
  that commit is shown".

Acceptance:

- [ ] A stash made on `main` is a row above the commit of `main`, with `stash@{0}` and a single parent, that commit,
  joined to it by a dashed line.
- [ ] It is not in the graph narrowed to a branch that does not have its commit, nor among what a search for its
  message finds, nor with `gitardo.graph.showStashes` set to `false`.
- [ ] Its context menu in the graph has **Apply**, **Pop**, **Create branch from stash…** and **Drop**, and none of
  the entries of a commit.
