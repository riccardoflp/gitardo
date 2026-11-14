# Worktrees

Worktrees are further checkouts of the same repository, each in a folder of its own: creating them from a branch, a
remote branch or a commit, opening them in a window, the Agents window or a terminal, showing one in the views of this
window, renaming, removing and pruning them, and the branch that another worktree has. Also what new worktrees get of
what git ignores in the checkout they are made from: folders linked, files copied, commands run in a terminal, chosen
in the tab of what new worktrees get or from the Explorer, and brought into worktrees that are there already.

## The worktrees and their folders

### `worktree.section` — The Worktrees section

The list of the worktrees of the repository among the references, beside the graph and in the Branches view, each
saying how it stands, with what creates more.

- **Kind**: View
- **Function**: the `worktrees` section of `RefsTree` in `src/webview/RefsTree.tsx`, with `worktreeLabel` and
  `worktreeStatus` of `src/shared/refs.ts`; the files not committed are counted by `Repo.worktreeChanges` in
  `src/git/repo.ts`, the worktrees parsed by `parseWorktrees` in `src/git/parse.ts`.
- **Entry points**: the **Worktrees** section of the references beside the graph and of the Branches view; its title
  has **+** (*Create worktree…*) and **…** (*More actions for worktrees*), and a right click on the title opens the
  same menu as **…**.
- **Reads**: `git worktree list --porcelain`, and `git status --porcelain --untracked-files=all` in each other worktree
  that is not missing, each time the views read the repository.
- **Settings**: `gitardo.worktrees.folder` (how the rows are named), `gitardo.worktrees.openIn` (the double click).
- **Uses**: `worktree.create`, `worktree.switch`, `setup.page`, `setup.bring-ignored`, `worktree.prune`,
  `worktree.open`.
- **Implemented in**: `RefsTree` (`src/webview/RefsTree.tsx`); `worktreesMenu` in `src/webview/menus.ts` for **…**;
  `worktreesFolder` in `src/extension/actions.ts` gives the views `worktreesRoot`.
- **Tested by**: `test/git.test.ts`, "parses worktrees and marks the current one"; `test/integration.test.ts`, "adds,
  lists and removes worktrees".

Acceptance:

- [ ] A worktree inside the folder of the worktrees is named by its path from there, slashes and all
  (`feature/login`); any other by the name of its folder; the main worktree is listed like the others.
- [ ] Each row says its branch, or `detached`, followed by `↑n` and `↓n` against its upstream, or `gone`; a worktree
  whose folder is gone says `missing` instead.
- [ ] A row with files not committed shows a pencil with their number, counted with `--untracked-files=all`: two new
  files in a new folder count as two.
- [ ] Its open pull request shows with its number.
- [ ] The tooltip has, on separate lines: the path, `<branch> → <upstream>` when it has one, `Nothing to commit` or
  `<n> file(s) not committed`, and the pull request.
- [ ] A click selects the commit it has checked out in the graph; a double click opens it as `worktree.open` does,
  except the worktree of this window and a missing one, where it does nothing.
- [ ] **…** lists **Create worktree…**, **Switch worktree…**, **Choose what new worktrees get…**, **Bring ignored files
  into the other worktrees…** (only while a worktree is neither missing nor current) and **Prune missing worktrees**
  (only while one is missing).
- [ ] Counting the files of the other worktrees runs `git status` with `GIT_OPTIONAL_LOCKS=0`, so it never takes the
  lock of their index; what changes in a worktree open elsewhere shows at the next refresh.

### `worktree.folder` — Where a new worktree goes

The folder a new worktree is made in, from `gitardo.worktrees.folder` and the name of the worktree.

- **Kind**: Derivation
- **Function**: `worktreesRoot(main, folder)`, `worktreeDir(main, folder, name)` and `worktreeFolderName(name)` in
  `src/git/ops.ts`; `worktreesHome(state)` in `src/extension/actions.ts` reads the setting.
- **Settings**: `gitardo.worktrees.folder`, `${repo}.worktrees` by default.
- **Uses**: by `worktree.create`, `worktree.create-for-branch`, `worktree.create-at-commit`,
  `worktree.open-in-worktree`, `worktree.rename` and `worktree.section`.
- **Implemented in**: `src/git/ops.ts`; `worktreesHome` and `worktreesFolder` in `src/extension/actions.ts`.
- **Tested by**: `test/git.test.ts`, "puts new worktrees in the folder the setting names, each in a folder named after
  it"; `test/e2e/suite.ts`, step "creates a worktree by name, renames it with its branch and removes both".

Rules of the result:

- The folder of the worktrees is the setting, trimmed, or `${repo}.worktrees` when it is empty. `${repo}` is the name of
  the folder of the main worktree (or of this checkout when git lists no main one). A `~` at its start, followed by a
  slash or nothing, is the home folder.
- Backslashes become forward slashes and slashes at the end go. A path that starts with `/` or a drive letter is
  absolute; any other starts from the folder that holds the main worktree, without a leading `./`.
- The name becomes the folder inside it: split at `/` and `\`, each part with every run of characters other than
  letters, digits, `_`, `.` and `-` made one `-`, the `-` and `.` at its ends taken off, cut to 60 characters, empty
  parts dropped; nothing left is `worktree`. Slashes make folders in folders.

Worked examples:

| Main worktree | Setting | Name | Folder |
| --- | --- | --- | --- |
| `C:/prj/app` | `${repo}.worktrees` | `feature/login` | `C:/prj/app.worktrees/feature/login` |
| `C:\prj\app\` | (empty) | `fix` | `C:/prj/app.worktrees/fix` |
| `/home/me/app` | `./trees/${repo}` | `fix` | `/home/me/trees/app/fix` |
| `/home/me/app` | `/tmp/wt/` | `fix` | `/tmp/wt/fix` |
| `C:/prj/app` | `D:\wt` | `fix` | `D:/wt/fix` |
| | | `a\b//c d` | `a/b/c-d` (inside the folder of the worktrees) |
| | | `-odd name!/` | `odd-name` |
| | | `///` | `worktree` |

Acceptance:

- [ ] Each worked example above gives the folder shown.
- [ ] While a name is typed, the box says `In <folder>`, or `<folder> already exists` as an error.

### `worktree.branch-name` — The name a format makes

The name of the new branch of a worktree from the format of branch names, with the values asked for and the branch it
starts from. The same format names the branches that creating a branch makes.

- **Kind**: Derivation
- **Function**: `expandFormat(format, values)`, `formatInputs(format)`, `formatProblem(format, defined)` and
  `withoutRemote(ref, remotes)` in `src/git/ops.ts`, with `WorktreeNameValues`; the questions are `nameFromFormat`,
  `askInput` and `commitBase` in `src/extension/actions.ts`.
- **Asks**: each `${input:<id>}` in the order it first appears, as `gitardo.branches.inputs` defines it: a
  `promptString` in an input box, with the value used last, or else its `default`, proposed; a `pickString` in a picker
  of its `options`, the value proposed first (with **Used last** when it is not one of them) and **Type another value…**
  when `allowCustom` is true. Then the name it makes, in an input box, to keep or change.
- **Refuses**: a format with a variable that is not `${base}`, `${repo}`, `${date}` or a defined input, before anything
  is asked: *The format uses ${branch}, which is not one of ${base}, ${repo}, ${date} or ${input:<id>}. Change it in
  the settings.*, or *The format uses ${input:kind}, which gitardo.branches.inputs does not define. Change it in the
  settings.*; an empty value (*Enter a value*); a typed value that does not match the `pattern` of its input, which the
  box says as *It has to match <pattern>* and refuses as *<description>: It has to match <pattern>.*; a `pattern` that
  is not a regular expression: *The pattern of the input <id> in gitardo.branches.inputs is not a valid regular
  expression.*
- **Settings**: `gitardo.branches.format` and `gitardo.branches.inputs`; where they are not set (the format empty, the
  inputs an empty list), `gitardo.worktrees.branchFormat` and `gitardo.worktrees.inputs`.
- **Uses**: by `worktree.create`, `worktree.create-for-branch`, `worktree.create-at-commit` and `branch.create`.
- **Implemented in**: `src/git/ops.ts`; `branchFormat`, `nameFromFormat`, `askInput`, `commitBase` in
  `src/extension/actions.ts`. The values typed or picked are kept in the workspace state under `worktreeInputs`.
- **Tested by**: `test/git.test.ts`, "names branches from a format, with inputs, the base without its remote, the
  repository and the date"; `test/e2e/suite.ts`, step "names the branch of a new worktree from the format, asking for
  its inputs, or as it is typed".

Rules of the result:

- `${input:<id>}` is the value given for that input; `${repo}` the name of the folder of the main worktree; `${date}`
  today in the local time, as `2026-10-15`; `${base}` the branch it starts from without its remote (`origin/main` is
  `main`, and with remotes `team` and `team/fork`, `team/fork/fix` is `fix`), or, at a commit, what the picker titled
  `Create worktree at <short>: what ${base} is` gives: a branch at the commit (**Branch at this commit**), the short hash
  (**This commit**) or a branch that has the commit (**Branch that has this commit**), at most 50 of them.
- The name made is checked as any branch name typed: not empty, a valid ref name, not taken, not beside a branch it
  would have to be a folder of or hold (`feature/login` beside `feature`: *Git cannot have it beside the branch
  feature*).

Worked examples (format `PRJ-${input:ticket}-${input:kind}-${base}`):

| Values | Name |
| --- | --- |
| ticket `123`, kind `feat`, base `feature/login` | `PRJ-123-feat-feature/login` |
| format `${repo}/${date}`, repo `app`, date `2026-10-15` | `app/2026-10-15` |
| inputs of `PRJ-${input:ticket}-${input:kind}-${base}-${input:ticket}` | `ticket`, `kind`, in that order |

Acceptance:

- [ ] Each worked example above gives the name shown.
- [ ] The ticket typed last is proposed the next time, in the same workspace.
- [ ] A ticket `12a`, where the pattern is `^\d+$` and the description *Ticket number*, is refused with *Ticket number:
  It has to match ^\d+$.* and nothing is created.
- [ ] With only `gitardo.worktrees.branchFormat` set to `old-${input:ticket}`, that format is used.

### `worktree.create` — Create a worktree

A new worktree on a new branch, made from the branch, remote branch or commit picked, in the folder of the worktrees.

- **Kind**: Operation
- **Function**: `worktreeAddOp(dir, ref, newBranch, detach)` in `src/git/ops.ts`, built by `addWorktree` in
  `src/extension/actions.ts` and given what new worktrees get by `withSetup`.
- **Entry points**: **+** on the title of the Worktrees section; **Create worktree…** in its **…**; **Gitardo: Create
  Worktree…** in the Command Palette and in the **…** of the Branches view; the last entry of **Gitardo: Switch
  Worktree…**.
- **Asks**:
  1. With a format of branch names set, a picker titled `Create worktree`: **Use the format** (with the format as its
     description, where the picker starts) or **Type a branch name**.
  2. A picker titled `Create worktree from`: the branch checked out (**Checked out**), or `HEAD` (**Detached**) when no
     branch is; the other local branches; the remote branches (**Remote**).
  3. With the format, its inputs and the name it makes (`worktree.branch-name`). Otherwise an input box titled `Create
     worktree`, *Name of the new branch, and of the folder of the worktree*, with `feature/my-change` as placeholder.
- **Plan**: `> git worktree add --no-track -b <name> <folder> <base>`, then what new worktrees get (`setup.link-and-copy`)
  and the commands for new worktrees (`setup.commands`). A local branch is given as `refs/heads/<name>` where a tag or a
  remote branch has its name; a remote branch as `refs/remotes/<name>` where needed.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: nothing takes it back: the operation writes nothing in the reflog of `HEAD` of the repository shown, and
  Gitardo remembers nothing for it, so **Undo** right after takes back the operation before it, if the reflog has one,
  after asking. The worktree goes with `worktree.remove`.
- **After**: the message *Worktree created in <folder>* with **Open in Agents Window** (where VS Code has it), **Open in
  New Window** and, the first time, **Choose What It Gets…** (`setup.offer`). With commands for new worktrees, the
  message waits for them (`setup.commands`).
- **Refuses**: an empty name (*Enter a name*); a name that is not a valid ref name, is taken (*A branch with that name
  already exists*) or cannot be beside another branch (*Git cannot have it beside the branch <name>*), said as
  *<name>: <problem>.*; a folder that exists: *<folder> already exists.*; a name starting with `-`: *"<name>" is not a
  valid name.*
- **Settings**: `gitardo.worktrees.folder`, `gitardo.branches.format` and `gitardo.branches.inputs` (or
  `gitardo.worktrees.branchFormat` and `gitardo.worktrees.inputs`), `gitardo.worktrees.linkFolders`,
  `gitardo.worktrees.copyFiles`, `gitardo.worktrees.setupCommands`, `gitardo.commandPreview`.
- **Uses**: `worktree.folder`, `worktree.branch-name`, `setup.source`, `setup.link-and-copy`, `setup.commands`,
  `setup.offer`, `worktree.open`, `preview.ask`, `log.write`, `activity.track`.
- **Implemented in**: `Action` `{ kind: 'worktreeAdd' }` in `src/shared/protocol.ts`; case `worktreeAdd` of
  `resolveAction` and `addWorktree`, `withSetup` in `src/extension/actions.ts`; `worktreeAddOp` in `src/git/ops.ts`;
  `worktreesMenu` in `src/webview/menus.ts` and the section actions of `RefsTree`; `gitardo.createWorktree` in
  `src/extension/extension.ts`; `Controller.action` and `Controller.run`.
- **Tested by**: `test/git.test.ts`, "creates worktrees for an existing branch, a new branch or a detached commit";
  `test/integration.test.ts`, "creates a worktree on a new branch, renames it with its branch and removes both";
  `test/e2e/suite.ts`, steps "creates a worktree by name, renames it with its branch and removes both" and "names the
  branch of a new worktree from the format, asking for its inputs, or as it is typed".
- **Differs**: Undo, against promise 5: `Controller.action` clears what it remembers after any operation
  (`src/extension/controller.ts:1689`) and records nothing for a worktree.

Acceptance:

- [ ] Picking `main` and typing `feature/login` runs `git worktree add --no-track -b feature/login
  <repo>.worktrees/feature/login main`, and the worktree is in the folder `login` inside `feature`.
- [ ] The new branch has no upstream, also when it starts from `origin/main`.
- [ ] The command in the log, and in the question when `gitardo.commandPreview` is `all`, is the one that ran; it is
  logged as `> git worktree add …` before the `symlink`, `copy` and `run` lines of what follows it.
- [ ] Without the command preview, nothing is asked before it runs.
- [ ] A name starting with `-` is refused before anything runs.
- [ ] Typing a name whose folder exists shows *<folder> already exists* under the box, and confirming it creates
  nothing.
- [ ] **Undo** right after does not remove the worktree or its branch (differs).

### `worktree.create-for-branch` — Create a worktree for a branch

Puts a branch in a new worktree, in a folder of its name, or starts a new branch from it there when a format is used.

- **Kind**: Operation
- **Function**: `worktreeAddOp(dir, ref, newBranch, detach)` in `src/git/ops.ts`, built by `addWorktree` in
  `src/extension/actions.ts`.
- **Entry points**: **Create worktree…** in the context menu of a local branch.
- **Offered when**: no worktree has the branch checked out, this one included.
- **Asks**: with a format set, a picker titled `Create worktree for <branch>`: **Use the format** or **Use the branch
  <branch>** (*Check it out in the worktree*). With the branch itself, an input box *Name of the folder of the
  worktree*, starting with the name of the branch; with the format, its inputs and the name (`worktree.branch-name`),
  `${base}` being the branch.
- **Plan**: the branch itself: `> git worktree add <folder> <branch>`; a new branch from the format: `> git worktree add
  --no-track -b <name> <folder> <branch>`; then `setup.link-and-copy` and `setup.commands`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: nothing takes it back, as for `worktree.create`.
- **After**: as `worktree.create`.
- **Refuses**: an empty folder name (*Enter a name*); a folder that exists: *<folder> already exists.*; a branch name
  as `worktree.create` refuses it.
- **Settings**: as `worktree.create`.
- **Uses**: `worktree.folder`, `worktree.branch-name`, `setup.link-and-copy`, `setup.commands`, `setup.offer`.
- **Implemented in**: `branchMenu` in `src/webview/menus.ts` (`{ kind: 'worktreeAdd', ref: name, isBranch: true }`);
  `addWorktree` in `src/extension/actions.ts`.
- **Tested by**: `test/git.test.ts`, "creates worktrees for an existing branch, a new branch or a detached commit";
  `test/menus.test.ts`, "checks out a branch without a worktree, or opens it in one made for it, first as a double click
  does" and "opens the worktree of a branch instead of checking it out, and removes it instead of deleting the branch";
  `test/e2e/suite.ts`, step "names the branch of a new worktree from the format, asking for its inputs, or as it is
  typed".
- **Differs**: the branch is given to git by its short name (`src/webview/menus.ts:333`,
  `src/extension/actions.ts:1443`): as the start of a new branch from the format, a tag of the same name would be
  taken instead of the branch, against promise 3. Undo, as for `worktree.create`.

Acceptance:

- [ ] On `side/work`, keeping the folder name proposed runs `git worktree add <repo>.worktrees/side/work side/work`,
  and the worktree has `side/work` checked out.
- [ ] The menu of the branch checked out here, and of a branch another worktree has, has no **Create worktree…**.
- [ ] A new branch from the format at a branch that shares its name with a tag starts from the branch (differs: it is
  given as the short name).
- [ ] **Undo** right after does not remove the worktree (differs).

### `worktree.create-at-commit` — Create a worktree at a commit

A new worktree at a commit of the graph, on a new branch made there or on a detached `HEAD`.

- **Kind**: Operation
- **Function**: `worktreeAddOp(dir, ref, newBranch, detach)` in `src/git/ops.ts`, built by `addWorktree` in
  `src/extension/actions.ts`.
- **Entry points**: **Create worktree here…** in the context menu of a commit in the graph.
- **Asks**: with a format set, **Use the format** or **Type a branch name**; with the format, what `${base}` is when the
  format has it, then its inputs and the name. Otherwise an input box titled `Create worktree at <short>`: *Name of a
  new branch, and of the folder of the worktree. Leave empty for a detached HEAD.*
- **Plan**: a name: `> git worktree add --no-track -b <name> <folder> <hash>`; nothing typed: `> git worktree add
  --detach <folder> <hash>`, in a folder named after the short hash; then `setup.link-and-copy` and `setup.commands`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: nothing takes it back, as for `worktree.create`.
- **After**: as `worktree.create`.
- **Refuses**: as `worktree.create`; an empty name is not refused here.
- **Settings**: as `worktree.create`.
- **Uses**: `worktree.folder`, `worktree.branch-name`, `setup.link-and-copy`, `setup.commands`, `setup.offer`.
- **Implemented in**: `commitMenu` in `src/webview/menus.ts` (`{ kind: 'worktreeAdd', ref: hash, isBranch: false }`);
  `addWorktree` in `src/extension/actions.ts`.
- **Tested by**: `test/git.test.ts`, "creates worktrees for an existing branch, a new branch or a detached commit";
  `test/e2e/suite.ts`, step "names the branch of a new worktree from the format, asking for its inputs, or as it is
  typed".
- **Differs**: Undo, as for `worktree.create`.

Acceptance:

- [ ] At `1a2b3c4…` with nothing typed, it runs `git worktree add --detach <repo>.worktrees/1a2b3c4 <hash>` and the box
  said *In <repo>.worktrees/1a2b3c4* while empty.
- [ ] With `fix` typed, it runs `git worktree add --no-track -b fix <repo>.worktrees/fix <hash>`.
- [ ] With the format `PRJ-${input:ticket}-${input:kind}-${base}`, the short hash picked as `${base}`, ticket `7` and
  kind `feat`, the branch is `PRJ-7-feat-<short>`, and the picker was titled `Create worktree at <short>: what ${base}
  is`.
- [ ] **Undo** right after does not remove the worktree (differs).

### `worktree.open-in-worktree` — Open a branch in a worktree made for it

Opens a branch in its own worktree, making that worktree first, with nothing asked, when there is none.

- **Kind**: Operation
- **Function**: `worktreeAddOp(dir, ref, newBranch, detach)` or `worktreeTrackOp(dir, branch, remote)` in
  `src/git/ops.ts`, built by `openInWorktree` in `src/extension/actions.ts`.
- **Entry points**: **Open in worktree** in the context menu of a local branch that no worktree has and of a remote
  branch whose local branch no other worktree has; a double click on them with `gitardo.branches.openIn` set to
  `worktree`, which also puts **Open in worktree** before **Check out** in their menus.
- **Reads**: the worktrees, for one that has the branch already.
- **Plan**: a local branch: `> git worktree add <folder> <branch>`; a remote branch with no local branch of its name:
  `> git worktree add --track -b <branch> <folder> <remote branch>`, a remote one with a local branch: the local branch
  as it is; then `setup.link-and-copy` and `setup.commands`. A worktree that has the branch already is only opened.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: nothing takes it back, as for `worktree.create`.
- **After**: the worktree opens where `gitardo.worktrees.openIn` says (`worktree.open`); with commands for new
  worktrees, once they have all succeeded (`setup.commands`).
- **Refuses**: the branch checked out in this window: *<branch> is checked out in this window.*; a branch whose worktree
  is missing: *The worktree of <branch> is missing from <folder>: prune the worktrees to free the branch.*; a folder
  that exists without being its worktree: *<folder> already exists.*
- **Settings**: `gitardo.branches.openIn`, `gitardo.worktrees.openIn`, `gitardo.worktrees.folder`,
  `gitardo.worktrees.linkFolders`, `gitardo.worktrees.copyFiles`, `gitardo.worktrees.setupCommands`.
- **Uses**: `worktree.folder`, `worktree.open`, `setup.link-and-copy`, `setup.commands`, `branch.checkout`.
- **Implemented in**: `Action` `{ kind: 'openInWorktree'; name; remote? }` in `src/shared/protocol.ts`; case
  `openInWorktree` of `resolveAction` in `src/extension/actions.ts`; `branchMenu`, `remoteBranchMenu`, `openBranch`,
  `openRemoteBranch` in `src/webview/menus.ts`.
- **Tested by**: `test/e2e/suite.ts`, steps "opens a branch in its worktree, made for it with nothing asked, and switches
  between worktrees" and "runs the commands for new worktrees in tasks, in order, before opening the worktree, and
  stops at one that fails"; `test/menus.test.ts`, "checks out a branch without a worktree, or opens it in one made for
  it, first as a double click does" and "checks out a branch on a double click, or opens it in a worktree, and does
  nothing for the one checked out".
- **Differs**: Undo, as for `worktree.create`.

Acceptance:

- [ ] On `merged`, it runs `git worktree add <repo>.worktrees/merged merged` and opens that folder, with no question
  asked while `gitardo.commandPreview` is `off`.
- [ ] Again on `merged`, nothing runs and the same folder opens.
- [ ] On `origin/remote/only`, with no local `remote/only`, it runs `git worktree add --track -b remote/only
  <repo>.worktrees/remote/only origin/remote/only`, and `remote/only` follows `origin/remote/only`.
- [ ] On the branch checked out here, nothing runs and *<branch> is checked out in this window.* is said.
- [ ] It never offers **Choose What It Gets…**.
- [ ] **Undo** right after does not remove the worktree (differs).

### `worktree.open` — Open a worktree

Opens a worktree as the folder of a new VS Code window, or in the Agents window of VS Code for agent sessions in it.

- **Kind**: View
- **Function**: `openWorktree(dir, where)` in `src/extension/actions.ts`, with `openIn(state)` for where it opens when
  nothing says.
- **Entry points**: **Open in Agents Window** and **Open in new window** in the context menu of a worktree; **Open
  worktree in Agents Window** and **Open worktree in new window** at the top of the menu of a branch, or of a remote
  branch, that another worktree has; a double click on a worktree, on such a branch and on its label in the graph;
  **Gitardo: Switch Worktree…**; the buttons **Open in Agents Window** and **Open in New Window** of the message after
  a worktree is created.
- **Offered when**: never for a missing worktree; **Open in new window** not for the worktree of this window; **Open in
  Agents Window** only where VS Code has the command `workbench.action.openAgentsWindow`, looked for once per session.
- **Settings**: `gitardo.worktrees.openIn`: `window` (default) or `agents`, for the double click, **Open in worktree**
  and **Switch Worktree**; `agents` opens a new window where VS Code has no Agents window.
- **Uses**: by `worktree.section`, `worktree.switch`, `worktree.create`, `worktree.open-in-worktree`,
  `worktree.branch-elsewhere`, `branch.checkout`.
- **Implemented in**: `Action` `{ kind: 'worktreeOpen'; path; where? }` in `src/shared/protocol.ts`; case
  `worktreeOpen` of `resolveAction` and `AGENTS_WINDOW` in `src/extension/actions.ts`; `worktreeOpenEntries` in
  `src/webview/menus.ts`; `agentsWindow` of the state, from `hasCommand` in `src/extension/controller.ts`.
- **Tested by**: `test/menus.test.ts`, "says what a worktree menu opens without naming the worktree" and "opens the
  worktree of a branch instead of checking it out, and removes it instead of deleting the branch"; `test/e2e/suite.ts`,
  step "opens a branch in its worktree, made for it with nothing asked, and switches between worktrees" (the new
  window only). The Agents window needs a signed-in account: no test reaches it.

Acceptance:

- [ ] **Open in new window** runs `vscode.openFolder` on the folder with `forceNewWindow`.
- [ ] **Open in Agents Window** runs `workbench.action.openAgentsWindow` with `{ folderUri }`; when that fails, the
  warning *The Agents window did not open (<reason>), so the worktree opens in a new window.* shows and a new window
  opens.
- [ ] With `gitardo.worktrees.openIn` set to `agents` in a VS Code without the Agents window, a double click opens a new
  window.
- [ ] Nothing in the repository changes, and nothing is written to the log.

### `worktree.open-terminal` — Open a terminal in a worktree

A terminal of this window in the folder of a worktree.

- **Kind**: View
- **Function**: the case `worktreeTerminal` of `resolveAction` in `src/extension/actions.ts`.
- **Entry points**: **Open in terminal** in the context menu of a worktree; **Open terminal in worktree** in that of a
  branch another worktree has.
- **Offered when**: the worktree is not missing.
- **Implemented in**: `Action` `{ kind: 'worktreeTerminal'; path }` in `src/shared/protocol.ts`; `worktreeOpenEntries`
  in `src/webview/menus.ts`.
- **Tested by**: `test/menus.test.ts`, "says what a worktree menu opens without naming the worktree" (the entry only).

Acceptance:

- [ ] It opens and shows a terminal named after the folder of the worktree, starting in that folder.

### `worktree.reveal` — Show a worktree in the file manager

Shows the folder of a worktree in the file manager of the system.

- **Kind**: View
- **Function**: the case `worktreeReveal` of `resolveAction` in `src/extension/actions.ts`, which runs VS Code's
  `revealFileInOS`.
- **Entry points**: in the context menu of a worktree, **Reveal in File Explorer** on Windows, **Reveal in Finder** on
  macOS, **Open containing folder** elsewhere; in that of a branch another worktree has, **Reveal worktree in File
  Explorer**, **Reveal worktree in Finder** or **Open folder of worktree**.
- **Offered when**: the worktree is not missing.
- **Implemented in**: `Action` `{ kind: 'worktreeReveal'; path }` in `src/shared/protocol.ts`; `REVEAL` and
  `worktreeOpenEntries` in `src/webview/menus.ts`.
- **Tested by**: `test/menus.test.ts`, "opens the worktree of a branch instead of checking it out, and removes it
  instead of deleting the branch" (the entry only).

Acceptance:

- [ ] It shows the folder of the worktree in the file manager, the way VS Code's own **Reveal in File Explorer** does.

### `worktree.copy-path` — Copy the path of a worktree

Puts the path of a worktree on the clipboard.

- **Kind**: View
- **Function**: the **Copy path** entry of `worktreeMenu` in `src/webview/menus.ts`, which posts `{ type: 'copy' }`.
- **Entry points**: **Copy path** in the context menu of a worktree.
- **Offered when**: always, a missing worktree included.
- **Implemented in**: `worktreeMenu` in `src/webview/menus.ts`; the `copy` message of `Controller.onMessage`.
- **Tested by**: *none*.

Acceptance:

- [ ] The clipboard holds the path as git lists it, with forward slashes.

### `worktree.show-in-graph` — Show what a worktree has checked out in the graph

Selects, in the graph, the commit a worktree has checked out.

- **Kind**: View
- **Function**: `revealEntry(hash, onReveal)` in `src/webview/menus.ts`, given the hash of the worktree.
- **Entry points**: **Show in the graph** in the context menu of a worktree, beside the graph and in the Branches view.
- **Offered when**: the worktree is not missing.
- **Implemented in**: `worktreeMenu` in `src/webview/menus.ts`; `onReveal` of `RefsTree`.
- **Tested by**: *none*.

Acceptance:

- [ ] The commit the worktree has checked out is selected in the graph, the history loaded as far back as it is.

### `worktree.switch` — Switch worktree

A picker of the worktrees with how they stand, which opens the one picked or creates another.

- **Kind**: View
- **Function**: `Controller.switchWorktree()` in `src/extension/controller.ts`.
- **Entry points**: **Gitardo: Switch Worktree…** in the Command Palette and in the **…** of the Branches view;
  **Switch worktree…** in the **…** of the Worktrees section.
- **Asks**: a picker titled `Switch Worktree`, *The worktree to open*, searching the descriptions and paths too.
- **Settings**: `gitardo.worktrees.openIn`.
- **Uses**: `worktree.open`, `worktree.create`.
- **Implemented in**: `UiCommand` `switchWorktree` in `src/shared/protocol.ts`; `Controller.command`;
  `gitardo.switchWorktree` in `src/extension/extension.ts`; `worktreesMenu` in `src/webview/menus.ts`.
- **Tested by**: `test/e2e/suite.ts`, step "opens a branch in its worktree, made for it with nothing asked, and
  switches between worktrees".

Acceptance:

- [ ] Each worktree that is not missing is listed, the main one with the home icon and the others with the worktree
  icon, named as in `worktree.section`, its path under it.
- [ ] Its description is the branch, or `detached at <short>`, with `↑n ↓n`, then a pencil with the files not
  committed, the pull request, and `this window` for the one open here, joined by ` · `: `merged · $(edit) 1`.
- [ ] The worktree of this window is listed last, and the last entry, after a line, is **Create worktree…**.
- [ ] Picking the worktree of this window does nothing; picking another opens it where `gitardo.worktrees.openIn` says;
  **Create worktree…** runs `worktree.create`.

### `worktree.show-here` — Show another worktree in this window

Points the views of this window at another worktree, to work in it without opening another window.

- **Kind**: View
- **Function**: `Controller.selectRepo(root)` in `src/extension/controller.ts`, reached by the message `selectRepo`.
- **Entry points**: **Show in Gitardo** in the context menu of a worktree; **Show worktree in Gitardo** in that of a
  branch another worktree has; the branch at the start of the toolbar of the graph (`worktree.select`); a worktree
  selected in VS Code's Repositories view (`repo.follow-source-control`); **Gitardo: Select Repository**, which lists
  the worktree shown as *worktree, current*.
- **Offered when**: the worktree is neither missing nor the one the views show.
- **Reads**: for a folder that is neither a repository of the workspace nor a worktree of the one shown,
  `git rev-parse --git-common-dir` in it and in each repository of the workspace, to find the one it is a worktree of.
- **Uses**: `worktree.banner`.
- **Implemented in**: `worktreeOpenEntries` in `src/webview/menus.ts`; `selectRepo` and `pickRepo` in
  `src/extension/controller.ts`; `home` of `RepoState` in `src/shared/protocol.ts`.
- **Tested by**: `test/e2e/suite.ts`, step "shows another worktree in the views of this window, works in it there and
  goes back"; `test/menus.test.ts`, "shows another worktree in the views of this window, and keeps the folder of the
  window from being removed there".

Acceptance:

- [ ] The graph, the Changes view, the Commits view and the Branches view show the worktree: its branch, its files not
  committed, its commits.
- [ ] Staging `shown.txt` there stages it in the index of that worktree, not in the one of the window's folder.
- [ ] The folder of the window is listed as another worktree meanwhile; it has no **Rename…**, **Remove…** or **Bring
  ignored files…**, and removing it is refused with *This worktree is the folder open in this window: remove it from
  another one.*
- [ ] A worktree of another repository of the workspace is shown too, with that repository as the one to go back to
  and as the one chosen in the list of repositories; a folder that is a worktree of none of them is not shown.
- [ ] The list of repositories keeps the repository the worktree is of chosen, and does not list the worktree.
- [ ] The choice is not remembered: the next time the window opens, the views show its own folder.

### `worktree.select` — Choose the worktree shown

The branch checked out, at the start of the toolbar of the graph, which also shows another worktree of the repository
when it has more than one.

- **Kind**: View
- **Function**: `WorktreeSelect` in `src/webview/WorktreeSelect.tsx`.
- **Entry points**: the branch at the start of the toolbar of the Graph view and of the Gitardo Graph tab.
- **Offered when**: the repository has two worktrees or more whose folders are there; otherwise the branch is only said.
- **Uses**: `worktree.show-here`, `worktree.section` (how a folder is named).
- **Implemented in**: `WorktreeSelect`, used by `Toolbar` in `src/webview/Toolbar.tsx`; the message `selectRepo` to
  `Controller.selectRepo`.
- **Tested by**: *none* (the e2e step "shows another worktree in the views of this window, works in it there and goes
  back" checks `Controller.selectRepo`, which it posts to).

Acceptance:

- [ ] It says the branch checked out, or `detached at <short>`, or `no commits yet`, with the path of the worktree
  shown as its tooltip.
- [ ] With a single worktree whose folder is there, it is a label with the branch icon and a click does nothing.
- [ ] With two or more it is a button with a chevron, and the worktree icon while the views show a worktree other than
  the main one. A click lists the worktrees whose folders are there, in the order git lists them, each by its branch or
  `detached at <short>`, followed by its folder named as in `worktree.section` when that is not the name of its branch;
  the one shown has a check.
- [ ] Picking another worktree shows it as `worktree.show-here` does; picking the one shown does nothing.
- [ ] The button is disabled while an operation runs.

### `worktree.banner` — The worktree banner

Says, above the views, which worktree they show while it is not the folder of this window, and goes back to it.

- **Kind**: View
- **Function**: `WorktreeBanner` in `src/webview/WorktreeBanner.tsx`.
- **Offered when**: the views show another worktree (`home` of the state is set): at the top of the Graph view (not
  of the Gitardo Graph), the Changes view and the Branches view.
- **Uses**: `worktree.show-here`.
- **Implemented in**: `WorktreeBanner`, used by `App.tsx` (when not compact), `ChangesApp.tsx` and `BranchesApp.tsx`.
- **Tested by**: *none* (the e2e step "shows another worktree in the views of this window, works in it there and goes
  back" checks the `home` it is drawn from).

Acceptance:

- [ ] It says *Showing the worktree <name> on <branch>*, or without *on <branch>* on a detached `HEAD`, the name as in
  `worktree.section`, its tooltip the full path.
- [ ] **Back to <repository>**, with the path of the window's folder as tooltip, points the views at it again and the
  banner goes.
- [ ] The button is disabled while an operation runs, in the Graph and Changes views.

### `worktree.rename` — Rename a worktree

Gives the folder of a worktree another name, and its branch along with it if wanted.

- **Kind**: Operation
- **Function**: `worktreeMoveOp(dir, to, branch?)` in `src/git/ops.ts`, built by `renameWorktree` in
  `src/extension/actions.ts`.
- **Entry points**: **Rename…** in the context menu of a worktree; **Rename worktree…** in that of a branch another
  worktree has.
- **Offered when**: the worktree is not missing, not the main worktree, not current and not the folder of the window.
- **Asks**:
  1. An input box titled `Rename worktree <old>`: *New name of its folder, which can have slashes* inside the folder of
     the worktrees, where its name is its path from there; *New name of its folder* elsewhere, where slashes become
     `-`. It starts with the old name and says *In <folder>* or *<folder> already exists*.
  2. With a branch, a picker titled `Rename worktree <old> to <new>`: **Rename the branch <branch> too** or **Keep the
     branch <branch>**.
  3. Renaming it, an input box titled `Rename branch <branch>`, starting with the name typed for the folder when the
     folder was named after the branch and that name is free and valid, or else with the branch.
- **Plan**: `> git.close <folder>` when VS Code's Git extension has it open, the folder that will hold it made, then `>
  git worktree move <folder> <new folder>`, and `> git branch -m <branch> <new branch>` when the branch is renamed;
  then the folders that held only it go, up to the folder of the worktrees.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`); the question notes *Git cannot move a folder
  that another program has open, such as another VS Code window or a terminal in it.*
- **Undo**: nothing takes it back: neither the move nor the rename of a branch not checked out here writes in the
  reflog of `HEAD` of the repository shown, and Gitardo remembers nothing for it, so **Undo** right after takes back
  the operation before it, after asking.
- **Refuses**: the main worktree: *Git cannot move the main worktree, the folder the repository was created in.*; the
  worktree of this window: *This worktree is the folder open in this window: rename it from another one.*; a folder
  that exists: *<folder> already exists.*; a name left as it was does nothing.
- **Settings**: `gitardo.worktrees.folder`.
- **Uses**: `worktree.folder`, `preview.ask`, `log.write`, `activity.track`.
- **Implemented in**: `Action` `{ kind: 'worktreeRename'; path }` in `src/shared/protocol.ts`; case `worktreeRename` of
  `resolveAction`, `renameWorktree`, `closeInVSCodeGit`, `recoverHeldWorktree`, `removeEmptyParents` in
  `src/extension/actions.ts`; `worktreeMoveOp` in `src/git/ops.ts`; `worktreeMenu`, `branchMenu` in
  `src/webview/menus.ts`.
- **Tested by**: `test/git.test.ts`, "renames a worktree, and its branch when asked"; `test/integration.test.ts`,
  "creates a worktree on a new branch, renames it with its branch and removes both"; `test/e2e/suite.ts`, step "creates
  a worktree by name, renames it with its branch and removes both".
- **Differs**: Undo, as for `worktree.create` (`src/extension/controller.ts:1689`). When the move succeeded and the
  rename of the branch failed, the folders that held only the worktree stay, as `finish` runs only after every command
  succeeded (`src/extension/controller.ts:2423`).

Acceptance:

- [ ] Renaming `agents/first` to `agents-two` with its branch runs `git worktree move <wt>/agents/first
  <wt>/agents-two` and `git branch -m agents/first agents-two`; the folder `agents` that held only it is gone.
- [ ] The title of the operation is `Rename worktree to agents-two and branch agents/first to agents-two`.
- [ ] A worktree outside the folder of the worktrees renamed to `a/b` goes to `a-b` beside it.
- [ ] Renaming the main worktree is refused with the message above and nothing runs.
- [ ] On Windows, a move refused with `Permission denied`, `Invalid argument` or `Device or resource busy` is said in
  the log as *…: Windows holds the folder, so it is tried again.* and run again up to 20 times, 200 ms apart; the one
  that succeeds is logged as `> git worktree move …`.
- [ ] A name starting with `-` cannot reach git: the folder name loses its leading `-`, and a branch name starting
  with `-` is refused as *That is not a valid Git ref name*.
- [ ] **Undo** right after does not move it back (differs).

### `worktree.remove` — Remove a worktree

Deletes the folder of a worktree, and its branch too if wanted and merged.

- **Kind**: Operation
- **Function**: `worktreeRemoveOp(dir, force, branch?)` in `src/git/ops.ts`, completed by the case `worktreeRemove` of
  `resolveAction` in `src/extension/actions.ts` with `closeInVSCodeGit`, `unlinkFolderLinks`, `recoverHeldWorktree` and
  `removeEmptyParents`.
- **Entry points**: **Remove…** in the context menu of a worktree; **Remove worktree…** and **Remove worktree and
  delete branch…** in that of a branch another worktree has.
- **Offered when**: the worktree is not the main one, not current and not the folder of the window; **Remove…** also
  for a missing worktree.
- **Asks**: from **Remove…** of a worktree with a branch, a picker titled `Remove worktree <name of its folder>`:
  **Remove the worktree** (*Keep the branch <branch>*) or **Remove the worktree and delete <branch>** (*Only if it is
  merged*). The entries of the branch's menu say it themselves. Then the question of the command preview, always.
- **Plan**: `> git.close <folder>` when VS Code's Git extension has it open; on Windows, `> unlink <link>` for each link
  in it to a folder outside it; `> git worktree remove <folder>`, and `> git branch -d <branch>` with the branch; then
  the folders that held only it go, up to the folder of the worktrees. On Windows, when git let go of the worktree but
  could not delete its folder, `> delete <folder>`.
- **Danger**: Confirm, because the folder is deleted with what git ignores in it: the question is *Remove worktree
  <folder>?* with *The directory is deleted. Its branch and commits stay in the repository.* (or *The branch <branch> is
  deleted too if it is merged; if it is not, git keeps it and says so.*) and the commands. Destructive for the forced
  removal that follows a refusal, because the files not committed are deleted.
- **Undo**: nothing takes it back: the folder is gone, and a branch deleted with it is not remembered as `branch.delete`
  remembers one, so **Undo** right after takes back the operation before it, after asking.
- **Refuses**: a folder that is no longer a worktree: *<folder> is no longer a worktree of this repository.*; the folder
  of the window: *This worktree is the folder open in this window: remove it from another one.*
- **Settings**: `gitardo.worktrees.folder` (how far up empty folders go), `gitardo.commandPreview`.
- **Uses**: `preview.ask`, `log.write`, `activity.track`, `branch.delete`.
- **Implemented in**: `Action` `{ kind: 'worktreeRemove'; path; deleteBranch? }` in `src/shared/protocol.ts`; the case
  `worktreeRemove` of `resolveAction` and its `leaveNoEmptyFolders`, `closeInVSCodeGit`, `recoverHeldWorktree`,
  `removeEmptyParents`, `removeFolder` in `src/extension/actions.ts`; `unlinkFolderLinks` in `src/git/worktreeSetup.ts`;
  `worktreeRemoveOp` in `src/git/ops.ts`; `worktreeMenu`, `branchMenu` in `src/webview/menus.ts`;
  `Controller.runWithFallback`.
- **Tested by**: `test/integration.test.ts`, "adds, lists and removes worktrees", "creates a worktree on a new branch,
  renames it with its branch and removes both" and "links and copies into a new worktree what git ignores, as the
  settings of VS Code ask"; `test/e2e/suite.ts`, steps "creates a worktree by name, renames it with its branch and
  removes both", "links and copies into a new worktree what the settings of VS Code ask, and unlinks before removing
  it" and "shows another worktree in the views of this window, works in it there and goes back"; `test/menus.test.ts`,
  "opens the worktree of a branch instead of checking it out, and removes it instead of deleting the branch".
- **Differs**: Undo, against promise 5 (`src/extension/controller.ts:1689`). When `git branch -d` refuses a branch that
  is not merged, the folders that held only the worktree stay, as `finish` runs only after every command succeeded
  (`src/extension/controller.ts:2423`), against the README's "A folder left empty when a worktree is renamed or removed
  goes too".

Acceptance:

- [ ] Removing a clean worktree with its merged branch `agents-two` asks first, then runs `git worktree remove <folder>`
  and `git branch -d agents-two`; the worktree, the branch and the folder are gone.
- [ ] A worktree with files not committed is refused by git with *…contains modified or untracked files, use --force to
  delete it*; Gitardo then asks again, as a warning, *Remove worktree <folder> (force)?* with *This worktree has
  modified or untracked files. They will be deleted with it.*, and runs `git worktree remove --force <folder>`.
- [ ] A branch that is not merged is kept, git saying why, and the worktree is removed.
- [ ] The log has, in this order, `> git.close <folder>` (when VS Code's Git extension had it open), the `> unlink`
  lines on Windows, then `> git worktree remove <folder>`; after it, VS Code's Git extension no longer has it.
- [ ] On Windows, before git runs, every link in the worktree to a folder outside it is taken out, looking only at what
  git lists as untracked or ignored there (`git ls-files --others --directory`, with and without `--ignored
  --exclude-standard`) and inside the folders among them, never past a link; links to files, links to nothing and links
  to folders inside the worktree stay. The folders they link to, in this checkout, keep every file.
- [ ] On Windows, a link that cannot be taken out stops the removal before git runs.
- [ ] On Windows, the links are taken out once the user agreed, before the first `git worktree remove`: when git then
  refuses for files not committed and the forced removal is declined, the worktree stays without them.
- [ ] On Windows, when git said `failed to delete '<folder>'` and no longer lists the worktree, the log says *…: Windows
  held the folder. It is no longer a worktree, and Gitardo deletes what is left of it.* and `> delete <folder>`, and
  the branch is deleted after it; when that delete fails too, *<folder> was not deleted: <reason>* and the operation
  fails.
- [ ] Elsewhere than Windows, no `unlink` or `delete` line is written.
- [ ] Removing `side/work` with **Remove worktree…** from its branch asks no picker and keeps the branch; the folder
  `side` that held only it is gone.
- [ ] Removing the folder of the window while the views show another worktree is refused and nothing runs.
- [ ] **Undo** right after brings back neither the worktree nor its branch (differs).

### `worktree.prune` — Prune missing worktrees

Has git forget the worktrees whose folder is gone, which frees their branches.

- **Kind**: Operation
- **Function**: `worktreePruneOp()` in `src/git/ops.ts`.
- **Entry points**: **Prune missing worktrees** in the context menu of a worktree and in the **…** of the Worktrees
  section.
- **Offered when**: at least one worktree is missing.
- **Plan**: `> git worktree prune`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: nothing takes it back: git keeps no trace of what it forgot, and Gitardo remembers nothing for it, so
  **Undo** right after takes back the operation before it, after asking.
- **Uses**: `preview.ask`, `log.write`.
- **Implemented in**: `Action` `{ kind: 'worktreePrune' }` in `src/shared/protocol.ts`; case `worktreePrune` of
  `resolveAction` in `src/extension/actions.ts`; `worktreeMenu`, `worktreesMenu` in `src/webview/menus.ts`.
- **Tested by**: *none*.
- **Differs**: Undo, as for `worktree.create` (`src/extension/controller.ts:1689`).

Acceptance:

- [ ] With a worktree whose folder was deleted, it runs `git worktree prune`, and the Worktrees section no longer lists
  it.
- [ ] With no missing worktree, the entry is not offered.
- [ ] Its branch can then be checked out here.

### `worktree.branch-elsewhere` — The branch another worktree has

A branch that another worktree has checked out is worked on there: Gitardo opens that worktree instead of checking the
branch out, and offers in its menu what concerns the worktree.

- **Kind**: View
- **Function**: `worktreeOf(branch, state)` in `src/shared/refs.ts`, with `branchMenu`, `remoteBranchMenu`,
  `openBranch`, `openRemoteBranch` and `dropMenu` in `src/webview/menus.ts`.
- **Entry points**: the branch among the local branches, its label in the graph, and the remote branch of the same
  name.
- **Uses**: `worktree.open`, `worktree.open-terminal`, `worktree.reveal`, `worktree.show-here`, `worktree.rename`,
  `worktree.remove`, `branch.checkout`, `sync.fast-forward`.
- **Implemented in**: `src/webview/menus.ts`; the branch items of `RefsTree` (`src/webview/RefsTree.tsx`) and the
  labels of `src/webview/labels.ts`.
- **Tested by**: `test/menus.test.ts`, "finds the other worktree that has a branch, and not this one", "opens the
  worktree of a branch instead of checking it out, and removes it instead of deleting the branch", "opens the main
  worktree from another one, which cannot be removed", "offers no rebase of a branch another worktree has, by dragging
  it onto a commit" and "opens the worktree that has the local branch of its name, instead of checking it out".

Acceptance:

- [ ] Its row among the local branches has the worktree icon, and its tooltip a line *In the worktree <folder>* with
  the files not committed there; its label in the graph has the worktree icon too.
- [ ] Its menu starts with **Open worktree in Agents Window** (where VS Code has it), **Open worktree in new window**,
  **Open terminal in worktree**, the reveal entry and **Show worktree in Gitardo**.
- [ ] Its menu has no **Check out**, **Delete** or **Create worktree…**, and has **Rename branch…**, **Rename
  worktree…**, **Remove worktree…** and **Remove worktree and delete branch…**, except for the main worktree's branch
  seen from another worktree and for the branch of the window's folder, which have neither rename nor remove of the
  worktree.
- [ ] A double click on it opens its worktree where `gitardo.worktrees.openIn` says; while its worktree is missing, it
  does nothing.
- [ ] The menu of the remote branch of the same name starts with the same entries, without **Check out**.
- [ ] Dragged onto a commit of the graph, it offers no rebase.

## What new worktrees get

### `setup.source` — Which settings apply

Which patterns new worktrees of a repository get, and which commands run in them, and where each comes from.

- **Kind**: Derivation
- **Function**: `setupPatterns(root)` and `setupCommands(root)` in `src/extension/worktreeSettings.ts`, read with
  `getConfiguration(section, Uri.file(root)).inspect`.
- **Settings**: `gitardo.worktrees.linkFolders`, `gitardo.worktrees.copyFiles`, `gitardo.worktrees.setupCommands` (all
  three of scope `resource`), and VS Code's `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles`.
- **Uses**: by `worktree.create`, `worktree.create-for-branch`, `worktree.create-at-commit`,
  `worktree.open-in-worktree`, `setup.bring-ignored`, `setup.page`, `setup.explorer-link`, `setup.offer`.
- **Implemented in**: `src/extension/worktreeSettings.ts`; `cleanPatterns`, `cleanCommands` in
  `src/shared/worktreeSetup.ts`.
- **Tested by**: `test/worktreeSetup.test.ts`, "keeps patterns trimmed, once each, without empty ones or line breaks"
  and "keeps commands trimmed and in their order, the same one twice too, without empty ones or line breaks";
  `test/e2e/suite.ts`, step "chooses what new worktrees get in a tab of its own, saves it for the workspace folder, and
  brings it into worktrees".

Rules of the result:

- For the folders to link and for the files to copy, each on its own: Gitardo's setting of the workspace folder (or of
  the workspace file) when it is set there, even to an empty list (source `workspace`); else the user's, set even
  empty (`user`); else VS Code's setting, `vscode` when it has patterns and `none` when not.
- The commands: Gitardo's setting of the workspace folder or file, then the user's; VS Code has none.
- Patterns are kept trimmed, once each, without empty ones, line breaks or values that are not text; commands the same
  but in their order and with repeats kept.

Worked examples:

| Set | Applies |
| --- | --- |
| user `linkFolders: ["node_modules/"]`, workspace folder `linkFolders: []` | nothing linked, from `workspace` |
| only VS Code's `git.worktreeSymlinkFolders: ["node_modules"]` | `node_modules`, from `vscode` |
| `[" node_modules/ ", "", "node_modules/", "a\nb", 3, "/.env"]` | `node_modules/`, `/.env` |
| commands `[" npm ci ", "", "npm run build", "a\nb", 3, "npm ci"]` | `npm ci`, `npm run build`, `npm ci` |

Acceptance:

- [ ] Each worked example above applies as shown.

### `setup.folders-to-link` — The folders a pattern links

The folders of this checkout that the patterns of the folders to link take, worked out the way VS Code's Git extension
does.

- **Kind**: Read
- **Function**: `foldersToLink(root, patterns)` in `src/git/worktreeSetup.ts`.
- **Reads**: `git ls-files --others --ignored --exclude-standard -z`, the same with `--exclude-from=<file of the
  patterns>` instead of `--exclude-standard`, and with `--directory`; then `git check-ignore --no-index -z --stdin` for
  the candidate folders in this checkout, and in an empty repository made in a temporary folder with
  `core.excludesFile` set to the patterns. All with `GIT_OPTIONAL_LOCKS=0`; the temporary folder is removed after.
- **Uses**: by `setup.link-and-copy`, `setup.preview`, `setup.explorer-change`.
- **Implemented in**: `src/git/worktreeSetup.ts`.
- **Tested by**: `test/integration.test.ts`, "links and copies into a new worktree what git ignores, as the settings of
  VS Code ask".

Rules of the result:

- A candidate is every folder that holds a file git ignores and the patterns match.
- A candidate is linked when git ignores it as a folder, the patterns name it as a folder, and it is, or is inside, a
  folder git lists as wholly ignored; of those, only the outermost.
- Paths are relative to the checkout, with forward slashes.

Worked example: with `.gitignore` `node_modules/`, and `node_modules/pkg/index.js` and
`packages/a/node_modules/x.js` in the checkout, the pattern `node_modules` takes `node_modules` and
`packages/a/node_modules`.

Acceptance:

- [ ] The worked example gives the two folders.
- [ ] No pattern takes nothing, without running git.

### `setup.paths-to-copy` — The files a pattern copies

The files and folders of this checkout that the patterns of the files to copy take for a worktree.

- **Kind**: Read
- **Function**: `pathsToCopy(root, worktree, patterns, linked)` in `src/git/worktreeSetup.ts`.
- **Reads**: in this checkout, `git ls-files --others --ignored --exclude-standard -z`, the same with `--directory
  --no-empty-directory`, and `git ls-files --others --ignored --exclude-from=<file of the patterns> -z`; in the
  worktree, `git ls-files -z`. All with `GIT_OPTIONAL_LOCKS=0`.
- **Uses**: by `setup.link-and-copy`, `setup.preview`, `setup.explorer-change`.
- **Implemented in**: `src/git/worktreeSetup.ts`.
- **Tested by**: `test/integration.test.ts`, "links and copies into a new worktree what git ignores, as the settings of
  VS Code ask".

Rules of the result:

- A file is copied when git ignores it, the patterns match it, it is not inside a folder linked, and the worktree does
  not have it from its commit: not the same path, not a folder it has files in, not under a file it has.
- A folder git lists as wholly ignored, all of whose files are copied, is copied as one folder; the files of other
  folders one by one.

Worked example: with `.gitignore` `node_modules/`, `.env`, `*.log`, `config/`, the patterns `.env`, `config/`,
`node_modules/` with `node_modules` linked copy `.env` and `config` (a whole folder of `one.json` and `two.json`), not
`node_modules` (linked) and not `debug.log` (not asked for).

Acceptance:

- [ ] The worked example gives `.env` and `config`.
- [ ] No pattern takes nothing, without running git.

### `setup.link-and-copy` — Link and copy into a new worktree

Once git has made a worktree, gives it the folders and files the settings ask for, from the checkout it is made from.

- **Kind**: Operation
- **Function**: `setUpWorktree(root, worktree, patterns, log)` in `src/git/worktreeSetup.ts`, run as the `finish` of
  the operation that creates the worktree, set by `withSetup` in `src/extension/actions.ts`.
- **Entry points**: every way of creating a worktree: `worktree.create`, `worktree.create-for-branch`,
  `worktree.create-at-commit`, `worktree.open-in-worktree`.
- **Offered when**: the settings that apply (`setup.source`) have patterns.
- **Plan**: after `git worktree add`, `> symlink <folder>` for each folder of `setup.folders-to-link`, then `> copy
  <path>` for each path of `setup.paths-to-copy`, each said before it is made.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`, as part of creating the worktree, whose question
  says *Then gitardo will link the folders matching <patterns>, and copy what matches <patterns> from <checkout>, where
  git ignores them.*).
- **Undo**: part of creating the worktree, which nothing takes back; the links and copies go with the worktree.
- **After**: what failed is said once at the end, as *Gitardo: Linking node_modules failed: the log says why.*, or
  *Gitardo: <n> folders or files failed: the log says why.*
- **Settings**: `gitardo.worktrees.linkFolders`, `gitardo.worktrees.copyFiles`, or `git.worktreeSymlinkFolders` and
  `git.worktreeIncludeFiles`.
- **Uses**: `setup.source`, `setup.folders-to-link`, `setup.paths-to-copy`, `log.write`.
- **Implemented in**: `setUpWorktree`, `linkFolder`, `copyPath`, `checkParents` in `src/git/worktreeSetup.ts`;
  `withSetup`, `setupNote`, `setUpWorktrees` in `src/extension/actions.ts`; `Controller.run`, which calls `finish`.
- **Tested by**: `test/integration.test.ts`, "links and copies into a new worktree what git ignores, as the settings of
  VS Code ask"; `test/e2e/suite.ts`, steps "links and copies into a new worktree what the settings of VS Code ask, and
  unlinks before removing it" and "chooses what new worktrees get in a tab of its own, saves it for the workspace
  folder, and brings it into worktrees".
- **Differs**: the question writes the name of the product in lower case, *Then gitardo will…*, as it lowers the first
  letter of the note (`src/extension/actions.ts:807`).

Acceptance:

- [ ] With VS Code's `git.worktreeSymlinkFolders` `["node_modules"]` and `git.worktreeIncludeFiles` `[".env"]` and
  nothing of Gitardo's set, the log has `> git worktree add …`, then `> symlink node_modules`, then `> copy .env`.
- [ ] `node_modules` in the worktree is a link to the one of this checkout (a junction on Windows, which needs no
  rights), and `.env` a copy; `debug.log` is not there; `git status --porcelain` in the worktree prints nothing.
- [ ] Run again on the same worktree, nothing is made over what is there, and the log says *<path> is in the worktree
  already.* for each.
- [ ] A folder that holds the worktree is not linked, and the log says *<folder> holds the worktree, so it is not
  linked.*
- [ ] Nothing is made under a folder of the worktree that is a link (*<folder> is a symbolic link.*) or a file
  (*<folder> is not a folder.*); that one fails, the others are made.
- [ ] A failure is logged as a warning *<what>: <reason>* and does not stop the others, nor the commands for new
  worktrees after them.
- [ ] Copies keep the links they hold as links (`verbatimSymlinks`).
- [ ] The question of the command preview says *gitardo* in lower case (differs).

### `setup.commands` — Run commands in a new worktree

Runs the commands of `gitardo.worktrees.setupCommands`, such as `npm ci`, in a new worktree once its links and copies
are made, in a terminal of this window.

- **Kind**: Operation
- **Function**: `runSetupCommands(root, dir, commands, log)` in `src/extension/worktreeCommands.ts`, started by the
  `finish` that `withSetup` in `src/extension/actions.ts` gives the operation that creates the worktree.
- **Entry points**: every way of creating a worktree: `worktree.create`, `worktree.create-for-branch`,
  `worktree.create-at-commit`, `worktree.open-in-worktree`. Not **Bring ignored files…**.
- **Offered when**: `gitardo.worktrees.setupCommands` has commands, set in the workspace folder (or the workspace file)
  or in the user's settings (`setup.source`).
- **Plan**: `> run <command>` for each, before it starts, one after the other, each as a VS Code task named after the
  folder of the worktree (source *Gitardo*), a shell execution with the worktree as its folder, in a dedicated terminal
  shown without taking the focus. The next starts only when the last ended with exit code 0.
- **Danger**: None. The commands are not asked about on their own: they are said in the question of creating the
  worktree, *Then Gitardo will run <command>, then <command>, in a terminal of this window, in the worktree.* (*After
  that it will run…* after the links and copies), which is asked only when `gitardo.commandPreview` is `all`.
- **Undo**: nothing takes back what the commands did; they are not git operations.
- **After**: the worktree opens, or the message of creating it shows, only once they have all succeeded. When one fails
  or is stopped, the warning *Gitardo: <command> failed with exit code <n> in <name>, so the command after it did not
  run.* (or *did not finish*, or *the <n> commands after it*) offers **Open in Agents Window** (where VS Code has it)
  and **Open in New Window**.
- **Refuses**: in a workspace that is not trusted nothing runs, the log says *The commands of
  gitardo.worktrees.setupCommands did not run: this workspace is not trusted.* and the warning *Gitardo: The commands
  for new worktrees did not run in <name>: this workspace is not trusted.* offers to open it.
- **Settings**: `gitardo.worktrees.setupCommands`.
- **Uses**: `setup.source`, `worktree.open`, `log.write`.
- **Implemented in**: `src/extension/worktreeCommands.ts` (`runSetupCommands`, `runTask`); `withSetup` in
  `src/extension/actions.ts`.
- **Tested by**: `test/e2e/suite.ts`, step "runs the commands for new worktrees in tasks, in order, before opening the
  worktree, and stops at one that fails".

Acceptance:

- [ ] With `git update-ref refs/setup/one HEAD` and `git update-ref refs/setup/two refs/setup/one`, **Open in worktree**
  on `commands/one` logs `> run …` for each, in that order, `refs/setup/two` ends at the commit of `main`, and the
  worktree opens only after both, with no question asked.
- [ ] With `git rev-parse --verify --quiet refs/setup/missing` first, the warning says *… failed with exit code 1 in two,
  so the command after it did not run.*, offers **Open in New Window**, the worktree does not open, and the second
  command does not run.
- [ ] The operation ends, and the views read the repository again, without waiting for the commands; other operations
  can run meanwhile.
- [ ] They run in the shell VS Code runs tasks with: each line is one command, with no `&&` added between them.
- [ ] In a workspace that is not trusted, no task starts, also for commands set in the user's settings.

### `setup.propose-commands` — The commands proposed

The commands the tab proposes for new worktrees, from the lock files the repository commits at its top.

- **Kind**: Derivation
- **Function**: `proposeCommands(files, packageManager)` in `src/git/worktreeSetup.ts`, given by `proposedCommands(root)`
  the files `git ls-files -z -- :(top,literal)<file>…` lists and the `packageManager` of `package.json`.
- **Uses**: by `setup.page`.
- **Implemented in**: `src/git/worktreeSetup.ts` (`JAVASCRIPT`, `OTHERS`, `PROPOSING_FILES`).
- **Tested by**: `test/worktreeSetup.test.ts`, "proposes the command that installs what each lock file locks", "says it
  with --immutable for Yarn 2 and later", "proposes one package manager of JavaScript: the one package.json names, or
  else npm last" and "proposes one command for each kind of project, and one for Python"; `test/integration.test.ts`,
  "proposes commands for new worktrees from the lock files committed at the top, as package.json names its manager".

Rules of the result:

- One command for JavaScript: of the managers whose lock file is committed, the one `packageManager` names, or else the
  first of pnpm (`pnpm-lock.yaml`: `pnpm install --frozen-lockfile`), yarn (`yarn.lock`: `yarn install
  --frozen-lockfile`, or `yarn install --immutable` with a `.yarnrc.yml` or a version 2 or later), bun (`bun.lock`,
  `bun.lockb`: `bun install`), npm (`package-lock.json`, `npm-shrinkwrap.json`: `npm ci`).
- Then one for each other kind, in this order: `uv sync --locked` (`uv.lock`) or else `poetry install`
  (`poetry.lock`), one of the two only; `cargo fetch` (`Cargo.lock`); `go mod download` (`go.mod`); `bundle install`
  (`Gemfile.lock`).
- Only files committed at the top count: one not committed, or in a subfolder, proposes nothing. A
  `requirements.txt` proposes nothing.

Worked examples:

| Committed | `packageManager` | Proposed |
| --- | --- | --- |
| `package-lock.json`, `package.json` | | `npm ci` (from `package-lock.json`) |
| `yarn.lock`, `.yarnrc.yml` | | `yarn install --immutable` |
| `yarn.lock` | `yarn@1.22.22` | `yarn install --frozen-lockfile` |
| `package-lock.json`, `pnpm-lock.yaml` | | `pnpm install --frozen-lockfile` |
| `package-lock.json`, `pnpm-lock.yaml` | `npm@10.8.0` | `npm ci` |
| `package-lock.json` | `pnpm@9.0.0` | `npm ci` |
| `go.mod`, `Gemfile.lock`, `package-lock.json`, `Cargo.lock`, `poetry.lock`, `uv.lock` | | `npm ci`, `uv sync --locked`, `cargo fetch`, `go mod download`, `bundle install` |
| `requirements.txt` | | nothing |
| `.yarnrc.yml` | | nothing |

Acceptance:

- [ ] Each worked example above proposes what is shown.
- [ ] `tools/go.mod` committed, and `Cargo.lock` not committed, propose nothing; once `Cargo.lock` is staged, `cargo
  fetch` is proposed.

### `setup.scan` — What git ignores here

What git ignores in the checkout new worktrees are made from, each with its size and what is proposed for it.

- **Kind**: Read
- **Function**: `scanIgnored(root)` in `src/git/worktreeSetup.ts`.
- **Reads**: `git ls-files --others --ignored --exclude-standard --directory -z` with `GIT_OPTIONAL_LOCKS=0`, and the
  size of each entry on disk, without following links.
- **Uses**: `setup.classify`; by `setup.page`.
- **Implemented in**: `scanIgnored`, `measure` in `src/git/worktreeSetup.ts`; `Controller.sendSetup`, which reads it
  when the tab opens and on **Look again**.
- **Tested by**: `test/integration.test.ts`, "links and copies into a new worktree what git ignores, as the settings of
  VS Code ask".

Acceptance:

- [ ] A folder all of whose content git ignores is one entry (`node_modules`), each other file one entry; a file of a
  folder that is listed too is left out.
- [ ] A folder counts its files and their bytes up to 5000 files, and says it has more past that.
- [ ] A link is not looked into: it counts no file, and is a folder when it leads to one.
- [ ] With `.env`, `config/one.json`, `config/two.json`, `debug.log`, `node_modules/pkg/index.js` and
  `packages/a/node_modules/x.js` ignored, it lists `.env` (copy), `config` (2 files, nothing), `debug.log` (nothing),
  `node_modules` (link, 7 bytes) and `packages` (nothing).

### `setup.classify` — What is proposed for each

The kind of something git ignores, which says what is proposed for it.

- **Kind**: Derivation
- **Function**: `suggest(entry)` in `src/shared/worktreeSetup.ts`.
- **Uses**: by `setup.scan`, `setup.page`.
- **Implemented in**: `src/shared/worktreeSetup.ts` (`LINKED`, `BUILT`, `NOISE`, `LOCAL`, `EDITOR`, `KINDS`).
- **Tested by**: `test/worktreeSetup.test.ts`, "proposes to link dependencies, copy local settings and leave the rest".

Rules of the result, the first that applies:

1. A folder named `node_modules`, `bower_components`, `jspm_packages`, `.venv`, `venv`, `.pnpm-store`, `Pods`,
   `Carthage`, `vendor`, `.gradle` or `.m2`, or at the path `.yarn/cache`: dependencies, linked.
2. A folder named `dist`, `build`, `out`, `target`, `bin`, `obj`, `.next`, `.nuxt`, `.svelte-kit`, `.angular`,
   `.turbo`, `.parcel-cache`, `.cache`, `coverage`, `.nyc_output`, `__pycache__`, `.pytest_cache`, `.mypy_cache`,
   `.ruff_cache`, `.tox`, `.dart_tool`, `DerivedData`, `tmp` or `temp`: build output, nothing.
3. A folder named `worktrees` or `.worktrees`: other worktrees, nothing.
4. `.DS_Store`, `Thumbs.db`, `desktop.ini`, or a name ending in `.log`, `.swp`, `.swo`, `.tmp`, `.bak` or `~`, in any
   case: logs and files the system leaves, nothing.
5. A file named `.env`, `.env.<anything>`, `.envrc`, `.npmrc`, `.yarnrc`, `.yarnrc.yml`, `.pypirc`, `.tool-versions`,
   `local.properties`, `appsettings.Development.json`, `appsettings.Local.json`, `<name>.local` or
   `<name>.local.<ext>`, in any case: settings and secrets of this machine, copied.
6. A folder named `.vscode`, `.idea`, `.fleet` or `.zed`: settings of the editor, copied.
7. Anything else: not one Gitardo knows, nothing.

Worked examples:

| Entry | Kind | Proposed |
| --- | --- | --- |
| folder `packages/web/node_modules` | dependencies | link |
| folder `.yarn/cache` | dependencies | link |
| file `apps/api/.env.development` | local | copy |
| file `config.local.json` | local | copy |
| folder `.vscode` | editor | copy |
| folder `server/__pycache__` | build | nothing |
| folder `.claude/worktrees` | worktrees | nothing |
| file `debug.log` | noise | nothing |
| file `vendor` | unknown | nothing |
| folder `secrets` | unknown | nothing |

Acceptance:

- [ ] Each worked example above is classified as shown.

### `setup.choose` — The patterns a choice writes

How choosing **Link**, **Copy** or **Nothing** for something git ignores changes the patterns, and what each entry
gets under them.

- **Kind**: Derivation
- **Function**: `patternFor`, `namePattern`, `pathPattern`, `choose`, `chooseAll`, `chooseOne`, `groupEntries`,
  `statusOf`, `statusNow` and `statusOfAll` in `src/shared/worktreeSetup.ts`.
- **Uses**: by `setup.page`.
- **Implemented in**: `src/shared/worktreeSetup.ts`.
- **Tested by**: `test/worktreeSetup.test.ts`, "puts what git ignores in sections by kind, with what has the same name
  in several places together", "names dependencies wherever they are, and anything else by its path from the top",
  "says what each one gets from what the patterns take, whole or in part", "says what each one gets as chosen since git
  was last asked, by its own pattern, and otherwise as git said", "chooses by the pattern of each one, leaving the
  others alone", "says what something gets by a pattern of its name, wherever it is" and "chooses for all the places of
  a name by the pattern of the name, and for one by its path".

Rules of the result:

- The pattern of a name is the name, with `/` for a folder (`node_modules/`, `.env.local`); the pattern of a path is
  `/<path>`, with `/` for a folder (`/server/public/`, `/.env`).
- Entries are grouped by kind, in the order unknown, dependencies, local, editor, build, worktrees, noise, and within a
  kind by the pattern of their name.
- A choice for one entry takes its own patterns (of its path and of its name) out of both lists and adds the pattern of
  a folder of dependencies' name, or of anything else's path, to the list chosen; **Nothing** adds none. Patterns that
  take more than it are left.
- A choice for all the places of a name takes the patterns of their paths and names out and adds the pattern of the
  name. A choice for one of them adds the pattern of its path; when the pattern of the name took them all, it gives way
  to one for the path of each of the others, which keep what they had.
- What an entry gets: by a pattern of its own path or name in the patterns as chosen; otherwise, once git was asked,
  nothing when its own pattern was taken out since, or else what git said the patterns take, whole or in part.

Worked examples:

| Patterns before | Choice | Patterns after |
| --- | --- | --- |
| link `vendor/`; copy `node_modules/`, `/.env` | folder `node_modules`: Link | link `vendor/`, `node_modules/`; copy `/.env` |
| same | file `.env`: Nothing | link `vendor/`; copy `node_modules/` |
| copy `.env`, `*.local` | file `.env`: Nothing | copy `*.local` |
| copy `/apps/web/.env.local`, `*.local` | the 3 places of `.env.local`: Copy | copy `*.local`, `.env.local` |
| copy `*.local`, `.env.local` | `packages/ui/.env.local` alone: Nothing | copy `*.local`, `/apps/web/.env.local`, `/.env.local` |
| copy `.env.local`, `/.env.local` | `.env.local` at the top alone: Copy | copy `.env.local` |

Acceptance:

- [ ] Each worked example above gives the patterns shown.
- [ ] With `node_modules` and `packages/web/node_modules` linked, `packages` gets **Link** in part.

### `setup.preview` — What the patterns take

Asks git what the patterns would take for a new worktree, as a whole and pattern by pattern.

- **Kind**: Read
- **Function**: `previewSetup(root, patterns)` in `src/git/worktreeSetup.ts`.
- **Entry points**: the tab of what new worktrees get, when it opens and on **Look again**; the message
  `previewWorktreeSetup`.
- **Uses**: `setup.folders-to-link`, `setup.paths-to-copy`; by `setup.page`.
- **Implemented in**: `previewSetup` in `src/git/worktreeSetup.ts`; case `previewWorktreeSetup` of
  `Controller.onMessage`, which cleans the patterns first.
- **Tested by**: `test/integration.test.ts`, "links and copies into a new worktree what git ignores, as the settings of
  VS Code ask"; `test/e2e/suite.ts`, step "chooses what new worktrees get in a tab of its own, saves it for the
  workspace folder, and brings it into worktrees".

Acceptance:

- [ ] The copies are worked out as for a worktree that has only what this checkout tracks, after the folders linked.
- [ ] For each pattern alone, it counts the folders it links or the paths it copies; a copy pattern counts with all the
  link patterns applied.
- [ ] Link `node_modules`, copy `.env`, `config/`, `node_modules/`, `*.missing` count `node_modules` 2, `.env` 1,
  `config/` 1, `node_modules/` 0, `*.missing` 0.
- [ ] When git fails, the tab says *Git could not say what the patterns take: <error>*.

### `setup.page` — The tab of what new worktrees get

The tab that lists what git ignores here, by kind, with what is proposed for each and what new worktrees get, the
patterns, the commands to run, and where to save them.

- **Kind**: View
- **Function**: `WorktreeSetupApp` in `src/webview/WorktreeSetupApp.tsx`, opened by `Controller.openWorktreeSetup()`
  and fed by `Controller.sendSetup` and `worktreeSetupInfo` in `src/extension/worktreeSettings.ts`.
- **Entry points**: **Gitardo: Choose What New Worktrees Get…** in the Command Palette and in the **…** of the Branches
  view; **Choose what new worktrees get…** in the **…** of the Worktrees section; **Choose What It Gets…** after a
  worktree is created (`setup.offer`); **Choose What New Worktrees Get…** in the messages of the Explorer entries.
- **Offered when**: a repository is shown; one tab at a time, for the repository the views show: a tab open for it is
  brought forward, one open for another repository is closed.
- **Settings**: `gitardo.worktrees.linkFolders`, `gitardo.worktrees.copyFiles`, `gitardo.worktrees.setupCommands`,
  `git.worktreeSymlinkFolders`, `git.worktreeIncludeFiles`: a change to any of them for the repository, elsewhere,
  shows in the tab at once.
- **Uses**: `setup.scan`, `setup.classify`, `setup.choose`, `setup.preview`, `setup.propose-commands`, `setup.source`,
  `setup.save`, `setup.bring-ignored`.
- **Implemented in**: view kind `worktreeSetup` in `src/shared/protocol.ts` and `src/webview/main.tsx`; messages
  `getWorktreeSetup`, `previewWorktreeSetup`, `saveWorktreeSetup`, `editWorktreeSetup` and `worktreeSetup`,
  `worktreeSetupPreview`; `openWorktreeSetup`, `sendSetup` and the cases of `Controller.onMessage` in
  `src/extension/controller.ts`; `editSetupJson` in `src/extension/worktreeSettings.ts`; `gitardo.worktreeSetup` in
  `src/extension/extension.ts`.
- **Tested by**: `test/e2e/suite.ts`, steps "chooses what new worktrees get in a tab of its own, saves it for the
  workspace folder, and brings it into worktrees" and "runs the commands for new worktrees in tasks, in order, before
  opening the worktree, and stops at one that fails".

Acceptance:

- [ ] The tab is titled *New Worktrees · <name of the repository's folder>*; while it reads it says *Looking at what git
  ignores…*, and when it cannot, *Gitardo could not read this repository. <error>*.
- [ ] Its head says what applies now: *Linked now*, *Copied now* and *Run now*, each with the patterns or commands, or
  *nothing*, and *from the settings of <workspace folder>*, *from your user settings*, *from VS Code's
  git.worktreeSymlinkFolders* (or *git.worktreeIncludeFiles*) or *from nothing set*.
- [ ] *What git ignores here* counts the entries; with none it says *Git ignores nothing here, so new worktrees have all
  there is.*
- [ ] The sections are, in this order and where they have entries: *Not one Gitardo knows*, *Dependencies*, *Settings
  and secrets of this machine*, *Settings of the editor*, *Build output*, *Other worktrees*, *Logs and files the system
  leaves*, each with its count, why it gets what is proposed, its size and what its entries get (*all linked*, *2
  copied*, *nothing*).
- [ ] The first four sections start open, the others only when something in them gets something; a section opened or
  closed by hand stays so the next time the tab opens.
- [ ] Each line has **Link**, **Copy** and **Nothing**, the one that applies pressed and the one proposed marked
  *(proposed)* in its tooltip; **Link** is disabled for a file, with *Only folders are linked*.
- [ ] A name in several places is one line, *<n> places*, whose buttons choose for *every <name>, wherever it is*;
  unfolded, it lists each place with its own buttons; it says *Not the same in each place* when they differ.
- [ ] A section, or a name, lists 200 lines, then **Show the other <n>**.
- [ ] Sizes read as *340 MB · 1,234 files*, *over 1.2 GB · 5,000+ files* past 5000 files, *512 B* for a file, *a link*
  for a link.
- [ ] A line notes *A link already, in this checkout*; *Only part of it is linked, by a pattern below* (or *copied*);
  and, for a copy over 100 MB, *Large to copy into each worktree: better linked, or made again*.
- [ ] **Use what is proposed** gives each name what is proposed for it; choosing changes the patterns at once, without
  running git.
- [ ] After a choice, the tab says *Changed since Gitardo last looked: each line shows what is chosen for it, and Look
  again checks what the patterns take as a whole.*; **Look again** (*Looking…* meanwhile) reads again what git ignores
  and asks git what the patterns take.
- [ ] *Folders to link* and *Files and folders to copy* list the patterns, each with what it takes (*2 folders*, *1 file
  or folder*) or *takes nothing here*, a button to remove it, and a box that adds patterns separated by commas;
  *None.* when empty.
- [ ] *Commands to run* lists the commands, numbered, each edited where it is, moved up or down, or removed; commands
  proposed and not listed are buttons that add them at the end, with the lock file they come from; a box adds one.
- [ ] *Save in* offers the workspace folder (tooltip: its `.vscode/settings.json`) and **User**, starting where what
  applies comes from (the user's settings when any of the three comes from there), and **User** alone when the
  repository is in no folder of the workspace.
- [ ] **Discard changes** puts back what applies; **Edit in settings.json** opens the user's `settings.json` at
  `gitardo.worktrees.linkFolders`, or the `.vscode/settings.json` of the workspace folder, made by VS Code when it is not
  there.
- [ ] **Bring into the other worktree** (or *other <n> worktrees*) is disabled while something is changed and not
  saved, while the views show another repository, and while nothing is linked or copied.

### `setup.save` — Save what new worktrees get

Writes the patterns and the commands chosen in the tab to the settings of the workspace folder or of the user, or takes
them out of there.

- **Kind**: Operation
- **Function**: `saveSetup(root, target, patterns, alsoVSCode, commands?)` in `src/extension/worktreeSettings.ts`.
- **Entry points**: **Save** and **Remove from <workspace folder>** or **Remove from User** in the tab of what new
  worktrees get.
- **Offered when**: **Save** while what the tab shows is not what is saved there, or **Also in VS Code's…** is ticked;
  **Remove from** while one of the three settings is set there.
- **Asks**: nothing.
- **Plan**: VS Code's settings API writes `gitardo.worktrees.linkFolders`, `gitardo.worktrees.copyFiles` and
  `gitardo.worktrees.setupCommands` to `.vscode/settings.json` of the workspace folder or to the user's settings; with
  **Also in VS Code's git.worktreeSymlinkFolders and git.worktreeIncludeFiles, for the worktrees VS Code creates**,
  those two as well. **Remove from** writes the three of Gitardo as not set there. No git command runs.
- **Danger**: None, and never asked.
- **Undo**: nothing takes it back: it is not an operation of the repository. **Remove from** takes Gitardo's out again.
- **Settings**: `gitardo.worktrees.linkFolders`, `gitardo.worktrees.copyFiles`, `gitardo.worktrees.setupCommands`,
  `git.worktreeSymlinkFolders`, `git.worktreeIncludeFiles`.
- **Uses**: `setup.source`.
- **Implemented in**: message `saveWorktreeSetup` in `src/shared/protocol.ts`; its case in `Controller.onMessage`
  (`src/extension/controller.ts`); `saveSetup` in `src/extension/worktreeSettings.ts`.
- **Tested by**: `test/e2e/suite.ts`, steps "chooses what new worktrees get in a tab of its own, saves it for the
  workspace folder, and brings it into worktrees" and "runs the commands for new worktrees in tasks, in order, before
  opening the worktree, and stops at one that fails".
- **Differs**: against promise 1, it writes a file without going through `Controller.run`: it is not shown first and
  nothing of it is written to the log (`src/extension/controller.ts:2260`).

Acceptance:

- [ ] Saving link `node_modules/` and copy `/.env`, `*.missing` for the workspace folder writes them as its
  `gitardo.worktrees.linkFolders` and `gitardo.worktrees.copyFiles`, and the tab then says they come from the
  workspace folder.
- [ ] Commands `git update-ref …` twice and `' '` are saved as the two commands, without the empty one.
- [ ] An empty list of commands is written only where one is set somewhere already.
- [ ] VS Code's two settings change only with the box ticked, and then get the same patterns.
- [ ] **Remove from** the workspace folder leaves the three of Gitardo not set there, and what is set elsewhere applies
  again.
- [ ] When writing fails, *Gitardo: the settings were not saved. <error>* is said.
- [ ] The log says what was written (differs).

### `setup.offer` — The offer to choose what new worktrees get

Says, once, after a worktree is created while new worktrees get nothing, that what they get can be chosen.

- **Kind**: View
- **Function**: the `onSuccess` of `addWorktree` in `src/extension/actions.ts`, with `SETUP_OFFERED`.
- **Entry points**: **Choose What It Gets…** in the message *Worktree created in <folder>*.
- **Offered when**: no pattern to link or copy and no command applies, and it was not offered before in this workspace
  and the tab was not opened before; only after `worktree.create`, `worktree.create-for-branch` and
  `worktree.create-at-commit`, not after `worktree.open-in-worktree`.
- **Uses**: `setup.page`, `setup.source`.
- **Implemented in**: `addWorktree` and `SETUP_OFFERED` in `src/extension/actions.ts`; `openWorktreeSetup` in
  `src/extension/controller.ts`, which marks it offered too. It is kept in the workspace state.
- **Tested by**: `test/e2e/suite.ts`, steps "creates a worktree by name, renames it with its branch and removes both"
  and "chooses what new worktrees get in a tab of its own, saves it for the workspace folder, and brings it into
  worktrees".

Acceptance:

- [ ] The first worktree created with nothing set offers **Open in Agents Window** (where VS Code has it), **Open in New
  Window** and **Choose What It Gets…**, which opens the tab.
- [ ] The next one created in the same workspace no longer offers it, whatever was chosen.
- [ ] With patterns saved, it is not offered.

### `setup.bring-ignored` — Bring ignored files into worktrees that exist

Gives worktrees that are there already what new ones get, without touching what they have.

- **Kind**: Operation
- **Function**: `setUpExistingOp(state, dir?)` in `src/extension/actions.ts`, whose `finish` runs `setUpWorktree` for
  each worktree.
- **Entry points**: **Bring ignored files…** in the context menu of a worktree; **Bring ignored files into the other
  worktrees…** in the **…** of the Worktrees section; **Bring into the other worktree** (or *other <n> worktrees*) in
  the tab of what new worktrees get.
- **Offered when**: **Bring ignored files…** for a worktree that is not missing, not the one the views show and not the
  folder of the window; the others while some worktree is neither missing nor current.
- **Asks**: always, as a question: *Bring ignored files into <name of the folder>?*, or *…into <n> worktrees?*, with
  *Gitardo will link the folders matching <patterns>, and copy what matches <patterns> from <checkout>, where git
  ignores them.*
- **Plan**: no git command; `> symlink <folder>` and `> copy <path>` for each worktree, from the checkout the views
  show, as in `setup.link-and-copy`. The commands for new worktrees do not run.
- **Danger**: Confirm, because files are made in other worktrees.
- **Undo**: nothing takes it back: it is not in the reflog and Gitardo remembers nothing for it; what it made can be
  deleted by hand.
- **After**: what failed is said once at the end, as in `setup.link-and-copy`.
- **Refuses**: with nothing to link or copy: *New worktrees get nothing yet: choose what they get first.*; with no
  other worktree: *There are no other worktrees to bring it into.*; into the checkout itself: *This is the checkout it
  comes from.*; a folder that is no longer a worktree: *<folder> is no longer a worktree of this repository.*
- **Settings**: `gitardo.worktrees.linkFolders`, `gitardo.worktrees.copyFiles`, `git.worktreeSymlinkFolders`,
  `git.worktreeIncludeFiles`.
- **Uses**: `setup.source`, `setup.link-and-copy`, `preview.ask`, `log.write`, `activity.track`.
- **Implemented in**: `Action` `{ kind: 'worktreeSetUp'; path? }` in `src/shared/protocol.ts`; case `worktreeSetUp` of
  `resolveAction` and `setUpExistingOp` in `src/extension/actions.ts`; `worktreeMenu`, `worktreesMenu` in
  `src/webview/menus.ts`; the button of `WorktreeSetupApp`.
- **Tested by**: `test/e2e/suite.ts`, step "chooses what new worktrees get in a tab of its own, saves it for the
  workspace folder, and brings it into worktrees".
- **Differs**: Undo, as for `worktree.create` (`src/extension/controller.ts:1689`).

Acceptance:

- [ ] With `.env` deleted from a worktree that had `node_modules` linked and `.env` copied, it asks, then logs `> copy
  .env` and *node_modules is in the worktree already.*, and `.env` is back with the content of this checkout.
- [ ] Into all, it goes into every worktree that is not missing and is not the checkout the views show, the main
  worktree included when the views show another worktree.
- [ ] No git command is run and none is shown in the question.
- [ ] **Undo** right after does not take out what it made (differs).

### `setup.explorer-link` — Link into new worktrees from the Explorer

From the Explorer, has new worktrees get a folder linked, by the pattern of its path.

- **Kind**: Operation
- **Function**: `Controller.setUpFromExplorer('link', uris)` in `src/extension/controller.ts`, with `changeSetup` in
  `src/git/worktreeSetup.ts` and `savePatterns` in `src/extension/worktreeSettings.ts`.
- **Entry points**: **Link into New Worktrees** in the context menu of a folder in the Explorer, for the folders
  selected; not in the Command Palette.
- **Offered when**: the folder is on disk, the workspace has a repository, and no pattern of its own path links it
  (else **Stop Linking into New Worktrees** is there instead).
- **Reads**: what the patterns take before and after, as `setup.explorer-change` says, with VS Code's progress
  *Gitardo: looking at what git ignores* in the status bar.
- **Plan**: the pattern of its path, `/<path>/`, is added to the folders to link, and taken out of the files to copy;
  the setting is saved where it is set now, the workspace folder or the user's settings; where it is not set, in the
  workspace folder (the user's when the repository is in no folder of the workspace), starting from VS Code's patterns
  when they apply. VS Code's own settings are not changed. No git command changes anything.
- **Danger**: None, and never asked.
- **Undo**: nothing takes it back; **Stop Linking into New Worktrees** does.
- **After**: one message for all that was chosen, *Gitardo: <what became of each> Saved in the settings of the
  workspace folder.* (or *of the user*), as information when something changed and as a warning when nothing did, with
  **Choose What New Worktrees Get…**, which shows the repository in the views and opens the tab.
- **Refuses**: what is in no repository of the workspace: *<name> is in no repository of the workspace.*; the folder of
  a repository itself counts as in none.
- **Settings**: `gitardo.worktrees.linkFolders`, falling back to `git.worktreeSymlinkFolders`.
- **Uses**: `setup.explorer-change`, `setup.source`.
- **Implemented in**: `gitardo.linkIntoWorktrees` in `package.json` (`explorer/context`, group
  `z_gitardo_worktrees@1`, when `explorerResourceIsFolder` and the path is not in `gitardo.linkedIntoWorktrees`) and in
  `src/extension/extension.ts`; `setUpFromExplorer` in `src/extension/controller.ts`; `showSetupInMenus` in
  `src/extension/worktreeSettings.ts`.
- **Tested by**: `test/e2e/ui.mjs`, step "the context menu of the Explorer offers the history of a folder but not of a
  repository, links a folder git ignores into new worktrees, stops it, and says why a file git tracks is not copied";
  `test/integration.test.ts`, "links and copies from the Explorer only what git ignores, and says why not".
- **Differs**: against promise 1, it writes the settings without going through `Controller.run`, and nothing of it is
  written to the log (`src/extension/controller.ts:1080`).

Acceptance:

- [ ] On `node_modules`, which git ignores, the workspace folder's `gitardo.worktrees.linkFolders` becomes
  `["/node_modules/"]`, the message says *node_modules will be linked into new worktrees.*, and the menu of
  `node_modules` then has **Stop Linking into New Worktrees** instead.
- [ ] On `.vscode`, of which git tracks `settings.json`, nothing is saved and the message, a warning, says *Git tracks
  files in .vscode: each worktree has them from its commit.*
- [ ] On `packages/a/node_modules` while `node_modules/` is linked, nothing is saved and it says
  *packages/a/node_modules is linked into new worktrees already, by the pattern node_modules/.*
- [ ] On a folder copied by its path, `/node_modules/` moves from the copies to the links.
- [ ] The log says what was written (differs).

### `setup.explorer-copy` — Copy into new worktrees from the Explorer

From the Explorer, has new worktrees get a file or folder copied, by the pattern of its path.

- **Kind**: Operation
- **Function**: `Controller.setUpFromExplorer('copy', uris)` in `src/extension/controller.ts`, with `changeSetup` and
  `savePatterns`.
- **Entry points**: **Copy into New Worktrees** in the context menu of a file or a folder in the Explorer, for those
  selected; not in the Command Palette.
- **Offered when**: it is on disk, the workspace has a repository, and no pattern of its own path copies it (else
  **Stop Copying into New Worktrees** is there instead).
- **Reads**: as `setup.explorer-link`.
- **Plan**: the pattern of its path, `/<path>` or `/<path>/`, is added to the files to copy and taken out of the
  folders to link; saved as `setup.explorer-link` saves. No git command changes anything.
- **Danger**: None, and never asked.
- **Undo**: nothing takes it back; **Stop Copying into New Worktrees** does.
- **After**: as `setup.explorer-link`.
- **Refuses**: as `setup.explorer-link`.
- **Settings**: `gitardo.worktrees.copyFiles`, falling back to `git.worktreeIncludeFiles`.
- **Uses**: `setup.explorer-change`, `setup.source`.
- **Implemented in**: `gitardo.copyIntoWorktrees` in `package.json` (`explorer/context`, group
  `z_gitardo_worktrees@2`, when the path is not in `gitardo.copiedIntoWorktrees`) and in `src/extension/extension.ts`;
  `setUpFromExplorer` in `src/extension/controller.ts`.
- **Tested by**: `test/e2e/ui.mjs`, step "the context menu of the Explorer offers the history of a folder but not of a
  repository, links a folder git ignores into new worktrees, stops it, and says why a file git tracks is not copied";
  `test/integration.test.ts`, "links and copies from the Explorer only what git ignores, and says why not".
- **Differs**: against promise 1, as for `setup.explorer-link` (`src/extension/controller.ts:1080`).

Acceptance:

- [ ] On `.env`, which git ignores, `gitardo.worktrees.copyFiles` gets `/.env` and the message says *.env will be
  copied into new worktrees.*
- [ ] On `a.txt`, which git tracks, nothing is saved and the warning says *Git tracks a.txt: each worktree has it from
  its commit.*
- [ ] On `.vscode`, of which git ignores all but `settings.json`, `/.vscode/` is saved and the message says *What git
  ignores in .vscode will be copied into new worktrees.*
- [ ] On `notes.txt`, which git does not ignore, nothing is saved and it says *Git does not ignore notes.txt: only what
  it ignores is linked or copied into new worktrees.*
- [ ] On `packages/a/node_modules/x.js` while `node_modules/` is linked, nothing is saved and it says
  *packages/a/node_modules/x.js is in packages/a/node_modules, which the pattern node_modules/ links into new worktrees:
  nothing in it is copied.*
- [ ] The log says what was written (differs).

### `setup.explorer-stop` — Stop linking or copying into new worktrees

From the Explorer, takes out the pattern of the path of a file or folder, so that new worktrees no longer get it by it.

- **Kind**: Operation
- **Function**: `Controller.setUpFromExplorer('none', uris)` in `src/extension/controller.ts`, with `changeSetup` and
  `savePatterns`.
- **Entry points**: **Stop Linking into New Worktrees** and **Stop Copying into New Worktrees** in the context menu of
  the Explorer; not in the Command Palette.
- **Offered when**: a pattern of its own path, `/<path>` or `/<path>/`, links it, or copies it, in the settings that
  apply to a repository of the workspace folders, VS Code's included when they apply.
- **Plan**: the patterns `/<path>` and `/<path>/` are taken out of both lists; patterns of its name, such as
  `node_modules/`, are left; saved as `setup.explorer-link` saves. Both entries do the same.
- **Danger**: None, and never asked.
- **Undo**: nothing takes it back; **Link into New Worktrees** or **Copy into New Worktrees** does.
- **After**: as `setup.explorer-link`: *New worktrees will no longer get <path>.*, or, when another pattern still gives
  it something, *<path> is still linked into new worktrees, by the pattern <pattern>.*
- **Settings**: `gitardo.worktrees.linkFolders`, `gitardo.worktrees.copyFiles`.
- **Uses**: `setup.explorer-change`, `setup.source`.
- **Implemented in**: `gitardo.stopLinkingIntoWorktrees` and `gitardo.stopCopyingIntoWorktrees` in `package.json` and
  `src/extension/extension.ts`; `showSetupInMenus` and `namedPaths`, which set the context keys
  `gitardo.linkedIntoWorktrees` and `gitardo.copiedIntoWorktrees` when the workspace folders or the four settings
  change.
- **Tested by**: `test/e2e/ui.mjs`, step "the context menu of the Explorer offers the history of a folder but not of a
  repository, links a folder git ignores into new worktrees, stops it, and says why a file git tracks is not copied";
  `test/worktreeSetup.test.ts`, "chooses from the Explorer by the pattern of the path, leaving the patterns of names
  alone"; `test/integration.test.ts`, "links and copies from the Explorer only what git ignores, and says why not".
- **Differs**: against promise 1, as for `setup.explorer-link` (`src/extension/controller.ts:1080`).

Acceptance:

- [ ] After **Link into New Worktrees** on `node_modules`, **Stop Linking into New Worktrees** leaves
  `gitardo.worktrees.linkFolders` an empty list in the workspace folder, and the menu offers **Link into New
  Worktrees** again.
- [ ] With link `node_modules/` and `/node_modules/`, stopping `node_modules` leaves `node_modules/` and says
  *node_modules is still linked into new worktrees, by the pattern node_modules/.*
- [ ] Only patterns that name one path, starting with `/` and without `*`, `?`, `[`, `\` or `!`, offer a Stop entry:
  of `/.env`, `/packages/api/vendor/`, `node_modules/`, `apps/*/.env`, `/build/**`, `!/keep`, `/a[b]`, only `.env` and
  `packages/api/vendor`.
- [ ] The log says what was written (differs).

### `setup.explorer-change` — What became of what was chosen in the Explorer

What choosing link, copy or nothing for a file or folder from the Explorer does to the patterns, and what is said of
it.

- **Kind**: Derivation
- **Function**: `changeSetup(root, entry, choice, patterns)` in `src/git/worktreeSetup.ts`, with `chooseByPath` in
  `src/shared/worktreeSetup.ts` and `setupChangeText` in `src/extension/worktreeSettings.ts`.
- **Reads**: `setup.folders-to-link` and `setup.paths-to-copy` before and after; when they do not give it what was
  chosen, `git ls-files -z -- :(literal)<path>` and `git ls-files --others --ignored --exclude-standard -z --
  :(literal)<path>`.
- **Uses**: by `setup.explorer-link`, `setup.explorer-copy`, `setup.explorer-stop`.
- **Implemented in**: `src/git/worktreeSetup.ts`, `src/shared/worktreeSetup.ts`, `src/extension/worktreeSettings.ts`.
- **Tested by**: `test/integration.test.ts`, "links and copies from the Explorer only what git ignores, and says why
  not"; `test/worktreeSetup.test.ts`, "chooses from the Explorer by the pattern of the path, leaving the patterns of
  names alone".

Rules of the result:

- Nothing: the patterns of its path go; *done* when nothing gives it anything any more, or only part of it; else
  *still*, with the first pattern that gives it something.
- Link or copy: when it gets that already, whole, the patterns stay, *already*, with the pattern that gives it.
  Otherwise the pattern of its path goes to the list chosen: *done* when it then gets that whole, or, to copy, in part.
- To copy what a folder linked holds: the patterns stay, *linked*, with the folder and its pattern.
- Otherwise the patterns stay: *tracked* when git tracks it (to link, or a file, or a folder git ignores nothing of);
  else *not ignored*, in part when git ignores some of it.

Worked examples (`.gitignore` `node_modules/`, `.env`, `.vscode/*`, `!.vscode/settings.json`):

| Patterns | Entry, choice | Result | Said |
| --- | --- | --- | --- |
| none | folder `node_modules`, link | link `/node_modules/` | *node_modules will be linked into new worktrees.* |
| none | file `.env`, copy | copy `/.env` | *.env will be copied into new worktrees.* |
| none | folder `.vscode`, copy | copy `/.vscode/` | *What git ignores in .vscode will be copied into new worktrees.* |
| none | folder `.vscode`, link | unchanged | *Git tracks files in .vscode: each worktree has them from its commit.* |
| none | file `a.txt`, copy | unchanged | *Git tracks a.txt: each worktree has it from its commit.* |
| none | file `notes.txt`, copy | unchanged | *Git does not ignore notes.txt: …* |
| none | folder `packages`, link | unchanged | *Git ignores only part of packages: only what it ignores is linked or copied into new worktrees.* |
| link `node_modules/` | folder `packages/a/node_modules`, link | unchanged | *… is linked into new worktrees already, by the pattern node_modules/.* |
| link `node_modules/` | file `packages/a/node_modules/x.js`, copy | unchanged | *… is in packages/a/node_modules, which the pattern node_modules/ links …* |
| link `node_modules/`, `/node_modules/` | folder `node_modules`, nothing | link `node_modules/` | *node_modules is still linked into new worktrees, by the pattern node_modules/.* |
| link `/node_modules/` | folder `node_modules`, copy | copy `/node_modules/` | *node_modules will be copied into new worktrees.* |

Acceptance:

- [ ] Each worked example above gives the result and the text shown.
- [ ] A folder linked by a pattern of its own name, asked to be copied, says *<name> is linked into new worktrees by
  the pattern <pattern>, which comes before copying.*
