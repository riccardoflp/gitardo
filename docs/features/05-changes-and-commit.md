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
  `ai.resolve-conflicts`, `repo.select`, `diff.page`, `merge-view.page`, `changes.beside-file`.

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
4. **Conflicts**, when there are files in conflict, with their count: each file with no box and no button. What
   resolves it is in the menu of its row (`changes.row-menu`): **Resolve with AI, after looking at what is proposed**
   (only when `ai.*` is set up for conflicts, `ai.resolve-conflicts`), **Resolve its conflicts automatically, after
   looking at what is proposed** (only when the rules can resolve some of them, `conflicts.resolve-by-rules`), **Mark
   as resolved** (`conflicts.mark-resolved`), **Take the incoming side for the whole file** and **Take the current
   side for the whole file** (`conflicts.take-side`). A click on one opens it in the Merge page.
5. **Changes**, with their count: the box on its header (`changes.stage-all`, `changes.unstage-all`), then the buttons
   **View as tree** or **View as list**, **Open all changes in one Diff page**, **Stash only the staged changes…**
   (only while something is staged), **Stash all changes…**, **Save all** (only while a file has edits not saved) and
   **Discard all changes** (`changes.discard-all`). When the heading is too narrow for them, as in a column, those
   that fit stay and **…** (**More for all changes**) at its right has the others in a menu; they leave in this
   order: the layout, **Stash only the staged changes…**, **Save all**, **Stash all changes…**, **Discard all
   changes**, and **Open all changes in one Diff page** last (Q762). Each file has its box and its name, with a dot
   after it when it has edits not saved, and no button, so that the name has the width of the row: what is done to
   the file is in the menu of its row (`changes.row-menu`). A file that only an editor has changed has its box greyed
   out, with the tooltip **Not saved: save it to stage it**.
6. **No changes** when there is nothing in either list.

A file opened from the page, its diff or the file itself, shows on the right of the page, which stays on the left as a
column with the file marked (`changes.beside-file`); the Changes view of the bar of views shows the whole page again.

With no repository the page says **No Git repository in this workspace** with **Check again**, which reads the folders
of the workspace again. While an operation runs, the buttons and the items of the menus that change something are
disabled.

**View as tree** shows the files under their directories, which fold; **View as list** shows them one after the other,
each with its directory beside its name. The choice is kept for the page, and the folds while the window is open. Each
file has the icon of its kind, from Symbols (`file-view.file-icons`).

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
- [ ] The menu of a deleted file has no **Open file**.
- [ ] In a column too narrow for the buttons of the heading of **Changes**, those that do not fit are in **…** at
      its right, in the order they leave, and **Open all changes in one Diff page** is the last on the heading.
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
  the files git ignores asked of the engine (`reads.ignored` in `src/engine/reads.ts`), and told to the Changes
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

Decides which page, and which two versions, a click on a file of the Changes page, or **Open changes** and **Open
file** of its menu (`changes.row-menu`), opens.

- **Kind**: Derivation
- **Function**: `model::changes::open(input: &OpenInput) -> PageRequest`
- **Settings**: `changes.openOnClick`: `diff`, the default in Gitardo Desktop
  ([D71](../08-decisions.md#d71--changes-a-section-of-the-side-column-whose-files-open-their-diffs)), or `file`;
  `gitardo.changes.openOnClick`, `file` by default, in the extension (Q763). It changes only what a click on a row
  does.
- **Uses**: `diff.page`, `file-view.page`, `merge-view.page`, `changes.beside-file`.
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

- **Open changes** opens a Diff page, beside the list (`changes.beside-file`): the index against the file on disk while
  some of the file is not staged; the last commit against the index once all of it is. A file added has nothing on its
  left side; a file deleted has nothing on its right side.
- **Open file** opens the file in the File page, as it is on disk, beside the list. A deleted file opens its Diff page
  instead.
- A click on a row does what **Open changes** does, so that a review goes from diff to diff, or, with
  `changes.openOnClick` set to `file`, what **Open file** does.
- A click on a file in conflict opens its Merge page.
- **Compare with the saved file**, on a file that only an editor has changed, opens a Diff page with the file as it is
  on disk on the left and the buffer not saved on the right, editable.
- **Open all changes in one Diff page** opens one Diff page, beside the list, with the diffs of every file of the list,
  one after the other, each between the last commit and the file on disk, titled **Changes**: a Diff page has one pair
  of versions for all its files (Q342). The rows of files only an editor has changed are left out.

Acceptance:

- [x] A file partly staged opens with the index on the left and the file on disk on the right.
- [x] A file all staged opens with `HEAD` on the left and the index on the right.
- [x] With `changes.openOnClick` not set, or set to `diff`, a click on a row opens the same Diff page as **Open
      changes**; with `file`, the File page.
- [x] A click on a deleted file opens its Diff page whatever the setting says.
- [x] **Open all changes in one Diff page** with three changed files opens one Diff page with three files.
- [ ] Extension: **Open all changes in one editor** with `a.txt` wholly staged and `b.txt` with changes not staged
      opens one editor titled **Changes**, with `HEAD` against the index for `a.txt` and the index against the file
      on disk for `b.txt`.
- [ ] Extension: a click on a file only an editor has changed opens that editor; **Compare with the saved file** opens
      VS Code's comparison of the file on disk with what its editor has, not saved.
- [ ] Extension: a click on a file in conflict that has its base and both sides opens it in the merge editor, whatever
      `gitardo.changes.openOnClick` says.
- [ ] Extension: with `gitardo.changes.openOnClick` not set, a click on a file of the Changes view opens the file.

### `changes.beside-file` — The Changes page beside what it opens

A file of the Changes page opens its Diff page or its File page on the right of the list of changes, which stays on
the left as a column, so that a review goes from file to file without coming back to the list
([D57](../08-decisions.md#d57--worktrees-in-the-side-column-pages-that-keep-what-opened-them-the-way-back-in-the-title)),
as the Files page keeps its tree (`file-view.beside-files`).

- **Kind**: View
- **Function**: `views::changes::beside_file_view(state: &BesideFileState, ui) -> Vec<Intent>`
- **Entry points**: a click on a file of the Changes page or of the **Changes** section of the side column
  (`changes.section`), **Open changes** and **Open file** of its menu (`changes.row-menu`), and **Open all changes in
  one Diff page**, as `changes.open` decides; `Alt+Down` and `Alt+Up` in the page opened so, for the next and the
  previous file of the list.
- **Uses**: `changes.pane`, `changes.section`, `changes.open`, `diff.page`, `file-view.page`, `graph.resize-panes`.
- **Extension**: VS Code keeps its Source Control view beside its editors: nothing to build.

- The Changes page is split in two: the page itself on the left as a column, its commit box, its identity and its
  lists as they are on the whole page with narrower margins, 360 logical pixels wide where it starts, dragged as
  `graph.resize-panes` says and remembered for the workspace (`changesColumn`); and the Diff page or the File page of
  the file opened on its right. The side column stays, as beside every page.
- The row of the file shown is marked, as a row selected, and brought into sight each time another file is shown.
  Staging, unstaging, discarding and stashing are done from the column as on the whole page, and the column follows
  the working tree as the whole page does.
- Another file of the list opens in the same place: one Diff page shows each diff in turn, and `Ctrl+Tab` lists no page
  for it; a File page replaced while it has edits not saved stays open in its own page, listed in `Ctrl+Tab`, as
  beside the Files page.
- `Alt+Down` and `Alt+Up` show the next and the previous file of the list, in the order the list shows them, folder
  by folder in **View as tree**: its diff while a diff is shown, the file itself while a File page is; from every file,
  the first. The last file stays shown on `Alt+Down`, the first on `Alt+Up`.
- Once the repository is read again, the place follows its file (Q550): a diff of one file shows what **Open changes**
  gives it now, so that a file staged wholly while the index against the file on disk was shown shows the last commit
  against the index; a file no longer in the list, discarded or committed, leaves the place, and the whole page is
  shown; a diff of every file shows the files listed now, and the whole page once there are none. Another repository
  shown leaves the place too.
- The title row starts with the button of the side column (`repo.side-column`), at the same place as on the whole
  page, and its trail reads `Timeline › Changes › a.txt` (`ui.page-trail`): **Changes** shows the whole page again, as
  `Alt+Left` does and as the Changes view of the bar of views, **Show changes** in Quick search and **Review** in the
  draft do (Q551). The File page or the Diff page beside the list is forgotten then, unless it has edits not saved. A
  page opened from the page beside the list names the Changes page by the name of that page, and that name goes back
  to the Changes page as it was left, with the same file beside its list.
- While the side column shows **Changes** open, the list is there, with the row of the file shown marked, and the
  page draws the file alone, in the middle of the window
  ([D71](../08-decisions.md#d71--changes-a-section-of-the-side-column-whose-files-open-their-diffs)); with the side
  column put away, or the section folded, it draws its own column of the list as above.
- In the narrow layout the page of the file takes the whole page, and **Changes** in the trail comes back to the list.
- A file in conflict still opens its Merge page in place of the Changes page.

Acceptance:

- [x] **Open changes** on `a.txt` shows its diff on the right, and the list is still on the left with `a.txt` marked.
- [x] **Open file** on `a.txt` shows the file in the same place; a click on `b.txt`, with `changes.openOnClick` set to
      `diff`, then shows its diff there, and `Ctrl+Tab` lists the Changes page alone.
- [x] Ticking the box of `a.txt` while its diff of the index against the file on disk is shown stages it, and the place
      then shows the last commit against the index.
- [x] Discarding the changes of `b.txt` while its diff is shown leaves the whole page, with `b.txt` gone from the list.
- [x] `Alt+Down` from `a.txt` shows `b.txt`, then `src/c.txt`, which stays on another `Alt+Down`; `Alt+Up` goes back
      to `b.txt`, in the same Diff page.
- [x] In **View as tree**, `Alt+Down` from `src/c.txt` shows `a.txt`, which the tree lists after the folder `src`.
- [x] **Changes** in the trail, and the Changes view of the bar of views, show the whole page again.
- [x] With the diff of `a.txt` beside the list, the title row starts with the button of the side column where it is on
      the whole page, then **Changes**; `Alt+Left` shows the whole page again.
- [x] The line between the column and the page dragged 60 pixels to the right makes the column 60 pixels wider.
- [x] In a window narrower than the narrow layout, the diff is shown alone, with no column.
- [x] **Open all changes in one Diff page** shows every file beside the list with no row marked; `Alt+Down` then shows
      the first.
- [x] While the side column shows **Changes** open, a file opened from the page is drawn alone where the page was; with
      the section folded, beside the column of the page.

### `changes.section` — Changes, a section of the side column

The changes of the repository shown, in the side column between **Repositories** and **Branches**, so that what is
not committed is in sight beside the timeline and beside every page, and a file of it opens in the middle of the
window ([D71](../08-decisions.md#d71--changes-a-section-of-the-side-column-whose-files-open-their-diffs)).

- **Kind**: View
- **Function**: `views::changes::section_view(state: &ChangesState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the side column (`repo.side-column`), under **Repositories**; a click on its heading folds and
  unfolds it.
- **Uses**: `repo.side-column`, `changes.pane`, `changes.list`, `changes.unsaved`, `changes.row-menu`,
  `changes.open`, `changes.beside-file`, `commit.create`, `commit.amend`, `commit.draft-message`, `identity.switch`,
  `conflicts.banner`, `graph.resize-panes`.
- **Extension**: VS Code's Source Control side bar is the column, and **Gitardo Changes** a view of it
  (`changes.pane`): nothing to build.

- Its heading reads `CHANGES` and how many files are changed, those in conflict counted with them, `CHANGES 3`;
  `CHANGES` alone when there are none and while no repository is shown (Q769). It has the chevron of the other
  headings, and a line above it whatever is folded; the screen reader hears the count as the value of the heading.
- Under the heading the section shows what the Changes page shows, with the margins of a column: the banner of
  another worktree and of an operation that stopped, the commit box with its message, the sparkle, **Commit** or
  **Amend**, **Amend last commit** and the identity (Q761), **Conflicts**, and **Changes** with the box on its
  heading and the files, each with its box and no button (`changes.row-menu`). The buttons of the heading of the
  list that the width of the column does not hold are in **…** at its right (`changes.pane`). What is typed, ticked
  and folded there is what the page has: they are one commit box and one list, shown in two places, both drawn while
  the Changes page is shown with the section open (Q774). The keyboard that **Show changes** in Quick search puts in
  the message box goes to the page, not to the section.
- While no repository is shown, none in the workspace or the one shown being read, the section is its heading over
  an empty room.
- A click on a file shows its diff in the middle of the window, where the timeline or the page was, the side column
  staying, and the row of the file is marked (`changes.open`, `changes.beside-file`); **Open file** of its menu shows
  the file there. `Alt+Down` and `Alt+Up` then show the next and the previous file.
- The section starts 280 logical pixels high, its heading included, or as tall as what it shows when that is less
  (Q768); the lines around it are dragged as `repo.side-column` says, and its height and whether it is folded are
  remembered for the workspace (`changesSection`, `changesFolded`).
- In the narrow layout it is in the side column opened over the timeline (Q771).

Acceptance:

- [x] With `a.txt` changed, the side column has **Changes** between **Repositories** and **Branches**, its heading
      reads `CHANGES 1`, and `a.txt` is listed under it with its box.
- [x] With no repository in the workspace, the section is its heading alone, with nothing under it.
- [x] Ticking the box of `a.txt` in the section stages it, and a message typed there with **Commit** makes the
      commit.
- [x] A click on `a.txt` in the section, the timeline shown, shows its diff where the timeline was, with the side
      column beside it and the row of `a.txt` marked.
- [x] Folding **Changes** leaves its heading alone, and it is folded after the window opens again.
- [x] In a window 800 logical pixels wide the section is in the side column opened over the timeline.
- [x] The side column with its three sections matches its snapshots in each theme.
- [x] `Alt+Down` and `Alt+Up` after a click in the section show the next and the previous file, and **Open file** of
      the menu of a row of the section shows the file.
- [x] With the Changes page shown too, both list the files, and a box ticked in either stages its file and shows
      ticked in both.

### `changes.row-menu` — The menu of a file of the changes

What is done to one file of the changes, in the context menu of its row: the rows have no buttons, so that a name is
read whole in a narrow column, and keep their box
([D71](../08-decisions.md#d71--changes-a-section-of-the-side-column-whose-files-open-their-diffs)).

- **Kind**: View
- **Function**: `views::changes::row_menu(state: &RowMenuState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the right button on the row of a changed file or of a file in conflict, in the Changes page
  (`changes.pane`) and in the **Changes** section of the side column (`changes.section`); the menu key on a row that
  has the keyboard, which `Tab` gives it (Q765, Q772).
- **Offered when**: the row is of a file; the headings and the folders of the tree have no menu.
- **Uses**: `changes.open`, `changes.stage`, `changes.unstage`, `changes.discard`, `stash.push-file`, `changes.save`,
  `conflicts.mark-resolved`, `conflicts.take-side`, `review.page`, `ai.resolve-conflicts`, `ui.menu-icons`.
- **Extension**: the rows of the **Gitardo Changes** view have these actions as buttons beside the name
  (`changes.pane`); whether they go into a context menu there too is Q764.

- A changed file: **Open changes**, **Open file** (not on a deleted file); then **Stage**, or **Unstage** on a file
  wholly staged (Q770), **Discard changes**, **Stash this file…**; then **Save**, only on a file with edits not
  saved.
- A file only an editor has changed: **Save** and **Compare with the saved file**.
- A file in conflict: **Resolve with AI, after looking at what is proposed** (only when `ai.*` is set up for
  conflicts), **Resolve its conflicts automatically, after looking at what is proposed** (only when the rules can
  resolve some of them), **Mark as resolved**, **Take the incoming side for the whole file** and **Take the current
  side for the whole file**.
- Each item does what the button of the same name did, and draws the icon that button drew (`ui.menu-icons`). While
  an operation runs, the items that change something cannot be chosen.
- The row takes the keyboard: `Enter` does what a click on it does, and the menu key opens its menu at the row.

Acceptance:

- [x] The row of a changed file has its box and no button.
- [x] The menu of a changed file has **Open changes**, **Open file**, **Stage**, **Discard changes** and **Stash
      this file…**, each with an icon, and **Unstage** in place of **Stage** on a file wholly staged.
- [x] **Stage** stages the file, **Discard changes** asks and discards it, **Stash this file…** stashes it, **Open
      file** shows the file and **Open changes** its diff.
- [x] The menu of a deleted file has no **Open file**; the menu of a file with edits not saved has **Save**.
- [x] The menu of a file in conflict has **Mark as resolved** and the two sides, and **Mark as resolved** stages it.
- [x] With the keyboard on the row of a file, the menu key opens its menu, and `Enter` on **Stage** stages the file.

### `changes.stage` — Stage a file

Puts the changes of a file, or of several files given at once, in the index, for the next commit.

- **Kind**: Operation
- **Function**: `features::changes::stage(context, answers) -> Outcome`
- **Entry points**: ticking the box of a file that is empty or half ticked, in the Changes page.
- **Extension**: ticking the box of a file in the Changes view (titled *Click to stage*, or *Partly staged: click to
  stage all of it*); **Mark as resolved** on a file in conflict there; ticking the header box while files are in
  conflict, which gives the files not all staged. Each sends `feature/run` of `changes.stage` to Gitardo's engine
  (`Controller.runOnEngine` in `src/extension/controller.ts`), on the one file or on the files given (`engineRuns` in
  `src/engine/actions.ts`); the plan is shown in a modal dialog when `gitardo.commandPreview` is `all` (`VscodeAsker` in
  `src/engine/vscode.ts`). **Stage File** in the title of the Flow Diff sends `changes.stage` on its file, through
  `Controller.runIn` (`src/flow/flowDiff.ts`). When the engine's binary is missing, it says so (D49).
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
- [ ] Extension: with `gitardo.engine.path` naming a missing file, the Changes view says **Gitardo's engine is not
      available on this system.** with **Show the log**, and nothing of this file runs.

### `changes.unstage` — Unstage a file

Takes the changes of a file, or of several files given at once, out of the index; the files on disk are not touched.

- **Kind**: Operation
- **Function**: `features::changes::unstage(context, answers) -> Outcome`
- **Entry points**: unticking the box of a file that is ticked, in the Changes page.
- **Extension**: unticking the ticked box of a file in the Changes view (titled *Staged: click to unstage*), which gives
  both names of a staged rename; unticking the header box while files are in conflict, which gives the files staged.
  Each sends `feature/run` of `changes.unstage` to Gitardo's engine (`Controller.runOnEngine`), on the one file or on
  the files given (`engineRuns` in `src/engine/actions.ts`), its plan shown in a modal dialog when
  `gitardo.commandPreview` is `all` (`VscodeAsker`). **Unstage File** in the title of the Flow Diff sends
  `changes.unstage` on its file, through `Controller.runIn` (`src/flow/flowDiff.ts`). When the engine's binary is
  missing, it says so (D49).
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
  staged. The plan is shown in a modal dialog when `gitardo.commandPreview` is `all` (`VscodeAsker`). When the engine's
  binary is missing, it says so (D49).
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
  unstage all changes*) while no file is in conflict, and **Unstage all changes** in the context menu of the uncommitted
  changes row of the graph, offered while something is staged, also while files are in conflict, send `feature/run` of
  `changes.unstage-all` to Gitardo's engine on the changes (`Controller.runOnEngine`, `engineRuns` in
  `src/engine/actions.ts`): neither ends a merge or a cherry-pick any more. While files are in conflict the header box
  sends `changes.unstage` with the files staged, both names of a rename. The plan is shown in a modal dialog when
  `gitardo.commandPreview` is `all` (`VscodeAsker`). When the engine's binary is missing, it says so (D49).
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
- **Entry points**: **Discard changes** in the menu of a file of the Changes page (`changes.row-menu`).
- **Extension**: **Discard changes** on a file of the Changes view, which gives the file alone when nothing of it is
  staged and the file with both names of a rename otherwise; **Discard all changes in this file, staged too…** on a file
  of the uncommitted changes in the details of the graph; **Discard all changes in 1 selected file…** or **Discard all
  changes in N selected files…** above those files, once some are ticked. Each sends `feature/run` of `changes.discard`
  to Gitardo's engine (`Controller.runOnEngine`), on the one file or on the files given (the actions `discard` and
  `discardChanges` in `engineRuns`, `src/engine/actions.ts`); its warning is always asked, in a modal warning dialog
  (`VscodeAsker`). **Discard Changes** in the title of the Flow Diff sends `changes.discard` on its file, through
  `Controller.runIn` (`src/flow/flowDiff.ts`). When the engine's binary is missing, it says so (D49).
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
- **Extension**: **Discard all changes…** in the context menu of the uncommitted changes row of the graph, and above the
  files of the uncommitted changes in the details while none is ticked, both offered while no operation is under way,
  send `feature/run` of `changes.discard-all` to Gitardo's engine on the whole working tree, as from the draft;
  **Discard all changes** on the header of **Changes** in the Changes view sends it on the files of the list when
  nothing is staged, and `changes.discard` with every file of the list, both names of a rename, when something is (the
  actions `discardChanges` and `discardAll` in `engineRuns`, `src/engine/actions.ts`, run by `Controller.runOnEngine`).
  The warning is always asked, in a modal warning dialog (`VscodeAsker`). When the engine's binary is missing, it says
  so (D49).
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
  in a File page. **Save** in the menu of a row of the Changes page (`changes.row-menu`) saves that file through
  `file-view.save`, which asks before writing over a file another program changed.
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
  commit box (the message `committed`). The message the box starts from comes from the engine's follow-ups, posted as
  `commitMessage` by `Controller.followUpsBeforeRead` (`src/extension/controller.ts`): after a squash merge,
  `preparedMessage`, for which the extension reads what `.git/SQUASH_MSG` says (`reads.squashMessage`,
  `commit.prepared-message`); after a split, `fillCommitMessage`, with the message of the commit split. When the
  engine's binary is missing, it says so (D49).
- **Offered when**: **Amend last commit** is not ticked, the message is not empty and something is changed; otherwise
  the button is disabled.
- **Asks**: when nothing is staged and some change is not: a confirmation, "There are no staged changes. Stage all
  changes and commit them?", with **Stage all and commit**.
- **Plan**: `git commit -m <message>`, the message without the spaces and blank lines around it, with the arguments of
  the options of the commit box between `commit` and `-m` (`commit.option-arguments`), given as answers the command
  line never asks; after the confirmation, `git add --all` first.
- **Danger**: None.
- **Undo**: By the reflog: Undo moves the branch back to the commit before, with `git reset --soft`, and the changes of
  the commit stay staged (`undo.last`).
- **After**: the message box is emptied, **Amend last commit** unticked and the options of the commit back where they
  start (`commit.options`); the timeline selects nothing new.
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
  it says so (D49).
- **Offered when**: there is a commit: the box **Amend last commit** is disabled before the first one. Nothing needs to
  be staged.
- **Plan**: `git commit --amend -m <message>`; with the message box empty, `git commit --amend --no-edit`, which keeps
  the message the commit has. The arguments of the options of the commit box go after `--amend`
  (`commit.option-arguments`).
- **Danger**: None.
- **Undo**: By the reflog: Undo takes the branch back to the commit as it was before the amend, with `git reset --soft`,
  and what the amend added stays staged (`undo.last`).
- **After**: the message box is emptied, **Amend last commit** unticked and the options of the commit back where they
  start.

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

### `commit.options` — The options of a commit

Signs the next commit or not, adds `Signed-off-by` and co-authors to its message, or skips the hooks, for that commit,
from a menu beside **Commit**
([D79](../08-decisions.md#d79--the-signature-of-a-commit-shown-and-the-options-of-the-commit-box-proposed)).

- **Kind**: View
- **Function**: `views::changes::commit_options_view(state: &CommitOptionsState, ui) -> Vec<Intent>`
- **Entry points**: the button **Commit options** beside **Commit**, or **Amend**, in the commit box of the Changes
  page and of the **Changes** section; a click on the line under the box that says the options in force.
- **Reads**: `commit.read-signing`, with the state of the repository.
- **Settings**: `commit.signOff`, a new key (User, Repository), `false` by default: **Add Signed-off-by to every
  commit**, for the projects that ask for it on each commit.
- **Uses**: `commit.read-signing`, `commit.co-authors`, `commit.option-arguments`, `commit.create`, `commit.amend`,
  `identity.switch`, `ui.menu-icons`.
- **Extension**: the same button, **Commit options** with the Codicon `settings`, beside **Commit** in the commit box of
  the **Gitardo Changes** view, its menu drawn by the view (`commitOptionsMenu` in `src/webview/commitOptions.ts`), the
  line under the box too; the options go to the engine in the answers of `feature/run` of `commit.create` and
  `commit.amend` (`optionAnswers` in `src/engine/actions.ts`), and the setting is `gitardo.commit.signOff`. As in the
  window, what an entry signs with or adds is its tooltip. The options of each repository are kept by the view while
  another is shown. VS Code's own **Commit (Signed Off)** and **Commit (No Verify)** are of its Source Control, not of
  Gitardo's box.

The menu has, each with a box ticked or not and its icon:

- **Sign this commit**, ticked to start when git's configuration signs every commit (`commit.gpgSign`), with what it
  signs with under it: `With the SSH key ~/.ssh/id_ed25519.pub`, `With the OpenPGP key 3AA5C34371567BD2`, or **With
  the OpenPGP key of your email** when none is set. When git signs with SSH or X.509 and no key is set, it cannot be
  ticked, and says **No key to sign with: set one with Switch identity…**, a link to `identity.switch`.
- **Add Signed-off-by**, ticked to start when `commit.signOff` is on, with `Signed-off-by: <name> <email>` of the
  identity under it.
- **Co-authors…**, which opens `commit.co-authors`; the co-authors chosen are listed under it, each with ×.
- **Skip the hooks**, never ticked to start, with **Commit without running the pre-commit and commit-msg hooks** under
  it.

A tick changed holds for the next commit or amend alone: once it is made, the boxes go back to where they start and
the co-authors are emptied (Q1093, Q1094). Each repository open keeps its own, as it keeps its message. While any option
differs from where it starts, or a co-author is chosen, a line under the box says them, muted, in the order of the
menu: `Not signed · Signed-off-by · With Ben, +1 · Hooks skipped`, and its tooltip says the arguments they add.

As built, the menu differs from the plan in three things, since a menu of the window has no line under an entry and an
entry that switches shows its check alone (`ui.menu-icons`): what **Sign this commit** signs with, the
`Signed-off-by` line and what **Skip the hooks** does are the tooltips of their entries; a ticked option has the check
and no icon of its own; and when no key can sign, **Switch identity…** is an entry of its own under **Sign this
commit**, which cannot be chosen and says **No key to sign with: set one with Switch identity…** in its tooltip. The
options a repository keeps last while the window is open, as its message does not: they are not written to disk. Each
co-author chosen is an entry of the menu under **Co-authors…**, `Ben <ben@example.com>` with ×, which takes it away.

Acceptance:

- [ ] With `commit.gpgSign` set to `true`, the menu opens with **Sign this commit** ticked; unticking it says **Not
      signed** under the box, and the commit made has no signature.
- [ ] With `gpg.format` set to `ssh` and no `user.signingKey`, **Sign this commit** cannot be ticked, and says so.
- [ ] With `commit.signOff` on in `.gitardo/settings.json`, **Add Signed-off-by** starts ticked in that repository
      only.
- [ ] After a commit with **Skip the hooks** ticked, the next commit runs the hooks again.
- [ ] Two co-authors chosen are listed in the menu and under the box, and are gone after the commit.
- [ ] The repository shown changed and back, the options ticked are as they were left.
- [ ] The menu open matches its snapshots in the dark and in the light theme.
- [ ] Extension: with `commit.gpgSign` on, **Sign this commit** starts ticked, unticked says **Not signed** under
      the box and sends `--no-gpg-sign`; with `gpg.format` `ssh` and no key it cannot be ticked; two co-authors are
      listed in the menu and under the box, each taken away by a click.

### `commit.option-arguments` — The arguments of the options of a commit

What the options of the commit box add to `git commit`, for `commit.create` and `commit.amend`
([D79](../08-decisions.md#d79--the-signature-of-a-commit-shown-and-the-options-of-the-commit-box-proposed)).

- **Kind**: Derivation
- **Function**: `model::commit::option_arguments(options: &CommitOptions, signing: &SigningConfig, message: &str) ->
  Vec<OsString>`
- **Uses**: by `commit.create` and `commit.amend`; `commit.read-signing`.
- **Extension**: the engine's, given the options in the answers of `feature/run`; the co-authors are known from its
  first call, as a flag is, since no question asks them (protocol 1.13).

- **Sign this commit** ticked while git's configuration does not sign: `--gpg-sign`, which signs with the key git's
  configuration names. Unticked while it signs: `--no-gpg-sign`. As the configuration says: nothing.
- **Add Signed-off-by**: `--signoff`, which git adds only when the message does not end with that very line.
- **Skip the hooks**: `--no-verify`.
- Each co-author, in the order chosen, unless the message has that line already: `--trailer` and `Co-authored-by:
  <name> <<email>>` as one argument, the form GitHub reads (Q1095).
- The arguments go after `commit` and `--amend`, and before `-m <message>` or `--no-edit`: `git commit --amend
  --gpg-sign --signoff --trailer "Co-authored-by: Ben <ben@example.com>" --no-edit`.

Acceptance:

- [ ] Nothing ticked, with a configuration that does not sign, gives no argument: `git commit -m "Fix the parser"`.
- [ ] **Sign this commit** ticked gives `--gpg-sign`; with `commit.gpgSign` set to `true` it gives nothing, and
      unticked `--no-gpg-sign`.
- [ ] Two co-authors give two `--trailer` arguments in the order chosen; one whose line is in the message already
      gives none.
- [ ] An amend with an empty message, **Add Signed-off-by** and a co-author runs `git commit --amend --signoff
      --trailer "Co-authored-by: Ben <ben@example.com>" --no-edit`, and the message of the commit ends with the two
      lines after.
- [ ] A name with a space or a quote is one argument, whatever it holds.
- [ ] Extension: **Add Signed-off-by**, two co-authors and **Skip the hooks** commit past a `pre-commit` hook that
      fails, and the message ends with the three lines.

### `commit.read-signing` — Read how git signs commits

Whether git's configuration signs every commit, in which kind and with which key, so that **Sign this commit** starts
as git would commit, and says what it signs with.

- **Kind**: Read
- **Function**: `git::read::commit::read_signing(git: &Git, request: &SigningRequest) -> Result<SigningConfig, GitError>`
- **Reads**: `git config -z --get-regexp ^(commit\.gpgsign|gpg\.format|user\.signingkey)$`, wherever they are set,
  with the reads of `repo.read-state`. Git ends with code 1 when none is set, which is an empty result, not an error.
  `commit.gpgSign` is read as git reads a boolean: `true`, `yes`, `on`, `1` and a key with no value are true; the
  kind is `openpgp` when `gpg.format` is not set.
- **Uses**: by `commit.options` and `commit.option-arguments`.
- **Extension**: `signing` of `read/repo.read-state` of the engine (protocol 1.13).

Acceptance:

- [ ] With nothing set, git does not sign, the kind is OpenPGP and there is no key.
- [ ] With `commit.gpgSign` set to `yes` in the global configuration and `false` in the repository, git does not sign.
- [ ] With `gpg.format` set to `ssh` and `user.signingKey` to a path, the kind is SSH and the key that path.
- [ ] Extension: with `commit.gpgSign` set to `true`, the state the engine sends says git signs every commit.

### `commit.read-authors` — Read the authors of the history

The people who made commits in the repository, with how many each made, for choosing co-authors.

- **Kind**: Read
- **Function**: `git::read::commit::read_authors(git: &Git, request: &AuthorsRequest) -> Result<Vec<Author>, GitError>`
- **Reads**: `git log -z --format=%aN%x1f%aE --max-count=10000 --branches --remotes --tags`, the names and emails as
  the repository's `.mailmap` says them; an author is an email, without regard to case, with the name of its latest
  commit. Read when `commit.co-authors` opens, and stopped when it closes (Q1096).
- **Uses**: by `commit.co-authors`.
- **Extension**: `read/commit.read-authors` of the engine (protocol 1.13), asked as the quick pick of the co-authors
  opens.

Acceptance:

- [ ] A history with three commits of `ben@example.com` and one of `cy@example.com` gives Ben with 3 and Cy with 1.
- [ ] `Ben@Example.com` and `ben@example.com` are one author, named as the latest of their commits says.
- [ ] A `.mailmap` that maps an old email to a new one gives the new one alone.
- [ ] Extension: `read/commit.read-authors` gives each author with how many commits they made.

### `commit.co-authors` — Choose the co-authors of a commit

Picks the people who made the next commit with the one who commits, among the authors of the history or typed, so that
the commit names them in `Co-authored-by` lines.

- **Kind**: View
- **Function**: `views::changes::co_authors_view(state: &CoAuthorsState, ui) -> Vec<Intent>`
- **Entry points**: **Co-authors…** in the menu of `commit.options`.
- **Reads**: `commit.read-authors`.
- **Uses**: `commit.read-authors`, `commit.options`, `identity.read`.
- **Extension**: a quick pick of many of VS Code, from **Co-authors…** of the **Gitardo Changes** view
  (`src/extension/coAuthors.ts`), titled **Co-authors of this commit**; it opens at once, busy until the authors are
  read, and its entries are made by `coAuthorEntries` of `src/shared/commitOptions.ts`.

Gitardo's picker of many, titled **Co-authors of this commit**: each author by name with the email under it and how
many commits, the ones with the most first, the identity that commits left out; the co-authors chosen already are
ticked. What is typed matches the names and the emails, as Quick search matches. When it is a name and an email,
`Ben <ben@example.com>`, that no author has, an entry **Add Ben <ben@example.com>** is first. `Enter` keeps the ticks
and closes the picker; `Escape` leaves the co-authors as they were.

As built, the picker is a dialog of the window with a box at its top, its button **Set co-authors**, and room for eight
people: the co-authors added by typing, then the authors found, so that a history of hundreds of authors keeps the
dialog its height; typing finds the others. **Reading the authors…** stands in their place until they are read.

Acceptance:

- [ ] The picker lists the authors of the history, the one with the most commits first, without the identity that
      commits.
- [ ] Typing `ben` keeps Ben, by his name or his email.
- [ ] Typing `Dee <dee@example.com>` offers **Add Dee <dee@example.com>**; chosen, Dee is a co-author.
- [ ] Typing `Dee` alone offers nothing to add.
- [ ] `Escape` after ticking two authors leaves the co-authors as they were.
- [ ] Extension: the quick pick lists the authors without the identity that commits, and offers someone typed as
      `Name <email>` first.

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
  changes to the model to draft a commit message.* (G3: not the model of `gitardo.conflicts.aiModel`). The engine drafts
  it; the extension only sends the request the engine wrote to VS Code's model and gives the answer back. When the
  engine's binary is missing, it says so (D49). The status bar says *Gitardo: drafting commit message* while it runs.
  With no model, it refuses with *Gitardo: No language model is available in VS Code. Install an extension that
  provides one, such as GitHub Copilot.*
- **Offered when**: something is changed; the button is disabled while a draft is being written.
- **Reads**: the staged changes, `git diff --cached --no-color --no-ext-diff`, when something is staged; otherwise all
  changes, `git diff --no-color --no-ext-diff`, with the paths of the untracked files; and the subjects of the last 10
  commits as the timeline lists them, without the draft and without stashes.
- **Settings**: `ai.provider`, `ai.endpoint`, `ai.model` and `ai.effort`, the same as for the conflicts, with the key in
  the credential store of the system; with `ai.provider` set to `copilot`, GitHub Copilot is asked through its CLI
  (`ai.copilot`).
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
- [ ] With `ai.provider` set to `copilot`, the draft is written by GitHub Copilot.
- [ ] Extension: nothing is sent to a model until the sparkle button is pressed.
- [ ] Extension: with no model in VS Code, the error above is shown and the box keeps what it had.
- [ ] Extension: nothing in the repository changes, and nothing is written to the log of operations.
- [ ] Extension: with `gitardo.engine.path` naming a missing file, the sparkle button shows **Gitardo's engine is not
      available on this system.** with **Show the log**, sends nothing to a model and leaves the box as it was.

### `commit.read-draft-diff` — The diff a draft is written from

The diff `commit.draft-message` sends to the model: of what is staged when something is, as **Commit** would commit
it, and of every change of the tracked files otherwise (Q398).

- **Kind**: Read
- **Function**: `git::read::commit::read_draft_diff(git: &Git, request: &DraftDiffRequest) -> Result<String, GitError>`
- **Reads**: `git diff --cached --no-color --no-ext-diff` when `is_staged`, `git diff --no-color --no-ext-diff`
  otherwise, as git prints it, bytes that are not UTF-8 replaced. An untracked file is in neither; the draft names
  those apart.
- **Extension**: the engine reads it for `read/commit.draft-message`; the extension does not read it.

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
  toolbar of the graph, and a click on the identity under the commit box of the Changes view. Each sends `feature/run`
  of `identity.switch` to Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`,
  `Controller.switchIdentity`); the pick is a quick pick, and the name and email input boxes. **Edit the list of
  identities…** is added to the pick by the extension, which opens the settings at `gitardo.identities` and runs nothing
  (`HostHooks.choices`, B9). When the engine's binary is missing, it says so (D49).
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

## What git leaves alone

Telling git to leave a file alone, from the changes and the Files page
([D83](../08-decisions.md#d83--ignoring-files-files-git-does-not-look-at-and-locked-worktrees-from-the-window-proposed)):
a pattern that ignores an untracked file, written in a `.gitignore` or in `.git/info/exclude`; a tracked file that git
stops tracking, kept on disk; and the files whose changes git is told not to look at, marked in the index, which are
listed under the changes so that they are not forgotten. Locking a worktree is in `worktree.`.

### `changes.ignore-patterns` — The patterns that ignore a file

The patterns offered to ignore a file or a folder, the files each can be written in, and the line written there.

- **Kind**: Derivation
- **Function**: `model::changes::ignore_patterns(input: &IgnoreInput) -> IgnoreChoices`
- **Uses**: by `changes.ignore`.

Rules:

- **Input**: the path from the top of the repository, whether it is a folder, the path of the exclude file of the
  repository, and the text of each file a line can go in. **Output**: the patterns, in order, each with its words, and
  for each place the line written there.
- For a file, in this order: **This file**, its path; **All `.<extension>` files**, `*.<extension>`, when its name has
  a dot that is not its first character, the extension being what follows the last dot (`archive.tar.gz` gives `*.gz`,
  `.env` gives none); **The folder `<folder>`**, the folder that holds it, when it is not at the top.
- For a folder: **This folder**, its path with `/` at its end; **Every folder named `<name>`**, `<name>/`, which
  ignores a folder of that name wherever it is, as `node_modules/` does.
- The places, in this order: **The `.gitignore` of the repository**, at the top; **The `.gitignore` of `<folder>`**,
  the folder that holds the file, when it is not the top, whether that file exists yet or not; **Only on this
  computer**, the exclude file of the repository (`git rev-parse --git-path info/exclude`), which is not committed and
  which the worktrees of the repository share.
- A path is written from the folder of the file it goes in, with `/` before it so that it names that place only:
  `/logs/today.log` in the `.gitignore` of the repository, `/today.log` in the one of `logs`. A pattern that names the
  folder of that `.gitignore` itself, or a place outside it, is not offered for that file.
- In a path, `*`, `?`, `[` and `\` are written with `\` before them, and so is a space at its end, so that the line
  names that path and no other: `notes[1].txt` is written `/notes\[1\].txt`. `#` and `!` need nothing, as a path
  written always starts with `/`.
- The line goes at the end of the file, on a line of its own, with the line ending the file uses (CRLF when its first
  line ends with CRLF); a file that does not exist is made with the line alone. A pattern the file has already, as a
  line equal to it once the spaces at its end are left aside, is not written again.

Acceptance:

- [ ] `logs/today.log` gives **This file** `/logs/today.log`, **All `.log` files** `*.log` and **The folder `logs`**
      `/logs/`; in the `.gitignore` of `logs`, the first two are `/today.log` and `*.log`, and the third is not offered.
- [ ] `.env` at the top gives **This file** `/.env` alone; `dist/app.tar.gz` gives `*.gz` among its patterns.
- [ ] The folder `packages/web/node_modules` gives `/packages/web/node_modules/` and `node_modules/`.
- [ ] `notes[1].txt` is written `/notes\[1\].txt`, and `git check-ignore` says that line ignores it and not
      `notes1.txt`.
- [ ] A `.gitignore` whose last line has no line ending gets the new line on a line of its own; a `.gitignore` with
      CRLF gets the line with CRLF.
- [ ] A `.gitignore` that has `*.log ` already is given no second `*.log`.

### `changes.ignore` — Ignore a file or a folder

Has git ignore a file or a folder, by a pattern written where the user chooses; on a file git tracks, offers to stop
tracking it too, since a pattern does not stop git from following a file it tracks.

- **Kind**: Operation
- **Function**: `features::changes::ignore(context, answers) -> Outcome`
- **Entry points**: **Ignore…** in the menu of a file of the changes (`changes.row-menu`), in the Changes page and in
  the **Changes** section; **Ignore…** in the context menu of a file or a folder of the Files page.
- **Offered when**: the file or the folder is in the repository, outside its submodules, and no pattern ignores it.
  Not on a file in conflict, nor on a file only an editor has changed.
- **Asks**:
  1. **Ignore what**: the patterns of `changes.ignore-patterns`, **This file** (or **This folder**) by default, each
     with its pattern beside it, and a pattern typed, which may not be empty nor hold a line ending (Q1170, Q1172).
  2. **Where**: the places of `changes.ignore-patterns`, **The `.gitignore` of the repository** by default (Q1171),
     each with the line it writes there; **Only on this computer** says *Not committed, and not shared with anyone*.
  3. Only when git tracks the file, or files under the folder: **Git tracks `<path>`: a pattern does not stop git from
     following it.** with **Stop tracking it, and keep it on disk** (the default), which adds the steps of
     `changes.stop-tracking`, and **Only write the pattern**.
- **Reads**: `git check-ignore -v -z --no-index -- <path>`, to refuse a path that some pattern ignores already and to
  say which; `git ls-files -z -- <path>`, to know whether git tracks it or files under it.
- **Plan**: `write <file>`, the file chosen with the line of `changes.ignore-patterns` added; then, when the third
  answer says so, the steps of `changes.stop-tracking`. No `write` when the file has the pattern already.
- **Danger**: None; with the steps of `changes.stop-tracking`, the danger of that card.
- **Undo**: Not taken back by Undo, which follows the reflog: the line is taken out of the file by hand, which **Open
  `<file>`** in the message after opens.
- **After**: says **`<path>` is ignored by `<pattern>` in `<file>`.** with **Open `<file>`**, which opens the File
  page at the line written. The repository is read again: the path leaves the changes, and the `.gitignore` written is
  listed as changed, or as untracked when it was made, to be committed (Q1175).
- **Refuses**: a path that a pattern ignores already: **`<path>` is ignored already, by `<pattern>` in `<file>`.**; a
  path in a submodule: **`<path>` is in the submodule `<name>`: ignore it there.**
- **Uses**: `changes.ignore-patterns`, `changes.stop-tracking`, `changes.pathspec`, `changes.row-menu`,
  `file-view.page`.
- **Extension**: planned: **Ignore…** in the context menu of a file of the **Gitardo Changes** view and of a file or a
  folder of VS Code's Explorer (`explorer/context`, beside **Link into New Worktrees**), sending `feature/run` of
  `changes.ignore` to Gitardo's engine (`Controller.runOnEngine`), its questions asked in VS Code's pickers
  (`VscodeAsker`) (Q1185). **Open `<file>`** opens the file in an editor of VS Code. VS Code's own **Add to
  .gitignore** of its Source Control view stays as it is.

Acceptance:

- [ ] On the untracked `debug.log`, **All `.log` files** in the repository's `.gitignore` adds `*.log` at its end and
      logs `write .gitignore`; once read again, `debug.log` is not in the changes and `.gitignore` is.
- [ ] **Only on this computer** writes the line in `.git/info/exclude`: `debug.log` leaves the changes, and no file
      of the working tree changes.
- [ ] On the folder `build` of the Files page, **This folder** writes `/build/`.
- [ ] On `config/local.json`, which git tracks, the third question is asked; **Stop tracking it, and keep it on disk**
      writes the pattern then runs `git rm --cached -- config/local.json`, and the file is still on disk.
- [ ] **Only write the pattern** on a tracked file writes the line, runs no `git rm`, and the file is still listed as
      changed.
- [ ] A pattern typed, `tmp-*`, is written as typed.
- [ ] On a path that `*.log` in `.gitignore` ignores already, it refuses with **`x.log` is ignored already, by
      `*.log` in `.gitignore`.**, and writes nothing.
- [ ] Backing out of any question writes nothing.
- [ ] Extension: **Ignore…** on `debug.log` in the Explorer writes `/debug.log` in the repository's `.gitignore`
      through the engine, written to the **Gitardo** output channel.

### `changes.stop-tracking` — Stop tracking a file, and keep it on disk

Takes a file, or every file of a folder, out of the index, so that the next commit deletes it from the repository while
it stays on disk: for a file committed by mistake, such as a file of local settings, once it is ignored.

- **Kind**: Operation
- **Function**: `features::changes::stop_tracking(context, answers) -> Outcome`
- **Entry points**: the third question of `changes.ignore`; **Stop tracking, keep on disk…** in the menu of a tracked
  file of the changes (`changes.row-menu`) and in the context menu of a file or a folder git tracks in the Files page.
- **Offered when**: git tracks the file, or files under the folder; not on a file in conflict.
- **Reads**: `changes.read-skipped`, for the files among them that are hidden from git.
- **Plan**: for each file hidden from git, `git update-index --no-skip-worktree -- <path>` first, since `git rm`
  leaves such a file alone; then `git rm --cached -- <path>`, or `git rm --cached -r -- <path>` for a folder, with the
  path as `changes.pathspec` writes it. When git refuses because the index has a version that is neither the last
  commit nor the file on disk, Gitardo offers **Stop tracking anyway**, the same plan with `git rm --cached --force`,
  Destructive, because **The version of `<path>` that is staged is lost: it is neither committed nor on disk.**
- **Danger**: Confirm, because **`<path>` stays on your disk. Once committed, it is deleted from the repository, and
  from the disk of whoever pulls that commit.**
- **Undo**: Not taken back by Undo: unticking the deletion in the changes (`changes.unstage`) takes it back while it
  is not committed.
- **After**: the deletion is staged and listed in the changes; the file on disk is not listed when a pattern ignores
  it, and is listed as untracked otherwise.
- **Refuses**: **Git does not track `<path>`.**; a file in conflict: **`<path>` is in conflict: resolve it first.**
- **Uses**: `changes.pathspec`, `changes.read-skipped`, `changes.unskip`, `changes.unstage`.
- **Extension**: planned: **Stop Tracking, Keep on Disk…** in the context menu of a file of the **Gitardo Changes**
  view and of a file or a folder of VS Code's Explorer, sending `feature/run` of `changes.stop-tracking` to the
  engine, its danger asked in a modal dialog (`VscodeAsker`) (Q1174).

Acceptance:

- [ ] On `config/local.json`, it asks with the warning, then runs `git rm --cached -- config/local.json`; the file
      is on disk after, and the changes list it as deleted, staged.
- [ ] On the folder `generated`, it runs `git rm --cached -r -- generated`.
- [ ] On a file hidden from git with `--skip-worktree`, it runs `git update-index --no-skip-worktree` first, and the
      file is out of the index after.
- [ ] With `a.txt` staged and changed again on disk, git's refusal is followed by **Stop tracking anyway**, shown as
      destructive, and nothing changes until it is accepted.
- [ ] Unticking the staged deletion after it puts the file back in the index as it was.
- [ ] Extension: the modal dialog shows `git rm --cached -- <path>` with the warning, whatever
      `gitardo.commandPreview` says.

### `changes.read-skipped` — Read the files hidden from git

Which tracked files git is told not to look at, and how: those marked **skip-worktree**, whose changes stay out of
commits and which a checkout leaves alone, and those marked **assume unchanged**, a promise git may forget.

- **Kind**: Read
- **Function**: `git::read::changes::read_skipped(git: &Git, request: &SkippedRequest) -> Result<SkippedFiles, GitError>`
- **Reads**: `git ls-files -v -z`, with `GIT_OPTIONAL_LOCKS=0`: each record is a letter, a space and a path. A letter
  in lower case is a file assumed unchanged; `S` is a file skipped in the working tree; `s` is both. Any other letter is
  a file git looks at as usual. And `git config --type=bool --get core.sparseCheckout`: in a sparse checkout git marks
  skip-worktree every file it leaves out of the working tree, so there the files marked `S` are the sparse checkout's,
  not files hidden by hand, and are left out of the result (Q1179).
- **Uses**: `repo.read-state`, with which it runs, and again only when the index changed since the read before
  (`repo.watch-changes`), as the counts of the other worktrees are read again only when their git folder changed
  (Q1177).
- **Extension**: read by the engine with the state of the repository; the extension shows what it reads.

Acceptance:

- [ ] `h a.txt\0S b.txt\0s c.txt\0H d.txt\0` gives `a.txt` assumed unchanged, `b.txt` skipped, `c.txt` both, and
      not `d.txt`.
- [ ] A path with spaces and non-ASCII letters comes back exactly as it is in the index.
- [ ] With `core.sparseCheckout` on, a file marked `S` is not in the result, and a file marked `h` is.
- [ ] A read of the repository after a change to a file of the working tree only, the index as it was, does not run
      `git ls-files -v`.

### `changes.skip` — Hide a file's changes from git

Has git leave the changes of a tracked file alone: they stay on disk and out of every commit, as for a settings file
that each one changes on their own computer.

- **Kind**: Operation
- **Function**: `features::changes::skip(context, answers) -> Outcome`
- **Entry points**: **Hide its changes from git…** in the menu of a tracked file of the changes (`changes.row-menu`)
  and in the context menu of a tracked file of the Files page.
- **Offered when**: git tracks the file, it is not hidden already, and it is not in conflict. Not on a folder (Q1184).
- **Asks**: **How git leaves `<path>` alone**: **Skip it in the working tree** (the default, Q1176), *Git keeps the
  file as committed and leaves your version on disk alone. A pull that changes it stops until you show its changes to
  git again.*; **Assume it is unchanged**, *A promise that the file does not change, for speed: git may look at it
  again, and a checkout may write over your changes.*
- **Plan**: `git update-index --skip-worktree -- <path>`, or `git update-index --assume-unchanged -- <path>`. The path
  is given as it is: `git update-index` takes paths, not patterns.
- **Danger**: None. The question says what each way means.
- **Undo**: Not taken back by Undo: **Show its changes to git again** (`changes.unskip`) does.
- **After**: the file leaves the list of the changes and is listed under **Hidden from git** (`changes.skipped-list`).
- **Refuses**: on a file git does not track: **Git does not track `<path>`: ignore it instead.** with **Ignore…**
  (`changes.ignore`).
- **Uses**: `changes.read-skipped`, `changes.skipped-list`, `changes.unskip`, `changes.ignore`.
- **Extension**: planned: **Hide Its Changes from Git…** in the context menu of a file of the **Gitardo Changes** view
  and of a file of VS Code's Explorer, sending `feature/run` of `changes.skip` to the engine.

Acceptance:

- [ ] On `config/local.json`, changed, **Skip it in the working tree** runs
      `git update-index --skip-worktree -- config/local.json`; the file leaves the changes, is listed under **Hidden
      from git**, and `git status` no longer lists it.
- [ ] **Assume it is unchanged** runs `git update-index --assume-unchanged -- config/local.json`.
- [ ] A file named `pages/[id].tsx` is given as `pages/[id].tsx`, not as `:(literal)pages/[id].tsx`.
- [ ] The entry is not in the menu of an untracked file, of a folder, nor of a file in conflict.

### `changes.unskip` — Show a file's changes to git again

Takes back what `changes.skip` did, for one file or for every file hidden.

- **Kind**: Operation
- **Function**: `features::changes::unskip(context, answers) -> Outcome`
- **Entry points**: **Show its changes to git again** in the menu of a row of **Hidden from git**
  (`changes.skipped-list`) and in the context menu of a hidden file of the Files page; **Show all to git again** in the
  **…** of the heading of **Hidden from git**.
- **Offered when**: the file is hidden from git by hand (`changes.read-skipped`).
- **Plan**: `git update-index --no-skip-worktree -- <path>…` for the files skipped, then
  `git update-index --no-assume-unchanged -- <path>…` for the files assumed unchanged: one command for each, since
  `git update-index` given both options at once changes only the first.
- **Danger**: None.
- **Undo**: Not taken back by Undo: `changes.skip` hides it again.
- **After**: the file leaves **Hidden from git**, and is listed in the changes when it differs from the last commit.
- **Uses**: `changes.read-skipped`, `changes.skipped-list`.
- **Extension**: planned: the same entries in the group **Hidden from git** of the **Gitardo Changes** view, sending
  `feature/run` of `changes.unskip` to the engine.

Acceptance:

- [ ] On `config/local.json`, skipped and changed on disk, it runs
      `git update-index --no-skip-worktree -- config/local.json`, and the file is listed in the changes as modified.
- [ ] On a file both skipped and assumed unchanged, it runs both commands, and `git ls-files -v` marks it `H` after.
- [ ] **Show all to git again** with three files hidden runs one command for each way they are hidden.

### `changes.skipped-list` — The files hidden from git, under the changes

The files git is told not to look at, listed under the changes while there are any, so that they are not forgotten.

- **Kind**: View
- **Function**: `views::changes::skipped_list(state: &SkippedListState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: under the list of the changed files, in the Changes page and in the **Changes** section; a mark
  on the row of such a file in the tree of the Files page.
- **Offered when**: `changes.read-skipped` gives at least one file; otherwise nothing of it is drawn.
- **Uses**: `changes.read-skipped`, `changes.unskip`, `changes.open`, `diff.page`, `diff.read-sides`,
  `file-view.page`, `file-view.files-changes`, `ui.menu-icons`.
- **Extension**: planned: a group **Hidden from git** at the end of the **Gitardo Changes** view, with the same rows
  and menus.

- A heading **Hidden from git** with their count, `Hidden from git 2`, unfolded when the repository is first shown,
  with **…** for **Show all to git again**. Whether it is folded is kept while the window is open, and not remembered
  (Q1178).
- A row for each file: its icon, its name, its folder, and in the small text of details, muted, **skip-worktree** or
  **assumed unchanged**. Its tooltip: *Git does not look at the changes of this file: they stay out of every commit.*
- A click on a row opens the Diff page of the file between the last commit and the disk, which Gitardo reads itself
  (`diff.read-sides`) whatever git is told; a file whose version on disk is as committed opens the File page instead.
  Its menu: **Open changes**, **Open file**, **Show its changes to git again**.
- In the tree of the Files page, a hidden file has an eye struck through at the end of its row, with the same tooltip,
  and **Show its changes to git again** in its context menu.

Acceptance:

- [ ] With `config/local.json` skipped, the Changes page and the section list it under **Hidden from git 1**, marked
      **skip-worktree**, and not in the list of the changes.
- [ ] A click on it, changed on disk, opens its Diff page between `HEAD` and the disk.
- [ ] With no file hidden, nothing of the group is drawn.
- [ ] In a sparse checkout, the files the checkout leaves out are not listed.
- [ ] The tree of the Files page shows the struck eye on `config/local.json`.
- [ ] The group matches its snapshots in each theme.
