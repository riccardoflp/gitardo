# Submodules

A submodule is a repository inside another, which records for it one commit of its own in each of its commits. This
area covers the Submodules list, which says where each submodule stands against the commit recorded for it, what
can be done with a submodule from its context menu, adding a submodule and synchronizing their URLs, bringing the
submodules to what a checkout records, and following what happens inside them.

A submodule inside another submodule is managed from that one: its commands run in the folder of the submodule that
has it (`git -C <parent> …`), with its path from there. A path with `*`, `?`, `[`, `]` or `\` in it, or one that starts
with `:`, is given to git as `:(literal)<path>`.

### `submodule.read-list` — Read the submodules and where they stand

The submodules of the repository, and theirs, each with the commit recorded for it, the commit it has checked out, its
branch, how far apart the two are and how many of its files are not committed.

- **Kind**: Read
- **Function**: `git::read::submodule::read_list(git, request) -> Result<Vec<Submodule>, GitError>`
- **Reads**, in a repository with a `.gitmodules` at its top:
  - `git config -z --file .gitmodules --get-regexp '^submodule\.'`: the name, folder, URL and branch of each;
  - `git ls-tree -z HEAD -- <folders>`: the commit recorded for each in the last commit, none before the first commit or
    for one added but not committed yet;
  - `git ls-files -s -z -- <folders>`, each as `:(literal)<folder>`: the commit staged for it, when that is not the one
    recorded;
  - for each whose folder has a repository (a `.git` in it), all at once: `git -C <folder> status --porcelain=v2 --branch
    -z --untracked-files=all`, for the commit it has checked out, its branch or none (detached) and the number of its
    files not committed; and, when that commit is not the recorded one, `git -C <folder> rev-list --left-right --count
    <recorded>...<checked out> --`, for how many commits each has that the other has not. That fails when the recorded
    commit is not in it, and then how far apart they are is not known.
  - then the same in each initialized submodule, for the submodules it has, down to five levels.
  - All of it with `GIT_OPTIONAL_LOCKS=0`.
- **Extension**: read by the extension itself, in TypeScript, each time it reads the repository:
  `Repo.submodules(parent, depth)` in `src/git/repo.ts`, called by `Repo.state`, runs the same commands with
  `GIT_OPTIONAL_LOCKS=0`, as deep as `SUBMODULE_DEPTH` says, and `parseGitmodules`, `parseGitlinks` and
  `parseBranchStatus` in `src/git/parse.ts` make the `Submodule` of `src/shared/protocol.ts`, until the reads move to
  the engine (stage 5 of the roadmap). The repository is read again when `HEAD` or the references of a clone in
  `.git/modules` change (`submodule.follow-changes`).

Acceptance:

- [ ] A repository with `lib` on its recorded commit gives `lib` with ahead and behind both 0.
- [ ] After two commits made in `lib`, it gives ahead 2, behind 0, and the branch checked out in it, or none when it is
      detached.
- [ ] A submodule whose folder is empty gives no commit checked out, and no `git status` runs in it, so that git does
      not run in the repository around it.
- [ ] A submodule whose recorded commit it does not have gives ahead and behind as not known.
- [ ] `vendor/a`, which has a submodule `b`, gives `vendor/a/b` after `vendor/a`, with `vendor/a` as the one that has it.
- [ ] A submodule added and staged but not committed gives no recorded commit and the staged one.
- [ ] Parsing is tested on recorded output, a folder with a space in its name included.
- [ ] Extension: a repository without `.gitmodules` has no submodules, and an entry of `.gitmodules` without a path is
      left out; a name with dots in it is read whole.
- [ ] Extension: a submodule not initialized lists none of the submodules inside it.
- [ ] Extension: a file renamed in a submodule counts as one file not committed.
- [ ] Extension: a commit made in `lib` from a terminal is read without a refresh.

### `submodule.status` — Where a submodule stands

What the icon of a submodule says, and the sentence of its tooltip, from the commit recorded for it and the commit it
has checked out.

- **Kind**: Derivation
- **Function**: `model::submodule::status(input) -> SubmoduleStatus`
- **Extension**: worked out by the extension itself, in TypeScript, from the `Submodule` it read
  (`submodule.read-list`): `submoduleStatus(submodule)` and `submoduleStatusText(submodule)` in
  `src/shared/submodules.ts`, the icon by `submoduleBadge` in `src/webview/RefsTree.tsx` (codicons: `circle-slash`,
  `diff-added`, `warning`, `arrow-swap`, `arrow-up`, `arrow-down`, `pass-filled`), and how deep a submodule is, for its
  indent, by `submoduleDepth`, until the reads move to the engine (stage 5 of the roadmap).

The rules, the first that applies:

| Status | When | Icon | Sentence |
| --- | --- | --- | --- |
| Not initialized | its folder has no repository | a grey sign | **Not initialized: its folder has no checkout** |
| Added | no commit is recorded for it yet | a grey sign of an addition | **Added, not committed yet** |
| Unknown | the recorded commit is not in it | a warning | **The recorded commit `<short>` is not in it: fetch in it, or update it** |
| Both | each has commits the other has not | both arrows, with both numbers, `2↑ 1↓` | **2 commits ahead of and 1 commit behind the recorded commit `<short>`** |
| Ahead | it has commits after the recorded one | a yellow arrow up, with the number | **2 commits ahead of the recorded commit `<short>`** |
| Behind | it is behind the recorded one | a yellow arrow down, with the number | **1 commit behind the recorded commit `<short>`** |
| On it | it has the recorded commit checked out | a green check | **On the recorded commit `<short>`** |

Acceptance:

- [ ] Each row of the table is given in the case it describes.
- [ ] One commit is said **1 commit**, more **`<n>` commits**.
- [ ] The icons read in the dark, the light and both high contrast themes.
- [ ] Extension: checked out `bbbbbbb…` with `aaaaaaa…` recorded, 1 ahead and 0 behind, says **1 commit ahead of the
      recorded commit aaaaaaa**; with 2 ahead and 3 behind it is both; with ahead and behind not known, unknown.
- [ ] Extension: no commit checked out is not initialized, whatever else is known; one checked out with nothing
      recorded is added.
- [ ] Extension: `vendor/lib/inner` inside `vendor/lib` is one step deep, `vendor/lib` none.

### `submodule.section` — The Submodules list

The submodules of the repository in the branch rail, under **Kept aside**, each with where it stands.

- **Kind**: View
- **Function**: `views::refs::submodules::section(state, ui) -> Vec<Intent>`
- **Entry points**: the **Submodules** list in **Kept aside** of the branch rail.
- **Extension**: the **Submodules** section of the references beside the graph and of the Branches view (the
  `submodules` section of `RefsTree` in `src/webview/RefsTree.tsx`, `SECTIONS` in `src/webview/BranchesApp.tsx`),
  closed when VS Code starts, as every section; its context menu is `submoduleMenu(submodule, state)` in
  `src/webview/menus.ts`, with **Deinitialize…** and **Remove…** marked as entries that lose work, and the **+** on
  its title is **Add submodule…**. Its rows, tooltips and menu are those of this list, the operations sent to Gitardo's
  engine as their cards say; what it shows is read in TypeScript (`submodule.read-list`, `submodule.status`).
- **Offered when**: the repository has submodules; otherwise the list is not there.
- **Reads**: `submodule.read-list`.
- **Uses**: `submodule.status`.

What each row shows:

- Its folder, from the submodule that has it for one inside another, indented under it, one step for each level.
- The icon of `submodule.status`, with its number.
- The branch it has checked out, or `detached`; nothing when it is not initialized.
- A pencil with the number of its files not committed, when there are some.
- Its tooltip: its folder, its URL, the sentence of `submodule.status`, **Checked out: `<branch or detached>` at
  `<short>`**, **Staged as the recorded commit: `<short>`** when another commit is staged for it, and how many files are
  not committed.

The title of the list has **+**, which is `submodule.add`. A double click on an initialized submodule opens it as
`submodule.open-in-new-window` does; on one that is not initialized it does nothing. Its context menu has, in this
order, those of the cards of this area that are offered: `submodule.open-in-new-window`, `submodule.open-in-terminal`,
`submodule.reveal`, `submodule.show-in-gitardo`; `submodule.initialize`, `submodule.update-to-recorded`,
`submodule.stage-recorded`, `submodule.update-to-latest`, `submodule.sync-urls`; `submodule.copy-path`,
`submodule.copy-url`; `submodule.deinitialize`, `submodule.remove`.

Acceptance:

- [ ] A repository with `lib` two commits ahead on `main`, with one file changed, shows `lib`, an arrow up with 2,
      `main`, and a pencil with 1.
- [ ] `vendor/a/b` shows as `b`, indented under `vendor/a`.
- [ ] The tooltip of `lib` has its URL and both commits.
- [ ] A repository without submodules has no Submodules list.
- [ ] Extension: a submodule just added shows the grey added icon; once committed, the green check with **On the
      recorded commit**; after a commit made in it, a yellow arrow up and `1`.
- [ ] Extension: the context menu of a submodule one commit ahead has **Open in new window**, **Show in Gitardo**,
      **Update to the recorded commit** and **Remove…**.
- [ ] Extension: a submodule not initialized has only **Initialize**, **Synchronize URLs from .gitmodules**, **Copy
      path**, **Copy URL** and **Remove…** in its menu, and a double click does nothing.
- [ ] Extension: a commit made in a submodule in a terminal changes its icon without a refresh.

### `submodule.open-in-new-window` — Open a submodule in a new window

Opens the submodule's folder in a new window of Gitardo Desktop, as a repository of its own.

- **Kind**: Operation
- **Function**: `features::submodule::open_in_new_window(context, answers) -> Outcome`
- **Entry points**: **Open in new window** in the context menu of a submodule; a double click on it.
- **Extension**: stays in the extension, in TypeScript: the `submoduleOpen` case of `resolveAction` in
  `src/extension/actions.ts` opens the folder in a new VS Code window with `openWorktree(folder, 'window')`
  (`vscode.openFolder`); no git runs, nothing is asked or logged. From a view drawn before, it refuses one not
  initialized, **`<folder>` is not initialized: update it first.**, and a path that is no longer a submodule,
  **`<folder>` is no longer a submodule of this repository.**
- **Offered when**: it is initialized.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: a new window of Gitardo Desktop shows the submodule's folder.
- **Refuses**: on one that is not initialized, **`<folder>` is not initialized: update it first.**

Acceptance:

- [ ] A double click on `lib` opens a window whose timeline is the history of `lib`.
- [ ] Extension: **Open in new window** on `vendor/lib` opens `<root>/vendor/lib` in a new VS Code window.
- [ ] Extension: it is not in the menu of a submodule that is not initialized.

### `submodule.open-in-terminal` — Open a terminal in a submodule

Opens a terminal with the submodule's folder as its folder.

- **Kind**: Operation
- **Function**: `features::submodule::open_in_terminal(context, answers) -> Outcome`
- **Entry points**: **Open in terminal** in the context menu of a submodule.
- **Extension**: stays in the extension, in TypeScript: the `worktreeTerminal` case of `resolveAction` in
  `src/extension/actions.ts`, given the folder of the submodule, opens a VS Code terminal named after the last part of
  that folder, in it, and shows it; no git runs.
- **Offered when**: it is initialized.
- **Plan**: `run <terminal>` in the submodule's folder: `start "" cmd` on Windows, `open -a Terminal .` on macOS,
  `x-terminal-emulator` elsewhere.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the terminal of `terminal.external`, or of the system, opens in the submodule's folder.
- **Settings**: `terminal.external`.

Acceptance:

- [ ] The terminal opens with its current folder set to the submodule's folder.
- [ ] Extension: **Open in terminal** on `vendor/lib` shows a terminal named `lib` whose folder is `<root>/vendor/lib`.

### `submodule.reveal` — Reveal a submodule in the file manager

Shows the submodule's folder in the file manager of the system.

- **Kind**: Operation
- **Function**: `features::submodule::reveal(context, answers) -> Outcome`
- **Entry points**: **Reveal in File Explorer** on Windows, **Reveal in Finder** on macOS, **Open containing folder**
  elsewhere, in the context menu of a submodule.
- **Extension**: stays in the extension, in TypeScript: the `worktreeReveal` case of `resolveAction` in
  `src/extension/actions.ts` runs VS Code's `revealFileInOS` on the folder; the name of the entry is `REVEAL` in
  `src/webview/menus.ts`.
- **Offered when**: it is initialized.
- **Plan**: `run start "" <folder>` on Windows, `run open <folder>` on macOS, `run xdg-open <folder>` elsewhere.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the file manager shows the folder.

Acceptance:

- [ ] On each system the entry has the name given above, and the file manager opens on the submodule's folder.
- [ ] Extension: on Windows the entry reads **Reveal in File Explorer** and shows the folder of the submodule.

### `submodule.show-in-gitardo` — Show a submodule in this window

Points this window at the submodule, as a repository of its own, without opening another window.

- **Kind**: Operation
- **Function**: `features::submodule::show_in_gitardo(context, answers) -> Outcome`
- **Entry points**: **Show in Gitardo** in the context menu of a submodule.
- **Extension**: stays in the extension, in TypeScript: `Controller.selectRepo(root)` in `src/extension/controller.ts`,
  from the `selectRepo` message. A submodule that is one of the repositories of the workspace is shown as when it is
  chosen in the list of repositories; another is shown with the banner of `worktree.banner` (`WorktreeBanner` in
  `src/webview/WorktreeBanner.tsx`), **Showing the worktree `<name>`** with **Back to `<repository>`**. Nothing happens
  while an operation runs.
- **Offered when**: it is initialized.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the timeline, the draft, the branch rail and the Changes page are those of the submodule, and what is done
  there is done in it. The banner of `worktree.banner` says which folder they show, with **Back to** the repository of
  the window. The choice is not remembered when the window is opened again.
- **Uses**: `worktree.banner`.

Acceptance:

- [ ] After **Show in Gitardo** on `lib`, a commit made from the draft is made in `lib`, and **Back to** shows the
      repository again, where `lib` is now one commit ahead.
- [ ] Extension: **Show in Gitardo** on `vendor/lib` makes the graph show the commits of `vendor/lib`, and **Back to
      `<repository>`** shows the repository that has it again.

### `submodule.update-to-recorded` — Update a submodule to the recorded commit

Checks out in the submodule the commit the repository records for it, and in its own submodules theirs.

- **Kind**: Operation
- **Function**: `features::submodule::update_to_recorded(context, answers) -> Outcome`
- **Entry points**: **Update to the recorded commit `<short>`** in the context menu of a submodule.
- **Extension**: **Update to the recorded commit `<short>`** in the context menu of a submodule in the Submodules
  section. It sends `feature/run` of `submodule.update-to-recorded` to Gitardo's engine on the submodule by its path
  (`submoduleUpdate` in `engineRuns` of `src/engine/actions.ts`, run by `Controller.runOnEngine` in
  `src/extension/controller.ts`), its plan shown in a modal dialog first when it is Confirm or `gitardo.commandPreview`
  is `all` (`VscodeAsker` in `src/engine/vscode.ts`). From a view drawn before, the engine refuses a submodule with no
  recorded commit, **No commit is recorded for `<folder>` yet.**, and one on its recorded commit, **`<folder>` is on the
  recorded commit already.** When the engine's binary is missing, `updateSubmodule(submodule)` of
  `src/extension/actions.ts`, with `submoduleRisks` and `submoduleUpdateOp(submodule, false)` of `src/git/ops.ts`, runs
  in its place, as before (D49).
- **Offered when**: a commit is recorded for it and it is ahead, behind, both, or does not have that commit.
- **Plan**: `git [-C <parent>] submodule update --init --recursive -- <folder>`, which brings the recorded commit in
  when it does not have it. When git refuses to clone from a folder on disk, as `submodule.allow-file-protocol`.
- **Danger**: None; Confirm when it would leave something behind, saying what: **It has `<n>` commits after the recorded
  one on a detached HEAD: no branch will have them. Create a branch in it first to keep them.**, and **It has changes
  that are not committed: git keeps them, or stops if they are in the way.**
- **Undo**: Not taken back by Undo: the commit it had is in the reflog of the submodule.
- **After**: the submodule shows the green check.
- **Uses**: `submodule.allow-file-protocol`.

Acceptance:

- [ ] On `lib`, one commit behind, it runs `git submodule update --init --recursive -- lib`, and `lib` is on the
      recorded commit afterwards.
- [ ] On `lib` with two commits on a detached HEAD after the recorded one, it asks first and says they will be on no
      branch.
- [ ] On `lib` with two commits on its branch `main`, it does not ask.
- [ ] On `vendor/a/b`, it runs `git -C vendor/a submodule update --init --recursive -- b`.
- [ ] The entry is not offered on a submodule on its recorded commit.
- [ ] Extension: `vendor/lib` one commit behind its recorded one, on a detached `HEAD`, is updated by
      `git submodule update --init --recursive -- vendor/lib` in the engine, without a question, written to the
      **Gitardo** output channel, and is then on the recorded commit.
- [ ] Extension: with 2 commits after the recorded one on a detached `HEAD`, it asks first, even with
      `gitardo.commandPreview` at `off`, and the dialog says **It has 2 commits after the recorded one on a detached
      HEAD**; with files not committed in it, it asks first and says **It has changes that are not committed**.
- [ ] Extension: it asks as a question, not with a warning.
- [ ] Extension: run from a menu drawn before on a submodule now on its recorded commit, it says *`vendor/lib` is on
      the recorded commit already.* and runs nothing.
- [ ] Extension: right after it, **Undo** says it does not take it back, rather than taking back the operation before
      it.
- [ ] Extension: with `gitardo.engine.path` naming a missing file, `submoduleUpdateOp` runs, as do the other operations
      of this file with their `GitOp`.

### `submodule.stage-recorded` — Stage the commit of a submodule as the recorded one

Stages the commit the submodule has checked out as the one the repository records for it, to commit it in the
repository.

- **Kind**: Operation
- **Function**: `features::submodule::stage_recorded(context, answers) -> Outcome`
- **Entry points**: **Stage `<short>` as the recorded commit** in the context menu of a submodule, `<short>` being the
  commit it has checked out.
- **Extension**: **Stage `<short>` as the recorded commit** in the context menu of a submodule in the Submodules
  section. It sends `feature/run` of `submodule.stage-recorded` to Gitardo's engine on the submodule by its path
  (`submoduleRecord` in `engineRuns`, run by `Controller.runOnEngine`), its plan shown in a modal dialog first when
  `gitardo.commandPreview` is `all` (`VscodeAsker`). From a view drawn before, the engine refuses one not initialized
  and one on its recorded commit, **`<folder>` is on the recorded commit already.** When the engine's binary is
  missing, `submoduleRecordOp(submodule, head)` of `src/git/ops.ts` runs in its place, as before (D49).
- **Offered when**: it is initialized and is ahead, behind, both, or does not have the recorded commit.
- **Plan**: `git [-C <parent>] add -- <folder>`. The plan's note: **Commit it to record that commit for the submodule.**
- **Danger**: None.
- **Undo**: Not taken back by Undo: unstaging the submodule in the Changes page takes it back.
- **After**: the submodule is among the staged changes of the repository; its tooltip says the commit staged.
- **Refuses**: on one that is not initialized, **`<folder>` is not initialized.**
- **Uses**: `changes.`.

Acceptance:

- [ ] On `lib`, two commits ahead at `9b1e4d2`, the entry reads **Stage 9b1e4d2 as the recorded commit** and runs
      `git add -- lib`.
- [ ] After it, committing in the repository records `9b1e4d2` for `lib`, and `lib` shows the green check.
- [ ] Extension: on `vendor/lib` one commit ahead, it runs `git add -- vendor/lib` in the engine, written to the
      **Gitardo** output channel; `git status` then has `M  vendor/lib`, and after a commit of the repository it is
      green again.
- [ ] Extension: for `inner` inside `vendor/lib` it runs `git -C vendor/lib add -- inner`.
- [ ] Extension: it is not offered for a submodule on its recorded commit; run from a menu drawn before, it says
      *`vendor/lib` is on the recorded commit already.* and runs nothing.
- [ ] Extension: on a submodule not initialized it says *`vendor/lib` is not initialized.* and runs nothing.
- [ ] Extension: **Undo** right after it does not unstage the submodule.

### `submodule.update-to-latest` — Update a submodule to the latest of its branch

Takes the submodule to the last commit of the branch it follows on its remote, and its own submodules to what that
commit records.

- **Kind**: Operation
- **Function**: `features::submodule::update_to_latest(context, answers) -> Outcome`
- **Entry points**: **Update to the latest of `<branch>`** in the context menu of a submodule, the branch `.gitmodules`
  says it follows; **Update to the latest of its remote** when it says none.
- **Extension**: **Update to the latest of `<branch>`**, or **Update to the latest of its remote**, in the context menu
  of a submodule in the Submodules section. It sends `feature/run` of `submodule.update-to-latest` to Gitardo's engine
  on the submodule by its path (`submoduleUpdate` with `remote` in `engineRuns`, run by `Controller.runOnEngine`), its
  plan shown in a modal dialog first when `gitardo.commandPreview` is `all` (`VscodeAsker`). From a view drawn before,
  the engine refuses one not initialized, **`<folder>` is not initialized: update it first.** When the engine's binary
  is missing, `submoduleUpdateOp(submodule, true)` of `src/git/ops.ts` runs in its place, as before (D49).
- **Offered when**: it is initialized.
- **Plan**: `git [-C <parent>] submodule update --init --remote -- <folder>`, then `git -C <folder> submodule update
  --init --recursive`. The plan's note: **Stage the submodule afterwards to record the commit it is on now.** When git
  refuses to clone from a folder on disk, as `submodule.allow-file-protocol`.
- **Danger**: None.
- **Undo**: Not taken back by Undo: the commit it had is in the reflog of the submodule.
- **After**: the submodule shows how far it is now from the recorded commit; `submodule.stage-recorded` records it.
- **Uses**: `submodule.allow-file-protocol`, `submodule.stage-recorded`.

Acceptance:

- [ ] On `lib`, following `main` whose remote has three commits more, it fetches them and `lib` shows an arrow up with
      3.
- [ ] On a submodule with no branch in `.gitmodules`, the entry reads **Update to the latest of its remote**.
- [ ] Extension: on `vendor/lib`, following `main`, the entry reads **Update to the latest of main** and runs
      `git submodule update --init --remote -- vendor/lib`, then `git -C vendor/lib submodule update --init
      --recursive`, in the engine, written to the **Gitardo** output channel.
- [ ] Extension: run from a menu drawn before on a submodule not initialized, it says *`vendor/lib` is not
      initialized: update it first.* and runs nothing.
- [ ] Extension: right after it, **Undo** says it does not take it back, rather than taking back the operation before
      it.

### `submodule.initialize` — Initialize a submodule

Clones a submodule that is not initialized and checks out the commit recorded for it, with its own submodules.

- **Kind**: Operation
- **Function**: `features::submodule::initialize(context, answers) -> Outcome`
- **Entry points**: **Initialize** in the context menu of a submodule.
- **Extension**: **Initialize** in the context menu of a submodule in the Submodules section. It sends `feature/run` of
  `submodule.initialize` to Gitardo's engine on the submodule by its path (`submoduleUpdate` in `engineRuns`, for a
  submodule with no commit checked out, run by `Controller.runOnEngine`), its plan shown in a modal dialog first when
  `gitardo.commandPreview` is `all` (`VscodeAsker`). When the engine's binary is missing, `updateSubmodule(submodule)`
  of `src/extension/actions.ts`, with `submoduleUpdateOp(submodule, false)` of `src/git/ops.ts`, runs in its place, as
  before (D49).
- **Offered when**: it is not initialized.
- **Plan**: `git [-C <parent>] submodule update --init --recursive -- <folder>`. When git refuses to clone from a folder
  on disk, as `submodule.allow-file-protocol`.
- **Danger**: None.
- **Undo**: Not taken back by Undo: `submodule.deinitialize` empties it again.
- **After**: the submodule shows where it stands, usually the green check.
- **Uses**: `submodule.allow-file-protocol`.

Acceptance:

- [ ] On `lib`, not initialized after a clone without `--recursive`, it clones it and `lib` shows the green check.
- [ ] Its own submodules are initialized too.
- [ ] Extension: **Initialize** on the deinitialized `vendor/lib` runs `git submodule update --init --recursive --
      vendor/lib` in the engine, and `vendor/lib` and `vendor/lib/inner` are then on their recorded commits.
- [ ] Extension: for `inner` inside `vendor/lib` it runs `git -C vendor/lib submodule update --init --recursive --
      inner`.
- [ ] Extension: git refusing with `transport 'file' not allowed` is followed by the question of
      `submodule.allow-file-protocol`, with `git -c protocol.file.allow=always -C vendor/lib submodule update --init
      --recursive -- inner`; run, `inner` is initialized.
- [ ] Extension: right after it, **Undo** says it does not take it back, rather than taking back the operation before
      it.

### `submodule.deinitialize` — Deinitialize a submodule

Empties the submodule's folder and forgets it locally; it stays in `.gitmodules` and in the commits.

- **Kind**: Operation
- **Function**: `features::submodule::deinitialize(context, answers) -> Outcome`
- **Entry points**: **Deinitialize…** in the context menu of a submodule.
- **Extension**: **Deinitialize…**, marked as an entry that loses work, in the context menu of a submodule in the
  Submodules section. First the extension has VS Code's Git extension close the submodule, and each initialized one
  inside it, that it has open, since Windows refuses to empty a folder it watches (`Controller.letGoInVSCodeGit` in
  `src/extension/controller.ts`, `closeInVSCodeGit` in `src/extension/actions.ts`, which runs `git.close` and logs
  `> git.close <folder>`); then it sends `feature/run` of `submodule.deinitialize` to Gitardo's engine on the submodule
  by its path (`submoduleDeinit` in `engineRuns`, run by `Controller.runOnEngine`), its question always asked in a
  modal dialog (`VscodeAsker`), and the forced variant, **Deinitialize `<folder>`, losing its changes**, asked at once
  with a warning when git refuses. On Windows the engine runs a command again while the folder is held
  (`worktree.recover-held-folder`). When the engine's binary is missing, `lettingGoOf(submoduleDeinitOp(submodule),
  submodule, state)` of `src/extension/actions.ts` and `src/git/ops.ts` runs in its place, as before (D49).
- **Offered when**: it is initialized.
- **Plan**: `git [-C <parent>] submodule deinit -- <folder>`. When git refuses because the submodule has local
  modifications, Gitardo offers `git [-C <parent>] submodule deinit --force -- <folder>`, Destructive, because **The
  submodule has changes that are not committed: they will be lost. Its commits stay in its clone in the git
  directory.** On Windows, a command that fails because the folder is held (`could not remove`, `Permission denied`,
  `Device or resource busy`) is run again, up to 20 times, 200 ms apart, each time said in the log.
- **Danger**: Confirm, because its folder is emptied: **Its folder is emptied, and it stays in .gitmodules and in the
  commits: update it to get it back. Its commits stay in its clone in the git directory.**
- **Undo**: Not taken back by Undo: `submodule.initialize` brings it back from its clone.
- **After**: the submodule shows as not initialized.

Acceptance:

- [ ] On a clean `lib`, after confirming, it runs `git submodule deinit -- lib`, the folder `lib` is empty, and
      `.gitmodules` is unchanged.
- [ ] On `lib` with a modified file, git's refusal is followed by the offer with `--force`, shown as destructive.
- [ ] After it, **Initialize** is offered and brings `lib` back.
- [ ] Extension: **Deinitialize…** on `vendor/lib` asks **Deinitialize vendor/lib?** with `git submodule deinit --
      vendor/lib`, even with `gitardo.commandPreview` at `off`; run, its folder is empty and it is listed as not
      initialized, with its recorded commit kept.
- [ ] Extension: with a file changed in it, git refuses, and Gitardo warns **Deinitialize vendor/lib, losing its
      changes?** with `git submodule deinit --force -- vendor/lib`.
- [ ] Extension: the forced variant is asked with a warning, which agrees with what loses work; the README's list of
      what asks names it.
- [ ] Extension: the `git.close` line of each folder VS Code's Git had open, then each command of the engine, are
      written to the **Gitardo** output channel; the commands shown are the ones that run.
- [ ] Extension: on Windows, with the folder of the submodule held for a moment, the deinit is tried again and
      succeeds, each try said in the log.
- [ ] Extension: right after it, **Undo** says it does not take it back, rather than taking back the operation before
      it.

### `submodule.remove` — Remove a submodule

Deletes the submodule from the repository: its folder, its entry in `.gitmodules`, its record, and its clone in the git
directory; the removal is staged, ready to commit.

- **Kind**: Operation
- **Function**: `features::submodule::remove(context, answers) -> Outcome`
- **Entry points**: **Remove…** in the context menu of a submodule.
- **Extension**: **Remove…**, marked as an entry that loses work, in the context menu of a submodule in the Submodules
  section, initialized or not. First VS Code's Git extension closes it, and each initialized one inside it, as for
  `submodule.deinitialize` (`Controller.letGoInVSCodeGit`, `closeInVSCodeGit`); then it sends `feature/run` of
  `submodule.remove` to Gitardo's engine on the submodule by its path (`submoduleRemove` in `engineRuns`, run by
  `Controller.runOnEngine`), its warning always asked in a modal warning dialog (`VscodeAsker`). The engine deletes the
  clone once the plan succeeded, and a clone it cannot delete is only said in the log; on Windows it runs a command
  again while the folder is held. When the engine's binary is missing, `lettingGoOf(submoduleRemoveOp(submodule,
  gitDir), submodule, state)`, with the `finish` of the `submoduleRemove` case of `resolveAction` in
  `src/extension/actions.ts` that deletes the clone, runs in its place, as before (D49).
- **Reads**: the git directory of the repository that has it, `git [-C <parent>] rev-parse --absolute-git-dir`, for its
  clone in `modules/<name>`, named as `.gitmodules` names it.
- **Plan**: `git [-C <parent>] submodule deinit --force -- <folder>`, then `git [-C <parent>] rm --force -- <folder>`,
  then `delete <git directory>/modules/<name>` when that folder is there and is inside `modules`: a name that leads out
  of it is no clone to delete. A deletion that fails is said in the log and stops nothing. On Windows, a command that
  fails because the folder is held is run again as in `submodule.deinitialize`.
- **Danger**: Destructive, because **Its folder is deleted, with the changes and the commits in it that are not
  pushed. Gitardo then deletes its clone in `<folder>`. The removal is staged along with .gitmodules: commit them to
  record it.**
- **Undo**: Cannot be undone: its clone is deleted. What was pushed of it can be added again.
- **After**: the submodule is no longer listed, and the Changes page has the removal and `.gitmodules` staged.

Acceptance:

- [ ] On `lib`, after the warning, it runs `git submodule deinit --force -- lib` and `git rm --force -- lib`, logs
      `delete <repo>/.git/modules/lib`, and the folder `.git/modules/lib` is gone.
- [ ] `.gitmodules` no longer has `lib`, and both changes are staged.
- [ ] Adding a submodule at `lib` again afterwards clones it anew rather than taking back the old clone.
- [ ] A submodule whose name in `.gitmodules` is `../../x` has no clone deleted, and nothing outside `.git/modules` is
      touched.
- [ ] Extension: **Remove…** on `vendor/lib` warns **Remove submodule vendor/lib?**, naming its clone in
      `.git/modules`, with `git submodule deinit --force -- vendor/lib` and `git rm --force -- vendor/lib`, even with
      `gitardo.commandPreview` at `off`; it asks with a warning, which agrees with what loses work, and the README's
      list of what asks names it.
- [ ] Extension: run, the folder `vendor/lib` and `.git/modules/vendor/lib` are gone, and the staged files are
      `.gitmodules` and `vendor/lib`.
- [ ] Extension: for `inner` inside `vendor/lib` the commands are `git -C vendor/lib submodule deinit --force -- inner`
      and `git -C vendor/lib rm --force -- inner`.
- [ ] Extension: each `git.close` line, each command and the `delete` line are written to the **Gitardo** output
      channel; the commands shown are the ones that run.
- [ ] Extension: a clone in `.git/modules` that cannot be deleted is said in the log, and the submodule is removed and
      staged all the same, with no failure shown.
- [ ] Extension: right after it, **Undo** says it cannot take it back, rather than taking back the operation before
      it.

### `submodule.copy-path` — Copy the path of a submodule

Puts the submodule's folder, from the root of the repository, on the clipboard.

- **Kind**: Operation
- **Function**: `features::submodule::copy_path(context, answers) -> Outcome`
- **Entry points**: **Copy path** in the context menu of a submodule.
- **Extension**: stays in the extension, in TypeScript: `copy(text)` in `src/webview/menus.ts` posts `copy`, which
  `Controller.onMessage` in `src/extension/controller.ts` writes to the clipboard; no git runs.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the path is on the clipboard.

Acceptance:

- [ ] On `vendor/a/b`, the clipboard holds `vendor/a/b`.
- [ ] Extension: **Copy path** on `vendor/lib/inner` puts `vendor/lib/inner` on the clipboard, with forward slashes,
      from the root of the repository shown.

### `submodule.copy-url` — Copy the URL of a submodule

Puts the URL `.gitmodules` gives the submodule on the clipboard.

- **Kind**: Operation
- **Function**: `features::submodule::copy_url(context, answers) -> Outcome`
- **Entry points**: **Copy URL** in the context menu of a submodule.
- **Extension**: stays in the extension, in TypeScript, as `submodule.copy-path` does: `copy(text)` in
  `src/webview/menus.ts`, written to the clipboard by `Controller.onMessage`.
- **Offered when**: `.gitmodules` gives it a URL.
- **Plan**: no steps.
- **Danger**: None.
- **Undo**: Nothing to take back: nothing changes.
- **After**: the URL is on the clipboard.

Acceptance:

- [ ] On `lib`, the clipboard holds its URL as `.gitmodules` writes it, a relative one such as `../lib.git` included.
- [ ] Extension: **Copy URL** puts the URL `.gitmodules` has, and is not in the menu when it has none.

### `submodule.add` — Add a submodule

Clones a repository into a folder of this one as a submodule, with its own submodules, staged with `.gitmodules`.

- **Kind**: Operation
- **Function**: `features::submodule::add(context, answers) -> Outcome`
- **Entry points**: the **+** on the title of the Submodules list; **Add submodule…** in the **…** menu of the header
  while the repository has none; Quick search, **Add Submodule…**.
- **Extension**: the **+** on the title of the Submodules section, **Add submodule…**, while there are submodules;
  **Add submodule…** in the **…** menu of the toolbar of the graph while there are none; **Add Submodule…** in the
  **…** menu on the title of the Branches view; **Gitardo: Add Submodule…** (`gitardo.addSubmodule`) in the Command
  Palette. Each sends `feature/run` of `submodule.add` to Gitardo's engine (`submoduleAdd` in `engineRuns` of
  `src/engine/actions.ts`, run by `Controller.runOnEngine` in `src/extension/controller.ts`); the three questions are
  VS Code's input boxes, each checked with `question/validate` as it is typed, and the plan is shown in a modal dialog
  when `gitardo.commandPreview` is `all` (`VscodeAsker` in `src/engine/vscode.ts`). When the engine's binary is
  missing, `addSubmodule(state)` of `src/extension/actions.ts`, with `submoduleAddOp` and `allowingDisk` of
  `src/git/ops.ts`, runs in its place, as before (D49).
- **Asks**:
  1. **URL of the repository, or its path on disk**, with `https://github.com/owner/repo.git` as a placeholder; refused
     while typed: **Enter a URL** when empty, **That is not a URL** when it starts with `-`.
  2. **Folder to clone it in, from the root of the repository**, by default the name of the repository: the last part
     of the URL or path, without `.git` (`https://github.com/owner/lib.git`, `git@github.com:owner/lib.git` and
     `../lib/` all give `lib`). Backslashes count as slashes. The box says **In `<full path>`**, or refuses: **Enter a
     folder**; **Enter a folder inside the repository** for a path starting with `-` or `/`, a drive letter, or one
     with `..`; **A submodule is there already**; **`<full path>` is not empty**.
  3. **Branch it follows, for updates from its remote. Leave empty for the default branch of the remote.**, refused
     while typed when git would not accept it as a branch name.
- **Plan**: `git submodule add [-b <branch>] -- <url> <folder>`, then `git submodule update --init --recursive --
  <folder>` for its own submodules. The plan's note: **Git clones it and stages it along with .gitmodules: commit them to
  record it in the repository.** When git refuses to clone from a folder on disk, as `submodule.allow-file-protocol`.
- **Danger**: None.
- **Undo**: Not taken back by Undo: `submodule.remove` removes it.
- **After**: the Submodules list has it as added, not committed yet, and the Changes page has it and
  `.gitmodules` staged.
- **Uses**: `submodule.allow-file-protocol`.

Acceptance:

- [ ] With `https://example.com/acme/lib.git`, the folder proposed is `lib`, and it runs `git submodule add --
      https://example.com/acme/lib.git lib` then `git submodule update --init --recursive -- lib`.
- [ ] With the branch `stable`, the first command is `git submodule add -b stable -- <url> lib`.
- [ ] A folder `../lib`, `/tmp/lib` or `C:/lib` is refused while typed, and so is a folder with files in it.
- [ ] A repository whose own submodules are not initialized by `git submodule add` has them initialized by the second
      command.
- [ ] Backing out of any question runs nothing.
- [ ] Extension: the URL `https://example.com/lib.git`, the folder `lib` and the branch `main` run
      `git submodule add -b main -- https://example.com/lib.git lib`, then
      `git submodule update --init --recursive -- lib`, in the engine, written to the **Gitardo** output channel; the
      commands shown are the ones that run.
- [ ] Extension: the folder box is filled with `lib` for `https://github.com/owner/lib.git`,
      `git@github.com:owner/lib.git` and `C:\repos\lib\`.
- [ ] Extension: a folder that is there and not empty is refused with **`<full path>` is not empty**, and `vendor/lib`
      says **In `<full path>`**.
- [ ] Extension: `--upload-pack=evil` as the URL and `-lib` as the folder are refused while they are typed, and nothing
      runs.
- [ ] Extension: a folder with `[` in its name, such as `lib[1]`, is given to the update step as `:(literal)lib[1]`.
- [ ] Extension: the submodules inside the one added are cloned and listed under it.
- [ ] Extension: right after it, **Undo** says it does not take it back, rather than taking back the operation before
      it.
- [ ] Extension: with `gitardo.engine.path` naming a missing file, `submoduleAddOp` runs, as do the other operations of
      this file with their `GitOp`.

### `submodule.sync-urls` — Synchronize the URLs of the submodules

Copies the URLs of `.gitmodules` to the configuration of the submodules, and of theirs, after they changed there.

- **Kind**: Operation
- **Function**: `features::submodule::sync_urls(context, answers) -> Outcome`
- **Entry points**: **Synchronize URLs from .gitmodules** in the context menu of any submodule.
- **Extension**: **Synchronize URLs from .gitmodules** in the context menu of any submodule in the Submodules section.
  It sends `feature/run` of `submodule.sync-urls` to Gitardo's engine, for every submodule whichever menu it was chosen
  from (`submoduleSync` in `engineRuns`, run by `Controller.runOnEngine`), its plan shown in a modal dialog first when
  `gitardo.commandPreview` is `all` (`VscodeAsker`). When the engine's binary is missing, `submoduleSyncOp()` of
  `src/git/ops.ts` runs in its place, as before (D49).
- **Plan**: `git submodule sync --recursive`.
- **Danger**: None.
- **Undo**: Not taken back by Undo: the URLs it replaced are not kept.

Acceptance:

- [ ] After the URL of `lib` is changed in `.gitmodules`, it runs `git submodule sync --recursive`, and
      `git -C lib remote get-url origin` gives the new URL.
- [ ] Extension: after the URL of `vendor/lib` in `.gitmodules` changed to `https://example.com/lib.git`, it runs
      `git submodule sync --recursive` in the engine, written to the **Gitardo** output channel, and
      `git config submodule.vendor/lib.url` then says `https://example.com/lib.git`.

### `submodule.allow-file-protocol` — Clone a submodule from a folder on disk

Git does not clone a submodule from a folder on disk unless allowed, since a repository could point one at any folder
of the computer. When it refuses, Gitardo offers to allow it for those commands only.

- **Kind**: Derivation
- **Function**: `model::submodule::allow_file_protocol(input) -> Option<Plan>`
- **Uses**: by `submodule.add`, `submodule.update-to-recorded`, `submodule.update-to-latest`, `submodule.initialize`
  and `submodule.update-on-checkout`.
- **Extension**: the engine offers it on the failure of the plans it runs for the extension, and the extension asks
  it in a modal dialog, always, as Confirm (`VscodeAsker` in `src/engine/vscode.ts`). For an update after a checkout
  still run in TypeScript, and when the engine's binary is missing (D49), `allowingDisk` of `src/git/ops.ts` offers the
  whole operation again with `-c protocol.file.allow=always`, as before.

The rules: when a step of one of those plans fails with git's message `transport 'file' not allowed`, the plan is
offered again from that step on with `-c protocol.file.allow=always` before each of its git commands, its title followed by **from a
folder on disk**, Confirm, with the reason first in its note: **Git does not clone submodules from folders on disk by
default, since a repository could point one at any folder of this computer. This allows it for these commands only.**
Nothing is changed in the configuration of git.

Example: `git submodule add -- ../lib lib` fails; the offer is `git -c protocol.file.allow=always submodule add --
../lib lib` and `git -c protocol.file.allow=always submodule update --init --recursive -- lib`, titled **Add submodule
lib from a folder on disk**.

Acceptance:

- [ ] Adding `../lib` as a submodule fails, is offered again with `-c protocol.file.allow=always` on both commands, and
      succeeds once confirmed.
- [ ] After it, `git config protocol.file.allow` gives nothing, in the repository and for the user.
- [ ] A failure with any other message offers nothing.
- [ ] Extension: adding a repository in a folder on disk, git refuses, and Gitardo asks **Add submodule vendor/lib from
      a folder on disk?** with `git -c protocol.file.allow=always submodule add --…`; run, the submodule is listed as
      added.
- [ ] Extension: when `git submodule add` succeeded and only the update of the submodules inside it is refused, the
      offer has only `git -c protocol.file.allow=always submodule update --init --recursive -- <folder>`, and
      `git submodule add` does not run again.

### `submodule.update-on-checkout` — Update the submodules after a checkout

Git leaves the submodules where they are on a checkout, so they no longer match the commit checked out. Gitardo updates
them to what that commit records once the checkout is done.

- **Kind**: Derivation
- **Function**: `model::submodule::update_on_checkout(input) -> Plan`
- **Reads**: whether the commit checked out has submodules, `git cat-file -e <commit>:.gitmodules`, when the repository
  has none now.
- **Settings**: `submodules.updateOnCheckout`, on by default.
- **Uses**: `submodule.allow-file-protocol`; by `branch.checkout`, `branch.create` when it checks the branch out,
  `stash.branch` and `undo.last` of a checkout.
- **Extension**: the setting is `gitardo.submodules.updateOnCheckout`, sent to the engine as
  `submodules.updateOnCheckout` (`src/engine/settings.ts`). The engine adds the update to the plans it runs for the
  extension: `branch.create` with **Create and check out**, `stash.branch` and `undo.last`. The checkouts still run in TypeScript (`branch.checkout`,
  `branch.checkout-remote`, `branch.checkout-detached`) get it from `updatingSubmodules(op, to, state, repo)` in
  `src/extension/actions.ts`, which sets the `then` of the checkout to `submodulesUpdateOp()` of `src/git/ops.ts`, with
  `submoduleRisks(submodule, true)` for what it leaves behind and `Repo.hasSubmodulesAt(to)` for a commit that has a
  `.gitmodules`, run by `Controller.runWithFallback(ran.then, true)` once the checkout succeeded; the question of the
  checkout says it after **and once that is done:** (`Controller.describe`), and a failure says **Update the
  submodules failed.** with the first error line of git.

The rules:

- When the setting is on and the repository has submodules before or after the checkout, the plan of the checkout ends
  with `git submodule update --init --recursive`, which also clones the ones that commit adds and the ones inside them.
  The preview of the checkout shows that command too.
- The plan's note adds **Then the submodules go to the commits recorded for them.**, and, for each submodule by its
  folder, what that leaves behind: **`<folder>` has `<n>` commits after the recorded one on a detached HEAD: no branch
  will have them. Create a branch in it first to keep them.**, and **`<folder>` has changes that are not committed: git
  keeps them, or stops if they are in the way.**
- The checkout is asked first (Confirm) when a submodule has commits on a detached HEAD that no branch would have then,
  whatever `commandPreview` says; files not committed alone do not make it ask.
- When the update fails, the checkout is done all the same, and the failure is shown.
- With the setting off, or without submodules on either side, the plan is the checkout alone.

Acceptance:

- [ ] Checking out a branch on which `lib` records another commit ends with `lib` on that commit, and the log has
      `git submodule update --init --recursive` after the checkout.
- [ ] Checking out a commit that adds a submodule clones it.
- [ ] With `lib` two commits ahead on a detached HEAD, the checkout asks first and names `lib`.
- [ ] With `submodules.updateOnCheckout` off, the checkout runs alone and `lib` stays where it was.
- [ ] In a repository without submodules on either side, no update runs.
- [ ] Extension: checking out `with-lib`, whose commit records `vendor/lib` that is not cloned here, runs
      `git checkout with-lib` and then `git submodule update --init --recursive`; from a folder on disk git refuses,
      Gitardo asks **Update the submodules from a folder on disk?**, and run, `vendor/lib` is on the commit `with-lib`
      records.
- [ ] Extension: with `gitardo.submodules.updateOnCheckout` off, checking out `with-lib-2` runs no `submodule update`,
      and `vendor/lib` is one commit behind what `with-lib-2` records.
- [ ] Extension: with `gitardo.commandPreview` at `all`, the question of the checkout lists `git submodule update
      --init --recursive` after **and once that is done:**, and nothing else runs.
- [ ] Extension: a submodule with 1 commit after its recorded one on a detached `HEAD` makes the checkout ask first,
      even with `gitardo.commandPreview` at `off`, saying **`<folder>` has 1 commit after the recorded one on a detached
      HEAD**.
- [ ] Extension: the command of the update is written to the **Gitardo** output channel after the checkout's.
- [ ] Extension: right after the checkout, **Undo** goes back to where it was and updates the submodules again.

### `submodule.follow-changes` — Follow what happens in the submodules

A commit or a checkout made in a submodule, in its own window or in a terminal, updates its icon here.

- **Kind**: Derivation
- **Function**: `model::submodule::follow_changes(input) -> bool`
- **Uses**: `repo.watch-changes`.

The rules: besides what changes in the repository itself, a change of `modules/<…>/HEAD` or of a file under
`modules/<…>/refs/` in the git directory of the repository, at any depth of submodules inside submodules, means the
repository is read again; a file written in a submodule's folder does too, after the delay files in the working tree
have, so that its count of files not committed follows. Lock files (`*.lock`) and the files of the file system monitor
of git mean nothing.

Acceptance:

- [ ] A commit made in `lib` from a terminal turns its icon into an arrow up with 1 without any action in Gitardo.
- [ ] A checkout of another branch in `vendor/a/b` from a terminal shows that branch beside it.
- [ ] A file changed in `lib` changes its pencil count.
- [ ] A `.lock` file appearing in `.git/modules/lib` reads nothing again.
