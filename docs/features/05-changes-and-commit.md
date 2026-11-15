# Changes and commit

What is not committed yet, and making a commit of it: the **Changes** page, opened by **Review** in the draft, with its
one list of files, staging and unstaging, marking conflicts as resolved, discarding, the files edited in Gitardo's own
editors and not saved yet, the commit box with **Commit** and **Amend**, whose message and **Commit** the draft at the
top of the timeline shows too, the message drafted by the AI provider, and the identity that commits are made as. The same staging and
discarding are in the context menu of the draft. Stashing from this page is in `stash.`; taking one side of a file in
conflict is in `conflicts.`.

### `changes.pane` — The Changes page

The page where the changes are staged and committed: one list of the changed files, each with a box, the files in
conflict apart above it, and the commit box at the top, whose message and **Commit** the draft at the top of the
timeline shows too. The rest of the commit box, **Amend last commit**, the button that drafts the message with the AI
provider and the identity, is on this page only. While the timeline is narrowed to branches that do not have the
commit checked out, the draft is not there, and this page is where one commits.

- **Kind**: View
- **Function**: `views::changes::pane(state: &ChangesState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: **Review** in the draft (`graph.uncommitted-row`); **Show changes** in Quick search opens it and
  puts the keyboard in its message box.
- **Extension**: the **Gitardo Changes** view of the Source Control side bar (`ChangesApp` in
  `src/webview/ChangesApp.tsx`, view kind `changes`, registered as `gitardo.changes`), declared hidden in
  `package.json`, so that VS Code shows it only once **Gitardo: Show Changes** in the Command Palette
  (`gitardo.openChanges`), ticking **Gitardo Changes** in the **…** menu of the side bar, or **Show Changes** in the
  message after a squash merge asks for it. Its rows, boxes and buttons are those of this page, with VS Code's editors
  in place of the pages (`changes.open`); a list of more than 300 files shows the first 300 and **Show all N files…**
  (`FILES_LIMIT` in `src/webview/Capped.tsx`). The view runs nothing itself: what its boxes and buttons ask is sent to
  Gitardo's engine as the cards of the operations say. **View as tree** or **View as list**, and the message of the
  commit box, are kept in the view's state (`loadUiState` and `saveUiState` in `src/webview/vscode.ts`), the layout as
  `changesFiles`. The placeholder of the message is `Message (Ctrl+Enter to commit on main)`, or `on detached HEAD`;
  a click on the identity under it runs `Controller.switchIdentity` (`src/extension/controller.ts`), still in
  TypeScript. Before the repository is read the view says *Loading repository…*.
- **Uses**: `changes.list`, `changes.unsaved`, `changes.open`, `changes.stage`, `changes.unstage`, `changes.stage-all`,
  `changes.unstage-all`, `conflicts.mark-resolved`, `changes.discard`, `changes.discard-all`, `changes.save`,
  `details.show-all-files`, `commit.create`, `commit.amend`, `commit.draft-message`, `identity.switch`, `stash.push`,
  `stash.push-staged`, `stash.push-file`, `conflicts.take-side`, `conflicts.resolve-by-rules`,
  `ai.resolve-conflicts`, `repo.select`, `diff.page`, `merge-view.page`.

From top to bottom the page shows:

1. The banners of a worktree, of an operation that stopped (**Continue**, **Skip**, **Abort**) and of a bisect, as the
   timeline has them.
2. The commit box, whose message and **Commit** the draft shows too, and nothing else of it: the message, with the
   placeholder **Message (Ctrl+Enter to commit on `<branch>`)**, or `detached HEAD` for the branch; **Commit**, which
   reads **Amend** while **Amend last commit** is ticked; the sparkle button that drafts the message
   (`commit.draft-message`), turning while it runs, here only; the box **Amend last commit**, here only. The message
   is kept while the page is closed and when the window is opened again.
3. Under the commit box, the identity commits will be made with, as `Name <email>`, or **No identity configured**,
   here only. A click on it is `identity.switch`.
4. **Conflicts**, when there are files in conflict, with their count: each file with no box, and the buttons
   **Resolve its conflicts automatically, after looking at what is proposed** (only when the rules can resolve some of
   them, `conflicts.resolve-by-rules`), **Resolve with AI, after looking at what is proposed** (only when `ai.*` is set
   up for conflicts and the rules leave some, `ai.resolve-conflicts`), **Take the current side for the whole
   file** and **Take the incoming side for the whole file** (`conflicts.take-side`), and **Mark as resolved**
   (`conflicts.mark-resolved`). A click on one opens it in the Merge page.
5. **Changes**, with their count: the box on its header (`changes.stage-all`, `changes.unstage-all`), then the buttons
   **View as tree** or **View as list**, **Open all changes in one Diff page**, **Stash only the staged changes…**
   (only while something is staged), **Stash all changes…**, **Save all** (only while a file has edits not saved) and
   **Discard all changes** (`changes.discard-all`). Each file has its box, its name with a dot after it when it has
   edits not saved, and the buttons **Open file** (not on a deleted file), **Open changes**, **Save** (only when not
   saved), **Stash this file…** and **Discard changes**. A file that only an editor has changed has its box greyed out,
   with the tooltip **Not saved: save it to stage it**, and the buttons **Compare with the saved file** and **Save**.
6. **No changes** when there is nothing in either list.

With no repository the page says **No Git repository in this workspace** with **Check again**, which reads the folders
of the workspace again. While an operation runs, the buttons that change something are disabled.

**View as tree** shows the files under their directories, which fold; **View as list** shows them one after the other,
each with its directory beside its name. The choice is kept for the page, and the folds while the window is open. Each
file has the icon of its kind from Gitardo's set of file icons.

Acceptance:

- [ ] With one file staged, one partly staged and one untracked, the list has three rows in the order of their paths,
      with the boxes ticked, half ticked and empty.
- [ ] `Ctrl+Enter` (`Cmd+Enter` on macOS) in the message box commits, as the **Commit** button does, and does nothing
      while the button is disabled.
- [ ] **Stash only the staged changes…** is on the header only while at least one file is staged; **Save all** only
      while at least one file has edits not saved.
- [ ] A file in conflict is listed under **Conflicts** only, with no box, and a click on it opens the Merge page.
- [ ] **View as tree** then closing the Changes page and opening it again keeps the tree; it is kept after the window
      is closed and opened again.
- [ ] A deleted file has no **Open file** button.
- [ ] With no repository in the workspace, the page shows **No Git repository in this workspace** and **Check again**.
- [ ] The identity line reads `Ada <ada@example.com>` when `git config user.name` and `user.email` say so, and
      **No identity configured** when neither is set.
- [ ] Extension: after installing, the side bar does not show **Gitardo Changes**; **Gitardo: Show Changes** shows it,
      open.
- [ ] Extension: with 301 changed files the list shows the first 300 and **Show all 301 files…**; the header still
      counts `301`, and its box and buttons act on all 301.
- [ ] Extension: with nothing to commit and no edits not saved the view says *No changes*; before the repository is
      read, *Loading repository…*.
- [ ] Extension: while an operation runs, **Commit** and the box that chooses the repository are disabled.
- [ ] Extension: **Commit** is disabled while the message is empty or only spaces and while nothing is changed; with
      **Amend last commit** ticked it is enabled with an empty message.
- [ ] Extension: a message typed and left, and the choice of tree or list, are still there when the view is hidden
      and shown again.
- [ ] Extension: the directory rows of the tree have no box: staging stays per file and for all.
- [ ] Extension: once a commit or an amend ran, the box is empty and **Amend last commit** unticked; when it fails or
      is refused, the message stays.

### `changes.read-working-tree` — Read what is not committed

What git says of each changed file: staged, not staged, untracked or in conflict.

- **Kind**: Read
- **Function**: `git::read::changes::read_working_tree(git: &Git, request: &WorkingTreeRequest) -> Result<WorkingTree, GitError>`
- **Reads**: `git status --porcelain=v1 -z --untracked-files=all`, run with `GIT_OPTIONAL_LOCKS=0`.

Each record is two status letters, the index one and the working tree one, a space and the path; a rename or a copy
in the index is followed by a record with its old path. The result has three lists:

- **Conflicts**: the records whose two letters are one of `DD`, `AU`, `UD`, `UA`, `DU`, `AA`, `UU`.
- **Staged**: every other record whose index letter is not a space, with its letter, and its old path for `R` and `C`.
- **Not staged**: every other record whose working tree letter is not a space, with that letter; untracked files
  (`??`) are here with the status `?`.

A file changed both in the index and in the working tree is in both lists.

Acceptance:

- [ ] `R  new.txt\0old.txt\0` gives one staged file `new.txt`, status `R`, old path `old.txt`, and nothing else.
- [ ] `MM a.txt\0` gives `a.txt` in the staged list with `M` and in the not staged list with `M`.
- [ ] `UU a.txt\0` and `AA b.txt\0` give two conflicts and nothing staged.
- [ ] `?? dir/new.txt\0` gives an untracked file `dir/new.txt` with the status `?`.
- [ ] A path with spaces or non-ASCII letters comes back exactly as it is on disk.

### `changes.list` — One list of the changed files

Turns what git says into the one list of the Changes page: each file once, as it is next to the last commit, with how
much of it is staged.

- **Kind**: Derivation
- **Function**: `model::changes::list(input: &ChangesInput) -> ChangesList`
- **Uses**: `changes.read-working-tree`, `changes.unsaved`.

Rules:

- A file only in the staged list is **all** staged; only in the not staged list (untracked included), **none**; in
  both, **some**: its box is half ticked. That is how a file looks once some of its changes were staged one by one in
  the Diff page (`diff.stage-change`).
- A file in both lists is shown as the working tree has it next to the last commit: staged as deleted and still on
  disk, it is modified; staged as anything but added and deleted from disk since, it is deleted; staged as added, it
  stays added whatever happened to it since.
- A staged rename is one row under its new name, with its old name beside it. Unstaged, git lists it as the old name
  deleted and the new one untracked: two rows. Ticking both makes it one rename again, as git sees it.
- Files in conflict are not in the list: they are in **Conflicts**, apart.
- The files of `changes.unsaved` that git sees no change in join the list as rows of their own, whose box cannot be
  ticked.
- The rows are ordered by path, byte by byte.
- The box on the header is ticked when every file git sees is all staged, half ticked when some of it is staged, empty
  when none is, and disabled when git sees no file at all (only unsaved files, or nothing).

Acceptance:

- [ ] A file staged and then changed again is one row, half ticked.
- [ ] A file staged as added and then changed again is one row with the status added, half ticked.
- [ ] `git mv a b` gives one row `b` with `a` beside it, ticked; after unstaging it, two rows, `a` deleted and `b`
      untracked, both empty.
- [ ] With every file staged the header box is ticked; with one of three staged it is half ticked.
- [ ] With only a file edited in Gitardo and not saved, the header box is disabled.

### `changes.unsaved` — Files with edits not saved

Which files of the repository have edits typed in one of Gitardo's own editors and not saved yet: the File page and the
editable sides of the Diff and Merge pages.

- **Kind**: Derivation
- **Function**: `model::changes::unsaved(input: &UnsavedInput) -> Vec<String>`, the paths from the top with `/` (Q341)
- **Reads**: `git check-ignore -z --stdin`, given the candidate paths separated by NUL on standard input; git ends
  with code 1 when none of them is ignored, which means none is.
- **Uses**: `file-view.page`, `diff.page`, `merge-view.page`.
- **Extension**: the files of VS Code's text documents with edits not saved, worked out by the extension itself
  (`Controller.checkUnsaved` and `Controller.unsavedIn` in `src/extension/controller.ts`) by the same rules, with
  `git check-ignore -z --stdin` run by `Repo.ignored` (`src/git/repo.ts`) without taking locks, and told to the Changes
  view with an `unsaved` message. It is worked out again at each edit, at each editor closed or saved and at each read
  of the repository, and sent only when the list changed.

Rules:

- The candidates are the buffers with edits not saved whose files are inside the repository, by their paths from its
  root, with forward slashes.
- Left out: the files of its submodules, the files of the other worktrees whose folders are inside it, and the files
  git ignores. A tracked file is never ignored.
- The paths come back sorted.
- The result is worked out again when a buffer becomes or stops being unsaved, and when the repository is read again;
  not at every key typed.

Acceptance:

- [x] Typing in the File page of `src/a.rs` without saving adds `src/a.rs` to the result; `Ctrl+S` takes it out.
- [x] A buffer of a file in a submodule, or in `node_modules` ignored by `.gitignore`, is not in the result.
- [x] A buffer of a file outside the repository is not in the result.
- [x] The row of a file that is changed for git and also has edits not saved has a dot after its name and **Save**.
- [ ] Extension: the dot after the name of a file with edits not saved is titled *Not saved*.
- [ ] Extension: a file git sees no change in, edited in an editor and not saved, is listed among the others in the
      order of the paths, and a click on it opens its editor.
- [ ] Extension: reverting the editor takes the file out of the list, or its dot away, without a read of the
      repository.
- [ ] Extension: the view does not say *No changes* while a file has edits not saved.

### `changes.open` — What a file of the Changes page opens

Decides which page, and which two versions, a click or a button on a file of the Changes page opens.

- **Kind**: Derivation
- **Function**: `model::changes::open(input: &OpenInput) -> PageRequest`
- **Settings**: `changes.openOnClick`: `file` (the default) or `diff`. It changes only what a click on a row does.
- **Uses**: `diff.page`, `file-view.page`, `merge-view.page`.
- **Extension**: a click on a file of the Changes view, its buttons **Open File** and **Open Changes**, **Open all
  changes in one editor** on the header of **Changes**, and **Compare with the saved file** on a file only an editor
  has changed, done by the extension itself (the messages `openChange`, `openFile`, `openDiff`, `openChanges` and
  `compareWithSaved` in `Controller.onMessage`, and `sides` in `src/extension/controller.ts`), with VS Code's editors
  in place of the pages. The diff opens in VS Code's diff editor, titled `a.txt (Working Tree)` or `a.txt (Index)`;
  when VS Code's Git extension has the repository open, its two sides are that extension's own `git:` documents, so
  that **Stage Selected Ranges**, **Unstage Selected Ranges** and **Revert Selected Ranges** work in it. **Open all
  changes in one editor** runs `vscode.changes`: one editor titled **Changes**, where each file has the two sides
  **Open Changes** gives it, rather than the last commit against the file on disk. **Compare with the saved file** runs
  VS Code's `workbench.files.action.compareWithSaved`. A file in conflict with its base and both sides opens in VS
  Code's merge editor. The setting is `gitardo.changes.openOnClick`.

Rules:

- **Open changes** opens a Diff page: the index against the file on disk while some of the file is not staged; the last
  commit against the index once all of it is. A file added has nothing on its left side; a file deleted has nothing on
  its right side.
- **Open file** opens the file in the File page, as it is on disk. A deleted file opens its Diff page instead.
- A click on a row does what **Open file** does, or, with `changes.openOnClick` set to `diff`, what **Open changes**
  does.
- A click on a file in conflict opens its Merge page.
- **Compare with the saved file**, on a file that only an editor has changed, opens a Diff page with the file as it is
  on disk on the left and the buffer not saved on the right, editable.
- **Open all changes in one Diff page** opens one Diff page with the diffs of every file of the list, one after the
  other, each between the last commit and the file on disk, titled **Changes**: a Diff page has one pair of versions
  for all its files (Q342). The rows of files only an editor has changed are left out.

Acceptance:

- [x] A file partly staged opens with the index on the left and the file on disk on the right.
- [x] A file all staged opens with `HEAD` on the left and the index on the right.
- [x] With `changes.openOnClick` set to `diff`, a click on a row opens the same Diff page as **Open changes**; with
      `file`, the File page.
- [x] A click on a deleted file opens its Diff page whatever the setting says.
- [x] **Open all changes in one Diff page** with three changed files opens one Diff page with three files.
- [ ] Extension: **Open all changes in one editor** with `a.txt` wholly staged and `b.txt` with changes not staged
      opens one editor titled **Changes**, with `HEAD` against the index for `a.txt` and the index against the file
      on disk for `b.txt`.
- [ ] Extension: a click on a file only an editor has changed opens that editor; **Compare with the saved file** opens
      VS Code's comparison of the file on disk with what its editor has, not saved.
- [ ] Extension: a click on a file in conflict that has its base and both sides opens it in the merge editor, whatever
      `gitardo.changes.openOnClick` says.

### `changes.stage` — Stage a file

Puts the changes of a file, or of several files given at once, in the index, for the next commit.

- **Kind**: Operation
- **Function**: `features::changes::stage(context, answers) -> Outcome`
- **Entry points**: ticking the box of a file that is empty or half ticked, in the Changes page.
- **Extension**: ticking the box of a file in the Changes view (titled *Click to stage*, or *Partly staged: click to
  stage all of it*); **Mark as resolved** on a file in conflict there; ticking the header box while files are in
  conflict, which gives the files not all staged. Each sends `feature/run` of `changes.stage` to Gitardo's engine
  (`Controller.runOnEngine` in `src/extension/controller.ts`), on the one file or on the files given (`engineRuns` in
  `src/engine/actions.ts`); the plan is shown in a modal dialog when `gitardo.commandPreview` is `all` (`VscodeAsker`
  in `src/engine/vscode.ts`). **Stage File** in the title of the Flow Diff is `diff.stage-file`, still the `GitOp`
  `stageOp` run by `Controller.runIn`. When the engine's binary is missing, `stageOp` of `src/git/ops.ts` runs in its
  place, as before (D49).
- **Offered when**: the file is changed for git; the box of a file that only an editor has changed is disabled.
- **Plan**: `git add -- <path>`, with the path as `changes.pathspec` writes it. Several files given at once
  (`Target::Files`) are added by one command, `git add -- <path>…`, titled **Stage N files**.
- **Danger**: None.
- **Undo**: Not taken back by Undo, which follows the reflog: unticking the box (`changes.unstage`) takes it back.
- **Uses**: `changes.pathspec`.

Acceptance:

- [ ] Ticking an untracked `new.txt` runs `git add -- new.txt` and the box is ticked once the repository is read again.
- [ ] Ticking a half ticked box stages the rest of the file: the box is ticked after.
- [ ] Ticking `pages/[id].tsx` runs `git add -- :(literal)pages/[id].tsx` and leaves `pages/i.tsx` unstaged.
- [ ] Ticking a deleted file stages its deletion.
- [ ] `a.txt` and `b.txt` given at once are added by one `git add -- a.txt b.txt`, titled **Stage 2 files**.
- [ ] Extension: ticking the box of `b.txt` runs `git add -- b.txt` in the engine, written to the **Gitardo** output
      channel.
- [ ] Extension: with `gitardo.commandPreview` set to `off` nothing is asked; set to `all`, a modal dialog *Stage
      b.txt?* shows `git add -- b.txt` and the folder it runs in, with **Run**; **Cancel** stages nothing and leaves
      the box unticked.
- [ ] Extension: a file named `-dash.txt` is staged with `git add -- -dash.txt`: it comes after `--`, so it is never an
      option.
- [ ] Extension: **Mark as resolved** on `clash.ts` in conflict runs `git add -- clash.ts`, and the file leaves
      **Conflicts**.
- [ ] Extension: before the first commit, ticking `a.txt` runs `git add -- a.txt` and lists it as added.
- [ ] Extension: **Undo** right after staging does not unstage the file, and staging does not make **Undo** forget a
      branch deleted just before.
- [ ] Extension: with `gitardo.engine.path` naming a missing file, the output channel says once that the engine was
      not found, and ticking a box runs `stageOp`, as do the other operations of this file with their `GitOp`.

### `changes.unstage` — Unstage a file

Takes the changes of a file, or of several files given at once, out of the index; the files on disk are not touched.

- **Kind**: Operation
- **Function**: `features::changes::unstage(context, answers) -> Outcome`
- **Entry points**: unticking the box of a file that is ticked, in the Changes page.
- **Extension**: unticking the ticked box of a file in the Changes view (titled *Staged: click to unstage*), which
  gives both names of a staged rename; unticking the header box while files are in conflict, which gives the files
  staged. Each sends `feature/run` of `changes.unstage` to Gitardo's engine (`Controller.runOnEngine`), on the one file
  or on the files given (`engineRuns` in `src/engine/actions.ts`), its plan shown in a modal dialog when
  `gitardo.commandPreview` is `all` (`VscodeAsker`). **Unstage File** in the title of the Flow Diff is
  `diff.unstage-file`, still the `GitOp` `unstageOp` run by `Controller.runIn`. When the engine's binary is missing,
  `unstageOp` of `src/git/ops.ts` runs in its place, as before (D49).
- **Plan**: `git restore --staged -- <path>`; for a staged rename, both names: `git restore --staged -- <new> <old>`.
  Before the first commit there is no `HEAD` to restore the index from: `git rm --cached -r -- <path>`. Several files
  given at once (`Target::Files`) are restored by one command, `git restore --staged -- <path>…`, a staged rename with
  both of its names, titled **Unstage N files**.
- **Danger**: None.
- **Undo**: Not taken back by Undo: ticking the box again (`changes.stage`) does.
- **Uses**: `changes.pathspec`.

Acceptance:

- [ ] Unticking a staged `a.txt` runs `git restore --staged -- a.txt`; the file keeps its changes on disk.
- [ ] Unticking a staged rename of `a` to `b` runs `git restore --staged -- b a`, and the list then has `a` deleted and
      `b` untracked.
- [ ] In a repository with no commit, unticking a staged file runs `git rm --cached -r -- <path>` and the file stays on
      disk, untracked.
- [ ] `a.txt` and the staged rename of `b` to `c` given at once are restored by one
      `git restore --staged -- a.txt c b`, titled **Unstage 2 files**.
- [ ] Extension: unticking `a.ts` runs `git restore --staged -- a.ts` in the engine, written to the **Gitardo** output
      channel; nothing is asked with `gitardo.commandPreview` set to `off`, and with `all` *Unstage a.ts?* shows the
      command first.
- [ ] Extension: unticking the staged rename of `old.txt` to `new.txt` leaves nothing staged, `old.txt` deleted and
      `new.txt` untracked.
- [ ] Extension: unticking `a*.ts` runs `git restore --staged -- :(literal)a*.ts`.

### `changes.stage-all` — Stage all changes

Stages every change at once.

- **Kind**: Operation
- **Function**: `features::changes::stage_all(context, answers) -> Outcome`
- **Entry points**: ticking the box on the header of the list in the Changes page, when it is empty or half ticked;
  **Stage all changes** in the context menu of the draft in the timeline.
- **Extension**: ticking the header box of **Changes** in the Changes view (titled *Click to stage all changes*) while
  no file is in conflict, and **Stage all changes** in the context menu of the uncommitted changes row of the graph,
  send `feature/run` of `changes.stage-all` to Gitardo's engine on the changes (`Controller.runOnEngine`, `engineRuns`
  in `src/engine/actions.ts`); while files are in conflict the header box sends `changes.stage` with the files not all
  staged. The plan is shown in a modal dialog when `gitardo.commandPreview` is `all` (`VscodeAsker`). When the
  engine's binary is missing, `stageAllOp` of `src/git/ops.ts` runs in its place, as before (D49).
- **Offered when**: in the context menu of the row, while some change is not staged and no file is in conflict.
- **Plan**: `git add --all`. While files are in conflict, which `--all` would take as resolved, markers and all, every
  other file that is not all staged is named instead: `git add -- <path>…`.
- **Danger**: None.
- **Undo**: Not taken back by Undo: `changes.unstage-all` does.
- **Uses**: `changes.pathspec`, `graph.uncommitted-row`.

Acceptance:

- [ ] With no conflict, ticking the header box runs `git add --all`, untracked files included.
- [ ] With `a.txt` in conflict and `b.txt`, `c.txt` changed, ticking the header box runs `git add -- b.txt c.txt`, and
      `a.txt` is still in conflict after.
- [ ] The context menu of the draft has no **Stage all changes** while a file is in conflict.
- [ ] Extension: with `clash.ts` in conflict and `a.ts`, `b.ts` changed, the header box runs `git add -- a.ts b.ts`,
      titled *Stage 2 files*, and `clash.ts` stays in conflict.
- [ ] Extension: nothing is asked with `gitardo.commandPreview` set to `off`; with `all`, *Stage all changes?* shows
      `git add --all` first.

### `changes.unstage-all` — Unstage all changes

Takes everything out of the index; the files on disk are not touched.

- **Kind**: Operation
- **Function**: `features::changes::unstage_all(context, answers) -> Outcome`
- **Entry points**: unticking the box on the header of the list in the Changes page, when it is ticked; **Unstage all
  changes** in the context menu of the draft in the timeline.
- **Extension**: unticking the ticked header box of **Changes** in the Changes view (titled *All staged: click to
  unstage all changes*) while no file is in conflict, and **Unstage all changes** in the context menu of the
  uncommitted changes row of the graph, offered while something is staged, also while files are in conflict, send
  `feature/run` of `changes.unstage-all` to Gitardo's engine on the changes (`Controller.runOnEngine`, `engineRuns` in
  `src/engine/actions.ts`): neither ends a merge or a cherry-pick any more. While files are in conflict the header box
  sends `changes.unstage` with the files staged, both names of a rename. The plan is shown in a modal dialog when
  `gitardo.commandPreview` is `all` (`VscodeAsker`). When the engine's binary is missing, `unstageAllOp` of
  `src/git/ops.ts` runs in its place, as before, `git reset` included (D49).
- **Offered when**: in the context menu of the row, while something is staged.
- **Plan**: `git reset`; before the first commit, `git rm --cached -r -- .`. While files are in conflict, whose conflicts
  `git reset` would forget, or while a merge, a rebase, a cherry-pick, a revert or a patch is under way, which
  `git reset` would end, the files that are staged are named instead: `git restore --staged -- <path>…`, a staged
  rename with both of its names.
- **Danger**: None.
- **Undo**: Not taken back by Undo: `changes.stage-all` does.
- **Uses**: `changes.pathspec`, `graph.uncommitted-row`.

Acceptance:

- [ ] With every file staged and no conflict, unticking the header box runs `git reset`, and every box is empty after.
- [ ] With a file in conflict, unticking runs `git restore --staged -- <the staged files>` and the conflict is still
      listed under **Conflicts** after.
- [ ] In a repository with no commit it runs `git rm --cached -r -- .` and every file stays on disk.
- [ ] While a merge waits to be committed with no conflict left, it runs `git restore --staged -- <the staged files>`,
      and the merge is still under way after.
- [ ] Extension: during a merge stopped on conflicts, **Unstage all changes** in the menu of the uncommitted changes
      row runs `git restore --staged -- <the staged files>`; the merge and its conflicts are still there after.
- [ ] Extension: during a merge of `other` with no conflict left and `a.txt`, `b.txt` staged, it runs
      `git restore --staged -- a.txt b.txt`, and `.git/MERGE_HEAD` is still there.
- [ ] Extension: nothing is asked with `gitardo.commandPreview` set to `off`; with `all`, *Unstage all changes?* shows
      the command first.

### `changes.discard` — Discard the changes of a file

Takes one file back to the last commit, staged changes included, or deletes it when it is untracked; several files
given at once are discarded together.

- **Kind**: Operation
- **Function**: `features::changes::discard(context, answers) -> Outcome`
- **Entry points**: **Discard changes** on a file of the Changes page.
- **Extension**: **Discard changes** on a file of the Changes view, which gives the file alone when nothing of it is
  staged and the file with both names of a rename otherwise; **Discard all changes in this file, staged too…** on a
  file of the uncommitted changes in the details of the graph; **Discard all changes in 1 selected file…** or
  **Discard all changes in N selected files…** above those files, once some are ticked. Each sends `feature/run` of
  `changes.discard` to Gitardo's engine (`Controller.runOnEngine`), on the one file or on the files given (the actions
  `discard` and `discardChanges` in `engineRuns`, `src/engine/actions.ts`); its warning is always asked, in a modal
  warning dialog (`VscodeAsker`). **Discard Changes** in the title of the Flow Diff is `diff.discard-file`, still the
  `GitOp` `discardOp` run by `Controller.runIn`. When the engine's binary is missing, `discardOp` and
  `discardChangesOp` of `src/git/ops.ts` run in its place, as before (D49).
- **Offered when**: the file is changed for git.
- **Plan**:
  - Nothing of it staged: `git restore -- <path>` for a tracked file, `git clean -f -- <path>` for an untracked one.
  - Some or all of it staged: `git restore --source=HEAD --staged --worktree -- <path>`, with the old name too for a
    rename, so that the file goes back to it; and `git clean -f -- <path>` for a file that is untracked and not staged.
  - Before the first commit, a staged file has nothing to go back to: `git rm -r --force --quiet -- <path>`, which
    deletes it.
  - Several files given at once (`Target::Files`) are discarded as `changes.discard-all` discards the files of the
    list of the Changes page, those files only, in one plan titled **Discard the changes of N files**, with one warning
    that counts them.
- **Danger**: Destructive, because changes that are not committed are lost for good, and an untracked or added file is
  deleted from disk. The warning says which: "Unstaged changes to tracked files will be lost.", "One untracked file will
  be deleted from disk.", or "One file goes back to the last commit, staged changes included: they will be lost, and a
  file added since is deleted from disk."
- **Undo**: Impossible: changes that were never committed are in no reflog.
- **Refuses**: "There is nothing to discard." when git sees no change in the file any more.
- **Uses**: `changes.pathspec`.

Acceptance:

- [ ] On a modified tracked file with nothing staged, it runs `git restore -- <path>` after the warning, and the file is
      as in `HEAD`.
- [ ] On an untracked file it runs `git clean -f -- <path>` and the file is gone from disk.
- [ ] On a file partly staged it runs `git restore --source=HEAD --staged --worktree -- <path>`; the index and the disk
      both match `HEAD` after.
- [ ] On a staged rename of `a` to `b` the command names `b` and `a`, and `a` is back after.
- [ ] With `commandPreview` set to `off`, it still asks, showing the warning.
- [ ] `a.txt` and `b.txt` modified and `new.txt` untracked, given at once with nothing staged, run
      `git restore -- a.txt b.txt` and `git clean -f -- new.txt`, after one warning, "Unstaged changes to tracked files
      will be lost. One untracked file will be deleted from disk.", titled **Discard the changes of 3 files**.
- [ ] Extension: **Discard changes** on `a.ts`, changed and not staged, asks *Discard the changes of a.ts?* in a modal
      warning dialog with `git restore -- a.ts`, whatever `gitardo.commandPreview` says; **Run** runs it in the engine,
      written to the **Gitardo** output channel; **Cancel** leaves every file as it was.
- [ ] Extension: in the details, ticking `one.txt` and `two.txt`, both untracked, and choosing **Discard all changes in
      2 selected files…** asks *Discard the changes of 2 files?* with `git clean -f -- one.txt two.txt` and "2
      untracked files will be deleted from disk."
- [ ] Extension: a file taken out of the index and left on disk goes back to what `HEAD` has, and is not deleted.
- [ ] Extension: discarding `pages/[id].txt` runs `git restore -- :(literal)pages/[id].txt`, and `pages/i.txt` and
      `pages/d.txt` keep their changes.

### `changes.discard-all` — Discard all changes

Takes every changed file back to the last commit, staged changes included, and deletes the untracked files.

- **Kind**: Operation
- **Function**: `features::changes::discard_all(context, answers) -> Outcome`
- **Entry points**: **Discard all changes** on the header of the list in the Changes page; **Discard all changes…** in
  the context menu of the draft in the timeline.
- **Extension**: **Discard all changes…** in the context menu of the uncommitted changes row of the graph, and above
  the files of the uncommitted changes in the details while none is ticked, both offered while no operation is under
  way, send `feature/run` of `changes.discard-all` to Gitardo's engine on the whole working tree, as from the draft;
  **Discard all changes** on the header of **Changes** in the Changes view sends it on the files of the list when
  nothing is staged, and `changes.discard` with every file of the list, both names of a rename, when something is (the
  actions `discardChanges` and `discardAll` in `engineRuns`, `src/engine/actions.ts`, run by
  `Controller.runOnEngine`). The warning is always asked, in a modal warning dialog (`VscodeAsker`). When the engine's
  binary is missing, `discardChangesOp` and `discardOp` of `src/git/ops.ts` run in its place, as before (D49).
- **Offered when**: in the context menu of the row, while no merge, rebase, cherry-pick, revert or `git am` is in
  progress: there, **Abort** is what goes back.
- **Plan**, from the header of the Changes page, which acts on the files of its list and leaves the files in conflict
  alone:
  - Nothing staged: `git restore -- <path>…` for the tracked files and `git clean -f -- <path>…` for the untracked
    ones, each file named.
  - Something staged: `git restore --source=HEAD --staged --worktree -- <path>…` with every file of the list, renames
    with both names, and `git clean -f -- <path>…` for the untracked files not staged.
  - Before the first commit, the staged files are deleted with `git rm -r --force --quiet -- <path>…`.

  From the draft, which acts on the whole working tree, files in conflict included:
  `git restore --source=HEAD --staged --worktree -- .` when some tracked file changed, then `git clean -f -d -- .` when
  there are untracked files, folders included. Before the first commit, `git rm -r --force --quiet -- .` in place of
  `git restore`.
- **Danger**: Destructive, because every change not committed is lost and untracked files are deleted. The warning
  counts them: "3 files go back to the last commit, staged changes included: they will be lost, and a file added since
  is deleted from disk. 2 untracked files will be deleted from disk."
- **Undo**: Impossible: changes that were never committed are in no reflog.
- **Refuses**: "There is nothing to discard." when nothing is changed; from the row, while an operation is in progress,
  as a key binding may still ask: "Abort the `<operation>` to discard all of it."
- **Uses**: `changes.pathspec`, `graph.uncommitted-row`.

Neither removes the files git ignores: there is no `-x`.

Acceptance:

- [ ] From the header, with `a.txt` modified and `b.txt` untracked, nothing staged, it runs `git restore -- a.txt` and
      `git clean -f -- b.txt`.
- [ ] From the header, with a file staged, it runs one `git restore --source=HEAD --staged --worktree -- …` that names
      every file of the list.
- [ ] From the header, a file in conflict is not named in any of the commands, and is still in conflict after.
- [ ] From the row, with a modified file, a staged file and an untracked folder `tmp/`, it runs
      `git restore --source=HEAD --staged --worktree -- .` and `git clean -f -d -- .`, and `git status` is empty after.
- [ ] The entry of the row is not in its menu while a merge is stopped on conflicts.
- [ ] The files only an editor has changed keep their edits not saved.
- [ ] With `commandPreview` set to `off`, it still asks, showing the warning.
- [ ] Extension: **Discard all changes…** in the menu of the uncommitted changes row asks *Discard all changes?* in a
      modal warning dialog with `git restore --source=HEAD --staged --worktree -- .` and `git clean -f -d -- .`,
      whatever `gitardo.commandPreview` says; after **Run**, `git status --porcelain` prints nothing.
- [ ] Extension: during a merge, neither the menu of the uncommitted changes row nor the details offer **Discard all
      changes…**.
- [ ] Extension: with `a.txt` staged and `b.txt` changed, **Discard all changes** of the Changes view asks *Discard the
      changes of 2 files?* with one `git restore --source=HEAD --staged --worktree -- a.txt b.txt`.
- [ ] Extension: before the first commit, **Discard all changes…** of the row runs `git rm -r --force --quiet -- .`,
      and the staged files are deleted from disk.
- [ ] Extension: **Cancel** leaves every file as it was.

### `changes.save` — Save a file, or all of them

Writes to disk the edits typed in Gitardo's editors and not saved yet, so that git sees them.

- **Kind**: Operation
- **Function**: `features::changes::save(context, answers) -> Outcome`
- **Entry points**: **Save all** on the header of the list, for every file of the working tree with edits not saved
  in a File page. **Save** on a row of the Changes page saves that file through `file-view.save`, which asks before
  writing over a file another program changed.
- **Extension**: **Save** on the row of a file with edits not saved, and **Save all** on the header of **Changes** in
  the Changes view, have VS Code save its own editors of those files (`Controller.save` in
  `src/extension/controller.ts`), as `Ctrl+S` would: no git command runs, and nothing is written to the log.
- **Offered when**: the file has edits not saved (`changes.unsaved`).
- **Plan**: `write <path>` for each file, with the text of its buffer.
- **Danger**: None.
- **Undo**: Not taken back by Undo; the editor of the file still takes the edits back with `Ctrl+Z`.
- **After**: the buffers are marked saved, the dots go, and the files are listed as git now sees them.
- **Uses**: `changes.unsaved`, `file-view.page`, `file-view.save`.

Acceptance:

- [x] **Save** on a file edited in the File page writes its buffer to disk, and its row then has a box that can be
      ticked.
- [x] **Save all** with two unsaved files writes both, and logs two `write` lines.
- [x] A file saved that ends up as in `HEAD` leaves the list.
- [ ] Extension: **Save** on `first.txt` writes the edits of its editor to disk; the dot goes, and `first.txt` is
      listed as changed once the repository is read again.
- [ ] Extension: **Save all** saves every file of the repository listed with a dot, and no other editor.

### `changes.pathspec` — A file named as itself

How a file is named to the git commands that take patterns, so that a name that is also a pattern means only that file.

- **Kind**: Derivation
- **Function**: `model::changes::pathspec(path: &RepoPath) -> OsString`

Rules:

- A path that contains `[`, `]`, `*`, `?` or `\`, or starts with `:`, is written `:(literal)<path>`.
- Any other path is written as it is.
- It applies wherever a file is named to `git add`, `git restore`, `git rm`, `git clean`, `git stash push` and
  `git checkout <commit> -- <path>`. `.` is never rewritten.

Acceptance:

- [ ] `pages/[id].tsx` gives `:(literal)pages/[id].tsx`.
- [ ] `a*.txt`, `what?.md` and `:odd` are prefixed; `src/main.rs` is not.
- [ ] Staging `pages/[id].tsx` in a repository that also has `pages/i.tsx` and `pages/d.tsx` changed leaves those two
      unstaged.

### `commit.create` — Commit

Makes a commit of what is staged, with the message of the commit box.

- **Kind**: Operation
- **Function**: `features::commit::create(context, answers) -> Outcome`
- **Entry points**: **Commit** in the draft and under the message in the Changes page; `Ctrl+Enter` (`Cmd+Enter` on
  macOS) in the message box.
- **Extension**: **Commit** in the commit box of the Changes view, and `Ctrl+Enter` (`Cmd+Enter`) in its message, send
  `feature/run` of `commit.create` to Gitardo's engine with the message typed as the answer `message` (`engineRuns` in
  `src/engine/actions.ts`, run by `Controller.runOnEngine`). The question when nothing is staged is a modal warning
  dialog with **Stage all and commit** (`VscodeAsker` in `src/engine/vscode.ts`), and the plan is shown in a modal
  dialog when `gitardo.commandPreview` is `all`; once it ran, the engine's follow-up `emptyCommitMessage` empties the
  commit box (the message `committed`). The message the box starts from is the extension's own: after a squash merge,
  what `.git/SQUASH_MSG` says (`Repo.squashMessage`), with the message *Gitardo: the changes are staged, and nothing
  is committed yet. Commit them to make them one commit on main.* and **Show Changes** and **Undo**; after a split,
  the message of the commit split (`Repo.commitMessage`), with *The changes of "\<subject\>" are in the working tree,
  unstaged: commit them in as many commits as you like.* and **Undo** after a split of the last commit; both posted as
  `commitMessage` by `Controller.reportSquashed` and `Controller.reportSplit` (`src/extension/controller.ts`), which
  still run in TypeScript with the merge and the split. When the engine's binary is missing, `commitOp` of
  `src/git/ops.ts` runs in its place, as before (D49).
- **Offered when**: **Amend last commit** is not ticked, the message is not empty and something is changed; otherwise
  the button is disabled.
- **Asks**: when nothing is staged and some change is not: a confirmation, "There are no staged changes. Stage all
  changes and commit them?", with **Stage all and commit**.
- **Plan**: `git commit -m <message>`, the message without the spaces and blank lines around it; after the
  confirmation, `git add --all` first.
- **Danger**: None.
- **Undo**: By the reflog: Undo moves the branch back to the commit before, with `git reset --soft`, and the changes of
  the commit stay staged (`undo.last`).
- **After**: the message box is emptied and **Amend last commit** unticked; the timeline selects nothing new.
- **Refuses**: "Enter a commit message." with an empty message; "There are no changes to commit." when nothing is
  staged and nothing else is changed.
- **Uses**: `changes.stage-all`.

When the repository has prepared a message, such as the one git writes for a squash merge (`merge.`) or the message of a
commit being split (`split.`), the message box starts from it while it is empty; what the user wrote is never replaced.

Acceptance:

- [ ] With `a.txt` staged and the message `Fix the parser`, it runs `git commit -m "Fix the parser"`, and the timeline
      has the new commit at the top after.
- [ ] With nothing staged and `a.txt` modified, it asks, and after **Stage all and commit** runs `git add --all` then
      `git commit -m …`; untracked files are in the commit.
- [ ] Declining the confirmation runs nothing and keeps the message.
- [ ] With the message `  \n  ` the button is disabled.
- [ ] After the commit the message box is empty.
- [ ] A squash merge that stopped to be committed fills the empty message box with git's prepared message.
- [ ] Extension: with `b.txt` staged, typing `Commit from the UI` and pressing **Commit** runs
      `git commit -m "Commit from the UI"` in the engine, written to the **Gitardo** output channel, and
      `git log -1 --format=%s` prints `Commit from the UI`; what is not staged stays out of the commit and stays
      listed.
- [ ] Extension: nothing is asked with `gitardo.commandPreview` set to `off`; with `all`, *Commit "Commit from the
      UI"?* shows the command first.
- [ ] Extension: with nothing staged and `a.txt` changed, **Commit** asks *There are no staged changes. Stage all
      changes and commit them?*; **Stage all and commit** runs `git add --all` and then `git commit -m "<message>"`.
- [ ] Extension: a message with a subject, a blank line and a body of several lines is committed exactly as typed,
      trimmed at its ends.
- [ ] Extension: right after, **Undo** asks first and then runs `git reset --soft` to the commit before: the commit is
      gone and its changes are staged.
- [ ] Extension: after **Split this commit…** on `Add the login page`, the empty box holds its whole message; when the
      box already has text other than spaces, that text stays.
- [ ] Extension: when git refuses, such as a hook that fails, an error message says why with **Show Log**, and the
      message stays in the box.

### `commit.amend` — Amend the last commit

Replaces the last commit with one that has the staged changes too, and the new message if one is written.

- **Kind**: Operation
- **Function**: `features::commit::amend(context, answers) -> Outcome`
- **Entry points**: **Amend** in the Changes page, which **Commit** becomes while **Amend last commit** is ticked
  there, the box being on the Changes page only; `Ctrl+Enter` (`Cmd+Enter` on macOS) in its message box then.
- **Extension**: ticking **Amend last commit** in the commit box of the Changes view, then **Amend** or `Ctrl+Enter`,
  sends `feature/run` of `commit.amend` to Gitardo's engine with the message typed, empty to keep the one the commit
  has, as the answer `message` (`engineRuns` in `src/engine/actions.ts`, run by `Controller.runOnEngine`); the plan is
  shown in a modal dialog when `gitardo.commandPreview` is `all` (`VscodeAsker`). When the engine's binary is missing,
  `commitOp` of `src/git/ops.ts` runs in its place, as before (D49).
- **Offered when**: there is a commit: the box **Amend last commit** is disabled before the first one. Nothing needs to
  be staged.
- **Plan**: `git commit --amend -m <message>`; with the message box empty, `git commit --amend --no-edit`, which keeps
  the message the commit has.
- **Danger**: None.
- **Undo**: By the reflog: Undo takes the branch back to the commit as it was before the amend, with `git reset --soft`,
  and what the amend added stays staged (`undo.last`).
- **After**: the message box is emptied and **Amend last commit** unticked.

The tooltip of the box says what it does: **Replace the last commit instead of adding a new one. Leave the message empty
to keep it.**

Acceptance:

- [ ] With nothing staged and the message `Better words`, it runs `git commit --amend -m "Better words"`, and `HEAD`
      has the new message and the same files.
- [ ] With `b.txt` staged and an empty message, it runs `git commit --amend --no-edit`; `HEAD` has `b.txt` and its old
      message.
- [ ] In a repository with no commit the box **Amend last commit** cannot be ticked.
- [ ] Extension: with nothing staged and the message empty, **Amend** is enabled and runs
      `git commit --amend --no-edit` in the engine.
- [ ] Extension: when the last commit is on its upstream already, nothing is asked with `gitardo.commandPreview` set
      to `off`, and nothing says the branch will need a force push.
- [ ] Extension: right after, **Undo** asks first and then puts back the commit as it was before the amend.

### `commit.prepared-message` — The message git prepared

The message git wrote for the commit of a squash merge while its changes wait to be committed, which fills the commit
box after `merge.squash` (`FollowUp::PreparedMessage`, Q398).

- **Kind**: Read
- **Function**: `git::read::commit::read_prepared_message(git: &Git, request: &PreparedMessageRequest) -> Result<Option<String>, GitError>`
- **Reads**: `git rev-parse --show-toplevel --absolute-git-dir` for the git folder of the worktree, then the file
  `SQUASH_MSG` in it, without the line breaks and spaces at its end. No file, or one with only blanks, is no message.
- **Extension**: the commit box of the Changes view; the extension kept the line breaks at the end of the message.

Acceptance:

- [ ] After `git merge --squash`, it is the message git wrote, starting with "Squashed commit of the following:", without
      the line breaks at its end.
- [ ] With no squash merge waiting, there is none.
- [ ] A linked worktree has its own: a squash merge waiting in the main worktree gives no message in another one.
- [ ] A `SQUASH_MSG` of blank lines is no message.

### `commit.draft-message` — Draft the commit message

Asks the AI provider to write the commit message from the changes, and puts its answer in the message box, to edit.

- **Kind**: Derivation
- **Function**: `model::commit::draft_message(input: &DraftInput) -> AiRequest`, and
  `model::commit::clean_draft(answer: &str) -> String`
- **Entry points**: the sparkle button beside **Commit** in the Changes page, and not in the draft, whose tooltip says
  **Draft a message from the changes with the AI provider**.
- **Extension**: the sparkle button of the commit box of the Changes view, titled *Draft a message from the changes
  with the language model set up in VS Code*. `Controller.draftMessage` in `src/extension/controller.ts` asks the
  engine `read/commit.draft-message` with the subjects of the graph: the engine reads the changes and the diff
  (`commit.read-draft-diff`), writes the request and cleans the answer, and asks VS Code's language models in place of
  the AI provider with `host/languageModel`, which `answerLanguageModel` in `src/extension/ai.ts` sends to the first
  model `vscode.lm.selectChatModels()` gives, with no effort, and with the justification *Gitardo sends your uncommitted
  changes to the model to draft a commit message.* (G3: not the model of `gitardo.conflicts.aiModel`). Without the
  engine, `draftCommitMessage` in `src/extension/ai.ts` drafts it in TypeScript the same way (`buildPrompt`,
  `cleanMessage` and `MAX_DIFF_CHARS` in `src/shared/commitPrompt.ts`, the diff read by `Repo.diffText`). The status
  bar says *Gitardo: drafting commit message* while it runs. With no model, it refuses with *Gitardo: No language
  model is available in VS Code. Install an extension that provides one, such as GitHub Copilot.*
- **Offered when**: something is changed; the button is disabled while a draft is being written.
- **Reads**: the staged changes, `git diff --cached --no-color --no-ext-diff`, when something is staged; otherwise all
  changes, `git diff --no-color --no-ext-diff`, with the paths of the untracked files; and the subjects of the last 10
  commits as the timeline lists them, without the draft and without stashes.
- **Settings**: `ai.provider`, `ai.endpoint`, `ai.model` and `ai.effort`, the same as for the conflicts, with the key in
  the credential store of the system.
- **Uses**: `ai.choose-model`, `changes.read-working-tree`.

The request goes to the AI provider over HTTP as those settings say. It is one message from the user, with these parts,
separated by lines:

1. "Write a Git commit message for the changes below." "First line: a summary of at most 72 characters, in the
   imperative mood, with no trailing period." "If the change needs explaining, add a blank line and a short body that
   says what changed and why." "Reply with the commit message only: no code fences, no quotes, no commentary."
2. When there are subjects: "Match the style of the recent commits in this repository:" and each subject on a line of
   its own after `- `.
3. When untracked files are sent: "New files that are not in the diff:" and each path after `- `.
4. "Diff:" and the diff, or, when the diff is longer than 20,000 characters, "Diff (truncated):" and its first 20,000
   characters.

The answer is cleaned: spaces and blank lines around it are taken off, and a code fence the model put around it anyway,
a first line of three backticks with or without a language and a last line of three backticks, is removed. The cleaned
answer replaces the text of the message box. Nothing is sent until the button is pressed. While the request runs, the
activity indicator of the status bar says **drafting commit message** and it can be cancelled there; cancelled or failed,
the message box is left as it was.

Refuses: "There are no changes to describe." with nothing changed; with `ai.provider` empty, "No AI provider is set
up." with **Open settings** at `ai.provider`.

Acceptance:

- [ ] With `a.txt` staged and `b.txt` modified but not staged, the request has the diff of `a.txt` only and no list of
      new files.
- [ ] With nothing staged and `new.txt` untracked, the request has the diff of the tracked files and `- new.txt` under
      "New files that are not in the diff:".
- [ ] A diff of 50,000 characters is sent as its first 20,000, under "Diff (truncated):".
- [ ] The answer "```text\nAdd the parser\n```" puts `Add the parser` in the message box.
- [ ] No HTTP request is made before the button is pressed.
- [ ] Cancelling while it runs leaves the message box as it was.
- [ ] Extension: nothing is sent to a model until the sparkle button is pressed.
- [ ] Extension: with no model in VS Code, the error above is shown and the box keeps what it had.
- [ ] Extension: nothing in the repository changes, and nothing is written to the log of operations.
- [ ] Extension: with the engine, the request VS Code's model is sent is the one the extension's TypeScript writes for
      the same changes, and the same answer gives the same message.

### `commit.read-draft-diff` — The diff a draft is written from

The diff `commit.draft-message` sends to the model: of what is staged when something is, as **Commit** would commit
it, and of every change of the tracked files otherwise (Q398).

- **Kind**: Read
- **Function**: `git::read::commit::read_draft_diff(git: &Git, request: &DraftDiffRequest) -> Result<String, GitError>`
- **Reads**: `git diff --cached --no-color --no-ext-diff` when `is_staged`, `git diff --no-color --no-ext-diff`
  otherwise, as git prints it, bytes that are not UTF-8 replaced. An untracked file is in neither; the draft names
  those apart.
- **Extension**: the engine reads it for `read/commit.draft-message`; the extension read it with `Repo.diffText`
  (`src/git/repo.ts`), the same commands.

Acceptance:

- [ ] With `a.txt` staged and `b.txt` changed and not staged, the staged diff has `a.txt` and not `b.txt`.
- [ ] With nothing staged, the diff has `a.txt` and `b.txt`, and not the untracked `new.txt`.
- [ ] With `color.diff` set to `always` and `diff.external` set to a program, the diff has no colors and is git's own.

### `identity.read` — Read the identity

Who commits will be made as in this repository, and which of the `user.*` keys the repository sets for itself.

- **Kind**: Read
- **Function**: `git::read::identity::read_identity(git: &Git, request: &IdentityRequest) -> Result<IdentityState, GitError>`
- **Reads**: `git config -z --get-regexp ^user\.(name|email)$` for the name and the email git will use, wherever they
  are set; `git config -z --local --get-regexp ^user\.` for the keys the repository has of its own. Git ends with code 1
  when there is none, which is an empty result, not an error.

Acceptance:

- [ ] With `user.name` and `user.email` set only in the global configuration, the identity has them, and the local keys
      are empty.
- [ ] With `user.signingkey` set in the repository, the local keys include `user.signingkey`.
- [ ] With nothing set anywhere, both are empty and no error is shown.

### `identity.switch` — Switch identity

Sets the name and email that commits are made with in this repository, from a profile of the settings or typed.

- **Kind**: Operation
- **Function**: `features::identity::switch(context, answers) -> Outcome`
- **Entry points**: **Switch identity** in Quick search; **Switch identity…** in the **…** menu of the header; a click
  on the identity under the commit box in the Changes page.
- **Extension**: **Gitardo: Switch Identity** in the Command Palette, **Switch identity…** in the **…** menu of the
  toolbar of the graph, and a click on the identity under the commit box of the Changes view. Each sends `feature/run` of `identity.switch` to Gitardo's engine (`Controller.runOnEngine` in
  `src/extension/controller.ts`, `Controller.switchIdentity`); the pick is a quick pick, and the name and
  email input boxes. **Edit the list of identities…** is added to the pick by the extension, which opens the settings at
  `gitardo.identities` and runs nothing (`HostHooks.choices`, B9). When the engine's binary is missing,
  `Controller.switchIdentity` asks the same in TypeScript, as before (D49).
- **Asks**:
  1. A pick, titled **Commit in `<repository>` as**, with the identity used now as its placeholder
     (**Now: `Name <email>`**, or **No identity is configured**): each profile of `identities`, with its email beside
     its name and **In use** under the one that matches; **Enter a name and email…**; **Use the global identity**, only
     when the repository has `user.*` keys of its own (`identity.use-global`); **Edit the list of identities…**, which
     opens the settings page at `identities` and runs nothing.
  2. After **Enter a name and email…**: the name, then the email, each starting from the one used now. Leaving either
     empty runs nothing.
- **Reads**: `identity.read`, for the identity used now and the keys the repository has of its own.
- **Plan**: `git config user.name <name>`, `git config user.email <email>`, then
  `git config user.signingkey <key>` for a profile that has `signingKey`; for one that has none, when the repository has
  a `user.signingkey` of its own, `git config --unset-all user.signingkey`, so that commits are not signed as someone
  else. The name and email without the spaces around them.
- **Danger**: None.
- **Undo**: Not taken back by Undo: switching again, or `identity.use-global`, does.
- **Refuses**: a name, email or key that starts with `-`, which git would read as an option: "`<value>` is not a valid
  name."
- **Settings**: `identities`, a list of `{ "name", "email", "signingKey"? }`; an entry without a name or an email that is
  text is left out of the pick.

The confirmation, when one is shown, says: "This is saved in the configuration of this repository only."

Acceptance:

- [ ] Picking the profile `{ "name": "Ada", "email": "ada@home.example" }` in a repository with a `user.signingkey` of
      its own runs `git config user.name Ada`, `git config user.email ada@home.example` and
      `git config --unset-all user.signingkey`.
- [ ] Picking a profile with `"signingKey": "ABCD1234"` runs `git config user.signingkey ABCD1234` as its third command.
- [ ] Without a local `user.signingkey`, a profile with no key runs only the two commands.
- [ ] The global configuration is never written: no command has `--global`.
- [ ] The identity under the commit box shows the new one once the repository is read again.
- [ ] **Use the global identity** is not in the pick when the repository has no `user.*` key of its own.
- [ ] Extension: **Gitardo: Switch Identity**, **Enter a name and email…**, `Other Me` and `other@gitardo.invalid` make
      them the identity of the repository.
- [ ] Extension: **Edit the list of identities…** opens the settings at `gitardo.identities` and runs nothing.

### `identity.use-global` — Use the global identity again

Removes the identity the repository set for itself, so that the one of the global configuration applies again.

- **Kind**: Operation
- **Function**: `features::identity::use_global(context, answers) -> Outcome`
- **Entry points**: **Use the global identity**, described **Remove this repository's own setting**, in the pick of
  `identity.switch`.
- **Offered when**: the repository has at least one of `user.name`, `user.email`, `user.signingkey` of its own.
- **Reads**: `identity.read`, for the keys the repository has.
- **Plan**: `git config --unset-all <key>` for each of `user.name`, `user.email` and `user.signingkey` that the
  repository has, in that order.
- **Danger**: None.
- **Undo**: Not taken back by Undo: `identity.switch` sets one again.
- **Refuses**: "This repository already uses the global identity." when it has none of the three.

Acceptance:

- [ ] With `user.name` and `user.email` set in the repository, it runs `git config --unset-all user.name` and
      `git config --unset-all user.email`, and the identity under the commit box is the global one after.
- [ ] A `user.signingkey` of the repository is unset too.
