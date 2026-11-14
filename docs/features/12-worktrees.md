# Worktrees

A worktree is another folder checked out from the same repository, on a branch of its own: a place to work on something
else, or to let an agent work, without touching the folder the window shows. This area covers creating worktrees and
where their folders go, what a new worktree gets of what git ignores and which commands run in it, the page that chooses
that, the Worktrees list of the branch rail with how each worktree stands, what can be done with a worktree and with a
branch that another worktree has, showing another worktree in this window, working in worktrees only, and what Windows
needs when a worktree is renamed or removed.

### `worktree.read-list` — Read the worktrees and how they stand

The list of the worktrees of the repository, each with its branch and the number of its files not committed, for the
Worktrees list, the picker of **Switch worktree…** and the rules about branches that another worktree has.

- **Kind**: Read
- **Function**: `git::read::worktree::read_list(git, request) -> Result<Vec<Worktree>, GitError>`
- **Reads**: `git worktree list --porcelain -z`, run in the folder the views show. For each worktree: its folder, the
  commit it has checked out, its branch or none (detached), whether it is the main worktree (the first one listed, the
  folder the repository was created in), whether it is the folder the views show, and whether it is missing (listed as
  `prunable`). Then, for every worktree other than the one the views show and other than a missing one, all at once:
  `git -C <folder> status --porcelain=v2 -z --untracked-files=all`, whose entries are counted: staged, not staged and
  untracked alike. The count of the worktree the views show is the count of the Changes page, read already.
- **Uses**: `repo.read-state`.

Acceptance:

- [ ] A repository with a main worktree and two others lists three worktrees, the main one marked as main.
- [ ] A worktree whose folder was deleted outside Gitardo is listed as missing, and no `git status` is run for it.
- [ ] A worktree with one staged file, one modified file and one untracked file in a new folder with two files has a
      count of 4.
- [ ] The `git status` of the other worktrees runs with `GIT_OPTIONAL_LOCKS=0`: a worktree whose index another program
      holds locked is counted without waiting for the lock and without leaving one behind.
- [ ] A worktree whose `git status` fails is listed with no count, and the others are counted all the same.
- [ ] A worktree on a detached HEAD is listed with no branch and with the commit it has checked out.
- [ ] Parsing is tested on recorded output, a folder with spaces and non-ASCII letters in its path included.

### `worktree.folder-path` — Where a new worktree goes

The folder of a new worktree, from the setting `worktrees.folder` and the name of the worktree, which is the name of its
branch, slashes and all.

- **Kind**: Derivation
- **Function**: `model::worktree::folder_path(input: &FolderInput) -> PathBuf`
- **Settings**: `worktrees.folder`, `${repo}.worktrees` by default.

The rules:

- The folder of the worktrees is `worktrees.folder`. An empty setting is the default, `${repo}.worktrees`.
- `${repo}` is the name of the folder of the main worktree: worktrees of a worktree go beside the repository, not beside
  that worktree.
- `~` at the start is the home folder of the user.
- A relative path starts from the folder that holds the main worktree; an absolute one (`/srv/trees`, `D:/trees`) is
  taken as it is. Backslashes count as slashes, and slashes at the end are dropped.
- The name gives the folder inside it, part by part between slashes: in each part, whatever is not an ASCII letter, a
  digit, `_`, `.` or `-` becomes one `-` per run; `-` and `.` are dropped from both ends; a part is cut to 60
  characters; empty parts are dropped. When nothing is left, the folder is `worktree`.

Examples, for the main worktree `~/code/app`:

| `worktrees.folder` | Name | Folder |
| --- | --- | --- |
| (default) | `fix-login` | `~/code/app.worktrees/fix-login` |
| (default) | `feature/login` | `~/code/app.worktrees/feature/login` |
| `../trees/${repo}` | `feature/login` | `~/trees/app/feature/login` |
| `~/wt` | `fix: the #42 bug` | `~/wt/fix-the-42-bug` |
| `D:/wt` | `../escape` | `D:/wt/escape` |
| (default) | `...` | `~/code/app.worktrees/worktree` |

Acceptance:

- [ ] Every example of the table gives the folder it shows.
- [ ] Asked from a worktree that is not the main one, the folder is beside the main worktree.
- [ ] No name, whatever it holds, gives a folder outside the folder of the worktrees.
- [ ] The label of a worktree in the views is its path from the folder of the worktrees when it is inside it
      (`feature/login`), and the name of its folder otherwise, compared without regard to case.

### `worktree.create` — Create a worktree

Makes a new worktree on a new branch, from a branch the user picks, in the folder of the worktrees.

- **Kind**: Operation
- **Function**: `features::worktree::create(context, answers) -> Outcome`
- **Entry points**: the **+** on the title of the Worktrees list, in Kept aside of the branch rail; **Create
  worktree…** in the **…** of the Worktrees list; the last entry of the picker of `worktree.switch`; Quick search,
  **Create Worktree…**.
- **Asks**:
  1. Only when `branches.format` is set: **Use the format** (with the format beside it, the default) or **Type a branch
     name**.
  2. What it starts from: the branch checked out (**Checked out**), or `HEAD` (**Detached**) when no branch is, first
     and the default; then the other local branches; then the remote branches (**Remote**).
  3. With the format: what `branch.name-from-format` asks, `${base}` being the name of the branch picked without its
     remote, then the name it makes, which can be changed. Without it: **Name of the new branch, and of the folder of the
     worktree**, with `feature/my-change` as a placeholder. While the name is typed, the box says where the folder goes,
     **In `<folder>`**, or that it is refused: a name git does not accept, **A branch with that name already exists**,
     **Git cannot have it beside the branch `<name>`** (for `feature/login` beside a branch `feature`, or the other way
     round), **`<folder>` already exists**.
- **Plan**: `git worktree add --no-track -b <name> <folder> <start>`, then the steps of
  `worktree-setup.link-and-copy-steps` and `worktree-setup.command-steps`. `--no-track` gives the new branch no upstream,
  even when it starts from a remote branch, so that its first push does not go onto the branch it came from.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Remove…** on the worktree removes it, and its branch with it if asked.
- **After**: says **Worktree created in `<folder>`** and offers to open it: **Open in new window**, **Open in external
  editor** while `editor.external` is set, and **Open in terminal**, the one `worktrees.openIn` names first. While no
  link or copy pattern and no command applies (`worktree-setup.settings-in-effect`), the first worktree created this way
  also offers **Choose What It Gets…**, which opens `worktree-setup.page`; it is offered once for the user, and never
  again once the page has been opened. With commands to run, the offer to open comes once they have all succeeded; when
  one fails, or did not run, the warning says which and offers to open it all the same.
- **Refuses**: when the folder exists by the time it runs: **`<folder>` already exists.**; a name refused while typed.
- **Settings**: `worktrees.folder`, `branches.format`, `branches.inputs`, `worktrees.openIn`, `commandPreview`.
- **Uses**: `worktree.folder-path`, `branch.name-from-format`, `worktree-setup.link-and-copy-steps`,
  `worktree-setup.command-steps`, `worktree.open`.

Acceptance:

- [ ] Named `feature/login` from `main`, it runs `git worktree add --no-track -b feature/login
      <repo>.worktrees/feature/login main`, and the box said **In `<repo>.worktrees/feature/login`** while it was typed.
- [ ] Started from `origin/release`, the new branch has no upstream after it is created.
- [ ] A name of an existing branch, and `feature` beside a branch `feature/x`, are refused while typed.
- [ ] With `branches.format` set to `${base}-${date}`, the picker starts on **Use the format**, and the name proposed
      from `main` on 2026-10-15 is `main-2026-10-15`, which can be changed before it is created.
- [ ] With nothing set for new worktrees, the first worktree created offers **Choose What It Gets…**, and the second one
      does not.
- [ ] With commands set, **Open in new window** is offered only after the last command succeeded.
- [ ] Backing out of any question runs nothing.

### `worktree.create-from-branch` — Put a branch in a new worktree

Puts a local branch in a worktree of its own, or, with a format of branch names, a new branch made from it.

- **Kind**: Operation
- **Function**: `features::worktree::create_from_branch(context, answers) -> Outcome`
- **Entry points**: **Create worktree…** in the context menu of a local branch, in the branch rail and on its label in
  the timeline.
- **Offered when**: no worktree has the branch checked out, this one included.
- **Asks**:
  1. Only when `branches.format` is set: **Use the format** (the default) or **Use the branch `<branch>`** (**Check it
     out in the worktree**).
  2. With the format: what `branch.name-from-format` asks, `${base}` being the branch, then the name it makes, which can
     be changed, refused while typed as in `worktree.create`. Without: **Name of the folder of the worktree**, the name
     of the branch by default, with **In `<folder>`** or **`<folder>` already exists** under it.
- **Plan**: on the branch itself, `git worktree add <folder> <branch>`; on a new branch from it,
  `git worktree add --no-track -b <name> <folder> <branch>`; then the steps of `worktree-setup.link-and-copy-steps` and
  `worktree-setup.command-steps`.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Remove…** on the worktree removes it.
- **After**: as `worktree.create`.
- **Refuses**: **`<folder>` already exists.**
- **Settings**: `worktrees.folder`, `branches.format`, `branches.inputs`, `worktrees.openIn`.
- **Uses**: `worktree.folder-path`, `branch.name-from-format`, `worktree-setup.link-and-copy-steps`,
  `worktree-setup.command-steps`.

Acceptance:

- [ ] On `feature/login`, with no format, it proposes the folder name `feature/login` and runs
      `git worktree add <repo>.worktrees/feature/login feature/login`.
- [ ] The entry is not in the menu of the branch checked out here, nor of a branch another worktree has.
- [ ] With a format, **Use the branch `feature/login`** puts that branch in the worktree, and **Use the format** makes a
      new branch from it with `--no-track -b`.
- [ ] A folder name changed to `login` puts the worktree in `<repo>.worktrees/login` on the branch `feature/login`.

### `worktree.create-at-commit` — Create a worktree at a commit

Makes a worktree at a commit of the timeline, on a new branch made there, or on a detached HEAD when no name is given.

- **Kind**: Operation
- **Function**: `features::worktree::create_at_commit(context, answers) -> Outcome`
- **Entry points**: **Create worktree here…** in the context menu of a commit in the timeline.
- **Asks**:
  1. Only when `branches.format` is set: **Use the format** (the default) or **Type a branch name**.
  2. With the format, when it has `${base}`: what `${base}` is, titled **Create worktree at `<short>`: what ${base} is**:
     the branches at the commit (**Branch at this commit**) first, then the short hash (**This commit**), then the other
     branches that have it (**Branch that has this commit**), at most 50, each by its own name, a remote branch without
     its remote. Then what `branch.name-from-format` asks, and the name it makes.
  3. Without the format: **Name of a new branch, and of the folder of the worktree. Leave empty for a detached HEAD.**
     Empty, the box says where the folder named after the short hash goes.
- **Plan**: `git worktree add --no-track -b <name> <folder> <commit>`, or `git worktree add --detach <folder> <commit>`
  with the folder named after the short hash; then the steps of `worktree-setup.link-and-copy-steps` and
  `worktree-setup.command-steps`.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Remove…** on the worktree removes it.
- **After**: as `worktree.create`.
- **Refuses**: **`<folder>` already exists.**
- **Settings**: `worktrees.folder`, `branches.format`, `branches.inputs`, `worktrees.openIn`.
- **Uses**: `worktree.folder-path`, `branch.name-from-format`, `worktree-setup.link-and-copy-steps`,
  `worktree-setup.command-steps`.

Acceptance:

- [ ] On `3f2a1c9…` with the name left empty, it runs `git worktree add --detach <repo>.worktrees/3f2a1c9 3f2a1c9…`.
- [ ] With the name `try/idea`, it runs `git worktree add --no-track -b try/idea <repo>.worktrees/try/idea 3f2a1c9…`.
- [ ] With the format `${base}-spike`, on a commit that `main` points at, the picker of `${base}` lists `main` first,
      and picking it proposes `main-spike`.

### `worktree.section` — The Worktrees list

The worktrees of the repository, each with how it stands, in **Kept aside** of the branch rail.

- **Kind**: View
- **Function**: `views::refs::entry::worktree_row(ui, state, entry, rect)`, a row of the branch rail, with
  `views::worktree::{label, standing, tooltip, open_intent}` and the menus of `views::worktree::worktree_entries`
  and `list_entries`
- **Entry points**: **Worktrees** in **Kept aside** of the branch rail, a list that opens in the rail.
- **Reads**: `worktree.read-list`, the branches with their upstreams (`refs.`), the pull requests (`github.`).

What each row shows:

- Its label: its path from the folder of the worktrees, or the name of its folder (`worktree.folder-path`).
- Its branch, or `detached`, then `↑n` for the commits its branch has that its upstream has not, and `↓m` the other way
  round, when it has an upstream; `missing` instead of all that when its folder is gone.
- A pencil with the number of its files not committed, when there are some.
- Its open pull request, colored by the result of its checks.
- The worktree the views show is marked as current.
- Its tooltip says the same in words: its folder, `<branch> → <upstream>`, how many files are not committed, the pull
  request.

The title of the list has **…**, which has **Create worktree…**, **Switch worktree…**,
**Choose what new worktrees get…**, **Bring ignored files into the other worktrees…** while there is another worktree
that is not missing, and **Prune missing worktrees** while one is missing. A double click on a row opens it as
`worktree.open` does, except on the current one and on a missing one, where it does nothing. Its context menu is the
one of the cards of this area: `worktree.open-in-new-window`, `worktree.open-in-editor`, `worktree.open-in-terminal`,
`worktree.reveal`, `worktree.show-in-gitardo`, `worktree.show-in-graph`, `worktree.rename`, `worktree.copy-path`,
`worktree-setup.bring-into-worktree`, `worktree.remove`, `worktree.prune`.

Acceptance:

- [ ] A worktree on `feature/login`, two commits ahead of its upstream and one behind, with three files not committed,
      shows `feature/login ↑2 ↓1` and a pencil with 3.
- [ ] A worktree whose folder is gone shows `missing`, and a double click on it does nothing.
- [ ] A file changed in another worktree from a terminal shows in its count at the next refresh of the views.
- [ ] The menu of the main worktree has no **Rename…** and no **Remove…**; the menu of the current one has neither,
      nor **Open in new window**, nor **Show in Gitardo**, nor **Bring ignored files…**.
- [ ] **Prune missing worktrees** is in the menus only while a worktree is missing.
- [ ] A worktree whose branch has the open pull request #12 shows its badge, and its tooltip names it.

### `worktree.open` — Open a worktree where the settings say

Opens a worktree where `worktrees.openIn` says: what a double click on a worktree, or on a branch another worktree
has, does, and what `worktree.open-in-worktree` and `worktree.switch` do once they have a worktree.

- **Kind**: Operation
- **Function**: `features::worktree::open(context, answers) -> Outcome`
- **Entry points**: a double click on a worktree in the Worktrees list, on a branch another worktree has (in the
  branch rail and on its label in the timeline), and on a remote branch whose local branch another worktree has.
- **Offered when**: the worktree is not the one the views show and is not missing.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: `window` (the default) opens a new window of Gitardo Desktop on its folder, as
  `worktree.open-in-new-window`; `editor` opens it in the external editor, as `worktree.open-in-editor`; `terminal`
  opens a terminal in it, as `worktree.open-in-terminal`. When `editor` is set but `editor.external` is not, or the
  editor does not start, it opens in a new window instead and says why.
- **Settings**: `worktrees.openIn`, `editor.external`, `terminal.external`.

Acceptance:

- [ ] With `worktrees.openIn` unset, a double click on a worktree opens a new window of Gitardo Desktop on it.
- [ ] With `worktrees.openIn` set to `terminal`, the same double click opens a terminal in its folder.
- [ ] With `editor` and `editor.external` unset, it opens a new window and says that no external editor is set.

### `worktree.open-in-new-window` — Open a worktree in a new window

Opens the worktree as the folder of a new window of Gitardo Desktop.

- **Kind**: Operation
- **Function**: `features::worktree::open_in_new_window(context, answers) -> Outcome`
- **Entry points**: **Open in new window** in the context menu of a worktree; **Open worktree in new window** in the
  menu of a branch another worktree has.
- **Offered when**: the worktree is not the one the views show and is not missing.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: a new window of Gitardo Desktop shows the worktree's folder.

Acceptance:

- [ ] The new window shows the repository of the worktree with the worktree as the folder of the window, and this
      window is left as it was.

### `worktree.open-in-editor` — Open a worktree in the external editor

Opens the worktree's folder in the editor that `editor.external` names, for instance to start an agent there.

- **Kind**: Operation
- **Function**: `features::worktree::open_in_editor(context, answers) -> Outcome`
- **Entry points**: **Open in external editor** in the context menu of a worktree; **Open worktree in external
  editor** in the menu of a branch another worktree has.
- **Offered when**: `editor.external` is set, and the worktree is not missing.
- **Plan**: `run <command>` of `editor.external` with the worktree's folder, in that folder.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the external editor starts on the folder; when it cannot be started, Gitardo says why.
- **Settings**: `editor.external`.

Acceptance:

- [ ] With `editor.external` set, the editor is started with the worktree's folder.
- [ ] With it unset, the entry is not in the menus.

### `worktree.open-in-terminal` — Open a terminal in a worktree

Opens a terminal with the worktree as its folder.

- **Kind**: Operation
- **Function**: `features::worktree::open_in_terminal(context, answers) -> Outcome`
- **Entry points**: **Open in terminal** in the context menu of a worktree; **Open terminal in worktree** in the menu
  of a branch another worktree has.
- **Offered when**: the worktree is not missing.
- **Plan**: `run <command>` of `terminal.external`, or of the system's terminal, in the worktree's
  folder.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the terminal of `terminal.external`, or the terminal of the system when it is not set, opens in the
  worktree's folder.
- **Settings**: `terminal.external`.

Acceptance:

- [ ] The terminal opens with its current folder set to the worktree's folder, the current worktree included.

### `worktree.reveal` — Reveal a worktree in the file manager

Shows the worktree's folder in the file manager of the system.

- **Kind**: Operation
- **Function**: `features::worktree::reveal(context, answers) -> Outcome`
- **Entry points**: **Reveal in File Explorer** on Windows, **Reveal in Finder** on macOS, **Open containing folder**
  elsewhere, in the context menu of a worktree; **Reveal worktree in File Explorer**, **Reveal worktree in Finder** or
  **Open folder of worktree** in the menu of a branch another worktree has.
- **Offered when**: the worktree is not missing.
- **Plan**: `run <command>` of the system's file manager on the worktree's folder: `start ""` on Windows,
  `open` on macOS, `xdg-open` elsewhere.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the file manager shows the folder.

Acceptance:

- [ ] On each system the entry has the name given above, and the file manager opens on the worktree's folder.

### `worktree.show-in-graph` — Show the commit of a worktree in the timeline

Selects in the timeline the commit a worktree has checked out.

- **Kind**: Operation
- **Function**: `features::worktree::show_in_graph(context, answers) -> Outcome`
- **Entry points**: **Show in the timeline** in the context menu of a worktree, in the branch rail.
- **Offered when**: the worktree is not missing.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the timeline scrolls to the commit and selects it, loading more of the history when it is not listed
  yet.
- **Uses**: `graph.show-in-graph`.

Acceptance:

- [ ] On a worktree whose commit is far down the history, the timeline loads up to it and selects it.

### `worktree.show-in-gitardo` — Show another worktree in this window

Points the views of this window at another worktree, so that what is done there, staging, committing, pulling or
pushing, is done in it, without opening another window.

- **Kind**: Operation
- **Function**: `features::worktree::show_in_gitardo(context, answers) -> Outcome`
- **Entry points**: **Show in Gitardo** in the context menu of a worktree; **Show worktree in Gitardo** in the menu of a
  branch another worktree has; the repository menu of the header, which lists it while it is shown.
- **Offered when**: the worktree is not the one the views show and is not missing.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the timeline with its draft, the branch rail and the Changes page are those of the worktree; the banner of
  `worktree.banner` says so. The folder of the window is then listed as another worktree, and can be neither renamed
  nor removed, nor given ignored files, from there. The choice is not remembered: the next time the window opens, the
  views show its own folder.
- **Uses**: `worktree.banner`, `repo.select`.

Acceptance:

- [ ] After **Show in Gitardo** on `feature/login`'s worktree, a commit made from the draft is made in that
      worktree, on `feature/login`, and the window's folder is untouched.
- [ ] While it is shown, the menu of the window's own folder in the Worktrees list has no **Rename…** and no
      **Remove…**.
- [ ] Closing the window and opening it again shows its own folder.

### `worktree.banner` — The banner of a worktree shown in this window

Says which worktree or submodule the views show while it is not the folder of the window, and goes back.

- **Kind**: View
- **Function**: `views::worktree::banner(state: &BannerState, ui) -> Vec<Intent>`
- **Entry points**: the top of the timeline and of the Changes page, while another worktree or a submodule is
  shown.
- **After**: **Back to `<name>`** points the views at the repository of the window again.

The banner says **Showing the worktree `<label>`**, followed by **on `<branch>`** when it has one, with its folder as the
tooltip, and the button **Back to `<name>`**, the name of the window's repository, with its folder as the tooltip. The
button is disabled while an operation runs.

Acceptance:

- [ ] While `feature/login`'s worktree is shown, the banner reads **Showing the worktree feature/login on feature/login**
      in both places.
- [ ] **Back to app** shows the window's folder again, and the banner goes.
- [ ] No banner shows while the views show the window's own folder.

### `worktree.rename` — Rename a worktree

Gives a worktree's folder another name with `git worktree move`, and its branch too if wanted.

- **Kind**: Operation
- **Function**: `features::worktree::rename(context, answers) -> Outcome`
- **Entry points**: **Rename…** in the context menu of a worktree; **Rename worktree…** in the menu of a branch another
  worktree has.
- **Offered when**: the worktree is not missing, not the main worktree, not the one the views show and not the folder
  of the window.
- **Asks**:
  1. **New name of its folder, which can have slashes** inside the folder of the worktrees, where the name is its path
     from there; **New name of its folder** anywhere else, where it is the name of its folder and a slash becomes `-`.
     The current name is the default, and the box says **In `<folder>`** or **`<folder>` already exists**. The name
     is made a folder name as in `worktree.folder-path`. The same name runs nothing.
  2. When it has a branch: **Rename the branch `<branch>` too** or **Keep the branch `<branch>`**.
  3. When renaming the branch: its new name, as `branch.rename` asks it. The default is the name typed for the folder
     when the folder was named after the branch and that name is free and valid, and the branch's own name otherwise.
- **Plan**: `git worktree move <folder> <new folder>`, then `git branch -m <branch> <new name>` when the branch is
  renamed, then the deletions of `worktree.remove-empty-folders` for the old place. The folders that lead to the new
  place are made with the move when they are missing, since git does not make them. On Windows, a move that fails
  because the folder is held is tried again (`worktree.recover-held-folder`).
- **Danger**: None. The question says that git cannot move a folder that another program has open, such as another
  window or a terminal in it.
- **Undo**: Not taken back by Undo: rename it again.
- **Refuses**: on the main worktree, **Git cannot move the main worktree, the folder the repository was created in.**;
  on the window's folder, **This worktree is the folder open in this window: rename it from another one.**; when the new
  folder exists by the time it runs, **`<folder>` already exists.**
- **Uses**: `worktree.folder-path`, `branch.rename`, `worktree.recover-held-folder`, `worktree.remove-empty-folders`.

Acceptance:

- [ ] `feature/login` renamed to `feature/signin`, with its branch, runs `git worktree move
      <trees>/feature/login <trees>/feature/signin` then `git branch -m feature/login feature/signin`, and the branch
      name was proposed as `feature/signin`.
- [ ] `feature/login` renamed to `hotfix/login` leaves no empty folder `feature` in the folder of the worktrees.
- [ ] A worktree outside the folder of the worktrees, renamed `a/b`, goes to a folder `a-b` beside where it was.
- [ ] The entry is not offered on the main worktree nor on the folder of the window.

### `worktree.copy-path` — Copy the path of a worktree

Puts the full path of a worktree's folder on the clipboard.

- **Kind**: Operation
- **Function**: `features::worktree::copy_path(context, answers) -> Outcome`
- **Entry points**: **Copy path** in the context menu of a worktree.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the path is on the clipboard.

Acceptance:

- [ ] The clipboard holds the worktree's folder as git lists it, also for a missing worktree.

### `worktree.remove` — Remove a worktree

Removes a worktree's folder with `git worktree remove`, and its branch too if wanted.

- **Kind**: Operation
- **Function**: `features::worktree::remove(context, answers) -> Outcome`
- **Entry points**: **Remove…** in the context menu of a worktree; **Remove worktree…** and **Remove worktree and
  delete branch…** in the menu of a branch another worktree has, which answer the question about the branch.
- **Offered when**: the worktree is not the main worktree, not the one the views show and not the folder of the window.
- **Asks**: when it has a branch and the entry did not say: **Remove the worktree** (**Keep the branch `<branch>`**,
  the default) or **Remove the worktree and delete `<branch>`** (**Only if it is merged**).
- **Reads**: on Windows, `worktree.read-folder-links`.
- **Plan**: on Windows, `unlink <path>` for each link of `worktree.read-folder-links`; then `git worktree remove
  <folder>`; then the deletions of `worktree.remove-empty-folders`; then `git branch -d <branch>` when the branch goes
  too, last, so that a branch git keeps leaves nothing else undone. When git refuses because the worktree has modified or untracked files (its message
  mentions them, or `--force`), Gitardo offers **Remove anyway**, the same plan with `git worktree remove --force <folder>`, Destructive,
  because **This worktree has modified or untracked files. They will be deleted with it.** On Windows, a removal that
  fails because the folder is held goes on as `worktree.recover-held-folder` says.
- **Danger**: Confirm, because the folder is deleted: **The directory is deleted.** followed by **Its branch and commits
  stay in the repository.**, or, with the branch, **The branch `<branch>` is deleted too if it is merged; if it is not,
  git keeps it and says so.**
- **Undo**: Cannot be undone: the folder is deleted. Its commits stay in the repository.
- **Refuses**: on the folder of the window, **This worktree is the folder open in this window: remove it from another
  one.**
- **Uses**: `worktree.read-folder-links`, `worktree.recover-held-folder`, `worktree.remove-empty-folders`.

Acceptance:

- [ ] On a clean worktree of `feature/login`, choosing to delete the branch runs `git worktree remove <folder>` then
      `git branch -d feature/login`.
- [ ] On a worktree with a modified file, git's refusal is followed by an offer with `--force`, shown as destructive,
      and nothing is deleted until it is accepted.
- [ ] A branch that is not merged is kept by `git branch -d`, and git's message is shown.
- [ ] On Windows, a worktree with a junction `node_modules` to the window's folder is removed with `unlink
      <folder>/node_modules` first, and the window's `node_modules` keeps all its files.
- [ ] Removing `feature/login`, the only worktree in `feature`, leaves no folder `feature` behind.

### `worktree.prune` — Prune missing worktrees

Has git forget the worktrees whose folders are gone, which frees their branches.

- **Kind**: Operation
- **Function**: `features::worktree::prune(context, answers) -> Outcome`
- **Entry points**: **Prune missing worktrees** in the context menu of a worktree and in the **…** of the Worktrees
  list.
- **Offered when**: at least one worktree is missing.
- **Plan**: `git worktree prune`.
- **Danger**: None.
- **Undo**: Not taken back by Undo: git forgets only worktrees whose folders are gone.

Acceptance:

- [ ] After a worktree's folder is deleted outside Gitardo, **Prune missing worktrees** is offered, runs
      `git worktree prune`, and the worktree is no longer listed.
- [ ] The branch it had can then be checked out here.

### `worktree.read-folder-links` — Read the links to folders outside a worktree

On Windows, git deletes what is inside a junction when it removes a worktree: the folder it links to, in the folder the
link came from. This read finds those links, so that they are taken out first.

- **Kind**: Read
- **Function**: `git::read::worktree::read_folder_links(git, request) -> Result<Vec<PathBuf>, GitError>`
- **Reads**: only on Windows, in the worktree: `git ls-files --others --directory -z` and
  `git ls-files --others --ignored --exclude-standard --directory -z`. Each entry that is a link is kept when it leads
  to a folder outside the worktree; each entry that is a folder is looked into, without going past a link. Elsewhere it
  returns nothing: git removes a link there, not what it leads to.
- **Uses**: by `worktree.remove` and `branch.clean-up`.

Acceptance:

- [ ] A worktree with a junction `node_modules` to another folder and a junction `packages/api/node_modules` returns
      both.
- [ ] A link to a folder inside the worktree, a link to a file and a link to nothing are not returned.
- [ ] Tracked files are not looked into.
- [ ] On macOS and Linux it returns nothing and runs nothing.

### `worktree.recover-held-folder` — Get past a folder that Windows holds

On Windows another program can hold a worktree's folder for a moment, such as a window or a terminal that has just
left it. A move or a removal that fails for that is not left half done.

- **Kind**: Derivation
- **Function**: `model::worktree::recover_held_folder(input) -> Recovery`
- **Uses**: by `worktree.rename`, `worktree.remove` and `branch.clean-up`.

The rules, only on Windows:

- `git worktree move` failing with `failed to move …: Permission denied`, `Invalid argument` or `Device or resource
  busy` changed nothing: it is run again, up to 20 times, 200 ms apart. The log says the first line of git's message
  followed by **Windows holds the folder, so it is tried again.**, and the command again when it succeeds. When it
  never succeeds, the last failure is the one shown.
- `git worktree remove` failing with `failed to delete '…` has already made git forget the worktree when
  `git worktree list --porcelain -z` no longer lists it. Then Gitardo deletes what is left of the folder, `delete
  <folder>` in the log after **Windows held the folder. It is no longer a worktree, and Gitardo deletes what is left of
  it.**, deleting links rather than following them, trying again up to 10 times with longer waits; and the plan goes
  on with the steps after it, such as the deletion of the branch. While git still lists the worktree, the failure is
  the operation's.
- Any other failure, and any failure elsewhere than on Windows, is the operation's.

Acceptance:

- [ ] A move that fails twice with `Permission denied` and then succeeds ends as a success, with two lines in the log
      that say it is tried again.
- [ ] A removal that fails with `failed to delete` while git no longer lists the worktree deletes the folder, logs
      `delete <folder>`, and then runs `git branch -d <branch>` when the branch goes too.
- [ ] A removal that fails while git still lists the worktree stops there and shows git's message.
- [ ] On macOS and Linux nothing is tried again.

### `worktree.remove-empty-folders` — Leave no empty folders behind

After a worktree is moved or removed, the folders that held only it go too, up to the folder of the worktrees.

- **Kind**: Derivation
- **Function**: `model::worktree::remove_empty_folders(input) -> Vec<Step>`
- **Uses**: by `worktree.rename`, `worktree.remove` and `branch.clean-up`.

The rules: from the folder that held the worktree upwards, each folder inside the folder of the worktrees gets a step
`delete <folder>` that deletes it only when it is empty by then, and the first one that is not empty ends it. The folder
of the worktrees itself, and anything outside it, is never deleted. On Windows a folder that is held for a moment is
tried again, 20 times, 100 ms apart.

Examples, with the folder of the worktrees `~/code/app.worktrees`:

- Removing `feature/login`, alone in `feature`: `feature` is deleted.
- Removing `feature/login` while `feature/signup` is there: nothing more is deleted.
- Removing a worktree in `~/elsewhere/x`: nothing more is deleted.

Acceptance:

- [ ] Each example gives the result it says.
- [ ] `~/code/app.worktrees` stays when its last worktree is removed.

### `worktree.branch-in-another-worktree` — The branch of another worktree

Git checks a branch out in one worktree at a time, so a branch that another worktree has is worked on there. This is
how the branch rail, the timeline and the menus treat it.

- **Kind**: Derivation
- **Function**: `model::worktree::branch_in_another_worktree(input) -> Option<WorktreeBranch>`, none for a branch
  no other worktree has
- **Uses**: by `refs.`, `graph.`, `branch.checkout`, `drag.`.

The rules, for a local branch that a worktree other than the one the views show has checked out:

- It has the worktree's icon at the end of its row among the local branches, and before the branch's icon on its
  label in the timeline; its tooltip says which worktree has it.
- A double click on it opens its worktree as `worktree.open` does, instead of checking it out; on a missing worktree it
  does nothing.
- Its context menu starts with what opens the worktree: **Open worktree in new window**, **Open worktree in external
  editor** (while `editor.external` is set), **Open terminal in worktree**, **Reveal worktree in File Explorer** (or its
  name on macOS and elsewhere, as in `worktree.reveal`) and **Show worktree in Gitardo**.
- **Check out**, **Delete** and **Create worktree…**, which git refuses, are not there. **Rename branch…**, **Rename
  worktree…**, **Remove worktree…** and **Remove worktree and delete branch…** are, the last three except for the main
  worktree and the folder of the window. What does not touch the branch's files, such as merging it into the current
  branch, comparing it or opening its pull request, is there as for any branch.
- Dragged onto a commit of the timeline, it offers no rebase.
- Checking out a remote branch whose local branch another worktree has opens that worktree instead; its menu starts with
  the same entries and has no **Check out … and rebase it onto this**.
- Seen from another worktree, the branch of the main worktree is the same, except that the main worktree can be neither
  renamed nor removed.
- When that worktree is missing, checking the branch out is refused with **The worktree of `<branch>` is missing from
  `<folder>`: prune the worktrees to free the branch.**

Acceptance:

- [ ] A branch another worktree has shows the worktree icon in the branch rail and on its label in the timeline, and its
      tooltip names the worktree.
- [ ] Its menu has no **Check out** and no **Delete**, and starts with **Open worktree in new window**.
- [ ] A double click on it opens the worktree and runs no checkout.
- [ ] Dragging it onto a commit offers no rebase, and still offers to merge.
- [ ] From another worktree, the menu of `main`, which the main worktree has, has no **Remove worktree…**.

### `worktree.open-in-worktree` — Open a branch in a worktree of its own

Works on a branch in its own worktree, with nothing asked: the worktree is made if it is not there yet, and opens.

- **Kind**: Operation
- **Function**: `features::worktree::open_in_worktree(context, answers) -> Outcome`
- **Entry points**: **Open in worktree** in the context menu of a local branch and of a remote branch; a double click on
  a branch while `branches.openIn` is `worktree`, where **Open in worktree** also comes before **Check out** in its
  menu.
- **Offered when**: the branch is not checked out here and no other worktree has it (for a remote branch: its local
  branch of the same name).
- **Plan**: when no worktree has the branch, in the folder `worktree.folder-path` gives for the branch's name: `git
  worktree add <folder> <branch>`; for a remote branch with no local branch of its name, `git worktree add --track -b
  <branch> <folder> <remote>/<branch>`, so that the local branch follows it; then the steps of
  `worktree-setup.link-and-copy-steps` and `worktree-setup.command-steps`. When a worktree has it already, no steps.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Remove…** on the worktree removes it.
- **After**: opens the worktree as `worktree.open` does, once the commands of `worktree-setup.command-steps` have all
  succeeded; when one fails, it says which and offers to open it all the same.
- **Refuses**: **`<branch>` is checked out in this window.**; **The worktree of `<branch>` is missing from `<folder>`:
  prune the worktrees to free the branch.**; **`<folder>` already exists.**
- **Settings**: `branches.openIn` (`checkout`, the default, or `worktree`), `worktrees.openIn`, `worktrees.folder`.
- **Uses**: `worktree.folder-path`, `worktree.open`, `worktree-setup.link-and-copy-steps`,
  `worktree-setup.command-steps`.

Acceptance:

- [ ] On `feature/login`, which no worktree has, it runs `git worktree add <trees>/feature/login feature/login` and
      opens it, with no question.
- [ ] On `origin/fix` with no local `fix`, it runs `git worktree add --track -b fix <trees>/fix origin/fix`, and `fix`
      follows `origin/fix` afterwards.
- [ ] On `origin/fix` with a local `fix`, it runs `git worktree add <trees>/fix fix`.
- [ ] With `branches.openIn` set to `worktree`, a double click on a branch makes and opens its worktree, and the folder
      of the window stays on its branch.
- [ ] A second **Open in worktree** on the same branch opens the worktree made the first time and creates nothing.

### `worktree.switch` — Switch worktree

Lists the worktrees with their branches and how they stand, and opens the one picked.

- **Kind**: Operation
- **Function**: `features::worktree::switch(context, answers) -> Outcome`
- **Entry points**: Quick search, **Switch Worktree…**; **Switch worktree…** in the **…** of the Worktrees list in the
  branch rail.
- **Asks**: **Switch Worktree**, **The worktree to open**: the worktrees that are not missing, each with the main
  worktree's icon or the worktree's icon and its label, then its branch (or **detached at `<short>`**) with `↑n ↓m`, a
  pencil with the number of its files not committed, its pull request as `#<number>`, and **this window** for the one the
  views show, which comes last; its folder under it. Typing matches the label, the description and the folder. After a
  separator, **Create worktree…**.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the worktree picked opens as `worktree.open` does; **Create worktree…** starts `worktree.create`; the
  worktree of this window does nothing.
- **Settings**: `worktrees.openIn`.
- **Uses**: `worktree.read-list`, `worktree.open`, `worktree.create`.

Acceptance:

- [ ] With three worktrees, the one of this window is listed last, marked **this window**.
- [ ] Typing part of a folder's path finds its worktree.
- [ ] Picking another worktree opens it where `worktrees.openIn` says; the last entry starts creating a worktree.

### `worktree-setup.settings-in-effect` — What new worktrees get, as set

Which link and copy patterns and which commands apply to new worktrees of a repository, and where each comes from.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::settings_in_effect(input) -> SetupSettings`
- **Settings**: `worktrees.linkFolders`, `worktrees.copyFiles`, `worktrees.setupCommands`; VS Code's
  `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles` in the repository's `.vscode/settings.json`.

The rules:

- For each of `worktrees.linkFolders` and `worktrees.copyFiles` on its own: the value in the repository's
  `.gitardo/settings.json` when the key is there, even as an empty list; otherwise the user's when the key is there;
  otherwise VS Code's matching setting, `git.worktreeSymlinkFolders` or `git.worktreeIncludeFiles`, in the repository's
  `.vscode/settings.json`, when it has a pattern; otherwise nothing.
- `worktrees.setupCommands`: the repository's, even empty, then the user's; VS Code has no such setting.
- Patterns are kept trimmed, without empty ones, without line breaks and without the same one twice; commands trimmed,
  without empty ones and without line breaks, in their order.
- Each value comes with where it comes from: the repository, the user, VS Code's setting, or nothing set.

Acceptance:

- [ ] With `worktrees.linkFolders: []` in the repository and `["node_modules/"]` for the user, nothing is linked.
- [ ] With only `git.worktreeSymlinkFolders: ["node_modules/"]` in `.vscode/settings.json`, `node_modules/` is linked,
      from VS Code's setting.
- [ ] `worktrees.copyFiles` set for the user and `worktrees.linkFolders` only in VS Code's setting take the copies from
      the user and the links from VS Code.
- [ ] `[" /.env ", "", "/.env"]` gives `["/.env"]`.

### `worktree-setup.folders-to-link` — The folders to link

Which folders of the checkout a new worktree gets as links, from the link patterns, worked out as VS Code's Git
extension works them out, so that the same patterns give the same folders.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::folders_to_link(input) -> Vec<String>`
- **Reads**: what git ignores, `git ls-files --others --ignored --exclude-standard -z` and the same with `--directory`;
  what the patterns match among the files git does not track, `git ls-files --others --ignored --exclude-from=<file of
  the patterns> -z`; whether git ignores a folder, and whether the patterns name it as a folder, `git check-ignore
  --no-index -z --stdin` in the checkout and in an empty repository whose excludes file is the patterns.

The rules: a folder is linked when it holds a file that git ignores and that the patterns match, git ignores the folder
itself, the patterns name it as a folder, and git ignores it whole or it is inside a folder git ignores whole. Of those,
only the outermost: none inside another one linked. The patterns are in `.gitignore` syntax.

Examples:

| Patterns | Ignored in the checkout | Linked |
| --- | --- | --- |
| `node_modules/` | `node_modules/`, `packages/api/node_modules/` | both |
| `node_modules/` | `node_modules/` holding `node_modules/a/node_modules/` | `node_modules` only |
| `/packages/api/node_modules/` | `node_modules/`, `packages/api/node_modules/` | `packages/api/node_modules` |
| `node_modules/` | a tracked `node_modules/` | nothing |
| `.vscode/` | `.vscode/` ignored but for a tracked `.vscode/launch.json` | nothing |
| `.env` | a file `.env` | nothing: only folders are linked |

Acceptance:

- [ ] Each example of the table gives what it says.
- [ ] No pattern gives a folder that git does not ignore.
- [ ] The paths are relative to the checkout, with forward slashes, on every system.

### `worktree-setup.paths-to-copy` — The files and folders to copy

Which files and folders of the checkout a new worktree gets as copies, from the copy patterns.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::paths_to_copy(input) -> Vec<String>`
- **Reads**: what git ignores, `git ls-files --others --ignored --exclude-standard -z`, and the same with `--directory
  --no-empty-directory`; what the patterns match, `git ls-files --others --ignored --exclude-from=<file of the patterns>
  -z`; what the worktree has from its commit, `git ls-tree -r -z --name-only <commit>`.

The rules: the files git ignores that the patterns match are copied, except what is inside a folder that is linked, and
except what clashes with what the worktree has from its commit: a file of that path, a folder it has files in, or a path
under a file it has. A folder that git ignores whole, all of whose files are copied, is copied as a whole instead of
file by file.

Examples:

| Patterns | In the checkout | Copied |
| --- | --- | --- |
| `/.env` | `.env`, `packages/api/.env` | `.env` |
| `.env` | `.env`, `packages/api/.env` | both |
| `.vscode/` | `.vscode/` ignored whole, with two files | `.vscode` |
| `*.local.json` | `node_modules/x.local.json`, with `node_modules/` linked | nothing |
| `/config.json` | an ignored `config.json` the commit also has | nothing |

Acceptance:

- [ ] Each example of the table gives what it says.
- [ ] A folder git ignores whole with one file that the patterns do not match is copied file by file.
- [ ] No pattern gives a path that git does not ignore.

### `worktree-setup.link-and-copy-steps` — Linking and copying into a new worktree

What a new worktree gets of what git ignores in the folder the views show, as steps after `git worktree add`.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::link_and_copy_steps(input) -> SetupSteps`
- **Settings**: `worktrees.linkFolders`, `worktrees.copyFiles`, with VS Code's as the fallback.
- **Uses**: `worktree-setup.settings-in-effect`, `worktree-setup.folders-to-link`, `worktree-setup.paths-to-copy`; by
  `worktree.create`, `worktree.create-from-branch`, `worktree.create-at-commit`, `worktree.open-in-worktree`,
  `worktree-setup.bring-into-worktree`, `worktree-setup.bring-into-other-worktrees`.

The rules:

- The folders to link come first, each a `Link` step, `symlink <folder>` in the log; then the files and folders to
  copy, each a `Copy` step, `copy <path>`.
- On Windows a link to a folder is a junction, which needs no rights.
- Nothing is made where the worktree has something already, a broken link included: the log says **`<path>` is in the
  worktree already.** A folder to link that holds the worktree is not linked: **`<folder>` holds the worktree, so it is
  not linked.** Nothing is made under a folder of the worktree that is a link or a file.
- A copy copies a folder with all it has, and copies links as links.
- These steps go on after one of them fails; once they are all done, Gitardo says what failed: **Gitardo: `<what>`
  failed: the log says why.**, or **`<n>` folders or files failed** for more than one. The log has the reason of each.
- The plan's note says the patterns and where from: **Then Gitardo will link the folders matching `<patterns>`, and copy
  what matches `<patterns>`, from `<folder>`, where git ignores them.**, so that the preview of `commandPreview` says
  it along with the steps.
- With no pattern in effect there are no steps and no note.

Acceptance:

- [ ] With `node_modules/` to link and `/.env` to copy, a new worktree's plan shows `symlink node_modules` and `copy
      .env` after `git worktree add`, and the note names both patterns and the folder they come from.
- [ ] A worktree whose commit has `.env` gets no copy of it, and the log says why.
- [ ] A copy that fails for lack of rights does not stop the next one, and one warning names it at the end.
- [ ] On Windows, `node_modules` in the new worktree is a junction to the window's `node_modules`.

### `worktree-setup.command-steps` — Commands in a new worktree

The commands of `worktrees.setupCommands`, such as `npm ci`, run in a new worktree once it has been created and its
links and copies made.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::command_steps(input) -> SetupSteps`
- **Settings**: `worktrees.setupCommands`.
- **Uses**: `worktree-setup.settings-in-effect`; by `worktree.create`, `worktree.create-from-branch`,
  `worktree.create-at-commit`, `worktree.open-in-worktree`.

The rules:

- Each command is a `Run` step in the worktree, `run <command>` in the log, after the links and copies.
- They run one after the other in **Tasks**, in the Activity drawer, each as a task named after the worktree's folder,
  with the shell of the system, so that what they print is seen and what they ask can be answered. One command to a
  line, with no `&&` between them, which Windows PowerShell does not know.
- One that fails or is stopped stops those after it, and Gitardo says which: **`<command>` failed with exit code `<n>`
  in `<worktree>`, so the `<k>` commands after it did not run.**, or **did not finish** for one stopped.
- They run only in a trusted folder. Otherwise none runs, the log says **The commands of worktrees.setupCommands did not
  run: this folder is not trusted.**, and Gitardo says **The commands for new worktrees did not run in `<worktree>`:
  this folder is not trusted.**
- Other operations do not wait for them. What the operation does once it succeeded, such as opening the worktree, waits
  for the last of them; when one did not succeed, Gitardo says why and offers to open the worktree all the same, with
  the choices of `worktree.create`.
- The plan's note adds **After that it will run `<command>`, then `<command>`, in Tasks, in the worktree.**
  (**Then Gitardo will run…** when nothing is linked or copied).

Acceptance:

- [ ] With `["npm ci", "npm run build"]`, the plan ends with `run npm ci` and `run npm run build`, and Tasks, in the
      Activity drawer, shows both, one after the other, with the worktree as their folder.
- [ ] When `npm ci` exits with 1, `npm run build` does not run, and the warning says so and offers to open the worktree.
- [ ] A command that waits for input can be answered in Tasks, in the Activity drawer.
- [ ] In a folder that is not trusted, no command runs and the worktree is still created, linked and copied.
- [ ] **Open in worktree** opens the worktree only once both commands exit with 0.

### `worktree-setup.read-ignored` — Read what git ignores, with its size

What git ignores in the folder the views show, as the page that chooses what new worktrees get lists it.

- **Kind**: Read
- **Function**: `git::read::worktree_setup::read_ignored(git, request) -> Result<Vec<IgnoredEntry>, GitError>`
- **Reads**: `git ls-files --others --ignored --exclude-standard --directory -z`: a folder whose content git ignores
  whole as one entry, such as `node_modules/`, and each other file on its own. An entry inside a folder also listed is
  dropped, as git lists both for `.vscode/*` and `.vscode/` in one `.gitignore`. On disk, each entry's size: a file's
  bytes; a folder's files and bytes, without following links, counting at most 5,000 files and saying when it has more;
  a link is not looked into.
- **Uses**: `worktree-setup.propose` for what each entry is and what is proposed for it.

Acceptance:

- [ ] A checkout with `node_modules/` and `.env` ignored gives two entries, `node_modules` as a folder with its files
      and bytes, and `.env` as a file.
- [ ] A folder with 6,000 files is said to have over its size, 5,000+ files.
- [ ] A `node_modules` that is a link already is listed as a link, with no size.
- [ ] Nothing is listed twice.

### `worktree-setup.propose` — What is proposed for what git ignores

What each thing git ignores is, which section of the page it goes in, and what is proposed for it.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::propose(input) -> Proposal`

The rules, in this order, the first that applies:

| Kind (section) | What it is | Proposed |
| --- | --- | --- |
| **Dependencies** | a folder named `node_modules`, `bower_components`, `jspm_packages`, `.venv`, `venv`, `.pnpm-store`, `Pods`, `Carthage`, `vendor`, `.gradle` or `.m2`, or the folder `.yarn/cache` | **Link** |
| **Build output** | a folder named `dist`, `build`, `out`, `target`, `bin`, `obj`, `.next`, `.nuxt`, `.svelte-kit`, `.angular`, `.turbo`, `.parcel-cache`, `.cache`, `coverage`, `.nyc_output`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.tox`, `.dart_tool`, `DerivedData`, `tmp` or `temp` | **Nothing** |
| **Other worktrees** | a folder named `worktrees` or `.worktrees` | **Nothing** |
| **Logs and files the system leaves** | `.DS_Store`, `Thumbs.db`, `desktop.ini`, or a name ending in `.log`, `.swp`, `.swo`, `.tmp`, `.bak` or `~`, without regard to case | **Nothing** |
| **Settings and secrets of this machine** | a file named `.env` or `.env.<anything>`, `.envrc`, `.npmrc`, `.yarnrc`, `.yarnrc.yml`, `.pypirc`, `.tool-versions`, `local.properties`, `appsettings.Development.json`, `appsettings.Local.json`, or `<name>.local` and `<name>.local.<extension>`, without regard to case | **Copy** |
| **Settings of the editor** | a folder named `.vscode`, `.idea`, `.fleet` or `.zed` | **Copy** |
| **Not one Gitardo knows** | anything else | **Nothing** |

Each section says why: **Nothing is proposed for them**; **The same in every worktree, as long as the branches agree on
them**; **Kept out of commits, and needed in each worktree**; **Each worktree starts with the same**; **Each worktree
makes its own**; **Not to be inside one another**; **No worktree needs them**. The page lists the sections in this order:
not known, dependencies, settings and secrets, editor, build output, other worktrees, logs.

The pattern a line chooses by: a folder of dependencies by its name wherever it is, `node_modules/`, as each package of a
repository can have one; anything else by its path from the top, `/.env`, `/config/`.

Acceptance:

- [ ] `packages/web/node_modules` is a dependency, proposed **Link**, with the pattern `node_modules/`.
- [ ] `.env.local` and `config/app.local.json` are settings and secrets, proposed **Copy**, with the patterns
      `/.env.local` and `/config/app.local.json`.
- [ ] `build` is build output, and `debug.log` is a log, both proposed **Nothing**.
- [ ] `.vscode` as a folder is proposed **Copy**; a file named `.vscode` is not one Gitardo knows.
- [ ] `notes` is not one Gitardo knows, proposed **Nothing**.

### `worktree-setup.propose-commands` — Commands proposed from lock files

The commands proposed for new worktrees from the lock files the repository commits at its top.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::propose_commands(input) -> Vec<CommandProposal>`
- **Reads**: the files committed at the top among those below,
  `git ls-files -z -- ':(top,literal)<file>'…`, and `packageManager` of `package.json` when a JavaScript lock file is
  there.

The rules:

- One for JavaScript, the one `packageManager` names when it has a lock file, otherwise the first of: `pnpm install
  --frozen-lockfile` for `pnpm-lock.yaml`; `yarn install --frozen-lockfile` for `yarn.lock`, or `yarn install
  --immutable` when `.yarnrc.yml` is committed or `packageManager` names Yarn 2 or later; `bun install` for `bun.lock`
  or `bun.lockb`; `npm ci` for `package-lock.json` or `npm-shrinkwrap.json`.
- One for Python: `uv sync --locked` for `uv.lock`, or else `poetry install` for `poetry.lock`.
- `cargo fetch` for `Cargo.lock`, `go mod download` for `go.mod`, `bundle install` for `Gemfile.lock`.
- A `requirements.txt` gets none: no one command installs it into an environment of the worktree's own, whatever the
  shell.
- Each comes with the file it is proposed for.

Acceptance:

- [ ] `package-lock.json` and `pnpm-lock.yaml` both committed propose `pnpm install --frozen-lockfile` only.
- [ ] The same with `packageManager: "npm@10.2.0"` proposes `npm ci` only.
- [ ] `yarn.lock` with `packageManager: "yarn@4.1.0"` proposes `yarn install --immutable`.
- [ ] `uv.lock`, `poetry.lock` and `Cargo.lock` propose `uv sync --locked` and `cargo fetch`.
- [ ] A lock file in a subfolder and a `requirements.txt` propose nothing.

### `worktree-setup.read-what-patterns-take` — Ask git what the patterns take

What the link and copy patterns of the page take from the folder the views show, as a whole and each on its own.

- **Kind**: Read
- **Function**: `git::read::worktree_setup::read_what_patterns_take(git, request) -> Result<SetupPreview, GitError>`
- **Reads**: the reads of `worktree-setup.folders-to-link` and `worktree-setup.paths-to-copy` for all the patterns, as
  for a worktree that has nothing of them yet; then, for each link pattern alone, how many folders it links, and for
  each copy pattern alone, how many files or folders it copies with the links of all the patterns in place.
- **Uses**: by `worktree-setup.page`, when the page opens and at **Look again**.

Acceptance:

- [ ] With `node_modules/` and `/.env` on a checkout that has both, it says one folder linked and one path copied, each
      pattern taking 1.
- [ ] A copy pattern that matches only files inside a linked folder takes 0.

### `worktree-setup.page` — Choose what new worktrees get

A page that lists what git ignores in the folder the views show, proposes what new worktrees get of it, and edits and
saves the patterns and the commands.

- **Kind**: View
- **Function**: `views::worktree_setup::page(state, ui) -> Vec<Intent>`
- **Entry points**: Quick search, **Choose What New Worktrees Get…**; **Choose what new worktrees get…** in the
  **…** of the Worktrees list in the branch rail; **Choose What It Gets…** offered after the first worktree is
  created; **Choose What New Worktrees Get…** in the messages of the Files page entries. The page is named **New
  Worktrees · `<repository>`** in its trail; asking for it again shows the one there.
- **Reads**: `worktree-setup.read-ignored`, `worktree-setup.settings-in-effect`, `worktree-setup.propose-commands`, the
  number of the other worktrees, and `worktree-setup.read-what-patterns-take` when it opens and at **Look again**.
- **Uses**: `worktree-setup.propose`, `worktree-setup.choose`, `worktree-setup.save`, `worktree-setup.remove-from`,
  `worktree-setup.edit-settings-file`, `worktree-setup.bring-into-other-worktrees`.

What it shows:

- A header, **What new worktrees get**, which says what a new worktree has and gets, then **Linked now**, **Copied
  now** and **Run now**, each with what applies and where it comes from: **the settings of `<repository>`**, **your user
  settings**, **VS Code's git.worktreeSymlinkFolders** (or **git.worktreeIncludeFiles**), or **nothing**.
- **What git ignores here**, with how many, **Use what is proposed** and **Look again** (**Looking…** while it reads).
  While it is first read, **Looking at what git ignores…**. With nothing ignored, **Git ignores nothing here, so new
  worktrees have all there is.**
- A table, **Path**, **Size**, **New worktrees get**, in sections by kind, in the order of `worktree-setup.propose`. Each
  section's line has its title, how many, why, its size, and what its lines get in short (**all linked**, **2 copied**,
  **nothing**); it opens and closes on a click. At first the sections of what is not known, of dependencies, of
  settings and secrets and of the editor are open, and any other where something is linked or copied; the ones opened
  or closed by hand stay so, also after the page is closed and opened again.
- Each line: the icon of the file, its path, its size (**12 MB · 1,234 files**, **over 1.2 GB · 5,000+ files** past what
  is counted, **a link**), and three buttons, **Link**, **Copy**, **Nothing**, with the one it gets pressed and the one
  proposed marked as **(proposed)** in its tooltip. **Link** is disabled on a file: **Only folders are linked**. Notes
  after it: **A link already, in this checkout**; **Only part of it is linked** (or **copied**) **, by a pattern below**;
  **Large to copy into each worktree: better linked, or made again** when copying more than 100 MB.
- What has the same name in several places, such as the `node_modules` or the `.env.local` of each package, is one line
  with **`<n>` places**, whose buttons choose for all of them by the pattern of the name; **Not the same in each place**
  when they differ. Unfolded, it lists each place, whose buttons choose for that one alone.
- A section or a name lists 200 lines at most, then **Show the other `<n>`**.
- **Patterns**: **Folders to link** (`worktrees.linkFolders`, **On Windows, junctions.**) and **Files and folders to
  copy** (`worktrees.copyFiles`, **Not what is inside a linked folder.**), each pattern with what it takes, **`<n>`
  folders** or **`<n>` files or folders**, or **takes nothing here** as a warning; a button to remove it; a box and
  **Add**, where patterns separated by commas are added each.
- **Commands to run** (`worktrees.setupCommands`): numbered, each edited where it is, moved up or down, or removed; a box
  and **Add**; **Proposed from what is committed here:** with a button for each proposal not in the list yet, which adds
  it at the end.
- At the bottom: **Save in** the repository (`.gitardo/settings.json`, **for everyone who works on it**) or **User**
  (**For every repository**), where what applies comes from by default, the repository when that is VS Code's setting or
  nothing; the box **Also in VS Code's git.worktreeSymlinkFolders and git.worktreeIncludeFiles, for the worktrees VS Code
  creates**, while saving in the repository; **Save**, disabled while what is shown is what is saved there; **Discard
  changes**; **Bring into the other worktree** (or **the other `<n>` worktrees**), disabled while there are changes not
  saved (**Save first: the worktrees get what is saved**), while there is no other worktree or no pattern; **Remove from
  `<place>`** while anything is set there; **Edit in settings file**.

How it behaves:

- Choosing on a line, **Use what is proposed** and editing the patterns change the patterns of the page at once,
  without running git; each line then shows what its own patterns give it.
- Git is asked what the patterns take only when the page opens and at **Look again**, which also reads again what git
  ignores. Until then, after a change: **Changed since Gitardo last looked: each line shows what is chosen for it, and
  Look again checks what the patterns take as a whole.** When git cannot say: **Git could not say what the patterns
  take:** and its message.
- The page follows what is changed in the settings elsewhere: what applies, the header and the lines change; changes
  made on the page and not saved are kept.
- Opening the page counts as having been offered it: `worktree.create` no longer offers **Choose What It Gets…**.

Acceptance:

- [ ] A checkout with `node_modules/`, `packages/api/node_modules/`, `.env` and `dist/` ignored shows `node_modules/`
      as one line of 2 places under **Dependencies**, `.env` under **Settings and secrets of this machine**, and the
      **Build output** section closed.
- [ ] **Use what is proposed** puts `node_modules/` among the folders to link and `/.env` among the files to copy, and
      runs no git command.
- [ ] Choosing **Nothing** for one place of `node_modules` replaces `node_modules/` with `/node_modules/` for the other
      place, which stays linked.
- [ ] A pattern that matches nothing shows **takes nothing here** after **Look again**.
- [ ] A section closed by hand is closed when the page is opened again.
- [ ] A change of `worktrees.copyFiles` made in the settings file shows on the page without closing it.
- [ ] The page reads the same in the dark and the light theme.

### `worktree-setup.choose` — Choose what a line gets

How the buttons of the page change the patterns: by the pattern of a path or of a name, keeping the patterns that take
more than the line.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::choose(input) -> SetupPatterns`

The rules:

- The patterns of a line are those of its path (`/packages/api/.env`, `/build/`) and of its name (`.env`, `build/`).
  Choosing for a line takes them out of both lists, and puts its pattern (`worktree-setup.propose`) in the list of what
  is chosen: **Link** in the folders to link, **Copy** in the files to copy, **Nothing** in neither. Patterns that take
  more than it, such as `*.local`, are left.
- Choosing for a name in several places does the same with the pattern of the name, `node_modules/` or `.env.local`,
  which takes the ones to come as well, and takes out the patterns of each place.
- Choosing for one place of such a name uses the pattern of its path. When the pattern of the name gave them all
  something else, it gives way to a pattern of the path of each other place, so that they keep what they had. Choosing
  for one place what the name gives them all already only takes out the pattern of its path.
- **Use what is proposed** gives each name in several places its proposal by the pattern of the name, and each other
  line its proposal by its own pattern.
- What a line gets is said from its own patterns when it has one, and otherwise from what git said the patterns take
  when it was last asked; a pattern of its own taken out since leaves it nothing.

Acceptance:

- [ ] Choosing **Copy** for `.env` with `["/.env"]` among the folders to link moves it to the files to copy.
- [ ] Choosing **Nothing** for `node_modules` in 3 places takes `node_modules/` and each `/…/node_modules/` out.
- [ ] Choosing **Copy** for one of 2 `.env.local` while `.env.local` is linked gives `/a/.env.local` to the folders to
      link and `/b/.env.local` to the files to copy, and takes `.env.local` out.
- [ ] `*.local` among the files to copy stays after choosing **Nothing** for `.env.local`.

### `worktree-setup.save` — Save what new worktrees get

Writes the patterns and the commands of the page to the settings of the repository or of the user.

- **Kind**: Operation
- **Function**: `features::worktree_setup::save(context, answers) -> Outcome`
- **Entry points**: **Save** on `worktree-setup.page`.
- **Offered when**: what the page shows is not what is saved in the place chosen, or the box of VS Code's settings is
  ticked.
- **Plan**: `write <file>` of the settings file of the place chosen, the repository's `.gitardo/settings.json` or the
  user's `settings.json`, with `worktrees.linkFolders`, `worktrees.copyFiles` and `worktrees.setupCommands` set; an
  empty list of commands is written only where one is set somewhere, as it would change nothing else. With the box
  ticked, also `write <repository>/.vscode/settings.json` with `git.worktreeSymlinkFolders` and
  `git.worktreeIncludeFiles` set to the same patterns. The rest of each file, comments included, is kept.
- **Danger**: None.
- **Undo**: Not taken back by Undo: change the patterns again, or **Remove from**.
- **After**: the page shows what is saved, with no changes pending.
- **Uses**: `worktree-setup.settings-in-effect`.

Acceptance:

- [ ] Saved in the repository, `.gitardo/settings.json` has the three keys, and a comment the file had is still there.
- [ ] With no commands anywhere, saving patterns writes no `worktrees.setupCommands`.
- [ ] With the box ticked, `.vscode/settings.json` gets `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles`
      with the same patterns, and nothing else in it changes.
- [ ] The command log shows a `write` line for each file written.

### `worktree-setup.remove-from` — Remove the settings of new worktrees from a place

Takes the three settings out of the place chosen on the page, so that what is set elsewhere applies again.

- **Kind**: Operation
- **Function**: `features::worktree_setup::remove_from(context, answers) -> Outcome`
- **Entry points**: **Remove from `<place>`** on `worktree-setup.page`, its tooltip **What is set elsewhere applies
  again**.
- **Offered when**: one of `worktrees.linkFolders`, `worktrees.copyFiles` or `worktrees.setupCommands` is set in the
  place chosen.
- **Plan**: `write <file>` of the place chosen without the three keys. VS Code's own settings are not changed.
- **Danger**: None.
- **Undo**: Not taken back by Undo: save them again.
- **After**: the page shows what applies now.

Acceptance:

- [ ] With `worktrees.linkFolders: []` in the repository and `["node_modules/"]` for the user, removing from the
      repository makes `node_modules/` apply, from the user.
- [ ] VS Code's `git.worktreeSymlinkFolders` stays in `.vscode/settings.json`.

### `worktree-setup.edit-settings-file` — Edit the settings of new worktrees in their file

Opens the settings file of the place chosen on the page, to edit the patterns and the commands by hand.

- **Kind**: Operation
- **Function**: `features::worktree_setup::edit_settings_file(context, answers) -> Outcome`
- **Entry points**: **Edit in settings file** on `worktree-setup.page`; **Edit in settings.json** on the settings
  page (`settings.edit`), for the file chosen there.
- **Plan**: when the repository's `.gitardo/settings.json` is chosen and is not there yet, `write
  <repository>/.gitardo/settings.json` with an empty object; otherwise no steps.
- **Danger**: None.
- **Undo**: Not taken back by Undo: a file it made stays.
- **After**: the file opens in the File page, at `worktrees.linkFolders` when it is set.
- **Uses**: `file-view.`.

Acceptance:

- [ ] In a repository without `.gitardo/settings.json`, it makes the file with `{}` and opens it.
- [ ] With **User** chosen, the user's settings file opens.

### `worktree-setup.link-into-new-worktrees` — Link a folder into new worktrees

Has new worktrees link a folder of the checkout, chosen in the Files page, by the pattern of its path.

- **Kind**: Operation
- **Function**: `features::worktree_setup::link_into_new_worktrees(context, answers) -> Outcome`
- **Entry points**: **Link into New Worktrees** in the context menu of a folder in the Files page, for the folders
  selected.
- **Offered when**: the folder is in a repository, and no link pattern of its own path, `/<path>/` or `/<path>`, names
  it already; otherwise **Stop Linking into New Worktrees** is there in its place.
- **Reads**: what the patterns would take before and after, as `worktree-setup.explain-change` needs it.
- **Plan**: `write <file>` of the settings where `worktrees.linkFolders` is set now, the repository's or the user's;
  where it is not set, the repository's `.gitardo/settings.json`, starting from VS Code's patterns when they apply. The
  pattern `/<path>/` goes in `worktrees.linkFolders`, and the patterns of its path come out of `worktrees.copyFiles`.
  VS Code's own settings are not changed. No steps when nothing would change.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Stop Linking into New Worktrees** takes it out.
- **After**: says what became of each folder (`worktree-setup.explain-change`) and where it was saved, **Saved in the
  settings of the repository** (or **of the user**), with **Choose What New Worktrees Get…**, which opens
  `worktree-setup.page` for its repository. A message that changed nothing is a warning.
- **Refuses**: for a folder in no repository of the workspace, **`<name>` is in no repository of the workspace.**
- **Uses**: `worktree-setup.explain-change`, `worktree-setup.settings-in-effect`.

Acceptance:

- [ ] On `packages/api/node_modules`, ignored, it adds `/packages/api/node_modules/` to `worktrees.linkFolders` and
      says **packages/api/node_modules will be linked into new worktrees.**
- [ ] On a folder git tracks it writes nothing and says **Git tracks files in `<folder>`: each worktree has them from
      its commit.**
- [ ] With `node_modules/` among the patterns, on `node_modules` it writes nothing and says it is linked already, by
      the pattern `node_modules/`.
- [ ] With only VS Code's `git.worktreeSymlinkFolders: ["node_modules/"]`, the repository's file gets both
      `node_modules/` and the new pattern, and `.vscode/settings.json` is unchanged.
- [ ] Two folders selected in two repositories each go to the settings of their own repository.

### `worktree-setup.copy-into-new-worktrees` — Copy a file or a folder into new worktrees

Has new worktrees get a copy of a file or a folder of the checkout, chosen in the Files page, by the pattern of its
path.

- **Kind**: Operation
- **Function**: `features::worktree_setup::copy_into_new_worktrees(context, answers) -> Outcome`
- **Entry points**: **Copy into New Worktrees** in the context menu of a file or a folder in the Files page, for those
  selected.
- **Offered when**: it is in a repository, and no copy pattern of its own path names it already; otherwise **Stop
  Copying into New Worktrees** is there in its place.
- **Reads**: what the patterns would take before and after, as `worktree-setup.explain-change` needs it.
- **Plan**: as `worktree-setup.link-into-new-worktrees`, with `/<path>` (or `/<path>/` for a folder) going in
  `worktrees.copyFiles` and the patterns of its path coming out of `worktrees.linkFolders`. Of a folder git tracks part
  of, what git ignores in it is copied.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Stop Copying into New Worktrees** takes it out.
- **After**: as `worktree-setup.link-into-new-worktrees`.
- **Refuses**: as `worktree-setup.link-into-new-worktrees`.
- **Uses**: `worktree-setup.explain-change`, `worktree-setup.settings-in-effect`.

Acceptance:

- [ ] On `.env`, ignored, it adds `/.env` to `worktrees.copyFiles` and says **.env will be copied into new worktrees.**
- [ ] On a folder git tracks part of, it adds the pattern and says **What git ignores in `<folder>` will be copied into
      new worktrees.**
- [ ] On a file inside a linked `node_modules`, it writes nothing and says it is in `node_modules`, which the pattern
      links into new worktrees, so nothing in it is copied.
- [ ] On a file git does not ignore, it writes nothing and says **Git does not ignore `<file>`: only what it ignores is
      linked or copied into new worktrees.**

### `worktree-setup.stop-linking` — Stop linking a folder into new worktrees

Takes the link pattern of a folder's own path out, from the Files page.

- **Kind**: Operation
- **Function**: `features::worktree_setup::stop_linking(context, answers) -> Outcome`
- **Entry points**: **Stop Linking into New Worktrees** in the context menu of a folder in the Files page, for those
  selected.
- **Offered when**: a link pattern of its own path, `/<path>/` or `/<path>`, is in `worktrees.linkFolders` as in effect.
- **Plan**: `write <file>` of the settings where `worktrees.linkFolders` is set now, without the patterns of its path.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Link into New Worktrees** puts it back.
- **After**: says **New worktrees will no longer get `<folder>`.**, or, when another pattern still gives it something,
  **`<folder>` is still linked into new worktrees, by the pattern `<pattern>`.**; and where it was saved, with **Choose
  What New Worktrees Get…**.
- **Uses**: `worktree-setup.explain-change`.

Acceptance:

- [ ] With `["/node_modules/", "node_modules/"]`, stopping on `node_modules` leaves `["node_modules/"]` and says it is
      still linked by `node_modules/`.
- [ ] With `["/node_modules/"]` alone, it leaves `[]` and says new worktrees will no longer get it.

### `worktree-setup.stop-copying` — Stop copying into new worktrees

Takes the copy pattern of a file's or a folder's own path out, from the Files page.

- **Kind**: Operation
- **Function**: `features::worktree_setup::stop_copying(context, answers) -> Outcome`
- **Entry points**: **Stop Copying into New Worktrees** in the context menu of a file or a folder in the Files page, for
  those selected.
- **Offered when**: a copy pattern of its own path is in `worktrees.copyFiles` as in effect.
- **Plan**: `write <file>` of the settings where `worktrees.copyFiles` is set now, without the patterns of its path.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Copy into New Worktrees** puts it back.
- **After**: as `worktree-setup.stop-linking`, with **copied**.
- **Uses**: `worktree-setup.explain-change`.

Acceptance:

- [ ] With `["/.env"]`, stopping on `.env` leaves `[]` and says **New worktrees will no longer get .env.**
- [ ] With `["/.env", ".env"]`, it says `.env` is still copied, by the pattern `.env`.

### `worktree-setup.explain-change` — Why a choice from the Files page changes nothing

What becomes of a file or a folder given to new worktrees, or taken from them, from the Files page, as git works the
patterns out, and the sentence that says it.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::explain_change(input) -> SetupChange`
- **Uses**: `worktree-setup.folders-to-link`, `worktree-setup.paths-to-copy`; by the four Files page entries.

The rules, for **Link** or **Copy**:

- When the patterns give it that already, as a whole: nothing changes, **`<name>` is linked** (or **copied**) **into new
  worktrees already, by the pattern `<pattern>`.**, the first pattern that by itself takes it or a folder it is in.
- When the new patterns give it that: **`<name>` will be linked** (or **copied**) **into new worktrees.**; to copy, when
  only part of it is copied, **What git ignores in `<name>` will be copied into new worktrees.**
- To copy something that a link takes: nothing changes, **`<name>` is linked into new worktrees by the pattern
  `<pattern>`, which comes before copying.**, or for what is inside a linked folder, **`<name>` is in `<folder>`, which
  the pattern `<pattern>` links into new worktrees: nothing in it is copied.**
- When git tracks it, or, for a folder to link, files in it: nothing changes, **Git tracks `<name>`: each worktree has
  it from its commit.**, or **Git tracks files in `<name>`: each worktree has them from its commit.**
- Otherwise nothing changes: **Git does not ignore `<name>`: only what it ignores is linked or copied into new
  worktrees.**, or **Git ignores only part of `<name>`: …**.

For **Stop**: when another pattern still gives it all of something, **`<name>` is still linked** (or **copied**) **into
new worktrees, by the pattern `<pattern>`.**; otherwise **New worktrees will no longer get `<name>`.**

Acceptance:

- [ ] Each sentence above is given in the case it describes, on a test repository built for it.
- [ ] Only the cases that say **will be** and **still** write the settings.

### `worktree-setup.bring-into-worktree` — Bring ignored files into a worktree

Gives a worktree that is there already what new worktrees get, without touching what it has.

- **Kind**: Operation
- **Function**: `features::worktree_setup::bring_into_worktree(context, answers) -> Outcome`
- **Entry points**: **Bring ignored files…** in the context menu of a worktree.
- **Offered when**: the worktree is not missing, not the one the views show and not the folder of the window.
- **Plan**: the `Link` and `Copy` steps of `worktree-setup.link-and-copy-steps` for that worktree, from the folder the
  views show, with what that worktree has from its commit. No commands run.
- **Danger**: Confirm, because files are made in another folder: the note says the patterns and where from.
- **Undo**: Not taken back by Undo: what was made is in the log, and can be deleted.
- **After**: once all are done, says what failed, if anything.
- **Refuses**: with no pattern in effect, **New worktrees get nothing yet: choose what they get first.**; on the folder
  the views show, **This is the checkout it comes from.**
- **Uses**: `worktree-setup.link-and-copy-steps`.

Acceptance:

- [ ] On a worktree with no `node_modules` and its own `.env`, with `node_modules/` to link and `/.env` to copy, it
      links `node_modules`, says `.env` is in the worktree already, and leaves its `.env` as it was.
- [ ] With no pattern set, it is refused with the message above.

### `worktree-setup.bring-into-other-worktrees` — Bring ignored files into the other worktrees

Gives every other worktree of the repository what new worktrees get, without touching what they have.

- **Kind**: Operation
- **Function**: `features::worktree_setup::bring_into_other_worktrees(context, answers) -> Outcome`
- **Entry points**: **Bring ignored files into the other worktrees…** in the **…** of the Worktrees list; **Bring into
  the other worktree** (**the other `<n>` worktrees**) on `worktree-setup.page`.
- **Offered when**: there is a worktree, other than the one the views show, that is not missing; on the page, also when
  the patterns are saved and there is one.
- **Plan**: the `Link` and `Copy` steps of `worktree-setup.link-and-copy-steps` for each such worktree in turn.
- **Danger**: Confirm, because files are made in other folders; the title is **Bring ignored files into `<n>`
  worktrees**.
- **Undo**: Not taken back by Undo: what was made is in the log, and can be deleted.
- **After**: once all are done, says once what failed, if anything.
- **Refuses**: with no pattern in effect, **New worktrees get nothing yet: choose what they get first.**; with no other
  worktree, **There are no other worktrees to bring it into.**
- **Uses**: `worktree-setup.link-and-copy-steps`.

Acceptance:

- [ ] With two other worktrees and one missing, the plan has steps for the two and none for the missing one.
- [ ] A failure in the first worktree does not stop the second, and one warning at the end says what failed.
