# Branches, tags, remotes and sync

Checking out, creating, renaming, deleting and cleaning up branches; bringing a branch that is not checked out up to its
upstream; the format that names new branches; tags; the branch rail as a whole; the remotes of a repository;
and fetch, pull and push with their variants, and the sync that does all three in one click. Merging and rebasing are
in `merge.` and `rebase.`; worktrees, which a branch is often opened in, are in `worktree.`; the pull requests of GitHub
are in `github.`.

### `branch.checkout` — Check out a branch

Switches the working tree to a local branch.

- **Kind**: Operation
- **Function**: `features::branch::checkout(context, answers) -> Outcome`
- **Entry points**: **Check out `<branch>`** in the context menu of a local branch, in the timeline and in the branch
  rail beside it; a double click on a local branch, as `branch.open` decides.
- **Extension**: **Check out `<branch>`** in the context menu of a local branch in the references tree, the Branches
  view and on its label in the graph; a double click there and in **Recent** and **Favorites** (`branch.open`). Each
  sends `feature/run` of `branch.checkout` to Gitardo's engine (`Controller.runOnEngine` in
  `src/extension/controller.ts`, `engineRuns` in `src/engine/actions.ts`), with the branch as its target; the plan is
  shown in a modal dialog when it asks first or `gitardo.commandPreview` is `all` (`VscodeAsker` in
  `src/engine/vscode.ts`). The settings are `gitardo.submodules.updateOnCheckout` and `gitardo.commandPreview`. When the
  engine's binary is missing, it says so (D49).
- **Offered when**: the branch is not the one checked out, and no other worktree has it checked out. Where another
  worktree has it, its menu opens that worktree instead (`worktree.open`).
- **Plan**: `git checkout <branch>`; then, as an operation of its own that the confirmation shows with it, the update of
  the submodules (`submodule.update-on-checkout`).
- **Danger**: None, or Confirm when the update of the submodules would leave commits behind (see
  `submodule.update-on-checkout`).
- **Undo**: By the reflog: Undo checks out again what was checked out before (`undo.last`).
- **Settings**: `branches.openIn`, through `branch.open`; `submodules.updateOnCheckout`.
- **Uses**: `branch.open`, `submodule.update-on-checkout`, `worktree.open`.

The branch is given by its bare name: `git checkout refs/heads/<branch>` would leave `HEAD` detached. Git refuses when
files not committed would be overwritten, and says which; the working tree is then as it was.

Acceptance:

- [ ] Double clicking `feature` in the branch rail runs `git checkout feature`, and the label of `feature` is the filled
      one after.
- [ ] With `branches.openIn` set to `worktree`, the double click runs `worktree.open-in-worktree` and no checkout.
- [ ] The menu of the branch that is checked out has no **Check out** entry.
- [ ] The menu of a branch another worktree has checked out has the entries of that worktree and no **Check out**.
- [ ] With a submodule in the repository and `submodules.updateOnCheckout` on, the checkout is followed by
      `git submodule update --init --recursive`; with it off, by nothing.
- [ ] Undo right after goes back to the branch checked out before.
- [ ] Extension: **Check out feature** on the branch `feature` runs `git checkout feature` in the engine, and the
      **Gitardo** output channel has `> git checkout feature`.
- [ ] Extension: with `gitardo.commandPreview` `off` and no submodules, nothing is asked; with `all`, **Check out
      feature?** shows `git checkout feature` before it runs.
- [ ] Extension: the menu of the branch checked out has no **Check out** entry, and a double click on it does nothing.
- [ ] Extension: with submodules and `gitardo.submodules.updateOnCheckout` on, the log has `git submodule update --init
      --recursive` after the checkout; a submodule cloned from a folder on disk is asked first, and the update runs
      again with `-c protocol.file.allow=always`.
- [ ] Extension: with `gitardo.engine.path` naming a missing file, **Check out** shows **Gitardo's engine is not
      available on this system.** with **Show the log**, and nothing of this file runs.

### `branch.open` — What a double click on a branch does

Decides what a double click on a local or a remote branch does, wherever it is listed or labels its commit, and which
entry comes first in its context menu.

- **Kind**: Derivation
- **Function**: `model::branch::open(input: &OpenBranchInput) -> Option<Intent>`
- **Entry points**: a double click on a branch or a remote branch in the branch rail, in its **Recent** and
  **Favorites** lists too, and on its label in the timeline.
- **Extension**: the same choice, made in `openBranch` and `openRemoteBranch` of `src/webview/menus.ts` for a double
  click in the references tree, the Branches view and on a label of the graph, and in the menus, which offer the entries
  of the other worktree in place of **Check out**; `gitardo.branches.openIn` is the setting. The checkout it leads to is
  sent to the engine as its card says; opening a worktree stays the extension's.
- **Settings**: `branches.openIn`: `checkout` (the default) or `worktree`.
- **Uses**: `branch.checkout`, `branch.checkout-remote`, `worktree.open`, `worktree.open-in-worktree`.

Rules, for a local branch:

- Checked out here: nothing.
- Checked out in another worktree: that worktree opens (`worktree.open`), since git checks it out nowhere else; when
  that worktree's folder is missing, nothing.
- Otherwise, with `branches.openIn` set to `checkout`: `branch.checkout`; set to `worktree`: `worktree.open-in-worktree`,
  which makes a worktree for it first when it has none.

For a remote branch, the local branch of its name (the remote branch without its remote) decides:

- Checked out in another worktree: that worktree opens, or nothing when its folder is missing.
- Otherwise, with `checkout`: `branch.checkout-remote`; with `worktree`: `worktree.open-in-worktree` on a local branch
  that tracks it.

In the context menu, the entry of what a double click does comes first: **Check out** then **Open in worktree** with
`checkout`, the other way round with `worktree`. Both are there whatever the setting says.

Acceptance:

- [ ] A double click on the branch checked out does nothing.
- [ ] With `branches.openIn` set to `worktree`, a double click on `feature` opens its worktree, and the menu of
      `feature` starts with **Open in worktree**.
- [ ] A double click on `origin/feature` while another worktree has `feature` opens that worktree, whatever the setting.
- [ ] A double click on a branch whose worktree's folder was deleted does nothing.
- [ ] Extension: the menu of a branch another worktree has offers **Open worktree in new window** and no **Check out**
      entry; a double click opens that worktree.
- [ ] Extension: with another worktree on `dev`, the menu of `origin/dev` has no **Check out** entry and a double click
      opens that worktree.

### `branch.checkout-remote` — Check out a remote branch

Checks out a remote branch in the local branch of its name: a new one that tracks it, or the one there is, brought up
to it.

- **Kind**: Operation
- **Function**: `features::branch::checkout_remote(context, answers) -> Outcome`
- **Entry points**: **Check out as local branch** in the context menu of a remote branch with no local branch of its
  name, **Check out `<name>`** when there is one; a double click on a remote branch, as `branch.open` decides.
- **Extension**: in the context menu of a remote branch, **Check out as local branch** when no local branch has its name
  and **Check out `<branch>`** when one has; a double click on it in the references tree, the Branches view or on its
  label in the graph (`branch.open`). Each sends `feature/run` of `branch.checkout-remote` to Gitardo's engine
  (`Controller.runOnEngine` in `src/extension/controller.ts`, `engineRuns` in `src/engine/actions.ts`), with the remote
  branch as its target; its question is a quick pick. The menu and the double click open the worktree that has the local
  branch instead, so the engine's refusal is met only by a caller that goes past them (B6). When the engine's binary is
  missing, it says so (D49).
- **Offered when**: no other worktree has the local branch of its name checked out; when one has, its menu and a double
  click open that worktree instead (`worktree.open`).
- **Reads**: when a local branch has its name, how the two stand:
  `git rev-list --left-right --count refs/heads/<name>...<remote branch> --`, the commits each has that the other has
  not.
- **Asks**: only when each has commits the other has not, a pick titled **`<name>` has N commits that `<remote branch>`
  has not, which has M commits of its own**: **Check out `<name>` and rebase it onto `<remote branch>`**, described
  **Replay its N commits on top**, or **Check out `<name>` as it is**, described **Leave the two apart**.
- **Plan**:
  - No local branch of its name: `git checkout --track <remote branch>`, which makes the local branch `<name>`
    following it.
  - The local branch has nothing of its own and is behind: `git checkout <name>`, unless it is checked out already,
    then `git merge --ff-only <remote branch>`.
  - The local branch has commits of its own and nothing to take, or it is chosen **as it is**:
    `git checkout <name>`.
  - Chosen to rebase: `git rebase --autostash <remote branch> <name>`, which checks the branch out first and puts aside
    what is not committed while it rebases.
  - Then the update of the submodules (`submodule.update-on-checkout`).
- **Danger**: None, or Confirm when the update of the submodules would leave commits behind.
- **Undo**: By the reflog: Undo checks out again what was checked out before, or, after the fast-forward or the rebase,
  moves the branch back (`undo.last`).
- **After**: when the local branch is checked out already and has everything the remote branch has, nothing runs and
  Gitardo says "`<name>` is checked out already, with everything `<remote branch>` has."
- **Refuses**: when the worktree that has the local branch is missing: "The worktree of `<name>` is missing from
  `<path>`: prune the worktrees to free the branch."
- **Settings**: `branches.openIn`, through `branch.open`; `submodules.updateOnCheckout`.
- **Uses**: `branch.open`, `branch.ref-argument`, `submodule.update-on-checkout`, `worktree.open`, `rebase.`.

The name of the local branch is the remote branch's without its remote, the longest remote name that matches first:
`origin/feature/login` gives `feature/login`, and with remotes `a` and `a/b`, `a/b/x` gives `x`. A remote branch that
has a namesake is named in full, `refs/remotes/<remote>/<name>`.

Acceptance:

- [ ] With no local `feature`, checking out `origin/feature` runs `git checkout --track origin/feature`, and `feature`
      follows `origin/feature` after.
- [ ] With a local `feature` two commits behind and nothing of its own, it runs `git checkout feature` and
      `git merge --ff-only origin/feature`.
- [ ] With `feature` checked out and behind, it runs only `git merge --ff-only origin/feature`.
- [ ] With `feature` one commit ahead and two behind, it asks; **as it is** runs `git checkout feature`, the rebase runs
      `git rebase --autostash origin/feature feature`.
- [ ] With `feature` checked out, ahead and behind, **as it is** runs nothing.
- [ ] With `feature` checked out and level with `origin/feature`, nothing runs and the message says so.
- [ ] Extension: with no local `dev`, **Check out as local branch** on `origin/dev` runs `git checkout --track
      origin/dev`.
- [ ] Extension: with a local `dev` 2 commits behind `origin/dev` and none of its own, **Check out dev** runs `git
      checkout dev` and `git merge --ff-only origin/dev`.
- [ ] Extension: with `dev` 1 ahead and 2 behind, a double click on `origin/dev` asks in a quick pick, and **Check out
      dev as it is** runs `git checkout dev` alone, followed by the update of the submodules that `dev` records.

### `branch.checkout-remote-and-rebase` — Check out a branch and rebase it onto its remote branch

Checks out the local branch of a remote branch's name and replays its own commits on top of the remote branch, in one
step.

- **Kind**: Operation
- **Function**: `features::branch::checkout_remote_and_rebase(context, answers) -> Outcome`
- **Entry points**: **Check out `<name>` and rebase it onto this** in the context menu of a remote branch.
- **Extension**: **Check out `<branch>` and rebase it onto this** in the context menu of a remote branch, offered when
  the local branch follows it, is not checked out, and each has commits the other has not; the choice **Check out
  `<branch>` and rebase it onto `<remote branch>`** in the question of `branch.checkout-remote`. Each sends
  `feature/run` of `branch.checkout-remote-and-rebase` to Gitardo's engine (`Controller.runOnEngine` in
  `src/extension/controller.ts`, `engineRuns` in `src/engine/actions.ts`). When the engine's binary is missing, it says
  so (D49).
- **Offered when**: the local branch `<name>` follows this remote branch, the upstream is not gone, each has commits the
  other has not as of the last fetch, the local branch is not checked out (there, `rebase.` rebases what is checked
  out), and no other worktree has it.
- **Plan**: `git rebase --autostash <remote branch> <name>`, then the update of the submodules
  (`submodule.update-on-checkout`).
- **Danger**: None, or Confirm when the update of the submodules would leave commits behind.
- **Undo**: By the reflog: Undo moves the branch back to where it was before the rebase (`undo.last`).
- **On failure**: when the rebase stops on a conflict, the conflicts open as for any rebase (`conflicts.`).
- **Uses**: `branch.checkout-remote`, `submodule.update-on-checkout`.

Acceptance:

- [ ] With `feature` following `origin/feature`, one commit ahead and two behind, the entry is there and runs
      `git rebase --autostash origin/feature feature`; `feature` is checked out after, its commit on top.
- [ ] The entry is not there when `feature` is only behind, or is checked out.
- [ ] Extension: **Check out dev and rebase it onto this** runs `git rebase --autostash origin/dev dev` without a
      question, and afterwards `dev` has its own commit on top of those of `origin/dev`.

### `branch.checkout-detached` — Check out a commit

Checks out a commit on a detached `HEAD`, to look at it or build it.

- **Kind**: Operation
- **Function**: `features::branch::checkout_detached(context, answers) -> Outcome`
- **Entry points**: **Check out commit (detached)** in the context menu of a commit in the timeline, narrowed or not.
- **Extension**: **Check out commit (detached)** in the context menu of a commit in the graph and in the Commits view,
  and **Check out (detached)** for a position picked in **Gitardo: Browse Reflog**. Each sends `feature/run` of
  `branch.checkout-detached` to Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`,
  `engineRuns` in `src/engine/actions.ts`), with the commit as its target. When the engine's binary is missing, it says
  so (D49).
- **Offered when**: the commit is not the one checked out.
- **Plan**: `git checkout --detach <hash>`, then the update of the submodules (`submodule.update-on-checkout`).
- **Danger**: None, or Confirm when the update of the submodules would leave commits behind.
- **Undo**: By the reflog: Undo checks out again what was checked out before (`undo.last`).
- **Uses**: `submodule.update-on-checkout`.

Acceptance:

- [ ] On a commit that is not `HEAD` it runs `git checkout --detach <full hash>`, and the header says
      **detached at `<short hash>`** after.
- [ ] The menu of the commit checked out has no such entry.
- [ ] Extension: **Check out commit (detached)** on a commit runs `git checkout --detach <full hash>` and the toolbar
      then says **detached at `<short hash>`**.
- [ ] Extension: neither entry is offered on the commit checked out.

### `branch.create` — Create a branch

Makes a new branch at a commit, a branch or a tag, and checks it out or not.

- **Kind**: Operation
- **Function**: `features::branch::create(context, answers) -> Outcome`
- **Entry points**: **Branch** in the **…** menu of the header, whose tooltip is **Branch: create a branch at HEAD**;
  **Create branch…** in Quick search, at `HEAD`; **Create branch here…** in the context menu of a commit in the
  timeline, narrowed or not, of a local branch and of a tag; the button **Create branch** at the top of the
  branch rail.
- **Extension**: **Branch** in the toolbar of the graph (tooltip **Branch: create a branch at HEAD**), **Gitardo: Create
  Branch…** in the Command Palette and on the title of the Branches view, all at `HEAD`; **Create branch here…** in the
  context menu of a commit (graph and Commits view), of a local branch, of a tag, and of a position picked in **Gitardo:
  Browse Reflog**. Each sends `feature/run` of `branch.create` to Gitardo's engine (`Controller.runOnEngine` in
  `src/extension/controller.ts`, the target from `targetOf` in `src/engine/target.ts`); its questions are VS Code's
  input boxes and quick picks, the name checked with `question/validate` as it is typed, and the plan shown in a modal
  dialog when `gitardo.commandPreview` is `all` (`VscodeAsker` in `src/engine/vscode.ts`). The settings are
  `gitardo.branches.format` and `gitardo.branches.inputs`, or, where they are empty, `gitardo.worktrees.branchFormat`
  and `gitardo.worktrees.inputs`, as before (`src/engine/settings.ts`); the inputs used last are kept in the workspace
  state `worktreeInputs`. When the engine's binary is missing, it says so (D49).
- **Offered when**: there is a commit: **Branch** in the header is disabled before the first one.
- **Asks**:
  1. With `branches.format` set: a pick titled **Create branch**, **Use the format** (the format beside it), which is
     where the pick starts, or **Type a branch name**. With the format, the questions of `branch.name-from-format`,
     and then the name it makes, in the input box **Name of the new branch**, to change or keep.
  2. Without a format, or after **Type a branch name**: the name, in an input box titled **Create branch** with the
     placeholder `feature/my-change`.
  3. A pick titled **Create branch `<name>`**: **Create and check out**, or **Create only**, described **Stay on the
     current branch**.

  The name is checked while it is typed: `branch.validate-name`, and "A branch with that name already exists".
- **Plan**: `git checkout -b <name> <start>` to check it out, then the update of the submodules
  (`submodule.update-on-checkout`), and the new branch is selected; `git branch <name> <start>` to create it only. `<start>` is `HEAD` from the
  header and Quick search, the full hash on a commit, and the branch or the tag as `branch.ref-argument` names it.
- **Danger**: None, or Confirm when the update of the submodules would leave commits behind.
- **Undo**: Checked out: by the reflog, Undo goes back to what was checked out before; the branch stays, and
  `branch.delete` removes it with nothing lost. Created only: from the journal, Undo deletes it.
- **Settings**: `branches.format`, `branches.inputs` (see `branch.format-name`); `submodules.updateOnCheckout`.
- **Uses**: `branch.name-from-format`, `branch.validate-name`, `branch.ref-argument`, `submodule.update-on-checkout`.

A new branch has no upstream until its first push (`sync.push`).

Acceptance:

- [ ] From the header, with no format, typing `fix/login` and **Create and check out** runs
      `git checkout -b fix/login HEAD`.
- [ ] On a commit, **Create only** runs `git branch <name> <full hash>` and leaves the branch checked out as it was.
- [ ] On the branch `release` when a tag `release` also exists, it runs `git branch <name> refs/heads/release`.
- [ ] Typing `main` when `main` exists shows "A branch with that name already exists" and the box cannot be accepted.
- [ ] Typing `bad name` shows "That is not a valid Git ref name".
- [ ] With a format set, the first question is the pick between **Use the format** and **Type a branch name**.
- [ ] **Create and check out** where the start of the branch or `HEAD` has submodules, with
      `submodules.updateOnCheckout` on, ends with `git submodule update --init --recursive`; with it off, or with
      **Create only**, nothing is read and nothing is updated.
- [ ] Backing out of any question runs nothing.
- [ ] In the extension, **Create branch here…** on a commit asks the name, refuses `feature x` with the engine's
      sentence while it is typed, refuses a name taken, creates the branch with the commands the engine logs in the
      **Gitardo** output channel, and the graph shows it.
- [ ] In the extension, with `gitardo.branches.format` set, the name is proposed from the format and the inputs given
      are remembered in `worktreeInputs`.
- [ ] In the extension, **Undo** right after **Create and check out** goes back to the branch before, by the reflog.
- [ ] In the extension, with `gitardo.engine.path` naming a missing file, **Create branch here…** shows **Gitardo's
      engine is not available on this system.** with **Show the log**, and no branch is created.
- [ ] The same answers through `gitardo-cli` and through the extension give the same plan and the same repository.

### `branch.format-name` — Expand the format of branch names

Makes the name of a new branch from `branches.format` and the values of its parts, and says what is wrong with a format
before anything is asked.

- **Kind**: Derivation
- **Function**: `model::branch::format_name(format: &str, values: &FormatValues) -> String`, with
  `model::branch::format_problem(format: &str, inputs: &[BranchInput]) -> Option<Text>`
- **Settings**: `branches.format`, such as `PRJ-${input:ticket}-${input:kind}-${base}`; `branches.inputs`, a list of
  `{ "id", "type": "promptString" | "pickString", "description"?, "default"?, "pattern"?, "options"?, "allowCustom"? }`.
  Both can be set in the repository's `.gitardo/settings.json`, for everyone who works on it.
- **Uses**: by `branch.name-from-format`.

Rules:

- `${input:<id>}` is the value given for the input `<id>`, wherever it appears, as many times as it appears.
- `${base}` is the branch the new one starts from, without its remote: `origin/feature/login` gives `feature/login`, the
  longest remote name that matches taken off. What it is, is said by `branch.name-from-format`.
- `${repo}` is the name of the folder of the main worktree of the repository, whichever worktree the window shows.
- `${date}` is today in the local time of the computer, as `2026-10-15`.
- The rest of the format is kept as it is written.
- Problems, said before anything is asked, with "Change it in the settings." after them: a `${…}` that is none of
  these, "The format uses `${x}`, which is not one of `${base}`, `${repo}`, `${date}` or `${input:<id>}`."; an input
  that `branches.inputs` does not define, "The format uses `${input:x}`, which `branches.inputs` does not define."

Acceptance:

- [ ] `PRJ-${input:ticket}-${input:kind}-${base}` with ticket `123`, kind `fix` and base `origin/feature/login` makes
      `PRJ-123-fix-feature/login`.
- [ ] `${repo}/${date}` in the worktree `app.worktrees/x` of the repository `app` on 15 October 2026 makes
      `app/2026-10-15`.
- [ ] `${user}` in the format is a problem that names `${user}`; `${input:ticket}` with no input `ticket` defined is a
      problem that names it.
- [ ] A format with no `${…}` makes itself.

### `branch.name-from-format` — Ask for the parts of the format

What is asked, in order, to fill the format of branch names in, and the values proposed; the name it makes is then
shown, to change or keep, by the feature that asked.

- **Kind**: Derivation
- **Function**: `model::branch::name_from_format(input: &FormatAsking, answers: &Answers) -> Result<FormatName, Outcome>`,
  which gives the name once every part is known, with the values given; until then the next question, the read of the
  branches that have a commit, or why the format is refused
- **Entry points**: the questions of `branch.create` and of `worktree.create` after **Use the format**.
- **Extension**: the questions are VS Code's input boxes and quick picks, asked by the engine for `branch.create` and
  for `worktree.create`. The settings are `gitardo.branches.format` and `gitardo.branches.inputs`, or
  `gitardo.worktrees.branchFormat` and `gitardo.worktrees.inputs` where those are not set; the values used last are kept
  in the workspace state under `worktreeInputs`.
- **Reads**: for `${base}` at a commit, the branches that have it:
  `git for-each-ref --contains <hash> --format=%(refname) refs/heads refs/remotes`, leaving out `…/HEAD`.
- **Settings**: `branches.format`, `branches.inputs`.
- **Uses**: `branch.format-name`.

Rules:

- A format with a problem (`branch.format-name`) is refused before any question.
- The inputs are asked in the order they first appear in the format, each once, with the description of the input as
  the prompt, or its id when it has none.
- `promptString`: an input box, filled with the value proposed. It cannot be empty, and when the input has a `pattern`,
  a regular expression, what is typed must match it: "It has to match `<pattern>`". A pattern that is not a valid
  regular expression is refused: "The pattern of the input `<id>` in `branches.inputs` is not a valid regular
  expression."
- `pickString`: a pick of its `options`, the value proposed first. A value proposed that is not among the options is
  listed first, described **Used last**, only when the input has `allowCustom`. With `allowCustom`, the last entry,
  **Type another value…**, opens an input box that cannot be left empty.
- The value proposed is the one used last for that input, in this workspace, or the input's `default` before any was
  used. The values given are remembered once all the inputs of the format have been answered and the plan of the
  feature that asked runs.
- `${base}`, only when the format has it: from the header and Quick search, the branch checked out; on a branch, that
  branch; on a remote branch, its name without its remote; on a tag, the tag. A branch or a tag named in full because
  of a namesake is matched by its short name.
- `${base}` at a commit, and on a detached `HEAD`, is asked in a pick titled **Create branch: what ${base} is**: the
  branches at that commit, described **Branch at this commit**; the commit itself, as its short hash, described **This
  commit**; then the other branches that have it, described **Branch that has this commit**. Remote branches are listed
  without their remote, each name once, and no more than 50 of them.
- Backing out of any question stops it: nothing is made.

Acceptance:

- [ ] `${input:a}-${input:a}` asks `a` once.
- [ ] With `"pattern": "^\d+$"`, typing `12a` cannot be accepted.
- [ ] The second time, the input box starts with the value given the first time.
- [ ] A `pickString` with `allowCustom` whose last value was typed lists that value first with **Used last**.
- [ ] From the header on `main`, `${base}` is `main` and nothing is asked for it.
- [ ] On a commit that `main` points at and `feature` contains, the pick lists `main`, then the short hash, then
      `feature`.
- [ ] A format without `${base}` asks nothing about it, also at a commit.
- [ ] Extension: with the format `${input:kind}/${base}-${input:ticket}`, kind `feat`, ticket `1`, from the toolbar on
      `main`, the name proposed is `feat/main-1`.
- [ ] Extension: with only `gitardo.worktrees.branchFormat` set to `old-${input:ticket}` and its inputs, the name is
      `old-3` for ticket `3`.

### `branch.validate-name` — What git does not take as a name

Says, while a name is typed, what keeps it from being a branch, a tag or a remote. Git has the final say; this catches
the common mistakes early.

- **Kind**: Derivation
- **Function**: `model::branch::validate_name(name: &str) -> Option<Text>`
- **Uses**: in `branch.create`, `branch.rename`, `tag.create`, `remote.add`, `remote.rename`, `stash.branch`,
  `worktree.create`.

Rules:

- An empty name: "Enter a name".
- A name that starts with `-`, `.` or `/`; has a space or another white space, `~`, `^`, `:`, `?`, `*`, `[` or `\`;
  has `..` or `@{`; ends with `/` or `.`; or ends with `.lock`: "That is not a valid Git ref name".
- Anything else: no problem. Each question adds the names already taken: "A branch with that name already exists",
  "A tag with that name already exists", "A remote with that name already exists".

Acceptance:

- [ ] `feature/login`, `v1.0.0` and `fix-123` have no problem.
- [ ] `-x`, `.x`, `a..b`, `a b`, `a:b`, `a@{1}`, `x/`, `x.lock` each have "That is not a valid Git ref name".
- [ ] `` (empty) has "Enter a name".

### `branch.ref-argument` — A branch named in full when it has a namesake

How a branch, a remote branch or a tag is named to git where git expects a commit, so that a namesake is not taken for
it.

- **Kind**: Derivation
- **Function**: `model::branch::ref_argument(kind: RefKind, name: &str, refs: &RefNames) -> String`

Rules:

- When the name is used by more than one of the local branches, the remote branches and the tags, it is given in full:
  `refs/heads/<name>`, `refs/remotes/<name>` or `refs/tags/<name>`. Of a branch and a tag both called `release`, git
  takes the tag, whichever was meant, and refuses some commands as ambiguous.
- Otherwise the short name is given.
- It is used where a branch starts a new one, is merged, rebased onto, compared, pushed, and where a tag or a remote
  branch is checked out. A local branch that is checked out is always given by its bare name to `git checkout`.

Acceptance:

- [ ] With a branch `release` and a tag `release`, the branch is `refs/heads/release` and the tag `refs/tags/release`.
- [ ] With a local `main` and a remote `origin/main`, the branch is `main`: their names differ.
- [ ] Pushing `release` with a tag of that name runs `git push origin refs/heads/release`.

### `branch.rename` — Rename a branch

Gives a local branch another name.

- **Kind**: Operation
- **Function**: `features::branch::rename(context, answers) -> Outcome`
- **Entry points**: **Rename…** in the context menu of a local branch; **Rename branch…** when another worktree has it
  checked out, beside **Rename worktree…** (`worktree.rename`).
- **Extension**: **Rename…** in the context menu of a local branch, **Rename branch…** when another worktree has it.
  Each sends `feature/run` of `branch.rename` to Gitardo's engine (`Controller.runOnEngine` in
  `src/extension/controller.ts`, `engineRuns` in `src/engine/actions.ts`); the name is asked in an input box, checked
  with `question/validate` as it is typed. When the engine's binary is missing, it says so (D49).
- **Asks**: the new name, in an input box titled **Rename branch `<name>`**, starting from the name it has; checked by
  `branch.validate-name` and "A branch with that name already exists".
- **Plan**: `git branch -m <old> <new>`.
- **Danger**: None.
- **Undo**: Not taken back by Undo: renaming it back does.
- **Uses**: `branch.validate-name`.

Git renames the branch's configuration with it, its upstream included; the upstream keeps its own name.

Acceptance:

- [ ] Renaming `feature` to `feature/login` runs `git branch -m feature feature/login`, and the branch rail lists
      `feature/login` with the same upstream after.
- [ ] Accepting the box with the old name unchanged is refused by the box: that name is taken.
- [ ] Renaming the branch checked out keeps it checked out.
- [ ] Extension: **Rename…** on `feature`, `done` typed, runs `git branch -m feature done` in the engine, and the branch
      is listed as `done` with the same commit.

### `branch.delete` — Delete a branch

Deletes a local branch, after saying whether its commits are merged, so that none is lost without knowing.

- **Kind**: Operation
- **Function**: `features::branch::delete(context, answers) -> Outcome`
- **Entry points**: **Delete** in the context menu of a local branch.
- **Extension**: **Delete** in the context menu of a local branch that is not checked out and that no other worktree has
  (that one offers **Remove worktree and delete branch…**, `worktree.remove`). Each sends `feature/run` of
  `branch.delete` to Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`, `engineRuns` in
  `src/engine/actions.ts`); the confirmation is a modal dialog, a warning when the branch is not merged. When git still
  finds it not fully merged, the error has **Delete anyway** beside **Show Log**, the `offerRun` follow-up of protocol
  1.3, which runs `branch.delete` again told it is not merged. When the engine's binary is missing, it says so (D49).
- **Offered when**: the branch is not checked out here, and no other worktree has it checked out; there, **Remove
  worktree and delete branch…** does both (`worktree.remove`).
- **Reads**: whether its commits are merged: `git merge-base --is-ancestor refs/heads/<name> <into>`, where `<into>` is
  its upstream, `refs/remotes/<upstream>`, when it has one that is not gone, and `HEAD` otherwise. Code 0 is merged.
- **Plan**: merged, `git branch -d <name>`; not merged, `git branch -D <name>`. When `git branch -d` still refuses with
  "not fully merged", **Delete anyway** offers the plan with `-D`, which is asked, as any plan is.
- **Danger**: merged, Confirm, saying "Its commits are merged, so none is lost. Undo brings the branch back."; not
  merged, Destructive, because "This branch is not fully merged. Commits that are only on it will no longer be reachable
  from any branch."
- **Undo**: Gitardo remembers the branch and its commit: Undo right after, while nothing else has moved `HEAD`, asks and
  runs `git branch <name> <hash>`. Its upstream is not set again.
- **Uses**: `undo.last`, `worktree.remove`.

Acceptance:

- [ ] A branch merged into its upstream is deleted with `git branch -d`, after a confirmation that says it is merged.
- [ ] A branch with no upstream and a commit that `HEAD` does not have is deleted with `git branch -D`, after the
      warning, also with `commandPreview` set to `off`.
- [ ] Undo right after runs `git branch <name> <hash>` and the branch is back at the same commit.
- [ ] After another commit, Undo takes back that commit instead.
- [ ] The branch checked out has no **Delete** entry.
- [ ] When `git branch -d` says the branch is not fully merged, **Delete anyway** is offered and plans
      `git branch -D <name>` with the warning.
- [ ] Extension: **Delete** on a merged branch `topic` asks **Delete branch topic?**, whose detail says **none is lost**
      and ends with `git branch -d topic`, whatever `gitardo.commandPreview` says.
- [ ] Extension: **Delete** on a branch with commits of its own asks once, with a warning, **Delete branch feature?**,
      whose detail says **not fully merged** and ends with `git branch -D feature`.
- [ ] Extension: when `git branch -d` is refused with *not fully merged*, the error offers **Delete anyway**, which asks
      with the warning and runs `git branch -D <name>`.
- [ ] Extension: **Undo** right after it asks, as a question and not a warning, and the branch is back at the same
      commit.
- [ ] Extension: the branch checked out, and a branch another worktree has, have no **Delete** entry.

### `branch.clean-up` — Clean up branches

Lists the local branches that are done with, merged or left without their upstream, and deletes the ones ticked, with
the worktrees that have them.

- **Kind**: Operation
- **Function**: `features::branch::clean_up(context, answers) -> Outcome`
- **Entry points**: **Clean up branches** in Quick search; **Clean up branches…** in the **…** menu of the header.
- **Extension**: **Gitardo: Clean Up Branches** in the Command Palette and in the **…** of the Branches view; **Clean up
  branches…** in the **…** menu of the toolbar of the graph. Each sends `feature/run` of `branch.clean-up` to Gitardo's
  engine (`Controller.runOnEngine` in `src/extension/controller.ts`, `Controller.cleanupBranches`); the pick is a quick
  pick of several, the merged branches ticked. Once the plan is confirmed, and before its steps, VS Code's Git extension
  is told to close each worktree the plan removes (`git.close`), which Windows would not let git delete otherwise
  (`HostHooks.confirmed`, `Controller.closeRemoved`). When the engine's binary is missing, it says so (D49).
- **Reads**: the branches merged into what is checked out, `git branch --merged HEAD --format=%(refname:short)`; the
  branches whose upstream is gone, and the worktrees with how many files each has not committed, from the state of the
  repository (`repo.read-state`).
- **Asks**: a pick of several, titled **Clean up branches**, with the placeholder **Branches to delete, with their
  worktrees**. Listed: every local branch that is merged into `HEAD` or whose upstream was deleted from the remote,
  except the one checked out here and the one of the main worktree when this window is another worktree. Each says
  why, **merged into `<branch>`** or **upstream is gone, not merged**, and, when another worktree has it, **with its
  worktree `<name>`**, followed by **N files not committed** when it has some, or **with its missing worktree**.
  Ticked to start with: the merged branches whose worktree, if any, has nothing uncommitted.
- **Plan**, in this order:
  1. `git worktree prune`, when a worktree picked is missing.
  2. For each other worktree picked, the steps of `worktree.remove`: `unlink <path>` for each link to a
     folder outside it, then `git worktree remove <path>`, with `--force` when it has files not committed, and
     `delete <path>` for what Windows kept from being deleted.
  3. `git branch -d <merged…>`, the merged branches in one command.
  4. `git branch -D <not merged…>`, the others in one command.
  5. `delete <folder>` for each folder of the worktrees left empty.
- **Danger**: Confirm; Destructive when a branch picked is not merged ("Not merged into the current branch, so their
  commits may become unreachable: `<branches>`") or a worktree picked has files not committed ("These worktrees have
  files not committed, which are deleted with them: `<paths>`").
- **Undo**: Impossible: the reflog keeps no record of deleted branches, and Gitardo takes back only a branch deleted on
  its own (`branch.delete`). The command log has each command.
- **Refuses**: when no branch is merged or has lost its upstream, before asking anything: "No local branches are merged
  or have lost their upstream."; when the pick is accepted with nothing ticked: "No branches selected."
- **Uses**: `worktree.remove`, `worktree.prune`.

The title of the plan counts what goes: "Delete 3 branches and 1 worktree".

Acceptance:

- [ ] With `a` merged and `b` whose upstream is gone and not merged, both are listed, `a` ticked and `b` not.
- [ ] Ticking both runs `git branch -d a` and `git branch -D b`, after a warning that names `b`.
- [ ] A merged branch checked out in another worktree with no changes is ticked, and its worktree is removed with
      `git worktree remove <path>` before `git branch -d`.
- [ ] A worktree with 2 files not committed is listed with **2 files not committed**, unticked, and ticking it adds
      `--force` and the warning.
- [ ] The branch checked out is never listed; in a window on a linked worktree, the branch of the main worktree is not
      listed either.
- [ ] With nothing to clean up, no pick opens and the message says so.
- [ ] The links of a worktree to folders outside it are unlinked before `git worktree remove`, so that the
      folders they lead to keep their files; the folder of the worktrees it leaves empty is deleted last.
- [ ] Extension: a merged branch `merged` with a worktree is listed as **merged into main, with its worktree merged**,
      ticked; the worktree is removed with `git worktree remove <path>` before `git branch -d merged`, and its folder is
      gone.
- [ ] Extension: with nothing to list, the engine says so and nothing runs.

### `branch.fast-forward` — Fast-forward a branch that is not checked out

Brings a local branch up to its upstream without switching to it: the files and the branch checked out are not touched.

- **Kind**: Operation
- **Function**: `features::branch::fast_forward(context, answers) -> Outcome`
- **Entry points**: **Fast-forward to `<upstream>`** in the context menu of a local branch.
- **Extension**: **Fast-forward to `<upstream>`** in the context menu of a local branch, in the references tree, the
  Branches view and on its label in the graph, offered as `canFastForward` in `src/shared/refs.ts` decides. Each sends
  `feature/run` of `branch.fast-forward` to Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`,
  `engineRuns` in `src/engine/actions.ts`), with the branch as its target. When the engine's binary is missing, it says
  so (D49).
- **Offered when**: the branch is not checked out here; it has an upstream that is not gone; it has no commits of its
  own as of the last fetch; and, when another worktree has it checked out, that worktree's folder is there and it has
  no file not committed. It is offered however far behind the last fetch says it is: the fetch finds out.
- **Plan**:
  - Checked out nowhere: `git fetch <remote> refs/heads/<remote branch>:refs/heads/<branch>`, which fetches the
    upstream and moves the branch to it in one step. Both are named in full, so that a tag of the same name is not
    taken for either. A branch that follows another local branch has `.` for its remote, and is fetched from this
    repository the same way.
  - Checked out in another worktree:
    `git -C <worktree> pull --ff-only --no-rebase --no-autostash <remote> refs/heads/<remote branch>`, so that its
    files move with it. It never rebases or puts anything aside, whatever `pull.rebase` and the configuration of git
    say.
- **Danger**: None.
- **Undo**: Not taken back by Undo, which follows what is checked out here; the branch's own reflog has where it was
  (`reflog.`).
- **On failure**: git refuses when the remote branch has moved in a way that is not a fast-forward since the last
  fetch, and says why; nothing changes.
- **Refuses**: when the branch no longer meets the conditions above: "`<branch>` cannot be fast-forwarded: it needs an
  upstream, no commits of its own, and not to be checked out here or in a worktree with changes not committed."

For a branch with commits of its own, check it out and pull.

Acceptance:

- [ ] `feature`, behind `origin/feature` and with nothing of its own, runs
      `git fetch origin refs/heads/feature:refs/heads/feature`, and points at the commit of `origin/feature` after, with
      the files of the working tree unchanged.
- [ ] A branch `main` that follows `origin/trunk` runs `git fetch origin refs/heads/trunk:refs/heads/main`.
- [ ] A branch with one commit of its own has no such entry.
- [ ] A branch checked out in another worktree with no changes runs the `git -C <worktree> pull --ff-only …` command,
      and that worktree's files are at the new commit after.
- [ ] A branch checked out in another worktree with a file not committed has no such entry.
- [ ] When the remote branch was force-pushed since the last fetch, git's refusal is shown and the branch has not
      moved.
- [ ] Extension: **Fast-forward to up/main** on `feature` runs `git fetch up refs/heads/main:refs/heads/feature` in the
      engine, and the branch checked out, `HEAD` and the files are as they were.
- [ ] Extension: with a clean worktree on `feature`, it runs `git -C <worktree> pull --ff-only --no-rebase
      --no-autostash up refs/heads/main`.

### `branch.pane` — The branch rail, its buttons and its comparison

The branch rail as one place, the legend and the filter of the timeline: its buttons, in the heading of **Branches** of
the side column; the recent branches and the favorites, the local branches, the branches of each remote under a heading
of its own, and the stashes, worktrees, tags and submodules kept aside, with their context menus (`refs.tree`); and the
comparison of a reference with the branch checked out.

- **Kind**: View
- **Function**: `views::branch::pane(state: &ReferencesState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the left of the window; **Show branches** in Quick search shows the branch rail when it was put
  away.
- **Settings**: `branches.sortOrder`.
- **Extension**: the references beside the graph and the Branches view, whose header names the branch checked out and
  has its buttons. They have no top of their own: the funnel of `refs.list-filter` ends their top row, after the box
  that finds a name. They list every branch whatever the graph is narrowed to, the ones it leaves out dimmed. Tested by
  `test/e2e/ui.mjs`, step "the branches list the favorites and the ones checked out last first, all of them whatever
  the graph is narrowed to, what their own filter chooses, and can be put away; the status bar opens the graph".
- **Uses**: `refs.tree`, `refs.list-filter`, `filter.list-too`, `branch.create`, `compare.with-current`,
  `compare.choose`, `worktree.create`, `worktree.switch`, `worktree-setup.page`,
  `worktree-setup.bring-into-other-worktrees`, `worktree.prune`, `graph.show-in-graph`, `graph.hover-history`.

What the branch rail shows and does:

- Its buttons are on the line of the heading of **Branches** in the side column (`repo.side-column`), on its right:
  **Create branch** (`branch.create` at `HEAD`) and **Compare** (`compare.choose`), which cannot be used before the
  first commit, and the button of the rail's own filter (`refs.list-filter`), each an icon with its name in its
  tooltip. The branch checked out is not named again there: the header names it (`graph.where`). Narrowed to its
  icons, the button of the filter is at the top of the icons.
- The lists, their order, their menus, the filter box from 15 branches and tags on, and the cap of 100 entries per
  list with **Show all** are those of `refs.tree`: **Local**, then a heading for each remote (**ORIGIN**) with its
  branches, then **Kept aside** with **Stashes**, **Worktrees**, **Tags** and **Submodules**. The lists of **Kept
  aside** are closed when the window opens and the others open; the ones opened or closed stay so until it is closed.
- The title of the **Worktrees** list has **+**, which creates a worktree (`worktree.create`), and **…**, with
  **Switch worktree…**, **Choose what new worktrees get…**, **Bring ignored files into the other worktrees…**
  and, when a worktree is missing, **Prune missing worktrees**.
- A click on a branch narrows the timeline to its history and selects its tip, and the rail keeps listing what it
  listed; a second click on it brings the whole history back (`refs.tree`). A click on a tag, a stash or a worktree still selects its commit in the timeline
  (`graph.show-in-graph`). Neither opens a page. **Show in the timeline** in the context menu of a reference shows the
  whole timeline at it (`graph.show-in-graph`).
- Resting the pointer on a branch, a tag or a worktree brings out its history in the timeline while it is shown
  (`graph.hover-history`), until the pointer leaves it.
- **Compare with `<current branch>`** in the menu of a reference shows the comparison in the inspector
  (`compare.in-place`), with a button that swaps its sides and one that closes it.
- When the repository shown changes, what was selected and compared goes.

Acceptance:

- [ ] The heading of **Branches** has **Create branch**, which asks for the name of a branch at `HEAD`, and
      **Compare**, on its line, and the rail under it does not name the branch checked out again.
- [ ] With the timeline narrowed to `main`, the branch rail lists every local branch, the others dimmed.
- [ ] A click on `feature` narrows the timeline to its history and selects its tip, and opens no page; a second click
      brings the whole history back.
- [ ] Every list of **Kept aside** is closed when the window opens, and the others are open.
- [ ] The **…** of the Worktrees list has **Prune missing worktrees** only while a worktree's folder is missing.

### `branch.read-log` — Read the commits of what is checked out

The commits of `HEAD`, newest first, whatever the graph is narrowed to, and which of them are not pushed: what the
Commits view of the extension lists (Q397).

- **Kind**: Read
- **Function**: `git::read::branch::read_log(git: &Git, request: &BranchLogRequest) -> Result<BranchLog, GitError>`
- **Reads**: `git rev-parse --verify --quiet HEAD^{commit}`, which ends with 1 before the first commit and then nothing
  else is read; `git log -z` with the format of every list (`graph.read-commits`) and `--max-count=<count + 1>`, from
  `HEAD`; `git rev-parse --symbolic-full-name @{upstream}`, which fails without an upstream; `git remote`; and, when
  there is an upstream or a remote, `git rev-list --max-count=<count> HEAD --not <upstream> --`, or `--remotes` in place
  of the upstream when there is none.
- **Extension**: the Commits view of the Source Control side bar, which lists these commits with a mark on those not
  pushed.

Acceptance:

- [ ] On a branch of three commits, asked for two, the two newest are listed, newest first, and it says there are
      more; the commits of another branch are not listed.
- [ ] Before the first commit nothing is listed and nothing fails.
- [ ] With an upstream that lacks the newest commit, that commit is the one not pushed and the upstream is named by its
      short name, `origin/main`.
- [ ] Without an upstream but with a remote, the commits in no remote branch are the ones not pushed, and no upstream
      is named.
- [ ] With no remote, nothing is said of what is pushed.

### `tag.create` — Create a tag

Puts a tag on a commit: a lightweight one, or an annotated one with a message.

- **Kind**: Operation
- **Function**: `features::tag::create(context, answers) -> Outcome`
- **Entry points**: **Create tag here…** in the context menu of a commit in the timeline, narrowed or not.
- **Extension**: **Create tag here…** in the context menu of a commit in the graph and in the Commits view. Each sends
  `feature/run` of `tag.create` to Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`,
  `engineRuns` in `src/engine/actions.ts`), with the commit as its target; the name and the message are asked in input
  boxes. When the engine's binary is missing, it says so (D49).
- **Asks**:
  1. The name, in an input box titled **Create tag** with the placeholder `v1.0.0`; checked by `branch.validate-name`
     and "A tag with that name already exists".
  2. The message, in an input box titled **Create tag `<name>`**, with the prompt **Message for an annotated tag. Leave
     empty for a lightweight tag.**
- **Plan**: with a message, `git tag -a <name> -m <message> <hash>`; without, `git tag <name> <hash>`. The message
  without the spaces around it.
- **Danger**: None.
- **Undo**: Not taken back by Undo, since nothing moved `HEAD`: `tag.delete` deletes it.
- **Uses**: `branch.validate-name`.

Acceptance:

- [ ] Name `v1.0.0` and an empty message run `git tag v1.0.0 <hash>`.
- [ ] Name `v1.0.0` and the message `First release` run `git tag -a v1.0.0 -m "First release" <hash>`, and the tag is
      dated when it was made.
- [ ] Backing out of the message box runs nothing; leaving it empty and accepting makes a lightweight tag.
- [ ] Extension: `v1` with the message `First release` runs `git tag -a v1 -m "First release" <hash>`, and the tag is
      listed at that commit.
- [ ] Extension: Escape on either input box runs nothing.

### `tag.push` — Push a tag

Sends a tag to the remote.

- **Kind**: Operation
- **Function**: `features::tag::push(context, answers) -> Outcome`
- **Entry points**: **Push tag** in the context menu of a tag.
- **Extension**: **Push tag** in the context menu of a tag, offered when the repository has a remote. Each sends
  `feature/run` of `tag.push` to Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`,
  `engineRuns` in `src/engine/actions.ts`), with the tag as its target. When the engine's binary is missing, it says so
  (D49).
- **Offered when**: the repository has a remote.
- **Plan**: `git push <remote> refs/tags/<name>`, where `<remote>` is the one of `remote.default`.
- **Danger**: None.
- **Undo**: Impossible: a push changes the remote, which Undo does not take back.
- **Refuses**: "This repository has no remote." when the last remote was removed since the menu was drawn.
- **Uses**: `remote.default`.

Acceptance:

- [ ] Pushing `v1.0.0` with remotes `upstream` and `origin` runs `git push origin refs/tags/v1.0.0`.
- [ ] With the remotes `a` and `b` only, it pushes to `a`.
- [ ] A repository with no remote has no **Push tag** entry.
- [ ] Extension: **Push tag** on `v1` with the remotes `origin` and `backup` runs `git push origin refs/tags/v1`, and
      the remote then has the tag.

### `tag.delete` — Delete a tag

Deletes a tag from this repository; a remote that has it keeps it.

- **Kind**: Operation
- **Function**: `features::tag::delete(context, answers) -> Outcome`
- **Entry points**: **Delete** in the context menu of a tag.
- **Extension**: **Delete** in the context menu of a tag. Each sends `feature/run` of `tag.delete` to Gitardo's engine
  (`Controller.runOnEngine` in `src/extension/controller.ts`, `engineRuns` in `src/engine/actions.ts`), with the tag as
  its target; the warning is a modal dialog. When the engine's binary is missing, it says so (D49).
- **Plan**: `git tag -d <name>`.
- **Danger**: Destructive, because the tag and its message, if it has one, are gone from this repository: "The tag, on
  `<short hash>`, is deleted from this repository, with its message if it has one. A remote that has it keeps it."
- **Undo**: Impossible: the reflog keeps no record of tags, and Gitardo does not remember deleted ones.

Acceptance:

- [ ] Deleting `v1.0.0` asks, also with `commandPreview` set to `off`, then runs `git tag -d v1.0.0`.
- [ ] The tag is still on the remote after: no push runs.
- [ ] Extension: **Delete** on `to-delete` asks **Delete tag to-delete?** as a warning, whose detail says **A remote
      that has it keeps it.**, and the tag is gone once it runs.

### `tag.checkout` — Check out a tag

Checks out the commit of a tag on a detached `HEAD`.

- **Kind**: Operation
- **Function**: `features::tag::checkout(context, answers) -> Outcome`
- **Entry points**: **Check out tag (detached)** in the context menu of a tag.
- **Extension**: **Check out tag (detached)** in the context menu of a tag. Each sends `feature/run` of `tag.checkout`
  to Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`, `engineRuns` in
  `src/engine/actions.ts`), with the tag as its target. When the engine's binary is missing, it says so (D49).
- **Offered when**: the commit of the tag is not the one checked out.
- **Plan**: `git checkout --detach <tag>`, the tag as `branch.ref-argument` names it, then the update of the submodules
  (`submodule.update-on-checkout`).
- **Danger**: None, or Confirm when the update of the submodules would leave commits behind.
- **Undo**: By the reflog: Undo checks out again what was checked out before (`undo.last`).
- **Uses**: `branch.ref-argument`, `submodule.update-on-checkout`.

Acceptance:

- [ ] Checking out `v1.0.0` runs `git checkout --detach v1.0.0`; with a branch also called `v1.0.0`,
      `git checkout --detach refs/tags/v1.0.0`.
- [ ] A tag on the commit checked out has no such entry.
- [ ] Extension: **Check out tag (detached)** on `v1` runs `git checkout --detach v1`; with a branch also called `v1`,
      `git checkout --detach refs/tags/v1`.

### `remote.read-remotes` — Read the remotes

The remotes of the repository, each with the URL it is fetched from.

- **Kind**: Read
- **Function**: `git::read::remote::read_remotes(git: &Git, request: &RemotesRequest) -> Result<Vec<Remote>, GitError>`
- **Reads**: `git remote`, then `git remote get-url <name>` for each, in the order git lists them.

Acceptance:

- [ ] A repository with `origin` and `upstream` gives both with their URLs.
- [ ] A repository with no remote gives an empty list and no error.
- [ ] A URL changed with `insteadOf` in the configuration comes back as git uses it.

### `remote.default` — The remote a new upstream or a tag goes to

- **Kind**: Derivation
- **Function**: `model::remote::default(remotes: &[RemoteName]) -> Option<RemoteName>`
- **Uses**: in `sync.push` for a branch with no upstream, and in `tag.push`.

Rules: `origin` when the repository has a remote of that name; otherwise the first remote git lists; none when there is
no remote.

Acceptance:

- [ ] `upstream`, `origin` gives `origin`; `a`, `b` gives `a`; no remote gives none.

### `remote.manage` — Manage remotes

Lists the remotes with their URLs, to fetch one alone, change its URL, rename it, copy its URL or remove it, or to add
another.

- **Kind**: Operation
- **Function**: `features::remote::manage(context, answers) -> Outcome`
- **Entry points**: **Manage remotes** in Quick search; **Manage remotes…** in the **…** menu of the header.
- **Extension**: **Gitardo: Manage Remotes** in the Command Palette and in the **…** of the Branches view; **Manage
  remotes…** in the **…** menu of the toolbar of the graph. Each sends `feature/run` of `remote.manage` to Gitardo's
  engine (`Controller.runOnEngine` in `src/extension/controller.ts`, `Controller.manageRemotes`); its picks are quick
  picks, the remotes searched by name and URL. Once something ran, the remote that pull requests are read from is found
  again (`Controller.detectGitHub`). When the engine's binary is missing, it says so (D49).
- **Reads**: `remote.read-remotes`.
- **Asks**:
  1. A pick titled **Remotes of `<repository>`**, with the placeholder **Pick a remote to fetch, rename, repoint or
     remove it**: each remote with its URL beside it, which the typed text also matches, then **Add remote…**. It is a
     pick with a box to type in, as `worktree.switch` has.
  2. On a remote, a pick titled **`<name>`: `<url>`**: **Fetch** (described `git fetch --prune <name>`),
     **Change URL…**, **Rename…**, **Copy URL** and **Remove** (described **Deletes its remote-tracking branches
     here**).
  3. Then the questions of the feature chosen.
- **Plan**: that of `remote.fetch`, `remote.set-url`, `remote.rename`, `remote.remove` or `remote.add`. **Copy URL**
  puts the URL on the clipboard and runs nothing.
- **Danger**: that of the feature chosen.
- **Undo**: that of the feature chosen.
- **After**: once something ran, the remote of GitHub is worked out again (`github.detect-remote`) and the repository
  is read again.
- **Uses**: `remote.read-remotes`, `remote.fetch`, `remote.set-url`, `remote.rename`, `remote.remove`, `remote.add`,
  `github.detect-remote`.

Acceptance:

- [ ] With `origin` and `upstream`, the first pick lists both with their URLs and **Add remote…** last.
- [ ] **Copy URL** on `origin` puts its URL on the clipboard and logs nothing.
- [ ] Typing `github` in the first pick finds a remote by its URL.
- [ ] Renaming `origin` to `github` and back makes the pull requests be read again from the right repository.
- [ ] Extension: **Gitardo: Manage Remotes**, **Add remote…**, `backup` and `https://example.invalid/backup.git` run
      `git remote add backup https://example.invalid/backup.git`, and the log has it.
- [ ] Extension: picking `backup`, **Rename…** and `mirror` renames it; **Remove** on `mirror` asks **Remove remote
      mirror?** as a warning.

### `remote.fetch` — Fetch one remote

- **Kind**: Operation
- **Function**: `features::remote::fetch(context, answers) -> Outcome`
- **Entry points**: **Fetch** in the pick of `remote.manage`.
- **Extension**: in the pick of **Gitardo: Manage Remotes**, run by the engine with it.
- **Plan**: `git fetch --prune <name>`.
- **Danger**: None.
- **Undo**: Impossible: a fetch only moves the remote-tracking branches, which Undo does not take back.

Acceptance:

- [ ] Fetching `upstream` runs `git fetch --prune upstream`, and its branches deleted on the remote are gone from the
      branch rail after.
- [ ] Extension: **Fetch** on `upstream` runs `git fetch --prune upstream`, and the log has it.

### `remote.set-url` — Change the URL of a remote

- **Kind**: Operation
- **Function**: `features::remote::set_url(context, answers) -> Outcome`
- **Entry points**: **Change URL…** in the pick of `remote.manage`.
- **Extension**: in the pick of **Gitardo: Manage Remotes**, run by the engine with it.
- **Asks**: the URL, in an input box titled **Change the URL of `<name>`** with the prompt **URL of the remote
  repository**, starting from the one it has; it cannot be empty or start with `-`: "Enter a URL", "That is not a URL".
- **Plan**: `git remote set-url <name> <url>`, the URL without the spaces around it. Nothing runs when it is unchanged.
- **Danger**: None.
- **Undo**: Not taken back by Undo: changing it back does.

Acceptance:

- [ ] Changing `origin` to `git@github.com:owner/app.git` runs `git remote set-url origin git@github.com:owner/app.git`.
- [ ] Accepting the URL unchanged runs nothing.

### `remote.rename` — Rename a remote

- **Kind**: Operation
- **Function**: `features::remote::rename(context, answers) -> Outcome`
- **Entry points**: **Rename…** in the pick of `remote.manage`.
- **Extension**: in the pick of **Gitardo: Manage Remotes**, run by the engine with it.
- **Asks**: the new name, in an input box titled **Rename remote `<name>`**, starting from the name it has; checked by
  `branch.validate-name` and "A remote with that name already exists", its own name excepted.
- **Plan**: `git remote rename <old> <new>`. Nothing runs when it is unchanged.
- **Danger**: None.
- **Undo**: Not taken back by Undo: renaming it back does.
- **Uses**: `branch.validate-name`.

Git renames its remote-tracking branches and the upstreams that name it.

Acceptance:

- [ ] Renaming `origin` to `github` runs `git remote rename origin github`, and `origin/main` is `github/main` after,
      the upstream of `main` included.

### `remote.remove` — Remove a remote

- **Kind**: Operation
- **Function**: `features::remote::remove(context, answers) -> Outcome`
- **Entry points**: **Remove** in the pick of `remote.manage`.
- **Extension**: in the pick of **Gitardo: Manage Remotes**, run by the engine with it.
- **Plan**: `git remote remove <name>`.
- **Danger**: Destructive, because "Its remote-tracking branches (`<name>`/…) are deleted from this repository, and
  local branches that follow them lose their upstream. Nothing changes on the remote itself."
- **Undo**: Impossible: `remote.add` adds it again, and a fetch brings its branches back, but not the upstreams.

Acceptance:

- [ ] Removing `upstream` asks, also with `commandPreview` set to `off`, then runs `git remote remove upstream`.
- [ ] A local branch that followed `upstream/main` has no upstream after.
- [ ] Extension: run, the repository has no remote `mirror`, and a branch that followed it has no upstream.

### `remote.add` — Add a remote

- **Kind**: Operation
- **Function**: `features::remote::add(context, answers) -> Outcome`
- **Entry points**: **Add remote…** in the pick of `remote.manage`; **Add remote…** on the warning "This repository has
  no remote.", which fetch, pull and push show in a repository without any remote instead of failing.
- **Extension**: **Add remote…** in the pick of **Gitardo: Manage Remotes**; **Add remote…** on the warning that the
  repository has no remote, after a fetch, a pull, a push or **Sync**. Each sends `feature/run` of `remote.add` to
  Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`, `Controller.manageRemotes`); the name and
  URL are asked in input boxes. When the engine's binary is missing, it says so (D49).
- **Asks**:
  1. The name, in an input box titled **Add remote** with the prompt **Name of the remote**, starting from `origin`
     when the repository has no remote and empty otherwise; checked by `branch.validate-name` and "A remote with that
     name already exists".
  2. The URL, in an input box titled **Add remote `<name>`** with the prompt **URL of the remote repository** and the
     placeholder `https://github.com/owner/repository.git`; it cannot be empty or start with `-`.
- **Plan**: `git remote add <name> <url>`.
- **Danger**: None.
- **Undo**: Not taken back by Undo: `remote.remove` does.
- **After**: the remote of GitHub is worked out again (`github.detect-remote`); nothing is fetched until asked.
- **Uses**: `branch.validate-name`, `github.detect-remote`.

Acceptance:

- [ ] In a repository with no remote, **Push** shows "This repository has no remote." with **Add remote…**, whose name
      box starts with `origin`.
- [ ] Adding `origin` with `https://github.com/owner/app.git` runs
      `git remote add origin https://github.com/owner/app.git`, and **Load pull requests from GitHub** is then in the
      **…** menu.
- [ ] A URL that starts with `-` cannot be accepted.
- [ ] Extension: in a repository without a remote, **Push** runs nothing and warns **This repository has no remote.**
      with **Add remote…**, which asks the name with `origin` filled in.

### `remote.delete-branch` — Delete a branch from its remote

Deletes a branch on the remote, for everyone.

- **Kind**: Operation
- **Function**: `features::remote::delete_branch(context, answers) -> Outcome`
- **Entry points**: **Delete from remote** in the context menu of a remote branch; **Delete `<branch>` from `<remote>`**
  at the end of the menu of a label that a branch shares with its upstream in the timeline (`graph.labels`).
- **Extension**: **Delete from remote** in the context menu of a remote branch; **Delete `<branch>` from `<remote>`**
  under the entries of a branch whose label in the graph stands for its upstream too (`gitardo.graph.combineUpstream`).
  Each sends `feature/run` of `remote.delete-branch` to Gitardo's engine (`Controller.runOnEngine` in
  `src/extension/controller.ts`, `engineRuns` in `src/engine/actions.ts`), with the remote branch as its target. When
  the engine's binary is missing, it says so (D49).
- **Plan**: `git push <remote> --delete <branch>`. The remote is the longest name among the remotes that the remote
  branch starts with followed by `/`; the branch is the rest, named `refs/heads/<branch>` when a tag has its name,
  since the remote would find the short name ambiguous and refuse the push.
- **Danger**: Destructive, because "This deletes the branch on the remote for everyone."
- **Undo**: Impossible: a push changes the remote, which Undo does not take back.
- **Refuses**: "Cannot tell which remote `<name>` belongs to." when no remote matches.
- **Uses**: `graph.labels`.

Acceptance:

- [ ] On `origin/feature/login` it asks, also with `commandPreview` set to `off`, then runs
      `git push origin --delete feature/login`, and `origin/feature/login` is gone after.
- [ ] With remotes `a` and `a/b`, deleting `a/b/x` runs `git push a/b --delete x`.
- [ ] With a tag `release`, deleting `origin/release` runs `git push origin --delete refs/heads/release`, and the tag is
      still on the remote after.
- [ ] Extension: **Delete from remote** on `origin/release`, whose repository has a tag `release`, runs `git push origin
      --delete refs/heads/release` after the warning, and the remote keeps its tag.

### `sync.fetch` — Fetch

Brings what every remote has, and forgets the remote branches deleted there.

- **Kind**: Operation
- **Function**: `features::sync::fetch(context, answers) -> Outcome`
- **Entry points**: **Fetch** in the menu of **Sync** in the header, whose tooltip is **Fetch: git fetch --all
  --prune**; **Fetch** in Quick search. **Sync** fetches first too (`sync.sync`).
- **Extension**: **Fetch** in the toolbar of the graph (tooltip **Fetch: git fetch --all --prune**); **Gitardo: Fetch**
  in the Command Palette and as a button on the titles of the Commits and Gitardo Graph views. Each sends `feature/run`
  of `sync.fetch` to Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`, `engineRuns` in
  `src/engine/actions.ts`) through `Controller.sync` and `syncRuns`; the pull requests are read again after it
  (`readPullRequests`). When the engine's binary is missing, it says so (D49).
- **Plan**: `git fetch --all --prune`.
- **Danger**: None.
- **Undo**: Impossible: a fetch only moves the remote-tracking branches, which Undo does not take back.
- **After**: the pull requests are read again from GitHub when the token is known (`github.read-pull-requests`).
- **Refuses**: in a repository with no remote, nothing runs: "This repository has no remote." with **Add remote…**
  (`remote.add`).
- **Uses**: `remote.add`, `github.read-pull-requests`.

Acceptance:

- [ ] **Fetch** runs `git fetch --all --prune`, and a branch deleted on the remote is gone from the remote branches
      after.
- [ ] The ahead and behind counts of the branches are those of the new remote branches after.
- [ ] Extension: **Fetch** runs `git fetch --all --prune` in the engine, and the log has it.
- [ ] Extension: in a repository without a remote, nothing runs and the warning offers **Add remote…**.

### `sync.deepen` — Load more history

Fetches more of the history of a shallow clone: a number of commits further back than where it ends now
([D77](../08-decisions.md#d77--clone-and-create-a-repository-and-the-history-of-a-shallow-clone-proposed)).

- **Kind**: Operation
- **Function**: `features::sync::deepen(context, answers) -> Outcome`
- **Entry points**: **Load more history** on the last row of the timeline and in the inspector of a commit whose
  parents were not fetched (`graph.shallow-end`); **Load More History** in Quick search.
- **Extension**: Planned: the same buttons in the graph, and **Gitardo: Load More History** in the Command Palette,
  each sending `feature/run` of `sync.deepen` to the engine.
- **Offered when**: the repository is shallow (`repo.read-shallow`).
- **Asks**: **How many commits further back**, `100` by default (Q1063), refused while typed when it is not a whole
  number of at least 1.
- **Reads**: `repo.read-shallow`.
- **Plan**: `git fetch --deepen=<n> <remote>`, from the remote of the upstream of the branch checked out, or from the
  default remote (`remote.default`). Titled **Load `<n>` more commits of history from `<remote>`**.
- **Danger**: None.
- **Undo**: Impossible: a fetch only adds commits, which Undo does not take back.
- **After**: the timeline is read again, and lists the commits fetched under those it had. When git reached the first
  commits of the history, the repository is no longer shallow, and `graph.shallow-end` shows nothing.
- **Refuses**: in a repository with no remote: "This repository has no remote." with **Add remote…** (`remote.add`).
- **Uses**: `repo.read-shallow`, `remote.default`, `graph.shallow-end`, `remote.add`.

Acceptance:

- [ ] In a clone with `--depth 2` of a history of ten commits, **Load more history** with `3` runs `git fetch
      --deepen=3 origin`; the timeline lists five commits after, and still ends with the row of a shallow clone.
- [ ] With `100`, the timeline lists the ten commits after, the repository is no longer shallow, and the row is gone.
- [ ] A repository that is not shallow has no **Load More History** in Quick search.
- [ ] `0` and `ten` are refused while typed.

### `sync.unshallow` — Load the whole history

Fetches all the history a shallow clone does not have, so that it is a clone like any other
([D77](../08-decisions.md#d77--clone-and-create-a-repository-and-the-history-of-a-shallow-clone-proposed)).

- **Kind**: Operation
- **Function**: `features::sync::unshallow(context, answers) -> Outcome`
- **Entry points**: **Load the whole history** on the last row of the timeline (`graph.shallow-end`); **Load the
  Whole History** in Quick search.
- **Extension**: Planned: the same button in the graph, and **Gitardo: Load the Whole History** in the Command
  Palette, sending `feature/run` of `sync.unshallow` to the engine.
- **Offered when**: the repository is shallow (`repo.read-shallow`).
- **Reads**: `repo.read-shallow`.
- **Plan**: `git fetch --unshallow <remote>`, from the remote `sync.deepen` fetches from. Titled **Load the whole
  history from `<remote>`**.
- **Danger**: None.
- **Undo**: Impossible: a fetch only adds commits, which Undo does not take back.
- **After**: the timeline is read again and lists the whole history; `graph.shallow-end` shows nothing.
- **Refuses**: in a repository with no remote: "This repository has no remote." with **Add remote…** (`remote.add`).
- **Uses**: `repo.read-shallow`, `sync.deepen`, `graph.shallow-end`, `remote.add`.

Acceptance:

- [ ] In a clone with `--depth 1` of a history of ten commits, it runs `git fetch --unshallow origin`, and the timeline
      lists the ten commits after, the last one ending at its dot.
- [ ] While it runs for more than half a second, the **Activity** tab lists **Load the whole history from origin**.
- [ ] A repository that is not shallow has no **Load the Whole History** in Quick search.

### `sync.pull` — Pull

Brings what the upstream has into the branch checked out, the way the settings choose.

- **Kind**: Operation
- **Function**: `features::sync::pull(context, answers) -> Outcome`
- **Entry points**: **Pull** in the menu of **Sync** in the header, which has the number of commits to pull when there
  are some, and whose tooltip says the command it runs, **Pull: git pull --rebase --autostash**; **Pull** in Quick
  search. **Sync** pulls the same way after its fetch (`sync.sync`).
- **Extension**: **Pull** in the toolbar of the graph, with the number of commits the branch is behind, its tooltip the
  command the settings make, as **Pull: git pull --rebase --autostash**; **Gitardo: Pull** in the Command Palette and as
  a button on the titles of the Commits and Gitardo Graph views. Each sends `feature/run` of `sync.pull` to Gitardo's
  engine (`Controller.runOnEngine` in `src/extension/controller.ts`, `engineRuns` in `src/engine/actions.ts`) through
  `Controller.sync` and `syncRuns`. The settings are `gitardo.pull.rebase` and `gitardo.pull.autostash`. The warning
  that the changes set aside do not fit on what came in, with **Show Stashes**, is said by the extension from the
  stashes read after the pull (`Controller.afterSync`), on both paths. When the engine's binary is missing, it says so
  (D49).
- **Plan**: `git pull`, with `--rebase` when `pull.rebase` is on and `--autostash` when `pull.autostash` is on:
  `git pull --rebase --autostash` with both on, as they are by default.
- **Danger**: None.
- **Undo**: By the reflog: Undo moves the branch back to where it was before the pull (`undo.last`).
- **After**: the pull requests are read again from GitHub when the token is known. With `--autostash`, when the changes
  that were set aside no longer fit on what came in, `stash.autostash-left` says so.
- **On failure**: when the pull stops on a conflict, the conflicts open as for any merge or rebase (`conflicts.`); git
  puts the changes it set aside back by itself once the merge or the rebase is finished or given up.
- **Refuses**: in a repository with no remote: "This repository has no remote." with **Add remote…** (`remote.add`).
- **Settings**: `pull.rebase` (on by default) adds `--rebase`, putting the commits not pushed on top of what comes in
  instead of merging the two; `pull.autostash` (on by default) adds `--autostash`, setting the changes that are not
  committed aside before the pull and putting them back after. One that is off leaves its option out, and with it the
  choice to the configuration of git.
- **Uses**: `stash.autostash-left`, `remote.add`, `github.read-pull-requests`.

Acceptance:

- [ ] With both settings on by default, **Pull** runs `git pull --rebase --autostash`, and its tooltip says so.
- [ ] With `pull.rebase` off, it runs `git pull --autostash`; with both off, `git pull`.
- [ ] With a file modified and the upstream ahead on other files, the pull succeeds and the file is still modified
      after; untracked files are not touched.
- [ ] Undo right after a pull that brought commits moves the branch back to where it was.
- [ ] Extension: with both settings on, **Pull** runs `git pull --rebase --autostash`; `gitardo.pull.rebase` `false`,
      `git pull --autostash`; both `false`, `git pull`.
- [ ] Extension: with a change where the pull changes the file too, **Pull (rebase, autostash)** leaves the file in
      conflict, a stash named `autostash`, and the warning with **Show Stashes**.

### `sync.pull-with` — Pull one way or another

Pulls in one of the four ways, whatever the settings say **Pull** does.

- **Kind**: Operation
- **Function**: `features::sync::pull_with(context, answers) -> Outcome`
- **Entry points**: the other ways to pull, in the menu of **Sync** in the header, which lists **Pull** (`git pull`),
  **Pull (rebase)** (`git pull --rebase`), **Pull (autostash)** (`git pull --autostash`) and **Pull (rebase,
  autostash)** (`git pull --rebase --autostash`), the one of **Pull** marked **default**; in Quick search, **Pull (no
  options)**, which is `git pull` alone, **Pull (rebase)**, **Pull (autostash)** and **Pull (rebase, autostash)**.
- **Extension**: the menu **More pull options** beside **Pull** in the toolbar: **Pull**, **Pull (rebase)**, **Pull
  (autostash)**, **Pull (rebase, autostash)**, the one of the button marked **default**; **Gitardo: Pull (No Options)**,
  **Gitardo: Pull (Rebase)**, **Gitardo: Pull (Autostash)** and **Gitardo: Pull (Rebase, Autostash)** in the Command
  Palette and in the **…** of the Commits and Gitardo Graph views. Each other than the way of the button sends
  `feature/run` of `sync.pull-with` with its way as the answer `way` (`syncRuns` in `src/engine/actions.ts`). When the
  engine's binary is missing, it says so (D49).
- **Plan**: the command of the way chosen.
- **Danger**: None.
- **Undo**: By the reflog, as `sync.pull`.
- **After**: as `sync.pull`.
- **On failure**: as `sync.pull`.
- **Refuses**: as `sync.pull`.
- **Uses**: `sync.pull`, `stash.autostash-left`.

Acceptance:

- [ ] **Pull (no options)** runs `git pull` even with `pull.rebase` and `pull.autostash` on.
- [ ] With `pull.rebase` off, the menu marks **Pull (autostash)** as the default.
- [ ] Extension: **Gitardo: Pull (No Options)** runs `git pull` whatever the settings.
- [ ] Extension: the menu beside the button lists the four ways in that order, with **default** on the one of the
      button.

### `sync.push` — Push

Sends the commits of the branch checked out to its upstream, or, for its first push, to the remote, making that its
upstream.

- **Kind**: Operation
- **Function**: `features::sync::push(context, answers) -> Outcome`
- **Entry points**: **Push** in the menu of **Sync** in the header, which has the number of commits to push when there
  are some, and whose tooltip is **Push: git push**; **Push** in Quick search. **Sync** pushes the same way after its
  pull (`sync.sync`).
- **Extension**: **Push** in the toolbar of the graph (tooltip **Push: git push**), with the number of commits the
  branch is ahead; **Push** in the menu **More push options** beside it, marked **default**; **Gitardo: Push** in the
  Command Palette and as a button on the titles of the Commits and Gitardo Graph views. Each sends `feature/run` of
  `sync.push` to Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`, `engineRuns` in
  `src/engine/actions.ts`) through `Controller.sync`. After the first push of a branch, when the main remote is on
  GitHub and the branch has no pull request, **Pushed `<branch>`.** offers **Create Pull Request**
  (`Controller.afterSync`). When the engine's binary is missing, it says so (D49).
- **Plan**:
  - With an upstream of the same name: `git push <remote> <branch>`.
  - With an upstream of another name: `git push <remote> <branch>:<upstream branch>`.
  - With no upstream: `git push --set-upstream <remote> <branch>`, to the remote of `remote.default`.
  - The branch is named as `branch.ref-argument` names it: `refs/heads/<branch>` when a tag or a remote branch has its
    name.
- **Danger**: None.
- **Undo**: Impossible: a push changes the remote, which Undo does not take back.
- **After**: the pull requests are read again from GitHub when the token is known; after the first push of a branch,
  `github.offer-after-push`.
- **Refuses**: on a detached `HEAD`: "Cannot push: HEAD is detached. Check out a branch first."; in a repository with no
  remote: "This repository has no remote." with **Add remote…** (`remote.add`).
- **Uses**: `branch.ref-argument`, `remote.default`, `github.offer-after-push`, `github.read-pull-requests`.

Acceptance:

- [ ] `main` following `origin/main` runs `git push origin main`.
- [ ] `local` following `origin/remote-name` runs `git push origin local:remote-name`.
- [ ] A new branch `feature` with no upstream runs `git push --set-upstream origin feature`, and follows
      `origin/feature` after.
- [ ] `release` with a tag `release` runs `git push origin refs/heads/release`.
- [ ] On a detached `HEAD` nothing runs and the message says why.
- [ ] Extension: `work` with no upstream pushes with `git push --set-upstream origin work`, and afterwards follows
      `origin/work`.
- [ ] Extension: on a detached `HEAD` nothing runs and the error says **HEAD is detached**.

### `sync.sync` — Sync with the upstream

Fetches, pulls what the upstream has, then pushes what the branch checked out has ahead of it: in one click, what the
counts on **Sync** promise.

- **Kind**: Operation
- **Function**: `features::sync::sync(context, answers) -> Outcome`
- **Entry points**: a click on **Sync** in the header (`graph.toolbar`), which shows beforehand the commits to pull and
  to push as of the last fetch, `↓2 ↑1`, and whose tooltip is **Sync: git fetch, git pull, git push**; **Sync** in
  Quick search. **Fetch**, **Pull** and its other ways, **Push** and its other ways stay in the menu of **Sync**.
- **Extension**: **Sync: fetch, pull, push** in the menu **More push options** beside **Push** in the toolbar of the
  graph; **Gitardo: Sync** (`gitardo.sync`) in the Command Palette and in the **…** of the Commits and Gitardo Graph
  views. Each sends `feature/run` of `sync.sync` to Gitardo's engine (`Controller.runOnEngine` in
  `src/extension/controller.ts`, `engineRuns` in `src/engine/actions.ts`) through `Controller.sync` and `syncRuns`.
  When the engine's binary is missing, it says so (D49), and runs nothing.
- **Asks**: when the branch has no upstream, a confirmation, "`<branch>` has no upstream. Publish it to `<remote>`?",
  with **Publish branch**, `<remote>` being the one of `remote.default`.
- **Plan**, in this order:
  - `git fetch --all --prune`, as `sync.fetch`;
  - the pull of `sync.pull`, as the settings of how to pull say: `git pull --rebase --autostash` with both on, as they
    are by default;
  - the push of `sync.push`, `git push <remote> <branch>` (with `:<upstream branch>` when the names differ, and the
    branch named as `branch.ref-argument` names it), only when the branch has commits to push;
  - with no upstream, after the confirmation, only the first push of `sync.push`:
    `git push --set-upstream <remote> <branch>`.
- **Danger**: None, as for a pull and a push: the plan is shown first only when `commandPreview` asks for it.
- **Undo**: By the reflog, as `sync.pull`: Undo moves the branch back to where it was before the pull (`undo.last`).
  The push is not taken back, as for `sync.push`, nor the fetch, as for `sync.fetch`; a sync that pulled nothing leaves
  nothing for Undo to take back.
- **After**: as `sync.pull` and `sync.push`: the pull requests are read again from GitHub when the token is known,
  `stash.autostash-left` says when the changes set aside no longer fit, and after the first push of a branch
  `github.offer-after-push`. The counts on **Sync** are read again with the rest of the state.
- **On failure**: no step after the one that failed runs. A fetch that fails (no network, a remote that refuses) stops
  the sync before the pull. A pull that stops on a conflict opens the conflicts as `sync.pull` does (`conflicts.banner`),
  and the push does not run. A pull that git refuses because what comes in would overwrite changes not committed stops
  the sync before the push. A push that the remote refuses, because its branch moved since the fetch, keeps what the
  pull brought, and says what git said.
- **Refuses**: what a pull and a push refuse: on a detached `HEAD`, "Cannot sync: HEAD is detached. Check out a branch
  first."; in a repository with no remote, "This repository has no remote." with **Add remote…** (`remote.add`); with
  `pull.rebase` on and `pull.autostash` off, while a tracked file has changes, staged or not, which would stop the pull:
  "Commit or stash the changes first: the pull cannot rebase over them."
- **Settings**: `pull.rebase` and `pull.autostash`, for the pull, as `sync.pull`; `commandPreview`.
- **Uses**: `sync.fetch`, `sync.pull`, `sync.push`, `remote.default`, `branch.ref-argument`, `remote.add`,
  `stash.autostash-left`, `conflicts.banner`, `github.read-pull-requests`, `github.offer-after-push`, `undo.last`.

Acceptance:

- [ ] With `main` one commit ahead of `origin/main` and two behind, **Sync** shows `↓2 ↑1`, and a click runs
      `git fetch --all --prune`, `git pull --rebase --autostash` and `git push origin main`, in that order; `main` and
      `origin/main` are at the same commit after.
- [ ] With nothing to push, the plan has the fetch and the pull and no push.
- [ ] With `commandPreview` set to `all`, the commands are shown before any of them runs, and declining runs nothing;
      with `off`, they run at once.
- [ ] A branch `feature` with no upstream asks to publish it, and **Publish branch** runs only
      `git push --set-upstream origin feature`; `feature` follows `origin/feature` after.
- [ ] With `pull.rebase` on, `pull.autostash` off and a tracked file modified, nothing runs and the message says why.
- [ ] When the pull stops on a conflict, the conflicts banner shows, the push does not run, and `origin/main` on the
      remote is where it was.
- [ ] On a detached `HEAD` nothing runs and the message says why.
- [ ] Undo right after a sync that pulled two commits and pushed one moves `main` back to where it was before the pull;
      the remote branch stays where the push put it.
- [ ] Extension: with `main` 1 commit behind and 1 ahead of `origin/main`, **Gitardo: Sync** runs `git fetch --all
      --prune`, `git pull --rebase --autostash` and `git push origin main`, and `main` is then neither ahead nor behind.
- [ ] Extension: on a branch with no upstream it asks whether to publish it, and runs `git push --set-upstream origin
      <branch>`.
- [ ] Extension: with `gitardo.engine.path` naming a missing file, it runs nothing and shows **Gitardo's engine is not
      available on this system.** with **Show the log**.

### `sync.read-overwritten` — Read what a force push would overwrite

The commits of the upstream that the branch checked out does not have, as of the last fetch: what a force push throws
away.

- **Kind**: Read
- **Function**: `git::read::sync::read_overwritten(git: &Git, request: &OverwrittenRequest) -> Result<Overwritten, GitError>`
- **Reads**: `git log --max-count=11 --format=%h%x00%s%x00 HEAD..<upstream>`, the short hash and the subject of each,
  newest first; 11 to tell whether there are more than the 10 shown.

Acceptance:

- [ ] With the upstream two commits ahead, it gives those two, newest first.
- [ ] With 15 commits only on the upstream, it gives 10 and says there are more.
- [ ] With nothing only on the upstream, it gives none.

### `sync.push-force-with-lease` — Push with `--force-with-lease`

Replaces the remote branch with the branch checked out, unless the remote branch has moved since the last fetch.

- **Kind**: Operation
- **Function**: `features::sync::push_force_with_lease(context, answers) -> Outcome`
- **Entry points**: **Push (force with lease)** among the other ways to push, in the menu of **Sync**, and in Quick
  search.
- **Extension**: **Push (force with lease)** in the menu **More push options** beside **Push** in the toolbar;
  **Gitardo: Push (Force with Lease)** in the Command Palette and in the **…** of the Commits and Gitardo Graph views.
  Each sends `feature/run` of `sync.push-force-with-lease` to Gitardo's engine (`Controller.runOnEngine` in
  `src/extension/controller.ts`, `engineRuns` in `src/engine/actions.ts`); the warning is a modal dialog. When the
  engine's binary is missing, it says so (D49).
- **Reads**: `sync.read-overwritten`, when the branch has an upstream that is not gone.
- **Plan**: `git push --force-with-lease <remote> <branch>`, with `:<upstream branch>` and `--set-upstream` as for
  `sync.push`.
- **Danger**: Destructive, because the commits only the remote branch has are lost for everyone. The warning lists them:
  "These commits on `<upstream>` are not in `<branch>` and will be overwritten:" with up to 10 lines `<hash> <subject>`
  and "…and more" after them when there are more; or "As of the last fetch, every commit on `<upstream>` is also in
  `<branch>`."
- **Undo**: Impossible: a push changes the remote, which Undo does not take back.
- **On failure**: git refuses when the remote branch is not where the last fetch saw it, and says why; nothing is
  overwritten.
- **After**: as `sync.push`.
- **Refuses**: as `sync.push`.
- **Uses**: `sync.push`, `sync.read-overwritten`.

Acceptance:

- [ ] After a rebase of a pushed branch, it asks with the list of the commits the remote has and the branch has not,
      also with `commandPreview` set to `off`, then runs `git push --force-with-lease origin <branch>`.
- [ ] When someone pushed to the remote branch since the last fetch, git refuses and the remote branch is unchanged.
- [ ] Extension: after the last commit was amended, it asks once, with a warning, whatever `gitardo.commandPreview`
      says; the detail says **will be overwritten:** followed by the hash and subject of the commit as it was, and ends
      with `git push --force-with-lease origin main`.
- [ ] Extension: cancelling runs nothing; **Run** pushes and the branch is then neither ahead nor behind its upstream.

### `sync.push-force` — Push with `--force`

Replaces the remote branch with the branch checked out, whatever it has.

- **Kind**: Operation
- **Function**: `features::sync::push_force(context, answers) -> Outcome`
- **Entry points**: **Push (force)** among the other ways to push, in the menu of **Sync**, and in Quick search.
- **Extension**: **Push (force)** in the menu **More push options** beside **Push** in the toolbar; **Gitardo: Push
  (Force)** in the Command Palette and in the **…** of the Commits and Gitardo Graph views. Each sends `feature/run` of
  `sync.push-force` to Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`, `engineRuns` in
  `src/engine/actions.ts`). When the engine's binary is missing, it says so (D49).
- **Reads**: `sync.read-overwritten`, when the branch has an upstream that is not gone.
- **Plan**: `git push --force <remote> <branch>`, with `:<upstream branch>` and `--set-upstream` as for `sync.push`.
- **Danger**: Destructive, because the commits only the remote branch has are lost for everyone; the warning lists them
  as for `sync.push-force-with-lease`, as of the last fetch.
- **Undo**: Impossible: a push changes the remote, which Undo does not take back.
- **After**: as `sync.push`.
- **Refuses**: as `sync.push`.
- **Uses**: `sync.push`, `sync.read-overwritten`.

Acceptance:

- [ ] It asks with the list of the commits that will be overwritten, then runs `git push --force origin <branch>`.
- [ ] With 12 commits only on the remote, the warning lists 10 and "…and more".
- [ ] Extension: **Push (force)** on a branch without an upstream runs `git push --force --set-upstream origin
      <branch>`.
