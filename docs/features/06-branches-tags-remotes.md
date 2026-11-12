# Branches, tags, remotes and sync

Checking out, creating, renaming and deleting branches, the remote branches and the tags; the branch cleanup, the
favorites and the order branches are listed in; the remotes of the repository; and fetching, pulling, pushing and
fast-forwarding a branch that is not checked out. Each operation here is a `GitOp` built in `src/git/ops.ts` and run by
`Controller.run`, shown first when it can lose work and logged command by command. What is offered where is decided in
`src/webview/menus.ts`; the sync operations come from the toolbar of the graph, the titles of the Commits and Gitardo
Graph views and the Command Palette, and go through `Controller.sync` rather than `Controller.action`.

### `branch.checkout` — Check out a branch

Switches the working tree to a local branch, from its context menu or with a double click where it is listed or on its
label in the graph.

- **Kind**: Operation
- **Function**: `checkoutOp('branch', ref)` in `src/git/ops.ts`.
- **Entry points**: **Check out `<branch>`** in the context menu of a local branch, in the references tree, in the
  Branches view and on its label in the graph; a double click on the branch in those places, in the **Recent** and
  **Favorites** sections too (`openBranch` in `src/webview/menus.ts`).
- **Offered when**: the branch is not checked out here and no other worktree has it. For a branch another worktree has,
  the menu offers that worktree's entries instead and a double click opens it (`worktree.open`); when its folder is
  missing, a double click does nothing. With `gitardo.branches.openIn` set to `worktree`, **Open in worktree** comes
  first and a double click opens the branch in a worktree made for it instead (`worktree.open`).
- **Plan**: `> git checkout <branch>`, by its short name: git takes the branch over a tag of the same name. Then, when
  the repository has submodules here or at the branch, `> git submodule update --init --recursive`
  (`submodule.update-on-checkout`).
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`); Confirm when updating the submodules would leave
  commits that no branch has (`submodule.update-on-checkout`).
- **Undo**: from the reflog: its entry `checkout: moving from <previous> to <branch>` becomes `git checkout <previous>`,
  or `git checkout --detach <hash>` when `HEAD` was detached, asked as **Undo checkout: go back to `<previous>`?**, and
  followed by the update of the submodules (`undo.last`).
- **Refuses**: git refuses when files not committed would be overwritten: **Check out `<branch>` failed.** and git's
  `error:` line, with **Show Log**.
- **Settings**: `gitardo.branches.openIn`, `gitardo.submodules.updateOnCheckout`, `gitardo.commandPreview`.
- **Uses**: `preview.ask`, `log.write`, `submodule.update-on-checkout`, `undo.last`.
- **Implemented in**: `Action` case `checkout` with `target: 'branch'` (`src/shared/protocol.ts:493`); `resolveAction`
  case `checkout` and `updatingSubmodules` (`src/extension/actions.ts:233`, `:883`); `checkoutOp`
  (`src/git/ops.ts:195`); `branchMenu` and `openBranch` (`src/webview/menus.ts:301`, `:367`); `RefsTree.tsx` and the
  label menus of `App.tsx`.
- **Tested by**: `test/git.test.ts`, *checks out branches, remote branches and detached commits differently*, *refuses
  names that git could read as options*; `test/awkward.test.ts`, *means the branch when a tag has the same name*;
  `test/menus.test.ts`, *checks out a branch without a worktree, or opens it in one made for it, first as a double
  click does*, *opens the worktree of a branch instead of checking it out, and removes it instead of deleting the
  branch*, *checks out a branch on a double click, or opens it in a worktree, and does nothing for the one checked out*;
  `test/integration.test.ts`, *undoes commits, checkouts, resets and rebases*; `test/e2e/suite.ts`, *updates the
  submodules after a checkout, cloning from a folder on disk once allowed, unless the settings say not to*.

Acceptance:

- [ ] **Check out feature** on the branch `feature` runs `git checkout feature`, through `Controller.run`, and the log
      has `> git checkout feature`.
- [ ] With `gitardo.commandPreview` `off` and no submodules, nothing is asked; with `all`, **Check out feature?** shows
      `git checkout feature` before it runs.
- [ ] With a tag also called `release`, checking out the branch `release` leaves `HEAD` on the branch, not detached at
      the tag.
- [ ] A branch named `-x` is refused with **"-x" is not a valid name.** and git is not run.
- [ ] The menu of the branch checked out has no **Check out** entry, and a double click on it does nothing.
- [ ] The menu of a branch another worktree has offers **Open worktree in new window** and no **Check out** entry; a
      double click opens that worktree.
- [ ] With submodules, the question and the log have `git submodule update --init --recursive` after the checkout.
- [ ] **Undo** right after it asks **Undo checkout: go back to main?** and runs `git checkout main`.

### `branch.checkout-remote` — Check out a remote branch in the local branch of its name

Starts working on a remote branch: in a new local branch that tracks it, or in the local branch of its name when there
is one, brought up to it by a fast-forward, or with its own commits rebased onto it if the user chooses.

- **Kind**: Operation
- **Function**: `checkoutRemote(ref, rebase, state, repo)` in `src/extension/actions.ts`, which builds
  `checkoutOp('remote', ref)` or `checkoutLocalOp(remote, local, rebase)` of `src/git/ops.ts`.
- **Entry points**: in the context menu of a remote branch, **Check out as local branch** when no local branch has its
  name, **Check out `<branch>`** when one has, and **Check out `<branch>` and rebase it onto this**; a double click on
  the remote branch in the references tree, the Branches view or on its label in the graph (`openRemoteBranch`).
- **Offered when**: no other worktree has the local branch of its name; when one has, the menu offers that worktree's
  entries and a double click opens it (`worktree.open`). **Check out `<branch>` and rebase it onto this** is offered
  when the local branch follows this remote branch, is not checked out, and each has commits the other has not, as of
  the last fetch. With `gitardo.branches.openIn` set to `worktree`, **Open in worktree** comes first and a double click
  opens it in a worktree instead.
- **Asks**: when the local branch and the remote branch each have commits the other has not and the rebase was not
  chosen from the menu, a picker titled **`<branch>` has `<n>` commits that `<remote branch>` has not, which has `<m>`
  commits of its own** (**1 commit** when one), with **Check out `<branch>` and rebase it onto `<remote branch>`**
  (**Replay its `<n>` commits on top**) and **Check out `<branch>` as it is** (**Leave the two apart**).
- **Reads**: `git rev-list --left-right --count refs/heads/<branch>...<remote branch> --`, for how the two stand.
- **Plan**:
  - no local branch of its name: `> git checkout --track <remote branch>`;
  - a local branch with nothing of its own and behind: `> git checkout <branch>` then
    `> git merge --ff-only <remote branch>` (only the merge when it is checked out already), titled **Check out
    `<branch>` and fast-forward it to `<remote branch>`**;
  - a local branch not behind, or kept as it is: `> git checkout <branch>`;
  - rebased: `> git rebase --autostash <remote branch> <branch>`, titled **Check out `<branch>` and rebase it onto
    `<remote branch>`**; git checks the branch out first and sets aside what is not committed meanwhile.
  - then, with submodules, `> git submodule update --init --recursive` (`submodule.update-on-checkout`).
  The remote branch is named `refs/remotes/<remote branch>` when a branch or a tag shares its name; the local branch is
  always given by its short name, as git would otherwise check out its commit detached.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`); Confirm when the submodules would leave commits
  no branch has.
- **Undo**: from the reflog (`undo.last`). After `--track` or a plain checkout, **Undo** goes back to the branch checked
  out before; the new local branch stays. After the fast-forward, **Undo** moves the branch back to where it was, with
  `git reset --keep <previous>`, and stays on it. After the rebase, **Undo** moves the branch back to its commits before
  the rebase, with `git reset --keep <previous>`.
- **After**: when the local branch is checked out already and is not behind, nothing runs and **`<branch>` is checked
  out already, with everything `<remote branch>` has.** is said. When the user keeps a branch that is checked out
  already as it is, nothing runs.
- **Refuses**: when the worktree that has the branch is missing: **The worktree of `<branch>` is missing from `<path>`:
  prune the worktrees to free the branch.** When another worktree has it, that worktree opens instead and nothing runs.
- **Settings**: `gitardo.branches.openIn`, `gitardo.submodules.updateOnCheckout`.
- **Uses**: `preview.ask`, `log.write`, `submodule.update-on-checkout`, `worktree.open`, `undo.last`.
- **Implemented in**: `Action` case `checkout` with `target: 'remote'` and `rebase` (`src/shared/protocol.ts:493`);
  `resolveAction` case `checkout`, `checkoutRemote` (`src/extension/actions.ts:233`, `:691`); `checkoutOp`,
  `checkoutLocalOp`, `withoutRemote` (`src/git/ops.ts:195`, `:213`, `:744`); `remoteBranchMenu` and `openRemoteBranch`
  (`src/webview/menus.ts:387`, `:380`).
- **Tested by**: `test/git.test.ts`, *checks out branches, remote branches and detached commits differently*, *checks
  out a remote branch in the local branch of its name, brought up to it*; `test/integration.test.ts`, *checks out a
  remote branch in the local branch of its name, fast-forwarded to it or with its own commits rebased onto it*;
  `test/menus.test.ts`, *checks out the local branch of its name, and offers to rebase it when each has commits the
  other has not*, *checks out a branch on a double click, or opens it in a worktree, and does nothing for the one
  checked out*, *opens the worktree that has the local branch of its name, instead of checking it out*. The picker of
  `checkoutRemote`: *none*.

Acceptance:

- [ ] With no local `dev`, **Check out as local branch** on `origin/dev` runs `git checkout --track origin/dev`.
- [ ] With a local `dev` 2 commits behind `origin/dev` and none of its own, **Check out dev** runs `git checkout dev`
      and `git merge --ff-only origin/dev`; what is not committed is still in the files afterwards.
- [ ] The same with `dev` checked out already runs only `git merge --ff-only origin/dev`.
- [ ] With `dev` checked out and nothing on `origin/dev` it lacks, nothing runs and **dev is checked out already, with
      everything origin/dev has.** is said.
- [ ] With `dev` 1 ahead and 2 behind, a double click on `origin/dev` asks **dev has 1 commit that origin/dev has not,
      which has 2 commits of its own**; **Check out dev as it is** runs `git checkout dev` alone.
- [ ] **Check out dev and rebase it onto this** runs `git rebase --autostash origin/dev dev` without the picker, and
      afterwards `dev` has its own commit on top of the 2 of `origin/dev`.
- [ ] Each of these runs as one `GitOp` through `Controller.run`, and the log has its commands as shown.
- [ ] With another worktree on `dev`, the menu of `origin/dev` has no **Check out** entry and a double click opens the
      worktree; with that worktree's folder gone, neither is offered, and the action asked anyway refuses with **The
      worktree of dev is missing from …** before git runs.
- [ ] With a tag also called `origin/dev`, the commands name `refs/remotes/origin/dev`.
- [ ] **Undo** right after a fast-forward moves `dev` back to its commit before it, and leaves `dev` checked out.

### `branch.checkout-detached` — Check out a commit or a tag, detached

Puts the working tree on a commit or a tag without a branch, to look at it or build it.

- **Kind**: Operation
- **Function**: `checkoutOp('detached', ref)` in `src/git/ops.ts`.
- **Entry points**: **Check out commit (detached)** in the context menu of a commit in the graph and in the Commits
  view; **Check out tag (detached)** in the context menu of a tag; **Check out (detached)** for a position picked in
  **Gitardo: Browse Reflog**.
- **Offered when**: the commit, or the commit of the tag, is not the one checked out.
- **Plan**: `> git checkout --detach <hash>`, or `> git checkout --detach <tag>`, the tag as `refs/tags/<tag>` when a
  branch or a remote branch shares its name; then, with submodules, `> git submodule update --init --recursive`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`); Confirm when the submodules would leave commits
  no branch has.
- **Undo**: from the reflog: `checkout: moving from <previous> to <hash or tag>` becomes `git checkout <previous>`,
  asked as **Undo checkout: go back to `<previous>`?** (`undo.last`).
- **Refuses**: git refuses when files not committed would be overwritten, as for `branch.checkout`.
- **Settings**: `gitardo.submodules.updateOnCheckout`.
- **Uses**: `preview.ask`, `log.write`, `submodule.update-on-checkout`, `undo.last`.
- **Implemented in**: `Action` case `checkout` with `target: 'detached'` (`src/shared/protocol.ts:493`);
  `resolveAction` case `checkout` (`src/extension/actions.ts:233`); `checkoutOp` (`src/git/ops.ts:195`);
  `commitMenu`, `logMenu`, `tagMenu` (`src/webview/menus.ts:187`, `:249`, `:433`); `Controller.browseReflog`.
- **Tested by**: `test/git.test.ts`, *checks out branches, remote branches and detached commits differently*;
  `test/awkward.test.ts`, *means the branch when a tag has the same name*.

Acceptance:

- [ ] **Check out commit (detached)** on a commit runs `git checkout --detach <full hash>` and the toolbar then says
      **detached at `<short hash>`**.
- [ ] **Check out tag (detached)** on `v1` runs `git checkout --detach v1`; with a branch also called `v1`, it runs
      `git checkout --detach refs/tags/v1` and `HEAD` is at the tag's commit.
- [ ] Neither entry is offered on the commit checked out.
- [ ] **Undo** right after it runs `git checkout <branch before>` after asking.

### `branch.create` — Create a branch

Makes a new branch at the commit checked out, at a branch, a tag or a commit, and checks it out or not, as chosen.

- **Kind**: Operation
- **Function**: `createBranchOp(name, at, checkout)` in `src/git/ops.ts`; the name comes from `newBranchName(at, state,
  repo, memory)` in `src/extension/actions.ts`.
- **Entry points**: **Branch** in the toolbar of the graph (tooltip **Branch: create a branch at HEAD**),
  **Gitardo: Create Branch…** in the Command Palette and on the title of the Branches view, all at `HEAD`; **Create
  branch here…** in the context menu of a commit (graph and Commits view), a local branch and a tag; **Create branch
  here…** for a position picked in **Gitardo: Browse Reflog**.
- **Offered when**: always in the menus; the toolbar button is disabled before the first commit and while an operation
  runs.
- **Asks**:
  1. With `gitardo.branches.format` set (or the older `gitardo.worktrees.branchFormat`), a picker titled **Create
     branch**: **Use the format** (the format as its description) or **Type a branch name**. Then, for the format, what
     `branch.name-format` asks, and an input box titled **Create branch** with the prompt **Name of the new branch** and
     the name the format made, to change.
  2. Without a format, or with **Type a branch name**: an input box titled **Create branch**, placeholder
     **feature/my-change**.
  3. A picker titled **Create branch `<name>`**: **Create and check out**, or **Create only** (**Stay on the current
     branch**).
  The name is checked as it is typed: **Enter a name**, **That is not a valid Git ref name** (a name starting with `-`,
  `.` or `/`, ending with `/`, `.` or `.lock`, or with a space, `~`, `^`, `:`, `?`, `*`, `[`, `\`, `..` or `@{`), **A
  branch with that name already exists**. Escape at any step does nothing.
- **Plan**: `> git checkout -b <name> <at>` to check it out, `> git branch <name> <at>` otherwise. `<at>` is `HEAD`,
  the commit's hash, or the branch or tag, named `refs/heads/<name>` or `refs/tags/<name>` when something else shares
  its name. Checked out, with submodules, then `> git submodule update --init --recursive`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`); Confirm when checking it out would leave
  commits in the submodules that no branch has.
- **Undo**: checked out: from the reflog, **Undo** goes back to the branch checked out before; the new branch stays.
  Created only: nothing moves `HEAD`, so **Undo** right after it takes back the operation before it, as it says in its
  question; the new branch stays.
- **Refuses**: a name that starts with `-` is refused twice, by the input box and by the builder (**"-D" is not a valid
  name.**). An invalid pattern in an input of the format: see `branch.name-format`.
- **Settings**: `gitardo.branches.format`, `gitardo.branches.inputs` (or, where they are not set,
  `gitardo.worktrees.branchFormat` and `gitardo.worktrees.inputs`), `gitardo.submodules.updateOnCheckout`.
- **Uses**: `branch.name-format`, `preview.ask`, `log.write`, `submodule.update-on-checkout`, `undo.last`.
- **Implemented in**: `Action` case `createBranch` (`src/shared/protocol.ts:494`); `resolveAction` case `createBranch`,
  `newBranchName`, `askName`, `nameProblem` (`src/extension/actions.ts:238`, `:1286`, `:98`, `:93`); `createBranchOp`,
  `refNameProblem` (`src/git/ops.ts:231`, `:82`); the toolbar button (`src/webview/Toolbar.tsx:380`); the command
  `gitardo.createBranch` (`src/extension/extension.ts:194`); `commitMenu`, `logMenu`, `branchMenu`, `tagMenu`
  (`src/webview/menus.ts:188`, `:250`, `:332`, `:436`).
- **Tested by**: `test/git.test.ts`, *creates branches and tags at a given commit*, *refuses names that git could read
  as options*, *validates ref names before git sees them*; `test/integration.test.ts`, *creates, renames, merges and
  deletes branches*, *undoes commits, checkouts, resets and rebases*; `test/e2e/suite.ts`, *asks for names and options
  with input boxes and pickers*, *names a new branch from the format, at the branch checked out, a tag or a commit, or
  as it is typed*; `test/e2e/ui.mjs`, *the context menu of a commit opens where it was clicked, and creates a branch
  through the real prompts*.
- **Differs**: **Undo** right after it takes back the operation before it, which Gitardo does not warn of: see
  `undo.last`.

Acceptance:

- [ ] **Create branch here…** on a commit, `fix` typed and **Create only**, runs `git branch fix <hash>`; the branch
      checked out does not change.
- [ ] The same with **Create and check out** runs `git checkout -b fix <hash>` and `fix` is checked out.
- [ ] From the toolbar the command is `git checkout -b <name> HEAD` or `git branch <name> HEAD`.
- [ ] On a branch `release` that shares its name with a tag, the command ends with `refs/heads/release`.
- [ ] Typing `-D`, `a b`, `a..b` or `x.lock` keeps the input box open with **That is not a valid Git ref name**;
      typing the name of an existing branch keeps it open with **A branch with that name already exists**.
- [ ] Escape on the name or on the choice runs nothing.
- [ ] It runs through `Controller.run` and the log has the command as shown.
- [ ] **Undo** right after **Create and check out** asks to go back to the branch before and runs `git checkout
      <branch before>`; the new branch is still there.

### `branch.name-format` — Name a new branch from a format

Turns the convention of `gitardo.branches.format` and the answers to its inputs into the name of a new branch, for
**Create branch…** and for creating a worktree.

- **Kind**: Derivation
- **Function**: `expandFormat(format, values)`, `formatInputs(format)`, `formatProblem(format, defined)` and
  `withoutRemote(ref, remotes)` in `src/git/ops.ts`; the questions in `nameFromFormat`, `askInput` and `commitBase` of
  `src/extension/actions.ts`.
- **Entry points**: `branch.create` and `worktree.create`, when a format is set and **Use the format** is picked.
- **Asks**: each `${input:<id>}` in the order the format first has it, once each:
  - `promptString`: an input box with the input's `description` (or its `id`) as prompt, the value used last (or its
    `default`) filled in; empty is refused with **Enter a value**, and with a `pattern`, a value that does not match
    with **It has to match `<pattern>`**.
  - `pickString`: a picker of its `options`, the value used last first (marked **Used last** when it is not one of the
    options), and with `allowCustom` a last entry **Type another value…** that opens an input box.
  - `${base}` at a commit: a picker titled **Create branch: what ${base} is**, with the branches at the commit
    (**Branch at this commit**), the short hash (**This commit**), and up to 50 branches that have it (**Branch that
    has this commit**), each remote branch by its name without its remote.
- **Reads**: `git for-each-ref --contains <hash> --format=%(refname) refs/heads refs/remotes`, for `${base}` at a
  commit.
- **Settings**: `gitardo.branches.format`, `gitardo.branches.inputs`; `gitardo.worktrees.branchFormat` and
  `gitardo.worktrees.inputs` where those are not set. The values used last are kept in the workspace state under
  `worktreeInputs`.
- **Implemented in**: `branchFormat`, `nameFromFormat`, `askInput`, `commitBase`, `newBranchName`
  (`src/extension/actions.ts:1033`, `:1251`, `:1212`, `:1316`, `:1286`); `formatInputs`, `formatProblem`,
  `expandFormat`, `withoutRemote` (`src/git/ops.ts:719`, `:725`, `:737`, `:744`).
- **Tested by**: `test/git.test.ts`, *names branches from a format, with inputs, the base without its remote, the
  repository and the date*; `test/e2e/suite.ts`, *names a new branch from the format, at the branch checked out, a tag
  or a commit, or as it is typed*, *names the branch of a new worktree from the format, asking for its inputs, or as it
  is typed*.

Rules of the result:

- Inputs: the format, the ids defined in the inputs, and the values: `base`, `repo` (the name of the folder of the main
  worktree), `date` (today, `YYYY-MM-DD`, local time) and one value per input.
- Every `${…}` is replaced: `${input:<id>}` by its value, `${base}`, `${repo}` and `${date}` by theirs. Text outside
  `${…}` is kept as it is.
- `${base}` is the branch or tag the branch starts from, without its remote; from `HEAD`, the branch checked out (or the
  commit, chosen as above, when `HEAD` is detached); a name given as `refs/heads/…`, `refs/remotes/…` or `refs/tags/…`
  is matched by its short name.
- A format that names an input that the inputs do not define, or a `${…}` that is not one of the four, is refused
  before anything is asked: **The format uses `${input:kind}`, which gitardo.branches.inputs does not define. Change it
  in the settings.**, or **The format uses `${branch}`, which is not one of ${base}, ${repo}, ${date} or
  ${input:<id>}. Change it in the settings.**
- A `pattern` that is not a regular expression is refused: **The pattern of the input `<id>` in gitardo.branches.inputs
  is not a valid regular expression.**
- The name made is offered in an input box to change, and checked as a typed name is.
- Worked examples:
  - `PRJ-${input:ticket}-${input:kind}-${base}` with ticket `123`, kind `feat`, base `feature/login` →
    `PRJ-123-feat-feature/login`.
  - `${repo}/${date}` in the folder `app` on 15 October 2026 → `app/2026-10-15`.
  - base `origin/feature/login` with the remote `origin` → `feature/login`; `team/fork/fix` with the remotes `team` and
    `team/fork` → `fix` (the longest remote that matches); `main` → `main`.
  - `formatInputs('PRJ-${input:ticket}-${input:kind}-${base}-${input:ticket}')` → `ticket`, `kind`.

Acceptance:

- [ ] With the format `${input:kind}/${base}-${input:ticket}`, kind `feat`, ticket `1`, from the toolbar on `main`,
      the name proposed is `feat/main-1`.
- [ ] On the tag `v1`, with the ticket used last proposed and kept, the name is `fix/v1-1`.
- [ ] At a commit, picking **This commit** gives `${base}` its short hash.
- [ ] A ticket `abc` against the pattern `^\d+$` is refused with **It has to match ^\d+$**.
- [ ] With only `gitardo.worktrees.branchFormat` set to `old-${input:ticket}` and its inputs, the name is `old-3` for
      ticket `3`.

### `branch.rename` — Rename a branch

Gives a local branch another name; git keeps its upstream and its reflog.

- **Kind**: Operation
- **Function**: `renameBranchOp(from, to)` in `src/git/ops.ts`.
- **Entry points**: **Rename…** in the context menu of a local branch, **Rename branch…** when another worktree has it.
- **Offered when**: always, the branch checked out included.
- **Asks**: an input box titled **Rename branch `<name>`**, with the name filled in and checked as for
  `branch.create`; the name as it was is refused as **A branch with that name already exists**.
- **Plan**: `> git branch -m <old> <new>`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Gitardo remembers nothing to rename it back; **Undo** works from the reflog of `HEAD`, which a rename of a
  branch that is not checked out does not change, so it takes back the operation before.
- **After**: a favorite under the old name is no longer listed: favorites are kept by full name.
- **Uses**: `preview.ask`, `log.write`.
- **Implemented in**: `Action` case `renameBranch` (`src/shared/protocol.ts:495`); `resolveAction` case `renameBranch`
  (`src/extension/actions.ts:250`); `renameBranchOp` (`src/git/ops.ts:236`); `branchMenu` (`src/webview/menus.ts:334`).
- **Tested by**: `test/integration.test.ts`, *creates, renames, merges and deletes branches*.
- **Differs**: **Undo** right after it takes back the operation before it, which Gitardo does not warn of: see
  `undo.last`.

Acceptance:

- [ ] **Rename…** on `feature`, `done` typed, runs `git branch -m feature done`, and the branch is listed as `done`
      with the same commit.
- [ ] Typing a name starting with `-` keeps the input box open with **That is not a valid Git ref name**.
- [ ] It runs through `Controller.run` and the log has `> git branch -m feature done`.

### `branch.delete` — Delete a branch

Deletes a local branch after asking, saying whether its commits are merged so that none is lost; **Undo** brings it
back.

- **Kind**: Operation
- **Function**: `deleteBranchOp(name, force)` in `src/git/ops.ts`.
- **Entry points**: **Delete** in the context menu of a local branch.
- **Offered when**: the branch is not checked out here and no other worktree has it (that one offers **Remove worktree
  and delete branch…**, `worktree.remove`).
- **Reads**: `git merge-base --is-ancestor refs/heads/<name> <into>`, where `<into>` is `refs/remotes/<upstream>` when
  the branch has an upstream that is not gone, `HEAD` otherwise: the same rule `git branch -d` follows.
- **Asks**: always: merged, **Delete branch `<name>`?** with **Its commits are merged, so none is lost. Undo brings the
  branch back.**; not merged, the warning **Delete branch `<name>` (force)?** with **This branch is not fully merged.
  Commits that are only on it will no longer be reachable from any branch.**
- **Plan**: merged, `> git branch -d <name>`; not merged, `> git branch -D <name>`. When `-d` is refused with *not fully
  merged* after all, the forced delete is offered and asked as above.
- **Danger**: Confirm when its commits are merged; Destructive when they are not, because commits only on it are no
  longer reachable from a branch.
- **Undo**: Gitardo remembers the branch's name and commit: **Undo** asks **Undo: restore branch `<name>`?** and runs
  `git branch <name> <hash>`, as long as no other operation ran through `Controller.action` and nothing moved `HEAD`
  since. Its upstream is not set again.
- **Refuses**: a name that starts with `-`: **"-x" is not a valid name.**
- **Uses**: `preview.ask`, `log.write`, `undo.last`.
- **Implemented in**: `Action` case `deleteBranch` (`src/shared/protocol.ts:496`); `resolveAction` case
  `deleteBranch` (`src/extension/actions.ts:255`); `Repo.isMergedForDelete` (`src/git/repo.ts:221`); `deleteBranchOp`
  and `restoreBranchOp` (`src/git/ops.ts:241`, `:969`); `Controller.action`, which runs the fallback and records the
  restore (`src/extension/controller.ts:1698`); `branchMenu` (`src/webview/menus.ts:336`).
- **Tested by**: `test/git.test.ts`, *asks before deleting a branch, and only treats the forced delete as dangerous*;
  `test/integration.test.ts`, *creates, renames, merges and deletes branches*, *restores a deleted branch and a dropped
  stash*, *lists what a stash changes and took along, and knows which branches delete without forcing*;
  `test/awkward.test.ts`, *means the branch when a tag has the same name*; `test/menus.test.ts`, *opens the worktree of
  a branch instead of checking it out, and removes it instead of deleting the branch*; `test/e2e/suite.ts`, *undoes the
  last commit, and restores a deleted branch*, *offers the forced delete when git refuses an unmerged branch*, *asks
  before applying a stash, saying what it would stop on, and before deleting a tag or a merged branch*.

Acceptance:

- [ ] **Delete** on a merged branch `topic` asks **Delete branch topic?**, whose detail says **none is lost** and ends
      with `git branch -d topic`, whatever `gitardo.commandPreview` says.
- [ ] **Delete** on a branch with commits of its own asks once, with a warning, **Delete branch feature (force)?**,
      whose detail says **not fully merged** and ends with `git branch -D feature`.
- [ ] Cancelling either runs nothing and the branch is still there.
- [ ] Deleting the branch `release` while a tag `release` exists leaves the tag.
- [ ] **Undo** right after it asks **Undo: restore branch topic?** and the branch is back at the same commit.
- [ ] The branch checked out, and a branch another worktree has, have no **Delete** entry.
- [ ] It runs through `Controller.run` and the log has the command that ran.

### `branch.delete-remote` — Delete a branch from its remote

Deletes a branch on the remote, for everyone, after a warning.

- **Kind**: Operation
- **Function**: `deleteRemoteBranchOp(name, state)` in `src/git/ops.ts`.
- **Entry points**: **Delete from remote** in the context menu of a remote branch; **Delete `<branch>` from
  `<remote>`** under the entries of a branch whose label in the graph stands for its upstream too
  (`gitardo.graph.combineUpstream`).
- **Offered when**: always on a remote branch.
- **Plan**: `> git push <remote> --delete <branch>`. The remote is the longest remote name that starts the remote
  branch's name, so that remotes with slashes in their names are found.
- **Danger**: Destructive, because the branch is deleted on the remote for everyone: **Delete `<branch>` from
  `<remote>`?** with **This deletes the branch on the remote for everyone.**
- **Undo**: Not possible: the branch is gone from the remote, and Gitardo remembers nothing to push it again; **Undo**
  right after it takes back the operation before.
- **Refuses**: a remote branch whose name starts with no known remote: **Cannot tell which remote `<name>` belongs
  to.**
- **Uses**: `preview.ask`, `log.write`.
- **Implemented in**: `Action` case `deleteRemoteBranch` (`src/shared/protocol.ts:499`); `resolveAction` case
  `deleteRemoteBranch` (`src/extension/actions.ts:265`); `deleteRemoteBranchOp` (`src/git/ops.ts:294`);
  `remoteBranchMenu` and `upstreamEntries` (`src/webview/menus.ts:423`, `:358`).
- **Tested by**: `test/git.test.ts`, *finds the remote of a remote branch even when names contain slashes*.
- **Differs**: the branch is passed to git by its short name (`src/git/ops.ts:301`), not as `refs/heads/<branch>`: on a
  remote that also has a tag of that name, git refuses the push as matching more than one.

Acceptance:

- [ ] **Delete from remote** on `team/fork/feature/x`, with the remotes `team` and `team/fork`, runs `git push
      team/fork --delete feature/x`.
- [ ] It always asks first, with a warning, whatever `gitardo.commandPreview` says; cancelling runs nothing.
- [ ] `other/main` with no remote `other` is refused with **Cannot tell which remote other/main belongs to.**
- [ ] It runs through `Controller.run` and the log has the command as shown.
- [ ] A branch `release` whose remote also has a tag `release` is deleted as `refs/heads/release`. (differs)

### `branch.favorite` — Mark a branch as a favorite

Keeps the branches the user goes back to most in a short list above the others, for each repository.

- **Kind**: View
- **Function**: `favoriteEntry(ref, state, name)` in `src/webview/menus.ts`, and `Controller.setFavorite(ref,
  favorite)` and `Controller.favorites(state)` in `src/extension/controller.ts`.
- **Entry points**: **Add to favorites** and **Remove from favorites** in the context menu of a local branch and of a
  remote branch; **Add `<remote branch>` to favorites** under the entries of a branch whose label stands for its
  upstream too.
- **Offered when**: always; the entry says **Remove** when the branch is a favorite.
- **Plan**: no git command: the list of full names (`refs/heads/<name>`, `refs/remotes/<name>`) of each repository is
  written to the workspace state under `gitardo.favorites`, and the views are sent the state at once.
- **After**: the **Favorites** section of the references tree and of the Branches view lists them in the order they
  were marked; each also carries a star where it is listed with the others.
- **Uses**: nothing of the repository.
- **Implemented in**: `WebviewMessage` `setFavorite` (`src/shared/protocol.ts:736`); `Controller.onMessage` case
  `setFavorite`, `setFavorite`, `favorites` (`src/extension/controller.ts:2226`, `:623`, `:616`); `favoriteEntry`,
  `branchMenu`, `remoteBranchMenu`, `upstreamEntries` (`src/webview/menus.ts:110`); the **Favorites** section of
  `RefsTree.tsx`.
- **Tested by**: `test/e2e/suite.ts`, *lists the branches checked out last, and remembers the favorites of each
  repository*.

Acceptance:

- [ ] **Add to favorites** on `feature` and on `origin/main` makes the favorites `refs/heads/feature`,
      `refs/remotes/origin/main`, in that order, and the views are told without the repository being read again.
- [ ] A favorite naming a branch that is not there is not listed.
- [ ] Another repository of the workspace has its own favorites, and switching back shows the first ones again.
- [ ] A branch deleted is no longer a favorite; the others stay.
- [ ] Nothing is run in git and nothing is written to the log.

### `branch.cleanup` — Clean up branches

Deletes, in one go, the local branches that are merged into the current one or whose upstream is gone, with the
worktrees that have them.

- **Kind**: Operation
- **Function**: `Controller.cleanupBranches()` in `src/extension/controller.ts`, which builds
  `cleanupBranchesOp(merged, unmerged, worktrees)` of `src/git/ops.ts`.
- **Entry points**: **Gitardo: Clean Up Branches** in the Command Palette and in the **…** of the Branches view;
  **Clean up branches…** in the **…** menu of the toolbar of the graph.
- **Reads**: `git branch --merged HEAD --format=%(refname:short)`, and the state: upstreams gone, worktrees and their
  files not committed.
- **Asks**:
  1. A picker of several, titled **Clean up branches**, placeholder **Branches to delete, with their worktrees**. It
     lists every local branch merged into `HEAD` or whose upstream is gone, except the branch checked out and the branch
     of the main worktree. Each says why, **merged into `<current branch>`** or **upstream is gone, not merged**, and,
     when a worktree has it, **with its worktree `<worktree>`**, with **`<n>` files not committed** (**1 file not
     committed**) when it has some, or **with its missing worktree**. Merged branches whose worktree has nothing
     uncommitted are ticked; the others are not.
  2. The question of the operation: always asked, **Delete `<n>` branches?** (**Delete 1 branch?**), with **and `<m>`
     worktrees** when worktrees go too; a warning when it includes a branch not merged or a worktree with files not
     committed, saying **Not merged into the current branch, so their commits may become unreachable: `<names>`** and
     **These worktrees have files not committed, which are deleted with them: `<paths>`**.
- **Plan**, in this order: `> git worktree prune` when a worktree picked is missing; for each other worktree picked,
  `> git worktree remove <path>`, with `--force` when it has files not committed; `> git branch -d <merged…>`;
  `> git branch -D <not merged…>`. Before the commands: `git.close <path>` for each worktree VS Code's Git extension has
  open, and on Windows `unlink <path>` for each link to a folder outside it; when Windows holds a folder, `delete
  <path>`; afterwards, the folder of the worktrees is removed when it was left empty.
- **Danger**: Confirm, because branches are deleted; Destructive when a branch is not merged or a worktree has files
  not committed.
- **Undo**: Gitardo remembers nothing of a cleanup: **Undo** does not bring the branches back, and right after it takes
  back the operation before.
- **Refuses**: with nothing to list, **Gitardo: no local branches are merged or have lost their upstream.**; nothing
  picked, nothing runs. Git refuses `-d` for a branch whose upstream does not have its commits, even when `HEAD` has
  them, and the commands after that one do not run.
- **Uses**: `preview.ask`, `log.write`, `worktree.remove`.
- **Implemented in**: `UiCommand` `cleanupBranches` (`src/shared/protocol.ts:650`); `Controller.command` and
  `Controller.cleanupBranches` (`src/extension/controller.ts:1385`, `:1472`); `Repo.mergedBranches`
  (`src/git/repo.ts:909`); `cleanupBranchesOp` (`src/git/ops.ts:267`); `closeInVSCodeGit`, `recoverHeldWorktree`,
  `removeEmptyParents` (`src/extension/actions.ts`), `unlinkFolderLinks` (`src/git/worktreeSetup.ts`); the command
  `gitardo.cleanupBranches` (`src/extension/extension.ts:162`); the **…** menu of `Toolbar.tsx`.
- **Tested by**: `test/git.test.ts`, *deletes merged branches safely and asks before forcing the others*, *removes the
  worktrees of the branches it cleans up first, forcing only the ones with files not committed*;
  `test/integration.test.ts`, *finds the branches that are safe to clean up and deletes them*, *cleans up branches
  together with their worktrees, with files not committed or missing*; `test/e2e/suite.ts`, *cleans up merged branches
  from a picker, with their worktrees*.
- **Differs**: **Undo** right after it takes back the operation before it, which Gitardo does not warn of: see
  `undo.last`.

Acceptance:

- [ ] With `done` merged and `wip` not, the picker lists both, `done` ticked and `wip` not; the branch checked out is
      not listed.
- [ ] Picking `done` and `wip` asks, with a warning naming `wip`, and runs `git branch -d done` then `git branch -D
      wip`.
- [ ] Picking only `done` asks **Delete 1 branch?** as a question, not a warning, whatever `gitardo.commandPreview`
      says.
- [ ] A merged branch `merged` with a worktree is listed **merged into main · with its worktree merged**, ticked; the
      worktree is removed with `git worktree remove <path>` before `git branch -d merged`, and its folder is gone.
- [ ] A worktree with 1 file not committed is unticked; picked, it is removed with `--force` after the warning.
- [ ] A worktree whose folder is gone is pruned with `git worktree prune` rather than removed.
- [ ] `--all` as a branch name is refused with **"--all" is not a valid name.**
- [ ] With nothing to list, **Gitardo: no local branches are merged or have lost their upstream.** is said and nothing
      runs.
- [ ] It runs as one `GitOp` through `Controller.run`; the log has each command and each `git.close`, `unlink` and
      `delete` line.

### `branch.sort` — List branches and tags by name or by date

Orders the local branches, the remote branches and the tags the same way everywhere they are listed: by name, or the
one committed to last first.

- **Kind**: Derivation
- **Function**: `sortRefs(refs, sort)` in `src/shared/refs.ts`; `branchSort()` in `src/extension/controller.ts`, which
  reads the settings.
- **Entry points**: **Sort by name** and **Sort by date** in the **…** on the title of the **Local**, **Remotes** and
  **Tags** sections of the references tree and of the Branches view, the one in use ticked; the setting
  `gitardo.branches.sortOrder`.
- **Plan**: no git command. The menu writes `gitardo.branches.sortOrder` to the user settings; the views are sent the
  state in the new order when the setting changes.
- **Settings**: `gitardo.branches.sortOrder` (`name` or `date`); while it is not set, VS Code's `git.branchSortOrder`
  set to `committerdate` means `date`, and anything else means `name`.
- **Implemented in**: `sortRefs` (`src/shared/refs.ts:19`); `branchSort` and its use when the state is sent
  (`src/extension/controller.ts:216`, `:434`); `Controller.onMessage` case `setBranchSort`
  (`src/extension/controller.ts:2214`); `sorting` in `src/webview/RefsTree.tsx`; the date of each reference,
  `%(creatordate:unix)` in `REF_FORMAT` (`src/git/parse.ts:22`).
- **Tested by**: `test/git.test.ts`, *sorts refs by kind, reads tracking info and peels annotated tags*, *keeps the
  order of the names among branches committed to at the same time*; `test/integration.test.ts`, *points annotated tags
  at their commit*. `branchSort` and its fallback: *none*.

Rules of the result:

- Inputs: the branches, remote branches and tags as `git for-each-ref` lists them (by name), each with its date: the
  date of its commit, or for a tag made with a message, when the tag was made; and the order, `name` or `date`.
- `name`: the lists are returned as they are.
- `date`: each list is ordered by date, latest first; references of the same date keep the order of their names. The
  lists given are not changed.
- The three lists are always ordered the same way.
- Worked examples:
  - branches `a` (date 5), `b` (9), `c` (5), by `date` → `b`, `a`, `c`; by `name` → `a`, `b`, `c`.
  - tags `v1` (1), `v2` (2), by `date` → `v2`, `v1`.
  - `gitardo.branches.sortOrder` not set and `git.branchSortOrder` `committerdate` → `date`; neither set → `name`.

Acceptance:

- [ ] **Sort by date** in the menu of **Local** sets `gitardo.branches.sortOrder` to `date` in the user settings, and
      the Local, Remotes and Tags sections, the Branches view and the pickers of branches list the latest first.
- [ ] With the setting not set and `git.branchSortOrder` set to `alphabetically`, they are listed by name.
- [ ] Nothing runs in git and nothing is written to the log.

### `tag.create` — Create a tag

Makes a tag at a commit, with a message (annotated) or without (lightweight).

- **Kind**: Operation
- **Function**: `createTagOp(name, at, message)` in `src/git/ops.ts`.
- **Entry points**: **Create tag here…** in the context menu of a commit in the graph and in the Commits view.
- **Asks**:
  1. An input box titled **Create tag**, placeholder **v1.0.0**, checked as a branch name is, and **A tag with that name
     already exists**.
  2. An input box titled **Create tag `<name>`** with the prompt **Message for an annotated tag. Leave empty for a
     lightweight tag.** Escape cancels; the message is trimmed.
- **Plan**: with a message, `> git tag -a <name> -m "<message>" <hash>`; without, `> git tag <name> <hash>`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Not taken back: a tag does not move `HEAD`, and Gitardo remembers nothing; **Undo** right after it takes
  back the operation before. `tag.delete` removes it.
- **Refuses**: a name starting with `-`, by the input box and by the builder.
- **Uses**: `preview.ask`, `log.write`.
- **Implemented in**: `Action` case `createTag` (`src/shared/protocol.ts:563`); `resolveAction` case `createTag`
  (`src/extension/actions.ts:501`); `createTagOp` (`src/git/ops.ts:535`); `commitMenu`, `logMenu`
  (`src/webview/menus.ts:189`, `:251`).
- **Tested by**: `test/git.test.ts`, *creates branches and tags at a given commit*; `test/integration.test.ts`, *points
  annotated tags at their commit*; `test/e2e/suite.ts`, *asks for names and options with input boxes and pickers*.
- **Differs**: **Undo** right after it takes back the operation before it, which Gitardo does not warn of: see
  `undo.last`.

Acceptance:

- [ ] `v1` with the message `First release` runs `git tag -a v1 -m "First release" <hash>`, and the tag is listed at
      that commit.
- [ ] `light` with an empty message runs `git tag light <hash>`.
- [ ] Escape on either input box runs nothing.
- [ ] The name of an existing tag keeps the input box open with **A tag with that name already exists**.
- [ ] It runs through `Controller.run` and the log has the command as shown, the message quoted.

### `tag.push` — Push a tag

Sends a tag to the default remote.

- **Kind**: Operation
- **Function**: `pushTagOp(name, state)` in `src/git/ops.ts`.
- **Entry points**: **Push tag** in the context menu of a tag.
- **Offered when**: the repository has a remote.
- **Plan**: `> git push <remote> refs/tags/<name>`, to `origin` when there is one, else to the first remote; the tag is
  named in full, so that a branch of the same name is not pushed instead.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Not possible: the remote has the tag, and `tag.delete` deletes it only here.
- **Refuses**: with no remote, **This repository has no remote.**
- **Uses**: `preview.ask`, `log.write`.
- **Implemented in**: `Action` case `pushTag` (`src/shared/protocol.ts:565`); `resolveAction` case `pushTag`
  (`src/extension/actions.ts:514`); `pushTagOp` and `defaultRemote` (`src/git/ops.ts:551`, `:94`); `tagMenu`
  (`src/webview/menus.ts:443`).
- **Tested by**: *none*.

Acceptance:

- [ ] **Push tag** on `v1` with the remotes `origin` and `backup` runs `git push origin refs/tags/v1`, titled **Push
      tag v1 to origin**.
- [ ] With only the remote `upstream`, it runs `git push upstream refs/tags/v1`.
- [ ] A repository without a remote has no **Push tag** entry.
- [ ] It runs through `Controller.run` and the log has the command as shown.

### `tag.delete` — Delete a tag

Deletes a tag from this repository, after a warning that the remotes keep it.

- **Kind**: Operation
- **Function**: `deleteTagOp(name, hash)` in `src/git/ops.ts`.
- **Entry points**: **Delete** in the context menu of a tag.
- **Asks**: always, with a warning: **Delete tag `<name>`?** with **The tag, on `<short hash>`, is deleted from this
  repository, with its message if it has one. A remote that has it keeps it.**
- **Plan**: `> git tag -d <name>`.
- **Danger**: Destructive, because a tag with a message is gone with its message.
- **Undo**: Gitardo remembers nothing to make it again: **Undo** right after it takes back the operation before.
- **Uses**: `preview.ask`, `log.write`.
- **Implemented in**: `Action` case `deleteTag` (`src/shared/protocol.ts:564`); `resolveAction` case `deleteTag`
  (`src/extension/actions.ts:511`); `deleteTagOp` (`src/git/ops.ts:541`); `tagMenu` (`src/webview/menus.ts:444`).
- **Tested by**: `test/git.test.ts`, *warns before deleting a tag that it stays on the remotes*; `test/e2e/suite.ts`,
  *asks before applying a stash, saying what it would stop on, and before deleting a tag or a merged branch*.
- **Differs**: **Undo** right after it takes back the operation before it, which Gitardo does not warn of: see
  `undo.last`.

Acceptance:

- [ ] **Delete** on `v1` asks **Delete tag v1?**, as a warning, whatever `gitardo.commandPreview` says; the detail
      names the tag's commit by its short hash and ends with `git tag -d v1`.
- [ ] Cancelling runs nothing and the tag is still there.
- [ ] After it, a remote that had `v1` still has it.
- [ ] It runs through `Controller.run` and the log has `> git tag -d v1`.

### `remote.manage` — List the remotes and choose what to do with one

Lists the remotes with their URLs, and leads to fetching, changing the URL of, renaming, copying the URL of or removing
one, or adding another.

- **Kind**: View
- **Function**: `Controller.manageRemotes(add)` in `src/extension/controller.ts`.
- **Entry points**: **Gitardo: Manage Remotes** in the Command Palette and in the **…** of the Branches view; **Manage
  remotes…** in the **…** menu of the toolbar of the graph; **Add Remote…** after a fetch, pull or push in a repository
  without a remote, which goes straight to `remote.add`.
- **Asks**:
  1. A picker titled **Remotes of `<repository>`**, placeholder **Pick a remote to fetch, rename, repoint or remove
     it**: each remote with its URL as description (searched too), then **Add remote…**.
  2. For a remote, a picker titled **`<name>`: `<url>`**: **Fetch** (described `git fetch --prune <name>`), **Change
     URL…**, **Rename…**, **Copy URL**, **Remove** (**Deletes its remote-tracking branches here**).
- **Reads**: `git remote` and `git remote get-url <name>` for each.
- **After**: when an operation ran, the remote that pull requests are read from is found again, and the repository is
  read again. **Copy URL** puts the URL on the clipboard and runs nothing.
- **Uses**: `remote.add`, `remote.fetch`, `remote.change-url`, `remote.rename`, `remote.remove`.
- **Implemented in**: `UiCommand` `remotes` (`src/shared/protocol.ts:656`); `Controller.command` and
  `Controller.manageRemotes` (`src/extension/controller.ts:1385`, `:1929`); `Repo.remotes` (`src/git/repo.ts:925`);
  the command `gitardo.remotes` (`src/extension/extension.ts:181`); the **…** menu of `Toolbar.tsx`.
- **Tested by**: `test/e2e/suite.ts`, *adds, renames and removes a remote, and offers to add one when there is none*.

Acceptance:

- [ ] **Gitardo: Manage Remotes** lists each remote with its URL, then **Add remote…**.
- [ ] Picking a remote offers **Fetch**, **Change URL…**, **Rename…**, **Copy URL** and **Remove**, in that order.
- [ ] **Copy URL** writes the URL to the clipboard and nothing to the log.
- [ ] Escape at either picker runs nothing.

### `remote.add` — Add a remote

Adds a remote by name and URL.

- **Kind**: Operation
- **Function**: `remoteAddOp(name, url)` in `src/git/ops.ts`.
- **Entry points**: **Add remote…** in `remote.manage`; **Add Remote…** in the warning **Gitardo: this repository has
  no remote.** after a fetch, pull or push.
- **Asks**: an input box titled **Add remote**, prompt **Name of the remote**, with `origin` filled in when there is no
  remote yet; then an input box titled **Add remote `<name>`**, prompt **URL of the remote repository**, placeholder
  **https://github.com/owner/repository.git**. The name is checked as a branch name is, and **A remote with that name
  already exists**; the URL with **Enter a URL** and, when it starts with `-`, **That is not a URL**. The URL is
  trimmed.
- **Plan**: `> git remote add <name> <url>`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Not taken back by **Undo**; `remote.remove` removes it.
- **Refuses**: a name or URL starting with `-`, also in the builder: **"`<name>`" is not a valid name for a remote.**,
  **"`<url>`" is not a valid name.**
- **Uses**: `remote.manage`, `preview.ask`, `log.write`.
- **Implemented in**: `Controller.manageRemotes` (`src/extension/controller.ts:1929`); `Controller.sync`, which offers
  it (`src/extension/controller.ts:1583`); `remoteAddOp`, `remoteName` (`src/git/ops.ts:141`, `:135`).
- **Tested by**: `test/integration.test.ts`, *adds, repoints, renames, fetches and removes remotes*;
  `test/e2e/suite.ts`, *adds, renames and removes a remote, and offers to add one when there is none*.

Acceptance:

- [ ] `backup` and `https://example.invalid/backup.git` run `git remote add backup https://example.invalid/backup.git`,
      and the log has it.
- [ ] In a repository without a remote, the name box starts with `origin`.
- [ ] `--upload-pack=x` as the name or as the URL is refused and git is not run.
- [ ] It runs through `Controller.run`.

### `remote.fetch` — Fetch one remote

Fetches a single remote, pruning its remote branches that are gone.

- **Kind**: Operation
- **Function**: `fetchRemoteOp(name)` in `src/git/ops.ts`.
- **Entry points**: **Fetch** for a remote in `remote.manage`.
- **Plan**: `> git fetch --prune <name>`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Nothing to take back on the local branches: **Undo** right after it takes back the operation before.
- **Uses**: `remote.manage`, `preview.ask`, `log.write`.
- **Implemented in**: `Controller.manageRemotes` (`src/extension/controller.ts:1929`); `fetchRemoteOp`
  (`src/git/ops.ts:162`).
- **Tested by**: `test/integration.test.ts`, *adds, repoints, renames, fetches and removes remotes*.
- **Differs**: **Undo** right after it takes back the operation before it, which Gitardo does not warn of: see
  `undo.last`.

Acceptance:

- [ ] **Fetch** on `upstream` runs `git fetch --prune upstream` through `Controller.run`, and the log has it.

### `remote.change-url` — Change the URL of a remote

Points a remote at another URL.

- **Kind**: Operation
- **Function**: `remoteSetUrlOp(name, url)` in `src/git/ops.ts`.
- **Entry points**: **Change URL…** for a remote in `remote.manage`.
- **Asks**: an input box titled **Change the URL of `<name>`** with the URL filled in, checked as for `remote.add`.
- **Plan**: `> git remote set-url <name> <url>`; nothing when the URL is the same.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Not taken back by **Undo**; changing the URL again sets it back.
- **Uses**: `remote.manage`, `preview.ask`, `log.write`.
- **Implemented in**: `Controller.manageRemotes` (`src/extension/controller.ts:1929`); `remoteSetUrlOp`
  (`src/git/ops.ts:145`).
- **Tested by**: `test/integration.test.ts`, *adds, repoints, renames, fetches and removes remotes*.

Acceptance:

- [ ] A new URL for `origin` runs `git remote set-url origin <url>`, and a push then goes to it.
- [ ] The same URL, or Escape, runs nothing.

### `remote.rename` — Rename a remote

Gives a remote another name; git renames its remote branches and the upstreams that follow them.

- **Kind**: Operation
- **Function**: `remoteRenameOp(from, to)` in `src/git/ops.ts`.
- **Entry points**: **Rename…** for a remote in `remote.manage`.
- **Asks**: an input box titled **Rename remote `<name>`** with the name filled in, checked as for `remote.add`; the
  name as it was runs nothing.
- **Plan**: `> git remote rename <old> <new>`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Not taken back by **Undo**; renaming it again does.
- **Uses**: `remote.manage`, `preview.ask`, `log.write`.
- **Implemented in**: `Controller.manageRemotes` (`src/extension/controller.ts:1929`); `remoteRenameOp`
  (`src/git/ops.ts:149`).
- **Tested by**: `test/integration.test.ts`, *adds, repoints, renames, fetches and removes remotes*;
  `test/e2e/suite.ts`, *adds, renames and removes a remote, and offers to add one when there is none*.

Acceptance:

- [ ] Renaming `origin` to `upstream` runs `git remote rename origin upstream`; the remote branches are then
      `upstream/…` and `main` follows `upstream/main`.
- [ ] A name taken by another remote keeps the input box open with **A remote with that name already exists**.

### `remote.remove` — Remove a remote

Removes a remote from this repository, after a warning; nothing changes on the remote.

- **Kind**: Operation
- **Function**: `remoteRemoveOp(name)` in `src/git/ops.ts`.
- **Entry points**: **Remove** for a remote in `remote.manage`.
- **Asks**: always, with a warning: **Remove remote `<name>`?** with **Its remote-tracking branches (`<name>`/…) are
  deleted from this repository, and local branches that follow them lose their upstream. Nothing changes on the remote
  itself.**
- **Plan**: `> git remote remove <name>`.
- **Danger**: Destructive, because its remote branches go and local branches lose their upstream.
- **Undo**: Not possible from Gitardo: the remote and the upstreams are gone from the configuration; `remote.add` and a
  fetch bring the remote branches back, not the upstreams.
- **Uses**: `remote.manage`, `preview.ask`, `log.write`.
- **Implemented in**: `Controller.manageRemotes` (`src/extension/controller.ts:1929`); `remoteRemoveOp`
  (`src/git/ops.ts:153`).
- **Tested by**: `test/integration.test.ts`, *adds, repoints, renames, fetches and removes remotes*;
  `test/e2e/suite.ts`, *adds, renames and removes a remote, and offers to add one when there is none*.

Acceptance:

- [ ] **Remove** on `mirror` asks **Remove remote mirror?** as a warning, whatever `gitardo.commandPreview` says.
- [ ] Run, the repository has no remote `mirror`, no remote branch under it, and a branch that followed it has no
      upstream.
- [ ] It runs through `Controller.run` and the log has `> git remote remove mirror`.

### `sync.fetch` — Fetch every remote

Brings what every remote has, and drops the remote branches deleted there.

- **Kind**: Operation
- **Function**: `syncOp('fetch', state)` in `src/git/ops.ts`, run by `Controller.sync(action)`.
- **Entry points**: **Fetch** in the toolbar of the graph (tooltip **Fetch: git fetch --all --prune**);
  **Gitardo: Fetch** in the Command Palette and as a button on the titles of the Commits and Gitardo Graph views.
- **Offered when**: the toolbar is wide enough for its buttons (not in a narrow panel), and no operation runs.
- **Plan**: `> git fetch --all --prune`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Nothing to take back on the local branches: **Undo** right after it takes back the operation before.
- **After**: the pull requests are read again from GitHub when the user signed in.
- **Refuses**: in a repository without a remote, nothing runs and **Gitardo: this repository has no remote.** offers
  **Add Remote…** (`remote.add`).
- **Uses**: `preview.ask`, `log.write`, `activity.track`.
- **Implemented in**: `SyncAction` `fetch` (`src/shared/protocol.ts:328`); `WebviewMessage` `sync` and
  `Controller.onMessage` (`src/extension/controller.ts:2368`); `Controller.sync` (`src/extension/controller.ts:1583`);
  `syncOp` (`src/git/ops.ts:107`); the toolbar button (`src/webview/Toolbar.tsx:355`); the commands `gitardo.fetch`
  (`src/extension/extension.ts:254`).
- **Tested by**: `test/integration.test.ts`, *pulls with a rebase*; `test/e2e/suite.ts`, *pushes, fetches and pulls
  with a rebase against the remote*.
- **Differs**: **Undo** right after it takes back the operation before it, which Gitardo does not warn of: see
  `undo.last`.

Acceptance:

- [ ] **Fetch** runs `git fetch --all --prune` through `Controller.run`, and the log has it.
- [ ] After a fetch, a branch whose remote has new commits shows how far behind it is.
- [ ] In a repository without a remote, nothing runs and the warning offers **Add Remote…**.

### `sync.pull` — Pull, in one of four ways

Brings the commits of the upstream into the branch checked out, merged or with the local commits put on top, and with
the changes not committed set aside and put back, or not, as the settings or the menu choose.

- **Kind**: Operation
- **Function**: `syncOp(action, state)` for `pull`, `pullRebase`, `pullAutostash` and `pullRebaseAutostash` in
  `src/git/ops.ts`; `pullAction(rebase, autostash)` picks the one of the button; `Controller.sync(action)` runs it.
- **Entry points**: **Pull** in the toolbar of the graph, with the number of commits the branch is behind; the menu
  **More pull options** beside it: **Pull**, **Pull (rebase)**, **Pull (autostash)**, **Pull (rebase, autostash)**, the
  one of the button marked **default**. **Gitardo: Pull** (the button), **Gitardo: Pull (No Options)**, **Gitardo: Pull
  (Rebase)**, **Gitardo: Pull (Autostash)**, **Gitardo: Pull (Rebase, Autostash)** in the Command Palette; on the
  titles of the Commits and Gitardo Graph views, **Pull** as a button and the four others in their **…**.
- **Offered when**: the toolbar is wide enough for its buttons, and no operation runs.
- **Plan**: one of `> git pull`, `> git pull --rebase`, `> git pull --autostash`, `> git pull --rebase --autostash`,
  titled **Pull**, **Pull (rebase)**, **Pull (autostash)**, **Pull (rebase, autostash)**. The button runs
  `--rebase` unless `gitardo.pull.rebase` is `false`, and `--autostash` unless `gitardo.pull.autostash` is `false`; its
  tooltip says the command, as **Pull: git pull --rebase --autostash**. **Gitardo: Pull (No Options)** is always
  `git pull`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: from the reflog of the branch: **Undo** moves the branch back to where it was before the pull, with `git
  reset --keep <previous>`, or `--mixed` when the files already match it (`undo.last`).
- **After**: with `--autostash`, when git kept the stash it made because the changes no longer fit on what came in, the
  warning **Gitardo: pulled, but your changes do not fit on what came in. They are in conflict in the files, and whole
  in the stash "`<message>`": resolve the conflicts, then drop the stash.** with **Show Stashes**, which opens the
  Stashes view. The pull requests are read again. When the pull stops on a conflict: **`<title>` stopped: …**, as for
  any operation with conflicts (`conflict.review`).
- **Refuses**: without a remote, nothing runs and **Gitardo: this repository has no remote.** offers **Add Remote…**.
  Without `--autostash`, git refuses a pull that would touch a file with changes not committed: **`<title>` failed.**
  and git's line, with **Show Log**.
- **Settings**: `gitardo.pull.rebase` (default on), `gitardo.pull.autostash` (default on).
- **Uses**: `preview.ask`, `log.write`, `activity.track`, `undo.last`, `stash.drop`, `conflict.review`.
- **Implemented in**: `SyncAction` and `PullAction` (`src/shared/protocol.ts:328`, `:340`); `RepoState.defaultPull`;
  `Controller.defaultPull`, `Controller.pull`, `Controller.sync` (`src/extension/controller.ts:1573`, `:1579`,
  `:1583`); `syncOp`, `pullAction` (`src/git/ops.ts:107`, `:101`); `PULL_VARIANTS` and the Pull button
  (`src/webview/Toolbar.tsx:38`, `:360`); the commands (`src/extension/extension.ts:254`, `:255`).
- **Tested by**: `test/integration.test.ts`, *pulls with a rebase*, *pulls with changes that are not committed set
  aside and put back, and keeps them in a stash when they no longer fit*; `test/e2e/suite.ts`, *pushes, fetches and
  pulls with a rebase against the remote*, *pulls with the changes that are not committed set aside, and says when they
  do not fit on what came in*, *the Pull button pulls the way the settings choose: with a rebase and the changes set
  aside, unless told otherwise*; `test/e2e/ui.mjs`, *the button beside Pull lists the ways to pull, with and without
  setting changes aside*.

Acceptance:

- [ ] With both settings on (the default), **Pull** runs `git pull --rebase --autostash` and its tooltip is **Pull: git
      pull --rebase --autostash**.
- [ ] `gitardo.pull.rebase` `false` makes the button run `git pull --autostash`; both `false`, `git pull`;
      `gitardo.pull.autostash` alone `false`, `git pull --rebase`.
- [ ] The menu beside the button lists the four ways in the order **Pull**, **Pull (rebase)**, **Pull (autostash)**,
      **Pull (rebase, autostash)**, with **default** on the one of the button, whatever the settings.
- [ ] **Gitardo: Pull (No Options)** runs `git pull` whatever the settings.
- [ ] After a pull with a rebase, the local commit is on top of the one that came in, with no merge commit.
- [ ] With a change not committed in a file the pull does not touch, **Pull (autostash)** leaves the change in the file
      and no stash.
- [ ] With a change where the pull changes the file too, **Pull (rebase, autostash)** leaves the file in conflict, a
      stash named `autostash`, and the warning with **Show Stashes**.
- [ ] Each runs through `Controller.run`, and the log has the command, as `> git pull --rebase --autostash`.
- [ ] **Undo** right after a pull puts the branch back on its commit before the pull.

### `sync.push` — Push the branch checked out

Sends the commits of the branch checked out to its upstream, or, the first time, to the default remote, setting the
upstream.

- **Kind**: Operation
- **Function**: `syncOp('push', state)` in `src/git/ops.ts`, run by `Controller.sync(action)`.
- **Entry points**: **Push** in the toolbar of the graph (tooltip **Push: git push**), with the number of commits the
  branch is ahead; **Push** in the menu **More push options** beside it, marked **default**; **Gitardo: Push** in the
  Command Palette and as a button on the titles of the Commits and Gitardo Graph views.
- **Offered when**: the toolbar is wide enough for its buttons, and no operation runs.
- **Plan**:
  - with an upstream of the same name: `> git push <remote> <branch>`;
  - with an upstream of another name: `> git push <remote> <branch>:<upstream branch>`;
  - without an upstream: `> git push --set-upstream <remote> <branch>`, to `origin` when there is one, else to the
    first remote.
  The branch is named `refs/heads/<branch>` when a tag or a remote branch shares its name.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Not possible: what the remote has cannot be taken back from here. **Undo** right after it takes back the
  last change of `HEAD`, which may be a commit that was just pushed; it says so in its question.
- **After**: the first push of a branch, when the main remote is on GitHub and the branch has no pull request yet:
  **Pushed `<branch>`.** with **Create Pull Request**, which opens GitHub's page to create one. The pull requests are
  read again.
- **Refuses**: on a detached `HEAD`, **Gitardo: Cannot push: HEAD is detached. Check out a branch first.**; without a
  remote, nothing runs and **Gitardo: this repository has no remote.** offers **Add Remote…**. A push the remote
  refuses, as when it has commits the branch has not: **Push `<branch>` failed.** and git's line.
- **Uses**: `preview.ask`, `log.write`, `activity.track`, `remote.add`.
- **Implemented in**: `SyncAction` `push` (`src/shared/protocol.ts:328`); `Controller.sync`
  (`src/extension/controller.ts:1583`); `syncOp`, `defaultRemote` (`src/git/ops.ts:107`, `:94`); `refArg`
  (`src/shared/refs.ts:9`); `PUSH_VARIANTS` and the Push button (`src/webview/Toolbar.tsx:45`, `:370`); the command
  `gitardo.push` (`src/extension/extension.ts:254`).
- **Tested by**: `test/git.test.ts`, *pushes to the upstream of the current branch*, *spells out the remote branch when
  its name differs*, *sets the upstream on the first push*, *names the branch in full when a tag is called the same,
  which git would not tell apart*, *refuses to push without a branch or a remote*; `test/awkward.test.ts`, *means the
  branch when a tag has the same name*; `test/e2e/suite.ts`, *pushes, fetches and pulls with a rebase against the
  remote*, *adds, renames and removes a remote, and offers to add one when there is none*.
- **Differs**: **Undo** right after it takes back the operation before it, which Gitardo does not warn of: see
  `undo.last`.

Acceptance:

- [ ] `work` following `up/work` pushes with `git push up work`; following `up/main`, with `git push up work:main`.
- [ ] `work` with no upstream pushes with `git push --set-upstream origin work`, and afterwards follows `origin/work`.
- [ ] With a tag `work`, the commands are `git push --set-upstream origin refs/heads/work` and `git push up
      refs/heads/work:main`, and the remote's `refs/heads/work` gets the branch's commit.
- [ ] On a detached `HEAD` nothing runs and the error says **HEAD is detached**.
- [ ] In a repository without a remote, nothing runs and **Gitardo: this repository has no remote.** offers **Add
      Remote…**.
- [ ] It runs through `Controller.run`; with `gitardo.commandPreview` `off` nothing is asked, and the log has the
      command as shown.

### `sync.push-force` — Force push, with or without a lease

Replaces the remote branch with the branch checked out, after a warning that lists the commits of the remote branch
that would be overwritten.

- **Kind**: Operation
- **Function**: `syncOp('pushForceWithLease' | 'pushForce', state)` in `src/git/ops.ts`; the list of commits is added by
  `Controller.sync(action)` with `Repo.commitsOnlyIn(theirs, limit)`.
- **Entry points**: **Push (force with lease)** and **Push (force)** in the menu **More push options** beside **Push**
  in the toolbar; **Gitardo: Push (Force with Lease)** and **Gitardo: Push (Force)** in the Command Palette and in the
  **…** of the Commits and Gitardo Graph views.
- **Reads**: `git log --max-count=11 --format=%h %s HEAD..<upstream>`: the commits of the remote branch, as of the last
  fetch, that the branch has not.
- **Asks**: always, with a warning: **Push `<branch>` (--force-with-lease)?** or **Push `<branch>` (--force)?**. With
  an upstream that is not gone, the detail starts with **These commits on `<upstream>` are not in `<branch>` and will be
  overwritten:** and up to 10 of them as `<short hash> <subject>`, then **…and more** when there are more; or **As of
  the last fetch, every commit on `<upstream>` is also in `<branch>`.**
- **Plan**: as `sync.push`, with `--force-with-lease` or `--force` right after `push`: `> git push --force-with-lease
  <remote> <branch>`, `> git push --force --set-upstream origin <branch>`.
- **Danger**: Destructive, because the commits listed are lost from the remote branch.
- **Undo**: Not possible from Gitardo: the overwritten commits are only where they were fetched or made.
- **Refuses**: as `sync.push`; with `--force-with-lease`, git refuses when the remote branch moved since the last
  fetch: **Push `<branch>` (--force-with-lease) failed.** and git's line.
- **Uses**: `preview.ask`, `log.write`, `sync.push`.
- **Implemented in**: `SyncAction` `pushForceWithLease`, `pushForce` (`src/shared/protocol.ts:328`); `FORCE_FLAGS` and
  `syncOp` (`src/git/ops.ts:88`, `:107`); `Controller.sync` and `LOST_COMMITS_SHOWN`
  (`src/extension/controller.ts:1583`, `:91`); `Repo.commitsOnlyIn` (`src/git/repo.ts:1043`); `PUSH_VARIANTS`
  (`src/webview/Toolbar.tsx:45`); the commands (`src/extension/extension.ts:254`).
- **Tested by**: `test/git.test.ts`, *marks force pushes as dangerous*; `test/e2e/suite.ts`, *warns about what a force
  push overwrites*.

Acceptance:

- [ ] After the last commit was amended, **Push (force with lease)** asks once, with a warning, whatever
      `gitardo.commandPreview` says; the detail says **will be overwritten:** followed by the hash and subject of the
      commit as it was, and ends with `git push --force-with-lease origin main`.
- [ ] Cancelling runs nothing; **Run** pushes and the branch is then neither ahead nor behind its upstream.
- [ ] With 12 commits only on the remote branch, the detail lists 10 and **…and more**.
- [ ] With nothing only on the remote branch, the detail says **As of the last fetch, every commit on origin/main is
      also in main.**
- [ ] **Push (force)** on a branch without an upstream runs `git push --force --set-upstream origin <branch>`.
- [ ] The log has the command as shown, with no flag added.

### `sync.fast-forward` — Fast-forward a branch that is not checked out

Brings a local branch up to its upstream without switching to it: the files and the branch checked out are not
touched. A branch that another worktree has checked out is pulled in that worktree instead.

- **Kind**: Operation
- **Function**: `fastForwardOp(name, state)` in `src/git/ops.ts`; `canFastForward(branch, state)` in
  `src/shared/refs.ts` decides when it is offered.
- **Entry points**: **Fast-forward to `<upstream>`** in the context menu of a local branch, in the references tree, the
  Branches view and on its label in the graph.
- **Offered when**: the branch is not checked out here, has an upstream that is not gone, and has no commits of its own
  as of the last fetch (whether it is behind is not required: the fetch finds out). When another worktree has it: only
  while that worktree's folder is there and nothing is uncommitted in it.
- **Plan**:
  - not checked out anywhere: `> git fetch <remote> refs/heads/<remote branch>:refs/heads/<branch>`, titled
    **Fast-forward `<branch>` to `<upstream>`**; without a `+`, git refuses anything but a fast-forward. An upstream
    that is a local branch fetches from `.`, this repository.
  - checked out in another worktree: `> git -C <worktree> pull --ff-only --no-rebase --no-autostash <remote>
    refs/heads/<remote branch>`, titled **Fast-forward `<branch>` to `<upstream>` in its worktree**, whatever
    `gitardo.pull.rebase` and `gitardo.pull.autostash` say.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Not taken back: `HEAD` of this window does not move, so **Undo** right after it takes back the operation
  before.
- **Refuses**: when the conditions above do not hold, **`<branch>` cannot be fast-forwarded: it needs an upstream, no
  commits of its own, and not to be checked out here or in a worktree with changes not committed.**; when the remote
  moved in a way that is not a fast-forward, git refuses: **Fast-forward `<branch>` to `<upstream>` failed.** and git's
  line, and the branch stays where it was.
- **Uses**: `preview.ask`, `log.write`.
- **Implemented in**: `Action` case `fastForward` (`src/shared/protocol.ts:498`); `resolveAction` case `fastForward`
  (`src/extension/actions.ts:262`); `fastForwardOp` (`src/git/ops.ts:172`); `canFastForward` and `worktreeOf`
  (`src/shared/refs.ts:74`, `:64`); `branchMenu` (`src/webview/menus.ts:319`).
- **Tested by**: `test/git.test.ts`, *fetches the remote branch of the upstream into the branch, named in full*,
  *fetches from this repository when the upstream is a local branch*, *is offered for a branch with no commits of its
  own, even when the last fetch saw nothing new*, *is not offered for what git would refuse or what Pull already does*,
  *pulls a branch that another worktree has checked out in that worktree, only while nothing is uncommitted there*;
  `test/integration.test.ts`, *fast-forwards a branch that is not checked out, and refuses one with commits of its
  own*, *fast-forwards a branch that another worktree has checked out in that worktree, only while nothing is
  uncommitted there*; `test/e2e/suite.ts`, *fast-forwards a branch that is not checked out to what its upstream has on
  the remote*.
- **Differs**: **Undo** right after it takes back the operation before it, which Gitardo does not warn of: see
  `undo.last`.

Acceptance:

- [ ] **Fast-forward to up/main** on `feature` runs `git fetch up refs/heads/main:refs/heads/feature`, and `feature` is
      then at the commit of `up/main`.
- [ ] Afterwards the branch checked out, `HEAD` and the files, a change not committed included, are as they were.
- [ ] With the upstream `main` of this repository, the command is `git fetch . refs/heads/main:refs/heads/feature`.
- [ ] It is not offered for the branch checked out, a branch without an upstream, one whose upstream is gone, or one
      with a commit of its own.
- [ ] When the remote branch moved in a way that is not a fast-forward, git refuses and `feature` stays on its commit.
- [ ] With a clean worktree on `feature`, it runs `git -C <worktree> pull --ff-only --no-rebase --no-autostash up
      refs/heads/main`, and the worktree's files have what came in.
- [ ] With 1 file not committed in that worktree, it is not offered, and asked anyway it refuses with **feature cannot
      be fast-forwarded: …** before git runs.
- [ ] It runs through `Controller.run`, and the log has the command as shown.
