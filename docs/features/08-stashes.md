# Stashes

Changes set aside with `git stash` and brought back later: all of them, only what is staged, or some files; the
**Stashes** list that has them, kept aside in the branch rail; applying, popping, dropping a stash or making a branch of
it, each said first with what it would stop on; the stashes in the timeline; the stash a pull leaves when the changes it
set aside no longer fit; and Undo of a stash dropped.

### `stash.read-list` — Read the stashes

The stashes of the repository, newest first, as the branch rail and the timeline list them.

- **Kind**: Read
- **Function**: `git::read::stash::read_list(git: &Git, request: &StashListRequest) -> Result<Vec<Stash>, GitError>`
- **Reads**: `git stash list -z --format=%H%x1f%gd%x1f%gs%x1f%at`: the hash of each stash, its name (`stash@{0}`), its
  message and its date.
- **Extension**: read by the engine with the state of the repository (`repo.read-state`, asked by `reads.parts` in
  `src/engine/reads.ts`) each time the extension reads the repository, into the `Stash` of `src/shared/state.ts`; the
  graph places them among its commits with the engine's graph read (`graphState` in `src/engine/graph.ts`). The
  repository is read again when `logs/refs/stash` changes in the git directory, also from a terminal.

Acceptance:

- [ ] Two stashes are returned newest first, as `stash@{0}` and `stash@{1}`, with their messages and dates.
- [ ] A repository with no stash returns none, and no error.
- [ ] A message with spaces, colons and quotes is returned whole.
- [ ] Extension: after `git stash push -m "work in progress"` on `main` in a terminal, the Stashes view lists
      `stash@{0}` as `On main: work in progress` without a refresh.

### `stash.read-files` — Read what a stash changes

The files a stash changes, and the untracked files it took along, for what applying it would stop on.

- **Kind**: Read
- **Function**: `git::read::stash::read_files(git: &Git, request: &StashRef) -> Result<StashFiles, GitError>`
- **Reads**:
  - `git diff --name-only -z <stash>^1 <stash>`: the files it changes against the commit it was made on;
  - `git ls-tree -r -z --name-only <stash>^3`: the untracked files it took along; a stash made without them has no third
    parent, which is no error.
- **Refuses**: a name that is not of the form `stash@{N}`: **"`<name>`" is not a stash.**
- **Extension**: what a stash holds is read by the engine as the details of a commit (`details.read-commit`, asked by
  `reads.commitDetail` in `src/engine/reads.ts`), the untracked files it took along in `CommitDetail.untrackedFiles`
  (`src/shared/state.ts`). The details of the graph and the tab of a commit show those files under **1 untracked file**
  or **N untracked files**, the Stashes view with no heading; an untracked file opens as a diff against nothing. What
  applying a stash would stop on is read by the engine, in the plan of `stash.apply` and `stash.pop`.

Acceptance:

- [ ] A stash of two changed files and one untracked file returns the two and the one apart.
- [ ] A stash made without `--include-untracked` returns no untracked files.
- [ ] `main` as a stash name is refused before git runs.
- [ ] Extension: a stash of `a.txt` changed and `new.txt` untracked lists `a.txt` as `M` among its files, and
      `new.txt` as `A` among its untracked files, whose content is `new.txt` as it was.
- [ ] Extension: a stash made with tracked changes only has no untracked files, and no heading for them.
- [ ] Extension: selecting in the graph a stash that took `new.txt` shows its files and **1 untracked file** with
      `new.txt` under it.

### `stash.pane` — The Stashes list

Lists the stashes, in **Kept aside** of the branch rail; each unfolds to the files it holds, which open as diffs.

- **Kind**: View
- **Function**: `views::refs::stashes_view(state: &StashesRow, ui: &mut Ui) -> Vec<Intent>`: a row of the list, as the
  list is rows of the branch rail, filtered, listed in full and drawn only in sight as the other lists are.
- **Entry points**: the **Stashes** list in **Kept aside** of the branch rail; Quick search: **Show Stashes**, which
  opens the list and shows it, with the branch rail and its filter box emptied; **Show Stashes** in the message of
  `stash.autostash-left`.
- **Extension**: the **Stashes** view of the Source Control side bar (`StashesApp` in `src/webview/StashesApp.tsx`,
  the view `gitardo.stashes`), folded at first, its header counting the stashes (`sideView('stashes', …)` in
  `src/extension/extension.ts`), and nothing when there are none; **Gitardo: Show Stashes** (`gitardo.openStashes`)
  and **Show Stashes** in the warning that a pull with `--autostash` kept the changes in a stash show it, and its
  title has **Stash Changes** (`stash.push`). Its rows, buttons and context menu are those of this list, the
  operations sent to Gitardo's engine as their cards say; a file of a stash opens in VS Code's diff editor. What it
  shows is read by the engine (`stash.read-list`, `stash.read-files`).
- **Reads**: `stash.read-list`; `details.read-commit` for a stash unfolded.
- **Uses**: `stash.push`, `stash.apply`, `stash.pop`, `stash.drop`, `stash.branch`, the Diff page (`diff.page`).

What it shows:

- its title counts the stashes and has a button that stashes all changes (`stash.push`), **Stash all changes…**, there
  under the pointer, as the **…** of the other titles, and while the list is open with no stash; it is not there before
  the first commit, and cannot be clicked while Gitardo is running an operation;
- with no stash: **No stashes. Put your changes aside with the button above, or from the Changes page: one file, what
  is staged, or everything.**;
- each stash by its message and how long ago it was made; its tooltip has `stash@{N}: <message>` and the full date;
- on each row under the pointer, in place of how long ago it was made, unless Gitardo is running an operation:
  **Apply: bring the changes back and keep the stash**, **Pop: bring the changes back and drop the stash** and
  **Drop…**;
- its context menu: **Apply**, **Pop**, **Create branch from stash…** and, apart, **Drop**;
- a click selects it in the timeline (`refs.tree`); the arrow before it unfolds it, **Loading…** while it is read: the
  files it changed, a click opening the diff of one against the commit it was made on, then the untracked files it took,
  a click opening the file against nothing; **No files** when it has none.

What was read of a stash is kept while it is listed, as a stash does not change; a stash gone is forgotten.

Acceptance:

- [ ] The title of the list says how many stashes there are.
- [ ] Unfolding a stash made with untracked files lists its changed files, then its untracked ones.
- [ ] A click on a changed file opens the Diff page of the file between the commit the stash was made on and the stash.
- [ ] The buttons of the rows are not there while Gitardo is running an operation.
- [ ] With no stash, the list says how to make one.
- [ ] A stash dropped from elsewhere leaves the list when the repository is read again.
- [ ] Extension: with the stash `Put aside`, made with the untracked file `new.txt`, the view lists one row whose name
      has `Put aside`, its header says `1`, and unfolded it lists `new.txt`.
- [ ] Extension: with no stash, the header of the view has no count.

### `stash.push` — Stash all changes

Sets every change aside, staged or not, untracked files included, and leaves the working tree as the last commit.

- **Kind**: Operation
- **Function**: `features::stash::push(context, answers) -> Outcome`
- **Entry points**: **Stash** in the **…** menu of the header (**Stash: git stash push --include-untracked**); the
  button on the title of the Stashes list; **Stash all changes…** on the header of the list in the Changes page;
  **Stash changes…** in the context menu of the draft; **Stash all changes…** above the files of the draft in the
  inspector when none is ticked; Quick search: **Stash Changes**.
- **Extension**: **Stash changes…** in the context menu of the uncommitted changes row of the graph; **Stash** in the
  toolbar of the graph (tooltip **Stash: git stash push --include-untracked**); **Stash all changes…** above the files
  of the uncommitted changes in the details while none is ticked, and on the header of **Changes** in the Changes view;
  **Stash Changes** on the title of the Stashes view; **Gitardo: Stash Changes** in the Command Palette; **Stash All
  Changes…** on the **Changes** group of VS Code's own list of changes in the Source Control view, as a button and in
  its context menu, while `gitardo.scm.stashButtons` is on, run in the repository of the first file of the group, which
  the views then show (`Controller.stashFromScm`). Each sends `feature/run` of `stash.push` to Gitardo's engine
  (`Controller.runOnEngine` in `src/extension/controller.ts`, `engineRuns` in `src/engine/actions.ts`); the message is
  asked in VS Code's input box, and the plan shown in a modal dialog when `gitardo.commandPreview` is `all`
  (`VscodeAsker` in `src/engine/vscode.ts`). When the engine's binary is missing, it says so (D49).
- **Offered when**: there is a commit (git has nothing to stash onto before the first one); **Stash** in the header also
  needs changes, and is disabled while Gitardo is running an operation.
- **Asks**: **Stash changes**, an input box with the prompt **Optional message. Untracked files are stashed too.**,
  empty by default. Escape backs out.
- **Plan**: `git stash push --include-untracked`, followed by `-m <message>` when one was typed, the message trimmed.
- **Danger**: None
- **Undo**: Undo does not take it back, as no branch moves: `stash.pop` brings the changes back.
- **After**: the stash is first in the Stashes list of the branch rail and, when shown, in the timeline.

Acceptance:

- [ ] With a changed file, a staged file and an untracked file, the stash takes the three, and `git status` is clean.
- [ ] A message typed becomes the message of the stash; with none, git names it `WIP on <branch>: …`.
- [ ] Escape in the box stashes nothing.
- [ ] Before the first commit, the entries are not offered.
- [ ] Extension: with `a.txt` changed and `new.txt` untracked on `main`, stashing with the message `work in progress`
      runs `git stash push --include-untracked -m "work in progress"` in the engine, written to the **Gitardo** output
      channel, leaves nothing uncommitted, and lists `stash@{0}` as `On main: work in progress`.
- [ ] Extension: with `gitardo.commandPreview` at `off` it does not ask; with `all`, the command is shown first.
- [ ] Extension: **Stash All Changes…** on the **Changes** group of VS Code's list of changes runs
      `git stash push --include-untracked`, untracked files included; with `gitardo.scm.stashButtons` off it is not
      there.
- [ ] Extension: **Gitardo: Stash Changes** with nothing to stash says *There are no changes to stash.* before asking
      for a message, and runs nothing.
- [ ] Extension: right after it, **Undo** says it cannot take the stash back, rather than taking back the operation
      before it.
- [ ] Extension: with `gitardo.engine.path` naming a missing file, **Gitardo: Stash Changes** shows **Gitardo's engine
      is not available on this system.** with **Show the log**, and nothing of this file runs.

### `stash.push-staged` — Stash the staged changes

Sets aside only what is staged; the rest stays where it is.

- **Kind**: Operation
- **Function**: `features::stash::push_staged(context, answers) -> Outcome`
- **Entry points**: **Stash only the staged changes…** on the header of the list in the Changes page; **Stash staged
  changes…** in the context menu of the draft.
- **Extension**: **Stash staged changes…** in the context menu of the uncommitted changes row of the graph; **Stash only
  the staged changes…** on the header of **Changes** in the Changes view; **Stash Only the Staged Changes…** on the
  **Staged Changes** group of VS Code's own list of changes, as a button and in its context menu, while
  `gitardo.scm.stashButtons` is on (`Controller.stashFromScm`). Each sends `feature/run` of `stash.push-staged` to
  Gitardo's engine (`Controller.runOnEngine`, `engineRuns`), its message asked in VS Code's input box (`VscodeAsker`).
  When the engine's binary is missing, it says so (D49).
- **Offered when**: there is a commit and something is staged.
- **Asks**: **Stash staged changes**, an input box with the prompt **Optional message. Only what is staged is stashed;
  the rest stays where it is.**, empty by default.
- **Plan**: `git stash push --staged`, followed by `-m <message>` when one was typed. Untracked files are never staged,
  so `--include-untracked` is not given.
- **Danger**: None
- **Undo**: Undo does not take it back, as no branch moves: `stash.pop` brings them back.
- **After**: as `stash.push`.
- **Refuses**: git refuses `--staged` before Git 2.35, and the failure is shown with git's words.

Acceptance:

- [ ] With one file staged and another changed, only the staged one is stashed; the other is still changed.
- [ ] The entry is not there while nothing is staged.
- [ ] Extension: with `a.txt` staged and `b.txt` changed and not staged, the message `staged` runs
      `git stash push --staged -m staged` in the engine, leaves `a.txt` as the last commit has it, and leaves `b.txt`
      and the untracked files changed.
- [ ] Extension: right after it, **Undo** says it cannot take the stash back, rather than taking back the operation
      before it.

### `stash.push-file` — Stash a file, or the files ticked

Sets aside every change to one file, or to the files chosen, staged or not.

- **Kind**: Operation
- **Function**: `features::stash::push_file(context, answers) -> Outcome`
- **Entry points**: **Stash this file…** on the row of a file in the Changes page and in the inspector while the draft
  is selected; **Stash N selected files…** above the files of the draft in the inspector, while some are ticked.
- **Extension**: **Stash this file…** on each file of the Changes view and of the uncommitted changes in the details;
  **Stash 1 selected file…** or **Stash N selected files…** above the files of the uncommitted changes in the details,
  while some are ticked; **Stash This File…** on each file of the **Staged Changes**, **Changes** and **Untracked
  Changes** groups of VS Code's own list of changes, as a button and in the context menu, for every file selected there,
  while `gitardo.scm.stashButtons` is on (`Controller.stashFromScm`, which leaves out files outside the repository and
  runs in the repository of the first file, which the views then show). Each sends `feature/run` of `stash.push-file` to
  Gitardo's engine on the files given, a renamed file with both its names (`engineRuns`, run by
  `Controller.runOnEngine`), its message asked in VS Code's input box (`VscodeAsker`). When the engine's binary is
  missing, it says so (D49).
- **Offered when**: there is a commit; not on a file whose changes are only unsaved edits in Gitardo's editor.
- **Asks**: **Stash `<file>`**, or **Stash N files**, an input box with the prompt **Optional message. Every change to
  the file is stashed, staged or not.**, empty by default.
- **Plan**: `git stash push --include-untracked [-m <message>] -- <file>…`; a renamed file is given with both its names,
  after `git reset --quiet -- <old name>` when the rename is staged, as git does not find the old name otherwise.
  A name that is also a pattern to git (with `[`, `]`, `*`, `?` or `\`, or starting with `:`) is given as
  `:(literal)<name>`.
- **Danger**: None
- **Undo**: Undo does not take it back, as no branch moves: `stash.pop` brings them back.
- **After**: as `stash.push`.

Acceptance:

- [ ] Stashing `src/app.rs` with other files changed stashes that file only, staged and unstaged changes alike.
- [ ] Stashing an untracked file takes it into the stash and out of the working tree.
- [ ] Ticking two files in the inspector and stashing stashes those two only.
- [ ] Stashing `pages/[id].tsx` leaves `pages/i.tsx` and `pages/d.tsx` as they were.
- [ ] Stashing a staged rename stashes both its old and its new name.
- [ ] Extension: **Stash This File…** on `a.txt` in VS Code's list of changes, with the message `from the list`, runs
      `git stash push --include-untracked -m "from the list" -- a.txt` in the engine, and `keep.txt`, untracked, stays.
- [ ] Extension: stashing `a.txt` and the untracked `new.txt` together leaves `b.txt` changed and `new.txt` gone, and
      popping the stash brings `new.txt` back.
- [ ] Extension: a file of another repository of the workspace is stashed in that repository, which the views then
      show.
- [ ] Extension: **Stash this file…** on a file whose rename is staged stashes both its names, rather than failing.
- [ ] Extension: right after it, **Undo** says it cannot take the stash back, rather than taking back the operation
      before it.

### `stash.preview` — What applying a stash would stop on

Says, before a stash is applied or popped, what git would stop on: the conflicts its changes make with what is
committed, the changed files it would refuse to touch, and the untracked files it took that are there again.

- **Kind**: Derivation
- **Function**: `model::stash::preview(input: &StashPreviewInput) -> StashPreview`
- **Extension**: the engine's plans of `stash.apply` and `stash.pop` carry it, and the extension shows it in the note
  of their modal dialog (`VscodeAsker` in `src/engine/vscode.ts`): files in conflict count as files with changes, and
  nothing is said of conflicts for a stash git will refuse. When the engine's binary is missing, it says so (D49).

Rules:

- **Input**: the files the stash changes and the untracked files it took (`stash.read-files`), the files of the working
  tree that have changes, staged or not, other than untracked ones, the untracked files of the stash that exist in the
  working tree, and the files `git merge-tree --write-tree --no-messages --name-only -z --merge-base=<stash>^1 HEAD
  <stash>` names in conflict, or nothing when git cannot tell (older than 2.40). **Output**: the paragraphs of the note
  of the plan, in this order, after the note of `stash.apply` or `stash.pop`.
- Files the stash changes that have changes not committed: **Git will refuse it and change nothing, as N files it
  changes have changes that are not committed: commit or stash them first.**, with their list.
- Untracked files it took that are there again: **N untracked files it took along are there again: git leaves them as
  it is and keeps the stash.**, with their list.
- Conflicts: **This will stop with conflicts in N files:**, with their list.
- No conflict, and no file of the first kind: **No conflicts expected.** When git cannot tell, nothing is said of the
  conflicts.
- `1 file` and the singular forms (**1 file it changes has**, **An untracked file it took along is there again: git
  leaves it as it is and keeps the stash.**) are used for one. Each list names the first 15 files and ends with **…and K
  more**.

Examples:

- A stash of `a.txt`, with `a.txt` changed in the working tree: the note says git will refuse it, naming `a.txt`.
- A stash whose change to `b.txt` meets a commit made since that changed the same lines: **This will stop with
  conflicts in 1 file:** `b.txt`.

Acceptance:

- [ ] A stash that applies cleanly on a clean working tree says **No conflicts expected.**
- [ ] A stash whose file is changed in the working tree names that file, and says nothing of conflicts expected.
- [ ] An untracked file of the stash created again in the working tree is named.
- [ ] A conflict with a commit made after the stash names the file.
- [ ] Twenty files in conflict list 15 and **…and 5 more**.
- [ ] Extension: the three cases (git will refuse it, an untracked file it took is there again, conflicts) each show
      in the dialog of **Apply** before anything runs.
- [ ] Extension: with a file the stash changes in conflict in the working tree, the dialog says git will refuse it,
      naming that file, and predicts no conflicts.

### `stash.apply` — Apply a stash

Brings the changes of a stash back into the files, and keeps the stash.

- **Kind**: Operation
- **Function**: `features::stash::apply(context, answers) -> Outcome`
- **Entry points**: **Apply** on the row and in the context menu of a stash in the Stashes list of the branch rail; in
  the context menu of its row in the timeline.
- **Extension**: **Apply** in the context menu of a stash in the Stashes view, in the Stashes section of the references
  beside the graph and on its row in the graph; the first button on the row of a stash in the Stashes view (tooltip
  **Apply: bring the changes back and keep the stash**). Each sends `feature/run` of `stash.apply` to Gitardo's engine,
  the stash by its index in `stash@{N}` (`stashIndex` and `engineRuns` in `src/engine/actions.ts`, run by
  `Controller.runOnEngine`), its plan always shown in a modal dialog first (`VscodeAsker`). A name that is not
  `stash@{N}` is not sent to the engine, and nothing runs. When the engine's binary is missing, it
  says so (D49).
- **Reads**: `stash.read-files`, the conflicts by `git merge-tree` as `stash.preview` says, the state of the working
  tree.
- **Plan**: `git stash apply stash@{N}`, titled **Apply stash@{N}**, with the note **Its changes go into the files, and
  it stays in the list.** followed by `stash.preview`.
- **Danger**: Confirm, because it changes the files and may stop with conflicts: it is always asked first, saying what
  it would stop on.
- **Undo**: not by the reflog, which it does not move; the stash stays, so the changes can be discarded and applied
  again.
- **After**: when files are left in conflict, the banner of conflicts shows (`conflicts.banner`), and the files are
  listed in the Changes page.
- **Settings**: `commandPreview`.
- **Uses**: `stash.read-files`, `stash.preview`, `preview.confirm`.

Acceptance:

- [ ] **Apply** asks first, with the note and what it would stop on, also with `commandPreview` set to `off`.
- [ ] Once confirmed, the changes are in the files and the stash is still listed.
- [ ] A stash that conflicts leaves the files in conflict, the banner shows, and the stash is kept.
- [ ] When git refuses because a file it changes has changes, nothing changes and git's words are shown.
- [ ] Extension: applying `stash@{0}` asks **Apply stash@{0}?** in a modal dialog, even with `gitardo.commandPreview`
      at `off`; with nothing in the way it says **No conflicts expected.** and **it stays in the list**; cancelled, it
      changes nothing.
- [ ] Extension: `--all` in place of a stash is refused, *"--all" is not a stash.*, and nothing runs.

### `stash.pop` — Pop a stash

Brings the changes of a stash back into the files, and drops the stash once they are in without conflicts.

- **Kind**: Operation
- **Function**: `features::stash::pop(context, answers) -> Outcome`
- **Entry points**: **Pop** on the row and in the context menu of a stash in the Stashes list of the branch rail; in
  the context menu of its row in the timeline.
- **Extension**: **Pop** in the context menu of a stash in the Stashes view, in the Stashes section of the references
  beside the graph and on its row in the graph; the second button on the row of a stash in the Stashes view (tooltip
  **Pop: bring the changes back and drop the stash**). Each sends `feature/run` of `stash.pop` to Gitardo's engine, as
  **Apply** sends `stash.apply`, its plan always shown in a modal dialog first (`VscodeAsker`). When the engine's binary
  is missing, it says so (D49).
- **Reads**: as `stash.apply`.
- **Plan**: `git stash pop stash@{N}`, titled **Pop stash@{N}**, with the note **Its changes go into the files, and it
  is dropped once they are in without conflicts: with conflicts it stays.** followed by `stash.preview`.
- **Danger**: Confirm, because it changes the files, may stop with conflicts, and drops the stash: it is always asked
  first, saying what it would stop on.
- **Undo**: not by the reflog, which it does not move. A stash dropped by a clean pop is no longer listed.
- **After**: as `stash.apply`. Git keeps the stash when it stopped with conflicts.
- **Settings**: `commandPreview`.
- **Uses**: `stash.read-files`, `stash.preview`, `preview.confirm`.

Acceptance:

- [ ] A clean pop puts the changes in the files and the stash leaves the list.
- [ ] A pop that conflicts leaves the files in conflict and the stash in the list.
- [ ] **Pop** asks first, also with `commandPreview` set to `off`.
- [ ] Extension: popping `stash@{0}` asks **Pop stash@{0}?** in a modal dialog, even with `gitardo.commandPreview` at
      `off`, and with nothing in the way it says **No conflicts expected.**
- [ ] Extension: with an untracked file it took there again, the tracked changes go in, the file is left as it was,
      and the stash is still listed.

### `stash.drop` — Drop a stash

Deletes a stash and the changes it holds.

- **Kind**: Operation
- **Function**: `features::stash::drop(context, answers) -> Outcome`
- **Entry points**: **Drop…** on the row and **Drop** in the context menu of a stash in the Stashes list of the branch
  rail; **Drop** in the context menu of its row in the timeline.
- **Extension**: **Drop**, marked as an entry that loses work, in the context menu of a stash in the Stashes view, in
  the Stashes section of the references beside the graph and on its row in the graph; the **Drop…** button on the row of
  a stash in the Stashes view. Each sends `feature/run` of `stash.drop` to Gitardo's engine, the stash by its index
  (`engineRuns` in `src/engine/actions.ts`, run by `Controller.runOnEngine`), its warning always asked in a modal
  warning dialog (`VscodeAsker`); the engine's journal keeps what `stash.undo-drop` needs, and **Undo** is the engine's
  (`undo.last`). When the engine's binary is missing, it says so (D49).
- **Plan**: `git stash drop stash@{N}`, titled **Drop stash@{N}**, with the note **The stashed changes will be
  deleted.**
- **Danger**: Destructive, because the changes of the stash are deleted and no reference keeps them.
- **Undo**: `stash.undo-drop`, right after and as long as nothing else happened since.
- **After**: the stash leaves every list; the ones after it are renumbered by git.
- **Settings**: `commandPreview`.

Acceptance:

- [ ] **Drop** asks with a warning, also with `commandPreview` set to `off`.
- [ ] Once confirmed, the stash is gone and `stash@{1}` becomes `stash@{0}`.
- [ ] Undo right after brings it back (`stash.undo-drop`).
- [ ] Extension: dropping `stash@{0}` warns **Drop stash@{0}?** with **The stashed changes will be deleted.** and the
      command, whatever `gitardo.commandPreview` says; cancelled, the stash is still listed.
- [ ] Extension: `--all` in place of a stash is refused, and nothing runs.

### `stash.undo-drop` — Undo a stash dropped

Brings back a stash dropped in Gitardo, while its commit is still in the repository.

- **Kind**: Operation
- **Function**: `features::stash::undo_drop(context, answers) -> Outcome`
- **Entry points**: **Undo** in the **…** menu of the header (`undo.last`), right after `stash.drop`.
- **Extension**: the **Undo** of the extension, which runs `undo.last` in Gitardo's engine (its **Extension** line),
  right after `stash.drop` ran in the engine.
- **Offered when**: the last operation Gitardo ran was the drop, and the last entry of the reflog of `HEAD` is the one
  there was when it ran: nothing else happened since. Otherwise Undo takes back what the reflog says.
- **Reads**: none: the message and the hash of the stash were kept when it was dropped.
- **Plan**: `git stash store -m <message> <hash>`, titled **Undo: restore dropped stash**.
- **Danger**: Confirm, because Undo always shows what it is about to do first.
- **Undo**: by `stash.drop` again.
- **After**: the stash is `stash@{0}` again, with its message. Undo is used up.
- **Refuses**: git refuses when the commit of the stash has been collected since; its words are shown.

Acceptance:

- [ ] Dropping `stash@{0}` and pressing **Undo** shows `git stash store -m "<message>" <hash>` and, confirmed, lists the
      stash again with its message and its files.
- [ ] A stash dropped, then a commit made, is not restored by Undo: Undo takes back the commit.
- [ ] A stash restored from `stash@{2}` comes back as `stash@{0}`.
- [ ] Extension: right after dropping the stash `On main: keep me`, **Undo** asks first and runs
      `git stash store -m "On main: keep me" <hash>`, and the stash is listed again with the same hash and message.
- [ ] Extension: two stashes dropped are brought back by two **Undo**, newest first; staging a file in between makes
      **Undo** forget neither.

### `stash.branch` — Create a branch from a stash

Makes a branch at the commit a stash was made on, checks it out, and applies the stash there, where it cannot conflict
with what came after.

- **Kind**: Operation
- **Function**: `features::stash::branch(context, answers) -> Outcome`
- **Entry points**: **Create branch from stash…** in the context menu of a stash in the Stashes list of the branch
  rail and of its row in the timeline.
- **Extension**: **Create branch from stash…** in the context menu of a stash in the Stashes view, in the Stashes
  section of the references beside the graph and on its row in the graph. It sends `feature/run` of `stash.branch` to
  Gitardo's engine, the stash by its index (`engineRuns` in `src/engine/actions.ts`, run by `Controller.runOnEngine`);
  the name is asked in VS Code's input box, checked by the engine with `question/validate` as it is typed (`VscodeAsker`
  in `src/engine/vscode.ts`), and the update of the submodules follows as here, by
  `gitardo.submodules.updateOnCheckout`. When the engine's binary is missing, it says so (D49).
- **Asks**: **Create branch from stash@{N}**: the name of the branch, refused while typing when git does not accept it
  as a branch name (`branch.validate-name`) or when **A branch with that name already exists**.
- **Plan**: `git stash branch <name> stash@{N}`, titled **Create branch `<name>` from stash@{N}**; followed by the update
  of the submodules when the commit it was made on records other submodule commits than those checked out
  (`submodule.update-on-checkout`).
- **Danger**: None
- **Undo**: by the reflog: Undo checks out again what was checked out before; the branch stays, and the stash, which git
  drops once it applied cleanly, is not brought back.
- **After**: the new branch is checked out with the changes of the stash in its files; git drops the stash when it
  applied cleanly, and keeps it otherwise.
- **Refuses**: git refuses when the files it would check out have changes; its words are shown.
- **Uses**: `branch.validate-name`, `submodule.update-on-checkout`.

Acceptance:

- [ ] On a stash made on `a1b2c3d`, typing `rescue` makes `rescue` at `a1b2c3d`, checks it out and applies the stash.
- [ ] The stash leaves the list when it applied cleanly.
- [ ] A name of an existing branch, or one git refuses such as `bad..name`, cannot be accepted.
- [ ] Extension: creating `rescue` from `stash@{0}` in the engine checks out `rescue` at the commit the stash was made
      on, with its changes, and the stash is gone from the list.
- [ ] Extension: `-x`, `a..b` and the name of an existing branch are refused in the input box while they are typed,
      and nothing runs.
- [ ] Extension: in a repository with submodules, `git submodule update --init --recursive` runs after it; with
      `gitardo.submodules.updateOnCheckout` off, it does not.
- [ ] Extension: right after it, **Undo** goes back to the branch checked out before.

### `stash.in-graph` — Stashes in the timeline

Shows each stash in the timeline, above the commit it was made on and where its date puts it, so that what was set aside
is seen where it belongs.

- **Kind**: Derivation
- **Function**: `model::stash::in_graph(input: &GraphWithStashes) -> Vec<Row>`
- **Entry points**: the timeline and its narrow layout; **Show stashes** in the **…** menu of the header, which
  switches them on and off.
- **Reads**: `stash.read-list`; the commits of the stashes with `git log --no-walk=unsorted -z --format=<hash, parents,
  author name, author email, author date, subject> <hashes of the stashes> --`.
- **Settings**: `graph.showStashes`, on by default.
- **Uses**: `graph.lanes`, `stash.apply`, `stash.pop`, `stash.drop`, `stash.branch`, `details.stash`.

Rules:

- **Input**: the commits of the timeline in their order, the stashes with their commits. **Output**: the rows of the
  timeline, with the stashes among the commits.
- A stash is shown only when the commit it was made on, its first parent, is among the commits loaded: on the branches
  the timeline shows, as far as it has loaded them. Not while the timeline shows what a search finds or the history of a
  file.
- A stash keeps its first parent only: the commits that keep its index and its untracked files are not shown, as the
  stash stands for them.
- The stashes are placed newest first, each before the first row whose date is not later than its own, but never below
  the commit it was made on: stashes made on the same commit are listed as they were made.
- A stash is drawn as a square with no fill rather than a dot, joined to its commit by a dashed line, as the draft
  is; its label, `stash@{N}`, has a dotted border, and its message is in italics.
- Its context menu has **Apply**, **Pop**, **Create branch from stash…** and **Drop**, as in the Stashes list, and none
  of the operations of a commit; it cannot be dragged, nor dropped on, nor selected together with commits.
- Selected, it shows in the inspector (`details.stash`); double-clicked, its page (`commit-page.open`).

Examples:

- A stash made on `main` an hour ago, after which a commit was made on `feature` ten minutes ago: the stash is below
  that commit of `feature` and above the tip of `main`.

Acceptance:

- [ ] A stash made on the commit checked out is the row right above it, joined by a dashed line.
- [ ] A stash made on a commit the timeline does not show is not in the timeline.
- [ ] **Show stashes** unchecked takes every stash out of the timeline, and checked brings them back; the setting is the
      same switch.
- [ ] With a search shown, or the history of a file, no stash is in the timeline.
- [ ] Two stashes on the same commit are listed newest first, both above it.
- [ ] The context menu of a stash row has the four stash entries only.

### `stash.autostash-left` — The stash a pull leaves

A pull with `--autostash` sets the changes aside, pulls, and puts them back; when they no longer fit on what came in,
git leaves them in conflict in the files and keeps the stash. Nothing says so unless git's output is read: Gitardo says
it, and offers the stashes.

- **Kind**: Derivation
- **Function**: `model::stash::autostash_left(input: &PullOutcome) -> Option<Notice>`
- **Entry points**: after `sync.pull` with `--autostash`, **Pull** or another way to pull in the menu of **Sync**.
- **Uses**: `stash.pane`, `stash.drop`, `conflicts.banner`.

Rules:

- **Input**: how the pull was run, whether it succeeded, the stashes before it and after it. **Output**: a warning, or
  nothing.
- A warning when the pull had `--autostash`, succeeded, there is one stash more than before, and the newest stash has
  `autostash` in its message, in any case: **Pulled, but your changes do not fit on what came in. They are in conflict
  in the files, and whole in the stash "`<message>`": resolve the conflicts, then drop the stash.**, with **Show
  Stashes**, which shows the Stashes list of the branch rail.
- Nothing otherwise: when the changes went back cleanly, git drops the stash; when the pull itself stops on a conflict,
  git puts the changes back once the merge or the rebase is finished or given up. Untracked files are not touched.

Acceptance:

- [ ] A pull with `--autostash` whose incoming commit changes the lines changed in the working tree leaves the files in
      conflict, a stash named `autostash`, and shows the warning.
- [ ] **Show Stashes** shows the Stashes list with that stash first.
- [ ] A pull whose changes go back cleanly says nothing of a stash.
- [ ] A pull without `--autostash` never shows the warning.
