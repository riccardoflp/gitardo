# Submodules

The repositories inside the repository: where each stands against the commit recorded for it, the Submodules section of
the references with its context menu, adding one, bringing one to its recorded commit or to the latest of its branch,
recording the commit it is on, deinitializing and removing one, and updating them all after a checkout. Opening a
submodule in a window, a terminal or the file manager runs no git.

A submodule inside another is listed under it, and its commands run from that one: `git -C <outer> …` with its path
from there. In the plans below, `<path>` is the path of the submodule from the repository that has it, passed as
`:(literal)<path>` when it is also a pattern to git.

Git refuses to clone a submodule from a folder on disk unless allowed, since a repository could point one at any folder
of the computer. Every operation here that clones offers, when git refuses with `transport 'file' not allowed`, the
same commands with `-c protocol.file.allow=always`, asked first as **<title> from a folder on disk?** with the note
**Git does not clone submodules from folders on disk by default, since a repository could point one at any folder of
this computer. This allows it for these commands only.** (`allowingDisk` in `src/git/ops.ts`).

### `submodule.read` — What is read of each submodule

Reads the submodules of the repository, and theirs, with what each records and has checked out, each time the
repository is read.

- **Kind**: Read
- **Function**: `Repo.submodules(parent, depth)` in `src/git/repo.ts`, with `parseGitmodules`, `parseGitlinks` and
  `parseBranchStatus` in `src/git/parse.ts`.
- **Reads**: in each repository that has a `.gitmodules`, starting with the one shown and going into each initialized
  submodule, down to four levels of submodules inside submodules:
  - `git config -z --file .gitmodules --get-regexp ^submodule\.`: the name, path, URL and branch of each; an entry
    without a path is left out, and a name may have dots in it.
  - `git ls-tree -z HEAD -- <paths>`: the commit recorded for each in `HEAD`, none before the first commit or when it
    is added and not committed.
  - `git ls-files -s -z -- :(literal)<path>…`: the commit staged for each, kept when it is not the recorded one.
  - In each folder that has a `.git`: `git status --porcelain=v2 --branch -z --untracked-files=all`, for the commit
    checked out, the branch (none on a detached `HEAD`) and how many files are not committed, a rename counted once.
  - When the commit checked out is not the recorded one: `git rev-list --left-right --count <recorded>...<head> --`,
    for how many commits it is behind and ahead; both unknown when that fails, because it does not have the recorded
    commit.
  Every read runs with `GIT_OPTIONAL_LOCKS=0`. The repository is read again when `HEAD` or the references of a clone
  in `.git/modules` change, so a commit or a checkout made in a submodule, in its window or a terminal, shows.
- **Implemented in**: `Repo.submodules`, called by `Repo.state`; `Submodule` in `src/shared/protocol.ts`; the watcher
  of `Controller`.
- **Tested by**: `test/submodules.test.ts` "reads .gitmodules, names with dots included, and leaves out entries
  without a path", "keeps the commits recorded for submodules, from the tree or the index", "reads the commit, the
  branch and the changes of a submodule, counting a rename once", "has none without .gitmodules", "adds a submodule
  with the ones inside it, and follows it against the commit recorded for it" and "cannot compare a submodule with a
  recorded commit it does not have".

Acceptance:

- [ ] A repository without `.gitmodules` has no submodules, and no Submodules section.
- [ ] `vendor/lib`, which has the submodule `inner`, is read as `vendor/lib` and then `vendor/lib/inner`, whose parent
  is `vendor/lib`.
- [ ] A submodule not initialized lists none of the submodules inside it.
- [ ] A submodule whose recorded commit it has never fetched has neither an ahead nor a behind count.
- [ ] A file changed in a submodule counts as one file not committed.

### `submodule.status` — Where a submodule stands

Turns what was read of a submodule into where it stands against the commit recorded for it, as an icon, a number and a
sentence.

- **Kind**: Derivation
- **Function**: `submoduleStatus(submodule)` and `submoduleStatusText(submodule)` in `src/shared/submodules.ts`; the
  icon by `submoduleBadge` in `src/webview/RefsTree.tsx`.
- **Implemented in**: `src/shared/submodules.ts`, `src/webview/RefsTree.tsx`.
- **Tested by**: `test/submodules.test.ts` "says where a submodule stands, and how deep it is";
  `test/e2e/ui.mjs` "a submodule is added from the real prompts, says where it stands against the recorded commit, and
  is removed".

Rules of its result:

- Input: a `Submodule` (`submodule.read`). Output: one of seven states, checked in this order:

  | State | When | Icon, number | Sentence |
  | --- | --- | --- | --- |
  | uninitialized | nothing is checked out in it | grey `circle-slash` | **Not initialized: its folder has no checkout** |
  | added | no commit is recorded for it | grey `diff-added` | **Added, not committed yet** |
  | unknown | ahead or behind could not be counted | yellow `warning` | **The recorded commit <short> is not in it: fetch in it, or update it** |
  | diverged | ahead and behind both above 0 | yellow `arrow-swap`, `<ahead>↑ <behind>↓` | **<n> commits ahead of and <m> commits behind the recorded commit <short>** |
  | ahead | ahead above 0 | yellow `arrow-up`, the count | **<n> commits ahead of the recorded commit <short>** |
  | behind | behind above 0 | yellow `arrow-down`, the count | **<n> commits behind the recorded commit <short>** |
  | synced | on the recorded commit | green `pass-filled` | **On the recorded commit <short>** |

- `<short>` is the first 7 characters of the recorded commit; one commit is said **1 commit**.
- How deep a submodule is, for its indent, is the number of submodules it is inside (`submoduleDepth`): 0 for one of
  the repository itself.

Worked examples:

- Head `bbbbbbb…`, recorded `aaaaaaa…`, ahead 1, behind 0 → ahead, **1 commit ahead of the recorded commit aaaaaaa**.
- The same with ahead 2 and behind 3 → diverged.
- The same with ahead and behind unknown → unknown.
- No head → uninitialized, whatever else is known.
- A head and nothing recorded → added.
- `vendor/lib/inner` inside `vendor/lib` → depth 1; `vendor/lib` → depth 0.

### `submodule.section` — The Submodules section

The section of the references, beside the graph and in the Branches view, that lists the submodules with where each
stands, and opens their context menu.

- **Kind**: View
- **Function**: the `submodules` section of `RefsTree` in `src/webview/RefsTree.tsx`, with `submoduleMenu(submodule,
  state)` in `src/webview/menus.ts`.
- **Entry points**: the references beside the graph; the Branches view.
- **Offered when**: the repository has at least one submodule; closed when VS Code starts, as every section.
- **Shows**:
  - One entry per submodule, in the order read: its path, or for one inside another its path from that one, indented
    under it.
  - The icon of `submodule.status`, with its number; beside it the branch checked out in it, or `detached`, nothing
    when it is not initialized; and the number of its files not committed, when there are some.
  - A tooltip of its path, its URL, the sentence of `submodule.status`, **Checked out: <branch or detached> at
    <short>**, **Staged as the recorded commit: <short>** when another commit is staged for it, and **Nothing to
    commit** or **<n> files not committed**.
  - A double click opens an initialized one in a new window (`submodule.open-window`).
  - The **+** on the title of the section, **Add submodule…** (`submodule.add`).
  - The context menu, in this order, each entry only when it applies: **Open in new window**, **Open in terminal**,
    **Reveal in File Explorer** (**Reveal in Finder** on macOS, **Open containing folder** elsewhere), **Show in
    Gitardo**; **Initialize**, **Update to the recorded commit <short>**, **Stage <short> as the recorded commit**,
    **Update to the latest of <branch>**, **Synchronize URLs from .gitmodules**; **Copy path**, **Copy URL**;
    **Deinitialize…** and **Remove…**, both marked as entries that lose work.
- **Implemented in**: `RefsTree` and `submoduleMenu`; `SECTIONS` in `src/webview/BranchesApp.tsx`.
- **Tested by**: `test/e2e/ui.mjs` "a submodule is added from the real prompts, says where it stands against the
  recorded commit, and is removed".

Acceptance:

- [ ] Without submodules there is no Submodules section.
- [ ] A submodule just added shows the grey added icon; once committed, the green check with **On the recorded
  commit**; after a commit made in it, a yellow arrow up and `1`.
- [ ] Its context menu has **Open in new window**, **Show in Gitardo**, **Update to the recorded commit** and
  **Remove…** for a submodule one commit ahead.
- [ ] A submodule not initialized has only **Initialize**, **Synchronize URLs from .gitmodules**, **Copy path**,
  **Copy URL** and **Remove…** in its menu, and a double click does nothing.
- [ ] A commit made in a submodule in a terminal changes its icon without a refresh.

### `submodule.open-window` — Open a submodule in a new window

Opens the folder of a submodule in a new VS Code window.

- **Kind**: View
- **Function**: `openWorktree(folder, 'window')` in `src/extension/actions.ts`, from the `submoduleOpen` case of
  `resolveAction`.
- **Entry points**: **Open in new window** in the context menu of a submodule; a double click on it.
- **Offered when**: it is initialized.
- **Plan**: no git command: VS Code's `vscode.openFolder` with a new window. It is no `GitOp`, so it is not asked and
  not logged.
- **Refuses**: one that is not initialized, as from a view drawn before: **<path> is not initialized: update it
  first.** A path that is no longer a submodule: **<path> is no longer a submodule of this repository.**
- **Implemented in**: the `submoduleOpen` case of `Action` and of `resolveAction`; `submoduleMenu`; `open` of the
  entry in `src/webview/RefsTree.tsx`.
- **Tested by**: *none*.

Acceptance:

- [ ] **Open in new window** on `vendor/lib` opens `<root>/vendor/lib` in a new window.
- [ ] It is not in the menu of a submodule that is not initialized.

### `submodule.terminal` — Open a terminal in a submodule

Opens a terminal of VS Code in the folder of a submodule.

- **Kind**: View
- **Function**: the `worktreeTerminal` case of `resolveAction` in `src/extension/actions.ts`, given the folder of the
  submodule.
- **Entry points**: **Open in terminal** in the context menu of a submodule.
- **Offered when**: it is initialized.
- **Plan**: no git command: a terminal named after the last part of its folder, opened in it and shown.
- **Implemented in**: `submoduleMenu`; the `worktreeTerminal` case of `resolveAction`.
- **Tested by**: *none*.

Acceptance:

- [ ] **Open in terminal** on `vendor/lib` shows a terminal named `lib` whose folder is `<root>/vendor/lib`.

### `submodule.reveal` — Reveal a submodule in the file manager

Shows the folder of a submodule in the file manager of the system.

- **Kind**: View
- **Function**: the `worktreeReveal` case of `resolveAction` in `src/extension/actions.ts`, which runs VS Code's
  `revealFileInOS`.
- **Entry points**: **Reveal in File Explorer** on Windows, **Reveal in Finder** on macOS, **Open containing folder**
  elsewhere, in the context menu of a submodule.
- **Offered when**: it is initialized.
- **Implemented in**: `submoduleMenu` and `REVEAL` in `src/webview/menus.ts`; the `worktreeReveal` case of
  `resolveAction`.
- **Tested by**: *none*.

Acceptance:

- [ ] On Windows the entry reads **Reveal in File Explorer** and shows the folder of the submodule.

### `submodule.show-in-gitardo` — Show a submodule in Gitardo

Shows the submodule in the views of this window, as the repository they show, with the way back.

- **Kind**: View
- **Function**: `Controller.selectRepo(root)` in `src/extension/controller.ts`, from the `selectRepo` message.
- **Entry points**: **Show in Gitardo** in the context menu of a submodule.
- **Offered when**: it is initialized.
- **Shows**: when the submodule is one of the repositories of the workspace, the views show it as when it is chosen in
  the list of repositories. Otherwise the views show it and remember the repository shown before: a banner above the
  graph says **Showing the worktree <name>** and has **Back to <repository>**. Nothing happens while an operation runs.
- **Implemented in**: `submoduleMenu`; `Controller.selectRepo`; `WorktreeBanner` in `src/webview/WorktreeBanner.tsx`.
- **Tested by**: *none*.

Acceptance:

- [ ] **Show in Gitardo** on `vendor/lib` makes the graph show the commits of `vendor/lib`.
- [ ] **Back to <repository>** shows the repository that has it again.

### `submodule.copy` — Copy the path or the URL of a submodule

Puts the path of a submodule, or its URL, on the clipboard.

- **Kind**: View
- **Function**: `copy(text)` in `src/webview/menus.ts`, which posts `copy`, written to the clipboard by
  `Controller.onMessage`.
- **Entry points**: **Copy path** and **Copy URL** in the context menu of a submodule.
- **Offered when**: **Copy path** always; **Copy URL** when `.gitmodules` gives it a URL.
- **Plan**: no git command.
- **Implemented in**: `submoduleMenu`; the `copy` case of `Controller.onMessage`.
- **Tested by**: *none*.

Acceptance:

- [ ] **Copy path** on `vendor/lib/inner` puts `vendor/lib/inner` on the clipboard, with forward slashes, from the
  root of the repository shown.
- [ ] **Copy URL** puts the URL `.gitmodules` has, and is not in the menu when it has none.

### `submodule.add` — Add a submodule

Clones a repository into a folder of this one as a submodule, with the submodules it has, staged and ready to commit.

- **Kind**: Operation
- **Function**: `submoduleAddOp(url, dir, branch)` in `src/git/ops.ts`, asked for by `addSubmodule(state)` in
  `src/extension/actions.ts`.
- **Entry points**: the **+** on the title of the Submodules section, **Add submodule…**; **Add submodule…** in the
  **…** menu of the toolbar of the graph; **Add Submodule…** in the **…** menu on the title of the Branches view;
  **Gitardo: Add Submodule…** in the Command Palette.
- **Offered when**: the **+** while there are submodules, the menu of the toolbar while there are none; the others
  always.
- **Asks**, in three input boxes titled **Add submodule**:
  1. **URL of the repository, or its path on disk**, with the placeholder **https://github.com/owner/repo.git**;
     refused as typed with **Enter a URL** when empty and **That is not a URL** when it starts with `-`.
  2. **Folder to clone it in, from the root of the repository**, filled with the last part of the URL without `.git`
     (`repoNameOfUrl`). Backslashes become `/` and a `/` at the end is dropped. Refused with **Enter a folder** when
     empty; **Enter a folder inside the repository** when it starts with `-` or `/`, has a drive letter or a `..`
     part; **A submodule is there already** for the path of a submodule of the repository; **<full path> is not
     empty** for a file, or a folder with something in it. Otherwise it says **In <full path>**.
  3. **Branch it follows, for updates from its remote. Leave empty for the default branch of the remote.**, refused
     with **That is not a valid Git ref name** when it is not one.
  Escape at any of them cancels, and nothing runs.
- **Plan**: `> git submodule add -- <url> <folder>`, with `-b <branch>` before `--` when a branch was given, then
  `> git submodule update --init --recursive -- <folder>`. From a folder on disk, after asking, the same two with
  `-c protocol.file.allow=always` after `git`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`); the question has the note **Git clones it and
  stages it along with .gitmodules: commit them to record it in the repository.** The variant from a folder on disk
  is Confirm, always asked.
- **Undo**: Gitardo remembers nothing, and the reflog has nothing of it: **Undo** right after it offers to take back
  the operation before it. `submodule.remove` takes it out again, its clone included.
- **After**: the submodule and `.gitmodules` are staged; the section lists it as added, and the submodules inside it,
  cloned too, under it.
- **Refuses**: the input boxes above; a URL or folder starting with `-` that reaches `submoduleAddOp` is refused with
  *"<value>" is not a valid name.*
- **Settings**: `gitardo.commandPreview`.
- **Uses**: `submodule.read`, `preview.ask`, `log.write`.
- **Implemented in**: the `submoduleAdd` case of `Action`; the `submoduleAdd` case of `resolveAction` and
  `addSubmodule`; `submoduleAddOp` and `allowingDisk` (`src/git/ops.ts`); the section action in
  `src/webview/RefsTree.tsx`; the **…** menu of `src/webview/Toolbar.tsx`; the command `gitardo.addSubmodule` in
  `src/extension/extension.ts` and `package.json`; the fallback run by `Controller.runWithFallback`.
- **Tested by**: `test/submodules.test.ts` "refuses a URL or a folder that git would take for an option", "names the
  folder of a submodule after its URL", "adds a submodule with the ones inside it, and follows it against the commit
  recorded for it"; `test/e2e/ui.mjs` "a submodule is added from the real prompts, says where it stands against the
  recorded commit, and is removed".
- **Differs**: `Repo.undoInfo` (`src/git/repo.ts` line 1053) finds nothing of it in the reflog and `Controller.action`
  (`src/extension/controller.ts` line 1689) records nothing for it, so **Undo** right after it offers to take back the
  operation before it.

Acceptance:

- [ ] The URL `https://example.com/lib.git`, the folder `lib` and the branch `main` run
  `git submodule add -b main -- https://example.com/lib.git lib`, then
  `git submodule update --init --recursive -- lib`.
- [ ] The folder box is filled with `lib` for `https://github.com/owner/lib.git`, `git@github.com:owner/lib.git` and
  `C:\repos\lib\`.
- [ ] A folder that is there and not empty is refused with **<full path> is not empty**, and `vendor/lib` says **In
  <full path>**.
- [ ] `--upload-pack=evil` as the URL and `-lib` as the folder are refused, and nothing runs.
- [ ] A repository in a folder on disk: git refuses, and Gitardo asks **Add submodule vendor/lib from a folder on
  disk?** with `git -c protocol.file.allow=always submodule add --…`; run, the submodule is listed as added.
- [ ] It runs as a `GitOp` through `Controller.run`, each command written to the **Gitardo** output channel; the
  commands shown are the ones that run.
- [ ] The submodules inside the one added are cloned and listed under it.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `submodule.initialize` — Initialize a submodule

Clones a submodule that is not initialized, and the ones inside it, at the commits recorded for them.

- **Kind**: Operation
- **Function**: `submoduleUpdateOp(submodule, false)` in `src/git/ops.ts`, through `updateSubmodule(submodule)` in
  `src/extension/actions.ts`.
- **Entry points**: **Initialize** in the context menu of a submodule.
- **Offered when**: it is not initialized.
- **Plan**: `> git submodule update --init --recursive -- <path>`, or `> git -C <outer> submodule update --init
  --recursive -- <path>` for one inside another. Its title is **Update <path> to the recorded commit**. From a folder
  on disk, after asking, with `-c protocol.file.allow=always`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`); from a folder on disk, Confirm.
- **Undo**: Gitardo remembers nothing, and the reflog has nothing of it: **Undo** right after it offers to take back
  the operation before it. `submodule.deinit` empties its folder again.
- **After**: it is on its recorded commit, green in the section, and the submodules inside it are listed under it.
- **Refuses**: a path that is no longer a submodule: **<path> is no longer a submodule of this repository.**
- **Uses**: `submodule.read`, `preview.ask`, `log.write`.
- **Implemented in**: the `submoduleUpdate` case of `Action` and of `resolveAction`; `updateSubmodule`;
  `submoduleUpdateOp` and `allowingDisk`; `submoduleMenu`.
- **Tested by**: `test/submodules.test.ts` "deinitializes a submodule, initializes it again, and removes it with its
  clone", "runs the commands about a submodule inside another from that one" and "offers to clone from a folder on
  disk, which git refuses for submodules, for its commands only".
- **Differs**: as for `submodule.add`: `src/git/repo.ts` line 1053 and `src/extension/controller.ts` line 1689.

Acceptance:

- [ ] **Initialize** on the deinitialized `vendor/lib` runs `git submodule update --init --recursive -- vendor/lib`,
  and `vendor/lib` and `vendor/lib/inner` are then on their recorded commits.
- [ ] For `inner` inside `vendor/lib` it runs `git -C vendor/lib submodule update --init --recursive -- inner`.
- [ ] Git refusing `transport 'file' not allowed` is followed by the question **Update <path> to the recorded commit
  from a folder on disk?** with `git -c protocol.file.allow=always -C vendor/lib submodule update --init --recursive
  -- inner`.
- [ ] It runs as a `GitOp` through `Controller.run`, logged; the commands shown are the ones that run.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `submodule.update` — Update a submodule to the recorded commit

Checks out in a submodule the commit the repository records for it, and in the submodules inside it theirs, after
saying what that leaves behind.

- **Kind**: Operation
- **Function**: `submoduleUpdateOp(submodule, false)` in `src/git/ops.ts`, through `updateSubmodule(submodule)` and
  `submoduleRisks(submodule, false)` in `src/extension/actions.ts`.
- **Entry points**: **Update to the recorded commit <short>** in the context menu of a submodule.
- **Offered when**: it is initialized, a commit is recorded for it, and it is ahead of it, behind it, both, or does
  not have it (`submodule.status`).
- **Plan**: `> git submodule update --init --recursive -- <path>` (`-C <outer>` for one inside another), titled
  **Update <path> to the recorded commit**; from a folder on disk, after asking, with
  `-c protocol.file.allow=always`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`), unless it leaves something behind: then
  Confirm, asked as a question with a note of one or both of **It has <n> commits after the recorded one on a detached
  HEAD: no branch will have them. Create a branch in it first to keep them.** (on a detached `HEAD` and ahead) and
  **It has changes that are not committed: git keeps them, or stops if they are in the way.**
- **Undo**: Gitardo remembers nothing, and the reflog of the repository has nothing of it: **Undo** right after it
  offers to take back the operation before it. What was checked out in the submodule is in the reflog of the
  submodule.
- **After**: it is green in the section, on the recorded commit.
- **Refuses**: a path that is no longer a submodule; git stops when its changes are in the way, and Gitardo says
  **Update <path> to the recorded commit failed.** with the first error line of git.
- **Uses**: `submodule.status`, `preview.ask`, `log.write`.
- **Implemented in**: the `submoduleUpdate` case of `Action` and of `resolveAction`; `updateSubmodule`,
  `submoduleRisks`; `submoduleUpdateOp`; `submoduleMenu`.
- **Tested by**: `test/submodules.test.ts` "adds a submodule with the ones inside it, and follows it against the
  commit recorded for it" and "runs the commands about a submodule inside another from that one".
- **Differs**: as for `submodule.add`: `src/git/repo.ts` line 1053 and `src/extension/controller.ts` line 1689.

Acceptance:

- [ ] `vendor/lib` checked out one commit behind its recorded one, on a detached `HEAD`, is updated by
  `git submodule update --init --recursive -- vendor/lib` without a question, and is then on the recorded commit.
- [ ] With 2 commits after the recorded one on a detached `HEAD`, it asks first, even with `gitardo.commandPreview` at
  `off`, and the question says **It has 2 commits after the recorded one on a detached HEAD**.
- [ ] With files not committed in it, it asks first and says **It has changes that are not committed**.
- [ ] It asks as a question, not with a warning; the README's list of what asks does not name it.
- [ ] It runs as a `GitOp` through `Controller.run`, logged; the commands shown are the ones that run.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `submodule.update-remote` — Update a submodule to the latest of its branch

Brings a submodule to the last commit of the branch it follows on its remote, and its own submodules to what that
commit records; the new commit is then staged by hand to record it.

- **Kind**: Operation
- **Function**: `submoduleUpdateOp(submodule, true)` in `src/git/ops.ts`.
- **Entry points**: **Update to the latest of <branch>** in the context menu of a submodule, or **Update to the latest
  of its remote** when `.gitmodules` names no branch for it.
- **Offered when**: it is initialized.
- **Plan**: `> git submodule update --init --remote -- <path>` (`-C <outer>` for one inside another), then
  `> git -C <path from the root> submodule update --init --recursive`; titled **Update <path> to its remote branch**,
  with the note **Stage the submodule afterwards to record the commit it is on now.** From a folder on disk, after
  asking, both with `-c protocol.file.allow=always`.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`); from a folder on disk, Confirm. Nothing is said
  of the commits on a detached `HEAD` or the changes it leaves behind, as `submodule.update` says them.
- **Undo**: Gitardo remembers nothing: **Undo** right after it offers to take back the operation before it.
  `submodule.update` goes back to the recorded commit.
- **After**: the submodule is usually ahead of its recorded commit; **Stage <short> as the recorded commit**
  (`submodule.record`) records it.
- **Refuses**: a path that is no longer a submodule.
- **Uses**: `submodule.record`, `preview.ask`, `log.write`.
- **Implemented in**: the `submoduleUpdate` case of `Action` with `remote`, and of `resolveAction`;
  `submoduleUpdateOp`; `submoduleMenu`.
- **Tested by**: *none*.
- **Differs**: as for `submodule.add`: `src/git/repo.ts` line 1053 and `src/extension/controller.ts` line 1689.

Acceptance:

- [ ] On `vendor/lib`, following `main`, the entry reads **Update to the latest of main** and runs
  `git submodule update --init --remote -- vendor/lib`, then `git -C vendor/lib submodule update --init --recursive`.
- [ ] Without a branch in `.gitmodules` the entry reads **Update to the latest of its remote**.
- [ ] It runs as a `GitOp` through `Controller.run`, logged; the commands shown are the ones that run.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `submodule.record` — Stage the commit of a submodule as the recorded one

Stages the commit a submodule has checked out, so that the next commit of the repository records it.

- **Kind**: Operation
- **Function**: `submoduleRecordOp(submodule, head)` in `src/git/ops.ts`.
- **Entry points**: **Stage <short> as the recorded commit** in the context menu of a submodule, `<short>` being the
  commit it has checked out.
- **Offered when**: it is initialized and is ahead of its recorded commit, behind it, both, or does not have it.
- **Plan**: `> git add -- <path>`, or `> git -C <outer> add -- <path>` for one inside another; titled **Stage <path> at
  <short>**, with the note **Commit it to record that commit for the submodule.**
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Gitardo remembers nothing: **Undo** right after it offers to take back the operation before it. Unstaging
  the submodule (`changes.unstage`) takes it back.
- **After**: its tooltip says **Staged as the recorded commit: <short>**; its icon still compares it with the commit
  recorded in `HEAD` until the repository is committed (`commit.create`).
- **Refuses**: one that is not initialized: **<path> is not initialized.**; a path that is no longer a submodule.
- **Uses**: `commit.create`, `preview.ask`, `log.write`.
- **Implemented in**: the `submoduleRecord` case of `Action` and of `resolveAction`; `submoduleRecordOp`;
  `submoduleMenu`.
- **Tested by**: `test/submodules.test.ts` "runs the commands about a submodule inside another from that one" and
  "adds a submodule with the ones inside it, and follows it against the commit recorded for it"; `test/e2e/ui.mjs` "a
  submodule is added from the real prompts, says where it stands against the recorded commit, and is removed".
- **Differs**: as for `submodule.add`: `src/git/repo.ts` line 1053 and `src/extension/controller.ts` line 1689.

Acceptance:

- [ ] `vendor/lib` one commit ahead: the entry runs `git add -- vendor/lib`, `git status` then has `M  vendor/lib`,
  and after a commit of the repository it is green again.
- [ ] For `inner` inside `vendor/lib` it runs `git -C vendor/lib add -- inner`.
- [ ] It is not offered for a submodule on its recorded commit.
- [ ] It runs as a `GitOp` through `Controller.run`, logged; the command shown is the one that runs.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `submodule.sync` — Synchronize the URLs of the submodules

Copies the URLs of `.gitmodules` to the configuration of the submodules, and of the ones inside them, after they changed
in `.gitmodules`.

- **Kind**: Operation
- **Function**: `submoduleSyncOp()` in `src/git/ops.ts`.
- **Entry points**: **Synchronize URLs from .gitmodules** in the context menu of any submodule.
- **Offered when**: always.
- **Plan**: `> git submodule sync --recursive`, titled **Synchronize submodule URLs**: for every submodule, whichever
  menu it was chosen from.
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`).
- **Undo**: Gitardo remembers nothing, and the URLs it replaced are not kept: **Undo** right after it offers to take
  back the operation before it.
- **Uses**: `preview.ask`, `log.write`.
- **Implemented in**: the `submoduleSync` case of `Action` and of `resolveAction`; `submoduleSyncOp`; `submoduleMenu`.
- **Tested by**: `test/submodules.test.ts` "synchronizes the URLs of .gitmodules".
- **Differs**: as for `submodule.add`: `src/git/repo.ts` line 1053 and `src/extension/controller.ts` line 1689.

Acceptance:

- [ ] After the URL of `vendor/lib` in `.gitmodules` changed to `https://example.com/lib.git`, it runs
  `git submodule sync --recursive`, and `git config submodule.vendor/lib.url` then says
  `https://example.com/lib.git`.
- [ ] It runs as a `GitOp` through `Controller.run`, logged; the command shown is the one that runs.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `submodule.deinit` — Deinitialize a submodule

Empties the folder of a submodule and forgets it locally, keeping it in `.gitmodules`, in the commits and in its clone in
the git directory, after asking; offers to force it when it has changes.

- **Kind**: Operation
- **Function**: `submoduleDeinitOp(submodule, force)` in `src/git/ops.ts`, with `lettingGoOf(op, submodule, state)` in
  `src/extension/actions.ts`.
- **Entry points**: **Deinitialize…**, marked as an entry that loses work, in the context menu of a submodule.
- **Offered when**: it is initialized.
- **Plan**: `> git.close <folder>` for the submodule and each initialized one inside it that VS Code's Git extension
  has open, then `> git submodule deinit -- <path>` (`-C <outer>` for one inside another). When git refuses because
  of `local modifications`, the forced variant: the same `git.close` lines and `> git submodule deinit --force --
  <path>`, titled **Deinitialize <path> (force)**. On Windows, a command that fails because the folder is held is run
  again, up to 20 times 200 ms apart, with a warning in the log **…: Windows holds the folder, so it is tried again.**
- **Danger**: Confirm, because its folder is emptied: always asked as a question, **Deinitialize <path>?**, with **Its
  folder is emptied, and it stays in .gitmodules and in the commits: update it to get it back. Its commits stay in its
  clone in the git directory.** The forced variant is Destructive, asked with a warning, **Deinitialize <path>
  (force)?**, with **The submodule has changes that are not committed: they will be lost. Its commits stay in its
  clone in the git directory.**
- **Undo**: Gitardo remembers nothing: **Undo** right after it offers to take back the operation before it.
  **Initialize** (`submodule.initialize`) checks it out again from its clone; changes that the forced variant deleted
  are lost.
- **After**: the section lists it as not initialized, without the submodules inside it.
- **Refuses**: a path that is no longer a submodule.
- **Uses**: `submodule.initialize`, `preview.ask`, `log.write`.
- **Implemented in**: the `submoduleDeinit` case of `Action` and of `resolveAction`; `submoduleDeinitOp` with its
  `fallback`; `lettingGoOf` and `closeInVSCodeGit`; `submoduleMenu`; `Controller.runWithFallback`.
- **Tested by**: `test/submodules.test.ts` "confirms what loses work, and offers to force a deinit git refuses" and
  "deinitializes a submodule, initializes it again, and removes it with its clone".
- **Differs**: the README's list of what asks before losing work (`README.md` line 632) does not name the forced
  deinitialize, which the code asks before as `danger` (`src/git/ops.ts` line 926). As for `submodule.add`, **Undo**
  right after it offers to take back the operation before it (`src/git/repo.ts` line 1053,
  `src/extension/controller.ts` line 1689).

Acceptance:

- [ ] **Deinitialize…** on `vendor/lib` asks **Deinitialize vendor/lib?** with `git submodule deinit -- vendor/lib`,
  even with `gitardo.commandPreview` at `off`; run, its folder is empty and it is listed as not initialized, with its
  recorded commit kept.
- [ ] With a file changed in it, git refuses, and Gitardo warns **Deinitialize vendor/lib (force)?** with
  `git submodule deinit --force -- vendor/lib`.
- [ ] The forced variant is asked with a warning, which agrees with what loses work; the README's list of what asks
  names it (differs).
- [ ] Each command, and each `git.close` line, is written to the **Gitardo** output channel; the commands shown are the
  ones that run.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `submodule.remove` — Remove a submodule

Deletes a submodule from the repository: its folder, with what is in it, its entry in `.gitmodules`, the record of it
and its clone in the git directory; the removal is staged, ready to commit.

- **Kind**: Operation
- **Function**: `submoduleRemoveOp(submodule, gitDir)` in `src/git/ops.ts`, with `lettingGoOf(op, submodule, state)`
  and the `finish` of the `submoduleRemove` case of `resolveAction` in `src/extension/actions.ts`.
- **Entry points**: **Remove…**, marked as an entry that loses work, in the context menu of a submodule.
- **Offered when**: always, initialized or not.
- **Reads**: the git directory of the repository that has it (`Repo.gitDirAt(parent)`), to find its clone in
  `modules/<name>`; a name of `.gitmodules` that leads out of that folder is no clone to delete.
- **Plan**: `> git.close <folder>` for it and each initialized one inside it that VS Code's Git extension has open;
  `> git submodule deinit --force -- <path>`; `> git rm --force -- <path>` (`-C <outer>` for one inside another); then,
  when its clone is there, `> delete <git directory>/modules/<name>`. On Windows a command that fails because the
  folder is held is run again, as for `submodule.deinit`.
- **Danger**: Destructive, because its changes and the commits in it that are not pushed are deleted: always asked
  with a warning, **Remove submodule <path>?**, with **Its folder is deleted, with the changes and the commits in it
  that are not pushed. Gitardo then deletes its clone in <folder>. The removal is staged along with .gitmodules:
  commit them to record it.** (the middle sentence only when there is a clone to delete).
- **Undo**: cannot: its clone is deleted, and with it the commits that were only there. **Undo** right after it offers
  to take back the operation before it. Unstaging `.gitmodules` and the submodule (`changes.unstage`) and initializing
  it clones it again from its URL.
- **After**: the removal of the submodule and the change to `.gitmodules` are staged; without other submodules the
  section is gone. A clone that cannot be deleted is said in the log as a warning, **<folder> could not be deleted:
  <reason>**, and the operation still succeeds.
- **Refuses**: a path that is no longer a submodule.
- **Uses**: `commit.create`, `preview.ask`, `log.write`.
- **Implemented in**: the `submoduleRemove` case of `Action` and of `resolveAction`, with its `finish`;
  `submoduleRemoveOp`; `lettingGoOf` and `closeInVSCodeGit`; `submoduleMenu`.
- **Tested by**: `test/submodules.test.ts` "runs the commands about a submodule inside another from that one",
  "confirms what loses work, and offers to force a deinit git refuses" and "deinitializes a submodule, initializes it
  again, and removes it with its clone"; `test/e2e/ui.mjs` "a submodule is added from the real prompts, says where it
  stands against the recorded commit, and is removed".
- **Differs**: the README's list of what asks before losing work (`README.md` line 632) does not name removing a
  submodule, which the code asks before as `danger` (`src/git/ops.ts` line 952). As for `submodule.add`, **Undo**
  right after it offers to take back the operation before it (`src/git/repo.ts` line 1053,
  `src/extension/controller.ts` line 1689).

Acceptance:

- [ ] **Remove…** on `vendor/lib` warns **Remove submodule vendor/lib?**, naming its clone in `.git/modules`, with
  `git submodule deinit --force -- vendor/lib` and `git rm --force -- vendor/lib`, even with `gitardo.commandPreview`
  at `off`.
- [ ] Run, the folder `vendor/lib` and `.git/modules/vendor/lib` are gone, and the staged files are `.gitmodules` and
  `vendor/lib`.
- [ ] For `inner` inside `vendor/lib` the commands are `git -C vendor/lib submodule deinit --force -- inner` and
  `git -C vendor/lib rm --force -- inner`.
- [ ] It asks with a warning, which agrees with what loses work; the README's list of what asks names it (differs).
- [ ] Each command, each `git.close` line and the `delete` line are written to the **Gitardo** output channel; the
  commands shown are the ones that run.
- [ ] Right after it, **Undo** does not offer to take back an operation made before it in its place (differs).

### `submodule.update-on-checkout` — Update the submodules after a checkout

Git leaves the submodules where they are on a checkout; Gitardo then checks out in each the commit the new `HEAD`
records, cloning the ones it adds, as part of the checkout.

- **Kind**: Operation
- **Function**: `updatingSubmodules(op, to, state, repo)` in `src/extension/actions.ts`, which sets the `then` of the
  checkout to `submodulesUpdateOp()` (`src/git/ops.ts`), with `submoduleRisks(submodule, true)` for each submodule.
- **Entry points**: none of its own: it runs after `branch.checkout` (a branch, a remote branch, a tag or a commit),
  after `branch.create` with **Create and check out**, after `stash.branch`, and after **Undo** of a checkout
  (`undo.last`).
- **Offered when**: `gitardo.submodules.updateOnCheckout` is on, its default, and the repository has submodules now, or
  the commit checked out has a `.gitmodules` (`Repo.hasSubmodulesAt(to)`, `git cat-file -e <to>:.gitmodules`).
- **Plan**: once the commands of the checkout succeeded, `> git submodule update --init --recursive`, titled **Update
  the submodules**. When the checkout is asked, the question says it after **and once that is done:**. From a folder
  on disk, after asking **Update the submodules from a folder on disk?**,
  `> git -c protocol.file.allow=always submodule update --init --recursive`.
- **Danger**: None of its own. It makes the checkout Confirm, asked even with `gitardo.commandPreview` at `off`, when a
  submodule has commits after its recorded one on a detached `HEAD`: the note of the checkout then has **Then the
  submodules go to the commits recorded for them.** and, for each, **<path> has <n> commits after the recorded one on a
  detached HEAD: no branch will have them. Create a branch in it first to keep them.** A submodule with files not
  committed adds **<path> has changes that are not committed: git keeps them, or stops if they are in the way.**, which
  is said when the checkout is asked but does not make it ask. What is said is of the submodules as they are before
  the checkout.
- **Undo**: the checkout is taken back from the reflog, and that **Undo** updates the submodules again the same way.
  What the update checked out in a submodule is in the reflog of the submodule.
- **After**: the submodules are on the commits the new `HEAD` records, those it adds cloned. When the checkout fails
  the update does not run. When the update fails, the checkout stays done and Gitardo says **Update the submodules
  failed.** with the first error line of git.
- **Settings**: `gitardo.submodules.updateOnCheckout`; off, the submodules stay on what they had and the section says
  how far that is from what is recorded.
- **Uses**: `branch.checkout`, `branch.create`, `stash.branch`, `undo.last`, `submodule.status`, `preview.ask`,
  `log.write`.
- **Implemented in**: `updatingSubmodules`, called by the `checkout`, `createBranch`, `undo` and `stashBranch` cases of
  `resolveAction`; `submodulesUpdateOp` and `allowingDisk`; `Repo.hasSubmodulesAt`; in `Controller.action`, the `then`
  run by `runWithFallback(ran.then, true)` once the checkout succeeded; `Controller.describe`, which says the `then`.
- **Tested by**: `test/e2e/suite.ts` "updates the submodules after a checkout, cloning from a folder on disk once
  allowed, unless the settings say not to"; `test/submodules.test.ts` "knows which commits have submodules, and updates
  them all after a checkout to one".

Acceptance:

- [ ] Checking out `with-lib`, whose commit records `vendor/lib` that is not cloned here, runs `git checkout with-lib`
  and then `git submodule update --init --recursive`; from a folder on disk git refuses, Gitardo asks **Update the
  submodules from a folder on disk?**, and run, `vendor/lib` is on the commit `with-lib` records.
- [ ] With `gitardo.submodules.updateOnCheckout` off, checking out `with-lib-2` runs no `submodule update`, and
  `vendor/lib` is one commit behind what `with-lib-2` records.
- [ ] In a repository without submodules, checking out a commit without `.gitmodules` runs no `submodule update`.
- [ ] With `gitardo.commandPreview` at `all`, the question of the checkout lists `git submodule update --init
  --recursive` after **and once that is done:**, and nothing else runs.
- [ ] A submodule with 1 commit after its recorded one on a detached `HEAD` makes the checkout ask first, even with
  `gitardo.commandPreview` at `off`, saying **<path> has 1 commit after the recorded one on a detached HEAD**.
- [ ] The update runs as a `GitOp` through `Controller.run`, and its command is written to the **Gitardo** output
  channel after the checkout's.
- [ ] Right after the checkout, **Undo** goes back to where it was and updates the submodules again.
