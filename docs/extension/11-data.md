# Stored data

What Gitardo keeps between runs, and where. It keeps none of the user's work: the repository is git's, and Gitardo
only remembers choices of the interface. These formats are already on users' machines, so a change to one reads the old
one too (R14.2 in the [code rules](03-code-rules.md)).

## What is stored

### VS Code's workspace state

`ExtensionContext.workspaceState`, a store VS Code keeps for each workspace and this extension, written with
`Memento.update`. Nothing is kept in the global state.

| Key | Holds | Written by | Read by | May it be lost? |
| --- | --- | --- | --- | --- |
| `gitardo.repository` | The root of the repository shown last, a path. | Choosing a repository (`Controller.selectRepo`) | Opening the workspace (`findRepos`) | Yes: the first repository is shown. |
| `gitardo.scope` | For each repository root, the branches shown (`Scope`): a kind or a list of reference names. | Choosing the branches shown (`settleScope`) | Opening a repository | Yes: every branch is shown. |
| `gitardo.branchesListed` | For each repository root, what the references list by their own filter (`Scope`), apart from the branches shown: a preset, the kinds listed or the full names ticked; none for a repository whose references list everything. | The funnel at the top of the references, and **Show all** under it (`setListed`) | Every read of the state (`listedIn`), which leaves out the names gone without writing it again | Yes: every branch is listed. |
| `gitardo.favorites` | For each repository root, the names of the favorite branches. | **Add to favorites**, **Remove from favorites** (`setFavorite`) | Every read of the state | Yes, but the user marked them by hand. |
| `gitardo.commitFiles` | `tree` or `list`: how the files of a commit or a comparison are listed. | The two buttons beside the count of files | Opening a tab | Yes. |
| `gitardo.recentComparisons` | For each repository root, the last 8 comparisons: the name and hash of each side, and the path they are narrowed to. | Opening a comparison (`rememberComparison`) | **Opened before** in the comparison's picker | Yes. |
| `worktreeSetupOffered` | `true` once Gitardo offered to choose what new worktrees get. | The offer after creating a worktree, opening the page | Creating a worktree | Yes: offered once more. |
| `worktreeInputs` | The last value typed for each input of the format of branch names (`gitardo.branches.inputs`, or `gitardo.worktrees.inputs`), by its name. | Creating a worktree or a branch with inputs; for a branch, the follow-up `rememberInputs` of the engine | The next creation, as defaults; sent to the engine as `inputs` of `feature/run` | Yes. |

*Not versioned.* None of these carries a version. The keys were named as they were added; the last two have no
`gitardo.` prefix. Renaming one loses what users have stored under it unless the old key is read too.

### Each webview's own state

`getState` and `setState` of the webview API, which VS Code keeps while the view is hidden, when it is loaded again and
across restarts. Written through `saveUiState` (`src/webview/vscode.ts`), which merges into what the view already saved.

| View | Holds |
| --- | --- |
| Graph, Gitardo Graph | The widths of the references, the details and the graph, the height of the details, whether the references are put away; the folded merges, for up to 20 repositories and 200 merges each (`rememberFolds`); whether the search lists only what it finds. |
| Changes view | The draft of the commit message; which folders of its files are open. |
| Details, tab of a commit | Which folders of the files are open. |
| References, Branches view | Which sections are open, for the session (`openedIn`). |
| Review | Which proposals are accepted, the layout (cards or columns), the widths of the columns, the file selected, the filter. |
| What new worktrees get | Which groups are open. |
| Flow Diff, Flow Merge | The widths of the columns. |

These are read with defaults for anything missing (`loadUiState`), so a value from an older version that lacks a field
is filled in.

What a repository left keeps with `gitardo.repositories.keepOpen` on, in the views and in the extension host, is kept
in memory only, never stored: it is gone once the window is closed or the setting is turned off (`repo.keep-open`).

### Settings sent to the engine

From stage 1 of [the transition](../transition/06-roadmap.md), the `gitardo.*` settings are sent to Gitardo's engine
in `initialize`, and again with `settings/changed` whenever a `gitardo.*` or `git.*` setting changes, under the
desktop's names, which are the same without `gitardo.` ([05-stored-data.md](../transition/05-stored-data.md)). The
engine keeps nothing of them on disk.

### Settings Gitardo writes

Gitardo writes VS Code settings only when the user changes the same thing from its interface:

| Setting | Target | Written by |
| --- | --- | --- |
| `gitardo.graph.showStashes`, `gitardo.graph.dateSeparators`, `gitardo.graph.combineUpstream`, `gitardo.branches.sortOrder` | User settings | The toggles of the **…** menu of the toolbar and of the references (`Controller.onMessage`) |
| `gitardo.worktrees.linkFolders`, `gitardo.worktrees.copyFiles`, `gitardo.worktrees.setupCommands` | The settings of the workspace folder or the user's, as chosen | The page of what new worktrees get, and **Link into New Worktrees** and **Copy into New Worktrees** in the Explorer (`src/extension/worktreeSettings.ts`) |
| `git.worktreeSymlinkFolders`, `git.worktreeIncludeFiles` | The same | The same page, when the user asks for VS Code's worktrees to get the same |

### Files

| File | Holds | Lives |
| --- | --- | --- |
| `.git/rebase-merge/gitardo-split` (of the worktree) | The hash of a commit being split by an interactive rebase | Until the rebase ends: git deletes the folder with it |
| `<git folder>/gitardo-rebase/` (of the worktree) | The todo list and the messages of an interactive rebase, a squash, a split or a new message for an older commit, when Gitardo's engine runs them: written as steps of the plan, once the user agreed | Deleted by the engine once no rebase is under way in the worktree, after the rebase ended in the same command or after it stopped and was continued to its end or aborted; written over by the next rebase when a delete failed |
| `<temp>/gitardo-rebase-*/` | The same, when the extension runs them itself, without the engine: written while the operation is built, before the question | Deleted when the rebase ends in the same command (`onSuccess` in `interactiveRebase`); left in the temporary folder when the rebase stops, fails or is cancelled at the question (see [RS7](13-risks.md)) |
| `<temp>/gitardo-<uuid>.patch` | A patch from the clipboard, for `git am` or `git apply`; the engine writes it with a step of its plan, from the text of VS Code's clipboard that the extension gives as the answer `clipboard` | Deleted once applied, whether git succeeded or not |
| `<temp>/gitardo-worktree-*/` | The patterns given to `git ls-files` while looking for ignored entries | Deleted after |
| A file the user chooses | A patch saved with **Save as patch…** | The user's |

## Versions

None of the stored formats has a version number. Each is read as what it is today, with defaults for what is missing:
a key that cannot be read as expected is ignored for the favorites (`Controller.favorites` checks the shape), and taken
as written for the others, which only Gitardo writes. A damaged value cannot crash the extension's activation: each is
read when it is needed, not at start. When a format has to change, the change adds a version or a new key, reads the
old one, and says so on this page.

## Migrations

None so far.

## How it is written

VS Code writes the workspace state and the webview state itself; Gitardo calls `update` and `setState` and does not
wait for the first to finish, except for the favorites and the repository chosen. Several windows of the same workspace
share the workspace state: the last to write a key wins, and nothing merges two windows' favorites (see
[RS4](13-risks.md)).

## What never changes

- The names of the settings, `gitardo.*`, and their meanings.
- The keys of the workspace state above.
- The name of `gitardo-split`, which a rebase stopped by an older version may still have in its folder.
- The name of `gitardo-rebase` in the git folder, which a rebase the engine stopped still reads, and which the desktop
  writes too.
