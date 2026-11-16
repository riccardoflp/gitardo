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
- **Extension**: read by the engine with the state of the repository, each time the views read it: `Reads.parts` in
  `src/engine/reads.ts` asks for `repo.read-state`, whose worktrees and counts the views show as they come.

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
- [ ] Extension: two new files in a new folder of another worktree count as two, and the worktree this window has open
      is marked as the current one.
- [ ] Extension: what changes in a worktree open elsewhere shows at the next refresh, and counting it never takes the
      lock of its index.

### `worktree.folder-path` — Where a new worktree goes

The folder of a new worktree, from the setting `worktrees.folder` and the name of the worktree, which is the name of its
branch, slashes and all.

- **Kind**: Derivation
- **Function**: `model::worktree::folder_path(input: &FolderInput) -> PathBuf`
- **Settings**: `worktrees.folder`, `${repo}.worktrees` by default.
- **Extension**: the folder of a new worktree is the engine's, from `gitardo.worktrees.folder` as the extension sends
  it (`src/engine/settings.ts`), and the box of the name says **In `<folder>`** as the engine checks it with
  `question/validate` (`VscodeAsker` in `src/engine/vscode.ts`). The views name the rows of the worktrees by their
  path from the folder `worktreesFolder` in `src/extension/actions.ts` gives them, by the same rules.

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
- [ ] Extension: with the main worktree `C:\prj\app\` and the setting empty, `fix` goes to `C:/prj/app.worktrees/fix`;
      with `/tmp/wt/`, to `/tmp/wt/fix`; with `./trees/${repo}` for `/home/me/app`, to `/home/me/trees/app/fix`.
- [ ] Extension: the names `a\b//c d`, `-odd name!/` and `///` go to `a/b/c-d`, `odd-name` and `worktree` inside the
      folder of the worktrees.
- [ ] Extension: while a name is typed, the box says `In <folder>`, or `<folder> already exists` as an error.

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
     remote, or the short hash of a detached `HEAD`, then the name it makes, which can be changed. Without it: **Name
     of the new branch, and of the folder of the worktree**, with `feature/my-change` as a placeholder. While the name is typed, the box says where the folder goes,
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
- **Extension**: **+** on the title of the Worktrees section, among the references beside the graph and in the Branches
  view; **Create worktree…** in its **…**; **Gitardo: Create Worktree…** (`gitardo.createWorktree`) in the Command
  Palette and in the **…** of the Branches view; the last entry of **Gitardo: Switch Worktree…**. Each sends
  `feature/run` of `worktree.create` on `HEAD` to Gitardo's engine (`worktreeAdd` with no `ref` in `engineRuns` of
  `src/engine/actions.ts`, run by `Controller.runOnEngine` in `src/extension/controller.ts`); the questions are asked in
  VS Code's pickers and input boxes, the name checked by the engine with `question/validate` as it is typed
  (`VscodeAsker` in `src/engine/vscode.ts`). The format and its inputs are `gitardo.branches.format` and
  `gitardo.branches.inputs`, or, where they are not set, `gitardo.worktrees.branchFormat` and
  `gitardo.worktrees.inputs`, sent by `src/engine/settings.ts`. What follows the worktree made is the extension's
  (`Controller.worktreeMade`): the engine hands the commands of `worktree-setup.command-steps` over in `waiting`, and
  the extension runs them in VS Code's terminals; once they have all succeeded, the message **Worktree created in
  `<folder>`** has **Open in Agents Window** (where VS Code has it) and **Open in New Window** in place of the desktop's
  three, and **Choose What It Gets…** while the engine offers it, once for the workspace (workspace state
  `SETUP_OFFERED`). When the engine's binary is missing, it says so (D49).

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
- [ ] Extension: picking `main` and typing `feature/login` runs `git worktree add --no-track -b feature/login
      <repo>.worktrees/feature/login main` in the engine, written to the **Gitardo** output channel before the links
      and copies that follow it, and the worktree is in the folder `login` inside `feature`.
- [ ] Extension: with `gitardo.commandPreview` at `off` nothing is asked before it runs; with `all`, the command shown
      first is the one that runs.
- [ ] Extension: a name starting with `-` is refused before anything runs; typing a name whose folder exists shows
      *<folder> already exists* under the box, and nothing is created.
- [ ] Extension: with the format `PRJ-${input:ticket}-${base}` it offers **Use the format**, asks the ticket after what
      the new branch starts from, and the ticket typed is proposed the next time, in the same workspace.
- [ ] Extension: a ticket `12a`, where the pattern of the input is `^\d+$`, cannot be accepted, and nothing is created.
- [ ] Extension: with only `gitardo.worktrees.branchFormat` set to `old-${input:ticket}`, that format is used.
- [ ] Extension: from a detached `HEAD` at `1a2b3c4`, the format `${base}-spike` proposes `1a2b3c4-spike`.
- [ ] Extension: the message after it has **Open in New Window**, and, with nothing set for new worktrees, **Choose
      What It Gets…**, which opens the tab, the first time only in the workspace; with patterns saved, it is not
      offered.
- [ ] Extension: right after it, **Undo** says it is not taken back, rather than taking back the operation before it.
- [ ] Extension: with `gitardo.engine.path` naming a missing file, it says **Gitardo's engine is not available on this
      system.** with **Show the log**, and nothing is created, as for the other operations of this file.

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
- **Extension**: **Create worktree…** in the context menu of a local branch among the references, in the Branches view
  and on its label in the graph (`branchMenu` in `src/webview/menus.ts`). It sends `feature/run` of
  `worktree.create-from-branch` on the branch to Gitardo's engine (`worktreeAdd` with `isBranch` in `engineRuns` of
  `src/engine/actions.ts`, run by `Controller.runOnEngine`), its questions asked in VS Code's pickers and input boxes
  (`VscodeAsker`), and what follows is as in `worktree.create` (`Controller.worktreeMade`). When the engine's binary is
  missing, it says so (D49).

Acceptance:

- [ ] On `feature/login`, with no format, it proposes the folder name `feature/login` and runs
      `git worktree add <repo>.worktrees/feature/login feature/login`.
- [ ] The entry is not in the menu of the branch checked out here, nor of a branch another worktree has.
- [ ] With a format, **Use the branch `feature/login`** puts that branch in the worktree, and **Use the format** makes a
      new branch from it with `--no-track -b`.
- [ ] A folder name changed to `login` puts the worktree in `<repo>.worktrees/login` on the branch `feature/login`.
- [ ] Extension: on `side/work`, keeping the folder name proposed runs `git worktree add <repo>.worktrees/side/work
      side/work` in the engine, and the worktree has `side/work` checked out.
- [ ] Extension: the menu of the branch checked out here, and of a branch another worktree has, has no **Create
      worktree…**.
- [ ] Extension: a new branch from the format at a branch that shares its name with a tag starts from the branch.
- [ ] Extension: right after it, **Undo** says it is not taken back, rather than taking back the operation before it.

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
- **Extension**: **Create worktree here…** in the context menu of a commit in the graph (`commitMenu` in
  `src/webview/menus.ts`). It sends `feature/run` of `worktree.create-at-commit` on the commit to Gitardo's engine
  (`worktreeAdd` with a commit in `engineRuns` of `src/engine/actions.ts`, run by `Controller.runOnEngine`), its
  questions asked in VS Code's pickers and input boxes (`VscodeAsker`), and what follows is as in `worktree.create`
  (`Controller.worktreeMade`). When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] On `3f2a1c9…` with the name left empty, it runs `git worktree add --detach <repo>.worktrees/3f2a1c9 3f2a1c9…`.
- [ ] With the name `try/idea`, it runs `git worktree add --no-track -b try/idea <repo>.worktrees/try/idea 3f2a1c9…`.
- [ ] With the format `${base}-spike`, on a commit that `main` points at, the picker of `${base}` lists `main` first,
      and picking it proposes `main-spike`.
- [ ] Extension: at `1a2b3c4…` with nothing typed, the box says *In <repo>.worktrees/1a2b3c4* while empty, and the
      engine runs `git worktree add --detach <repo>.worktrees/1a2b3c4 <hash>`.
- [ ] Extension: with the format `PRJ-${input:ticket}-${input:kind}-${base}`, the short hash picked as `${base}`, ticket
      `7` and kind `feat`, the branch is `PRJ-7-feat-<short>`, and the picker was titled `Create worktree at <short>:
      what ${base} is`, listing the branches at the commit, then the short hash, then the branches that have it.
- [ ] Extension: right after it, **Undo** says it is not taken back, rather than taking back the operation before it.

### `worktree.section` — The Worktrees list

The worktrees of the repository, each with how it stands, in **Kept aside** of the branch rail.

- **Kind**: View
- **Function**: `views::refs::entry::worktree_row(ui, state, entry, rect)`, a row of the branch rail, with
  `views::worktree::{label, standing, tooltip, open_intent}` and the menus of `views::worktree::worktree_entries`
  and `list_entries`
- **Entry points**: **Worktrees** in **Kept aside** of the branch rail, a list that opens in the rail.
- **Reads**: `worktree.read-list`, the branches with their upstreams (`refs.`), the pull requests (`github.`).
- **Extension**: the **Worktrees** section of the references beside the graph and of the Branches view (the
  `worktrees` section of `RefsTree` in `src/webview/RefsTree.tsx`, with `worktreeLabel` and `worktreeStatus` of
  `src/shared/refs.ts`), drawn in TypeScript from what the engine read (`worktree.read-list`). Its title has **+**
  (*Create worktree…*) and **…** (*More actions for worktrees*, `worktreesMenu` in `src/webview/menus.ts`), and a right
  click on the title opens the same menu as **…**. Its rows and menus are those of this list, the operations sent to
  Gitardo's engine as their cards say; the double click opens a worktree where `gitardo.worktrees.openIn` says.

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
- [ ] Extension: a worktree inside the folder of the worktrees is named by its path from there, slashes and all
      (`feature/login`); any other by the name of its folder; the main worktree is listed like the others.
- [ ] Extension: the tooltip has, on separate lines: the path, `<branch> → <upstream>` when it has one, `Nothing to
      commit` or `<n> file(s) not committed`, and the pull request.
- [ ] Extension: a click selects the commit it has checked out in the graph; a double click opens it as
      `worktree.open` does, except the worktree of this window and a missing one, where it does nothing.
- [ ] Extension: **…** lists **Create worktree…**, **Switch worktree…**, **Choose what new worktrees get…**, **Bring
      ignored files into the other worktrees…** (only while a worktree is neither missing nor current) and **Prune
      missing worktrees** (only while one is missing).

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
- **Extension**: done by the extension itself, in TypeScript, with nothing sent to the engine: `openWorktree(dir,
  where)` in `src/extension/actions.ts`, with `openIn(state)` for where it opens when nothing says (the `worktreeOpen`
  action of `src/shared/protocol.ts`). `gitardo.worktrees.openIn` is `window` (the default), a new VS Code window as
  `worktree.open-in-new-window`, or `agents`, the Agents window of VS Code for agent sessions in it, which runs
  `workbench.action.openAgentsWindow` with `{ folderUri }` (`AGENTS_WINDOW`), in place of the desktop's `editor` and
  `terminal`. **Open in Agents Window** is in the context menu of a worktree, at the top of the menu of a branch or a
  remote branch another worktree has (**Open worktree in Agents Window**) and in the message after a worktree is
  created, only where VS Code has that command, looked for once per session (`agentsWindow` of the state, from
  `hasCommand` in `src/extension/controller.ts`). It also opens what **Gitardo: Switch Worktree…** picks.

Acceptance:

- [ ] With `worktrees.openIn` unset, a double click on a worktree opens a new window of Gitardo Desktop on it.
- [ ] With `worktrees.openIn` set to `terminal`, the same double click opens a terminal in its folder.
- [ ] With `editor` and `editor.external` unset, it opens a new window and says that no external editor is set.
- [ ] Extension: **Open in Agents Window** runs `workbench.action.openAgentsWindow` with `{ folderUri }`; when that
      fails, the warning *The Agents window did not open (<reason>), so the worktree opens in a new window.* shows and
      a new window opens.
- [ ] Extension: with `gitardo.worktrees.openIn` set to `agents` in a VS Code without the Agents window, a double click
      opens a new window.
- [ ] Extension: nothing in the repository changes, and nothing is written to the log.

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
- **Extension**: **Open in new window** in the context menu of a worktree, beside the graph and in the Branches view;
  **Open worktree in new window** in that of a branch or a remote branch another worktree has (`worktreeOpenEntries` in
  `src/webview/menus.ts`); **Open in New Window** in the message after a worktree is created. Done by the extension
  itself, in TypeScript: `openWorktree(dir, 'window')` in `src/extension/actions.ts` runs `vscode.openFolder` on the
  folder with `forceNewWindow`, nothing sent to the engine. Not offered for the worktree of this window, nor for a
  missing one.

Acceptance:

- [ ] The new window shows the repository of the worktree with the worktree as the folder of the window, and this
      window is left as it was.
- [ ] Extension: **Open in new window** runs `vscode.openFolder` on the folder with `forceNewWindow`, and its menu
      entry does not name the worktree.

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
- **Extension**: **Open in terminal** in the context menu of a worktree; **Open terminal in worktree** in that of a
  branch another worktree has (`worktreeOpenEntries` in `src/webview/menus.ts`). Done by the extension itself, in
  TypeScript: the case `worktreeTerminal` of `hostAction` in `src/extension/actions.ts` opens a terminal of this
  VS Code window, nothing sent to the engine; `terminal.external` has no place there.

Acceptance:

- [ ] The terminal opens with its current folder set to the worktree's folder, the current worktree included.
- [ ] Extension: it opens and shows a terminal of this window named after the folder of the worktree, starting in that
      folder.

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
- **Extension**: the same entries, in the context menu of a worktree beside the graph and in the Branches view, and in
  that of a branch another worktree has (`REVEAL` and `worktreeOpenEntries` in `src/webview/menus.ts`). Done by the
  extension itself, in TypeScript: the case `worktreeReveal` of `hostAction` in `src/extension/actions.ts` runs
  VS Code's `revealFileInOS`, nothing sent to the engine.

Acceptance:

- [ ] On each system the entry has the name given above, and the file manager opens on the worktree's folder.
- [ ] Extension: it shows the folder of the worktree in the file manager, the way VS Code's own **Reveal in File
      Explorer** does.

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
- **Extension**: **Show in the graph** in the context menu of a worktree, beside the graph and in the Branches view
  (`worktreeMenu` in `src/webview/menus.ts`). Done in the webview, in TypeScript: `revealEntry(hash, onReveal)` of
  `src/webview/menus.ts`, given the hash of the worktree, and `onReveal` of `RefsTree`; nothing is sent to the engine.

Acceptance:

- [ ] On a worktree whose commit is far down the history, the timeline loads up to it and selects it.
- [ ] Extension: the commit the worktree has checked out is selected in the graph, the history loaded as far back as
      it is.

### `worktree.show-in-gitardo` — Show another worktree in this window

Points the views of this window at another worktree, so that what is done there, staging, committing, pulling or
pushing, is done in it, without opening another window.

- **Kind**: Operation
- **Function**: `features::worktree::show_in_gitardo(context, answers) -> Outcome`
- **Entry points**: **Show in Gitardo** in the context menu of a worktree; **Show worktree in Gitardo** in the menu of a
  branch another worktree has; in the extension, the repository menu of the header, which lists it while it is shown.
  In Gitardo Desktop it is shown in the tab of its repository, whose tooltip names its folder.
- **Offered when**: the worktree is not the one the views show and is not missing.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the timeline with its draft, the branch rail and the Changes page are those of the worktree; the banner of
  `worktree.banner` says so. The folder of the window is then listed as another worktree, and can be neither renamed
  nor removed, nor given ignored files, from there. The choice is not remembered: the next time the window opens, the
  views show its own folder. In Gitardo Desktop the tab is named after its repository and the worktree's branch, as
  `api › feature/login`, and the worktree keeps what it showed apart from its repository, as `repo.group-tabs` says.
- **Uses**: `worktree.banner`, `repo.select`, `repo.group-tabs`.
- **Extension**: **Show in Gitardo** in the context menu of a worktree; **Show worktree in Gitardo** in that of a branch
  another worktree has (`worktreeOpenEntries` in `src/webview/menus.ts`); a worktree selected in VS Code's Repositories
  view (`repo.follow-source-control`); **Gitardo: Select Repository**, which lists the worktree shown as *worktree,
  current*; and the branch at the start of the toolbar of the Graph view and of the Gitardo Graph tab, which, while the
  repository has two worktrees or more whose folders are there, is a button with a chevron that lists them, in the
  order git lists them, each by its branch or `detached at <short>`, followed by its folder when that is not the name
  of its branch, the one shown with a check (`WorktreeSelect` in `src/webview/WorktreeSelect.tsx`, used by `Toolbar`).
  Done by the extension itself, in TypeScript: the message `selectRepo` to `Controller.selectRepo(root)` in
  `src/extension/controller.ts`, with `pickRepo` and `home` of `RepoState` in `src/shared/protocol.ts`; for a folder
  that is neither a repository of the workspace nor a worktree of the one shown, `git rev-parse --git-common-dir` in it
  and in each repository of the workspace finds the one it is a worktree of. Nothing is sent to the engine; the
  operations run there afterwards run in the worktree shown.

Acceptance:

- [ ] After **Show in Gitardo** on `feature/login`'s worktree, a commit made from the draft is made in that
      worktree, on `feature/login`, and the window's folder is untouched.
- [ ] While it is shown, the menu of the window's own folder in the Worktrees list has no **Rename…** and no
      **Remove…**.
- [ ] Closing the window and opening it again shows its own folder.
- [ ] Extension: the graph, the Changes view, the Commits view and the Branches view show the worktree: its branch, its
      files not committed, its commits; staging `shown.txt` there stages it in the index of that worktree.
- [ ] Extension: the folder of the window is listed as another worktree meanwhile, with no **Rename…**, **Remove…** or
      **Bring ignored files…**, and removing it is refused with *This worktree is the folder open in this window:
      remove it from another one.*
- [ ] Extension: a worktree of another repository of the workspace is shown too, with that repository as the one to go
      back to and as the one chosen in the list of repositories, which does not list the worktree; a folder that is a
      worktree of none of them is not shown.
- [ ] Extension: the branch in the toolbar says the branch checked out, or `detached at <short>`, or `no commits yet`,
      with the path of the worktree shown as its tooltip; with a single worktree whose folder is there, it is a label
      with the branch icon and a click does nothing.
- [ ] Extension: with two worktrees or more, the button has the worktree icon while the views show a worktree other
      than the main one; picking another worktree shows it, picking the one shown does nothing, and the button is
      disabled while an operation runs.

### `worktree.banner` — The banner of a worktree shown in this window

Says which worktree or submodule the views show while it is not the folder of the window, and goes back.

- **Kind**: View
- **Function**: `views::worktree::banner(state: &BannerState, ui) -> Vec<Intent>`
- **Entry points**: the top of the timeline and of the Changes page, while another worktree or a submodule is
  shown.
- **After**: **Back to `<name>`** points the views at the repository of the window again.
- **Extension**: drawn in the webviews, in TypeScript: `WorktreeBanner` in `src/webview/WorktreeBanner.tsx`, at the top
  of the Graph view (not of the Gitardo Graph tab), the Changes view and the Branches view, while `home` of the state is
  set (`App.tsx` when not compact, `ChangesApp.tsx`, `BranchesApp.tsx`). **Back to `<repository>`** posts `selectRepo`
  to `Controller.selectRepo`; nothing is sent to the engine.

The banner says **Showing the worktree `<label>`**, followed by **on `<branch>`** when it has one, with its folder as the
tooltip, and the button **Back to `<name>`**, the name of the window's repository, with its folder as the tooltip. The
button is disabled while an operation runs.

Acceptance:

- [ ] While `feature/login`'s worktree is shown, the banner reads **Showing the worktree feature/login on feature/login**
      in both places.
- [ ] **Back to app** shows the window's folder again, and the banner goes.
- [ ] No banner shows while the views show the window's own folder.
- [ ] Extension: it says *Showing the worktree <name> on <branch>*, or without *on <branch>* on a detached `HEAD`, its
      tooltip the full path; **Back to <repository>** has the path of the window's folder as its tooltip.
- [ ] Extension: the button is disabled while an operation runs, in the Graph and Changes views.

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
- **Plan**: `git worktree move <folder> <new folder>`, then the deletions of `worktree.remove-empty-folders` for the
  old place, then `git branch -m <branch> <new name>` when the branch is renamed, last, so that a branch git refuses to
  rename leaves no empty folder behind. The folders that lead to the new
  place are made with the move when they are missing, since git does not make them. On Windows, a move that fails
  because the folder is held is tried again (`worktree.recover-held-folder`).
- **Danger**: None. The question says that git cannot move a folder that another program has open, such as another
  window or a terminal in it.
- **Undo**: Not taken back by Undo: rename it again.
- **Refuses**: on the main worktree, **Git cannot move the main worktree, the folder the repository was created in.**;
  on the window's folder, **This worktree is the folder open in this window: rename it from another one.**; when the new
  folder exists by the time it runs, **`<folder>` already exists.**
- **Uses**: `worktree.folder-path`, `branch.rename`, `worktree.recover-held-folder`, `worktree.remove-empty-folders`.
- **Extension**: **Rename…** in the context menu of a worktree, beside the graph and in the Branches view; **Rename
  worktree…** in that of a branch another worktree has (`worktreeMenu`, `branchMenu` in `src/webview/menus.ts`). It
  sends `feature/run` of `worktree.rename` on the worktree to Gitardo's engine (`worktreeRename` in `engineRuns` of
  `src/engine/actions.ts`, run by `Controller.runOnEngine`), its questions asked in VS Code's input boxes and pickers
  (`VscodeAsker`). Before it, `Controller.letGoInVSCodeGit` has VS Code's Git extension close the worktree
  (`closeInVSCodeGit` in `src/extension/actions.ts`, `> git.close <folder>` in the log), which Windows would otherwise
  hold. The folder open in this window is not sent to the engine, which does not know it: `hostAction` in
  `src/extension/actions.ts` refuses it. When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] `feature/login` renamed to `feature/signin`, with its branch, runs `git worktree move
      <trees>/feature/login <trees>/feature/signin` then `git branch -m feature/login feature/signin`, and the branch
      name was proposed as `feature/signin`.
- [ ] `feature/login` renamed to `hotfix/login` leaves no empty folder `feature` in the folder of the worktrees.
- [ ] A worktree outside the folder of the worktrees, renamed `a/b`, goes to a folder `a-b` beside where it was.
- [ ] The entry is not offered on the main worktree nor on the folder of the window.
- [ ] Extension: renaming `agents/first` to `agents-two` with its branch runs, in the engine, `git worktree move
      <wt>/agents/first <wt>/agents-two`, then deletes the folder `agents` that held only it, then runs `git branch -m
      agents/first agents-two`.
- [ ] Extension: when `git branch -m` fails after the move, the folder that held only the worktree is gone all the
      same.
- [ ] Extension: the title of the operation is `Rename worktree agents/first to agents-two`.
- [ ] Extension: renaming the main worktree is refused with *Git cannot move the main worktree, the folder the
      repository was created in.*, and nothing runs.
- [ ] Extension: the log has `> git.close <folder>` before the move when VS Code's Git extension had the worktree open,
      and on Windows a move refused because the folder is held is tried again as `worktree.recover-held-folder` says.
- [ ] Extension: a name starting with `-` cannot reach git: the folder name loses its leading `-`, and a branch name
      starting with `-` cannot be accepted.
- [ ] Extension: right after it, **Undo** says it is not taken back, rather than taking back the operation before it.

### `worktree.copy-path` — Copy the path of a worktree

Puts the full path of a worktree's folder on the clipboard.

- **Kind**: Operation
- **Function**: `features::worktree::copy_path(context, answers) -> Outcome`
- **Entry points**: **Copy path** in the context menu of a worktree.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the path is on the clipboard.
- **Extension**: **Copy path** in the context menu of a worktree (`worktreeMenu` in `src/webview/menus.ts`), done by
  the extension itself, in TypeScript: the webview posts `{ type: 'copy' }`, which `Controller.onMessage` puts on
  VS Code's clipboard; nothing is sent to the engine.

Acceptance:

- [ ] The clipboard holds the worktree's folder as git lists it, also for a missing worktree.
- [ ] Extension: the clipboard holds the path as git lists it, with forward slashes.

### `worktree.remove` — Remove a worktree

Removes a worktree's folder with `git worktree remove`, and its branch too if wanted.

- **Kind**: Operation
- **Function**: `features::worktree::remove(context, answers) -> Outcome`
- **Entry points**: **Remove…** in the context menu of a worktree; **Remove worktree…** and **Remove worktree and
  delete branch…** in the menu of a branch another worktree has, which answer the question about the branch.
- **Offered when**: the worktree is not the main worktree, not the one the views show and not the folder of the window.
- **Asks**: when it has a branch and the entry did not say: **Remove the worktree** (**Keep the branch `<branch>`**,
  the default) or **Remove the worktree and delete `<branch>`** (**Only if it is merged**).
- **Reads**: `worktree.read-folder-links`.
- **Plan**: `unlink <path>` for each link of `worktree.read-folder-links`; then `git worktree remove <folder>`; then the deletions of `worktree.remove-empty-folders`; then `git branch -d <branch>` when the branch goes
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
- **Extension**: **Remove…** in the context menu of a worktree, beside the graph and in the Branches view; **Remove
  worktree…** and **Remove worktree and delete branch…** in that of a branch another worktree has (`worktreeMenu`,
  `branchMenu` in `src/webview/menus.ts`), which give the answer `branch` (`keep` or `delete`). It sends `feature/run`
  of `worktree.remove` on the worktree to Gitardo's engine (`worktreeRemove` in `engineRuns` of `src/engine/actions.ts`,
  run by `Controller.runOnEngine`), its question and its danger asked in VS Code's picker and modal dialog
  (`VscodeAsker`). Before it, `Controller.letGoInVSCodeGit` has VS Code's Git extension close the worktree
  (`closeInVSCodeGit` in `src/extension/actions.ts`, `> git.close <folder>` in the log). The folder open in this window
  is not sent to the engine, which does not know it: `hostAction` in `src/extension/actions.ts` refuses it. When git
  refuses a worktree with changes, the engine's offer **Remove anyway** comes in the error's `data.followUps`, and the
  extension shows it as a button of the error message beside **Show Log**. When the engine's binary is missing, it says
  so (D49).

Acceptance:

- [ ] On a clean worktree of `feature/login`, choosing to delete the branch runs `git worktree remove <folder>` then
      `git branch -d feature/login`.
- [ ] On a worktree with a modified file, git's refusal is followed by an offer with `--force`, shown as destructive,
      and nothing is deleted until it is accepted.
- [ ] A branch that is not merged is kept by `git branch -d`, and git's message is shown.
- [ ] A worktree with a link `node_modules` to the window's folder, a junction on Windows, is removed with `unlink
      <folder>/node_modules` first, and the window's `node_modules` keeps all its files; elsewhere than Windows, a
      `.gitignore` that says `node_modules/` does not stop the removal.
- [ ] Removing `feature/login`, the only worktree in `feature`, leaves no folder `feature` behind.
- [ ] Extension: removing a clean worktree with its merged branch `agents-two` asks first, then runs, in the engine,
      `git worktree remove <folder>` and `git branch -d agents-two`; the worktree, the branch and the folder are gone.
- [ ] Extension: a worktree with files not committed is refused by git with *…contains modified or untracked files, use
      --force to delete it*; the error message has **Remove anyway** beside **Show Log**, which runs `git worktree
      remove --force <folder>`, and nothing is deleted until it is chosen.
- [ ] Extension: a branch that is not merged is kept, git saying why, the worktree is removed, and the folder that held
      only it is gone all the same.
- [ ] Extension: **Remove…** on a worktree whose folder is gone is refused with *The folder of this worktree is missing
      from `<folder>`: prune the worktrees to forget it.*, and nothing runs.
- [ ] Extension: the log has, in this order, `> git.close <folder>` (when VS Code's Git extension had it open), the
      `unlink` lines, then `git worktree remove <folder>`; after it, VS Code's Git extension no longer has it.
- [ ] Extension: before git runs, every link in the worktree to a folder outside it, such as the folders the
      setup linked, is taken out (`worktree.read-folder-links`): the folders they lead to, in the checkout they come
      from, keep every file; a link that cannot be taken out stops the removal before git runs.
- [ ] Extension: on Windows, when git said `failed to delete '<folder>'` and no longer lists the worktree, what is left
      of the folder is deleted as `worktree.recover-held-folder` says, and the branch is deleted after it.
- [ ] Extension: elsewhere than Windows, a link `node_modules` that the exclude file ignores as `node_modules/` is
      unlinked first, and the worktree is removed.
- [ ] Extension: removing `side/work` with **Remove worktree…** from its branch asks no picker and keeps the branch; the
      folder `side` that held only it is gone.
- [ ] Extension: removing the folder of the window while the views show another worktree is refused, and nothing
      runs.
- [ ] Extension: right after it, **Undo** says it cannot be undone, rather than taking back the operation before it.

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
- **Extension**: **Prune missing worktrees** in the context menu of a worktree and in the **…** of the Worktrees
  section, beside the graph and in the Branches view (`worktreeMenu`, `worktreesMenu` in `src/webview/menus.ts`). It
  sends `feature/run` of `worktree.prune` to Gitardo's engine (`worktreePrune` in `engineRuns` of
  `src/engine/actions.ts`, run by `Controller.runOnEngine`), its plan shown first only when `gitardo.commandPreview` is
  `all` (`VscodeAsker`). When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] After a worktree's folder is deleted outside Gitardo, **Prune missing worktrees** is offered, runs
      `git worktree prune`, and the worktree is no longer listed.
- [ ] The branch it had can then be checked out here.
- [ ] Extension: with a worktree whose folder was deleted, it runs `git worktree prune` in the engine, and the
      Worktrees section no longer lists it; with no missing worktree, the entry is not offered.
- [ ] Extension: right after it, **Undo** says it is not taken back, rather than taking back the operation before it.

### `worktree.read-folder-links` — Read the links to folders outside a worktree

On Windows, git deletes what is inside a junction when it removes a worktree: the folder it links to, in the folder the
link came from. Elsewhere, a symbolic link that a pattern for folders such as `node_modules/` does not ignore is a file
git does not track, and `git worktree remove` refuses the worktree. This read finds those links, so that they are taken
out first.

- **Kind**: Read
- **Function**: `git::read::worktree::read_folder_links(git, request) -> Result<Vec<PathBuf>, GitError>`
- **Reads**: in the worktree: `git ls-files --others --directory -z` and
  `git ls-files --others --ignored --exclude-standard --directory -z`. Each entry that is a link is kept when it leads
  to a folder outside the worktree; each entry that is a folder is looked into, without going past a link.
- **Uses**: by `worktree.remove` and `branch.clean-up`.
- **Extension**: read by the engine, in the plan of `worktree.remove`, which the extension runs there.

Acceptance:

- [ ] A worktree with a junction `node_modules` to another folder and a junction `packages/api/node_modules` returns
      both.
- [ ] A link to a folder inside the worktree, a link to a file and a link to nothing are not returned.
- [ ] Tracked files are not looked into.
- [ ] On macOS and Linux, a symbolic link to a folder outside the worktree is returned, as a junction is on Windows.

### `worktree.recover-held-folder` — Get past a folder that Windows holds

On Windows another program can hold a worktree's folder for a moment, such as a window or a terminal that has just
left it. A move or a removal that fails for that is not left half done.

- **Kind**: Derivation
- **Function**: `model::worktree::recover_held_folder(input) -> Recovery`
- **Uses**: by `worktree.rename`, `worktree.remove` and `branch.clean-up`.
- **Extension**: carried out by the engine in the plans of `worktree.rename` and `worktree.remove` the extension runs
  there, after `Controller.letGoInVSCodeGit` had VS Code's Git extension close the worktree, which is what holds it
  most often inside VS Code.

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
- **Extension**: steps of the engine's plans of `worktree.rename`, right after `git worktree move`, and of
  `worktree.remove`, right after `git worktree remove`, both before git touches the branch.

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
- **Extension**: worked out in the webviews, in TypeScript: `worktreeOf(branch, state)` in `src/shared/refs.ts`, with
  `branchMenu`, `remoteBranchMenu`, `openBranch`, `openRemoteBranch` and `dropMenu` in `src/webview/menus.ts`, for the
  branches of `RefsTree` and the labels of `src/webview/labels.ts`. The entries that open the worktree are those of
  `worktree.open` (**Open worktree in Agents Window** first, where VS Code has it, in place of **Open worktree in
  external editor**); the others are sent to Gitardo's engine as their cards say.

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
- [ ] Extension: its row among the local branches has the worktree icon, and its tooltip a line *In the worktree
      <folder>* with the files not committed there; its label in the graph has the worktree icon too.
- [ ] Extension: its menu starts with **Open worktree in Agents Window** (where VS Code has it), **Open worktree in new
      window**, **Open terminal in worktree**, the reveal entry and **Show worktree in Gitardo**, and has **Rename
      branch…**, **Rename worktree…**, **Remove worktree…** and **Remove worktree and delete branch…**, except for the
      main worktree's branch seen from another worktree and for the branch of the window's folder.
- [ ] Extension: a double click on it opens its worktree where `gitardo.worktrees.openIn` says; while its worktree is
      missing, it does nothing.
- [ ] Extension: the menu of the remote branch of the same name starts with the same entries, without **Check out**.

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
- **Extension**: **Open in worktree** in the context menu of a local branch and of a remote branch, among the
  references, in the Branches view and on its label in the graph; a double click on them while `gitardo.branches.openIn`
  is `worktree` (`branchMenu`, `remoteBranchMenu`, `openBranch`, `openRemoteBranch` in `src/webview/menus.ts`). It sends
  `feature/run` of `worktree.open-in-worktree` on the branch or the remote branch to Gitardo's engine (`openInWorktree`
  in `engineRuns` of `src/engine/actions.ts`, run by `Controller.runOnEngine`). A local branch that another worktree has
  already is not sent: the extension opens that worktree itself, where `gitardo.worktrees.openIn` says, the Agents
  window included. Once the engine made the worktree and the commands of `worktree-setup.command-steps` the extension
  runs in VS Code's terminals have all succeeded, `Controller.worktreeMade` opens it where `gitardo.worktrees.openIn`
  says. When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] On `feature/login`, which no worktree has, it runs `git worktree add <trees>/feature/login feature/login` and
      opens it, with no question.
- [ ] On `origin/fix` with no local `fix`, it runs `git worktree add --track -b fix <trees>/fix origin/fix`, and `fix`
      follows `origin/fix` afterwards.
- [ ] On `origin/fix` with a local `fix`, it runs `git worktree add <trees>/fix fix`.
- [ ] With `branches.openIn` set to `worktree`, a double click on a branch makes and opens its worktree, and the folder
      of the window stays on its branch.
- [ ] A second **Open in worktree** on the same branch opens the worktree made the first time and creates nothing.
- [ ] Extension: on `merged`, the engine runs `git worktree add <repo>.worktrees/merged merged` and the folder opens,
      with no question asked while `gitardo.commandPreview` is `off`; again on `merged`, nothing runs and the same
      folder opens.
- [ ] Extension: on `origin/remote/only`, with no local `remote/only`, it runs `git worktree add --track -b remote/only
      <repo>.worktrees/remote/only origin/remote/only`, and `remote/only` follows `origin/remote/only`.
- [ ] Extension: on `origin/fix` while a tag is also named `origin/fix`, the remote branch is given to git as
      `refs/remotes/origin/fix`.
- [ ] Extension: on the branch checked out here, nothing runs and *`<branch>` is checked out in this window.* is said.
- [ ] Extension: it never offers **Choose What It Gets…**.
- [ ] Extension: right after it, **Undo** says it is not taken back, rather than taking back the operation before it.

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
- **Extension**: **Gitardo: Switch Worktree…** (`gitardo.switchWorktree`) in the Command Palette and in the **…** of
  the Branches view; **Switch worktree…** in the **…** of the Worktrees section. Done by the extension itself, in
  TypeScript: `Controller.switchWorktree()` in `src/extension/controller.ts` shows VS Code's picker, titled `Switch
  Worktree`, *The worktree to open*, from the state the engine read, and opens the one picked as `worktree.open` does;
  **Create worktree…** runs `worktree.create` in the engine.

Acceptance:

- [ ] With three worktrees, the one of this window is listed last, marked **this window**.
- [ ] Typing part of a folder's path finds its worktree.
- [ ] Picking another worktree opens it where `worktrees.openIn` says; the last entry starts creating a worktree.
- [ ] Extension: the main worktree has the home icon and the others the worktree icon, each named as in
      `worktree.section`, its path under it; its description joins, by ` · `, the branch or `detached at <short>` with
      `↑n ↓n`, a pencil with the files not committed, the pull request, and `this window`: `merged · $(edit) 1`.
- [ ] Extension: picking the worktree of this window does nothing; picking another opens it where
      `gitardo.worktrees.openIn` says.

### `worktree-setup.settings-in-effect` — What new worktrees get, as set

Which link and copy patterns and which commands apply to new worktrees of a repository, and where each comes from.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::settings_in_effect(input) -> SetupSettings`
- **Settings**: `worktrees.linkFolders`, `worktrees.copyFiles`, `worktrees.setupCommands`; VS Code's
  `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles` in the repository's `.vscode/settings.json`.
- **Extension**: inside VS Code the settings are VS Code's, and the engine reads no settings file of its own: with
  `host` `vscode`, it takes `worktrees.linkFolders`, `worktrees.copyFiles` and `worktrees.setupCommands` from the
  settings the extension sends (`engineSettings` in `src/engine/settings.ts`), which are `gitardo.worktrees.*` as
  VS Code resolves them for the folder of the repository (scope `resource`: the workspace folder's or the workspace
  file's over the user's), each of the first two falling back on VS Code's `git.worktreeSymlinkFolders` or
  `git.worktreeIncludeFiles` while Gitardo's is set nowhere. Where each comes from, for the tab of what new worktrees
  get and for the Explorer, is read in TypeScript by `setupPatterns(root)` and `setupCommands(root)` of
  `src/extension/worktreeSettings.ts`, with `getConfiguration(section, Uri.file(root)).inspect`, and kept clean by
  `cleanPatterns` and `cleanCommands` of `src/shared/worktreeSetup.ts`: `workspace`, `user`, `vscode` or `none`.

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
- [ ] Extension: the user's `gitardo.worktrees.linkFolders: ["node_modules/"]` and the workspace folder's
      `gitardo.worktrees.linkFolders: []` link nothing, from `workspace`.
- [ ] Extension: with only VS Code's `git.worktreeSymlinkFolders: ["node_modules"]`, `node_modules` is linked, from
      `vscode`, and the engine is sent `worktrees.linkFolders: ["node_modules"]`.
- [ ] Extension: read for the tab, `[" node_modules/ ", "", "node_modules/", "a\nb", 3, "/.env"]` gives
      `node_modules/`, `/.env`; the commands `[" npm ci ", "", "npm run build", "a\nb", 3, "npm ci"]` give `npm ci`,
      `npm run build`, `npm ci`.
- [ ] Extension: a `.gitardo/settings.json` in the repository changes nothing of what new worktrees get.

### `worktree-setup.folders-to-link` — The folders to link

Which folders of the checkout a new worktree gets as links, from the link patterns, worked out as VS Code's Git
extension works them out, so that the same patterns give the same folders.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::folders_to_link(input) -> Vec<String>`
- **Reads**: what git ignores, `git ls-files --others --ignored --exclude-standard -z` and the same with `--directory`;
  what the patterns match among the files git does not track, `git ls-files --others --ignored --exclude-from=<file of
  the patterns> -z`; whether git ignores a folder, and whether the patterns name it as a folder, `git check-ignore
  --no-index -z --stdin` in the checkout and in an empty repository whose excludes file is the patterns.
- **Extension**: what a new worktree, or one that gets ignored files brought in, is given is worked out by the engine,
  in the plans the extension runs there. The tab of what new worktrees get, and the Explorer's entries, ask the engine
  for it too, with `worktree-setup.read-what-patterns-take` (`Reads.setupPreview` and `Reads.setupTakes` in
  `src/engine/reads.ts`).

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
- [ ] Extension: with `.gitignore` `node_modules/`, and `node_modules/pkg/index.js` and `packages/a/node_modules/x.js`
      in the checkout, the pattern `node_modules` takes `node_modules` and `packages/a/node_modules`, in the engine's
      plan and in the tab alike.
- [ ] Extension: no pattern takes nothing, without running git.

### `worktree-setup.paths-to-copy` — The files and folders to copy

Which files and folders of the checkout a new worktree gets as copies, from the copy patterns.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::paths_to_copy(input) -> Vec<String>`
- **Reads**: what git ignores, `git ls-files --others --ignored --exclude-standard -z`, and the same with `--directory
  --no-empty-directory`; what the patterns match, `git ls-files --others --ignored --exclude-from=<file of the patterns>
  -z`; what the worktree has from its commit, `git ls-tree -r -z --name-only <commit>`.
- **Extension**: worked out by the engine in the plans the extension runs there. The tab of what new worktrees get and
  the Explorer's entries ask the engine for it too, with `worktree-setup.read-what-patterns-take` (`Reads.setupPreview`
  and `Reads.setupTakes` in `src/engine/reads.ts`).

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
- [ ] Extension: with `.gitignore` `node_modules/`, `.env`, `*.log`, `config/`, the patterns `.env`, `config/`,
      `node_modules/` with `node_modules` linked copy `.env` and `config` (a whole folder of `one.json` and
      `two.json`), not `node_modules` and not `debug.log`.
- [ ] Extension: no pattern takes nothing, without running git.

### `worktree-setup.link-and-copy-steps` — Linking and copying into a new worktree

What a new worktree gets of what git ignores in the folder the views show, as steps after `git worktree add`.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::link_and_copy_steps(input) -> SetupSteps`
- **Settings**: `worktrees.linkFolders`, `worktrees.copyFiles`, with VS Code's as the fallback.
- **Uses**: `worktree-setup.settings-in-effect`, `worktree-setup.folders-to-link`, `worktree-setup.paths-to-copy`; by
  `worktree.create`, `worktree.create-from-branch`, `worktree.create-at-commit`, `worktree.open-in-worktree`,
  `worktree-setup.bring-into-worktree`, `worktree-setup.bring-into-other-worktrees`.
- **Extension**: steps of the engine's plans that the extension runs there for every way of creating a worktree and for
  **Bring ignored files…**, the patterns being `gitardo.worktrees.linkFolders` and `gitardo.worktrees.copyFiles`, or VS
  Code's `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles`, as the extension sends them
  (`worktree-setup.settings-in-effect`); the note is shown with the plan when `gitardo.commandPreview` is `all`
  (`VscodeAsker`). When the engine's binary is missing, it says so (D49).

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
- [ ] Extension: with VS Code's `git.worktreeSymlinkFolders` `["node_modules"]` and `git.worktreeIncludeFiles`
      `[".env"]` and nothing of Gitardo's set, the log has `git worktree add …`, then `symlink node_modules`, then
      `copy .env`.
- [ ] Extension: `node_modules` in the worktree is a link to the one of this checkout, and `.env` a copy; `debug.log` is
      not there; `git status --porcelain` in the worktree prints nothing.
- [ ] Extension: brought again into the same worktree, nothing is made over what is there, and the log says *<path> is
      in the worktree already.* for each.
- [ ] Extension: a folder that holds the worktree is not linked, and the log says *<folder> holds the worktree, so it
      is not linked.*
- [ ] Extension: a failure is logged with its reason and stops neither the other links and copies nor the commands for
      new worktrees after them; copies keep the links they hold as links.
- [ ] Extension: the question of the command preview says **Then Gitardo will link…**, the name of the product
      written as it is.

### `worktree-setup.command-steps` — Commands in a new worktree

The commands of `worktrees.setupCommands`, such as `npm ci`, run in a new worktree once it has been created and its
links and copies made.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::command_steps(input) -> SetupSteps`
- **Settings**: `worktrees.setupCommands`.
- **Uses**: `worktree-setup.settings-in-effect`, `trust.ask`; by `worktree.create`, `worktree.create-from-branch`,
  `worktree.create-at-commit`, `worktree.open-in-worktree`.
- **Extension**: the commands are `gitardo.worktrees.setupCommands`, set in the workspace folder, the workspace file or
  the user's settings. The extension says `host.terminal` when it starts the engine, so the engine hands the `Task`
  steps over in `waiting` instead of running them, and `Controller.worktreeMade` runs them with `runSetupCommands` of
  `src/extension/worktreeCommands.ts`, without waiting for them: one after the other, each as a VS Code task named after
  the folder of the worktree (source *Gitardo*), a shell execution with the worktree as its folder, in a terminal of
  this window shown without taking the focus, the next only once the last ended with exit code 0. Trust is VS Code's:
  the extension sends `trust` `vscode`, so the engine asks no trust question, and the commands run only in a trusted
  workspace. A command that fails, or a workspace not trusted, is said in a warning with **Open in Agents Window**
  (where VS Code has it) and **Open in New Window**. When the engine's binary is missing, it says so (D49).

The rules:

- Each command is a `Task` step in the worktree, named after it, `run <command>` in the log, after the links and
  copies. The engine runs the steps of the plan before the first task, then hands that task and the steps after it to
  **Tasks**, which runs them apart.
- They run one after the other in **Tasks**, in the Activity drawer, each as a task named after the worktree's folder,
  with the shell of the system, so that what they print is seen and what they ask can be answered. One command to a
  line, with no `&&` between them, which Windows PowerShell does not know.
- One that fails or is stopped stops those after it, and Gitardo says which: **`<command>` failed with exit code `<n>`
  in `<worktree>`, so the `<k>` commands after it did not run.**, or **did not finish** for one stopped.
- They run only in a trusted folder (`trust.is-trusted`); in another, Gitardo asks first whether to trust it
  (`trust.ask`). With **Not now**, none runs, the log says **The commands of worktrees.setupCommands did not
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
- [ ] Extension: with `git update-ref refs/setup/one HEAD` and `git update-ref refs/setup/two refs/setup/one`, **Open in
      worktree** on `commands/one` logs `run …` for each, in that order, `refs/setup/two` ends at the commit of `main`,
      and the worktree opens only after both, with no question asked.
- [ ] Extension: with `git rev-parse --verify --quiet refs/setup/missing` first, the warning says *… failed with exit
      code 1 in two, so the command after it did not run.*, offers **Open in New Window**, the worktree does not open,
      and the second command does not run.
- [ ] Extension: the operation ends, and the views read the repository again, without waiting for the commands; other
      operations can run meanwhile.
- [ ] Extension: they run in the shell VS Code runs tasks with: each line is one command, with no `&&` added between
      them.
- [ ] Extension: in a workspace that is not trusted, no task starts, also for commands set in the user's settings; the
      log says *The commands of gitardo.worktrees.setupCommands did not run: this workspace is not trusted.*, the
      warning *The commands for new worktrees did not run in <name>: this workspace is not trusted.* offers to open it,
      and no trust question is asked.

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
- **Extension**: read by the engine for the tab of what new worktrees get: `Reads.setupIgnored` in
  `src/engine/reads.ts`, asked by `Controller.sendSetup` when the tab opens and on **Look again**, which looks on disk
  whether a link leads to a folder, as the engine does not say, and adds what is proposed for each entry
  (`ignoredEntry` in `src/engine/shapes.ts`).

Acceptance:

- [ ] A checkout with `node_modules/` and `.env` ignored gives two entries, `node_modules` as a folder with its files
      and bytes, and `.env` as a file.
- [ ] A folder with 6,000 files is said to have over its size, 5,000+ files.
- [ ] A `node_modules` that is a link already is listed as a link, with no size.
- [ ] Nothing is listed twice.
- [ ] Extension: with `.env`, `config/one.json`, `config/two.json`, `debug.log`, `node_modules/pkg/index.js` and
      `packages/a/node_modules/x.js` ignored, it lists `.env` (copy), `config` (2 files, nothing), `debug.log`
      (nothing), `node_modules` (link, 7 bytes) and `packages` (nothing).
- [ ] Extension: a link counts no file, and is a folder when it leads to one.

### `worktree-setup.propose` — What is proposed for what git ignores

What each thing git ignores is, which section of the page it goes in, and what is proposed for it.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::propose(input) -> Proposal`
- **Extension**: worked out in TypeScript for the tab of what new worktrees get: `suggest(entry)` in
  `src/shared/worktreeSetup.ts` (`LINKED`, `BUILT`, `NOISE`, `LOCAL`, `EDITOR`, `KINDS`).

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
- [ ] Extension: the folder `.yarn/cache` is a dependency, proposed **Link**; the file `config.local.json` is proposed
      **Copy**; the folder `server/__pycache__` is build output and the folder `.claude/worktrees` other worktrees, both
      proposed **Nothing**; a file named `vendor` and a folder `secrets` are not ones Gitardo knows.

### `worktree-setup.propose-commands` — Commands proposed from lock files

The commands proposed for new worktrees from the lock files the repository commits at its top.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::propose_commands(input) -> Vec<CommandProposal>`
- **Reads**: the files committed at the top among those below,
  `git ls-files -z -- ':(top,literal)<file>'…`, and `packageManager` of `package.json` when a JavaScript lock file is
  there.
- **Extension**: worked out by the engine for the tab of what new worktrees get: `Reads.setupCommands` in
  `src/engine/reads.ts`, asked by `Controller.sendSetup` with `worktree-setup.read-ignored`.

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
- [ ] Extension: `yarn.lock` with `.yarnrc.yml` proposes `yarn install --immutable`; `yarn.lock` with `packageManager`
      `yarn@1.22.22` proposes `yarn install --frozen-lockfile`; `package-lock.json` with `packageManager` `pnpm@9.0.0`
      proposes `npm ci`.
- [ ] Extension: `go.mod`, `Gemfile.lock`, `package-lock.json`, `Cargo.lock`, `poetry.lock` and `uv.lock` propose
      `npm ci`, `uv sync --locked`, `cargo fetch`, `go mod download`, `bundle install`, in that order; `.yarnrc.yml`
      alone proposes nothing.
- [ ] Extension: `Cargo.lock` not committed proposes nothing; once it is staged, `cargo fetch` is proposed.

### `worktree-setup.read-what-patterns-take` — Ask git what the patterns take

What the link and copy patterns of the page take from the folder the views show, as a whole and each on its own.

- **Kind**: Read
- **Function**: `git::read::worktree_setup::read_what_patterns_take(git, request) -> Result<SetupPreview, GitError>`
- **Reads**: the reads of `worktree-setup.folders-to-link` and `worktree-setup.paths-to-copy` for all the patterns, as
  for a worktree that has nothing of them yet; then, for each link pattern alone, how many folders it links, and for
  each copy pattern alone, how many files or folders it copies with the links of all the patterns in place.
- **Uses**: by `worktree-setup.page`, when the page opens and at **Look again**.
- **Extension**: read by the engine for the tab of what new worktrees get: `Reads.setupPreview` in
  `src/engine/reads.ts`, asked by the message `previewWorktreeSetup`, whose case in `Controller.onMessage` cleans the
  patterns first.

Acceptance:

- [ ] With `node_modules/` and `/.env` on a checkout that has both, it says one folder linked and one path copied, each
      pattern taking 1.
- [ ] A copy pattern that matches only files inside a linked folder takes 0.
- [ ] Extension: link `node_modules`, copy `.env`, `config/`, `node_modules/`, `*.missing` count `node_modules` 2,
      `.env` 1, `config/` 1, `node_modules/` 0, `*.missing` 0.
- [ ] Extension: when git fails, the tab says *Git could not say what the patterns take: <error>*.

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
- **Extension**: an editor tab, `WorktreeSetupApp` in `src/webview/WorktreeSetupApp.tsx` (view kind `worktreeSetup`),
  opened by `Controller.openWorktreeSetup()` and fed by `Controller.sendSetup` and `worktreeSetupInfo` in
  `src/extension/worktreeSettings.ts`: what it reads is read by the engine (`worktree-setup.read-ignored`,
  `worktree-setup.propose-commands`, `worktree-setup.read-what-patterns-take`), and what is proposed is worked out in
  TypeScript (`worktree-setup.propose`). Its entries: **Gitardo: Choose What New Worktrees Get…**
  (`gitardo.worktreeSetup`) in the Command Palette and in the **…** of the Branches view; **Choose
  what new worktrees get…** in the **…** of the Worktrees section; **Choose What It Gets…** after a worktree is created;
  **Choose What New Worktrees Get…** in the messages of the Explorer's entries. One tab at a time, for the repository
  the views show: a tab open for it is brought forward, one open for another repository is closed. **Save in** offers
  the workspace folder (its `.vscode/settings.json`) and **User**, VS Code's settings, in place of the repository's
  `.gitardo/settings.json` and the user's file; **Edit in settings.json** in place of **Edit in settings file**. Saving
  stays in the extension (`worktree-setup.save`); **Bring into the other worktree** sends
  `worktree-setup.bring-into-other-worktrees` to the engine.

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
- [ ] Extension: the tab is titled *New Worktrees · <name of the repository's folder>*; while it reads it says *Looking
      at what git ignores…*, and when it cannot, *Gitardo could not read this repository. <error>*.
- [ ] Extension: its head says *Linked now*, *Copied now* and *Run now*, each with the patterns or commands, or
      *nothing*, and *from the settings of <workspace folder>*, *from your user settings*, *from VS Code's
      git.worktreeSymlinkFolders* (or *git.worktreeIncludeFiles*) or *from nothing set*.
- [ ] Extension: sizes read as *340 MB · 1,234 files*, *over 1.2 GB · 5,000+ files* past 5000 files, *512 B* for a
      file, *a link* for a link; *None.* stands for an empty list of patterns.
- [ ] Extension: *Save in* offers the workspace folder (tooltip: its `.vscode/settings.json`) and **User**, starting
      where what applies comes from, and **User** alone when the repository is in no folder of the workspace.
- [ ] Extension: **Discard changes** puts back what applies; **Edit in settings.json** opens the user's
      `settings.json` at `gitardo.worktrees.linkFolders`, or the `.vscode/settings.json` of the workspace folder, made
      by VS Code when it is not there.
- [ ] Extension: **Bring into the other worktree** (or *other <n> worktrees*) is disabled while something is changed
      and not saved, while the views show another repository, and while nothing is linked or copied.
- [ ] Extension: a change to `gitardo.worktrees.copyFiles` made in the Settings editor for the repository shows in the
      tab at once.

### `worktree-setup.choose` — Choose what a line gets

How the buttons of the page change the patterns: by the pattern of a path or of a name, keeping the patterns that take
more than the line.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::choose(input) -> SetupPatterns`
- **Extension**: worked out in the tab, in TypeScript: `patternFor`, `namePattern`, `pathPattern`, `choose`,
  `chooseAll`, `chooseOne`, `groupEntries`, `statusOf`, `statusNow` and `statusOfAll` in `src/shared/worktreeSetup.ts`.

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
- [ ] Extension: with link `vendor/` and copy `node_modules/`, `/.env`, **Link** for the folder `node_modules` gives
      link `vendor/`, `node_modules/` and copy `/.env`; **Nothing** for the file `.env` gives link `vendor/` and copy
      `node_modules/`.
- [ ] Extension: with copy `*.local`, `.env.local`, **Nothing** for `packages/ui/.env.local` alone gives copy
      `*.local`, `/apps/web/.env.local`, `/.env.local`; with copy `.env.local`, `/.env.local`, **Copy** for the
      `.env.local` at the top alone gives copy `.env.local`.
- [ ] Extension: with `node_modules` and `packages/web/node_modules` linked, `packages` gets **Link** in part.

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
- **Extension**: **Save** in the tab of what new worktrees get, done by the extension itself, in TypeScript, in
  VS Code's settings: `saveSetup(root, target, patterns, alsoVSCode, commands?)` in `src/extension/worktreeSettings.ts`,
  from the message `saveWorktreeSetup`, writes `gitardo.worktrees.linkFolders`, `gitardo.worktrees.copyFiles` and
  `gitardo.worktrees.setupCommands` with VS Code's settings API, to the `.vscode/settings.json` of the workspace folder
  or to the user's settings; with **Also in VS Code's git.worktreeSymlinkFolders and git.worktreeIncludeFiles, for the
  worktrees VS Code creates**, those two as well. Nothing is asked, and it is not a plan: nothing of it is shown first
  or written to the log (W12). Under VS Code the engine refuses this feature, and the others that write its own
  settings files: **Inside VS Code, what new worktrees get is in VS Code's settings: Gitardo changes them there, not in
  a file of its own.**, until the engine can have the extension write a setting (`WriteSetting`, Q399).

Acceptance:

- [ ] Saved in the repository, `.gitardo/settings.json` has the three keys, and a comment the file had is still there.
- [ ] With no commands anywhere, saving patterns writes no `worktrees.setupCommands`.
- [ ] With the box ticked, `.vscode/settings.json` gets `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles`
      with the same patterns, and nothing else in it changes.
- [ ] The command log shows a `write` line for each file written.
- [ ] Extension: saving link `node_modules/` and copy `/.env`, `*.missing` for the workspace folder writes them as its
      `gitardo.worktrees.linkFolders` and `gitardo.worktrees.copyFiles`, and the tab then says they come from the
      workspace folder.
- [ ] Extension: commands `git update-ref …` twice and `' '` are saved as the two commands, without the empty one; an
      empty list of commands is written only where one is set somewhere already.
- [ ] Extension: VS Code's two settings change only with the box ticked, and then get the same patterns.
- [ ] Extension: when writing fails, *Gitardo: the settings were not saved. <error>* is said.
- [ ] Extension: `feature/run` of `worktree-setup.save` sent to the engine is refused with **Inside VS Code, what new
      worktrees get is in VS Code's settings: Gitardo changes them there, not in a file of its own.**, and no file is
      written.

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
- **Extension**: **Remove from `<workspace folder>`** or **Remove from User** in the tab of what new worktrees get, done
  by the extension itself, in TypeScript: `saveSetup` in `src/extension/worktreeSettings.ts` writes the three
  `gitardo.worktrees.*` settings as not set there, with VS Code's settings API; not a plan, nothing logged (W12). The
  engine refuses this feature under VS Code, as it refuses `worktree-setup.save`.

Acceptance:

- [ ] With `worktrees.linkFolders: []` in the repository and `["node_modules/"]` for the user, removing from the
      repository makes `node_modules/` apply, from the user.
- [ ] VS Code's `git.worktreeSymlinkFolders` stays in `.vscode/settings.json`.
- [ ] Extension: **Remove from** the workspace folder leaves the three of Gitardo not set there, and what is set
      elsewhere applies again.

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
- **Extension**: **Edit in settings.json** in the tab of what new worktrees get, done by the extension itself, in
  TypeScript: `editSetupJson` in `src/extension/worktreeSettings.ts`, from the message `editWorktreeSetup`, opens the
  user's `settings.json` or the `.vscode/settings.json` of the workspace folder in VS Code's editor, made by VS Code
  when it is not there. The engine refuses this feature under VS Code, as it refuses `worktree-setup.save`.

Acceptance:

- [ ] In a repository without `.gitardo/settings.json`, it makes the file with `{}` and opens it.
- [ ] With **User** chosen, the user's settings file opens.
- [ ] Extension: with **User** chosen, the user's `settings.json` opens at `gitardo.worktrees.linkFolders`; with the
      workspace folder, its `.vscode/settings.json`.

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
- **Extension**: **Link into New Worktrees** in the context menu of a folder in VS Code's Explorer, for the folders
  selected, not in the Command Palette (`gitardo.linkIntoWorktrees` in `package.json`, `explorer/context`, group
  `z_gitardo_worktrees@1`, while the path is not in the context key `gitardo.linkedIntoWorktrees`). Done by the
  extension itself, in TypeScript, in VS Code's settings: `Controller.setUpFromExplorer('link', uris)` in
  `src/extension/controller.ts`, with `changeSetup` and `savePatterns` of `src/extension/worktreeSettings.ts`, writes
  `gitardo.worktrees.linkFolders` where it is set now, the workspace folder or the user's settings, and where it is
  not set in the workspace folder (the user's when the repository is in no folder of the workspace), with VS Code's
  progress *Gitardo: looking at what git ignores* in the status bar meanwhile. It is not a plan: nothing of it is shown
  first or written to the log (W12). The engine refuses this feature under VS Code, as it refuses `worktree-setup.save`. The folder of a repository itself counts as in none.

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
- [ ] Extension: on `node_modules`, which git ignores, the workspace folder's `gitardo.worktrees.linkFolders` becomes
      `["/node_modules/"]`, the message says *node_modules will be linked into new worktrees.* and *Saved in the
      settings of the workspace folder.*, and the menu of `node_modules` then has **Stop Linking into New Worktrees**
      instead.
- [ ] Extension: on `.vscode`, of which git tracks `settings.json`, nothing is saved and the message, a warning, says
      *Git tracks files in .vscode: each worktree has them from its commit.*
- [ ] Extension: on a folder copied by its path, `/node_modules/` moves from the copies to the links.
- [ ] Extension: **Choose What New Worktrees Get…** in the message shows the repository in the views and opens the tab.

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
- **Extension**: **Copy into New Worktrees** in the context menu of a file or a folder in VS Code's Explorer, for those
  selected, not in the Command Palette (`gitardo.copyIntoWorktrees` in `package.json`, `explorer/context`, group
  `z_gitardo_worktrees@2`, while the path is not in the context key `gitardo.copiedIntoWorktrees`). Done by the
  extension itself, in TypeScript, in VS Code's settings, as `worktree-setup.link-into-new-worktrees` does:
  `Controller.setUpFromExplorer('copy', uris)` writes `gitardo.worktrees.copyFiles`, not as a plan and not logged
  (W12). The engine refuses this feature under VS Code, as it refuses `worktree-setup.save`.

Acceptance:

- [ ] On `.env`, ignored, it adds `/.env` to `worktrees.copyFiles` and says **.env will be copied into new worktrees.**
- [ ] On a folder git tracks part of, it adds the pattern and says **What git ignores in `<folder>` will be copied into
      new worktrees.**
- [ ] On a file inside a linked `node_modules`, it writes nothing and says it is in `node_modules`, which the pattern
      links into new worktrees, so nothing in it is copied.
- [ ] On a file git does not ignore, it writes nothing and says **Git does not ignore `<file>`: only what it ignores is
      linked or copied into new worktrees.**
- [ ] Extension: on `.env`, which git ignores, `gitardo.worktrees.copyFiles` gets `/.env` and the message says *.env
      will be copied into new worktrees.*
- [ ] Extension: on `a.txt`, which git tracks, nothing is saved and the warning says *Git tracks a.txt: each worktree
      has it from its commit.*
- [ ] Extension: on `.vscode`, of which git ignores all but `settings.json`, `/.vscode/` is saved and the message says
      *What git ignores in .vscode will be copied into new worktrees.*
- [ ] Extension: on `packages/a/node_modules/x.js` while `node_modules/` is linked, nothing is saved and it says
      *packages/a/node_modules/x.js is in packages/a/node_modules, which the pattern node_modules/ links into new
      worktrees: nothing in it is copied.*

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
- **Extension**: **Stop Linking into New Worktrees** in the context menu of VS Code's Explorer, not in the Command
  Palette (`gitardo.stopLinkingIntoWorktrees` in `package.json`), offered by the context key
  `gitardo.linkedIntoWorktrees` that `showSetupInMenus` and `namedPaths` of `src/extension/worktreeSettings.ts` set when
  the workspace folders or the four settings change. Done by the extension itself, in TypeScript:
  `Controller.setUpFromExplorer('none', uris)` takes the patterns `/<path>` and `/<path>/` out of both lists, saved as
  `worktree-setup.link-into-new-worktrees` saves, not as a plan and not logged (W12); **Stop Copying into New
  Worktrees** does the same. The engine refuses this feature under VS Code, as it refuses `worktree-setup.save`.

Acceptance:

- [ ] With `["/node_modules/", "node_modules/"]`, stopping on `node_modules` leaves `["node_modules/"]` and says it is
      still linked by `node_modules/`.
- [ ] With `["/node_modules/"]` alone, it leaves `[]` and says new worktrees will no longer get it.
- [ ] Extension: after **Link into New Worktrees** on `node_modules`, **Stop Linking into New Worktrees** leaves
      `gitardo.worktrees.linkFolders` an empty list in the workspace folder, and the menu offers **Link into New
      Worktrees** again.
- [ ] Extension: only patterns that name one path, starting with `/` and without `*`, `?`, `[`, `\` or `!`, offer a
      Stop entry: of `/.env`, `/packages/api/vendor/`, `node_modules/`, `apps/*/.env`, `/build/**`, `!/keep`, `/a[b]`,
      only `.env` and `packages/api/vendor`.

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
- **Extension**: **Stop Copying into New Worktrees** in the context menu of VS Code's Explorer
  (`gitardo.stopCopyingIntoWorktrees` in `package.json`, offered by the context key `gitardo.copiedIntoWorktrees`),
  done by the extension itself as `worktree-setup.stop-linking` says: both entries take the patterns of the path out of
  both lists. The engine refuses this feature under VS Code, as it refuses `worktree-setup.save`.

Acceptance:

- [ ] With `["/.env"]`, stopping on `.env` leaves `[]` and says **New worktrees will no longer get .env.**
- [ ] With `["/.env", ".env"]`, it says `.env` is still copied, by the pattern `.env`.

### `worktree-setup.explain-change` — Why a choice from the Files page changes nothing

What becomes of a file or a folder given to new worktrees, or taken from them, from the Files page, as git works the
patterns out, and the sentence that says it.

- **Kind**: Derivation
- **Function**: `model::worktree_setup::explain_change(input) -> SetupChange`
- **Uses**: `worktree-setup.folders-to-link`, `worktree-setup.paths-to-copy`; by the four Files page entries.
- **Extension**: worked out in TypeScript for the four entries of VS Code's Explorer, from what the engine reads:
  `changeSetup(root, entry, choice, patterns)` and `setupChangeText` in `src/extension/worktreeSettings.ts`, with
  `chooseByPath` in `src/shared/worktreeSetup.ts`, ask the engine what the patterns take before and after
  (`worktree-setup.read-what-patterns-take`, `Reads.setupTakes` in `src/engine/reads.ts`); when the patterns do not
  give it what was chosen, `read/worktree-setup.explain-change` (`Reads.pathStatuses`) says whether git tracks or
  ignores it.

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
- [ ] Extension: with `.gitignore` `node_modules/`, `.env`, `.vscode/*`, `!.vscode/settings.json`, linking the folder
      `packages` changes nothing and says *Git ignores only part of packages: only what it ignores is linked or copied
      into new worktrees.*; with link `/node_modules/`, copying the folder `node_modules` gives copy `/node_modules/`.
- [ ] Extension: a folder linked by a pattern of its own name, asked to be copied, says *<name> is linked into new
      worktrees by the pattern <pattern>, which comes before copying.*

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
- **Extension**: **Bring ignored files…** in the context menu of a worktree, beside the graph and in the Branches view
  (`worktreeMenu` in `src/webview/menus.ts`). It sends `feature/run` of `worktree-setup.bring-into-worktree` on the
  worktree to Gitardo's engine (`worktreeSetUp` with a `path` in `engineRuns` of `src/engine/actions.ts`, run by
  `Controller.runOnEngine`), the patterns being those the extension sends (`worktree-setup.settings-in-effect`), its
  danger asked in a modal dialog whatever `gitardo.commandPreview` says (`VscodeAsker`). When the engine's binary is
  missing, it says so (D49).

Acceptance:

- [ ] On a worktree with no `node_modules` and its own `.env`, with `node_modules/` to link and `/.env` to copy, it
      links `node_modules`, says `.env` is in the worktree already, and leaves its `.env` as it was.
- [ ] With no pattern set, it is refused with the message above.
- [ ] Extension: with `.env` deleted from a worktree that had `node_modules` linked and `.env` copied, it asks, then
      logs `copy .env` and *node_modules is in the worktree already.*, and `.env` is back with the content of this
      checkout.
- [ ] Extension: no git command is run and none is shown in the question, and the commands for new worktrees do not
      run.

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
- **Extension**: **Bring ignored files into the other worktrees…** in the **…** of the Worktrees section
  (`worktreesMenu` in `src/webview/menus.ts`); **Bring into the other worktree** (or *other <n> worktrees*) in the tab
  of what new worktrees get (`WorktreeSetupApp`). Each sends `feature/run` of
  `worktree-setup.bring-into-other-worktrees` to Gitardo's engine (`worktreeSetUp` with no `path` in `engineRuns` of
  `src/engine/actions.ts`, run by `Controller.runOnEngine`), its danger asked in a modal dialog (`VscodeAsker`). When
  the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] With two other worktrees and one missing, the plan has steps for the two and none for the missing one.
- [ ] A failure in the first worktree does not stop the second, and one warning at the end says what failed.
- [ ] Extension: it goes into every worktree that is not missing and is not the checkout the views show, the main
      worktree included when the views show another worktree.
- [ ] Extension: with nothing to link or copy, it is refused with *New worktrees get nothing yet: choose what they get
      first.*, and nothing runs.
